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

"""Documentation surface checks for minify/unminify features."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


def test_kcs_feature_index_lists_minify_and_unminify() -> None:
    index = _read("docs/kcs/features/README.md")
    assert "kcs-feature-minifier.md" in index
    assert "kcs-feature-unminify-error.md" in index


def test_mcp_kcs_lists_unminify_error_tool() -> None:
    kcs = _read("docs/kcs/features/kcs-feature-mcp-server.md")
    assert "`unminify_error`" in kcs
