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

"""struct::queue -- Queue data structure (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "tcllib struct::queue package"
_PACKAGE = "struct::queue"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av("clear", "Remove all elements from the queue.", "queueObj clear"),
    _av("destroy", "Destroy the queue object.", "queueObj destroy"),
    _av("get", "Remove and return n elements from the front.", "queueObj get ?count?"),
    _av("peek", "Return n elements from the front without removing.", "queueObj peek ?count?"),
    _av("put", "Add elements to the back of the queue.", "queueObj put item ?item ...?"),
    _av("size", "Return the number of elements.", "queueObj size"),
    _av("unget", "Put an element back at the front of the queue.", "queueObj unget item"),
)


@register
class StructQueueCommand(CommandDef):
    name = "struct::queue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Create and manipulate FIFO queue objects.",
                synopsis=("struct::queue ?queueName? ?=|:=|as|deserialize source?",),
                snippet=(
                    "Creates a new queue object. Elements are added at "
                    "the back and removed from the front."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="struct::queue ?queueName? ?=|:=|as|deserialize source?",
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
