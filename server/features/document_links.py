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

"""Document link provider -- make 'source' paths and 'package require' clickable."""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, Range
from compiler.parsing.command_segmenter import segment_commands
from server._lsp_conv import to_lsp_range
from shared.tokens import TokenType


def _collect_source_links(source: str) -> list[types.DocumentLink]:
    """Find 'source filename' commands and create links for the filename."""
    links: list[types.DocumentLink] = []
    for cmd in segment_commands(source):
        if not cmd.texts:
            continue
        if cmd.texts[0] == "source" and len(cmd.argv) >= 2:
            path_tok = cmd.argv[1]
            if path_tok.type in (TokenType.ESC, TokenType.STR):
                path_text = path_tok.text
                # Skip variable references and command substitutions
                if "$" in path_text or "[" in path_text:
                    continue
                links.append(
                    types.DocumentLink(
                        range=to_lsp_range(Range(start=path_tok.start, end=path_tok.end)),
                        tooltip=f"Open {path_text}",
                    )
                )
    return links


def _collect_package_require_links(
    analysis: AnalysisResult,
) -> list[types.DocumentLink]:
    """Create links for 'package require' statements."""
    links: list[types.DocumentLink] = []
    for pkg in analysis.package_requires:
        tooltip = f"package require {pkg.name}"
        if pkg.version:
            tooltip += f" {pkg.version}"
        links.append(
            types.DocumentLink(
                range=to_lsp_range(pkg.range),
                tooltip=tooltip,
            )
        )
    return links


def get_document_links(
    source: str,
    analysis: AnalysisResult | None = None,
) -> list[types.DocumentLink]:
    """Return document links for source commands and package requires."""
    if analysis is None:
        analysis = analyse(source)

    links: list[types.DocumentLink] = []
    links.extend(_collect_source_links(source))
    links.extend(_collect_package_require_links(analysis))
    return links
