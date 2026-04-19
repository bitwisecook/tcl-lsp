"""Tcl 9 tcltest suite runner via WASM compilation.

Uses the real Tcl 9 tcltest package from ``tmp/tcl9.0.3/library/tcltest/``
and runs individual test files from ``tmp/tcl9.0.3/tests/``.  Each test
file is bundled with tcltest.tcl into a single compilation unit, compiled
to WASM, and executed via wasmtime.

Stage structure (mirrors run_tcllib_test.py):

  1. ``test_tcltest9_compiles``  — Tcl 9 tcltest.tcl alone compiles to WASM
  2. ``test_tcltest9_top_runs``  — top-level init executes without trapping
  3. ``test_<name>_compiles``    — tcltest + test-file bundle compiles
  4. ``test_<name>_runs``        — bundle executes and reports Failed == 0

Per-file results are recorded via ``record_tcl9_result`` (see
``tests/external/conftest.py``).  When pytest is run with
``--tcl9-report=<path>``, the collection is flushed to JSON at session
finish for the triage report generator.

Known accepted limitations
--------------------------
- Tests that require the C extension ``tcl::test`` (testbytestring, etc.)
  are skipped by the test file's own ``testConstraint`` calls — correct.
- Filesystem-heavy tests (makeFile, cd-based) need a preopen'd tmpdir.
- Some Tcl 9-specific commands may trap if not yet implemented; those
  tests will show up as XFAIL until the implementation catches up.
"""

from __future__ import annotations

import re
import tempfile
from collections import Counter
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from tests.conftest import ensure_tcl_source, record_tcl9_result
from tests.test_wasm_real_tcl import (
    _compile_tcl,
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

if TYPE_CHECKING:
    from core.compiler.codegen.wasm._ir import DiagMap

_EXTERNAL = Path(__file__).resolve().parent

# ---------------------------------------------------------------------------
# Resolve Tcl 9 source paths (lazy — skip if not available)
# ---------------------------------------------------------------------------


def _tcl9_tcltest() -> Path:
    """Return path to Tcl 9 tcltest.tcl, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir.parent / "library" / "tcltest" / "tcltest.tcl"
    if not p.exists():
        pytest.skip(f"Tcl 9 tcltest.tcl not found at {p}")
    return p


def _tcl9_test_file(name: str) -> Path:
    """Return path to a Tcl 9 test file, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir / name
    if not p.exists():
        pytest.skip(f"Tcl 9 test file not found: {p}")
    return p


# ---------------------------------------------------------------------------
# Bundle construction
# ---------------------------------------------------------------------------

_PREAMBLE = r"""
# ----- run_tcl9_tests.py preamble -----
# Import tcltest commands into the global namespace.  Test files that detect
# ::tcltest already loaded skip their own namespace import block; we fill the
# gap here so unqualified 'test', 'testConstraint', etc. resolve correctly.
namespace import -force ::tcltest::*

# Silence per-test progress chatter; keep the summary line.
::tcltest::configure -verbose {}
"""


def _bundle(test_file_path: Path) -> str:
    """Concatenate Tcl 9 tcltest + preamble + test file into one script."""
    tcltest_src = _tcl9_tcltest().read_text(encoding="utf-8")
    test_src = test_file_path.read_text(encoding="utf-8")

    return "\n".join(
        [
            "# ===== Tcl 9 tcltest (tmp/tcl9.0.3/library/tcltest/tcltest.tcl) =====",
            tcltest_src,
            "# ===== run_tcl9_tests preamble =====",
            _PREAMBLE,
            f"# ===== {test_file_path.name} =====",
            test_src,
            "",
        ]
    )


_SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)")
_FAIL_RE = re.compile(r"^==== (\S+) FAILED", re.MULTILINE)


def _parse_summary(stdout: str) -> tuple[int, int, int, int] | None:
    """Extract (total, passed, skipped, failed) from a tcltest summary line."""
    m = _SUMMARY_RE.search(stdout)
    if m is None:
        return None

    def _int(s: str) -> int:
        return int(s) if s else 0

    return (_int(m.group(1)), _int(m.group(2)), _int(m.group(3)), _int(m.group(4)))


def _first_failing(stdout: str) -> str | None:
    """Extract the name of the first failing tcltest test from stdout."""
    m = _FAIL_RE.search(stdout)
    return m.group(1) if m else None


def _stderr_tail(stderr: str, max_chars: int = 400) -> str:
    """Return the last ``max_chars`` characters of stderr for triage."""
    if len(stderr) <= max_chars:
        return stderr
    return "…" + stderr[-max_chars:]


