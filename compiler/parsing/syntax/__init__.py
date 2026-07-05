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

"""Canonical Tcl concrete syntax tree (red-green CST).

A lossless, position-independent green tree (:mod:`.green`) with a lazy
absolute-position red overlay (:mod:`.red`), built from the lexer stream
(:mod:`.build`).  Intended as the single representation the segmenter, AOT
lowering, formatter, minifier, and per-command tooling all consume.

See ``docs/design/compiler/syntax-tree.md``.
"""

from __future__ import annotations

from .build import build_document
from .descend import CommandBody, Descended, descend_command, descend_token
from .green import (
    GreenNode,
    GreenToken,
    GreenTrivia,
    SyntaxKind,
    TriviaKind,
)
from .red import SyntaxNode, SyntaxToken, SyntaxTree
from .segment import segments_from_document, segments_from_tree

__all__ = [
    "CommandBody",
    "Descended",
    "GreenNode",
    "GreenToken",
    "GreenTrivia",
    "SyntaxKind",
    "SyntaxNode",
    "SyntaxToken",
    "SyntaxTree",
    "TriviaKind",
    "build_document",
    "descend_command",
    "descend_token",
    "segments_from_document",
    "segments_from_tree",
]
