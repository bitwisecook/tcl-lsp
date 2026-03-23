"""Differential fuzzing harness.

Runs each generated Tcl script through multiple backends and compares
the results:

  1. Our VM — unoptimised
  2. Our VM — optimised
  3. Our parser (tokenise round-trip)
  4. C tclsh (if available in PATH)

Any mismatch in output, return code, or error/no-error status is flagged
as a finding.
"""

from __future__ import annotations

import io
import os
import shutil
import subprocess
import tempfile
import threading
from dataclasses import dataclass, field


@dataclass
class RunResult:
    """Outcome of running a Tcl script through one backend."""

    stdout: str
    stderr: str
    return_code: int  # 0 = ok, 1 = Tcl error, 2 = crash/timeout
    error_message: str | None = None

    @property
    def ok(self) -> bool:
        return self.return_code == 0

    @property
    def normalised_stdout(self) -> str:
        """Stdout with trailing whitespace stripped per line."""
        return "\n".join(line.rstrip() for line in self.stdout.splitlines())


@dataclass
class Mismatch:
    """A difference found between backends."""

    kind: str  # "output", "error_status", "crash"
    description: str
    backend_a: str
    backend_b: str
    detail_a: str
    detail_b: str


@dataclass
class FuzzResult:
    """Result of running one script through the differential harness."""

    seed: int | None
    script: str
    results: dict[str, RunResult] = field(default_factory=dict)
    mismatches: list[Mismatch] = field(default_factory=list)
    parse_error: str | None = None
    # True when the input was intentionally corrupted.  For bad inputs
    # only crashes are considered real findings — error-status and output
    # mismatches are expected.
    bad_input: bool = False

    @property
    def ok(self) -> bool:
        if self.bad_input:
            # For bad inputs only crashes matter
            return not any(m.kind == "crash" for m in self.mismatches) and (
                self.parse_error is None or "CRASH" not in self.parse_error
            )
        return len(self.mismatches) == 0 and self.parse_error is None


# VM runner


def _run_vm(script: str, *, optimise: bool = False, timeout: float = 5.0) -> RunResult:
    """Run script through our Python VM.

    Uses a daemon thread with *timeout* seconds to prevent infinite loops
    from hanging the test suite.
    """
    from vm.interp import TclInterp
    from vm.types import TclError, TclReturn

    result_box: list[RunResult] = []
    stdout_buf = io.StringIO()
    stderr_buf = io.StringIO()

    def _target() -> None:
        try:
            interp = TclInterp(optimise=optimise, source_init=False)
            interp.channels["stdout"] = stdout_buf
            interp.channels["stderr"] = stderr_buf
            interp.eval(script)
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=0,
                )
            )
        except TclError as e:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=1,
                    error_message=str(e.message) if hasattr(e, "message") else str(e),
                )
            )
        except TclReturn:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=0,
                )
            )
        except (SystemExit, KeyboardInterrupt):
            raise
        except Exception as e:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=2,
                    error_message=f"CRASH: {type(e).__name__}: {e}",
                )
            )

    t = threading.Thread(target=_target, daemon=True)
    t.start()
    t.join(timeout)

    if t.is_alive():
        # Thread is stuck — return a timeout result.  The daemon thread
        # will be reaped when the process exits.
        return RunResult(
            stdout="",
            stderr="",
            return_code=2,
            error_message="TIMEOUT",
        )

    if result_box:
        return result_box[0]

    # Should not happen, but handle defensively
    return RunResult(stdout="", stderr="", return_code=2, error_message="CRASH: no result")


# Parser checker


def _check_parse(script: str) -> str | None:
    """Try to tokenise the script; return error message or None."""
    from core.parsing.lexer import TclLexer, TclParseError

    try:
        lexer = TclLexer(script)
        lexer.tokenise_all()
        return None
    except TclParseError as e:
        return str(e)
    except Exception as e:
        return f"CRASH: {type(e).__name__}: {e}"


# C tclsh runner

_TCLSH_NAMES = ["tclsh9.0", "tclsh8.6", "tclsh8.5", "tclsh"]


def find_tclsh() -> str | None:
    """Find a tclsh in PATH, preferring newer versions."""
    for name in _TCLSH_NAMES:
        if shutil.which(name):
            return name
    return None


