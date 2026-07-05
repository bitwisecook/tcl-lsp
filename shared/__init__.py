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

"""Cross-cutting utilities shared by every concern.

`shared/` is a leaf package: it must not import from any sibling concern
(compiler, analyser, server, tooling, ai, dialects). It exposes the small
amount of infrastructure that those concerns all rely on — diagnostic codes,
source positions, ranges, document buffers, the WASM-runtime locator, the
tclsh discoverer, KCS help data, and the wire-format sentinels that the
compiler and the VM agree on.

Imports from this package are always safe.
"""
