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

"""Taint hints for command arguments and return values.

This module is deliberately free of registry imports so command
definition files can safely import from it without introducing
circular dependencies (same pattern as ``signatures.py`` and
``type_hints.py``).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Flag, auto

from .signatures import Arity


class TaintColour(Flag):
    """Properties carried by a tainted value.

    Colours compose with ``|``.  The lattice join of two colours is
    their *intersection* (``&``): if one path has ``PATH_PREFIXED``
    and another doesn't, the join loses it.
    """

    TAINTED = auto()
    PATH_PREFIXED = auto()  # always starts with "/" (HTTP::uri, HTTP::path)
    NON_DASH_PREFIXED = auto()  # provably starts with a non-"-" literal
    CRLF_FREE = auto()  # proven to contain no CR/LF characters
    SHELL_ATOM = auto()  # token-safe atom (no shell metachar splitting)
    LIST_CANONICAL = auto()  # canonical Tcl list representation
    REGEX_LITERAL = auto()  # regex-escaped literal payload
    PATH_NORMALISED = auto()  # path has been normalised (no raw traversal form)
    PATH_BOUNDED = auto()  # normalised path verified within an intended directory
    PATH_JOINED = auto()  # assembled via [file join] (portable, not canonicalised)
    HEADER_TOKEN_SAFE = auto()  # valid HTTP header-token charset
    HTML_ESCAPED = auto()  # HTML-escaped text context
    URL_ENCODED = auto()  # URL-encoded text context
    IP_ADDRESS = auto()  # IPv4 or IPv6 address (digits, dots, colons)
    PORT = auto()  # integer 0-65535
    FQDN = auto()  # fully qualified domain name
    CHANNEL = auto()  # I/O channel handle (from open, socket, chan create, HSL::open)


@dataclass(frozen=True, slots=True)
class TaintSinkSpec:
    """Marks argument positions as dangerous for tainted data."""

    code: str  # diagnostic code (e.g. "IRULE3001")
    subcommands: frozenset[str] | None = None  # None = all invocations


@dataclass(frozen=True, slots=True)
class SetterConstraint:
    """Constraint on a setter argument."""

    arg_index: int  # which arg (0-based after command name)
    required_prefix: str  # e.g. "/"
    code: str  # diagnostic code for violations
    message: str  # human-readable explanation


@dataclass(frozen=True, slots=True)
class TaintHint:
    """Taint metadata for a command.

    Attributes:
        source: If non-None, the command's return value is tainted.
            Maps :class:`Arity` → :class:`TaintColour`.
            Use ``None`` as the key for a catch-all default.
        source_subcommands: If non-None, restricts ``source`` to only
            apply when the first argument (subcommand) is in this set.
            Used for ensemble commands like ``chan`` where only ``read``
            and ``gets`` produce tainted data.
        sinks: Argument positions that are dangerous to pass tainted data.
        setter_constraints: For setter forms, constraints the argument must satisfy.
    """

    source: dict[Arity | None, TaintColour] | None = None
    source_subcommands: frozenset[str] | None = None
    sinks: tuple[TaintSinkSpec, ...] = ()
    setter_constraints: tuple[SetterConstraint, ...] = ()
