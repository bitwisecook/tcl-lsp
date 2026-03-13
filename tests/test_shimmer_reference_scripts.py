"""Fixture-driven shimmer behaviour tests.

These tests keep shimmering scenarios as standalone Tcl scripts so they can be
reused by other tools and compared against reference runtimes.
"""

from __future__ import annotations

from pathlib import Path

from core.compiler.shimmer import find_shimmer_warnings

_FIXTURE_DIR = Path(__file__).parent / "fixtures" / "shimmer"


def _load_fixture(name: str) -> str:
    return (_FIXTURE_DIR / name).read_text(encoding="utf-8")


def _codes(source: str) -> set[str]:
    return {warning.code for warning in find_shimmer_warnings(source)}


def test_string_list_roundtrip_reports_shimmer():
    codes = _codes(_load_fixture("string_list_roundtrip.tcl"))
    assert "S100" in codes or "S101" in codes


def test_numeric_string_loop_thrash_reports_loop_shimmer():
    codes = _codes(_load_fixture("numeric_string_loop_thrash.tcl"))
    assert "S101" in codes


def test_list_string_loop_toggle_reports_loop_pressure():
    warnings = find_shimmer_warnings(_load_fixture("list_string_loop_toggle.tcl"))
    codes = {warning.code for warning in warnings}
    assert "S101" in codes
    assert any(warning.variable == "bucket" for warning in warnings)


def test_namespace_scalar_reports_shimmer_with_qualified_var_name():
    warnings = find_shimmer_warnings(_load_fixture("namespace_scalar_vs_list.tcl"))
    assert any(w.code in {"S100", "S101"} and w.variable == "::demo::payload" for w in warnings)


def test_namespace_array_reports_shimmer_on_base_array_var():
    warnings = find_shimmer_warnings(_load_fixture("namespace_array_vs_list.tcl"))
    assert any(w.code in {"S100", "S101"} and w.variable == "::demo::arr" for w in warnings)


def test_dict_list_oscillation_reports_loop_shimmer():
    codes = _codes(_load_fixture("dict_list_oscillation.tcl"))
    assert "S101" in codes or "S102" in codes


def test_boolean_int_promotion_does_not_report_shimmer():
    codes = _codes(_load_fixture("boolean_int_promotion.tcl"))
    assert not ({"S100", "S101", "S102"} & codes)
