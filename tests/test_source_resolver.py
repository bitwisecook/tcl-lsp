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

"""Tests for the source command path resolver."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.source_resolver import resolve_source_target


class TestResolveSourceTarget:
    """Verify resolve_source_target path resolution."""

    def test_literal_relative_to_script(self, tmp_path):
        script = tmp_path / "main.tcl"
        lib = tmp_path / "lib.tcl"
        script.touch()
        lib.touch()
        result = resolve_source_target("lib.tcl", True, str(script))
        assert result == str(lib)

    def test_literal_subdir_relative_to_script(self, tmp_path):
        script = tmp_path / "main.tcl"
        subdir = tmp_path / "lib"
        subdir.mkdir()
        helper = subdir / "helper.tcl"
        script.touch()
        helper.touch()
        result = resolve_source_target("lib/helper.tcl", True, str(script))
        assert result == str(helper)

    def test_literal_not_found_returns_none(self, tmp_path):
        script = tmp_path / "main.tcl"
        script.touch()
        result = resolve_source_target("nonexistent.tcl", True, str(script))
        assert result is None

    def test_literal_workspace_root_fallback(self, tmp_path):
        project = tmp_path / "project"
        project.mkdir()
        script = project / "src" / "main.tcl"
        script.parent.mkdir()
        script.touch()
        lib = project / "lib.tcl"
        lib.touch()
        # Not relative to script dir, but relative to workspace root
        result = resolve_source_target("lib.tcl", True, str(script), [str(project)])
        assert result == str(lib)

    def test_dynamic_path_returns_none(self):
        result = resolve_source_target("$dir/foo.tcl", False, "/some/script.tcl")
        assert result is None

    def test_file_join_info_script_idiom(self, tmp_path):
        script = tmp_path / "main.tcl"
        helper = tmp_path / "helper.tcl"
        script.touch()
        helper.touch()
        raw = "[file join [file dirname [info script]] helper.tcl]"
        result = resolve_source_target(raw, False, str(script))
        assert result == str(helper)

    def test_absolute_path(self, tmp_path):
        target = tmp_path / "abs.tcl"
        target.touch()
        result = resolve_source_target(str(target), True, "/other/script.tcl")
        assert result == str(target)
