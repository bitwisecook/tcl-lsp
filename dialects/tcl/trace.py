"""trace -- Monitor variable accesses, command usages and command executions."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page trace.n"


def _trace_add_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Map ``trace add`` argument roles by trace type.

    Indices are relative to the args after the ``add`` subcommand word
    (so ``args[0]`` is the trace type: ``variable``/``command``/``execution``).

    For ``trace add variable name ops body`` the *name* arg is treated as
    a write-site: the registered body can mutate the variable's value when
    fired, so SSA must see *name* as a definition.  ``trace add command``
    and ``trace add execution`` refer to commands, not variables.
    """
    if len(args) >= 2 and args[0] == "variable":
        return {1: frozenset({ArgRole.VAR_WRITE})}
    return {}


@register
class TraceCommand(CommandDef):
    name = "trace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="trace",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Monitor variable accesses, command usages and command executions",
                synopsis=("trace option ?arg arg ...?",),
                snippet="Arranges for commands to be executed whenever certain operations "
                "are invoked.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="trace option ?arg arg ...?",
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(4, 4),
                    detail="Arrange for a command to be executed when the specified operation occurs.",
                    synopsis="trace add type name ops commandPrefix",
                    arg_role_resolver=_trace_add_arg_roles,
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(2, 2),
                    detail="Returns a list of ops/commandPrefix pairs for the given name.",
                    synopsis="trace info type name",
                    return_type=TclType.LIST,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(4, 4),
                    detail="Remove a previously set trace.",
                    synopsis="trace remove type name opList commandPrefix",
                ),
                "variable": SubCommand(
                    name="variable",
                    dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6"}),
                    arity=Arity(3, 3),
                    detail="Arrange for command to be executed whenever variable name is accessed. Deprecated in favour of trace add variable.",
                    synopsis="trace variable name ops command",
                ),
                "vdelete": SubCommand(
                    name="vdelete",
                    dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6"}),
                    arity=Arity(3, 3),
                    detail="Delete a variable trace. Deprecated in favour of trace remove variable.",
                    synopsis="trace vdelete name ops command",
                ),
                "vinfo": SubCommand(
                    name="vinfo",
                    dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6"}),
                    arity=Arity(1, 1),
                    detail="Return trace information for the given variable. Deprecated in favour of trace info variable.",
                    synopsis="trace vinfo name",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            # No ``wasm_runtime_import`` — ``trace add variable`` and
            # friends take 5 words (``trace add variable NAME OPS CMD``)
            # which the generic two-arg ``tcl_cmd_trace_cmd`` import
            # can't carry.  The compiled call path falls through to the
            # interpreter via ``_emit_eval_fallback`` and lands in
            # :func:`cmds/inspect.zig::eval_trace`, which dispatches to
            # both the directory-keyed registry (globals / namespace
            # vars / arrays) and the per-frame chain (proc-locals).
        )
