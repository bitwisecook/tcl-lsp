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

"""Tests for debugger backend factory and shared discovery."""

from __future__ import annotations

from shared.tcl_discovery import find_tclsh, has_tkinter_tcl
from tooling.debugger.backends import create_backend
from tooling.debugger.backends.vm_backend import VmBackend


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
        """Unknown preference falls back to VM backend."""
        backend = create_backend("nonexistent_backend")
        assert isinstance(backend, VmBackend)
