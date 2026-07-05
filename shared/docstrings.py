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

"""Tcl-text-generic docstring helpers (leaf utilities).

Lives in ``shared/`` because both the IDE analyser (proc-doc semantic
enrichment) and the developer formatter (docstring rendering) need to
pull the leading comment block off a proc body, and ``shared/`` is the
only place both may import from without crossing a layering contract.
"""

from __future__ import annotations

# Characters that, on their own, make a comment line pure decoration
# (``# -----`` / ``# .....`` / ``# =====``) rather than documentation.
_DECORATION_CHARS = frozenset(".-=*~#")


def extract_body_docstring(body: str) -> str:
    """Extract the leading comment block from a proc body.

    Returns the accumulated comment text (lines joined with newlines) if
    the body starts with one or more comment lines, otherwise returns an
    empty string.  Decoration lines consisting only of dots, dashes,
    hashes, or similar characters are stripped.
    """
    lines: list[str] = []
    for raw_line in body.splitlines():
        stripped = raw_line.strip()
        if not stripped:
            if lines:
                break
            continue
        if stripped.startswith("#"):
            text = stripped.lstrip("#").strip()
            # Skip hash-only decoration lines
            if not text and set(stripped) <= {"#"}:
                continue
            # Skip decoration lines (dots, dashes, equals, etc.)
            if text and all(ch in _DECORATION_CHARS for ch in text):
                continue
            lines.append(text)
        else:
            break
    return "\n".join(lines)
