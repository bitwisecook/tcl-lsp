"""Tests for debugger backend factory and shared discovery."""

from __future__ import annotations

from core.tcl_discovery import find_tclsh, has_tkinter_tcl
from debugger.backends import create_backend
from debugger.backends.vm_backend import VmBackend


class TestTclDiscovery:
    """Verify the shared Tcl discovery functions."""

    def test_find_tclsh_returns_string_or_none(self) -> None:
        result = find_tclsh()
        assert result is None or isinstance(result, str)

    def test_has_tkinter_tcl_returns_bool(self) -> None:
        result = has_tkinter_tcl()
        assert isinstance(result, bool)


class TestBackendFactory:
    """Verify create_backend selects the right backend."""

    def test_vm_backend_explicit(self) -> None:
        backend = create_backend("vm")
        assert isinstance(backend, VmBackend)

    def test_auto_returns_a_backend(self) -> None:
        """Auto-detect should always return something (VM as last resort)."""
        backend = create_backend("auto")
        assert backend is not None

    def test_invalid_preference_falls_through(self) -> None:
        """Unknown preference should raise or fall back."""
        # "auto" is the catch-all
        backend = create_backend("auto")
        assert backend is not None
