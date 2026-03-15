"""Shared Tcl runtime discovery — find tclsh and tkinter.Tcl().

Both the debugger and the iRule test framework use this module to
locate available Tcl interpreters.
"""

from __future__ import annotations

import shutil


def has_tkinter_tcl() -> bool:
    """Check if ``tkinter.Tcl()`` is available and functional."""
    try:
        import tkinter

        interp = tkinter.Tcl()
        # Verify the interp actually works
        interp.eval("expr {1 + 1}")
        del interp
        return True
    except Exception:
        return False


def find_tclsh() -> str | None:
    """Find a suitable ``tclsh`` binary on PATH, or return ``None``."""
    for name in ("tclsh8.5", "tclsh8.6", "tclsh8.4", "tclsh"):
        path = shutil.which(name)
        if path:
            return path
    return None


__all__ = ["find_tclsh", "has_tkinter_tcl"]
