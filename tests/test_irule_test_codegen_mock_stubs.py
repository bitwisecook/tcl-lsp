"""Smoke tests for iRule mock stub code generation."""

from __future__ import annotations

from core.irule_test.codegen_mock_stubs import _generate


def test_codegen_mock_stubs_generates_tcl() -> None:
    generated = _generate()
    assert "namespace eval ::itest::cmd {" in generated
    assert "proc " in generated
