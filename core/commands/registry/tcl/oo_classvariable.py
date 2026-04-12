"""classvariable -- TclOO command to link class-level variables."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page define.n"


@register
class OoClassvariableCommand(CommandDef):
    name = "classvariable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="classvariable",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="link local variables to class-shared variables",
                synopsis=("classvariable variableName ?variableName ...?",),
                snippet=(
                    "The classvariable command arranges for local variables with the given "
                    "names to refer to variables shared among all instances of the class. "
                    "Used inside method bodies or class definitions."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="classvariable variableName ?variableName ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
