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

"""binary -- Manipulate binary data."""

from __future__ import annotations

from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import CommandDef, register

_SOURCE = "Tcl man page binary.n"

_SUBCOMMANDS = (
    ArgumentValueSpec(value="format", detail="Format a binary string."),
    ArgumentValueSpec(value="scan", detail="Scan a binary string."),
    ArgumentValueSpec(value="encode", detail="Encode a binary string."),
    ArgumentValueSpec(value="decode", detail="Decode a binary string."),
)


@register
class BinaryCommand(CommandDef):
    name = "binary"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="binary",
            byte_compiled=True,
            hover=HoverSnippet(
                summary="Manipulate binary data",
                synopsis=(
                    "binary format formatString ?arg arg ...?",
                    "binary scan string formatString ?varName varName ...?",
                    "binary encode format ?-option value ...? data",
                    "binary decode format ?-option value ...? data",
                ),
                snippet="This command provides facilities for manipulating binary data. The principal operations are inserting values into a binary string and extracting values from a binary string.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary format formatString ?arg arg ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary scan string formatString ?varName varName ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary encode format ?-option value ...? data",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary decode format ?-option value ...? data",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "decode": SubCommand(
                    name="decode",
                    arity=Arity(2),
                    detail="Decode a binary string.",
                    return_type=TclType.BYTEARRAY,
                    arg_types={0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True)},
                ),
                "encode": SubCommand(
                    name="encode",
                    arity=Arity(2),
                    detail="Encode a binary string.",
                    return_type=TclType.STRING,
                    arg_types={0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True)},
                ),
                "format": SubCommand(
                    name="format",
                    arity=Arity(1),
                    detail="Format a binary string.",
                    return_type=TclType.BYTEARRAY,
                    arg_types={0: ArgTypeHint(expected=TclType.STRING, shimmers=True)},
                ),
                "scan": SubCommand(
                    name="scan",
                    arity=Arity(2),
                    detail="Scan a binary string.",
                    return_type=TclType.INT,
                    # D4-F2 closure: dynamic resolver instead of
                    # hard-coded slots.  ``binary scan VALUE FORMAT
                    # ?var var ...?`` -- indices are 0-based against
                    # ``args`` AFTER the ensemble strip, so VALUE=0,
                    # FORMAT=1, varName=2..len.
                    arg_role_resolver=lambda args: {
                        i: frozenset({ArgRole.VAR_WRITE}) for i in range(2, len(args))
                    },
                    arg_types={
                        0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True),
                        1: ArgTypeHint(expected=TclType.STRING, shimmers=True),
                    },
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
