from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...compiler.cfg import CFGBranch, CFGFunction
from ...compiler.core_analyses import FunctionAnalysis
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
