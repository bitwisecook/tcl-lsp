"""struct::stack -- Stack data structure (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib struct::stack package"
_PACKAGE = "struct::stack"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av("clear", "Remove all elements from the stack.", "stackObj clear"),
    _av("destroy", "Destroy the stack object.", "stackObj destroy"),
    _av("get", "Remove and return all elements (top first).", "stackObj get"),
    _av("getr", "Remove and return all elements (bottom first).", "stackObj getr"),
    _av("peek", "Return top n elements without removing.", "stackObj peek ?count?"),
    _av("peekr", "Return bottom n elements without removing.", "stackObj peekr ?count?"),
    _av("pop", "Remove and return the top n elements.", "stackObj pop ?count?"),
    _av("push", "Push elements onto the stack.", "stackObj push item ?item ...?"),
    _av("rotate", "Rotate the top n elements by steps positions.", "stackObj rotate count steps"),
    _av("size", "Return the number of elements.", "stackObj size"),
    _av("trim", "Trim the stack to at most n elements.", "stackObj trim newsize"),
    _av("trim*", "Like trim but discard the removed elements.", "stackObj trim* newsize"),
)


@register
class StructStackCommand(CommandDef):
    name = "struct::stack"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Create and manipulate LIFO stack objects.",
                synopsis=("struct::stack ?stackName?",),
                snippet=(
                    "Creates a new stack object. Elements are pushed and popped from the top."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="struct::stack ?stackName?",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
