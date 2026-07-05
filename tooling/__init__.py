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

"""Developer tools: CLIs, the bytecode VM, debugger, fuzzer, compiler explorer, codemods, and the Tcl package manager.

This is an umbrella concern. Its sub-packages are loosely coupled and
share no internal API surface beyond what they import from `compiler/`,
`analyser/`, `dialects/`, and `shared/`. Each sub-package is itself an
entry point: `tooling.tcl`, `tooling.f5`, `tooling.wasm`,
`tooling.explorer`, `tooling.vm`, `tooling.debugger`, `tooling.fuzzing`,
`tooling.tclpkg`, plus the per-codemod helpers (`refactoring/`,
`formatter/`, `minifier/`, `diagram/`, `irule_test/`).
"""
