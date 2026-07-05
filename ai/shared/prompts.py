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

"""Dialect-aware system prompt composition for Python consumers.

Reads ``ai/prompts/manifest.json`` and assembles a composite prompt from
all markdown files whose dialect set includes the requested dialect.
"""

from __future__ import annotations

import json
from pathlib import Path

_PROMPTS_DIR = Path(__file__).resolve().parent.parent / "prompts"
_MANIFEST = json.loads((_PROMPTS_DIR / "manifest.json").read_text())


def build_prompt(dialect: str) -> str:
    """Return the composite system prompt for *dialect*.

    Concatenates all prompt fragments whose manifest entry lists *dialect*,
    separated by double newlines.
    """
    parts: list[str] = []
    for entry in _MANIFEST["prompts"]:
        if dialect in entry["dialects"]:
            path = _PROMPTS_DIR / entry["file"]
            parts.append(path.read_text())
    return "\n\n".join(parts)
