"""S4.2 v0 — empty-body splice tests."""

from __future__ import annotations

import textwrap

from core.compiler.inlining import (
    apply_inline_catalogue,
    inline_module,
)
from core.compiler.ir import InlineDecision, IRCall
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
    return sum(1 for s in script.statements if isinstance(s, IRCall) and s.command == command)


class TestEmptyBodySplice:
    def test_empty_body_call_is_dropped(self):
        module, summaries = _prepare("proc noop {} {}\nnoop\nnoop\n")
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
        # not mutated.  Per PR #237 review the proc itself stays
        # in the module — embedding hosts may still reach it via
        # eval / namespace import / rename — but the static call
        # site is gone.
        module, summaries = _prepare("proc setup {} { set x 1 }\nsetup\n")
        new_module = inline_module(module, summaries)
        # Call vanishes; setup body STAYS (observable as a Tcl
        # command from outside the compilation unit).
        assert _calls_to(new_module.top_level, "setup") == 0
        assert "::setup" in new_module.procedures

    def test_non_pure_leaf_is_not_inlined(self):
        # ``upvar`` blocks pure_leaf, so the catalogue tags ``skip_me``
        # NEVER and the inliner skips it even with an empty body.
        module, summaries = _prepare("proc skip_me {name} { upvar 1 $name v }\nskip_me x\n")
        proc = module.procedures["::skip_me"]
        assert proc.inline_decision is InlineDecision.NEVER
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "skip_me") == 1


class TestNestedSites:
    def test_call_inside_if_clause_dropped(self):
        module, summaries = _prepare("proc noop {} {}\nif {1} { noop }\n")
        new_module = inline_module(module, summaries)
        # Find the IRIf statement; its clause body should have no
        # remaining ``noop`` call.
        from core.compiler.ir import IRIf

        if_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRIf)]
        assert if_nodes, "expected an IRIf statement"
        clause_body = if_nodes[0].clauses[0].body
        assert _calls_to(clause_body, "noop") == 0

    def test_call_inside_for_body_dropped(self):
        module, summaries = _prepare("proc noop {} {}\nfor {set i 0} {$i < 3} {incr i} { noop }\n")
        new_module = inline_module(module, summaries)
        from core.compiler.ir import IRFor

        for_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRFor)]
        assert for_nodes, "expected an IRFor statement"
        assert _calls_to(for_nodes[0].body, "noop") == 0


class TestPurity:
    def test_input_module_unchanged(self):
        module, summaries = _prepare("proc noop {} {}\nnoop\n")
        original_calls = _calls_to(module.top_level, "noop")
        inline_module(module, summaries)
        # The original module's statements still mention the call.
        assert _calls_to(module.top_level, "noop") == original_calls

    def test_idempotent(self):
        module, summaries = _prepare("proc noop {} {}\nnoop\n")
        once = inline_module(module, summaries)
        twice = inline_module(once, summaries)
        assert _calls_to(once.top_level, "noop") == _calls_to(twice.top_level, "noop")


