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

    def test_non_empty_body_v3_inlines(self):
        # v3 lifts the v0/v1/v2 zero-statement / verbatim-only
        # restriction.  ``proc setup {} { set x 1 }`` is a v3
        # candidate: body writes a local, no params, no return.
        # The set is α-renamed so the caller's ``x`` (if any) is
        # not mutated.
        module, summaries = _prepare(
            "proc setup {} { set x 1 }\n"
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        # Call vanishes; setup body is dead-proc-eliminated.
        assert _calls_to(new_module.top_level, "setup") == 0
        assert "::setup" not in new_module.procedures

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


class TestMultiStatementWrapperSplice:
    """v2: zero-param wrappers whose body is N pure-side-effect IRCalls."""

    def test_two_call_wrapper_inlines_both(self):
        # ``proc setup {} { puts "a"; puts "b" }`` — every body
        # statement is a splice-safe IRCall.  Calls to ``setup``
        # become two ``puts`` calls inline.
        module, summaries = _prepare(
            'proc setup {} { puts "a"; puts "b" }\n'
            "setup\n"
        )
        assert _calls_to(module.top_level, "setup") == 1
        assert _calls_to(module.top_level, "puts") == 0

        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "puts") == 2

    def test_mixed_safe_calls_inline(self):
        # ``puts`` + ``string`` are both splice-safe.
        module, summaries = _prepare(
            'proc setup {} { puts "starting"; ::string length "x" }\n'
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "puts") == 1
        assert _calls_to(new_module.top_level, "::string") == 1

    def test_set_in_body_v3_inlines_with_alpha_rename(self):
        # v3 lifts the v2 "verbatim-only" restriction: a body that
        # writes a local now inlines via the parameterised path,
        # which α-renames the local so the caller's slot isn't
        # touched.
        module, summaries = _prepare(
            'proc setup {} { puts "starting"; set marker 1 }\n'
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        # ``puts`` propagated to top level.
        assert _calls_to(new_module.top_level, "puts") == 1
        # ``set marker 1`` becomes ``set __inline_<n>__marker 1`` —
        # the caller's ``marker`` (if any) is untouched.
        names = [
            getattr(s, "name", None)
            for s in new_module.top_level.statements
        ]
        assert any(
            n is not None and n.startswith("__inline_") and n.endswith("__marker")
            for n in names
        )


class TestParameterisedInline:
    """v3: procs with parameters and / or local writes."""

    def test_proc_with_one_param_inlines(self):
        # ``proc inc {x} { puts $x }`` called as ``inc 5``:
        # inline binds ``__inline_1__x = 5`` then ``puts $__inline_1__x``.
        module, summaries = _prepare(
            'proc inc {x} { puts $x }\n'
            "inc 5\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "inc") == 0
        # Mangled binding present.
        assert any(
            isinstance(s, IRCall) and s.command == "puts"
            for s in new_module.top_level.statements
        )

    def test_proc_with_local_write_uses_mangled_name(self):
        # ``proc f {} { set y 5; puts $y }``: the body's ``y`` is
        # α-renamed.  After inlining, the call site has the
        # mangled ``set`` followed by the mangled ``$y`` reference.
        module, summaries = _prepare(
            'proc f {} { set y 5; puts $y }\n'
            "f\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # The set's ``name`` is mangled; the puts's arg uses the
        # same mangled name via $-substitution.
        from core.compiler.ir import IRAssignConst, IRAssignValue

        mangled_name: str | None = None
        for s in new_module.top_level.statements:
            if isinstance(s, (IRAssignConst, IRAssignValue)) and s.name.startswith(
                "__inline_"
            ):
                mangled_name = s.name
                break
        assert mangled_name is not None
        # ``puts ${mangled}`` should appear with $-subst rewritten.
        # The lowering normalises ``$y`` to ``${y}`` in args, so the
        # rewritten form is the braced ``${mangled}``.
        assert any(
            isinstance(s, IRCall)
            and s.command == "puts"
            and any(("${" + mangled_name + "}") in a for a in s.args)
            for s in new_module.top_level.statements
        )

    def test_proc_with_trailing_return_inlines_at_terminal_position(self):
        # ``proc f {x} { puts $x; return $x }`` called at terminal
        # position of the top-level body: v3 keeps the trailing
        # IRReturn intact, so the renamed return becomes the
        # caller's effective return value.
        module, summaries = _prepare(
            'proc f {x} { puts $x; return $x }\n'
            "f 5\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # An IRReturn ends up at the top level.
        from core.compiler.ir import IRReturn

        assert any(
            isinstance(s, IRReturn) for s in new_module.top_level.statements
        )

    def test_proc_with_trailing_return_not_inlined_at_non_terminal(self):
        # Same proc but the call is NOT in terminal position —
        # there's a statement after.  Keeping the IRReturn would
        # short-circuit ``set marker after``; v3 declines.
        module, summaries = _prepare(
            'proc f {x} { puts $x; return $x }\n'
            "f 5\n"
            "set marker after\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_arity_mismatch_declines(self):
        # Caller passes one arg to a two-param proc — defaults
        # would normally apply, but v3 doesn't synthesise them yet.
        # Decline rather than mis-inline.
        module, summaries = _prepare(
            'proc f {x y} { puts $x }\n'
            "f 1\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_variadic_args_inlines_with_list_pack(self):
        # ``proc f {args} { puts $args }`` called as ``f 1 2 3``:
        # v3 binds ``__inline_<n>__args`` to ``[list 1 2 3]`` and
        # inlines the body.
        module, summaries = _prepare(
            'proc f {args} { puts $args }\n'
            "f 1 2 3\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # The bound ``args`` slot's value is a ``[list …]`` literal.
        from core.compiler.ir import IRAssignValue

        args_binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and s.name.endswith("__args")
        )
        assert "[list" in args_binding.value
        assert "1" in args_binding.value
        assert "3" in args_binding.value

    def test_variadic_args_with_zero_extras_inlines_to_empty(self):
        # ``proc f {a args} { ... }`` called as ``f 1`` — no extras.
        module, summaries = _prepare(
            'proc f {a args} { puts $a }\n'
            "f 1\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignValue

        args_binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and s.name.endswith("__args")
        )
        assert args_binding.value == ""

    def test_default_arg_inlines(self):
        # ``proc f {x {y 5}} { puts $x }`` called with one arg —
        # the default ``5`` fills in for ``y``.
        module, summaries = _prepare(
            'proc f {x {y 5}} { puts $x }\n'
            "f 3\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignValue

        y_binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and s.name.endswith("__y")
        )
        assert y_binding.value == "5"

    def test_missing_arg_no_default_declines(self):
        # ``proc f {x y}`` called with only one arg — no default
        # for ``y`` so v3 declines and lets the runtime raise the
        # standard "wrong # args" error.
        module, summaries = _prepare(
            'proc f {x y} { puts $x }\n'
            "f 1\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_nested_control_flow_inlines(self):
        # The body has an IRIf with a nested ``puts`` — v3 now
        # accepts nested control flow and the rewriter handles
        # the iterator vars / nested bodies.
        module, summaries = _prepare(
            'proc f {x} {\n'
            '  if {$x > 0} { puts "positive" }\n'
            '}\n'
            "f 5\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # The IRIf survives the inline; the bound x is referenced
        # inside the renamed condition.
        from core.compiler.ir import IRIf

        assert any(isinstance(s, IRIf) for s in new_module.top_level.statements)

    def test_two_call_sites_get_distinct_mangling(self):
        # Each call site gets a unique mangling counter so the two
        # bound parameter slots don't collide.
        module, summaries = _prepare(
            'proc f {x} { puts $x }\n'
            "f 1\n"
            "f 2\n"
        )
        new_module = inline_module(module, summaries)
        from core.compiler.ir import IRAssignValue

        param_writes = [
            s.name
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and s.name.startswith("__inline_")
        ]
        # Two call sites → two distinct mangled-x writes.
        assert len(param_writes) == 2
        assert param_writes[0] != param_writes[1]


class TestDeadProcElimination:
    def test_inlinable_proc_dropped_after_inlining(self):
        # ``noop`` is inlined at every call site → no static refs
        # remain → drop from module.procedures.
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
            "noop\n"
        )
        new_module = inline_module(module, summaries)
        assert "::noop" not in new_module.procedures

    def test_unreferenced_inlinable_proc_kept_for_external_callers(self):
        # No internal call sites — but the proc may be invoked
        # externally (Python test harness, embedding host).  The
        # post-inline pruning conservatively keeps procs whose
        # pre-inline ``static_call_count`` was 0 because we can't
        # distinguish "truly dead" from "only externally called".
        module, summaries = _prepare("proc lonely {} {}\n")
        new_module = inline_module(module, summaries)
        assert "::lonely" in new_module.procedures

    def test_non_inlinable_proc_kept_even_if_unreferenced(self):
        # ``upvar`` makes it non-pure_leaf → not in candidates set
        # → kept regardless of static reference count.
        module, summaries = _prepare(
            "proc opaque {name} { upvar 1 $name v }\n"
        )
        new_module = inline_module(module, summaries)
        assert "::opaque" in new_module.procedures


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
