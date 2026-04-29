"""S4.2 v0 — empty-body splice tests."""

from __future__ import annotations

import textwrap

from core.compiler.inlining import (
    apply_inline_catalogue,
    inline_module,
)
from core.compiler.ir import IRCall, InlineDecision
from core.compiler.lowering import lower_to_ir
from core.compiler.var_escape import analyse_var_escape


def _prepare(source: str):
    """Lower + tag + return ``(module, summaries)``."""
    module = lower_to_ir(textwrap.dedent(source))
    summaries = analyse_var_escape(ir_module=module)
    tagged = apply_inline_catalogue(module, summaries)
    return tagged, summaries


def _statement_kinds(script):
    return [type(s).__name__ for s in script.statements]


def _calls_to(script, command: str) -> int:
    """Count IRCall statements whose ``command`` field equals ``command``."""
    return sum(
        1
        for s in script.statements
        if isinstance(s, IRCall) and s.command == command
    )


class TestEmptyBodySplice:
    def test_empty_body_call_is_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
            "noop\n"
        )
        # Sanity: pre-inline the top-level has two ``noop`` calls.
        assert _calls_to(module.top_level, "noop") == 2
        new_module = inline_module(module, summaries)
        # Both calls vanish; the ``proc`` definition stays put.
        assert _calls_to(new_module.top_level, "noop") == 0

    def test_non_empty_body_is_not_inlined(self):
        # Even though ``setup`` is pure_leaf and small, its body has
        # a statement so the v0 splice declines.
        module, summaries = _prepare(
            "proc setup {} { set x 1 }\n"
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 1

    def test_non_pure_leaf_is_not_inlined(self):
        # ``upvar`` blocks pure_leaf, so the catalogue tags ``skip_me``
        # NEVER and the inliner skips it even with an empty body.
        module, summaries = _prepare(
            "proc skip_me {name} { upvar 1 $name v }\n"
            "skip_me x\n"
        )
        proc = module.procedures["::skip_me"]
        assert proc.inline_decision is InlineDecision.NEVER
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "skip_me") == 1


class TestNestedSites:
    def test_call_inside_if_clause_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "if {1} { noop }\n"
        )
        new_module = inline_module(module, summaries)
        # Find the IRIf statement; its clause body should have no
        # remaining ``noop`` call.
        from core.compiler.ir import IRIf

        if_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRIf)]
        assert if_nodes, "expected an IRIf statement"
        clause_body = if_nodes[0].clauses[0].body
        assert _calls_to(clause_body, "noop") == 0

    def test_call_inside_for_body_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "for {set i 0} {$i < 3} {incr i} { noop }\n"
        )
        new_module = inline_module(module, summaries)
        from core.compiler.ir import IRFor

        for_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRFor)]
        assert for_nodes, "expected an IRFor statement"
        assert _calls_to(for_nodes[0].body, "noop") == 0


class TestPurity:
    def test_input_module_unchanged(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
        )
        original_calls = _calls_to(module.top_level, "noop")
        inline_module(module, summaries)
        # The original module's statements still mention the call.
        assert _calls_to(module.top_level, "noop") == original_calls

    def test_idempotent(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
        )
        once = inline_module(module, summaries)
        twice = inline_module(once, summaries)
        assert _calls_to(once.top_level, "noop") == _calls_to(
            twice.top_level, "noop"
        )