def _run_tclsh(script: str, tclsh: str, *, timeout: float = 10.0) -> RunResult:
    """Run script through C tclsh."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".tcl", delete=False) as f:
        f.write(script)
        f.flush()
        tmp_path = f.name

    try:
        proc = subprocess.run(
            [tclsh, tmp_path],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        error_msg = proc.stderr.strip() if proc.returncode != 0 else None
        return RunResult(
            stdout=proc.stdout,
            stderr=proc.stderr,
            return_code=min(proc.returncode, 2),
            error_message=error_msg,
        )
    except subprocess.TimeoutExpired:
        return RunResult(
            stdout="",
            stderr="",
            return_code=2,
            error_message="TIMEOUT",
        )
    finally:
        os.unlink(tmp_path)


# Comparison logic


def _compare_results(
    results: dict[str, RunResult],
) -> list[Mismatch]:
    """Compare all backend results pairwise and return mismatches."""
    mismatches: list[Mismatch] = []
    backends = list(results.keys())

    for i, a_name in enumerate(backends):
        for b_name in backends[i + 1 :]:
            a = results[a_name]
            b = results[b_name]

            # Both crashed — surface unless both are TIMEOUT (not a bug)
            if a.return_code == 2 and b.return_code == 2:
                if a.error_message == "TIMEOUT" and b.error_message == "TIMEOUT":
                    continue  # Skip timeout-vs-timeout — not a VM bug
                mismatches.append(
                    Mismatch(
                        kind="crash",
                        description=f"both {a_name} and {b_name} crashed",
                        backend_a=a_name,
                        backend_b=b_name,
                        detail_a=a.error_message or "(no error)",
                        detail_b=b.error_message or "(no error)",
                    )
                )
                continue

            # One crashed, one didn't — that's interesting
            if a.return_code == 2 or b.return_code == 2:
                crashed = a_name if a.return_code == 2 else b_name
                other = b_name if a.return_code == 2 else a_name
                mismatches.append(
                    Mismatch(
                        kind="crash",
                        description=f"{crashed} crashed but {other} did not",
                        backend_a=a_name,
                        backend_b=b_name,
                        detail_a=a.error_message or "(no error)",
                        detail_b=b.error_message or "(no error)",
                    )
                )
                continue

            # Error status mismatch
            if a.ok != b.ok:
                mismatches.append(
                    Mismatch(
                        kind="error_status",
                        description=f"{a_name} {'ok' if a.ok else 'error'} vs {b_name} {'ok' if b.ok else 'error'}",
                        backend_a=a_name,
                        backend_b=b_name,
                        detail_a=a.error_message or "(ok)",
                        detail_b=b.error_message or "(ok)",
                    )
                )

            # Output mismatch (only when both succeeded)
            if a.ok and b.ok:
                if a.normalised_stdout != b.normalised_stdout:
                    mismatches.append(
                        Mismatch(
                            kind="output",
                            description=f"stdout differs between {a_name} and {b_name}",
                            backend_a=a_name,
                            backend_b=b_name,
                            detail_a=a.normalised_stdout[:500],
                            detail_b=b.normalised_stdout[:500],
                        )
                    )

    return mismatches


# Main harness


def run_differential(
    script: str,
    *,
    seed: int | None = None,
    use_tclsh: bool = True,
    tclsh_path: str | None = None,
    bad_input: bool = False,
) -> FuzzResult:
    """Run a script through all backends and compare results.

    Parameters
    ----------
    script:
        The Tcl source to test.
    seed:
        The generator seed (for reproducibility tracking).
    use_tclsh:
        Whether to also run against C tclsh.
    tclsh_path:
        Explicit path to tclsh; auto-detected if None.
    bad_input:
        Mark this script as intentionally corrupted so only crashes
        (not error-status mismatches) are flagged.
    """
    result = FuzzResult(seed=seed, script=script, bad_input=bad_input)

    # 1. Parse check
    parse_err = _check_parse(script)
    if parse_err:
        result.parse_error = parse_err
        # Still try to run — the VM might handle it

    # 2. Our VM — unoptimised
    result.results["vm"] = _run_vm(script, optimise=False)

    # 3. Our VM — optimised
    result.results["vm_opt"] = _run_vm(script, optimise=True)

    # 4. C tclsh (if available)
    if use_tclsh:
        tclsh = tclsh_path or find_tclsh()
        if tclsh:
            result.results[f"tclsh({tclsh})"] = _run_tclsh(script, tclsh)

    # 5. Compare
    result.mismatches = _compare_results(result.results)

    return result


def format_finding(result: FuzzResult) -> str:
    """Format a FuzzResult with mismatches as a human-readable report."""
    lines = [
        f"{'=' * 60}",
        f"FUZZ FINDING — seed={result.seed}",
        f"{'=' * 60}",
    ]

    if result.parse_error:
        lines.append(f"PARSE ERROR: {result.parse_error}")

    for m in result.mismatches:
        lines.append(f"\n--- {m.kind}: {m.description} ---")
        lines.append(f"  [{m.backend_a}]: {m.detail_a[:200]}")
        lines.append(f"  [{m.backend_b}]: {m.detail_b[:200]}")

    lines.append(f"\n--- Script ({len(result.script)} chars) ---")
    lines.append(result.script[:2000])

    return "\n".join(lines)
