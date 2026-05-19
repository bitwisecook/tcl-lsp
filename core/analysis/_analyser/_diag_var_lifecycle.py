# canonicalisation: audited #246
from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from shared.naming import normalise_var_name as _normalise_var_name

from ...compiler.cfg import CFGFunction
from ...compiler.core_analyses import FunctionAnalysis
from ...compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRIncr,
    IRProcedure,
    IRStatement,
)
from ..semantic_model import Diagnostic, Severity
from ._utils import _UNUSED_VAR_RE, _format_literal_for_message, _possible_paste_fingerprint


class _AnalyserDiagVarLifecycleMixin(_Base):
    """W210/211/213/214/220 + H300 diagnostics: variable lifecycle."""

    def _emit_dead_store_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
        defined_vars: set[str] | None = None,
    ) -> None:
        existing_unused: set[tuple[str, int]] = set()
        for d in self.result.diagnostics:
            if d.code != "W211":
                continue
            m = _UNUSED_VAR_RE.match(d.message)
            if m is None:
                continue
            existing_unused.add((m.group(1), d.range.start.offset))

        all_vars = defined_vars if defined_vars is not None else self._collect_defined_vars(cfg)
        for dead in analysis.dead_stores:
            if dead.variable in cross_event_vars:
                continue
            block = cfg.blocks.get(dead.block)
            if block is None:
                continue
            if dead.statement_index < 0 or dead.statement_index >= len(block.statements):
                continue
            stmt = block.statements[dead.statement_index]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            if (dead.variable, stmt_range.start.offset) in existing_unused:
                continue
            msg = f"Assignment to '{dead.variable}' is never read"
            similar = self._find_case_mismatch(dead.variable, all_vars)
            if similar is not None:
                msg += f"; did you mean '{similar}'?"
            self.result.diagnostics.append(
                Diagnostic(
                    range=stmt_range,
                    message=msg,
                    severity=Severity.HINT,
                    code="W220",
                )
            )

    def _emit_possible_paste_error_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        """Emit low-confidence duplicate-assignment paste-error heuristics."""
        dead_store_indices: dict[str, set[int]] = {}
        for dead in analysis.dead_stores:
            dead_store_indices.setdefault(dead.block, set()).add(dead.statement_index)

        for block_name, block in cfg.blocks.items():
            dead_indices = dead_store_indices.get(block_name)
            if not dead_indices:
                continue

            statements = block.statements
            for idx in range(len(statements) - 1):
                if idx not in dead_indices:
                    continue

                first = _possible_paste_fingerprint(statements[idx])
                if first is None:
                    continue

                second = _possible_paste_fingerprint(statements[idx + 1])
                if second is None or first != second:
                    continue

                var_name, literal = first
                if var_name.startswith("_"):
                    continue

                stmt_range = getattr(statements[idx + 1], "range", None)
                if stmt_range is None:
                    continue

                self.result.diagnostics.append(
                    Diagnostic(
                        range=stmt_range,
                        message=(
                            f"Possible paste error: repeated assignment to '{var_name}' "
                            f"with static value '{_format_literal_for_message(literal)}'; "
                            "did you mean to assign a different variable?"
                        ),
                        severity=Severity.HINT,
                        code="H300",
                    )
                )

    @staticmethod
    def _collect_defined_vars(cfg: CFGFunction) -> set[str]:
        """Return all variable names defined in *cfg* (from IR statements)."""
        names: set[str] = set()
        for block in cfg.blocks.values():
            for stmt in block.statements:
                if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
                    names.add(_normalise_var_name(stmt.name))
                elif isinstance(stmt, IRCall) and stmt.defs:
                    names.update(stmt.defs)
        return names

    @staticmethod
    def _find_case_mismatch(variable: str, defined_vars: set[str]) -> str | None:
        """Return a defined variable that matches *variable* case-insensitively.

        When multiple candidates exist, the lexicographically smallest is
        returned so the suggestion is deterministic across runs.
        """
        lower = variable.lower()
        matches = sorted(
            name for name in defined_vars if name != variable and name.lower() == lower
        )
        if matches:
            return matches[0]
        return None

    @staticmethod
    def _is_safe_uninit_var(stmt: IRStatement, variable: str) -> bool:
        """Check that *variable* is the one the statement safely initialises."""
        if isinstance(stmt, IRCall) and stmt.reads_own_defs:
            return variable in stmt.defs
        if isinstance(stmt, IRIncr):
            return variable == _normalise_var_name(stmt.name)
        return False

    def _emit_read_before_set_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
        defined_vars: set[str] | None = None,
    ) -> None:
        all_vars = defined_vars if defined_vars is not None else self._collect_defined_vars(cfg)
        for rbs in analysis.read_before_set:
            block = cfg.blocks.get(rbs.block)
            if block is None:
                continue
            stmt = None
            if rbs.statement_index == -1:
                # Version-0 use in a branch condition — use terminator range.
                r = getattr(block.terminator, "range", None)
            elif 0 <= rbs.statement_index < len(block.statements):
                stmt = block.statements[rbs.statement_index]
                r = getattr(stmt, "range", None)
            else:
                continue
            if r is None:
                continue
            if isinstance(stmt, IRCall) and stmt.canonical_command == "::unset":
                # unset without -nocomplain on a possibly-undefined variable.
                # Still warn even for cross-event vars — unset is explicit.
                self.result.diagnostics.append(
                    Diagnostic(
                        range=r,
                        message=(
                            f"Variable '{rbs.variable}' may not exist; "
                            "use 'unset -nocomplain' to suppress the error"
                        ),
                        severity=Severity.WARNING,
                        code="W213",
                    )
                )
            elif rbs.variable in cross_event_vars:
                # Variable is set in another event — not a real read-before-set.
                continue
            elif (
                stmt is not None
                and getattr(stmt, "safe_on_uninit", False)
                and self._is_safe_uninit_var(stmt, rbs.variable)
            ):
                # Commands like lappend/append/dict set/incr safely
                # initialise an uninitialised variable (set from the
                # command registry at lowering time).  Only suppress for
                # the variable the command defines — other variables
                # in the same statement (e.g. $x in `lappend list $x`)
                # are genuine reads.
                continue
            else:
                msg = f"Variable '{rbs.variable}' is read before it is set"
                similar = self._find_case_mismatch(rbs.variable, all_vars)
                if similar is not None:
                    msg += f"; did you mean '{similar}'?"
                self.result.diagnostics.append(
                    Diagnostic(
                        range=r,
                        message=msg,
                        severity=Severity.WARNING,
                        code="W210",
                    )
                )

    def _emit_unused_variable_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
        defined_vars: set[str] | None = None,
    ) -> None:
        all_vars = defined_vars if defined_vars is not None else self._collect_defined_vars(cfg)
        for unused in analysis.unused_variables:
            if unused.variable in cross_event_vars:
                continue
            block = cfg.blocks.get(unused.block)
            if block is None:
                continue
            if unused.statement_index < 0 or unused.statement_index >= len(block.statements):
                continue
            stmt = block.statements[unused.statement_index]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            msg = f"Variable '{unused.variable}' is set but never used"
            similar = self._find_case_mismatch(unused.variable, all_vars)
            if similar is not None:
                msg += f"; did you mean '{similar}'?"
            self.result.diagnostics.append(
                Diagnostic(
                    range=stmt_range,
                    message=msg,
                    severity=Severity.HINT,
                    code="W211",
                )
            )

    def _emit_unused_param_diagnostics(
        self,
        ir_proc: IRProcedure,
        analysis: FunctionAnalysis,
    ) -> None:
        """W214: flag proc parameters that are never read in the body."""
        # Procs registered as ``trace`` callbacks must accept the fixed
        # trailing signature dictated by Tcl's trace API
        # (e.g. ``name1 name2 op``); the body legitimately may not use
        # those arguments, so suppress W214 entirely for them.
        proc_def = self.result.all_procs.get(ir_proc.qualified_name)
        if proc_def is not None and proc_def.is_trace_callback:
            return
        for param_name in analysis.unused_params:
            self.result.diagnostics.append(
                Diagnostic(
                    range=ir_proc.range,
                    message=(f"Parameter '{param_name}' of proc '{ir_proc.name}' is unused"),
                    severity=Severity.HINT,
                    code="W214",
                )
            )
