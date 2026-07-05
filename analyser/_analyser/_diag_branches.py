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

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.cfg import CFGBranch, CFGFunction
from compiler.core_analyses import FunctionAnalysis

from ..semantic_model import Diagnostic, Severity


class _AnalyserDiagBranchesMixin(_Base):
    """I230/I231 diagnostics: constant branch conditions."""

    def _emit_constant_branch_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        for branch in analysis.constant_branches:
            if branch.not_taken_target not in analysis.unreachable_blocks:
                continue
            block = cfg.blocks.get(branch.block)
            if block is None or not isinstance(block.terminator, CFGBranch):
                continue
            r = block.terminator.range
            if r is None:
                continue

            names = (branch.block, branch.taken_target, branch.not_taken_target)
            is_switch = any(name.startswith("switch_") for name in names)
            is_if = any(name.startswith("if_") for name in names)
            is_loop = any(name.startswith(("while_", "for_", "foreach_")) for name in names)

            # A constant-*true* loop condition (`while 1`, `for {…} 1 {…}`,
            # `while true`) is the idiomatic infinite loop — it exits via
            # ``break``/``return``, not the condition, so the loop-exit "branch"
            # is not really unreachable and flagging it is pure noise.  Keep
            # constant-*false* loop conditions (`while 0` — the body genuinely
            # never runs) and all if/switch constant branches.
            if is_loop and not is_if and not is_switch and branch.value:
                continue

            if is_switch:
                code = "I231"
                if branch.value:
                    msg = (
                        f"Switch condition '{branch.condition}' is always true here; "
                        "subsequent switch arms are unreachable"
                    )
                else:
                    msg = (
                        f"Switch arm condition '{branch.condition}' is always false; "
                        "this arm is unreachable"
                    )
            else:
                code = "I230"
                if branch.value:
                    msg = (
                        f"Condition '{branch.condition}' is always true; "
                        "the alternate branch is unreachable"
                    )
                else:
                    msg = (
                        f"Condition '{branch.condition}' is always false; "
                        "the alternate branch is unreachable"
                    )
                if not is_if:
                    msg = f"Branch condition '{branch.condition}' is constant; one branch is unreachable"

            self.result.diagnostics.append(
                Diagnostic(
                    range=r,
                    message=msg,
                    severity=Severity.INFO,
                    code=code,
                )
            )
