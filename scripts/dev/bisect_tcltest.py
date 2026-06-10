#!/usr/bin/env python3
"""Bisect a tcltest file to find the test that hangs / takes too long.

Wraps the test bundle with a ``rename test`` shim that prints
``RUN <test-name>`` to stderr before each test runs, then runs the
bundle with a short timeout.  Whatever was the LAST printed name
is the suspect.
"""

import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

from tests.external.run_tcl9_tests import _bundle  # noqa: E402
from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm  # noqa: E402

_PROBE = r"""
# ----- bisect_tcltest.py probe -----
puts stderr "PROBE: reached top of test-file body"
flush stderr
# Trace each ``test`` invocation to stderr so we can identify which
# test is running when the watchdog fires.  The tcltest preamble
# imports ``::tcltest::*`` into the global namespace, so we shim the
# global ``test`` proc.
rename test ::__orig_test
proc test {name args} {
    puts stderr "RUN $name"
    return [uplevel 1 [list ::__orig_test $name {*}$args]]
}
"""


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: bisect_tcltest.py <stem.test> [timeout_s]")
    stem = sys.argv[1]
    timeout = float(sys.argv[2]) if len(sys.argv) > 2 else 8.0
    test_path = REPO / "tmp" / "tcl9.0.3" / "tests" / stem
    if not test_path.exists():
        sys.exit(f"missing: {test_path}")

    bundle = _bundle(test_path)
    # Inject the probe just before the test file body.  The bundle
    # is laid out: pre-tcltest | tcltest.tcl | preamble | test file.
    marker = f"# ===== {test_path.name} ====="
    if marker in bundle:
        bundle = bundle.replace(marker, marker + "\n" + _PROBE, 1)
    else:
        bundle = bundle + "\n" + _PROBE

    wasm, _ = _compile_tcl_with_diag(bundle, stem)
    print(f"# wasm bytes: {len(wasm)}", file=sys.stderr)

    with tempfile.TemporaryDirectory() as pre:
        try:
            ret = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=pre,
                timeout_s=timeout,
            )
            stdout = ret[1]
            stderr = ret[2]
            tail = stdout.strip().splitlines()[-3:]
            print("# WASM completed normally")
            for ln in tail:
                print("#  ", ln[:120])
            run_lines = [ln for ln in stderr.splitlines() if ln.startswith("RUN ")]
            print(f"# {len(run_lines)} tests dispatched")
            if run_lines:
                print(f"# last: {run_lines[-1]}")
        except BaseException as e:
            stderr = getattr(e, "tcl_stderr", "") or ""
            run_lines = [ln for ln in stderr.splitlines() if ln.startswith("RUN ")]
            print(f"# WASM TRAPPED/TIMED OUT after {len(run_lines)} tests")
            if run_lines:
                last = run_lines[-1]
                print(f"# LAST RUN: {last}")
                # Show the prior 3 too
                for ln in run_lines[-4:-1]:
                    print(f"#   prev: {ln}")
            else:
                print("# (no test had a chance to run)")


if __name__ == "__main__":
    main()