class TestMultiStatementWrapperSplice:
    """v2: zero-param wrappers whose body is N pure-side-effect IRCalls."""

    def test_two_call_wrapper_inlines_both(self):
        # ``proc setup {} { puts "a"; puts "b" }`` — every body
        # statement is a splice-safe IRCall.  Calls to ``setup``
        # become two ``puts`` calls inline.
        module, summaries = _prepare('proc setup {} { puts "a"; puts "b" }\nsetup\n')
        assert _calls_to(module.top_level, "setup") == 1
        assert _calls_to(module.top_level, "puts") == 0

        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "puts") == 2

    def test_mixed_safe_calls_inline(self):
        # ``puts`` + ``string`` are both splice-safe.
        module, summaries = _prepare(
            'proc setup {} { puts "starting"; ::string length "x" }\nsetup\n'
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
        module, summaries = _prepare('proc setup {} { puts "starting"; set marker 1 }\nsetup\n')
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        # ``puts`` propagated to top level.
        assert _calls_to(new_module.top_level, "puts") == 1
        # ``set marker 1`` becomes ``set __inline_<n>__marker 1`` —
        # the caller's ``marker`` (if any) is untouched.
        names = [getattr(s, "name", None) for s in new_module.top_level.statements]
        assert any(
            n is not None and n.startswith("__inline_") and n.endswith("__marker") for n in names
        )


class TestParameterisedInline:
    """v3: procs with parameters and / or local writes."""

    def test_proc_with_one_param_inlines(self):
        # ``proc inc {x} { puts $x }`` called as ``inc 5``:
        # inline binds ``__inline_1__x = 5`` then ``puts $__inline_1__x``.
        module, summaries = _prepare("proc inc {x} { puts $x }\ninc 5\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "inc") == 0
        # Mangled binding present.
        assert any(
            isinstance(s, IRCall) and s.command == "puts" for s in new_module.top_level.statements
        )

    def test_proc_with_local_write_uses_mangled_name(self):
        # ``proc f {} { set y 5; puts $y }``: the body's ``y`` is
        # α-renamed.  After inlining, the call site has the
        # mangled ``set`` followed by the mangled ``$y`` reference.
        module, summaries = _prepare("proc f {} { set y 5; puts $y }\nf\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # The set's ``name`` is mangled; the puts's arg uses the
        # same mangled name via $-substitution.
        from core.compiler.ir import IRAssignConst, IRAssignValue

        mangled_name: str | None = None
        for s in new_module.top_level.statements:
            if isinstance(s, (IRAssignConst, IRAssignValue)) and s.name.startswith("__inline_"):
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
        module, summaries = _prepare("proc f {x} { puts $x; return $x }\nf 5\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # An IRReturn ends up at the top level.
        from core.compiler.ir import IRReturn

        assert any(isinstance(s, IRReturn) for s in new_module.top_level.statements)

    def test_proc_with_trailing_return_inlines_at_non_terminal_via_wrap(self):
        # Same proc but the call is NOT in terminal position —
        # the for/break wrap routes the IRReturn through a
        # synthesised result var so the rest of the caller's body
        # still runs.
        module, summaries = _prepare("proc f {x} { puts $x; return $x }\nf 5\nset marker after\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # ``set marker after`` survives — the wrap didn't short-
        # circuit the rest of the caller body.
        from core.compiler.ir import IRAssignConst, IRAssignValue, IRWhile

        assert any(
            isinstance(s, (IRAssignConst, IRAssignValue)) and s.name == "marker"
            for s in new_module.top_level.statements
        )
        # The wrap inserts an IRWhile {1} containing the
        # rewritten body — the IRReturn turns into ``set
        # __result; break`` and the loop's break exits to the
        # caller's surrounding body.
        assert any(isinstance(s, IRWhile) for s in new_module.top_level.statements)

    def test_arity_mismatch_declines(self):
        # Caller passes one arg to a two-param proc — defaults
        # would normally apply, but v3 doesn't synthesise them yet.
        # Decline rather than mis-inline.
        module, summaries = _prepare("proc f {x y} { puts $x }\nf 1\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_variadic_args_inlines_with_list_pack(self):
        # ``proc f {args} { puts $args }`` called as ``f 1 2 3``:
        # v3 binds ``__inline_<n>__args`` to ``[list 1 2 3]`` and
        # inlines the body.
        module, summaries = _prepare("proc f {args} { puts $args }\nf 1 2 3\n")
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
        # The empty-args binding may be IRAssignConst (preferred for
        # the literal empty string) or IRAssignValue depending on
        # how the binder represents empties; either is correct.
        module, summaries = _prepare("proc f {a args} { puts $a }\nf 1\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignConst, IRAssignValue

        args_binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, (IRAssignConst, IRAssignValue)) and s.name.endswith("__args")
        )
        assert args_binding.value == ""

    def test_braced_literal_arg_uses_iassignconst(self):
        # PR #237 review: ``f {$y}`` passes the *literal string*
        # ``$y`` to f.  Bound parameter must be IRAssignConst so
        # the inlined ``set __inline_x ...`` doesn't re-substitute
        # ``$y`` from the caller's frame.
        module, summaries = _prepare("proc f {x} { puts $x }\nf {literal-$y}\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignConst

        binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignConst) and s.name.endswith("__x")
        )
        # Value preserves the dollar sign verbatim — no substitution.
        assert binding.value == "literal-$y"

    def test_substitution_arg_keeps_iassignvalue(self):
        # ``f $y`` passes the value of caller's ``y``.  Bound
        # parameter remains IRAssignValue so the inlined ``set
        # __inline_x $y`` reads the caller's frame correctly.
        module, summaries = _prepare("proc f {x} { puts $x }\nset y 42\nf $y\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignValue

        binding = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and s.name.endswith("__x")
        )
        assert binding.value == "${y}"

    def test_expand_arg_declines_inlining(self):
        # ``f {*}$lst`` expands the runtime list into call words.
        # The inliner can't statically unpack a runtime list into
        # parameter slots, so it must decline and let the call
        # fall back to the runtime dispatch path.
        module, summaries = _prepare("proc f {a b c} { puts $a }\nset lst {1 2 3}\nf {*}$lst\n")
        new_module = inline_module(module, summaries)
        # Call NOT inlined — still present as an IRCall.
        assert _calls_to(new_module.top_level, "f") == 1

    def test_default_arg_inlines(self):
        # ``proc f {x {y 5}} { puts $x }`` called with one arg —
        # the default ``5`` fills in for ``y``.
        module, summaries = _prepare("proc f {x {y 5}} { puts $x }\nf 3\n")
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
        module, summaries = _prepare("proc f {x y} { puts $x }\nf 1\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_early_return_in_if_inlines_via_wrap(self):
        # ``proc f {x} { if {$x > 0} { return 1 }; return 0 }``
        # has an early return inside an IRIf clause.  The for/
        # break wrap rewrites both IRReturns to ``set __result;
        # break`` and the inlined site emits ``return $__result``
        # (terminal call) after the loop.  Note: ``return -1``
        # would lower to IRBarrier (interpreted as ``return -code
        # ...``), defeating pure_leaf — use a positive literal.
        module, summaries = _prepare("proc f {x} { if {$x > 0} { return 1 }; return 0 }\nf 5\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # Wrap emits an IRFor surrounding the rewritten body.
        from core.compiler.ir import IRReturn, IRWhile

        assert any(isinstance(s, IRWhile) for s in new_module.top_level.statements)
        # Terminal call → the trailing ``return $__result`` is
        # appended.
        assert any(isinstance(s, IRReturn) for s in new_module.top_level.statements)

    def test_implicit_trailing_return_with_early_irreturn_captured(self):
        # PR #237 review: ``proc f {x} { if {$x} { return 1 }; set y 2 }``
        # has a non-trailing IRReturn (in the if clause) and an
        # implicit-trailing-value statement (``set y 2`` returns 2).
        # The wrap path must capture that implicit value so the
        # synthesised ``return $__result`` forwards it on the
        # if-false fallthrough.  Otherwise the inlined site
        # silently returns "" instead of 2.
        module, summaries = _prepare("proc f {x} { if {$x} { return 1 }; set y 2 }\nf 5\n")
        new_module = inline_module(module, summaries)
        # Inlined.
        assert _calls_to(new_module.top_level, "f") == 0
        # Look inside the wrap loop for a ``set __inline_..__RESULT 2``
        # capture statement on the fall-through path.
        from core.compiler.ir import IRAssignValue, IRWhile

        loop = next(s for s in new_module.top_level.statements if isinstance(s, IRWhile))
        captures = [
            s
            for s in loop.body.statements
            if isinstance(s, IRAssignValue) and s.name.endswith("__RESULT") and s.value == "2"
        ]
        assert captures, (
            "wrap path did not capture the implicit trailing return value "
            "(set y 2 should produce __RESULT=2 on the fall-through)"
        )

    def test_implicit_trailing_call_declines(self):
        # When the trailing statement is a *call* (not an
        # IRAssignConst/Value), capturing the implicit return
        # would need command-substitution rewriting that we don't
        # do today.  Decline so the runtime path produces correct
        # implicit-return semantics.
        module, summaries = _prepare("proc f {x} { if {$x} { return 1 }; puts ok }\nf 5\n")
        new_module = inline_module(module, summaries)
        # NOT inlined — call site survives as IRCall.
        assert _calls_to(new_module.top_level, "f") == 1

    def test_irreturn_inside_loop_declines(self):
        # ``proc f {} { for {set i 0} {$i < 10} {incr i} { if {$i == 5} { return $i } } }``
        # has IRReturn inside a for body.  ``break`` rewriting
        # would exit the inner for, not the wrap, so v3 declines.
        module, summaries = _prepare(
            "proc f {} {\n"
            "  for {set i 0} {$i < 10} {incr i} {\n"
            "    if {$i == 5} { return $i }\n"
            "  }\n"
            "}\n"
            "f\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_irreturn_inside_catch_declines(self):
        # ``catch`` traps return-codes including ``break``, so a
        # break-rewritten IRReturn inside catch would be trapped
        # rather than reaching the wrap.  Decline.
        module, summaries = _prepare(
            "proc f {x} {\n  catch { if {$x < 0} { return -1 } }\n  return $x\n}\nf 5\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 1

    def test_array_element_write_inlines_with_array_rename(self):
        # ``proc f {} { set arr(0) 1; set arr(1) 2 }`` writes two
        # elements of a local array.  v3 now accepts this: the
        # array base ``arr`` is α-renamed to ``__inline_<n>__arr``
        # and the ``(idx)`` suffix is preserved on every write,
        # so all array elements bind to the same renamed array.
        module, summaries = _prepare("proc f {} { set arr(0) 1; set arr(1) 2 }\nf\n")
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        from core.compiler.ir import IRAssignConst, IRAssignValue

        # Both writes should have a mangled ``arr`` base with
        # the original ``(0)`` / ``(1)`` index intact.
        array_writes = [
            s.name
            for s in new_module.top_level.statements
            if isinstance(s, (IRAssignConst, IRAssignValue)) and "(" in s.name
        ]
        assert len(array_writes) == 2
        # Both writes share the same mangled base.
        bases = {n.split("(")[0] for n in array_writes}
        assert len(bases) == 1
        base = next(iter(bases))
        assert base.startswith("__inline_") and base.endswith("__arr")
        # Indices preserved.
        assert {n.split("(")[1].rstrip(")") for n in array_writes} == {"0", "1"}

    def test_array_read_after_write_uses_same_rename(self):
        # ``set arr(k) "hi"; puts $arr(k)`` — the read of the
        # array element after the write must resolve to the same
        # renamed array.
        module, summaries = _prepare('proc f {} { set arr(k) "hi"; puts $arr(k) }\nf\n')
        new_module = inline_module(module, summaries)
        from core.compiler.ir import IRAssignValue, IRCall

        write_name = next(
            s.name
            for s in new_module.top_level.statements
            if isinstance(s, IRAssignValue) and "(" in s.name
        )
        write_base = write_name.split("(")[0]
        # The puts arg should reference ``${<write_base>(k)}`` (or
        # the bare-form equivalent) so the read targets the same
        # renamed array.
        puts_call = next(
            s
            for s in new_module.top_level.statements
            if isinstance(s, IRCall) and s.command == "puts"
        )
        assert any(write_base in a for a in puts_call.args)

    def test_nested_control_flow_inlines(self):
        # The body has an IRIf with a nested ``puts`` — v3 now
        # accepts nested control flow and the rewriter handles
        # the iterator vars / nested bodies.
        module, summaries = _prepare('proc f {x} {\n  if {$x > 0} { puts "positive" }\n}\nf 5\n')
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "f") == 0
        # The IRIf survives the inline; the bound x is referenced
        # inside the renamed condition.
        from core.compiler.ir import IRIf

        assert any(isinstance(s, IRIf) for s in new_module.top_level.statements)

    def test_two_call_sites_get_distinct_mangling(self):
        # Each call site gets a unique mangling counter so the two
        # bound parameter slots don't collide.
        module, summaries = _prepare("proc f {x} { puts $x }\nf 1\nf 2\n")
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
    """Per PR #237 review: user-defined procs are externally
    observable Tcl commands (host eval, ``info procs``, ``namespace
    import``, ``rename``).  The inliner can't safely remove them
    even when its static IRCall sites are all spliced away.  These
    tests verify that procs survive inlining unconditionally;
    dead-proc removal will return only when an explicit
    ``compiler_synthetic`` marker exists on :class:`IRProcedure`."""

    def test_inlinable_proc_kept_after_inlining(self):
        # ``noop`` is inlined at every call site, but the proc
        # definition stays — it's a Tcl command observable from
        # outside the compilation unit.
        module, summaries = _prepare("proc noop {} {}\nnoop\nnoop\n")
        new_module = inline_module(module, summaries)
        assert "::noop" in new_module.procedures

    def test_unreferenced_inlinable_proc_kept_for_external_callers(self):
        # No internal call sites — proc may be invoked externally
        # (Python test harness, embedding host).
        module, summaries = _prepare("proc lonely {} {}\n")
        new_module = inline_module(module, summaries)
        assert "::lonely" in new_module.procedures

    def test_non_inlinable_proc_kept_even_if_unreferenced(self):
        # ``upvar`` makes it non-pure_leaf → not in candidates set
        # → kept regardless of static reference count (same as
        # before).
        module, summaries = _prepare("proc opaque {name} { upvar 1 $name v }\n")
        new_module = inline_module(module, summaries)
        assert "::opaque" in new_module.procedures

    def test_compiler_synthetic_proc_dropped_after_inlining(self):
        # When a proc is flagged ``compiler_synthetic=True``, the
        # dead-proc-removal pass IS allowed to drop it after every
        # static call site has been inlined.  Synthetic procs are
        # passes-introduced helpers with no external observers.
        from dataclasses import replace as dc_replace

        module, summaries = _prepare("proc helper {} {}\nhelper\n")
        # Lowering can't synthesise the marker (yet) — patch it on
        # for this test to exercise the gate without a real pass.
        proc = module.procedures["::helper"]
        synthetic_proc = dc_replace(proc, compiler_synthetic=True)
        module = dc_replace(
            module,
            procedures={**module.procedures, "::helper": synthetic_proc},
        )
        new_module = inline_module(module, summaries)
        # Synthetic + inlinable + every static site spliced → drop.
        assert "::helper" not in new_module.procedures


class TestSingleCallWrapperSplice:
    """v1: zero-param wrapper procs whose body is a single IRCall."""

    def test_wrapper_call_is_replaced_with_inner(self):
        # ``setup`` wraps a single ``puts`` call.  Calls to
        # ``setup`` should be replaced by the wrapped ``puts``
        # call.  Since ``puts`` doesn't resolve to a tracked
        # proc, the namespace-invariance check passes (it's a
        # runtime builtin).
        module, summaries = _prepare('proc setup {} { puts "starting" }\nsetup\n')
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
        module, summaries = _prepare('proc setup {} { puts "hi" }\nsetup [list 1 2]\n')
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 1
        assert _calls_to(new_module.top_level, "puts") == 0

    def test_wrapper_with_unqualified_proc_call_declined(self):
        # If the wrapped call resolves to a tracked proc from
        # the callee's namespace, the bare command word might
        # bind to a different proc from a caller in another
        # namespace.  Decline.
        module, summaries = _prepare("proc helper {} {}\nproc setup {} { helper }\nsetup\n")
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
        module, summaries = _prepare('proc setup {} { ::puts "start" }\nsetup\n')
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 0
        assert _calls_to(new_module.top_level, "::puts") == 1


class TestPipelineIntegration:
    """Confirm the inliner fires when invoked through ``wasm_codegen_module``."""

    def test_empty_body_call_disappears_in_emitted_wasm(self):
        """``noop`` calls should produce zero ``call`` instructions to ``::noop``."""
        from core.compiler.cfg import build_cfg
        from core.compiler.codegen.wasm import WasmOp, wasm_codegen_module

        source = "proc noop {} {}\nnoop\nnoop\nnoop\n"
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        module = wasm_codegen_module(cfg, ir, inline=True)

        # Find the ::top function and the ::noop function (if it
        # still exists in the WASM module).  We assert the top-
        # level body has zero calls into ``::noop``.
        top = next(f for f in module.functions if f.name == "::top")
        noop_funcs = [(i, f) for i, f in enumerate(module.functions) if f.name == "::noop"]
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
            assert target != noop_idx, "expected zero direct calls into ::noop after inlining"

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
            "namespace eval ::ns {\n  proc noop {} {}\n  proc caller {} { noop }\n}\n"
        )
        # The catalogue should mark ::ns::noop as ALWAYS.
        assert module.procedures["::ns::noop"].inline_decision is InlineDecision.ALWAYS
        new_module = inline_module(module, summaries)
        caller_body = new_module.procedures["::ns::caller"].body
        assert _calls_to(caller_body, "noop") == 0

    def test_unqualified_call_in_irblock_namespace(self):
        # PR #237 review: a call sitting inside the *top-level
        # IRBlock* (the ``namespace eval ::ns { … }`` body itself,
        # not inside a proc body) must also resolve against ``::ns``
        # — previously the inliner used ``::`` as the resolution
        # context for IRBlock children and missed ``::ns::noop``.
        module, summaries = _prepare("namespace eval ::ns {\n  proc noop {} {}\n  noop\n}\n")
        new_module = inline_module(module, summaries)
        # Find the IRBlock and confirm the noop call inside it
        # got inlined away.
        from core.compiler.ir import IRBlock

        block = next(s for s in new_module.top_level.statements if isinstance(s, IRBlock))
        assert _calls_to(block.body, "noop") == 0
