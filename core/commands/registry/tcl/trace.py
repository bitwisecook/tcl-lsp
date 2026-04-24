"""trace -- Monitor variable accesses, command usages and command executions."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
    WasmRuntimeImport,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page trace.n"


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
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_trace",
                argc=2,
                export_name="tcl_cmd_trace_cmd",
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
