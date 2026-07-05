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
    """Find a suitable ``tclsh`` binary on PATH, or return ``None``.

    Prefers newer versions (9.0 > 8.6 > 8.5 > 8.4) and falls back to
    the unversioned ``tclsh`` if no explicit version is found.
    """
    for name in ("tclsh9.0", "tclsh8.6", "tclsh8.5", "tclsh8.4", "tclsh"):
        path = shutil.which(name)
        if path:
            return path
    return None


__all__ = ["find_tclsh", "has_tkinter_tcl"]
