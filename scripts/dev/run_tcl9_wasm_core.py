#!/usr/bin/env python3
"""Run the focused C-Tcl 9.0.3 core test slice through the **Zig WASM runtime**.

**Scope.**  This harness compiles each upstream ``.test`` file (bundled
with the upstream Tcl 9 ``tcltest.tcl`` framework — *unmodified on
disk*; the bundler in ``tests/external/run_tcl9_tests.py:_bundle()``
applies a small, documented set of in-memory ``_patch_tcltest_source``
rewrites and prepends ``_PRE_TCLTEST`` / ``_PREAMBLE`` stubs around
it) to WASM via ``core.compiler.codegen.wasm`` and executes it under
wasmtime against the Zig runtime in ``runtime/zig/``.  Pass / fail
numbers from this script are the **production correctness signal**
for the WASM ship target — unlike the Python-VM-side sibling
(``scripts/run_tcl9_vm_core.py``), which is internal dev signal only.

A regression here represents a real WASM-runtime gap that blocks
production correctness against upstream Tcl 9.  See
``tests/baselines/tcl9-tcltest-wasm/README.md`` for the full hand-off.

Hard rules
----------
* **Never** edit ``tmp/tcl9.0.3/library/tcltest/tcltest.tcl``,
  ``tmp/tcl9.0.3/library/init.tcl``, or any ``.test`` file under
  ``tmp/tcl9.0.3/tests/``.
* **No new monkey-patches.**  The bundle's pre-tcltest preamble
  (``tests/external/run_tcl9_tests.py:_PRE_TCLTEST``) and the
  ``_patch_tcltest_source`` rewrites are existing technical debt
  carried over from when tcltest first booted under WASM; they are
  not a pattern to extend.  Fix the runtime instead.
* Fix root causes in ``runtime/zig/``,
  ``core/compiler/codegen/wasm/``, ``core/parsing/``, or
  ``core/commands/registry/tcl/``.  Mirror new contracts with focused
  pytest under ``tests/test_wasm_*.py``.

Architecture
------------
Each stem is a fresh, isolated unit:

  1. ``_bundle(<stem>.test)`` — concatenate the patched tcltest, the
     pre-tcltest preamble, and the unmodified test file.
  2. ``_compile_tcl_with_diag`` — lower → CFG → WASM codegen.
  3. ``_run_wasm`` with a fresh wasmtime engine + store, capturing
     stdout / stderr and the populated ``DiagMap``.
  4. Parse the tcltest summary out of stdout; map any trap or
     compile failure into a WASM-shaped bucket (W0-..-W10).

Unlike the Python VM harness there is no parent-fork copy-on-write
amortisation — each WASM module is compiled fresh because tcltest is
inlined into the bundle.  Compile + wasmtime cold start typically
runs ~1-4 s per clean stem on a developer laptop; per-stem
wall-clock timeout (default 180 s) catches hangs.

Outputs
-------
* ``tmp/tcl9-wasm-core-report.json`` — per-stem result rows.
* ``tmp/tcl9-wasm-core-categories.md`` — human-readable bucket dossier.
* ``tests/baselines/tcl9-tcltest-wasm/summary.json`` — committed baseline.
* ``tests/baselines/tcl9-tcltest-wasm/categories/<stem>.toml`` — per-stem
  TOML mirroring the existing WASM-side schema (``good_to_have``,
  ``just_to_match_ctcl``, ``skip``, ``[baseline]``).

Usage
-----
::

    python scripts/run_tcl9_wasm_core.py             # full slice
    python scripts/run_tcl9_wasm_core.py --stems parseOld word
    python scripts/run_tcl9_wasm_core.py --workers 1 --timeout 120
    python scripts/run_tcl9_wasm_core.py --refresh-baseline
"""

from __future__ import annotations

import argparse
import json
import multiprocessing as mp
import multiprocessing.connection  # noqa: F401  (mp.connection isn't auto-imported)
import os
import re
import sys
import time
import traceback
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
TCL_TREE = REPO_ROOT / "tmp" / "tcl9.0.3"
TESTS_DIR = TCL_TREE / "tests"

OUT_REPORT = REPO_ROOT / "tmp" / "tcl9-wasm-core-report.json"
OUT_DOSSIER = REPO_ROOT / "tmp" / "tcl9-wasm-core-categories.md"
BASELINE_DIR = REPO_ROOT / "tests" / "baselines" / "tcl9-tcltest-wasm"
BASELINE_SUMMARY = BASELINE_DIR / "summary.json"
BASELINE_CATEGORIES_DIR = BASELINE_DIR / "categories"

# CORE_STEMS — mirrors scripts/run_tcl9_vm_core.py.  Same parser /
# interp / control-flow / variable-scope / builtin / introspection
# slice.  Out-of-scope for this round: IO/channels/sockets/exec/safe/
# threads/load/encoding/event-loop/TclOO/clock/package/tcltest itself.
CORE_STEMS: list[str] = [
    # Parser
    "parse",
    "parseExpr",
    "parseOld",
    "word",
    "subst",
    # Interpreter
    "basic",
    "compile",
    "execute",
    "eval",
    "result",
    "misc",
    # Control flow
    "if",
    "if-old",
    "while",
    "while-old",
    "for",
    "for-old",
    "foreach",
    "switch",
    "error",
    # Variable / scope
    "set",
    "set-old",
    "var",
    "upvar",
    "uplevel",
    "incr",
    "incr-old",
    "proc",
    "proc-old",
    "namespace",
    "namespace-old",
    "apply",
    "rename",
    "unknown",
    "trace",
    # Builtins
    "append",
    "format",
    "scan",
    "list",
    "lset",
    "lsetComp",
    "llength",
    "lindex",
    "linsert",
    "lreplace",
    "lrange",
    "lmap",
    "lpop",
    "lrepeat",
    "lsearch",
    "lseq",
    "split",
    "join",
    "concat",
    "string",
    "regexp",
    "regexpComp",
    "mathop",
    "expr",
    "expr-old",
    "compExpr",
    "compExpr-old",
    # Introspection
    "info",
    "cmdAH",
    "cmdIL",
    "cmdInfo",
    "cmdMZ",
    "util",
]


