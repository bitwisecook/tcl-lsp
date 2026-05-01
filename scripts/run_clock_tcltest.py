"""One-shot driver: run upstream Tcl 9 ``tests/clock.test`` against our WASM runtime.

Bundles the test file with the same scaffolding ``tests/external/run_tcl9_tests.py``
uses, runs it in wasmtime, prints the tcltest summary line, the first
five failing test names, and any trap stderr.  Designed to give a
quick "where do we stand vs upstream tcltest" answer for the clock
implementation work.
"""

from __future__ import annotations

import shutil
import sys
import tempfile
import time
import traceback
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from tests.external.run_tcl9_tests import _bundle, _parse_summary, _tcl9_tcltest  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

TEST_FILE = REPO / "tmp" / "tcl9.0.3" / "tests" / "clock.test"


def main() -> None:
    src = _bundle(TEST_FILE)
    print(f"[bundle] {len(src):,} bytes")
    t0 = time.time()
    try:
        wasm, diag = _compile_tcl_with_diag(src, "clock.test")
    except Exception as exc:
        print(f"[compile] FAILED: {exc}")
        traceback.print_exc()
        return
    print(f"[compile] {time.time() - t0:.1f}s, wasm = {len(wasm):,} bytes")
    _ = _compile_tcl  # silence import-unused

    with tempfile.TemporaryDirectory(prefix="tcl9clock-") as host_tmp:
        helpers_dir = _tcl9_tcltest().parent.parent.parent / "tests"
        for helper in ("tcltests.tcl", "internals.tcl", "encodingVectors.tcl", "pkgIndex.tcl"):
            sp = helpers_dir / helper
            if sp.exists():
                shutil.copyfile(sp, Path(host_tmp) / helper)
        # Make tzdata reachable inside the sandbox so timezone-aware
        # tests have something to resolve.  ``preopen_tmpdir`` maps
        # the host dir to guest root; we stage tzdata symlinked under
        # the same name so ``/usr/share/zoneinfo`` resolves.
        zoneinfo = Path("/usr/share/zoneinfo")
        if zoneinfo.exists():
            target = Path(host_tmp) / "usr" / "share"
            target.mkdir(parents=True, exist_ok=True)
            (target / "zoneinfo").symlink_to(zoneinfo)
        t0 = time.time()
        try:
            result = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=host_tmp,
            )
        except Exception as trap:
            stderr_text = getattr(trap, "tcl_stderr", "")
            print(f"[run] TRAPPED after {time.time() - t0:.1f}s")
            print(f"[trap] {_resolve_trap(trap, stderr_text, diag)}")
            if stderr_text:
                print("[stderr tail (last 1500)]")
                print(stderr_text[-1500:])
            return
    elapsed = time.time() - t0
    stdout = result[1] if len(result) >= 2 else ""
    stderr = result[2] if len(result) >= 3 else ""
    print(f"[run] {elapsed:.1f}s, stdout={len(stdout)} bytes, stderr={len(stderr)} bytes")

    summary = _parse_summary(stdout)
    if summary is None:
        print("[summary] no tcltest summary line found")
        print("[stdout tail]")
        print(stdout[-2000:])
        print("[stderr tail]")
        print(stderr[-1000:])
        return
    total, passed, skipped, failed = summary
    print(f"[summary] Total {total} Passed {passed} Skipped {skipped} Failed {failed}")
    if total > 0:
        pass_pct = 100.0 * passed / total
        print(f"[pass-rate] {pass_pct:.1f}% of total ({passed}/{total})")
        if total - skipped > 0:
            run_pct = 100.0 * passed / (total - skipped)
            print(f"[run-rate]  {run_pct:.1f}% of non-skipped ({passed}/{total - skipped})")

    # First few failing test names — handy for triage.
    import re

    fails = re.findall(r"^==== (\S+) FAILED", stdout, re.MULTILINE)
    if fails:
        print(f"[fails] showing first 10 of {len(fails)}:")
        for name in fails[:10]:
            print(f"  {name}")
    if stderr:
        print("[stderr tail (last 500)]")
        print(stderr[-500:])


if __name__ == "__main__":
    main()
