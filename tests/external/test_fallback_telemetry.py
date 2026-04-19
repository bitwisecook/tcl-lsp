"""Regression tests for Tcl 9 fallback-site telemetry (triage).

These tests cover the pre-run ``DiagMap`` summarisation fed into
``tmp/tcl9-report.json``: a compile of a script that cannot be
specialised produces a non-zero fallback count, and the count is
bucketed by command + kind so the triage renderer can surface
per-file coverage.
"""

from __future__ import annotations

from tests.external.run_tcl9_tests import _summarise_diag
from tests.test_wasm_real_tcl import _compile_tcl_with_diag


def test_unknown_command_registers_fallback_site() -> None:
    """An unknown command lowers to IRBarrier and emits a diag site."""
    _wasm, diag = _compile_tcl_with_diag("foo 1 2\n", "unknown_cmd.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] >= 1
    # Every site is counted; "foo" is the only command here that
    # could trigger a fallback.
    commands = [cmd for cmd, _ in summary["top_fallback_commands"]]
    assert "foo" in commands


def test_summary_empty_for_pure_compile() -> None:
    """``set x 1`` lowers entirely inline; no diag sites emitted."""
    _wasm, diag = _compile_tcl_with_diag("set x 1\n", "pure.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] == 0
    assert summary["fallback_sites_by_kind"] == {}
    assert summary["top_fallback_commands"] == []


def test_summary_none_diag_returns_zeroes() -> None:
    """Compile failed before a DiagMap could be produced — all zeroes."""
    summary = _summarise_diag(None)
    assert summary == {
        "fallback_sites_total": 0,
        "fallback_sites_by_kind": {},
        "top_fallback_commands": [],
    }


def test_kind_bucketing_distinguishes_fallback_from_unsupported() -> None:
    """Kind histogram must separate fallback from unsupported sites.

    Any script that compiles but cannot specialise and must defer to
    the runtime should register at least one ``fallback`` site. The
    histogram is keyed by ``DiagSite.kind`` so the triage table can
    track architectural dead-ends separately from relaxable barriers.
    """
    _wasm, diag = _compile_tcl_with_diag(
        "proc p {} { unknown_user_cmd a b }\np\n",
        "kinds.tcl",
    )
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] >= 1
    assert "fallback" in summary["fallback_sites_by_kind"]
    assert summary["fallback_sites_by_kind"]["fallback"] >= 1
