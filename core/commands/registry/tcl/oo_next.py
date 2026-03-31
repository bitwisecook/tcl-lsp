"""next -- TclOO command to invoke the next method in the MRO."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page next.n"


@register
class OoNextCommand(CommandDef):
    name = "next"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="next",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke the next implementation of a method",
                synopsis=("next ?arg ...?",),
                snippet=(
                    "The next command is used within the body of a method to call the "
                    "next implementation of that method in the method resolution order (MRO). "
                    "Arguments are passed to the next implementation."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="next ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
            ),
        )


@register
class OoNextToCommand(CommandDef):
    name = "nextto"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="nextto",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke a specific superclass implementation of a method",
                synopsis=("nextto class ?arg ...?",),
                snippet=(
                    "The nextto command is like next but invokes a specific class's "
                    "implementation of the current method rather than the next in the MRO."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="nextto class ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
