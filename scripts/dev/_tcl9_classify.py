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

"""Shared classifier for Tcl 9 tcltest sweep results.

Used by ``refresh_tcl9_baseline.py`` and ``diff_tcl9_tcltest.py``.
"""

from __future__ import annotations


def classify(w: dict) -> str:
    if w.get("compile_error"):
        return "compile-fail"
    if w.get("run_error"):
        return "run-trap"
    if "total" not in w:
        return "no-summary"
    failed = w.get("failed", 0)
    passed = w.get("passed", 0)
    total = w.get("total", 0)
    if total == 0:
        return "no-summary"
    if failed == 0 and passed > 0 and passed == total - w.get("skipped", 0):
        return "pass"
    if passed > 0:
        return "partial"
    return "no-pass"