# Regex helpers


SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)")
FAILED_TEST_LINE_RE = re.compile(r"^==== (\S+) FAILED", re.MULTILINE)
UNSUPPORTED_CMD_RE = re.compile(r"unsupported command:\s*([^\s\n]+)")
INVALID_CMD_RE = re.compile(r'invalid command name "([^"]+)"')


@dataclass
class StemResult:
    """Single-stem outcome from one WASM bundle run."""

    stem: str
    total: int = 0
    passed: int = 0
    skipped: int = 0
    failed: int = 0
    crashed: bool = False
    crash_type: str = ""  # ``CompileFail`` | ``Trap`` | ``Timeout`` | ``ChildDied`` | ``NoSummary``
    crash_msg: str = ""
    trap_site: str = ""
    failed_ids: list[str] = field(default_factory=list)
    missing_commands: list[str] = field(default_factory=list)
    compile_s: float = 0.0
    run_s: float = 0.0
    duration_s: float = 0.0
    bucket: str = ""
    bucket_reason: str = ""

    def to_row(self) -> dict[str, object]:
        return {
            "stem": self.stem,
            "total": self.total,
            "passed": self.passed,
            "skipped": self.skipped,
            "failed": self.failed,
            "crashed": self.crashed,
            "crash_type": self.crash_type,
            "crash_msg": self.crash_msg,
            "trap_site": self.trap_site,
            "failed_ids": self.failed_ids,
            "missing_commands": self.missing_commands,
            "compile_s": round(self.compile_s, 3),
            "run_s": round(self.run_s, 3),
            "duration_s": round(self.duration_s, 3),
            "bucket": self.bucket,
            "bucket_reason": self.bucket_reason,
        }


# Worker — one process per stem.


