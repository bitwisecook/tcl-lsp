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

"""Shared Tcl language constants.

Canonical definitions for boolean literals and their prefix forms
as accepted by ``Tcl_GetBoolean``.
"""

from __future__ import annotations

# Full-word boolean literals (for exact matching in non-expr contexts).
TCL_BOOL_TRUE = frozenset({"true", "yes", "on"})
TCL_BOOL_FALSE = frozenset({"false", "no", "off"})
TCL_BOOL_LITERALS = TCL_BOOL_TRUE | TCL_BOOL_FALSE

# Tcl accepts unique prefixes of boolean words (Tcl_GetBoolean).
# See Tcl 9.0 test suite: ``string is true TrU`` → 1, ``string is true ye`` → 1.
TCL_BOOL_TRUE_PREFIXES = frozenset({"t", "tr", "tru", "true", "y", "ye", "yes", "on"})
TCL_BOOL_FALSE_PREFIXES = frozenset({"f", "fa", "fal", "fals", "false", "n", "no", "of", "off"})
TCL_BOOL_ALL_PREFIXES = TCL_BOOL_TRUE_PREFIXES | TCL_BOOL_FALSE_PREFIXES
