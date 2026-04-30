"""Tests for ``trace add/remove execution`` capture and side-effect composition.

Covers issue #251: a ``trace add execution`` directive composes the
trace body's effects into every call to the target command, so a
registry-pure command (like ``set``) is no longer pure once a trace
fires around it.  ``trace remove execution`` cancels the propagation.
"""

from __future__ import annotations

from core.compiler.ir import CommandTrace, IRModule
from core.compiler.lowering import lower_to_ir
from core.compiler.side_effects import (
    SideEffectTarget,
    classify_side_effects,
)

# Lowering capture


class TestCommandTraceCapture:
    def test_add_execution_literal_target(self) -> None:
        module = lower_to_ir("trace add execution set {enter leave} {puts traced}")
        assert len(module.command_traces) == 1
        trace = module.command_traces[0]
        assert trace.target == "set"
        assert trace.action == "add"
        assert trace.ops == ("enter", "leave")
        assert trace.body == "puts traced"
        assert not trace.target_dynamic

    def test_remove_execution_literal_target(self) -> None:
        module = lower_to_ir(
            "trace add execution set {enter} {puts hi}\n"
            "trace remove execution set {enter} {puts hi}\n"
        )
        assert [t.action for t in module.command_traces] == ["add", "remove"]

    def test_dynamic_target_marked(self) -> None:
        module = lower_to_ir("trace add execution $cmd {enter} {puts hi}")
        assert len(module.command_traces) == 1
        assert module.command_traces[0].target_dynamic

    def test_traced_commands_net_set(self) -> None:
        module = lower_to_ir(
            "trace add execution set {enter} {puts a}\n"
            "trace add execution incr {enter} {puts b}\n"
            "trace remove execution set {enter} {puts a}\n"
        )
        # ``set`` cancelled, ``incr`` still active.
        assert module.traced_commands() == frozenset({"incr"})

    def test_has_dynamic_trace_flag(self) -> None:
        module = lower_to_ir("trace add execution $cmd {enter} {puts hi}")
        assert module.has_dynamic_trace()

    def test_no_capture_for_variable_or_command_subforms(self) -> None:
        # Out of scope: ``trace add variable`` / ``trace add command``.
        module = lower_to_ir(
            "trace add variable foo write {puts $foo}\ntrace add command bar rename {puts $bar}\n"
        )
        assert module.command_traces == ()

    def test_dead_code_not_captured(self) -> None:
        module = lower_to_ir("if {0} { trace add execution set {enter} {puts hi} }")
        assert module.command_traces == ()


# Side-effect composition


class TestTraceComposition:
    def test_set_pure_when_untraced(self) -> None:
        # Sanity baseline: ``set x 1`` is registry-pure for our purposes.
        # ``set`` writes a variable, so ``pure`` is False but it has no
        # UNKNOWN write target.
        effect = classify_side_effects("set", ("x", "1"))
        assert SideEffectTarget.UNKNOWN not in effect.targets

    def test_traced_command_loses_purity(self) -> None:
        traced = frozenset({"::tcl::mathfunc::abs"})
        # Pick a registry-pure expr command for the test.
        baseline = classify_side_effects("expr", ("{1 + 1}",))
        composed = classify_side_effects(
            "expr",
            ("{1 + 1}",),
            traced_commands=frozenset({"expr"}),
        )
        assert baseline.pure
        assert not composed.pure
        assert SideEffectTarget.UNKNOWN in composed.targets
        # Untraced commands in the same call retain their classification.
        unaffected = classify_side_effects("expr", ("{1 + 1}",), traced_commands=traced)
        assert unaffected.pure

    def test_dynamic_trace_pessimises_all_calls(self) -> None:
        composed = classify_side_effects(
            "expr",
            ("{1 + 1}",),
            has_dynamic_trace=True,
        )
        assert not composed.pure
        assert SideEffectTarget.UNKNOWN in composed.targets

    def test_remove_undoes_propagation(self) -> None:
        # Module-level: an add+remove pair leaves traced_commands() empty,
        # so classification is unchanged from the registry baseline.
        module = lower_to_ir(
            "trace add execution expr {enter} {puts a}\n"
            "trace remove execution expr {enter} {puts a}\n"
        )
        traced = module.traced_commands()
        composed = classify_side_effects(
            "expr",
            ("{1 + 1}",),
            traced_commands=traced,
            has_dynamic_trace=module.has_dynamic_trace(),
        )
        assert composed.pure


class TestIRModuleHelpers:
    def test_traced_commands_skips_dynamic_target(self) -> None:
        module = IRModule(
            command_traces=(
                CommandTrace(
                    target="",
                    ops=("enter",),
                    body=None,
                    action="add",
                    target_dynamic=True,
                ),
                CommandTrace(
                    target="set",
                    ops=("enter",),
                    body="puts hi",
                    action="add",
                ),
            )
        )
        assert module.traced_commands() == frozenset({"set"})
        assert module.has_dynamic_trace()
