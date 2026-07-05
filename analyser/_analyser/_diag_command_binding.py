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

"""W128: a command called after it was renamed/deleted earlier in the file.

Backed by the flow-sensitive command-binding lattice
(:mod:`compiler.command_binding`).  A call whose name the lattice proves is
*opaque at that point because it was rebound* — i.e. it was a real proc/builtin
that an earlier ``rename``/``rename … {}`` removed — almost certainly does not do
what the author intended: in Tcl the call falls through to the ``unknown`` handler
(it is not a hard error, hence a *warning*, not an error).

Only the top level is analysed: there the lattice (seeded with every module proc)
sees definition and rename order exactly.  A name that is merely *never defined*
in this file (an ordinary library/auto-loaded command) is **not** flagged — it
was never bound here, so it is absent from ``rebound_names`` and stays quiet.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.compilation_unit import CompilationUnit

    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.command_binding import (
    Binding,
    BindingKind,
    analyse_command_binding,
)
from compiler.ir import IRCall
from shared.naming import normalise_qualified_name as _nqn

from ..semantic_model import Diagnostic, Severity


class _AnalyserDiagCommandBindingMixin(_Base):
    """W128: call to a command renamed/deleted away earlier in the same file."""

    def _emit_renamed_command_diagnostics(self, cu: CompilationUnit) -> None:
        top = cu.top_level
        if top.complexity_guarded:
            return
        cfg = top.cfg
        # Seed with every module proc (already canonically qualified) so a proc
        # defined inside a ``namespace eval`` block is known, matching the
        # optimiser's gating view.
        seed: dict[str, Binding] = {
            qname: Binding(BindingKind.PROC, qname) for qname in cu.ir_module.procedures
        }
        binding = analyse_command_binding(cfg, initial=seed)
        # Names that were ever bound-then-perturbed in this unit; a name that is
        # merely undefined (library command) never appears here.
        rebound = binding.rebound_names()
        if not rebound:
            return

        for block_name, block in cfg.blocks.items():
            for idx, stmt in enumerate(block.statements):
                if not isinstance(stmt, IRCall) or stmt.range is None:
                    continue
                name = stmt.command
                if not name or name in ("rename", "interp", "proc"):
                    continue
                resolved = binding.binding_at(block_name, idx, name)
                if resolved.kind is not BindingKind.OPAQUE:
                    continue
                if _nqn(name) not in rebound:
                    continue  # never bound here → an ordinary external command
                self.result.diagnostics.append(
                    Diagnostic(
                        range=stmt.range,
                        message=(
                            f"Command '{name}' was renamed or deleted earlier in this "
                            "file; this call falls through to the 'unknown' handler."
                        ),
                        severity=Severity.WARNING,
                        code="W128",
                    )
                )
