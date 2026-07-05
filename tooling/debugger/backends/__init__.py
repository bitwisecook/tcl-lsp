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
        from shared.tcl_discovery import find_tclsh

        from .tclsh_backend import TclshBackend

        tclsh = find_tclsh()
        if not tclsh:
            msg = "No tclsh found on PATH"
            raise RuntimeError(msg)
        return TclshBackend(tclsh)

    if preference == "tkinter":
        from shared.tcl_discovery import has_tkinter_tcl

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
