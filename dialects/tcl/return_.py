"""return -- Return from the current procedure or script."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl return(1)"


@register
class ReturnCommand(CommandDef):
    name = "return"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="return",
            byte_compiled=True,
            frameless_runtime=True,
            is_language_keyword=True,
            needs_start_cmd=True,
            terminates_block=True,
            hover=HoverSnippet(
                summary="Return from the current procedure/script with optional control-code metadata.",
                synopsis=("return ?-code code? ?-level level? ?result?",),
                snippet="Advanced forms can emulate `break`, `continue`, or custom return codes.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="return ?-code code? ?-level level? ?result?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
