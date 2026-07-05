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

"""Locate and extract embedded Tcl bodies from ``sys application template`` blocks.

Extracts ``implementation`` and ``presentation`` sections from iApp
templates in BIG-IP configuration files so they can receive full Tcl
semantic tokenisation.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ._text_utils import find_brace_end


@dataclass(frozen=True, slots=True)
class EmbeddedSection:
    """A Tcl code section inside an iApp template."""

    kind: str  # "implementation" or "presentation"
    body: str  # the Tcl source between the outermost { }
    body_start_offset: int  # offset of the first char after the opening {
    body_end_offset: int  # offset of the last char before the closing }


# Match:  implementation {  or  presentation {
_SECTION_RE = re.compile(
    r"^\s+(implementation|presentation)\s*\{",
    re.MULTILINE,
)


def find_embedded_iapp_sections(source: str) -> list[EmbeddedSection]:
    """Find ``implementation`` and ``presentation`` bodies in iApp templates."""
    sections: list[EmbeddedSection] = []
    for m in _SECTION_RE.finditer(source):
        kind = m.group(1)
        brace_pos = m.end() - 1  # position of '{'
        end_pos = find_brace_end(source, brace_pos)
        body = source[brace_pos + 1 : end_pos - 1]
        # Only include sections that actually contain Tcl code.
        if body.strip():
            sections.append(
                EmbeddedSection(
                    kind=kind,
                    body=body,
                    body_start_offset=brace_pos + 1,
                    body_end_offset=end_pos - 1,
                )
            )
    return sections
