"""struct::list -- Advanced list operations (tcllib)."""

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

_SOURCE = "tcllib struct::list package"
_PACKAGE = "struct::list"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av(
        "assign", "Assign list elements to variables.", "struct::list assign sequence var ?var ...?"
    ),
    _av(
        "dbJoin",
        "Perform a relational join on two lists.",
        "struct::list dbJoin ?-inner|-left|-right|-full? ?-keys varname? keyedList1 keyedList2",
    ),
    _av(
        "dbJoinKeyed",
        "Relational join on keyed lists.",
        "struct::list dbJoinKeyed ?options? keyedList1 keyedList2",
    ),
    _av("equal", "Test if two lists are structurally equal.", "struct::list equal a b"),
    _av(
        "filter", "Return elements matching a condition.", "struct::list filter sequence cmdprefix"
    ),
    _av(
        "filterfor",
        "Filter using an expression over each element.",
        "struct::list filterfor var sequence expr",
    ),
    _av(
        "flatten",
        "Flatten nested lists by one or more levels.",
        "struct::list flatten ?-full? ?--? sequence",
    ),
    _av(
        "fold",
        "Left-fold a list with an accumulator.",
        "struct::list fold sequence initialValue cmdprefix",
    ),
    _av(
        "foreachperm",
        "Iterate over all permutations of a list.",
        "struct::list foreachperm var sequence body",
    ),
    _av("iota", "Generate a list of integers 0..n-1.", "struct::list iota n"),
    _av(
        "lcsInvert",
        "Invert a longest-common-subsequence result.",
        "struct::list lcsInvert lcsData len1 len2",
    ),
    _av(
        "longestCommonSubsequence",
        "Find the longest common subsequence of two lists.",
        "struct::list longestCommonSubsequence list1 list2 ?maxOccurs?",
    ),
    _av(
        "map",
        "Apply a command to each element and collect results.",
        "struct::list map sequence cmdprefix",
    ),
    _av(
        "mapfor",
        "Map using an expression over each element.",
        "struct::list mapfor var sequence expr",
    ),
    _av(
        "repeat",
        "Create a list by repeating elements.",
        "struct::list repeat count element ?element ...?",
    ),
    _av("reverse", "Reverse the order of a list.", "struct::list reverse sequence"),
    _av("shift", "Remove and return the first element.", "struct::list shift listVar"),
    _av("shuffle", "Randomly reorder elements of a list.", "struct::list shuffle list"),
    _av("swap", "Swap two elements in a list.", "struct::list swap listVar i j"),
)


@register
class StructListCommand(CommandDef):
    name = "struct::list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Advanced list manipulation commands.",
                synopsis=("struct::list subcommand ?args ...?",),
                snippet=(
                    "Provides operations beyond the core Tcl list commands: "
                    "filtering, mapping, folding, shuffling, permutations, "
                    "longest-common-subsequence, and more."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="struct::list subcommand ?args ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "assign": SubCommand(
                    name="assign",
                    arity=Arity(2),
                    detail="Assign list elements to variables.",
                    synopsis="struct::list assign sequence var ?var ...?",
                ),
                "dbJoin": SubCommand(
                    name="dbJoin",
                    arity=Arity(2),
                    detail="Perform a relational join on two lists.",
                    synopsis="struct::list dbJoin ?-inner|-left|-right|-full? ?-keys varname? keyedList1 keyedList2",
                ),
                "dbJoinKeyed": SubCommand(
                    name="dbJoinKeyed",
                    arity=Arity(2),
                    detail="Relational join on keyed lists.",
                    synopsis="struct::list dbJoinKeyed ?options? keyedList1 keyedList2",
                ),
                "equal": SubCommand(
                    name="equal",
                    arity=Arity(2, 2),
                    detail="Test if two lists are structurally equal.",
                    synopsis="struct::list equal a b",
                ),
                "filter": SubCommand(
                    name="filter",
                    arity=Arity(2, 2),
                    detail="Return elements matching a condition.",
                    synopsis="struct::list filter sequence cmdprefix",
                ),
                "filterfor": SubCommand(
                    name="filterfor",
                    arity=Arity(3, 3),
                    detail="Filter using an expression over each element.",
                    synopsis="struct::list filterfor var sequence expr",
                ),
                "flatten": SubCommand(
                    name="flatten",
                    arity=Arity(1),
                    detail="Flatten nested lists by one or more levels.",
                    synopsis="struct::list flatten ?-full? ?--? sequence",
                ),
                "fold": SubCommand(
                    name="fold",
                    arity=Arity(3, 3),
                    detail="Left-fold a list with an accumulator.",
                    synopsis="struct::list fold sequence initialValue cmdprefix",
                ),
                "foreachperm": SubCommand(
                    name="foreachperm",
                    arity=Arity(3, 3),
                    detail="Iterate over all permutations of a list.",
                    synopsis="struct::list foreachperm var sequence body",
                ),
                "iota": SubCommand(
                    name="iota",
                    arity=Arity(1, 1),
                    detail="Generate a list of integers 0..n-1.",
                    synopsis="struct::list iota n",
                ),
                "lcsInvert": SubCommand(
                    name="lcsInvert",
                    arity=Arity(3, 3),
                    detail="Invert a longest-common-subsequence result.",
                    synopsis="struct::list lcsInvert lcsData len1 len2",
                ),
                "longestCommonSubsequence": SubCommand(
                    name="longestCommonSubsequence",
                    arity=Arity(2, 3),
                    detail="Find the longest common subsequence of two lists.",
                    synopsis="struct::list longestCommonSubsequence list1 list2 ?maxOccurs?",
                ),
                "map": SubCommand(
                    name="map",
                    arity=Arity(2, 2),
                    detail="Apply a command to each element and collect results.",
                    synopsis="struct::list map sequence cmdprefix",
                ),
                "mapfor": SubCommand(
                    name="mapfor",
                    arity=Arity(3, 3),
                    detail="Map using an expression over each element.",
                    synopsis="struct::list mapfor var sequence expr",
                ),
                "repeat": SubCommand(
                    name="repeat",
                    arity=Arity(2),
                    detail="Create a list by repeating elements.",
                    synopsis="struct::list repeat count element ?element ...?",
                ),
                "reverse": SubCommand(
                    name="reverse",
                    arity=Arity(1, 1),
                    detail="Reverse the order of a list.",
                    synopsis="struct::list reverse sequence",
                ),
                "shift": SubCommand(
                    name="shift",
                    arity=Arity(1, 1),
                    detail="Remove and return the first element.",
                    synopsis="struct::list shift listVar",
                ),
                "shuffle": SubCommand(
                    name="shuffle",
                    arity=Arity(1, 1),
                    detail="Randomly reorder elements of a list.",
                    synopsis="struct::list shuffle list",
                ),
                "swap": SubCommand(
                    name="swap",
                    arity=Arity(3, 3),
                    detail="Swap two elements in a list.",
                    synopsis="struct::list swap listVar i j",
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
