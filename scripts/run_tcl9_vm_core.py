#!/usr/bin/env python3
"""Run the focused C-Tcl 9.0.3 core test slice through the Python VM.

Sources the original ``tmp/tcl9.0.3/library/init.tcl`` and the original
``tmp/tcl9.0.3/library/tcltest/tcltest.tcl`` unmodified, executes each
test file via ``vm.interp.TclInterp``, and records pass / skip / fail
counts plus a categorised failure dossier.

**Hard rules** (see ``tests/baselines/tcl9-tcltest-vm/README.md`` for
the full hand-off): never edit ``tcltest.tcl``, never edit any
``.test`` file under ``tmp/tcl9.0.3/tests/``, never edit ``init.tcl``,
and never add a new monkey-patch — the existing ``auto_load`` /
``parray`` shim in ``vm/commands/tcltest_cmds.py`` is technical debt
to remove, not a pattern to extend.

Compile-cost minimisation
-------------------------
``init.tcl`` (~1500 lines) and ``tcltest.tcl`` (~3700 lines) dwarf the
cost of any single test file.  The parent process boots one
``TclInterp`` and sources both files exactly once.  Each test file
then runs in a forked child (Linux ``fork`` start method) which
inherits the parent's loaded interp through copy-on-write — no
re-compile, full per-file isolation against state leaks.  The parent
enforces a per-file wall-clock timeout (default 60 s); a child that
exceeds it is SIGKILLed and recorded as ``Timeout``.  Up to
``--workers`` (default ``min(4, cpu_count())``) children run in
parallel.

Outputs
-------
* ``tmp/tcl9-vm-core-report.json`` — per-stem result rows.
* ``tmp/tcl9-vm-core-categories.md`` — human-readable bucket dossier.
* ``tests/baselines/tcl9-tcltest-vm/summary.json`` — committed baseline.
* ``tests/baselines/tcl9-tcltest-vm/categories/<stem>.toml`` — per-stem
  TOML mirroring the WASM-side schema.

Usage
-----
::

    python scripts/run_tcl9_vm_core.py             # full slice
    python scripts/run_tcl9_vm_core.py --stems parse basic info
    python scripts/run_tcl9_vm_core.py --workers 1 --timeout 30
    python scripts/run_tcl9_vm_core.py --refresh-baseline
"""

from __future__ import annotations

import argparse
import io
import json
import multiprocessing as mp
import os
import re
import sys
import tempfile
import time
import traceback
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TCL_TREE = REPO_ROOT / "tmp" / "tcl9.0.3"
TESTS_DIR = TCL_TREE / "tests"

OUT_REPORT = REPO_ROOT / "tmp" / "tcl9-vm-core-report.json"
OUT_DOSSIER = REPO_ROOT / "tmp" / "tcl9-vm-core-categories.md"
BASELINE_DIR = REPO_ROOT / "tests" / "baselines" / "tcl9-tcltest-vm"
BASELINE_SUMMARY = BASELINE_DIR / "summary.json"
BASELINE_CATEGORIES_DIR = BASELINE_DIR / "categories"

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


# Bucket classification

# Patterns that classify a single tcltest test ID as "incompatible by design"
# (B9-internal) — we explicitly do not implement C-Tcl bytecode / object
# internals, so failures here are never bugs.
B9_INTERNAL_HINTS = (
    "tcl::unsupported::disassemble",
    "tcl::unsupported::representation",
    "tcl::unsupported::assemble",
    "info frame",
    "info cmdcount",
)


SUMMARY_RE = re.compile(
    r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)"
)
SUMMARY_FALLBACK_RE = re.compile(
    r"Total.?(\d+).?Passed.?(\d+).?Skipped.?(\d+).?Failed.?(\d+)"
)
INVALID_CMD_RE = re.compile(r'invalid command name "([^"]+)"')
FAILED_TEST_LINE_RE = re.compile(r"==== ([\S]+) FAILED")


def _tcl_quote(s: str) -> str:
    """Quote a host filesystem path for safe interpolation into Tcl source."""
    return "{" + s.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}") + "}"


