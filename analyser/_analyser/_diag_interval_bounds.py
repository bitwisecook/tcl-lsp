from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.cfg import CFGFunction
from compiler.core_analyses import FunctionAnalysis
from compiler.execution_intent import FunctionExecutionIntent
from compiler.interval_bounds import find_divide_by_zero, find_interval_bounds
from compiler.ssa import SSAFunction
from shared.codes import diag

from ..semantic_model import Diagnostic, Severity

diag(
    "W233",
    "Division or modulo by a provably-zero divisor — raises 'divide by zero' at runtime.",
    section="warning",
    ai_category="control_flow",
)


class _AnalyserDiagIntervalBoundsMixin(_Base):
    """W230/W231 (dynamic): interval-driven out-of-range index detection.

    Complements the syntactic bounds checks (``analyser/checks/_bounds.py``),
    which only fire on literal index + literal list.  This consults the
    :mod:`compiler.intervals` range domain to prove out-of-range accesses where
    the index is a ``$var`` — the cases the syntactic check deliberately skips.
    """

    def _emit_interval_bounds_diagnostics(
        self,
        cfg: "CFGFunction",
        ssa: "SSAFunction",
        analysis: FunctionAnalysis,
        intent: FunctionExecutionIntent,
    ) -> None:
        # Restrict to SCCP-reachable blocks so a dynamic index in dead code
        # (e.g. `if {0} { ... }`) doesn't warn — same discipline as the
        # divide-by-zero path below.
        executable = set(cfg.blocks) - analysis.unreachable_blocks
        findings = find_interval_bounds(cfg, ssa, intent, analysis.values, executable)
        for f in findings:
            block = cfg.blocks.get(f.block)
            if block is None:
                continue
            if f.statement_index < 0:
                # Terminator-anchored finding (e.g. ``return [lindex …]``).
                r = getattr(block.terminator, "range", None)
            elif f.statement_index < len(block.statements):
                r = getattr(block.statements[f.statement_index], "range", None)
            else:
                continue
            if r is None:
                continue
            iv = f.index_interval
            bound = "below 0" if f.reason == "negative" else f"past the end ({f.length})"
            if f.reason == "negative":
                rng = "negative"
            elif iv.lo == iv.hi:
                rng = f"is {iv.lo}"
            else:
                rng = f"is in [{iv.lo}, {'+inf' if iv.hi is None else iv.hi}]"
            if f.code == "W231":
                outcome = "raises 'index out of range' at runtime"
            else:
                # W230 (lindex) / W232 (string index): silent empty result.
                outcome = "silently returns the empty string"
            msg = f"{f.command}: index ${f.index_var} {rng}, {bound} — {outcome}."
            self.result.diagnostics.append(
                Diagnostic(
                    range=r,
                    message=msg,
                    severity=Severity.WARNING,
                    code=f.code,
                )
            )

        # W233: division / modulo by a provably-zero divisor (only in
        # SCCP-reachable blocks — see find_divide_by_zero for the soundness).
        executable = set(cfg.blocks) - analysis.unreachable_blocks
        for dz in find_divide_by_zero(cfg, ssa, analysis.values, executable):
            block = cfg.blocks.get(dz.block)
            if block is None:
                continue
            if dz.statement_index < 0:
                r = getattr(block.terminator, "range", None)
            elif dz.statement_index < len(block.statements):
                r = getattr(block.statements[dz.statement_index], "range", None)
            else:
                continue
            if r is None:
                continue
            verb = "Division" if dz.op == "/" else "Modulo"
            self.result.diagnostics.append(
                Diagnostic(
                    range=r,
                    message=(
                        f"{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                    ),
                    severity=Severity.WARNING,
                    code="W233",
                )
            )
