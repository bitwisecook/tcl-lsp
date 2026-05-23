"""subst -- Perform backslash, command, and variable substitutions."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl subst(1)"


@register
class SubstCommand(CommandDef):
    name = "subst"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="subst",
            hover=HoverSnippet(
                summary="Perform backslash, command, and variable substitutions.",
                synopsis=("subst ?options? string",),
                snippet=(
                    "**Security**: Without `-nocommands`, any `[cmd]` in the "
                    "string is executed as Tcl. Use `-nocommands` when only "
                    "variable substitution is needed: "
                    "`subst -nocommands $template`. For safe templating, "
                    "prefer `[string map]` or `[format]`."
                ),
                source=_SOURCE,
                return_value="The string with substitutions applied.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="subst ?options? string",
                    options=(
                        OptionSpec(name="-nobackslashes"),
                        OptionSpec(name="-nocommands"),
                        OptionSpec(name="-novariables"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            is_unescape_command=True,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            performs_substitution=True,
        )