def _summarise_diag(diag: "DiagMap | None") -> dict:
    """Aggregate fallback-site telemetry from a populated :class:`DiagMap`.

    Returns a dict with three fields the triage harness records alongside
    per-file pass/fail data:

    * ``fallback_sites_total`` — total number of diag sites registered
      (every ``tcl_eval`` fallback, unsupported-command trap, or unknown
      dispatch emits one).
    * ``fallback_sites_by_kind`` — counts bucketed by ``DiagSite.kind``
      (e.g. ``fallback``, ``unsupported``, ``unknown``, ``runtime``).
    * ``top_fallback_commands`` — the five most common commands that
      triggered a fallback, as ``(command, count)`` pairs.  Only sites
      with ``kind == "fallback"`` are counted; unsupported/unknown sites
      are architectural dead-ends and show up in the kind histogram.

    Returns all zeroes / empty when ``diag`` is ``None`` (compile never
    produced a map) so callers can unconditionally merge the result into
    the per-file report dict.
    """
    if diag is None or not diag.sites:
        return {
            "fallback_sites_total": 0,
            "fallback_sites_by_kind": {},
            "top_fallback_commands": [],
        }
    by_kind: Counter[str] = Counter()
    by_cmd: Counter[str] = Counter()
    for site in diag.sites:
        by_kind[site.kind] += 1
        if site.kind == "fallback" and site.command:
            by_cmd[site.command] += 1
    return {
        "fallback_sites_total": len(diag.sites),
        "fallback_sites_by_kind": dict(by_kind),
        "top_fallback_commands": by_cmd.most_common(5),
    }


def _infer_category(
    *,
    compiled: bool,
    ran: bool,
    summary: tuple[int, int, int, int] | None,
    trap_site: str | None,
    stderr: str,
    deferred: bool,
) -> str:
    """Best-effort triage category inference.

    * D — test file is on the deferred-by-design list.
    * E — tcltest-viability regression or missing-command plumbing.
    * B — compile failed or runtime trap before any tcltest result.
    * A — ran to completion but reported failing tests.
    * pass — all tcltest assertions passed.
    """
    if deferred:
        return "D"
    if not compiled:
        return "B"
    if not ran:
        if trap_site:
            return "B"
        return "E"
    if summary is None:
        return "E"
    _total, _passed, _skipped, failed = summary
    if failed == 0:
        return "pass"
    return "A"


def _run_bundle(bundle_src: str, label: str) -> tuple[str, str]:
    """Compile and run a bundle; return (stdout, stderr).

    Raises AssertionError on compile or runtime failure, with a
    human-readable diagnostic that includes the trap site if available.
    """
    try:
        wasm, diag = _compile_tcl_with_diag(bundle_src, label)
    except Exception as exc:
        pytest.fail(f"{label}: compilation failed: {exc}")

    with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
        try:
            result = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=host_tmp,
            )
        except Exception as trap:
            pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))

    stdout = result[1] if len(result) >= 2 else ""
    stderr = result[2] if len(result) >= 3 else ""
    return stdout, stderr


# ---------------------------------------------------------------------------
# Stage 1 & 2: tcltest.tcl alone (viability gate)
# ---------------------------------------------------------------------------


class TestTcltest9Init:
    """Stages 1–2: Tcl 9 tcltest.tcl compiles and its top-level runs.

    This is the viability gate: if ``tcltest.tcl`` itself cannot compile
    or its top-level cannot run, every downstream test file is poisoned
    and the per-file results are meaningless.
    """

    def test_tcltest9_compiles(self, request: pytest.FixtureRequest) -> None:
        """Tcl 9 tcltest.tcl compiles to WASM without errors."""
        src = _tcl9_tcltest().read_text(encoding="utf-8")
        try:
            _compile_tcl(src)
        except Exception as exc:
            record_tcl9_result(
                request.config,
                {
                    "file": "tcltest.tcl",
                    "stage": "compile",
                    "status": "fail",
                    "trap_site": None,
                    "stderr_tail": str(exc)[-400:],
                    "category": "E",
                    "notes": "tcltest.tcl itself does not compile",
                },
            )
            pytest.fail(f"Tcl 9 tcltest.tcl failed to compile: {exc}")
        record_tcl9_result(
            request.config,
            {
                "file": "tcltest.tcl",
                "stage": "compile",
                "status": "pass",
                "category": "pass",
            },
        )

    def test_tcltest9_top_runs(self, request: pytest.FixtureRequest) -> None:
        """Tcl 9 tcltest.tcl top-level init executes without trapping."""
        src = _tcl9_tcltest().read_text(encoding="utf-8")
        try:
            wasm, diag = _compile_tcl_with_diag(src, "tcl9_tcltest.tcl")
        except Exception as exc:
            pytest.skip(f"Tcl 9 tcltest.tcl does not yet compile: {exc}")

        with tempfile.TemporaryDirectory(prefix="tcl9test-init-") as host_tmp:
            try:
                result = _run_wasm(wasm, capture_stderr=True, preopen_tmpdir=host_tmp)
            except Exception as trap:
                stderr_text = getattr(trap, "tcl_stderr", "")
                record_tcl9_result(
                    request.config,
                    {
                        "file": "tcltest.tcl",
                        "stage": "run",
                        "status": "fail",
                        "trap_site": _resolve_trap(trap, stderr_text, diag),
                        "stderr_tail": _stderr_tail(stderr_text),
                        "category": "E",
                        "notes": "tcltest.tcl init trapped",
                    },
                )
                pytest.fail(_resolve_trap(trap, stderr_text, diag))

        val = result[0]
        stderr_text = result[2] if len(result) >= 3 else ""
        status = "pass" if val == 0 else "fail"
        record_tcl9_result(
            request.config,
            {
                "file": "tcltest.tcl",
                "stage": "run",
                "status": status,
                "return_value": val,
                "stderr_tail": _stderr_tail(stderr_text),
                "category": "pass" if status == "pass" else "E",
            },
        )
        assert val == 0, f"::top returned {val}; stderr:\n{stderr_text}"


