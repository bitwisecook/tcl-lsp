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

"""Stub command/expression definitions parsed from structured comments.

The `# tcl-lsp: stubs-begin … stubs-end` blocks let users declare extra
command signatures (and expression functions / operators) for dialect
extensions the registry doesn't know about. The compiler's signature
overlay (`compiler.registry.runtime.stub_signature_scope`) folds
:class:`StubCommandDef` and :class:`StubExprDef` into the active
signature map so diagnostics, completion, and semantic understanding
work even without a full registry entry.

Lives in `compiler.registry/` because stub data is registry input.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from shared.diagnostic import Range


@dataclass(frozen=True, slots=True)
class StubArgDef:
    """A parameter in a stub command definition."""

    name: str
    role: str = "value"  # "body", "expr", "var", "var_read", "name", "pattern", "channel", "value"
    optional: bool = False


@dataclass(frozen=True, slots=True)
class StubCommandDef:
    """A command stub defined via ``# tcl-lsp: stub`` structured comment.

    Allows users to declare command signatures for unknown dialect
    extensions so the LSP can provide diagnostics, completion, and
    semantic understanding without a full registry entry.

    ``subcommand`` is the optional dispatch word for ensemble-style
    commands.  ``stub db eval {sql script:body}`` parses as
    ``name="db"``, ``subcommand="eval"``, args after the subcommand
    word.  Multiple stubs with the same ``name`` but different
    ``subcommand`` values fold into a single :class:`SubcommandSig`
    in the signature overlay so consumers can dispatch on the actual
    subcommand at the call site.
    """

    name: str
    args: tuple[StubArgDef, ...]
    range: Range
    barrier: bool = False  # creates_dynamic_barrier
    loop: bool = False  # has_loop_body
    pure: bool = False
    mutator: bool = False
    unsafe: bool = False
    scope_alias: bool = False  # creates_scope_alias (upvar-like)
    subcommand: str | None = None


@dataclass(frozen=True, slots=True)
class StubExprDef:
    """An expression function or operator stub defined via structured comment.

    Allows users to declare custom math functions or infix operators
    for dialects that extend the expr sub-language.
    """

    name: str
    kind: str  # "function" or "operator"
    arity: int = 1  # number of arguments (functions) or operands (operators)
    pure: bool = True
    range: Range = field(default_factory=Range.zero)
