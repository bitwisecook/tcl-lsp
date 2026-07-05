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

"""Tcl source code formatter."""

from shared.docstrings import extract_body_docstring

from .config import BraceStyle, DocstringStyle, DocstringTagStyle, FormatterConfig, IndentStyle
from .docstring import (
    DocstringInfo,
    ParamDoc,
    format_docstring,
    generate_stub,
    generate_stub_for_proc,
    parse_docstring,
    render_comment_block,
    render_markdown,
    resolve_tag_style,
)
from .formatter import format_body, format_tcl

__all__ = [
    "BraceStyle",
    "DocstringInfo",
    "DocstringStyle",
    "DocstringTagStyle",
    "FormatterConfig",
    "IndentStyle",
    "ParamDoc",
    "extract_body_docstring",
    "format_body",
    "format_docstring",
    "format_tcl",
    "generate_stub",
    "generate_stub_for_proc",
    "parse_docstring",
    "render_comment_block",
    "render_markdown",
    "resolve_tag_style",
]