@dataclass
class StemResult:
    """Single-stem outcome."""

    stem: str
    total: int = 0
    passed: int = 0
    skipped: int = 0
    failed: int = 0
    crashed: bool = False
    crash_type: str = ""
    crash_msg: str = ""
    failed_ids: list[str] = field(default_factory=list)
    missing_commands: list[str] = field(default_factory=list)
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
            "failed_ids": self.failed_ids,
            "missing_commands": self.missing_commands,
            "duration_s": round(self.duration_s, 3),
            "bucket": self.bucket,
            "bucket_reason": self.bucket_reason,
        }


# Worker entry point

# Set in the parent before forking children so each child inherits a
# fully-loaded interpreter via copy-on-write rather than re-compiling
# init.tcl + tcltest.tcl.
_INTERP = None  # type: ignore[var-annotated]


def _boot_parent_interp():
    """Load init.tcl + tcltest.tcl in the parent.  Children inherit it."""
    global _INTERP
    sys.path.insert(0, str(REPO_ROOT))

    from vm.commands.tcltest_cmds import setup_tcltest
    from vm.commands.test_support_cmds import setup_test_support
    from vm.interp import TclInterp

    interp = TclInterp(source_init=True)
    setup_test_support(interp)
    setup_tcltest(interp)
    # Force tcltest.tcl to source now so the compile / load happens once.
    interp.eval("package require tcltest 2.5; namespace import -force ::tcltest::*")
    _INTERP = interp
    return interp


def _run_stem_in_child(stem: str, conn) -> None:
    """Run one .test file in a forked child and send the result back."""
    from vm.commands import tcltest_cmds
    from vm.types import TclBreak, TclContinue, TclError, TclReturn

    interp = _INTERP
    if interp is None:  # pragma: no cover — parent never booted
        conn.send({"stem": stem, "crashed": True, "crash_type": "NoInterp",
                   "crash_msg": "parent did not boot interp before forking"})
        conn.close()
        return

    # Per-file fresh state (Python tcltest counters + Tcl-side aggregates).
    tcltest_cmds._reset_state()
    for k in ("Total", "Passed", "Skipped", "Failed"):
        try:
            interp.eval(f"set ::tcltest::numTests({k}) 0")
        except Exception:
            pass

    stdout_buf = io.StringIO()
    stderr_buf = io.StringIO()
    interp.channels["stdout"] = stdout_buf
    interp.channels["stderr"] = stderr_buf

    result = StemResult(stem=stem)
    start = time.monotonic()
    path = TESTS_DIR / f"{stem}.test"
    if not path.is_file():
        result.crashed = True
        result.crash_type = "MissingTestFile"
        result.crash_msg = f"{path} not found"
        conn.send(asdict(result))
        conn.close()
        return

    # Sandbox: each child runs in a fresh tempdir, so any stray relative-path
    # file writes by a misbehaving test land there and get reaped on exit
    # rather than polluting the repo root.  The original tests directory is
    # exposed via ::tcltest::testsDirectory so tests that source siblings
    # (e.g. `source [file join $::tcltest::testsDirectory constraints.tcl]`)
    # still resolve.
    sandbox = tempfile.TemporaryDirectory(prefix=f"tcl9-vm-core-{stem}-")
    try:
        os.chdir(sandbox.name)
        try:
            interp.eval(
                "set ::tcltest::testsDirectory "
                + _tcl_quote(str(TESTS_DIR))
            )
            interp.eval(
                "set ::tcltest::temporaryDirectory "
                + _tcl_quote(sandbox.name)
            )
            interp.eval(
                "set ::tcltest::workingDirectory "
                + _tcl_quote(sandbox.name)
            )
        except Exception:
            pass
        interp.global_frame.set_var("argv0", str(path))
        interp.global_frame.set_var("argv", "")
        interp.global_frame.set_var("argc", "0")
        interp.script_file = str(path)
        try:
            interp.eval(path.read_text(errors="replace"))
        except (SystemExit, TclReturn, TclBreak, TclContinue):
            # tcltest's cleanupTests propagates as TclReturn; all four
            # are non-error exits and must not count as crashes.
            pass
    except TclError as exc:
        result.crashed = True
        result.crash_type = "TclError"
        msg = getattr(exc, "message", None) or str(exc)
        result.crash_msg = msg.split("\n", 1)[0][:400]
        cmd = INVALID_CMD_RE.search(msg)
        if cmd:
            result.missing_commands.append(cmd.group(1))
    except Exception as exc:
        result.crashed = True
        result.crash_type = type(exc).__name__
        result.crash_msg = str(exc).split("\n", 1)[0][:400]
        cmd = INVALID_CMD_RE.search(str(exc))
        if cmd:
            result.missing_commands.append(cmd.group(1))
    finally:
        # Some tests `cd` elsewhere — get back into the sandbox so its
        # cleanup() can rmtree it without "directory not empty" / EBUSY.
        try:
            os.chdir("/")
        except Exception:
            pass
        try:
            sandbox.cleanup()
        except Exception:
            pass
    result.duration_s = time.monotonic() - start

    stdout_text = stdout_buf.getvalue()
    stderr_text = stderr_buf.getvalue()

    counts = _read_counts(interp)
    if counts["Total"] == 0:
        parsed = _parse_summary(stdout_text)
        if parsed:
            counts = parsed
    result.total = counts["Total"]
    result.passed = counts["Passed"]
    result.skipped = counts["Skipped"]
    result.failed = counts["Failed"]

    for m in FAILED_TEST_LINE_RE.finditer(stdout_text):
        result.failed_ids.append(m.group(1))

    if stderr_text and not result.crash_msg:
        tail = stderr_text.strip().splitlines()[-1:]
        if tail:
            result.crash_msg = tail[0][:400]

    conn.send(asdict(result))
    conn.close()