class TestSingleCallWrapperSplice:
    """v1: zero-param wrapper procs whose body is a single IRCall."""

    def test_wrapper_call_is_replaced_with_inner(self):
        # ``setup`` wraps a single ``puts`` call.  Calls to
        # ``setup`` should be replaced by the wrapped ``puts``
        # call.  Since ``puts`` doesn't resolve to a tracked
        # proc, the namespace-invariance check passes (it's a
        # runtime builtin).
        module, summaries = _prepare(
            "proc setup {} { puts \"starting\" }\n"
            "setup\n"
        )
        # Pre-inline: top-level has 0 ``puts`` calls, 1 ``setup`` call.
        assert _calls_to(module.top_level, "setup") == 1
        assert _calls_to(module.top_level, "puts") == 0

        new_module = inline_module(module, summaries)
        # Post-inline: 0 ``setup`` calls, 1 ``puts`` call.
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "puts") == 1

    def test_wrapper_with_args_at_call_site_declined(self):
        # If the call site passes args (even though the callee
        # has zero params), inlining would silently discard their
        # evaluation.  Decline.
        module, summaries = _prepare(
            "proc setup {} { puts \"hi\" }\n"
            "setup [list 1 2]\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 1
        assert _calls_to(new_module.top_level, "puts") == 0

    def test_wrapper_with_unqualified_proc_call_declined(self):
        # If the wrapped call resolves to a tracked proc from
        # the callee's namespace, the bare command word might
        # bind to a different proc from a caller in another
        # namespace.  Decline.
        module, summaries = _prepare(
            "proc helper {} {}\n"
            "proc setup {} { helper }\n"
            "setup\n"
        )
        # ``helper`` resolves to ``::helper`` from inside
        # ``::setup``.  v1 declines the splice for safety.
        new_module = inline_module(module, summaries)
        # ``::setup`` should still be present, but ``::helper``
        # (empty body, ALWAYS) inlined into ``setup``'s body —
        # which leaves setup with an empty body.  Re-running
        # would then qualify setup itself for v0 splice.
        # First-pass behaviour: setup body becomes empty after
        # helper-inlining; a SECOND pass would inline the now-
        # empty setup.  We only run once here, so setup stays.
        assert _calls_to(new_module.top_level, "setup") == 1

    def test_qualified_wrapper_call_inlines(self):
        # ``::puts`` is a fully qualified call to a known frameless
        # builtin, so the wrapped call is namespace-invariant and
        # the splice fires.
        module, summaries = _prepare(
            "proc setup {} { ::puts \"start\" }\n"
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "::puts") == 1


class TestPipelineIntegration:
    """Confirm the inliner fires when invoked through ``wasm_codegen_module``."""

    def test_empty_body_call_disappears_in_emitted_wasm(self):
        """``noop`` calls should produce zero ``call`` instructions to ``::noop``."""
        from core.compiler.cfg import build_cfg
        from core.compiler.codegen.wasm import WasmOp, wasm_codegen_module

        source = (
            "proc noop {} {}\n"
            "noop\n"
            "noop\n"
            "noop\n"
        )
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        module = wasm_codegen_module(cfg, ir, inline=True)

        # Find the ::top function and the ::noop function (if it
        # still exists in the WASM module).  We assert the top-
        # level body has zero calls into ``::noop``.
        top = next(f for f in module.functions if f.name == "::top")
        noop_funcs = [
            (i, f) for i, f in enumerate(module.functions) if f.name == "::noop"
        ]
        if not noop_funcs:
            return  # noop wasn't even emitted — strongest possible result
        noop_idx = module.imports.__len__() + noop_funcs[0][0]
        for instr in top.body:
            if instr.op != WasmOp.CALL:
                continue
            target = 0
            shift = 0
            for byte in instr.operands:
                target |= (byte & 0x7F) << shift
                if not (byte & 0x80):
                    break
                shift += 7
            assert target != noop_idx, (
                "expected zero direct calls into ::noop after inlining"
            )

    def test_inline_flag_off_preserves_calls(self):
        """``inline=False`` skips the splice — calls remain in the bytecode."""
        from core.compiler.cfg import build_cfg
        from core.compiler.codegen.wasm import wasm_codegen_module

        source = "proc noop {} {}\nnoop\nnoop\n"
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        module_off = wasm_codegen_module(cfg, ir, inline=False)
        module_on = wasm_codegen_module(cfg, ir, inline=True)
        # The off variant should be at least as large in the top
        # function body as the on variant.
        top_off = next(f for f in module_off.functions if f.name == "::top")
        top_on = next(f for f in module_on.functions if f.name == "::top")
        assert len(top_off.body) >= len(top_on.body), (
            f"inline=True should not grow the body; off={len(top_off.body)} on={len(top_on.body)}"
        )


class TestResolution:
    def test_unqualified_call_in_namespace(self):
        # The callee is defined inside ``::ns``; the call is bare.
        # The interprocedural resolver walks namespaces, so the
        # bare ``noop`` from inside ``::ns::caller`` resolves to
        # ``::ns::noop`` and is eligible.
        module, summaries = _prepare(
            "namespace eval ::ns {\n"
            "  proc noop {} {}\n"
            "  proc caller {} { noop }\n"
            "}\n"
        )
        # The catalogue should mark ::ns::noop as ALWAYS.
        assert module.procedures["::ns::noop"].inline_decision is InlineDecision.ALWAYS
        new_module = inline_module(module, summaries)
        caller_body = new_module.procedures["::ns::caller"].body
        assert _calls_to(caller_body, "noop") == 0