# ---------------------------------------------------------------------------
# Stage 3 & 4: individual Tcl 9 test files
# ---------------------------------------------------------------------------


def _make_test_class(test_name: str, *, subsystem: str, deferred: bool = False):
    """Dynamically build a test class for a Tcl 9 .test file.

    ``test_name`` is the stem of the file (e.g. ``"append"`` for
    ``append.test``).  ``subsystem`` is the inventory-KCS subsystem
    label; ``deferred`` marks files that exercise deferred-by-design
    primitives (I/O, threads, fs, encoding) and pre-categorises them
    as D.
    """
    filename = f"{test_name}.test"

    class _TestClass:
        def test_compiles(self, request: pytest.FixtureRequest) -> None:
            """Tcl 9 test-file bundle compiles to WASM."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)
            try:
                _wasm, diag = _compile_tcl_with_diag(src, filename)
            except Exception as exc:
                record_tcl9_result(
                    request.config,
                    {
                        "file": filename,
                        "subsystem": subsystem,
                        "stage": "compile",
                        "status": "fail",
                        "stderr_tail": str(exc)[-400:],
                        "category": "D" if deferred else "B",
                    },
                )
                pytest.fail(f"{filename} bundle failed to compile: {exc}")
            record_tcl9_result(
                request.config,
                {
                    "file": filename,
                    "subsystem": subsystem,
                    "stage": "compile",
                    "status": "pass",
                    "category": "pass",
                    **_summarise_diag(diag),
                },
            )

        def test_runs(self, request: pytest.FixtureRequest) -> None:
            """Tcl 9 test-file bundle executes and reports Failed == 0."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)

            trap_site: str | None = None
            compiled = False
            ran = False
            stdout = ""
            stderr = ""
            summary: tuple[int, int, int, int] | None = None
            diag = None

            try:
                wasm, diag = _compile_tcl_with_diag(src, filename)
                compiled = True
                with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
                    result = _run_wasm(
                        wasm,
                        capture_stdout=True,
                        capture_stderr=True,
                        preopen_tmpdir=host_tmp,
                    )
                ran = True
                stdout = result[1] if len(result) >= 2 else ""
                stderr = result[2] if len(result) >= 3 else ""
                summary = _parse_summary(stdout)
            except Exception as exc:
                if compiled and "wasm" in locals():
                    trap_site = _resolve_trap(exc, getattr(exc, "tcl_stderr", ""), diag)
                    stderr = getattr(exc, "tcl_stderr", "")
                else:
                    stderr = str(exc)

            category = _infer_category(
                compiled=compiled,
                ran=ran,
                summary=summary,
                trap_site=trap_site,
                stderr=stderr,
                deferred=deferred,
            )
            total, passed, skipped, failed = summary or (0, 0, 0, 0)
            record_tcl9_result(
                request.config,
                {
                    "file": filename,
                    "subsystem": subsystem,
                    "stage": "run",
                    "status": "pass" if category == "pass" else "fail",
                    "total": total,
                    "passed": passed,
                    "skipped": skipped,
                    "failed": failed,
                    "first_failing_test": _first_failing(stdout),
                    "trap_site": trap_site,
                    "stderr_tail": _stderr_tail(stderr),
                    "category": category,
                    **_summarise_diag(diag),
                },
            )

            if not compiled:
                pytest.fail(f"{filename} bundle failed to compile: {stderr[-400:]}")
            if not ran:
                pytest.fail(f"{filename} trapped: {trap_site or stderr[-400:]}")
            if summary is None:
                pytest.fail(
                    f"No tcltest summary line in stdout for {filename}.\n"
                    f"stdout tail:\n{stdout[-400:]}\nstderr tail:\n{stderr[-400:]}"
                )
            assert failed == 0, (
                f"{filename}: {failed} test(s) FAILED "
                f"(total={total}, passed={passed}, skipped={skipped}).\n"
                f"first failing: {_first_failing(stdout)}\n"
                f"stdout tail:\n{stdout[-400:]}"
            )

    _TestClass.__name__ = f"TestTcl9_{test_name.replace('-', '_').replace('.', '_')}"
    _TestClass.__qualname__ = _TestClass.__name__
    return _TestClass