def _run_stem_in_child(stem: str, run_timeout_s: float, conn) -> None:
    """Compile+run one stem inside a worker process and send the row back.

    Each worker is a fresh process — no shared wasmtime state, no
    cross-stem leak risk by construction.  We import the harness
    primitives lazily so an import failure surfaces inside the
    worker (where it can be reported to the parent) rather than
    poisoning the whole sweep at module-load time.
    """
    sys.path.insert(0, str(REPO_ROOT))

    try:
        from tests.external.run_tcl9_tests import _bundle  # type: ignore[attr-defined]
        from tests.test_wasm_real_tcl import (  # type: ignore[attr-defined]
            _compile_tcl_with_diag,
            _resolve_trap,
            _run_wasm,
        )
    except Exception as exc:
        conn.send(
            asdict(
                StemResult(
                    stem=stem,
                    crashed=True,
                    crash_type="ImportFail",
                    crash_msg=f"{type(exc).__name__}: {exc}",
                )
            )
        )
        conn.close()
        return

    result = StemResult(stem=stem)
    overall_start = time.monotonic()
    path = TESTS_DIR / f"{stem}.test"
    if not path.is_file():
        result.crashed = True
        result.crash_type = "MissingTestFile"
        result.crash_msg = f"{path} not found"
        conn.send(asdict(result))
        conn.close()
        return

    # --- Bundle + compile ---
    try:
        bundle_src = _bundle(path)
    except Exception as exc:
        result.crashed = True
        result.crash_type = "BundleFail"
        result.crash_msg = f"{type(exc).__name__}: {str(exc).splitlines()[0][:300]}"
        result.duration_s = time.monotonic() - overall_start
        conn.send(asdict(result))
        conn.close()
        return

    compile_start = time.monotonic()
    try:
        wasm_bytes, diag = _compile_tcl_with_diag(bundle_src, f"{stem}.test")
    except Exception as exc:
        result.crashed = True
        result.crash_type = "CompileFail"
        result.crash_msg = f"{type(exc).__name__}: {str(exc).splitlines()[0][:300]}"
        result.compile_s = time.monotonic() - compile_start
        result.duration_s = time.monotonic() - overall_start
        conn.send(asdict(result))
        conn.close()
        return
    result.compile_s = time.monotonic() - compile_start

    # --- Stage tcltest helper files (tcltests.tcl, internals.tcl, …) into a
    # preopened tmpdir so test files that ``source [file join [file dirname
    # [info script]] X]`` can resolve them.  Mirrors the staging logic in
    # tests/external/run_tcl9_tests.py:_run_bundle.
    import shutil
    import tempfile

    helpers_dir = TCL_TREE / "tests"
    run_start = time.monotonic()
    stdout_text = ""
    stderr_text = ""
    try:
        with tempfile.TemporaryDirectory(prefix=f"tcl9-wasm-core-{stem}-") as host_tmp:
            for helper in (
                "tcltests.tcl",
                "internals.tcl",
                "encodingVectors.tcl",
                "pkgIndex.tcl",
            ):
                src_path = helpers_dir / helper
                if src_path.exists():
                    shutil.copyfile(src_path, Path(host_tmp) / helper)
            try:
                run_result = _run_wasm(
                    wasm_bytes,
                    capture_stdout=True,
                    capture_stderr=True,
                    preopen_tmpdir=host_tmp,
                    timeout_s=run_timeout_s,
                )
            except Exception as trap:
                stderr_text = getattr(trap, "tcl_stderr", "") or ""
                stdout_text = getattr(trap, "tcl_stdout", "") or ""
                result.crashed = True
                # Watchdog-driven epoch interrupts produce a wasmtime
                # interrupt trap with empty stderr — surface those
                # specifically rather than mis-classifying as a runtime
                # trap, since the bucket dictionary distinguishes them.
                trap_repr = str(trap)
                if "interrupt" in trap_repr.lower() or "epoch" in trap_repr.lower():
                    result.crash_type = "Timeout"
                    result.crash_msg = (
                        f"WASM watchdog interrupt at run_timeout_s={run_timeout_s:.0f}s"
                    )
                else:
                    result.crash_type = "Trap"
                    try:
                        result.trap_site = _resolve_trap(trap, stderr_text, diag)[:600]
                    except Exception:
                        result.trap_site = trap_repr[:600]
                    # Headline message for the categories markdown.
                    first_stderr_line = next(
                        (
                            line
                            for line in stderr_text.splitlines()
                            if line.strip() and "tcl trap:" in line
                        ),
                        "",
                    )
                    result.crash_msg = (first_stderr_line or trap_repr.splitlines()[0])[:400]
            else:
                stdout_text = run_result[1] if len(run_result) >= 2 else ""
                stderr_text = run_result[2] if len(run_result) >= 3 else ""
    except Exception as exc:
        # Outer staging / tempdir issue.  Should be rare; surface it so
        # the row carries enough to triage.
        result.crashed = True
        result.crash_type = "HostError"
        result.crash_msg = f"{type(exc).__name__}: {str(exc).splitlines()[0][:300]}"
    result.run_s = time.monotonic() - run_start

    # --- Parse tcltest summary out of stdout (whether or not we trapped) ---
    summary = SUMMARY_RE.search(stdout_text)
    if summary:

        def _i(s: str) -> int:
            return int(s) if s else 0

        result.total = _i(summary.group(1))
        result.passed = _i(summary.group(2))
        result.skipped = _i(summary.group(3))
        result.failed = _i(summary.group(4))

    for m in FAILED_TEST_LINE_RE.finditer(stdout_text):
        result.failed_ids.append(m.group(1))

    # Surface unsupported / invalid commands seen anywhere in stderr,
    # ordered by first appearance.  These feed the W0-stub-trap and
    # W8-missing-command buckets.
    seen: set[str] = set()
    for m in UNSUPPORTED_CMD_RE.finditer(stderr_text):
        cmd = m.group(1)
        if cmd not in seen:
            seen.add(cmd)
            result.missing_commands.append(cmd)
    for m in INVALID_CMD_RE.finditer(stderr_text):
        cmd = m.group(1)
        if cmd not in seen:
            seen.add(cmd)
            result.missing_commands.append(cmd)

    # Trap with no summary line is the biggest leverage class — capture
    # a stderr tail when nothing else was set, so the categories.md
    # has something readable to fall back to.
    if result.crashed and not result.crash_msg and stderr_text.strip():
        result.crash_msg = stderr_text.strip().splitlines()[-1][:400]

    # ``no summary, no trap`` is a distinct mode: the bundle ran to
    # completion but tcltest never emitted a summary line — typically
    # because every test was filtered out by a constraint, or the
    # bundle returned early before invoking ``cleanupTests``.
    if not result.crashed and result.total == 0 and not summary:
        result.crashed = True
        result.crash_type = "NoSummary"
        result.crash_msg = "ran without trapping but no tcltest summary line emerged"

    result.duration_s = time.monotonic() - overall_start
    conn.send(asdict(result))
    conn.close()


# Driver: parent forks one child per stem.


