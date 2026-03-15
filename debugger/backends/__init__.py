"""Debugger backend selection and auto-detection."""

from __future__ import annotations

from .base import DebugBackend


def create_backend(preference: str = "auto") -> DebugBackend:
    """Create a debug backend based on *preference*.

    When *preference* is ``"auto"`` the priority order is:
    VM > tkinter > tclsh.  The VM backend is always available and
    provides the most reliable debugging experience (proper depth
    tracking, variable inspection, and expression evaluation).
    """
    if preference == "vm":
        from .vm_backend import VmBackend

        return VmBackend()

    if preference == "tclsh":
        from core.tcl_discovery import find_tclsh

        from .tclsh_backend import TclshBackend

        tclsh = find_tclsh()
        if not tclsh:
            msg = "No tclsh found on PATH"
            raise RuntimeError(msg)
        return TclshBackend(tclsh)

    if preference == "tkinter":
        from core.tcl_discovery import has_tkinter_tcl

        from .tkinter_backend import TkinterBackend

        if not has_tkinter_tcl():
            msg = "tkinter.Tcl() not available"
            raise RuntimeError(msg)
        return TkinterBackend()

    # auto: VM > tkinter > tclsh
    # VM is always available and the most reliable.
    from .vm_backend import VmBackend

    return VmBackend()


__all__ = ["DebugBackend", "create_backend"]
