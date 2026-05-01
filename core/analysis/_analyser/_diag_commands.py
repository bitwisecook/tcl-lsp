# canonicalisation: audited #246
from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...commands.registry import REGISTRY
from ...common.dialect import active_dialect
from ...compiler.compilation_unit import CompilationUnit
from ...compiler.core_analyses import LatticeValue
from ...compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRIncr,
)
from ...parsing.known_commands import known_command_names
from ..semantic_model import (
    CodeFix,
    Diagnostic,
    Severity,
)

log = logging.getLogger(__name__)


class _AnalyserDiagCommandsMixin(_Base):
    """W123 diagnostics: unresolved commands."""

    if TYPE_CHECKING:

        def _emit_unresolved_command_diagnostics(self, *a: Any, **kw: Any) -> None: ...

    def _emit_unresolved_command_diagnostics(
        self,
        cu: CompilationUnit | None = None,
    ) -> None:
        """Emit W123 for commands that cannot be resolved."""
        if self._unresolved_commands_emitted:
            return
        self._unresolved_commands_emitted = True

        if "W123" in self._disabled_diagnostics:
            return

        from ...common.text import suggest_similar

        dialect = active_dialect()
        registry_names = frozenset(REGISTRY.command_names(dialect))
        stub_names = frozenset(s.name for s in self.result.stub_commands)

        proc_tail_names: set[str] = set()
        for qname in self.result.all_procs:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                proc_tail_names.add(tail)

        upi = self.result.unknown_proc_info
        if upi is not None and (
            upi.chains_original
            or upi.has_exec
            or upi.has_auto_load
            or upi.case_insensitive
            or upi.has_pattern_dispatch
        ):
            return

        if self.result.has_dynamic_providers:
            return

        if self.result.package_requires:
            return

        dispatch_targets = upi.dispatch_targets if upi is not None else frozenset()

        alias_names: set[str] = set()
        for qname in self.result.command_aliases:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                alias_names.add(tail)

        candidates: set[str] = set()
        candidates.update(registry_names)
        candidates.update(proc_tail_names)
        candidates.update(stub_names)
        candidates.update(dispatch_targets)
        candidates.update(alias_names)

        _class_tail_names: set[str] = set()
        for qname in self.result.all_classes:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                _class_tail_names.add(tail)
        candidates.update(_class_tail_names)

        _ensemble_cmds: set[str] = set()
        for ns in self._ensemble_namespaces:
            tail = ns.rsplit("::", 1)[-1]
            if tail:
                _ensemble_cmds.add(tail)
        candidates.update(_ensemble_cmds)

        _interp_sccp_values: dict[tuple[str, int], LatticeValue] | None = None
        _interp_sccp_uses: dict[str, int] = {}
        if cu is not None and "$" in "".join(inv.name for inv in self.result.command_invocations):
            _interp_sccp_values = {}
            for analysis in [
                cu.top_level.analysis,
                *(fu.analysis for fu in cu.procedures.values()),
            ]:
                for key, lv in analysis.values.items():
                    _interp_sccp_values[key] = lv
            for name, ver in _interp_sccp_values:
                if name not in _interp_sccp_uses or ver > _interp_sccp_uses[name]:
                    _interp_sccp_uses[name] = ver

        for inv in self.result.command_invocations:
            cmd_name = inv.name

            if cmd_name in registry_names:
                continue
            if inv.resolved_qualified_name is not None:
                continue
            if "::" in cmd_name:
                continue
            if cmd_name.startswith("$") or cmd_name.startswith("["):
                continue
            if cmd_name in stub_names:
                continue
            if cmd_name in alias_names:
                continue
            if cmd_name in dispatch_targets:
                continue
            if cmd_name in _ensemble_cmds:
                continue
            if cmd_name in proc_tail_names:
                continue
            if cmd_name in _class_tail_names:
                continue

            if "$" in cmd_name and _interp_sccp_values is not None:
                from ...compiler.core_analyses import _fold_interpolation_set

                resolved = _fold_interpolation_set(
                    cmd_name,
                    _interp_sccp_uses,
                    _interp_sccp_values,
                )
                if resolved is not None and all(
                    r in registry_names
                    or r in proc_tail_names
                    or f"::{r}" in self.result.all_procs
                    or r in _class_tail_names
                    or r in stub_names
                    or r in alias_names
                    for r in resolved
                ):
                    continue

            msg = f"Unknown command '{cmd_name}'"
            suggestions = suggest_similar(cmd_name, candidates, max_suggestions=1, max_distance=2)
            fixes: tuple[CodeFix, ...] = ()
            if suggestions:
                msg += f"; did you mean '{suggestions[0]}'?"
                fixes = (
                    CodeFix(
                        range=inv.range,
                        new_text=suggestions[0],
                        description=f"Replace with '{suggestions[0]}'",
                    ),
                )

            self.result.diagnostics.append(
                Diagnostic(
                    range=inv.range,
                    message=msg,
                    severity=Severity.HINT,
                    code="W123",
                    fixes=fixes,
                )
            )

    def _resolve_interpolated_commands(self, cu: CompilationUnit) -> None:
        """Suppress W123 for interpolated command names resolvable via CONSTSET."""
        w123_diags = [d for d in self.result.diagnostics if d.code == "W123"]
        if not w123_diags:
            return

        from ...compiler.core_analyses import _fold_interpolation_set

        _func_values: dict[str, dict[tuple[str, int], LatticeValue]] = {}
        _func_uses: dict[str, dict[str, int]] = {}
        _named_fus = [("::top", cu.top_level)] + list(cu.procedures.items())
        for qname, fu_unit in _named_fus:
            vals: dict[tuple[str, int], LatticeValue] = {}
            uses: dict[str, int] = {}
            for (var_name, ver), lv in fu_unit.analysis.values.items():
                key = (var_name, ver)
                vals[key] = lv
                if var_name not in uses or ver > uses[var_name]:
                    uses[var_name] = ver
            _func_values[qname] = vals
            _func_uses[qname] = uses

        _func_ranges: list[tuple[str, int, int]] = []
        for qname, fu_unit in _named_fus:
            ir_proc = cu.ir_module.procedures.get(qname)
            if ir_proc is not None:
                _func_ranges.append((qname, ir_proc.range.start.offset, ir_proc.range.end.offset))
            elif qname == "::top":
                _func_ranges.append(("::top", 0, 2**31))

        def _sccp_for_offset(
            offset: int,
        ) -> tuple[dict[str, int], dict[tuple[str, int], LatticeValue]]:
            for qname, start, end in _func_ranges:
                if start <= offset <= end:
                    return _func_uses.get(qname, {}), _func_values.get(qname, {})
            return _func_uses.get("::top", {}), _func_values.get("::top", {})

        _known_cmds = known_command_names()
        _known_procs = frozenset(self.result.all_procs)
        _known_proc_bare = frozenset(qn.rsplit("::", 1)[-1] for qn in _known_procs if "::" in qn)

        resolved_ranges: set[tuple[int, int]] = set()
        for w123_diag in w123_diags:
            msg = w123_diag.message
            if "'" not in msg:
                continue
            start = msg.index("'") + 1
            end = msg.index("'", start)
            cmd_name = msg[start:end]

            if "$" not in cmd_name:
                continue

            site_uses, site_values = _sccp_for_offset(w123_diag.range.start.offset)
            resolved = _fold_interpolation_set(cmd_name, site_uses, site_values)
            if resolved is None:
                continue

            if all(
                name in _known_cmds
                or name in _known_procs
                or name in _known_proc_bare
                or f"::{name}" in _known_procs
                for name in resolved
            ):
                resolved_ranges.add((w123_diag.range.start.offset, w123_diag.range.end.offset))

        if resolved_ranges:
            self.result.diagnostics = [
                d
                for d in self.result.diagnostics
                if d.code != "W123"
                or (d.range.start.offset, d.range.end.offset) not in resolved_ranges
            ]

    def _globals_written_by_procs(self, cu: CompilationUnit) -> frozenset[str]:
        """Return names of global variables that any proc in ``cu`` writes."""
        result: set[str] = set()
        for fu in cu.procedures.values():
            global_aliases: set[str] = set()
            written: set[str] = set()
            for block in fu.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, IRCall):
                        if stmt.canonical_command == "::global":
                            global_aliases.update(stmt.defs)
                            continue
                        if stmt.canonical_command in ("::variable", "::upvar"):
                            continue
                        if REGISTRY.is_destroys_variable(stmt.command):
                            continue
                        names: tuple[str, ...] = stmt.defs
                    elif isinstance(
                        stmt,
                        (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr),
                    ):
                        names = (stmt.name,)
                    else:
                        continue
                    for name in names:
                        if name.startswith("::"):
                            bare = name.lstrip(":")
                            if bare:
                                result.add(bare)
                        else:
                            written.add(name)
            result.update(global_aliases & written)
        return frozenset(result)