def run_sweep(
    stems: list[str],
    *,
    workers: int = 1,
    wall_timeout: float = 180.0,
    run_timeout: float = 120.0,
) -> list[StemResult]:
    """Execute *stems* with at most *workers* concurrent children.

    *wall_timeout* is the parent-side hard ceiling (SIGKILL on overrun)
    that catches hangs in the compile pipeline or anywhere outside
    the WASM watchdog's reach.  *run_timeout* is the wasmtime epoch
    deadline applied inside ``_run_wasm`` — should be < *wall_timeout*.
    """
    ctx = mp.get_context("fork")
    pending = list(stems)
    in_flight: list[tuple[str, mp.Process, "multiprocessing.connection.Connection", float]] = []
    results: dict[str, StemResult] = {}

    def launch(stem: str) -> tuple[mp.Process, "multiprocessing.connection.Connection"]:
        parent_conn, child_conn = ctx.Pipe(duplex=False)
        p = ctx.Process(target=_run_stem_in_child, args=(stem, run_timeout, child_conn))
        p.start()
        child_conn.close()
        return p, parent_conn

    print(
        f"  Running {len(stems)} stems × {workers} concurrent worker(s), "
        f"wall_timeout={wall_timeout:.0f}s/stem, run_timeout={run_timeout:.0f}s/stem"
    )

    def fill() -> None:
        while pending and len(in_flight) < workers:
            stem = pending.pop(0)
            p, c = launch(stem)
            in_flight.append((stem, p, c, time.monotonic()))

    fill()

    while in_flight or pending:
        if not in_flight:
            fill()
            continue

        now = time.monotonic()
        wait_until = min(start + wall_timeout for _, _, _, start in in_flight)
        wait = max(0.05, wait_until - now)

        readable_conns = [conn for _, _, conn, _ in in_flight]
        try:
            ready = multiprocessing.connection.wait(readable_conns, timeout=wait)
        except OSError:
            ready = []

        # Time out any children past their deadline.
        now = time.monotonic()
        timed_out = [
            (stem, p, conn) for stem, p, conn, start in in_flight if (now - start) >= wall_timeout
        ]
        for stem, p, conn in timed_out:
            if p.is_alive():
                p.kill()
            p.join(timeout=2)
            try:
                conn.close()
            except Exception:
                pass
            results[stem] = StemResult(
                stem=stem,
                crashed=True,
                crash_type="WallTimeout",
                crash_msg=f"exceeded {wall_timeout:.0f}s wall-clock (parent SIGKILL)",
                duration_s=wall_timeout,
            )
            print(f"    {stem:<14} WALL-TIMEOUT  {wall_timeout:5.1f}s")
            in_flight[:] = [t for t in in_flight if t[0] != stem]

        # Collect anything ready.
        for conn in ready:
            entry = next((t for t in in_flight if t[2] is conn), None)
            if entry is None:
                continue
            stem, p, _, start = entry
            try:
                payload = conn.recv()
            except EOFError:
                p.join(timeout=2)
                results[stem] = StemResult(
                    stem=stem,
                    crashed=True,
                    crash_type="ChildDied",
                    crash_msg=f"exitcode={p.exitcode}",
                    duration_s=time.monotonic() - start,
                )
                print(f"    {stem:<14} CHILD-DIED  exitcode={p.exitcode}")
                try:
                    conn.close()
                except Exception:
                    pass
                in_flight[:] = [t for t in in_flight if t[0] != stem]
                continue

            try:
                conn.close()
            except Exception:
                pass
            p.join(timeout=2)
            if p.is_alive():
                p.kill()
                p.join(timeout=1)

            sr = StemResult(**payload)  # type: ignore[arg-type]
            results[stem] = sr
            if sr.crashed:
                status = f"{sr.crash_type:<12}"
            else:
                status = f"{sr.passed:>4}/{sr.total:<4} (F{sr.failed} S{sr.skipped})"
            print(
                f"    {sr.stem:<14} {status}  "
                f"compile={sr.compile_s:4.1f}s run={sr.run_s:4.1f}s wall={sr.duration_s:5.1f}s"
            )
            in_flight[:] = [t for t in in_flight if t[0] != stem]

        fill()

    return [results.get(s, StemResult(stem=s, crashed=True, crash_type="Lost")) for s in stems]


# Categorisation — WASM-shaped buckets.
#
# Failure modes here are genuinely different from the Python VM side —
# we do not see a Python ValueError leaking from a builtin (Zig has no
# such concept); we see wasmtime traps, ``unsupported command: X``
# stderr lines from runtime/zig/dispatch/tcl_stub_fallback.zig,
# arity violations against runtime/zig/dispatch/tcl_cmd_registry.zig,
# and codegen-side IR validation failures.

# Test ID hints that classify a single tcltest test as "deliberately
# don't fix on the WASM side" — bytecode internals, refcount probes,
# C-side test framework that we do not emulate.  Per-test bucketing
# into ``just_to_match_ctcl`` / ``skip`` is curated manually in the
# committed TOMLs; this list seeds the auto-generated baseline so the
# obvious incompatibility cases land in the right bucket on first run.
W9_INTERNAL_HINTS = (
    "tcl::unsupported::disassemble",
    "tcl::unsupported::representation",
    "tcl::unsupported::assemble",
    "info frame",
    "info cmdcount",
    "testbytestring",
    "testevalex",
    "testexprparser",
    "testparser",
    "testobj",
    "tcl::test",
)


def _is_w9_internal(failed_id: str, msg: str) -> bool:
    text = f"{failed_id} {msg}".lower()
    return any(h in text for h in (h.lower() for h in W9_INTERNAL_HINTS))


