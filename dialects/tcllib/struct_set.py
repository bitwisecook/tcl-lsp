# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""struct::set -- Set operations on lists (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

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
            # Return types verified against real tclsh 9.0.3 + tcllib 2.0
            # (SF-1 data-coverage closure): tcl::unsupported::representation
            # reports list / int / int for the union/contains/size families.
            subcommands={
                "contains": SubCommand(
                    name="contains",
                    arity=Arity(2, 2),
                    detail="Test if set contains an element.",
                    synopsis="struct::set contains set element",
                    return_type=TclType.BOOLEAN,
                ),
                "difference": SubCommand(
                    name="difference",
                    arity=Arity(2, 2),
                    detail="Return elements in A but not in B.",
                    synopsis="struct::set difference setA setB",
                    return_type=TclType.LIST,
                ),
                "empty": SubCommand(
                    name="empty",
                    arity=Arity(1, 1),
                    detail="Test if a set is empty.",
                    synopsis="struct::set empty set",
                    return_type=TclType.BOOLEAN,
                ),
                "equal": SubCommand(
                    name="equal",
                    arity=Arity(2, 2),
                    detail="Test if two sets are equal.",
                    synopsis="struct::set equal setA setB",
                    return_type=TclType.BOOLEAN,
                ),
                "exclude": SubCommand(
                    name="exclude",
                    arity=Arity(2, 2),
                    detail="Remove elements from a set variable.",
                    synopsis="struct::set exclude setVar element",
                    return_type=TclType.STRING,
                ),
                "include": SubCommand(
                    name="include",
                    arity=Arity(2, 2),
                    detail="Add an element to a set variable.",
                    synopsis="struct::set include setVar element",
                    return_type=TclType.STRING,
                ),
                "intersect": SubCommand(
                    name="intersect",
                    arity=Arity(0),
                    detail="Return the intersection of two or more sets.",
                    synopsis="struct::set intersect ?set ...?",
                    return_type=TclType.LIST,
                ),
                "subsetof": SubCommand(
                    name="subsetof",
                    arity=Arity(2, 2),
                    detail="Test if A is a subset of B.",
                    synopsis="struct::set subsetof setA setB",
                    return_type=TclType.BOOLEAN,
                ),
                "symdiff": SubCommand(
                    name="symdiff",
                    arity=Arity(2, 2),
                    detail="Return the symmetric difference of two sets.",
                    synopsis="struct::set symdiff setA setB",
                    return_type=TclType.LIST,
                ),
                "union": SubCommand(
                    name="union",
                    arity=Arity(0),
                    detail="Return the union of two or more sets.",
                    synopsis="struct::set union ?set ...?",
                    return_type=TclType.LIST,
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Return the number of elements in a set.",
                    synopsis="struct::set size set",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "intersect3": SubCommand(
                    name="intersect3",
                    arity=Arity(2, 2),
                    detail="Return a three-element list: elements only in A, elements in both, elements only in B.",
                    synopsis="struct::set intersect3 setA setB",
                    pure=True,
                    return_type=TclType.LIST,
                ),
                "add": SubCommand(
                    name="add",
                    arity=Arity(2, 2),
                    detail="Add elements of set B to set variable A.",
                    synopsis="struct::set add setVar setB",
                    return_type=TclType.STRING,
                ),
                "subtract": SubCommand(
                    name="subtract",
                    arity=Arity(2, 2),
                    detail="Remove elements of set B from set variable A.",
                    synopsis="struct::set subtract setVar setB",
                    return_type=TclType.STRING,
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
