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

"""Pipeline-level tests for shared CompilationUnit usage in explorer."""

from __future__ import annotations

from unittest.mock import patch

from tooling.explorer.pipeline import run_pipeline


def test_run_pipeline_compiles_once():
    source = "set x [expr {1 + 2}]\n"

    from compiler import compilation_unit as cu_module

    real_compile_source = cu_module.compile_source
    with patch(
        "compiler.compilation_unit.compile_source", wraps=real_compile_source
    ) as mocked_compile:
        result = run_pipeline(source)

        assert mocked_compile.call_count == 1
        assert result.source == source
