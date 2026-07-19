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

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from analyser.compiler_checks import run_compiler_checks
from compiler.cfg import CFGFunction
from compiler.compilation_unit import CompilationUnit, FunctionUnit, ensure_compilation_unit
from compiler.core_analyses import FunctionAnalysis
from compiler.ir import when_event_name
from compiler.registry.runtime import command_enabled_in_active_dialect
from compiler.ssa import SSAFunction

log = logging.getLogger(__name__)


class _AnalyserDiagsMixin(_Base):
    """Diagnostic emission orchestrator — delegates to focused sub-mixins."""

    if TYPE_CHECKING:

        def _emit_unresolved_command_diagnostics(
            self, cu: CompilationUnit | None = None
        ) -> None: ...

        def _globals_written_by_procs(self, cu: CompilationUnit) -> frozenset[str]: ...

        def _resolve_interpolated_commands(self, cu: CompilationUnit) -> None: ...

        def _emit_var_command_diagnostics(self, cu: CompilationUnit) -> None: ...
        def _record_method_invocations(self, cu: CompilationUnit) -> None: ...

        def _emit_renamed_command_diagnostics(self, cu: CompilationUnit) -> None: ...

        def _emit_constant_branch_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
        ) -> None: ...

        def _emit_dead_store_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
            *,
            cross_event_vars: frozenset[str] = frozenset(),
            defined_vars: set[str] | None = None,
        ) -> None: ...

        def _emit_possible_paste_error_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
        ) -> None: ...

        def _emit_read_before_set_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
            *,
            cross_event_vars: frozenset[str] = frozenset(),
            defined_vars: set[str] | None = None,
        ) -> None: ...

        def _emit_unused_variable_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
            *,
            cross_event_vars: frozenset[str] = frozenset(),
            defined_vars: set[str] | None = None,
        ) -> None: ...

        def _emit_unused_param_diagnostics(
            self,
            ir_proc: Any,
            analysis: FunctionAnalysis,
        ) -> None: ...

        def _collect_defined_vars(self, cfg: CFGFunction) -> set[str]: ...

        def _emit_invalid_ip_diagnostics(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
        ) -> None: ...

        def _emit_ip_diag(
            self,
            cfg: CFGFunction,
            analysis: FunctionAnalysis,
            key: tuple[str, int],
            message: str,
            severity: Any,
            seen_offsets: set[int],
        ) -> None: ...

        def _emit_channel_diagnostics(
            self,
            ssa: SSAFunction,
            analysis: FunctionAnalysis,
        ) -> None: ...

        def _emit_interval_bounds_diagnostics(
            self,
            cfg: CFGFunction,
            ssa: SSAFunction,
            analysis: FunctionAnalysis,
            intent: Any,
        ) -> None: ...

        def _emit_racy_static_diagnostics(
            self,
            fu: FunctionUnit,
            racy_vars: frozenset[str],
        ) -> None: ...

        def _resolve_pending_trace_callbacks(self) -> None: ...

    def _emit_variable_usage_diagnostics(self) -> None:
        """Kept for potential future scope-tree consumers.

        W211 is now emitted by the SSA-based analysis in
        _emit_cfg_ssa_diagnostics_for_function.
        """

    def _emit_cfg_ssa_diagnostics(
        self,
        source: str,
        *,
        cu: CompilationUnit | None = None,
    ) -> None:
        """Emit diagnostics backed by CFG/SSA core analyses."""
        # Resolve any pending ``trace`` callback registrations now that
        # all procs have been parsed.  Marks ``ProcDef.is_trace_callback``
        # so W214 is suppressed for those procs.
        self._resolve_pending_trace_callbacks()
        cu = ensure_compilation_unit(
            source,
            cu,
            logger=log,
            context="analyser",
            known_classes=frozenset(self.result.all_classes),
        )
        if cu is None:
            return

        ir_module = cu.ir_module
        self.result.diagnostics.extend(
            run_compiler_checks(source, ir_module=ir_module),
        )
        # In pkgIndex.tcl files, $dir is always set by the package loader
        # before the script body is evaluated — suppress W210/W211/W220 for it.
        implicit_vars: frozenset[str] = frozenset()
        if self._file_path and self._file_path.endswith("pkgIndex.tcl"):
            implicit_vars = frozenset({"dir"})
        # Globals that any proc in this module writes may be populated
        # by a proc call (directly or via ``source``) before the
        # top-level reads the variable — suppress W210 (read-before-set)
        # only.  Unused/dead-store diagnostics still apply because a
        # top-level write is not shadowed by a proc's write unless the
        # proc actually runs, and we don't reason about call order here.
        self._emit_cfg_ssa_diagnostics_for_function(
            cu.top_level.cfg,
            cu.top_level.analysis,
            cross_event_vars=implicit_vars,
            extra_known_defined_vars=self._globals_written_by_procs(cu),
            ssa=cu.top_level.ssa,
        )
        self._emit_interval_bounds_diagnostics(
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis,
            cu.top_level.execution_intent,
        )
        # W121: a call to a command renamed/deleted away earlier in the file.
        self._emit_renamed_command_diagnostics(cu)
        conn = cu.connection_scope
        # Synthesised ``when EVENT { body }`` handlers are lifted into
        # ``::when::*`` procedures.  ``when`` is an iRules-only builtin, so
        # under a non-iRules dialect it is an unknown would-be user command
        # whose braced argument is opaque data, not a handler script — the body
        # is still lowered (the diagram extractor relies on it) but must not be
        # analysed for diagnostics (no spurious W210 read-before-set, etc.).
        # Only the *synthesised* handlers are skipped: a real user proc that
        # merely lives in the ``::when`` namespace (``proc ::when::helper {} …``)
        # carries a ``body_source`` (the literal proc body), whereas synthetic
        # ``when`` handlers leave it ``None`` — so it distinguishes the two.
        when_enabled = command_enabled_in_active_dialect("when")
        for qname, fu in cu.procedures.items():
            if fu.complexity_guarded:
                continue  # deep analysis skipped for pathologically large bodies
            ir_proc = ir_module.procedures.get(qname)
            if (
                not when_enabled
                and qname.startswith("::when::")
                and ir_proc is not None
                and ir_proc.body_source is None
            ):
                continue
            cross_vars: frozenset[str] = frozenset()
            if conn is not None and qname.startswith("::when::"):
                cross_vars = conn.cross_event_defs | conn.cross_event_imports
            self._emit_cfg_ssa_diagnostics_for_function(
                fu.cfg,
                fu.analysis,
                cross_event_vars=cross_vars,
                ssa=fu.ssa,
            )
            self._emit_interval_bounds_diagnostics(fu.cfg, fu.ssa, fu.analysis, fu.execution_intent)
            if ir_proc is not None:
                self._emit_unused_param_diagnostics(ir_proc, fu.analysis)
            # IRULE4005: racy static:: cross-event flow
            if conn is not None and qname.startswith("::when::") and conn.racy_static_defs:
                event = when_event_name(qname)
                if event != "RULE_INIT":
                    self._emit_racy_static_diagnostics(fu, conn.racy_static_defs)

        # Post-pass: resolve $var-as-command sites using the type lattice.
        self._emit_var_command_diagnostics(cu)

        # Post-pass: resolve TclOO method-dispatch sites into references using
        # the same OBJECT type lattice.  Runs unconditionally — find-references
        # must not depend on the W307/W308 diagnostics being enabled.
        self._record_method_invocations(cu)

        # Post-pass: resolve interpolated command names using CONSTSET.
        self._resolve_interpolated_commands(cu)

    def _emit_cfg_ssa_diagnostics_for_function(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        *,
        cross_event_vars: frozenset[str] = frozenset(),
        extra_known_defined_vars: frozenset[str] = frozenset(),
        ssa: SSAFunction | None = None,
    ) -> None:
        defined_vars = self._collect_defined_vars(cfg)
        self._emit_constant_branch_diagnostics(cfg, analysis)
        self._emit_dead_store_diagnostics(
            cfg, analysis, cross_event_vars=cross_event_vars, defined_vars=defined_vars
        )
        self._emit_possible_paste_error_diagnostics(cfg, analysis)
        self._emit_read_before_set_diagnostics(
            cfg,
            analysis,
            cross_event_vars=cross_event_vars | extra_known_defined_vars,
            defined_vars=defined_vars,
        )
        self._emit_unused_variable_diagnostics(
            cfg, analysis, cross_event_vars=cross_event_vars, defined_vars=defined_vars
        )
        self._emit_invalid_ip_diagnostics(cfg, analysis)
        if ssa is not None:
            self._emit_channel_diagnostics(ssa, analysis)
