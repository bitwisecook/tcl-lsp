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

"""Shared package suggestion ranking for LSP features and commands."""

from __future__ import annotations


def rank_package_suggestions(symbol: str, package_names: list[str], limit: int) -> list[str]:
    """Rank package names against a symbol prefix heuristic."""
    query = (symbol or "").strip()
    if not query:
        return []

    prefix = query.split("::", 1)[0].lower()
    if len(prefix) < 2:
        return []

    ranked: list[tuple[int, str]] = []
    for package_name in package_names:
        lower = package_name.lower()
        if lower == prefix:
            score = 0
        elif lower.startswith(prefix):
            score = 1
        elif prefix in lower:
            score = 2
        else:
            continue
        ranked.append((score, package_name))

    ranked.sort(key=lambda item: (item[0], item[1]))
    return [name for _score, name in ranked[:limit]]
