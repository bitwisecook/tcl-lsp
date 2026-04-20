"""Tests for whole-callee ``uplevel``-passthrough inlining.

Verifies both the IR-level rewrite (callsites collapse to
:class:`IRBlock` wrapping the callee's body) and the runtime
observation (compiled code behaves identically to pre-inlining).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.compilation_unit import compile_source
from core.compiler.inline_uplevel import inline_uplevel_passthrough
from core.compiler.ir import IRBlock, IRCall, IRUpFrame
from core.compiler.lowering import lower_to_ir


def _ir_module(source: str):
    mod = lower_to_ir(source)
    inline_uplevel_passthrough(mod)
    return mod


class TestPassthroughRecognition:
    def test_zero_param_single_uplevel_is_candidate(self):
        mod = _ir_module("proc reset {} { uplevel 1 {set counter 0} }\nproc caller {} { reset }\n")
        caller = mod.procedures["::caller"]
        # Callsite should be an IRBlock with the spliced body — not
        # an IRCall to ``reset``.
        first = caller.body.statements[0]
        assert isinstance(first, IRBlock), f"expected IRBlock, got {type(first).__name__}"
        assert any(not isinstance(s, IRUpFrame) for s in first.body.statements)
        assert all(
            not isinstance(s, IRCall) or s.command != "reset" for s in caller.body.statements
        )

    def test_non_passthrough_not_inlined(self):
        # Body does real work before the uplevel — not a pure
        # passthrough, so callsite stays an IRCall.
        mod = _ir_module(
            """
            proc worker {} {
                set local 1
                uplevel 1 {set g 2}
            }
            proc caller {} { worker }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)
        assert first.command == "worker"

    def test_non_unit_frame_shift_not_inlined(self):
        # ``uplevel #0`` goes to absolute global — cannot be inlined
        # into the caller's compiled frame.
        mod = _ir_module(
            """
            proc reset {} { uplevel #0 {set ::g 0} }
            proc caller {} { reset }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall), f"expected IRCall, got {type(first).__name__}"

    def test_non_body_param_not_inlined(self):
        # ``setter`` embeds the parameter inside a quoted body — the
        # param-body gate only recognises the literal ``uplevel 1
        # $param`` shape, so this stays a runtime call.
        mod = _ir_module(
            """
            proc setter {n} { uplevel 1 "set counter $n" }
            proc caller {} { setter 5 }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_nested_uplevel_poisons_inlining(self):
        # Body itself does ``uplevel`` — after inlining it would
        # reference the caller's caller, a frame the original author
        # may not have anticipated.  Reject conservatively.
        mod = _ir_module(
            """
            proc double_up {} { uplevel 1 { uplevel 1 {set g 1} } }
            proc caller {} { double_up }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_args_at_callsite_prevents_inline(self):
        # ``reset`` is passthrough but the callsite passes an
        # unexpected arg — arity would error at runtime, don't silently
        # inline away the error.
        mod = _ir_module(
            """
            proc reset {} { uplevel 1 {set g 0} }
            proc caller {} { reset extra_arg }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)


class TestPassthroughRuntime:
    def test_counter_reset_matches_direct_mutation(self):
        # Runtime observation: calling the passthrough produces the
        # same counter value as writing ``set counter 0`` directly.
        src = """
            proc reset {} { uplevel 1 {set counter 0} }
            proc run {} {
                set counter 10
                reset
                return $counter
            }
            puts [run]
        """
        from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm

        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "0\n"


class TestInliningIdempotence:
    def test_running_twice_is_noop(self):
        mod = lower_to_ir("proc reset {} { uplevel 1 {set g 0} }\nproc caller {} { reset }\n")
        inline_uplevel_passthrough(mod)
        first_rewrite = mod.procedures["::caller"].body
        inline_uplevel_passthrough(mod)
        second_rewrite = mod.procedures["::caller"].body
        # Second pass finds no IRCall to a candidate — body should be
        # structurally identical to the first rewrite.
        assert len(first_rewrite.statements) == len(second_rewrite.statements)


class TestCompilationPipelineIntegration:
    def test_compile_source_runs_inlining(self):
        cu = compile_source("proc reset {} { uplevel 1 {set g 0} }\nproc caller {} { reset }\n")
        caller = cu.ir_module.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRBlock)


class TestParamBodyPassthrough:
    """Single-body-param passthrough: ``proc P {body} { uplevel 1 $body }``.

    The callee cannot be relaxed at lowering time (body token is
    ``$var`` against an unknown parameter value), but each call site
    that passes a braced literal *does* know the body, so we inline
    per-call.
    """

    def test_explicit_level_one_call_site_inlined(self):
        mod = _ir_module(
            """
            proc dispatcher {body} { uplevel 1 $body }
            proc caller {} { dispatcher {set x 1} }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRBlock), f"expected IRBlock, got {type(first).__name__}"

    def test_implicit_level_call_site_inlined(self):
        # ``uplevel $body`` (no explicit level) defaults to 1.
        mod = _ir_module(
            """
            proc dispatcher2 {body} { uplevel $body }
            proc caller {} { dispatcher2 {set x 1} }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRBlock)

    def test_dynamic_arg_at_call_site_stays_call(self):
        # Call site passes ``$var`` — unknown body at compile time,
        # runtime still dispatches the proc.
        mod = _ir_module(
            """
            proc dispatcher {body} { uplevel 1 $body }
            proc caller {} { set b {set x 1}; dispatcher $b }
            """
        )
        caller = mod.procedures["::caller"]
        calls = [s for s in caller.body.statements if isinstance(s, IRCall)]
        assert any(c.command == "dispatcher" for c in calls)

    def test_wrong_arity_at_call_site_stays_call(self):
        mod = _ir_module(
            """
            proc dispatcher {body} { uplevel 1 $body }
            proc caller {} { dispatcher {set x 1} extra }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_multiple_params_not_recognised(self):
        # Two params — the single-body-param gate rejects.  The
        # runtime-facing uplevel stays a barrier.
        mod = _ir_module(
            """
            proc dispatcher2 {a body} { uplevel 1 $body }
            proc caller {} { dispatcher2 x {set x 1} }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_non_param_var_reference_not_recognised(self):
        # Body is ``$other`` which is not the sole param name.  Stay
        # conservative.
        mod = _ir_module(
            """
            proc dispatcher {body} { uplevel 1 $other }
            proc caller {} { dispatcher {set x 1} }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_explicit_level_other_than_one_not_recognised(self):
        # ``uplevel 2 $body`` — deeper shift, different semantics.
        # Param-body gate explicitly checks level == 1.
        mod = _ir_module(
            """
            proc dispatcher {body} { uplevel 2 $body }
            proc caller {} { dispatcher {set x 1} }
            """
        )
        caller = mod.procedures["::caller"]
        first = caller.body.statements[0]
        assert isinstance(first, IRCall)

    def test_inlined_body_runs_in_caller_frame(self):
        # Runtime observation: the inlined body's ``set`` lands in the
        # caller's frame — same observable as writing the ``set``
        # directly inline.
        src = """
            proc dispatcher {body} { uplevel 1 $body }
            proc run {} {
                set counter 10
                dispatcher {set counter 0}
                return $counter
            }
            puts [run]
        """
        from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm

        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "0\n"
