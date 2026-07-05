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

"""Shared CLI framework: result serialisation + output formatting.

This package is the common machinery the tcl-lsp CLI tools build on —
the result serialiser (`serialise.py`), output formatters
(`formatters.py`), and shell-completion support
(`_argcomplete_support.py`).  The reusable compiler-explorer pipeline
(source-in → structured result) lives in `tooling.explorer.pipeline`,
the per-tool verb registries live with their tools
(`tooling.tcl.verbs`, `tooling.f5.verbs`), and the F5 remote REST/UCS
client lives in `tooling.f5.f5_remote`.

It is consumed by the per-tool CLI trees — `tooling.tcl`, `tooling.f5`,
`tooling.wasm` — and by the compiler explorer (`tooling.explorer.cli`
for its CLI face, `tooling.explorer.web` for its web face).  It is NOT
itself a command; it has no `__main__`.
"""