def _categorise(sr: StemResult) -> tuple[str, str]:
    """Pick the dominant bucket for a stem result.

    Returns ``(bucket_id, short_reason)``.  Per-test-ID bucketing is
    seeded into the categories TOML at write time; this stem-level
    bucket is just the headline for the report.

    Crashes win over partial results — a crashed bundle truncates
    every later test in the file, so it is always higher leverage
    than a mixed-fail row.
    """
    if sr.crashed:
        ctype = sr.crash_type
        if ctype in ("Timeout", "WallTimeout"):
            return ("W0-timeout", "exceeded wall-clock — likely infinite loop or hang")
        if ctype == "ChildDied":
            return ("W0-child-died", f"worker died — {sr.crash_msg}")
        if ctype in ("CompileFail", "BundleFail", "ImportFail", "MissingTestFile", "HostError"):
            # Compile-side or harness-side failure; worth distinguishing
            # CompileFail (real codegen bug) from the rest in the dossier
            # tier ordering, but they share the W0-bootstrap stem-level
            # bucket because each one stops every test in the file.
            if ctype == "CompileFail":
                return ("W0-codegen-bug", f"compile failed: {sr.crash_msg}")
            return ("W0-bootstrap", f"{ctype}: {sr.crash_msg}")
        if ctype == "NoSummary":
            return ("W7-no-tests", "ran without trapping but no tcltest summary line emerged")
        if ctype == "Trap":
            # Distinguish stub-fallback traps (unsupported command in
            # runtime/zig/dispatch/tcl_stub_fallback.zig) from genuine
            # runtime traps in implemented commands.  ``invalid command
            # name "X"`` from ::unknown / dispatch is a separate bucket
            # because it indicates a *missing-from-spec* command rather
            # than a runtime stub.
            for cmd in sr.missing_commands:
                # ``unsupported command: X`` lines win over generic
                # traps — they point directly at a stub in
                # tcl_stub_fallback.zig that needs an implementation.
                if "unsupported" in sr.crash_msg.lower() or "unsupported" in sr.trap_site.lower():
                    return ("W0-stub-trap", f"unsupported command: {cmd}")
                if "invalid command name" in sr.crash_msg.lower():
                    return ("W8-missing-command", f"invalid command name: {cmd}")
            return ("W0-runtime-error", sr.crash_msg or "runtime trap")
        return ("W0-bootstrap", sr.crash_msg or ctype)
    if sr.total == 0:
        return ("W7-no-tests", "tcltest produced no summary line")
    if sr.failed == 0:
        return ("clean", "all tests pass or skip")
    return ("W-mixed-fail", f"{sr.failed} of {sr.total} failed")


# Output writers


def _aggregate(results: list[StemResult]) -> dict[str, int]:
    return {
        "stems": len(results),
        "stems_clean": sum(1 for r in results if r.failed == 0 and r.total > 0 and not r.crashed),
        "stems_partial": sum(1 for r in results if r.failed > 0 and not r.crashed),
        "stems_crashed": sum(1 for r in results if r.crashed),
        "tests_total": sum(r.total for r in results),
        "tests_passed": sum(r.passed for r in results),
        "tests_skipped": sum(r.skipped for r in results),
        "tests_failed": sum(r.failed for r in results),
    }


def write_json_report(results: list[StemResult]) -> None:
    OUT_REPORT.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "tcl_tree": str(TCL_TREE),
        "summary": _aggregate(results),
        "rows": [sr.to_row() for sr in results],
    }
    OUT_REPORT.write_text(json.dumps(payload, indent=2, sort_keys=True))
    print(f"  wrote {OUT_REPORT.relative_to(REPO_ROOT)}")


