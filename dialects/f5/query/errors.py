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

"""Exception hierarchy for the query DSL.

Every error message is shaped so the CLI verb can prefix it with
``error:`` and present it directly to the user.  The optional
*position* on parse / lex errors lets us underline the offending span
in the source.
"""

from __future__ import annotations

from dataclasses import dataclass


class QueryError(Exception):
    """Base class for every error raised by the query DSL."""


@dataclass
class LexError(QueryError):
    """The lexer hit a character it does not understand."""

    message: str
    offset: int

    def __str__(self) -> str:  # pragma: no cover - exercised via the CLI
        return f"{self.message} at offset {self.offset}"


@dataclass
class ParseError(QueryError):
    """The parser saw a token it did not expect."""

    message: str
    offset: int

    def __str__(self) -> str:  # pragma: no cover - exercised via the CLI
        return f"{self.message} at offset {self.offset}"


class EvalError(QueryError):
    """Evaluation hit an undefined name, a type mismatch, or similar."""


class EditError(QueryError):
    """An assignment cannot be applied (conflict, non-writable path, ...)."""


class BuiltinError(QueryError):
    """A builtin function rejected its arguments."""


class RendererError(QueryError):
    """A renderer rejected its input or options.

    Raised by registered renderers (see :mod:`dialects.f5.query.renderers`)
    and by the registry itself when an unknown name is requested.  Keeping
    it under :class:`QueryError` lets the CLI's ``error:`` handling path
    treat renderer failures the same way it treats parse and evaluation
    failures.
    """
