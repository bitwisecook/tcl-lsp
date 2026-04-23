from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...compiler.compilation_unit import FunctionUnit
from ...compiler.ir import IRCall
from ..semantic_model import Diagnostic, Severity


class _AnalyserDiagRacyMixin(_Base):
    """IRULE4005 diagnostics: racy static variable access."""

    def _emit_racy_static_diagnostics(
        self,
        fu: "FunctionUnit",
        racy_vars: frozenset[str],
    ) -> None:
        """IRULE4005: static:: variable written outside RULE_INIT and used cross-event."""
        for block in fu.ssa.blocks.values():
            for stmt in block.statements:
                ir_stmt = stmt.statement
                # Skip unset — not a real write
                if isinstance(ir_stmt, IRCall) and ir_stmt.command == "unset":
                    continue
                for name in stmt.defs:
                    if name in racy_vars:
                        self.result.diagnostics.append(
                            Diagnostic(
                                range=ir_stmt.range,
                                message=(
                                    f"Potential race: '{name}' is written outside "
                                    f"RULE_INIT and read in another event. "
                                    f"static:: variables persist across all "
                                    f"connections on the same virtual server; "
                                    f"concurrent writes can produce "
                                    f"unpredictable results."
                                ),
                                severity=Severity.WARNING,
                                code="IRULE4005",
                            )
                        )
