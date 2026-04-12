"""Tests for Tk package detection."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.semantic_model import AnalysisResult, PackageRequire, Range
from core.tk.detection import has_tk_require


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
