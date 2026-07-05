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

"""Tcl code generation — two backends behind a shared front-end.

The compiler front-end (parse → IR → CFG → SSA → lowering) is shared;
this package holds the two back-ends that consume a lowered
:class:`CFGModule` and emit executable output:

- :mod:`compiler.codegen.bytecode` — Tcl VM bytecode assembly text,
  matching ``tcl::unsupported::disassemble`` (driven by the internal
  Python VM in ``tooling/vm``).
- :mod:`compiler.codegen.wasm` — WebAssembly binary modules, plus the
  whole-program linker in :mod:`compiler.codegen.wasm.link`.

There is deliberately no parent-level re-export: a caller picks a
backend explicitly (``from compiler.codegen.bytecode import …`` /
``from compiler.codegen.wasm import …``) so the two stay symmetric and
neither is privileged as "the" codegen API.
"""

from __future__ import annotations
