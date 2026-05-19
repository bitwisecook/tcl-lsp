"""my -- TclOO method-context command for the current object."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import register

_SOURCE = "Tcl man page my.n"

_av = make_av(_SOURCE)


@register
class OoMyCommand(CommandDef):
    name = "my"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="my",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke a method on the current object",
                synopsis=("my method ?arg ...?",),
                snippet=(
                    "The my command is used within the body of a method, constructor, or "
                    "destructor to invoke a method on the current object.  It is equivalent "
                    "to [self] method ?arg ...? but avoids the overhead of determining the "
                    "object name."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="my method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "variable",
                                "Link a local variable to an object variable.",
                                "my variable varName ?varName ...?",
                            ),
                            _av(
                                "varname",
                                "Return the fully qualified name of an object variable.",
                                "my varname varName",
                            ),
                            _av("object", "Return the name of the current object.", "my object"),
                            _av(
                                "class",
                                "Return the name of the class of the current object.",
                                "my class",
                            ),
                            _av(
                                "namespace",
                                "Return the namespace of the current object.",
                                "my namespace",
                            ),
                            _av("destroy", "Destroy the current object.", "my destroy"),
                        ),
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