def _read_counts(interp) -> dict[str, int]:
    counts = {"Total": 0, "Passed": 0, "Skipped": 0, "Failed": 0}
    for k in counts:
        try:
            r = interp.eval(f"set ::tcltest::numTests({k})")
            counts[k] = int(r.value)
        except Exception:
            try:
                v = interp.global_frame.get_var(f"::tcltest::numTests({k})")
                counts[k] = int(v)
            except Exception:
                pass
    return counts


def _parse_summary(out: str) -> dict[str, int] | None:
    for line in reversed(out.splitlines()):
        m = SUMMARY_RE.search(line)
        if m:
            return {
                "Total": int(m.group(1)),
                "Passed": int(m.group(2)),
                "Skipped": int(m.group(3)),
                "Failed": int(m.group(4)),
            }
        m = SUMMARY_FALLBACK_RE.search(line)
        if m:
            return {
                "Total": int(m.group(1)),
                "Passed": int(m.group(2)),
                "Skipped": int(m.group(3)),
                "Failed": int(m.group(4)),
            }
    return None


# Driver: parent boots once, forks one child per stem (Linux fork — children
# inherit init.tcl + tcltest.tcl in memory via copy-on-write).

def run_sweep(
    stems: list[str],
    *,
    workers: int = 1,
    timeout: float = 60.0,
) -> list[StemResult]:
    print(
        f"  Booting parent interp (init.tcl + tcltest.tcl) — "
        f"shared by {len(stems)} forked children …"
    )
    boot_started = time.monotonic()
    _boot_parent_interp()
    print(f"  Parent ready in {time.monotonic() - boot_started:.1f}s")

    ctx = mp.get_context("fork")
    pending = list(stems)
    in_flight: list[tuple[str, mp.Process, "mp.connection.Connection", float]] = []
    results: dict[str, StemResult] = {}

    def launch(stem: str) -> tuple[mp.Process, "mp.connection.Connection"]:
        parent_conn, child_conn = ctx.Pipe(duplex=False)
        p = ctx.Process(target=_run_stem_in_child, args=(stem, child_conn))
        p.start()
        # Close our end of the write side so EOF propagates if the child
        # dies without sending.
        child_conn.close()
        return p, parent_conn

    print(f"  Running {len(stems)} stems × {workers} concurrent fork(s), {timeout:.0f}s/stem")

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

        # Find the next event: either a child is ready to read, or a timeout fires.
        now = time.monotonic()
        wait_until = min(start + timeout for _, _, _, start in in_flight)
        wait = max(0.05, wait_until - now)

        readable_conns = [conn for _, _, conn, _ in in_flight]
        try:
            ready = mp.connection.wait(readable_conns, timeout=wait)
        except OSError:
            ready = []

        # Time out any children past their deadline first.
        now = time.monotonic()
        timed_out = [
            (stem, p, conn) for stem, p, conn, start in in_flight if (now - start) >= timeout
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
                crash_type="Timeout",
                crash_msg=f"exceeded {timeout:.0f}s wall-clock",
                duration_s=timeout,
            )
            print(f"    {stem:<14} TIMEOUT  {timeout:5.1f}s")
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
            status = (
                "CRASH" if sr.crashed
                else f"{sr.passed:>4}/{sr.total:<4} (F{sr.failed} S{sr.skipped})"
            )
            print(f"    {sr.stem:<14} {status}  {sr.duration_s:5.1f}s")
            in_flight[:] = [t for t in in_flight if t[0] != stem]

        fill()

    return [results.get(s, StemResult(stem=s, crashed=True, crash_type="Lost")) for s in stems]


