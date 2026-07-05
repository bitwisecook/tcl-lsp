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

"""Type hints for command arguments and return values.

This module is deliberately free of registry imports so command
definition files can safely import from it without introducing
circular dependencies (same pattern as ``signatures.py``).
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

from compiler.types import TclType


@dataclass(frozen=True, slots=True)
class ArgTypeHint:
    """What intrep a command expects for an argument position.

    Attributes:
        expected: The Tcl type the argument should already have.
            ``None`` means no constraint.
        shimmers: ``True`` if the command forces conversion to this type,
            destroying the previous intrep.
    """

    expected: TclType | None = None
    shimmers: bool = False


@dataclass(frozen=True, slots=True)
class CommandTypeHint:
    """Type metadata for a simple command.

    Attributes:
        return_type: The Tcl type the command produces.
            ``None`` means unknown / variable.
        arg_types: Maps argument index (0-based after command name)
            to the expected type for that argument.
    """

    return_type: TclType | None = None
    arg_types: dict[int, ArgTypeHint] = field(default_factory=dict)
    arg_type_resolver: Callable[[tuple[str, ...]], dict[int, ArgTypeHint]] | None = field(
        default=None, hash=False, compare=False
    )


@dataclass(frozen=True, slots=True)
class SubcommandTypeHint:
    """Type metadata for a subcommand-based command.

    Attributes:
        subcommands: Maps subcommand name to its type hints.
    """

    subcommands: dict[str, CommandTypeHint] = field(default_factory=dict)
