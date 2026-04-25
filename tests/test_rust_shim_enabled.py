"""Helper-level tests for ``core.compiler.rust_spans.rust_shim_enabled``.

The dispatcher tests in
``tests/test_rust_signature_scan_differential.py`` (and the matching
files for `OPTIMISER` / `INTERPROC` / `GVN`) cover end-to-end
behaviour through their respective shims; this file pins the helper
contract directly so a polarity bug in ``rust_shim_enabled`` is
caught at its source instead of via a per-shim regression.

The helper is the single point of truth for the
``TCL_LSP_RUST_*`` opt-in / opt-out vocabulary, so every truthy /
falsy / unrecognised case is enumerated explicitly.
"""

from __future__ import annotations

import pytest

from core.compiler.rust_spans import rust_shim_enabled

_VAR = "TCL_LSP_RUST_TEST_HELPER"

_TRUTHY = ["1", "true", "yes", "on", "y", "t", "TRUE", "On", " 1 "]
_FALSY = ["0", "false", "no", "off", "n", "f", "FALSE", "No", " 0 "]
_UNRECOGNISED = ["", "banana", "2", "yesplease", "  "]


@pytest.mark.parametrize("value", _TRUTHY)
def test_truthy_value_enables_regardless_of_default(monkeypatch, value: str) -> None:
    monkeypatch.setenv(_VAR, value)
    assert rust_shim_enabled(_VAR) is True
    assert rust_shim_enabled(_VAR, default=True) is True
    assert rust_shim_enabled(_VAR, default=False) is True


@pytest.mark.parametrize("value", _FALSY)
def test_falsy_value_disables_regardless_of_default(monkeypatch, value: str) -> None:
    monkeypatch.setenv(_VAR, value)
    assert rust_shim_enabled(_VAR) is False
    assert rust_shim_enabled(_VAR, default=True) is False
    assert rust_shim_enabled(_VAR, default=False) is False


@pytest.mark.parametrize("value", _UNRECOGNISED)
def test_unrecognised_value_falls_through_to_default(monkeypatch, value: str) -> None:
    monkeypatch.setenv(_VAR, value)
    assert rust_shim_enabled(_VAR) is False
    assert rust_shim_enabled(_VAR, default=False) is False
    assert rust_shim_enabled(_VAR, default=True) is True


def test_unset_var_falls_through_to_default(monkeypatch) -> None:
    monkeypatch.delenv(_VAR, raising=False)
    assert rust_shim_enabled(_VAR) is False
    assert rust_shim_enabled(_VAR, default=False) is False
    assert rust_shim_enabled(_VAR, default=True) is True
