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

"""Tcl dialect support and command-spec data.

`dialects/` houses every variant of Tcl: the vanilla Tcl 8.4 / 8.5+ /
9.0 stdlib specs, tcllib, expect, EDA tool dialects, F5 BigIP/iRules,
and Tk. It's a data-heavy layer — command specifications, parser
rules, and dialect-specific diagnostics.

Other concerns import dialect-specific helpers (e.g. F5 BigIP parsing
lives under `dialects.f5.bigip`); the registry runtime lives in
`compiler.registry/` and consumes spec data from here through a
uniform loader.

`dialects/` may import from `shared/` and from `compiler.registry`
(registration primitives only); it must not import from `analyser/`,
`server/`, `tooling/`, `ai/`, or from compiler internals like
`compiler.codegen`, `compiler.ssa`, etc.
"""