def write_dossier(results: list[StemResult]) -> None:
    OUT_DOSSIER.parent.mkdir(parents=True, exist_ok=True)
    summary = _aggregate(results)

    lines: list[str] = []
    lines.append("# Tcl 9 core test slice — Zig WASM runtime dossier\n")
    lines.append(f"Generated: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\n")
    lines.append("")
    lines.append(
        "**Scope: Zig WASM runtime (`runtime/zig/` + `core/compiler/codegen/wasm/`).**  "
        "Unlike the Python-VM-side sibling, this *is* the production "
        "ship-target signal.  Crashes here represent real WASM-runtime "
        "gaps that block production correctness against upstream Tcl 9.\n"
    )
    lines.append("")
    lines.append(
        "Sources used unmodified: `tmp/tcl9.0.3/library/tcltest/tcltest.tcl`, "
        "every `.test` file in `tmp/tcl9.0.3/tests/`.  The bundle is "
        "constructed by `tests/external/run_tcl9_tests.py:_bundle()`, "
        "which prepends a documented preamble and applies the targeted "
        "`_patch_tcltest_source` rewrites carried over from the existing "
        "WASM tcltest harness.  No new monkey-patches.\n"
    )
    lines.append("")
    lines.append("## Summary\n")
    lines.append(f"- Stems run: **{summary['stems']}**")
    lines.append(f"- Clean (all pass / skip): **{summary['stems_clean']}**")
    lines.append(
        f"- Partial fail (file completed, some tests failed): **{summary['stems_partial']}**"
    )
    lines.append(
        f"- Crashed (compile fail / trap / timeout / no summary): **{summary['stems_crashed']}**"
    )
    pct = 100 * summary["tests_passed"] // max(1, summary["tests_total"])
    lines.append(
        f"- Tests: total **{summary['tests_total']}**, "
        f"passed **{summary['tests_passed']}** ({pct}%), "
        f"skipped **{summary['tests_skipped']}**, "
        f"failed **{summary['tests_failed']}**\n"
    )

    # Crash root-cause section — the headline.
    crashed = [r for r in results if r.crashed]
    if crashed:
        lines.append("## Crashes by root cause")
        lines.append("")
        lines.append(
            "Each row is a stem whose bundle either failed to compile "
            "or trapped before reaching a tcltest summary.  These are "
            "the highest-leverage fixes — every later test in the same "
            "file is blocked by them.\n"
        )
        by_cause: dict[str, list[StemResult]] = {}
        for r in crashed:
            by_cause.setdefault(r.crash_type, []).append(r)
        for cause, rs in sorted(by_cause.items(), key=lambda kv: -len(kv[1])):
            lines.append(f"### `{cause}` ({len(rs)} stem(s))")
            lines.append("")
            for r in rs:
                lines.append(
                    f"- **`{r.stem}`** (partial: {r.passed}/{r.total} P, "
                    f"{r.failed} F before crash) — `{r.crash_msg}`"
                )
                if r.missing_commands:
                    lines.append(
                        f"    - command(s) seen on the trap path: "
                        f"`{', '.join(r.missing_commands[:5])}`"
                    )
                if r.trap_site and r.trap_site != r.crash_msg:
                    first = r.trap_site.splitlines()[:3]
                    for line in first:
                        lines.append(f"    - `{line}`")
            lines.append("")

    # Per-stem table
    lines.append("## Per-stem result")
    lines.append("")
    lines.append("| stem | total | passed | skipped | failed | bucket | note |")
    lines.append("|------|------:|-------:|--------:|-------:|--------|------|")
    for r in results:
        note = r.crash_msg or (r.failed_ids[0] if r.failed_ids else "")
        if len(note) > 60:
            note = note[:57] + "..."
        lines.append(
            f"| `{r.stem}` | {r.total} | {r.passed} | {r.skipped} | {r.failed} | "
            f"`{r.bucket}` | {note} |"
        )
    lines.append("")

    # Bucket roll-up
    counts: Counter[str] = Counter()
    for r in results:
        counts[r.bucket] += 1
    lines.append("## Stem-level bucket roll-up")
    lines.append("")
    lines.append("| bucket | stems |")
    lines.append("|--------|------:|")
    for b, n in counts.most_common():
        lines.append(f"| `{b}` | {n} |")
    lines.append("")

    # Bucket dictionary — WASM-shaped.
    lines.append("## Bucket dictionary (WASM-shaped)")
    lines.append("")
    lines.append(
        "| id | symptom | leverage |\n"
        "|----|---------|----------|\n"
        "| `W0-bootstrap` | Bundle harness / setup error before WASM "
        "ever ran (test file missing, bundle pre-flight failure). | very high |\n"
        "| `W0-codegen-bug` | `core/compiler/codegen/wasm/` raised "
        "during lowering / IR build / codegen. | very high |\n"
        "| `W0-stub-trap` | wasmtime trap with `unsupported command: X` "
        "on stderr — `runtime/zig/dispatch/tcl_stub_fallback.zig` was hit. | very high |\n"
        "| `W0-runtime-error` | wasmtime trap inside an implemented Zig "
        "handler — bug in `runtime/zig/cmds/`. | very high |\n"
        "| `W0-timeout` | wasmtime watchdog or parent wall-clock killed "
        "the run — likely infinite loop / pathological input. | very high |\n"
        "| `W0-child-died` | Worker process died without sending a row "
        "(SIGSEGV in wasmtime, OOM, etc.). | very high |\n"
        "| `W7-no-tests` | Bundle ran without trapping but tcltest never "
        "emitted a `Total / Passed / Skipped / Failed` summary line. | high |\n"
        '| `W8-missing-command` | Trap with `invalid command name "X"` '
        "for X **not currently in the registry** — Python-side spec gap, "
        "not just a Zig stub. | very high |\n"
        "| `W9-internal` | Per-test-ID classification (in TOMLs): "
        "bytecode disassembly, object-rep probes, `info frame` / `info "
        "cmdcount`, `tcl::test` C-extension hooks.  **Incompatible by "
        "design.** | none |\n"
        "| `W9-cosmetic` | Per-test-ID classification: error-message "
        "wording / list-quoting / frame counts that round-trip "
        "identically but differ byte-for-byte. **Do not fix.** | none |\n"
        "| `W10-environment` | C-test commands (`testparser`, "
        "`testevalex`, …) deliberately unregistered; tests should skip. | none |\n"
        "| `W-mixed-fail` | Stem completed; some tests passed and some "
        "failed.  Per-test triage in the matching `categories/<stem>.toml`. | medium |\n"
        "| `clean` | All tests passed or skipped — no failures. | (no fix needed) |\n"
    )

    # Ranked fix order
    lines.append("## Ranked fix order")
    lines.append("")
    lines.append(
        "Buckets are ordered by **leverage** — number of test IDs unblocked "
        "per fix.  Crashes in `W0-*` sit at the top because each one currently "
        "truncates dozens to hundreds of tests in a single file.\n"
    )

    crashed = [r for r in results if r.crashed]
    by_cause = {}
    for r in crashed:
        by_cause.setdefault(r.crash_type, []).append(r)

    if "CompileFail" in by_cause:
        lines.append("### Tier 1 — `CompileFail` (`core/compiler/codegen/wasm/` raised)")
        lines.append("")
        lines.append(
            "**Why first**: codegen-side failures block the bundle from "
            "ever instantiating; every test in the file is silently "
            "dropped.  Look at the failing module / IR node in the "
            "exception stack and either lower it to a runtime fallback "
            "or fix the codegen path.\n"
        )
        for r in by_cause["CompileFail"]:
            lines.append(f"- **`{r.stem}`** — `{r.crash_msg}`")
        lines.append("")

    if "Trap" in by_cause:
        lines.append("### Tier 2 — wasmtime trap before the tcltest summary")
        lines.append("")
        lines.append(
            "**Why second**: each row stops the bundle mid-run — "
            "test IDs after the trap point never execute.  Group by "
            "trap site (`runtime/zig/dispatch/tcl_stub_fallback.zig` "
            "for `W0-stub-trap`, the named handler for "
            "`W0-runtime-error`) and fix the underlying Zig code.\n"
        )
        for r in by_cause["Trap"]:
            cmds = f" — missing: {', '.join(r.missing_commands[:3])}" if r.missing_commands else ""
            lines.append(
                f"- **`{r.stem}`** ({r.passed}/{r.total} P before trap) — `{r.crash_msg}`{cmds}"
            )
        lines.append("")

    for label in ("Timeout", "WallTimeout"):
        if label in by_cause:
            lines.append(f"### Tier 3 — `{label}` (suspected hang)")
            lines.append("")
            lines.append(
                "**Why third**: the WASM watchdog or the parent "
                "wall-clock killed the run.  Bisect by test ID; the "
                "trap point usually points at a non-terminating loop "
                "or quadratic loop in the runtime.\n"
            )
            for r in by_cause[label]:
                lines.append(f"- **`{r.stem}`**")
            lines.append("")

    no_tests = [r for r in results if r.bucket == "W7-no-tests"]
    if no_tests:
        lines.append("### Tier 4 — Stem completes without a tcltest summary")
        lines.append("")
        lines.append(
            "**Why fourth**: the bundle ran cleanly but tcltest "
            "never observed a `test` invocation — typically because "
            "every test fell through a constraint, or the bundle "
            "returned early before `cleanupTests`.  Check that the "
            "constraint predicates are evaluating correctly under "
            "the WASM runtime's `info commands` / `package require` "
            "primitives.\n"
        )
        for r in no_tests:
            lines.append(f"- **`{r.stem}`** ({r.duration_s:.1f}s elapsed)")
        lines.append("")

    mixed = sorted(
        [r for r in results if r.bucket == "W-mixed-fail"],
        key=lambda r: -r.failed,
    )
    if mixed:
        lines.append("### Tier 5 — Largest partial-fail stems")
        lines.append("")
        lines.append(
            "Once Tier 1–4 are clear, these are where per-test-ID "
            "triage into the per-stem TOML's `good_to_have` / "
            "`just_to_match_ctcl` / `skip` buckets pays off.  "
            "Top 15 by failure count:\n"
        )
        for r in mixed[:15]:
            lines.append(
                f"- **`{r.stem}`** — {r.failed}/{r.total} failed, "
                f"first failing: `{r.failed_ids[0] if r.failed_ids else '?'}`"
            )
        lines.append("")

    # Hand-off rules — WASM-specific.
    lines.append("### Permanent hand-off rules")
    lines.append("")
    lines.append(
        "Read `tests/baselines/tcl9-tcltest-wasm/README.md` first — it "
        "is the durable hand-off; the section below is a running "
        "summary.\n\n"
        "- **This is the ship gate.**  Crashes catalogued here block "
        "production correctness against upstream Tcl 9.\n"
        "- **Never** edit `tmp/tcl9.0.3/library/tcltest/tcltest.tcl`, "
        "`tmp/tcl9.0.3/library/init.tcl`, or any `.test` file under "
        "`tmp/tcl9.0.3/tests/`.  Fix the runtime / codegen instead.\n"
        "- **No new monkey-patches.**  The bundle's pre-tcltest preamble "
        "(`tests/external/run_tcl9_tests.py:_PRE_TCLTEST`) and the "
        "`_patch_tcltest_source` rewrites are existing technical debt; "
        "do not extend them.  If `tcltest.tcl` needs framework support "
        "that isn't there, add it to the runtime, not the framework.\n"
        "- **Fix root causes in `runtime/zig/`, `core/compiler/codegen/"
        "wasm/`, `core/parsing/`, or `core/commands/registry/tcl/`.**  "
        "Mirror new contracts with focused pytest under "
        "`tests/test_wasm_*.py`.\n"
        "- This baseline must **not** treat existing parity-gate "
        "failures as new regressions — interrogate "
        "`tests/baselines/wasm_command_parity.json` first.\n"
        "- Tests classified `W9-internal` / `W9-cosmetic` "
        "(bytecode disassembly, object-rep refcount probes, `info frame` "
        "line tables, `info cmdcount`, `tcl::test` C-extension hooks) "
        "are **incompatible by design** and must never be reclassified "
        "as bugs.\n"
        "- Re-run `make test-tcl9-wasm-core` after each fix; "
        "`tests/test_wasm_tcl9_core_baseline.py` will pass only if no "
        "stem regresses against "
        "`tests/baselines/tcl9-tcltest-wasm/summary.json`.\n"
    )

    # Top missing commands — feeds W0-stub-trap / W8-missing-command triage.
    miss: Counter[str] = Counter()
    for r in results:
        for c in r.missing_commands:
            miss[c] += 1
    if miss:
        lines.append("## Top traps by command (stub / missing-command blockers)")
        lines.append("")
        lines.append("| command | stems blocked |")
        lines.append("|---------|--------------:|")
        for c, n in miss.most_common(25):
            lines.append(f"| `{c}` | {n} |")
        lines.append("")

    fails = [r for r in results if r.failed > 0]
    if fails:
        lines.append("## Sample failing test IDs")
        lines.append("")
        for r in sorted(fails, key=lambda x: -x.failed)[:25]:
            ids = ", ".join(f"`{i}`" for i in r.failed_ids[:5])
            more = "" if len(r.failed_ids) <= 5 else f" … (+{len(r.failed_ids) - 5} more)"
            lines.append(f"- `{r.stem}` ({r.failed} fail / {r.total}): {ids}{more}")
        lines.append("")

    OUT_DOSSIER.write_text("\n".join(lines))
    print(f"  wrote {OUT_DOSSIER.relative_to(REPO_ROOT)}")


