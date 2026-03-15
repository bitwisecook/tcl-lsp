"""Debugger backend selection and auto-detection."""

from __future__ import annotations

from .base import DebugBackend


def create_backend(preference: str = "auto") -> DebugBackend:
    """Create a debug backend based on *preference*.

    When *preference* is ``"auto"`` the priority order is:
    tclsh > tkinter > VM.
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

    # auto: tclsh > tkinter > VM
    from core.tcl_discovery import find_tclsh, has_tkinter_tcl

    tclsh = find_tclsh()
    if tclsh:
        from .tclsh_backend import TclshBackend

        return TclshBackend(tclsh)

    if has_tkinter_tcl():
        from .tkinter_backend import TkinterBackend

        return TkinterBackend()

    from .vm_backend import VmBackend

    return VmBackend()


__all__ = ["DebugBackend", "create_backend"]
