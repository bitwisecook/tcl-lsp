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


class TestEvalListLiteralRuntime:
    def test_eval_list_literal_runs_compiled(self):
        # ``eval [list set x 42]`` — compiler synthesises the body
        # ``set x 42`` and inlines it, so the compiled proc's ``$x``
        # reads the updated value without a ``tcl_eval`` round-trip.
        src = "proc run {} {\n    eval [list set x 42]\n    return $x\n}\nputs [run]\n"
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "42\n"


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


class TestBarrierBracedBracketArg:
    """Per-arg brace info must survive the IRBarrier → eval-fallback path.

    A user proc with a variadic ``args`` tail compiles to an
    :class:`IRBarrier` (the args-tail packing defeats the
    static-call analysis).  When the codegen reassembles the call
    into a script for ``tcl_eval``, each arg's brace flag must come
    from the parsed ``CommandTokens`` — without it, an arg whose IR
    text starts with ``[`` (a braced description like
    tcltest's ``{[Bug 1234] foo}``) is mistaken for a command-
    substitution word and passed unquoted; the runtime then tries
    to invoke ``Bug 1234`` (or ``<hex>``) as a command and traps
    with ``unknown command: …``.

    Repro that previously failed::

        proc test {a b args} { puts "ok desc=$b" }
        test x {[abc123] description text} -setup foo
        # → tcl trap: unknown command: abc123

    Fix is in
    :mod:`compiler.codegen.wasm._emitter._statements`'s
    ``case IRBarrier(...)`` arm: extract ``tokens`` from the match
    and pass it to ``_emit_eval_fallback``.
    """

    def test_args_tail_proc_preserves_braced_bracket(self):
        # Hex-id form (the basic.test / namespace.test / proc.test /
        # string.test / var.test style: a fossil-tracker hash inside
        # a braced description).
        source = (
            'proc test {a b args} { puts "name=$a desc=$b args=$args" }\n'
            "test basic-50.1 {[586e71dce4] EvalObjv level} -setup foo\n"
        )
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == ("name=basic-50.1 desc=[586e71dce4] EvalObjv level args=-setup foo\n")

    def test_args_tail_proc_preserves_braced_bug_marker(self):
        # ``[Bug NNN]`` form (the compExpr.test / interp.test /
        # tailcall.test pattern).  Same root cause as the hex form
        # but exercises a description that starts with a literal
        # word inside the brackets.
        source = (
            'proc test {a b args} { puts "desc=$b" }\n'
            "test compExpr-7.2 {[Bug 1869989]: expr parser memleak} -setup foo\n"
        )
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "desc=[Bug 1869989]: expr parser memleak\n"
