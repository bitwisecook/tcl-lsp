"""exec -- Invoke subprocesses."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    OptionSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register


@register
class ExecCommand(CommandDef):
    name = "exec"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exec",
            dialects=DIALECTS_EXCEPT_IRULES,
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exec ?switches? arg ?arg ...?",
                    options=(
                        # -ignorestderr: TIP 358, Tcl 8.5+.
                        OptionSpec(
                            name="-ignorestderr",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(name="-keepnewline"),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            taint_sink_safe_colour=TaintColour.SHELL_ATOM,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            # No ``wasm_runtime_import`` — ``exec`` is variadic and
            # the codegen's argc=N truncation would silently drop
            # extra argv words.  Routing every call through the
            # eval-fallback into the BUILTIN handler in
            # ``runtime/zig/cmds/exec.zig`` keeps the multi-arg path
            # honest, and the BUILTIN handler enforces the
            # ``CAP_EXEC`` capability gate.
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
