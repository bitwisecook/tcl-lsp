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

"""Formatting provider -- textDocument/formatting and textDocument/rangeFormatting."""

from __future__ import annotations

from dataclasses import replace

from lsprotocol import types

from tooling.formatter import FormatterConfig, IndentStyle, format_body, format_tcl


def get_formatting(
    source: str,
    options: types.FormattingOptions,
    config: FormatterConfig | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.TextEdit]:
    """Format the entire document, returning a list of TextEdits."""
    if config is None:
        config = FormatterConfig()

    config = _apply_lsp_options(config, options)
    formatted = format_tcl(source, config)

    if formatted == source:
        return []

    # Replace the entire document
    if lines is None:
        lines = source.split("\n")
    last_line = len(lines) - 1
    last_char = len(lines[-1]) if lines else 0

    return [
        types.TextEdit(
            range=types.Range(
                start=types.Position(line=0, character=0),
                end=types.Position(line=last_line, character=last_char),
            ),
            new_text=formatted,
        )
    ]


def get_range_formatting(
    source: str,
    range_: types.Range,
    options: types.FormattingOptions,
    config: FormatterConfig | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.TextEdit]:
    """Format a range within the document."""
    if config is None:
        config = FormatterConfig()

    config = _apply_lsp_options(config, options)

    if lines is None:
        lines = source.split("\n")
    start_line = range_.start.line
    end_line = range_.end.line

    # Clamp to valid line range
    start_line = max(0, min(start_line, len(lines) - 1))
    end_line = max(0, min(end_line, len(lines) - 1))

    range_lines = lines[start_line : end_line + 1]
    range_source = "\n".join(range_lines)

    # Detect current indent level of the first line
    indent_level = _detect_indent_level(range_lines[0], config) if range_lines else 0

    formatted = format_body(range_source, config, indent_level=indent_level)

    # Apply trailing whitespace trimming
    if config.trim_trailing_whitespace:
        formatted = "\n".join(line.rstrip() for line in formatted.split("\n"))

    if formatted == range_source:
        return []

    end_char = len(lines[end_line]) if end_line < len(lines) else 0
    return [
        types.TextEdit(
            range=types.Range(
                start=types.Position(line=start_line, character=0),
                end=types.Position(line=end_line, character=end_char),
            ),
            new_text=formatted,
        )
    ]


def _apply_lsp_options(
    config: FormatterConfig,
    options: types.FormattingOptions,
) -> FormatterConfig:
    """Apply LSP FormattingOptions to our config."""
    trim = getattr(options, "trim_trailing_whitespace", None)
    final_nl = getattr(options, "insert_final_newline", None)

    return replace(
        config,
        indent_size=options.tab_size,
        indent_style=IndentStyle.TABS if not options.insert_spaces else IndentStyle.SPACES,
        trim_trailing_whitespace=trim if trim is not None else config.trim_trailing_whitespace,
        ensure_final_newline=final_nl if final_nl is not None else config.ensure_final_newline,
    )


def _detect_indent_level(line: str, config: FormatterConfig) -> int:
    """Detect the indent level of a line based on its leading whitespace."""
    spaces = len(line) - len(line.lstrip())
    if config.indent_style == IndentStyle.TABS:
        return line.count("\t", 0, spaces)
    if config.indent_size > 0:
        return spaces // config.indent_size
    return 0