# Categorisation

def _is_b9_internal(failed_id: str, msg: str) -> bool:
    text = f"{failed_id} {msg}".lower()
    return any(h in text for h in (h.lower() for h in B9_INTERNAL_HINTS))


def _categorise(sr: StemResult) -> tuple[str, str]:
    """Pick the dominant bucket for a stem result.

    Returns (bucket_id, short_reason).  Per-test-ID bucketing is recorded
    inside the categories TOML at write time; this stem-level bucket is
    just the headline for the report.

    Order of precedence: crashes win over partial results, because a
    crashed file means the test author's `catch` envelope didn't hold
    — there's a host-Python exception leaking out of a builtin that
    would silently corrupt or truncate later tests in the same file.
    """
    if sr.crashed:
        if sr.crash_type == "Timeout":
            return ("B0-timeout", "exceeded wall-clock — likely infinite loop")
        if sr.crash_type == "ValueError":
            # Python ValueError leaking from a builtin — usually
            # ``int(...)`` on user input that should have raised TclError.
            return ("B0-host-exception", f"ValueError leak: {sr.crash_msg}")
        if sr.crash_type == "ChildDied":
            return ("B0-child-died", f"child process died — {sr.crash_msg}")
        if sr.missing_commands:
            return ("B8-missing-command", f"invalid command: {sr.missing_commands[0]}")
        if "package require" in sr.crash_msg.lower() or "can't find package" in sr.crash_msg.lower():
            return ("B0-bootstrap", f"package: {sr.crash_msg}")
        if sr.crash_type == "TclError":
            # A genuine TclError that escaped the test file's catches —
            # commonly a parser / lookup / namespace bug surfacing via a
            # construct that the test author wraps in `catch`, but our
            # error path bypasses the catch.
            return ("B0-tcl-error", f"escaped TclError: {sr.crash_msg}")
        return ("B0-bootstrap", sr.crash_msg or sr.crash_type)
    if sr.total == 0:
        # No crash, no test summary — file ran but tcltest never produced
        # output (e.g. all tests skipped via constraint, or `runAllTests`
        # short-circuit).
        return ("B7-no-tests", "tcltest produced no summary line")
    if sr.failed == 0:
        return ("clean", "all tests pass or skip")
    return ("B-mixed-fail", f"{sr.failed} of {sr.total} failed")


# Output writers

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


