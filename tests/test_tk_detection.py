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

"""Tests for Tk package detection."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.semantic_model import AnalysisResult, PackageRequire, Range
from analyser.tk_detection import has_tk_require


def _make_analysis(package_names: list[str]) -> AnalysisResult:
    """Build a minimal AnalysisResult with the given package requires."""
    result = AnalysisResult()
    zero = Range.zero()
    result.package_requires = [
        PackageRequire(name=n, version=None, range=zero) for n in package_names
    ]
    return result


class TestHasTkRequire:
    def test_no_packages(self):
        assert not has_tk_require(_make_analysis([]))

    def test_other_package(self):
        assert not has_tk_require(_make_analysis(["http"]))

    def test_tk_present(self):
        assert has_tk_require(_make_analysis(["Tk"]))

    def test_tk_among_others(self):
        assert has_tk_require(_make_analysis(["http", "Tk", "tls"]))

    def test_case_sensitive(self):
        """Tk detection is case-sensitive: 'tk' should not match 'Tk'."""
        assert not has_tk_require(_make_analysis(["tk"]))

    def test_tk_only_package(self):
        assert has_tk_require(_make_analysis(["Tk"]))
