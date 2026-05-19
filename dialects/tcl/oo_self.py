"""self -- TclOO command to query the current object identity."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page self.n"

_av = make_av(_SOURCE)


@register
class OoSelfCommand(CommandDef):
    name = "self"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="self",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="query the identity of the current object",
                synopsis=("self ?subcommand?",),
                snippet=(
                    "The self command is used within the body of a method to query the "
                    "identity or other properties of the current object."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="self ?subcommand?",
                    arg_values={
                        0: (
                            _av("object", "Return the name of the current object.", "self object"),
                            _av("class", "Return the class of the current object.", "self class"),
                            _av(
                                "namespace",
                                "Return the namespace of the current object.",
                                "self namespace",
                            ),
                            _av("method", "Return the name of the current method.", "self method"),
                            _av("caller", "Return info about the calling method.", "self caller"),
                            _av(
                                "target",
                                "Return the name of the target of a forward method.",
                                "self target",
                            ),
                            _av("call", "Return the current call chain.", "self call"),
                            _av("filter", "Return the current filter.", "self filter"),
                        ),
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.STRING,
        )