def write_dossier(results: list[StemResult]) -> None:
    OUT_DOSSIER.parent.mkdir(parents=True, exist_ok=True)
    summary = _aggregate(results)

    lines: list[str] = []
    lines.append("# Tcl 9 core test slice — VM dossier\n")
    lines.append(f"Generated: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\n")
    lines.append("")
    lines.append(
        "Sources used unmodified: `tmp/tcl9.0.3/library/init.tcl`, "
        "`tmp/tcl9.0.3/library/tcltest/tcltest.tcl`, "
        "and every `.test` file in `tmp/tcl9.0.3/tests/`.\n"
    )
    lines.append("")
    lines.append("## Summary\n")
    lines.append(f"- Stems run: **{summary['stems']}**")
    lines.append(f"- Clean (all pass / skip): **{summary['stems_clean']}**")
    lines.append(f"- Partial fail (file completed, some tests failed): **{summary['stems_partial']}**")
    lines.append(f"- Crashed (escaped exception, no tcltest summary): **{summary['stems_crashed']}**")
    lines.append(
        f"- Tests: total **{summary['tests_total']}**, "
        f"passed **{summary['tests_passed']}** "
        f"({100 * summary['tests_passed'] // max(1, summary['tests_total'])}%), "
        f"skipped **{summary['tests_skipped']}**, "
        f"failed **{summary['tests_failed']}**\n"
    )

    # Crash root-cause section — the headline of the dossier.
    crashed = [r for r in results if r.crashed]
    if crashed:
        lines.append("## Crashes by root cause")
        lines.append("")
        lines.append(
            "Each row is a stem whose test file crashed before tcltest "
            "could emit its `Total / Passed / Skipped / Failed` summary "
            "line. These are the highest-leverage fixes — every later "
            "test in the same file is blocked by them.\n"
        )
        lines.append("")
        # Group by crash type
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
                        f"    - missing command(s): `{', '.join(r.missing_commands)}`"
                    )
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

    # Bucket dictionary
    lines.append("## Bucket dictionary")
    lines.append("")
    lines.append(
        "| id | meaning | leverage |\n"
        "|----|---------|----------|\n"
        "| `B0-bootstrap` | Stem crashes before tcltest emits a summary line. "
        "Fix unblocks every test in the file. | very high |\n"
        "| `B1-parser` | Token / range / line-number drift in `parse.test`, "
        "`parseExpr.test`, `parseOld.test`, `subst.test`, `word.test`. | high |\n"
        "| `B2-expr` | Expr / mathop / number formatting, integer width, "
        "NaN/Inf/bignum. | medium |\n"
        "| `B3-control-flow` | `if/while/for/foreach/switch/error/break/continue/`\n"
        "`return`, `apply` — return-code propagation, `errorInfo`/`errorCode`. | medium |\n"
        "| `B4-list-string` | `list*`, `string`, `lset*`, `split`/`join`/`concat`, "
        "`format`, `scan`, `append`. | medium |\n"
        "| `B5-var-scope` | `set*`, `var`, `upvar`, `uplevel`, `incr*`, `proc*`, "
        "`namespace*`, `rename`, `cmdInfo`. | medium |\n"
        "| `B6-introspection` | `info` subcommands, `cmdAH/IL/MZ`, `apply`, `rename`. | medium |\n"
        "| `B7-framework` | `tcltest` itself / its glue commands "
        "(`fconfigure`, `interp create -safe`, `makeFile`, `viewFile`, `outputChannel`). | very high |\n"
        "| `B8-missing-command` | Whole file dies on `invalid command name \"X\"`. | very high |\n"
        "| `B9-cosmetic` | Cosmetic error wording / list-quoting / frame counts that "
        "round-trip identically but differ byte-for-byte. **Do not fix.** | none |\n"
        "| `B9-internal` | Bytecode / object-internal / `info frame` / `info cmdcount` / "
        "`tcl::unsupported::disassemble` / `representation` / refcount / shimmer. "
        "**Incompatible by design.** | none |\n"
        "| `B10-environment` | C-test commands (`testparser`, `testevalex`, …) "
        "deliberately unregistered; tests should skip. | none |\n"
    )

    # Suggested fix order — leverage-ranked, data-driven from this run.
    lines.append("## Ranked fix order")
    lines.append("")
    lines.append(
        "Buckets are ordered by **leverage** — number of test IDs unblocked "
        "per fix.  Crashes in B0-* sit at the top because each one currently "
        "truncates dozens to hundreds of tests in a single file.\n"
    )
    lines.append("")

    # Group crashed stems by crash type for the ranked view.
    crashed = [r for r in results if r.crashed]
    by_cause: dict[str, list[StemResult]] = {}
    for r in crashed:
        by_cause.setdefault(r.crash_type, []).append(r)

    # Tier 1: ValueError leaks
    if "ValueError" in by_cause:
        rs = by_cause["ValueError"]
        lines.append("### Tier 1 — Python `ValueError` leaks from builtins")
        lines.append("")
        lines.append(
            "**Why first**: a host-Python `ValueError` escaping from a "
            "builtin truncates the entire test file — every later test "
            "is silently dropped.  The pattern is identical across "
            "stems: a builtin calls Python `int()` on user input "
            "without converting the failure to `TclError`.  This is "
            "exactly the contract `tests/test_vm_tcltest_bootstrap.py` "
            "already pins for `lsearch -regexp` / `string compare "
            "-length`; extend the same pattern to the builtins below.\n"
        )
        for r in rs:
            lines.append(
                f"- **`{r.stem}`** — `{r.crash_msg}`. "
                f"Truncates at {r.passed + r.failed}/{r.total or '?'} reported. "
                "Fix: catch `ValueError` at the option-parser site and raise "
                "`TclError(f'expected integer but got \"{val}\"')`."
            )
        lines.append("")
        lines.append(
            "    Likely sites (search starting points): "
            "`vm/commands/proc_cmds.py` (`return -code`), "
            "`vm/commands/format_cmds.py`, `vm/commands/scan_cmds.py`, "
            "`vm/commands/info_cmds.py`, `vm/commands/string_cmds.py`."
        )
        lines.append("")

    # Tier 2: TclError escaping a user-level catch
    if "TclError" in by_cause:
        lines.append("### Tier 2 — `TclError` escaping a user-level `catch`")
        lines.append("")
        lines.append(
            "**Why second**: each of these is a real Tcl error that the "
            "test author wrapped in `catch` — but our error path bypasses "
            "the catch, propagating the error to the top of the file.  "
            "Investigate per-stem.\n"
        )
        for r in by_cause["TclError"]:
            lines.append(
                f"- **`{r.stem}`** — `{r.crash_msg}`. "
                f"Truncates at {r.passed + r.failed}/{r.total or '?'} reported."
            )
        lines.append("")

    # Tier 3: timeouts
    if "Timeout" in by_cause:
        lines.append("### Tier 3 — Timeouts (suspected infinite loop / pathological case)")
        lines.append("")
        lines.append(
            "**Why third**: 60 s wall-clock is an order of magnitude "
            "more than the slowest non-crashing stem (`expr-old` at 44 s "
            "with 461 tests).  These are almost certainly hangs, not "
            "honest slow paths.  Bisect by running individual test IDs.\n"
        )
        for r in by_cause["Timeout"]:
            lines.append(f"- **`{r.stem}`**")
        lines.append("")
    if "error" in by_cause:
        for r in by_cause["error"]:
            lines.append(f"- **`{r.stem}`** — generic `error`: `{r.crash_msg}`")
        lines.append("")

    # Tier 4: B7-no-tests
    no_tests = [r for r in results if r.bucket == "B7-no-tests"]
    if no_tests:
        lines.append("### Tier 4 — Stem completes without a tcltest summary")
        lines.append("")
        lines.append(
            "**Why fourth**: file ran to the end, but tcltest never "
            "saw any test invoke through the `test` command (or all "
            "tests skipped via constraint).  Check that the constraint "
            "predicates are evaluating correctly.\n"
        )
        for r in no_tests:
            lines.append(f"- **`{r.stem}`** ({r.duration_s:.1f}s elapsed, no tests run)")
        lines.append("")

    # Tier 5: largest mixed-fail stems
    mixed = sorted(
        [r for r in results if r.bucket == "B-mixed-fail"],
        key=lambda r: -r.failed,
    )
    if mixed:
        lines.append("### Tier 5 — Largest partial-fail stems (mixed-bucket)")
        lines.append("")
        lines.append(
            "Once Tier 1–4 are clear, these are where per-test-ID triage "
            "into B1 / B2 / B3 / B4 / B5 / B6 / B9-internal pays off.  "
            "Top 15 by failure count:\n"
        )
        for r in mixed[:15]:
            lines.append(
                f"- **`{r.stem}`** — {r.failed}/{r.total} failed, "
                f"first failing: `{r.failed_ids[0] if r.failed_ids else '?'}`"
            )
        lines.append("")

    # Hand-off note
    lines.append("### Permanent hand-off rules")
    lines.append("")
    lines.append(
        "Read `tests/baselines/tcl9-tcltest-vm/README.md` first — it is "
        "the durable hand-off; the section below is a running summary.\n"
        "\n"
        "- **Never** edit `tmp/tcl9.0.3/library/tcltest/tcltest.tcl`, "
        "`tmp/tcl9.0.3/library/init.tcl`, or any `.test` file under "
        "`tmp/tcl9.0.3/tests/`.  Fix the compiler / runtime instead.\n"
        "- **No new monkey-patches.**  The existing `auto_load` / "
        "`parray` shim in `vm/commands/tcltest_cmds.py:583` "
        "(`_setup_real_tcltest`) is a debug breadcrumb that **must be "
        "removed** before this slice ships — it is not a pattern to "
        "extend.  The fix is to make our auto-load path through "
        "`init.tcl`'s `tclIndex` work end-to-end.\n"
        "- **Don't bypass `catch`.**  Every B0-host-exception crash "
        "below is a Python exception escaping past a Tcl-level `catch`. "
        " The fix is always to convert the host exception to "
        "`TclError` at the boundary, never to widen the harness's "
        "exception filter.\n"
        "- Add a focused pytest in `tests/test_vm_<area>_test.py` for "
        "every contract you fix.  Pin both the success path and the "
        "host-exception → `TclError` conversion path.\n"
        "- Re-run `make test-tcl9-vm-core` after each fix; "
        "`tests/test_vm_tcl9_core_baseline.py` will pass only if no "
        "stem regresses against "
        "`tests/baselines/tcl9-tcltest-vm/summary.json`.\n"
        "- Tests classified `B9-internal` (bytecode disassembly, "
        "object-rep refcount probes, `info frame` line tables, "
        "`info cmdcount`) are **incompatible by design** and must "
        "never be reclassified as bugs.\n"
    )

    # Top crash messages
    crashes = [r for r in results if r.crashed]
    if crashes:
        lines.append("## Bootstrap / crash details")
        lines.append("")
        for r in crashes:
            lines.append(f"- **`{r.stem}`** — `{r.crash_type}`: {r.crash_msg}")
            if r.missing_commands:
                lines.append(f"    - missing command(s): `{', '.join(r.missing_commands)}`")
        lines.append("")

    # Top missing commands
    miss: Counter[str] = Counter()
    for r in results:
        for c in r.missing_commands:
            miss[c] += 1
    if miss:
        lines.append("## Top missing commands (bootstrap blockers)")
        lines.append("")
        lines.append("| command | stems blocked |")
        lines.append("|---------|--------------:|")
        for c, n in miss.most_common(25):
            lines.append(f"| `{c}` | {n} |")
        lines.append("")

    # Per-stem first failing IDs
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
        b9_internal: list[str] = []
        plain_failing: list[str] = []
        for fid in r.failed_ids:
            if _is_b9_internal(fid, r.crash_msg):
                b9_internal.append(fid)
            else:
                plain_failing.append(fid)

        body: list[str] = []
        body.append(f"# {r.stem}.test — auto-generated baseline.\n")
        body.append(f"# bucket = {r.bucket!r}\n")
        if r.bucket_reason:
            body.append(f"# reason = {r.bucket_reason!r}\n")
        body.append(f"trap_allowed = {'true' if r.crashed else 'false'}\n")
        body.append("good_to_have = []\n")
        body.append("just_to_match_ctcl = " + _toml_list(b9_internal) + "\n")
        body.append("skip = []\n\n")
        body.append("[baseline]\n")
        body.append("good_to_have_failing = 0\n")
        body.append(f"just_to_match_ctcl_failing = {len(b9_internal)}\n")
        body.append("skip_failing = 0\n")
        body.append(f"observed_total = {r.total}\n")
        body.append(f"observed_passed = {r.passed}\n")
        body.append(f"observed_failed = {r.failed}\n")
        body.append(f"observed_skipped = {r.skipped}\n")
        if plain_failing:
            body.append("\n[failing]\n")
            body.append("# Test IDs failing today — kept here for visibility, not gating.\n")
            body.append("ids = " + _toml_list(plain_failing) + "\n")

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
        default=max(1, min(4, (os.cpu_count() or 2))),
        help="Worker pool size (default: min(4, cpu_count))",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=60.0,
        help="Per-stem wall-clock timeout in seconds (default: 60)",
    )
    parser.add_argument(
        "--no-baseline",
        action="store_true",
        help="Don't write tests/baselines/tcl9-tcltest-vm/",
    )
    parser.add_argument(
        "--refresh-baseline",
        action="store_true",
        help="Rewrite per-stem TOML baselines even if they already exist.",
    )
    args = parser.parse_args()

    stems = args.stems if args.stems else CORE_STEMS
    missing = [s for s in stems if not (TESTS_DIR / f"{s}.test").is_file()]
    if missing:
        print(f"error: missing test files: {missing}", file=sys.stderr)
        return 2

    if args.refresh_baseline and BASELINE_CATEGORIES_DIR.exists():
        for p in BASELINE_CATEGORIES_DIR.glob("*.toml"):
            p.unlink()

    started = time.monotonic()
    results = run_sweep(stems, workers=args.workers, timeout=args.timeout)
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