# ---------------------------------------------------------------------------
# Register test classes.
#
# In-scope files (core Tcl semantics). Deferred-by-design files — I/O,
# sockets, threads, fs, encoding, platform-specific — are NOT registered
# here; they are recorded only in the inventory KCS and triaged as
# category D without being executed.
# ---------------------------------------------------------------------------

_IN_SCOPE: list[tuple[str, str]] = [
    # parsing
    ("parse", "parsing"),
    ("parseOld", "parsing"),
    ("parseExpr", "parsing"),
    ("subst", "parsing"),
    ("word", "parsing"),
    # list
    ("list", "list"),
    ("listObj", "list"),
    ("listRep", "list"),
    ("llength", "list"),
    ("lindex", "list"),
    ("linsert", "list"),
    ("lrange", "list"),
    ("lreplace", "list"),
    ("lsearch", "list"),
    ("lset", "list"),
    ("lsetComp", "list"),
    ("lmap", "list"),
    ("lpop", "list"),
    ("lseq", "list"),
    ("lrepeat", "list"),
    ("foreach", "list"),
    ("abstractlist", "list"),
    # dict
    ("dict", "dict"),
    # string
    ("string", "string"),
    ("stringObj", "string"),
    ("format", "string"),
    ("scan", "string"),
    ("regexp", "string"),
    ("regexpComp", "string"),
    ("reg", "string"),
    ("get", "string"),
    ("split", "string"),
    ("join", "string"),
    # expr
    ("expr", "expr"),
    ("expr-old", "expr"),
    ("compExpr", "expr"),
    ("compExpr-old", "expr"),
    ("mathop", "expr"),
    # control
    ("if", "control"),
    ("if-old", "control"),
    ("for", "control"),
    ("for-old", "control"),
    ("while", "control"),
    ("while-old", "control"),
    ("switch", "control"),
    ("error", "control"),
    ("result", "control"),
    # variables / scopes
    ("set", "variable"),
    ("set-old", "variable"),
    ("var", "variable"),
    ("upvar", "variable"),
    ("uplevel", "variable"),
    ("namespace", "variable"),
    ("namespace-old", "variable"),
    ("trace", "variable"),
    ("resolver", "variable"),
    # proc / apply / info
    ("proc", "proc"),
    ("proc-old", "proc"),
    ("apply", "proc"),
    ("info", "proc"),
    ("cmdInfo", "proc"),
    ("rename", "proc"),
    ("unknown", "proc"),
    # eval / subst / execution
    ("eval", "eval"),
    ("compile", "eval"),
    ("execute", "eval"),
    ("basic", "eval"),
    # command dispatch buckets
    ("cmdAH", "cmd-dispatch"),
    ("cmdIL", "cmd-dispatch"),
    ("cmdMZ", "cmd-dispatch"),
    # TclOO
    ("oo", "object"),
    ("ooNext2", "object"),
    ("ooProp", "object"),
    ("ooUtil", "object"),
    # coroutine / nre / tailcall
    ("coroutine", "coroutine"),
    ("nre", "coroutine"),
    ("tailcall", "coroutine"),
    # interp / safe / source
    ("interp", "interp"),
    ("safe", "interp"),
    ("safe-stock", "interp"),
    ("safe-stock86", "interp"),
    ("source", "interp"),
    # misc scalar / object machinery
    ("append", "misc"),
    ("appendComp", "misc"),
    ("concat", "misc"),
    ("incr", "misc"),
    ("incr-old", "misc"),
    ("obj", "misc"),
    ("indexObj", "misc"),
    ("dstring", "misc"),
    ("assocd", "misc"),
    ("opt", "misc"),
    ("stack", "misc"),
    ("misc", "misc"),
    ("brodnik", "misc"),
    ("range", "misc"),
    ("aaa_exit", "misc"),
]

for _stem, _sub in _IN_SCOPE:
    _cls = _make_test_class(_stem, subsystem=_sub, deferred=False)
    globals()[_cls.__name__] = _cls
