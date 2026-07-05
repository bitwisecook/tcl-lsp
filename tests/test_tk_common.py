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

"""Tests for shared Tk constants/path helpers."""

from __future__ import annotations

from analyser import analyse
from analyser.checks.tk import check_tk_diagnostics
from dialects.tk.dialect.common import is_widget_path, parent_widget_path
from dialects.tk.dialect.extract import extract_tk_layout


def test_widget_path_validation_and_parent_derivation() -> None:
    assert not is_widget_path(".")
    assert is_widget_path(".frame.child")
    assert not is_widget_path("frame.child")
    assert not is_widget_path(".1bad")

    assert parent_widget_path(".") == ""
    assert parent_widget_path(".child") == "."
    assert parent_widget_path(".root.child") == ".root"


def test_tk_extract_and_diagnostics_share_widget_path_classification() -> None:
    valid_source = "package require Tk\nframe .f\nbutton .f.b -text Ok\n"
    layout = extract_tk_layout(valid_source)
    diagnostics = check_tk_diagnostics(valid_source, analyse(valid_source))
    codes = {d.code for d in diagnostics}

    assert layout["widget_count"] >= 3  # root + frame + button
    assert "TK1002" not in codes

    invalid_source = "package require Tk\nbutton .1bad -text Nope\n"
    invalid_layout = extract_tk_layout(invalid_source)
    invalid_diags = check_tk_diagnostics(invalid_source, analyse(invalid_source))
    invalid_codes = {d.code for d in invalid_diags}

    # Both modules should treat invalid widget paths as non-widget targets.
    assert invalid_layout["widget_count"] == 1
    assert "TK1002" not in invalid_codes