def write_baseline(results: list[StemResult]) -> None:
    BASELINE_DIR.mkdir(parents=True, exist_ok=True)
    BASELINE_CATEGORIES_DIR.mkdir(parents=True, exist_ok=True)

    summary = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "schema_version": 1,
        "stems": {
            r.stem: {
                "total": r.total,
                "passed_min": r.passed,
                "failed_max": r.failed,
                "skipped": r.skipped,
                "crashed": r.crashed,
                "crash_type": r.crash_type,
                "bucket": r.bucket,
            }
            for r in results
        },
    }
    BASELINE_SUMMARY.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(f"  wrote {BASELINE_SUMMARY.relative_to(REPO_ROOT)}")

    for r in results:
        toml_path = BASELINE_CATEGORIES_DIR / f"{r.stem}.toml"
        if toml_path.exists():
            continue  # don't overwrite curated TOMLs

        # Auto-seed: route IDs hinting at C-internals into
        # ``just_to_match_ctcl``; everything else stays in
        # ``[failing]`` for manual triage.  This mirrors the VM-side
        # auto-seed shape so curators can re-bucket later.
        w9_internal: list[str] = []
        plain_failing: list[str] = []
        for fid in r.failed_ids:
            if _is_w9_internal(fid, r.crash_msg):
                w9_internal.append(fid)
            else:
                plain_failing.append(fid)

        body: list[str] = []
        body.append(f"# {r.stem}.test — auto-generated WASM baseline.\n")
        body.append(f"# bucket = {r.bucket!r}\n")
        if r.bucket_reason:
            body.append(f"# reason = {r.bucket_reason!r}\n")
        body.append(f"trap_allowed = {'true' if r.crashed else 'false'}\n")
        body.append("good_to_have = []\n")
        body.append("just_to_match_ctcl = " + _toml_list(w9_internal) + "\n")
        body.append("skip = []\n\n")
        body.append("[baseline]\n")
        body.append("good_to_have_failing = 0\n")
        body.append(f"just_to_match_ctcl_failing = {len(w9_internal)}\n")
        body.append("skip_failing = 0\n")
        body.append(f"observed_total = {r.total}\n")
        body.append(f"observed_passed = {r.passed}\n")
        body.append(f"observed_failed = {r.failed}\n")
        body.append(f"observed_skipped = {r.skipped}\n")
        if plain_failing:
            body.append("\n[failing]\n")
            body.append("# Test IDs failing today — kept here for visibility, not gating.\n")
            body.append("ids = " + _toml_list(plain_failing) + "\n")
        if r.crashed:
            body.append("\n[crash]\n")
            body.append(f"type = {json.dumps(r.crash_type)}\n")
            body.append(f"msg = {json.dumps(r.crash_msg)}\n")
            if r.missing_commands:
                body.append("commands = " + _toml_list(r.missing_commands[:10]) + "\n")

        toml_path.write_text("".join(body))


