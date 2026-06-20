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
from .const_fold import fold_subst

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
                        # Tcl 9.1 (TIP) adds positive forms that enable only the
                        # named substitutions.  Positive and negated options may
                        # not be combined in a single call.
                        OptionSpec(name="-backslashes", dialects=frozenset({"tcl9.1"})),
                        OptionSpec(name="-commands", dialects=frozenset({"tcl9.1"})),
                        OptionSpec(name="-variables", dialects=frozenset({"tcl9.1"})),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            is_unescape_command=True,
            # Folds only the literal ``subst string`` form (no options): ``$var``
            # is resolved by the lattice as subst would, ``[command]`` makes the
            # folder bail (side effect), and a backslash escape is left unfolded.
            const_fold=fold_subst,
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
