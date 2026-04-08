"""struct::set -- Set operations on lists (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib struct::set package"
_PACKAGE = "struct::set"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av("contains", "Test if set contains an element.", "struct::set contains set element"),
    _av("difference", "Return elements in A but not in B.", "struct::set difference setA setB"),
    _av("empty", "Test if a set is empty.", "struct::set empty set"),
    _av("equal", "Test if two sets are equal.", "struct::set equal setA setB"),
    _av("exclude", "Remove elements from a set.", "struct::set exclude set element ?element ...?"),
    _av("include", "Add elements to a set.", "struct::set include set element ?element ...?"),
    _av(
        "intersect",
        "Return the intersection of two or more sets.",
        "struct::set intersect ?set ...?",
    ),
    _av("subsetof", "Test if A is a subset of B.", "struct::set subsetof setA setB"),
    _av("symdiff", "Return the symmetric difference of two sets.", "struct::set symdiff setA setB"),
    _av("union", "Return the union of two or more sets.", "struct::set union ?set ...?"),
)


@register
class StructSetCommand(CommandDef):
    name = "struct::set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Set operations on Tcl lists (union, intersect, difference, etc.).",
                synopsis=("struct::set subcommand ?args ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="struct::set subcommand ?args ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "contains": SubCommand(
                    name="contains",
                    arity=Arity(2, 2),
                    detail="Test if set contains an element.",
                    synopsis="struct::set contains set element",
                ),
                "difference": SubCommand(
                    name="difference",
                    arity=Arity(2, 2),
                    detail="Return elements in A but not in B.",
                    synopsis="struct::set difference setA setB",
                ),
                "empty": SubCommand(
                    name="empty",
                    arity=Arity(1, 1),
                    detail="Test if a set is empty.",
                    synopsis="struct::set empty set",
                ),
                "equal": SubCommand(
                    name="equal",
                    arity=Arity(2, 2),
                    detail="Test if two sets are equal.",
                    synopsis="struct::set equal setA setB",
                ),
                "exclude": SubCommand(
                    name="exclude",
                    arity=Arity(2, 2),
                    detail="Remove elements from a set variable.",
                    synopsis="struct::set exclude setVar element",
                ),
                "include": SubCommand(
                    name="include",
                    arity=Arity(2, 2),
                    detail="Add an element to a set variable.",
                    synopsis="struct::set include setVar element",
                ),
                "intersect": SubCommand(
                    name="intersect",
                    arity=Arity(0),
                    detail="Return the intersection of two or more sets.",
                    synopsis="struct::set intersect ?set ...?",
                ),
                "subsetof": SubCommand(
                    name="subsetof",
                    arity=Arity(2, 2),
                    detail="Test if A is a subset of B.",
                    synopsis="struct::set subsetof setA setB",
                ),
                "symdiff": SubCommand(
                    name="symdiff",
                    arity=Arity(2, 2),
                    detail="Return the symmetric difference of two sets.",
                    synopsis="struct::set symdiff setA setB",
                ),
                "union": SubCommand(
                    name="union",
                    arity=Arity(0),
                    detail="Return the union of two or more sets.",
                    synopsis="struct::set union ?set ...?",
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Return the number of elements in a set.",
                    synopsis="struct::set size set",
                    pure=True,
                ),
                "intersect3": SubCommand(
                    name="intersect3",
                    arity=Arity(2, 2),
                    detail="Return a three-element list: elements only in A, elements in both, elements only in B.",
                    synopsis="struct::set intersect3 setA setB",
                    pure=True,
                ),
                "add": SubCommand(
                    name="add",
                    arity=Arity(2, 2),
                    detail="Add elements of set B to set variable A.",
                    synopsis="struct::set add setVar setB",
                ),
                "subtract": SubCommand(
                    name="subtract",
                    arity=Arity(2, 2),
                    detail="Remove elements of set B from set variable A.",
                    synopsis="struct::set subtract setVar setB",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