def _toml_list(items: list[str]) -> str:
    if not items:
        return "[]"
    return "[" + ", ".join(json.dumps(s) for s in items) + "]"


# CLI


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--stems", nargs="*", help="Subset of stems to run (default: full CORE_STEMS)"
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=max(1, min(2, (os.cpu_count() or 2))),
        help="Worker pool size (default: min(2, cpu_count); WASM compile + wasmtime are RAM-heavy)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=180.0,
        help="Parent wall-clock timeout per stem in seconds (default: 180)",
    )
    parser.add_argument(
        "--run-timeout",
        type=float,
        default=120.0,
        help="wasmtime watchdog timeout per WASM run in seconds (default: 120)",
    )
    parser.add_argument(
        "--no-baseline",
        action="store_true",
        help="Don't write tests/baselines/tcl9-tcltest-wasm/",
    )
    parser.add_argument(
        "--refresh-baseline",
        action="store_true",
        help="Rewrite per-stem TOML baselines even if they already exist.",
    )
    args = parser.parse_args()

    if args.refresh_baseline and args.no_baseline:
        parser.error(
            "--refresh-baseline and --no-baseline are mutually exclusive: "
            "the first asks to rewrite baselines, the second asks to skip "
            "writing them."
        )
    if args.run_timeout >= args.timeout:
        parser.error(
            f"--run-timeout ({args.run_timeout}) must be strictly less than "
            f"--timeout ({args.timeout}) so the WASM watchdog fires before "
            "the parent SIGKILL takes the row's stderr with it"
        )

    stems = args.stems if args.stems else CORE_STEMS
    missing = [s for s in stems if not (TESTS_DIR / f"{s}.test").is_file()]
    if missing:
        print(f"error: missing test files: {missing}", file=sys.stderr)
        return 2

    if args.refresh_baseline and BASELINE_CATEGORIES_DIR.exists():
        for p in BASELINE_CATEGORIES_DIR.glob("*.toml"):
            p.unlink()

    started = time.monotonic()
    results = run_sweep(
        stems,
        workers=args.workers,
        wall_timeout=args.timeout,
        run_timeout=args.run_timeout,
    )
    elapsed = time.monotonic() - started

    for sr in results:
        sr.bucket, sr.bucket_reason = _categorise(sr)

    write_json_report(results)
    write_dossier(results)
    if not args.no_baseline:
        write_baseline(results)

    summary = _aggregate(results)
    print()
    print(
        f"  done in {elapsed:.1f}s — "
        f"clean={summary['stems_clean']} "
        f"partial={summary['stems_partial']} "
        f"crashed={summary['stems_crashed']} "
        f"tests P/F/S/T = "
        f"{summary['tests_passed']}/{summary['tests_failed']}/"
        f"{summary['tests_skipped']}/{summary['tests_total']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("\ninterrupted", file=sys.stderr)
        raise SystemExit(130) from None
    except Exception:
        traceback.print_exc()
        raise SystemExit(1) from None
