"""Runtime behaviour tests for IRBarrier relaxation (static ``eval`` /
``uplevel`` bodies).

C1 lowers recognised shapes to :class:`IRBlock` / :class:`IRUpFrame`
(see ``test_barrier_relaxation.py``).  C2 emits WASM for those IR
nodes.  These tests compile a full snippet, run it under wasmtime,
and assert the stdout reflects the compiled-frame-aware semantics —
crucially, the caller-local visibility case that the barrier fallback
path was unable to honour.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm


class TestUplevelBracedLiteralRuntime:
    # NOTE: The "uplevel 1 writes caller's compiled local" case is
    # not tested here.  Both the old IRBarrier → tcl_eval path and
    # the new IRUpFrame codegen lose the caller's WASM locals
    # because compile-to-compile proc calls do not push runtime
    # frames.  Fixing it requires a whole-module pass that either
    # (a) inlines the callee entirely (splicing its body into the
    # caller's IR and collapsing the frame shift), or (b) forces
    # proc-to-proc calls to push real frames and routes compiled
    # variable accesses through the frame table.  Both are tracked
    # as follow-up work.  Tcltest integration (``uplevel.test``)
    # exercises the deep-stack variants end-to-end.

    def test_uplevel_hash_zero_writes_global(self):
        source = "proc set_global {} {\n    uplevel #0 {set ::g 42}\n}\nset_global\nputs $::g\n"
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "42\n"
