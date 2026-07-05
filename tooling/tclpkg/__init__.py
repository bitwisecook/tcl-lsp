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

"""Tcl package manager and virtual-environment system.

``tclpkg`` is a deterministic, MVS-based dependency manager for Tcl
projects, integrated into the ``tcl`` CLI at ``tooling/tcl/main.py``.  It
evaluates a ``tclpkg.tcl`` manifest in a sandboxed Tcl interpreter,
resolves the dependency graph using Go-style Minimum Version Selection,
fetches packages into a content-addressable cache keyed by SHA-256, and
materialises them into ``./lib/<pkg>-<ver>/`` or a virtual environment's
``lib/`` directory.

See ``docs/kcs/kcs-tclpkg-overview.md`` for the architecture overview and
the design plan at ``docs/kcs/kcs-tclpkg-manifest-contracts.md`` for the
manifest grammar.
"""

from __future__ import annotations

from .errors import (
    IntegrityError,
    ManifestError,
    RegistryError,
    ResolutionError,
    TclPkgError,
)
from .version import Version, VersionError

__all__ = [
    "IntegrityError",
    "ManifestError",
    "RegistryError",
    "ResolutionError",
    "TclPkgError",
    "Version",
    "VersionError",
]
