from __future__ import annotations

import logging
import re

from ...commands.registry import REGISTRY
from ...commands.registry.runtime import (
    ArgRole,
    arg_indices_for_role,
)
from ...common.dialect import active_dialect
from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...compiler.cfg import CFGBranch, CFGFunction
from ...compiler.compilation_unit import CompilationUnit, FunctionUnit, ensure_compilation_unit
from ...compiler.compiler_checks import run_compiler_checks
from ...compiler.core_analyses import FunctionAnalysis, LatticeKind, LatticeValue
from ...compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
    IRProcedure,
    IRStatement,
    when_event_name,
)
from ...compiler.ssa import SSAFunction
from ...parsing.known_commands import known_command_names
from ..semantic_model import (
    CodeFix,
    Diagnostic,
    Range,
    Severity,
)
from ._utils import (
    _UNUSED_VAR_RE,
    _format_literal_for_message,
    _possible_paste_fingerprint,
)

log = logging.getLogger(__name__)


class _AnalyserDiagsMixin:
    """Diagnostic emission methods."""

    def _emit_unresolved_command_diagnostics(
        self,
        cu: CompilationUnit | None = None,
    ) -> None:
        """Emit W123 for commands that cannot be resolved.

        Runs as a post-analysis pass so that all procs (including
        forward-defined ``unknown``) are already collected.

        W123 is ``default=False`` (opt-in in editor settings).  The
        analyser always emits the diagnostic; filtering is done
        downstream by the LSP layer via ``disabled_diagnostics``.
        """
        if self._unresolved_commands_emitted:
            return
        self._unresolved_commands_emitted = True

        # W123 is opt-in (default=False).  Skip the entire pass when
        # the caller has told us it is disabled — avoids building the
        # candidate pool and running edit-distance comparisons.
        if "W123" in self._disabled_diagnostics:
            return

        from ...common.text import suggest_similar

        dialect = active_dialect()
        registry_names = frozenset(REGISTRY.command_names(dialect))
        stub_names = frozenset(s.name for s in self.result.stub_commands)

        # Build candidate pool for "did you mean?" suggestions.
        proc_tail_names: set[str] = set()
        for qname in self.result.all_procs:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                proc_tail_names.add(tail)

        upi = self.result.unknown_proc_info
        # If unknown handler is opaque, suppress all W123.
        if upi is not None and (
            upi.chains_original
            or upi.has_exec
            or upi.has_auto_load
            or upi.case_insensitive
            or upi.has_pattern_dispatch
        ):
            return

        # If dynamic providers detected (load, rename, namespace import, etc.),
        # suppress all.
        if self.result.has_dynamic_providers:
            return

        # If package require is present, external commands may be loaded.
        if self.result.package_requires:
            return

        dispatch_targets = upi.dispatch_targets if upi is not None else frozenset()

        # Build set of alias tail names (e.g. "=" from "::=").
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

        # TclOO class names are commands (oo::class create Foo creates "Foo").
        _class_tail_names: set[str] = set()
        for qname in self.result.all_classes:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                _class_tail_names.add(tail)
        candidates.update(_class_tail_names)

        # Namespace ensemble commands: ``namespace ensemble create`` inside
        # ``namespace eval foo`` creates a command named ``foo``.
        _ensemble_cmds: set[str] = set()
        for ns in self._ensemble_namespaces:
            tail = ns.rsplit("::", 1)[-1]
            if tail:
                _ensemble_cmds.add(tail)
        candidates.update(_ensemble_cmds)

        # Build SCCP value maps for interpolated command name resolution.
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
            # Build a simple uses map: variable name → latest known version.
            for name, ver in _interp_sccp_values:
                if name not in _interp_sccp_uses or ver > _interp_sccp_uses[name]:
                    _interp_sccp_uses[name] = ver

        for inv in self.result.command_invocations:
            cmd_name = inv.name

            # Skip commands already resolved to a registry entry.
            if cmd_name in registry_names:
                continue

            # Skip commands resolved to a user-defined proc.
            if inv.resolved_qualified_name is not None:
                continue

            # Skip namespace-qualified commands (could be imported).
            if "::" in cmd_name:
                continue

            # Skip variable/substitution commands (W307 covers those).
            if cmd_name.startswith("$") or cmd_name.startswith("["):
                continue

            # Skip stub commands.
            if cmd_name in stub_names:
                continue

            # Skip commands defined as aliases (interp alias).
            if cmd_name in alias_names:
                continue

            # Skip commands explicitly handled by unknown dispatch.
            if cmd_name in dispatch_targets:
                continue

            # Skip namespace ensemble commands.
            if cmd_name in _ensemble_cmds:
                continue

            # Skip if known as a proc tail name (e.g. forward-defined proc).
            if cmd_name in proc_tail_names:
                continue

            # Skip TclOO class names used as commands (e.g. "Logger new").
            # oo::class create ClassName creates a command named ClassName.
            if cmd_name in _class_tail_names:
                continue

            # Skip interpolated command names that resolve to known commands
            # via the CONSTSET lattice (e.g. cmd_$var where $var is CONSTSET).
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

            # Build suggestion.
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
        conn = cu.connection_scope
        for qname, fu in cu.procedures.items():
            cross_vars: frozenset[str] = frozenset()
            if conn is not None and qname.startswith("::when::"):
                cross_vars = conn.cross_event_defs | conn.cross_event_imports
            self._emit_cfg_ssa_diagnostics_for_function(
                fu.cfg,
                fu.analysis,
                cross_event_vars=cross_vars,
                ssa=fu.ssa,
            )
            ir_proc = ir_module.procedures.get(qname)
            if ir_proc is not None:
                self._emit_unused_param_diagnostics(ir_proc, fu.analysis)
            # IRULE4005: racy static:: cross-event flow
            if conn is not None and qname.startswith("::when::") and conn.racy_static_defs:
                event = when_event_name(qname)
                if event != "RULE_INIT":
                    self._emit_racy_static_diagnostics(fu, conn.racy_static_defs)

        # Post-pass: resolve $var-as-command sites using the type lattice.
        self._emit_var_command_diagnostics(cu)

        # Post-pass: resolve interpolated command names using CONSTSET.
        self._resolve_interpolated_commands(cu)

    def _emit_var_command_diagnostics(self, cu: CompilationUnit) -> None:
        """Resolve ``$var method`` patterns using the type lattice.

        For each recorded variable-as-command site:
        - If the variable has ``TclType.OBJECT`` with a known class, validate
          the method name against the class hierarchy.  Emit W308 if the
          method doesn't exist.
        - If the variable has a ``CONSTSET`` (or ``CONST``) SCCP value whose
          elements are all resolvable to known commands, procs, or TclOO
          objects, suppress W307 (the set of command names is statically known).
        - Otherwise emit W307 (non-literal command name).
        """
        if not self._var_command_sites and not self._cmd_command_sites:
            return
        if "W307" in self._disabled_diagnostics and "W308" in self._disabled_diagnostics:
            return

        from ...compiler.core_analyses import (
            _extract_foreach_elements,
            _parse_literal_value,
        )
        from ...compiler.core_analyses import (
            _to_set as _lattice_to_set,
        )
        from ...compiler.ir import IRCall
        from ...compiler.types import TclType, TypeKind
        from ..class_hierarchy import build_class_hierarchy

        # Collect all SSA type entries across top-level and procedures.
        all_types: dict[str, set[str]] = {}  # var_name → set of class_names
        all_typed_vars: set[str] = set()  # vars known to be OBJECT
        # Per-function CONSTSET maps: keyed by function qname, then var_name.
        _func_constsets: dict[str, dict[str, frozenset[int | float | bool | str]]] = {}
        _all_fus_named = [("::top", cu.top_level)] + list(cu.procedures.items())
        for qname, fu_unit in _all_fus_named:
            analysis = fu_unit.analysis
            for (var_name, _ver), tl in analysis.types.items():
                if tl.kind is TypeKind.KNOWN and tl.tcl_type is TclType.OBJECT and tl.class_name:
                    all_typed_vars.add(var_name)
                    all_types.setdefault(var_name, set()).add(tl.class_name)
            func_cs: dict[str, frozenset[int | float | bool | str]] = {}
            for (var_name, _ver), lv in analysis.values.items():
                vs = _lattice_to_set(lv)
                if vs is not None:
                    existing = func_cs.get(var_name)
                    func_cs[var_name] = (existing | vs) if existing else vs
            _func_constsets[qname] = func_cs

        # Build a flat all_constsets for backwards compat (foreach fallback).
        all_constsets: dict[str, frozenset[int | float | bool | str]] = {}
        for func_cs in _func_constsets.values():
            for var_name, vs in func_cs.items():
                existing = all_constsets.get(var_name)
                all_constsets[var_name] = (existing | vs) if existing else vs

        # Build function offset ranges for scoping site lookups.
        _func_ranges: list[tuple[str, int, int]] = []
        for qname, fu_unit in _all_fus_named:
            ir_proc = cu.ir_module.procedures.get(qname)
            if ir_proc is not None:
                _func_ranges.append((qname, ir_proc.range.start.offset, ir_proc.range.end.offset))
            elif qname == "::top":
                _func_ranges.append(("::top", 0, 2**31))

        def _constsets_for_offset(offset: int) -> dict[str, frozenset[int | float | bool | str]]:
            """Return the CONSTSET map for the function containing *offset*."""
            for qname, start, end in _func_ranges:
                if start <= offset <= end:
                    return _func_constsets.get(qname, {})
            return all_constsets  # fallback to merged

        # Fallback: directly extract foreach iteration elements from the CFG.
        # SCCP barriers (e.g. oo::class create) may have widened foreach
        # variables to OVERDEFINED even when the list is statically known.
        for qname, fu_unit in _all_fus_named:
            func_cs = _func_constsets.setdefault(qname, {})
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if (
                        isinstance(stmt, IRCall)
                        and stmt.command in ("foreach", "lmap")
                        and len(stmt.defs) == 1
                        and len(stmt.args) == 1
                    ):
                        var_name = stmt.defs[0]
                        if var_name not in func_cs:
                            elements = _extract_foreach_elements(stmt.args[0])
                            if elements:
                                vs = frozenset(_parse_literal_value(e) for e in elements)
                                func_cs[var_name] = vs
                                all_constsets[var_name] = vs

        # Interprocedural constant return resolution: if a variable was
        # assigned from [known_proc] and that proc always returns a constant,
        # record the constant value so W307 can check it.
        if cu.interproc is not None:
            import re as _re

            _CMD_SUB_RE = _re.compile(r"^\[(\S+?)(?:\s.*)?\]$")
            from ...compiler.ir import IRAssignValue

            for qname, fu_unit in _all_fus_named:
                func_cs = _func_constsets.setdefault(qname, {})
                for block in fu_unit.cfg.blocks.values():
                    for stmt in block.statements:
                        if not isinstance(stmt, IRAssignValue):
                            continue
                        val = stmt.value.strip()
                        m = _CMD_SUB_RE.match(val)
                        if m is None:
                            continue
                        called = m.group(1)
                        for qn, summary in cu.interproc.procedures.items():
                            bare = qn.rsplit("::", 1)[-1]
                            if bare == called or qn == called:
                                if summary.returns_constant and summary.constant_return:
                                    if stmt.name not in func_cs:
                                        vs = frozenset((summary.constant_return,))
                                        func_cs[stmt.name] = vs
                                        all_constsets[stmt.name] = vs
                                break

        # Build a set of known command names for CONSTSET resolution.
        _known_cmds = known_command_names()
        _known_procs = frozenset(self.result.all_procs)
        # Also include bare proc names (without :: prefix) for matching.
        _known_proc_bare = frozenset(qn.rsplit("::", 1)[-1] for qn in _known_procs if "::" in qn)
        # TclOO class names are valid commands (oo::class create X → command X).
        _class_tail_names: set[str] = set()
        for qname in self.result.all_classes:
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                _class_tail_names.add(tail)

        # Build class hierarchy for method resolution.
        hierarchy = (
            build_class_hierarchy(self.result.all_classes) if self.result.all_classes else None
        )

        # Collect source offset ranges of procedures that contain
        # ``dict with``/``dict update`` barriers.  Variables in these
        # scopes may have been created by dict unpacking — suppress W307.

        dict_with_ranges: list[tuple[int, int]] = []
        _all_fus = [("::top", cu.top_level)] + list(cu.procedures.items())
        for qname, fu_unit in _all_fus:
            func_has_dw = False
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if (
                        isinstance(stmt, IRBarrier)
                        and stmt.command == "dict"
                        and stmt.args
                        and stmt.args[0] in ("with", "update")
                    ):
                        func_has_dw = True
                        break
                if func_has_dw:
                    break
            if func_has_dw:
                ir_proc = cu.ir_module.procedures.get(qname)
                if ir_proc is not None:
                    dict_with_ranges.append((ir_proc.range.start.offset, ir_proc.range.end.offset))
                else:
                    # Top-level: covers entire source.
                    dict_with_ranges.append((0, 2**31))

        for var_name, method_name, site_range, in_method in self._var_command_sites:
            class_names = all_types.get(var_name)
            if class_names:
                # Variable is a TclOO object — validate the method if we have
                # a class hierarchy and a method name.
                if (
                    hierarchy is not None
                    and method_name is not None
                    and "W308" not in self._disabled_diagnostics
                ):
                    # Check if the method exists on any of the possible classes.
                    found = False
                    has_local_class = False
                    for cls in class_names:
                        # Check direct methods on the class and its MRO.
                        if hierarchy.method_target(cls, method_name) is not None:
                            found = True
                            break
                        # Also check the class definition directly (for classes
                        # not in the hierarchy, e.g. missing superclass).
                        cd = self.result.all_classes.get(cls)
                        if cd is not None:
                            has_local_class = True
                            if (
                                method_name in cd.methods
                                or method_name in cd.class_methods
                                or method_name == "new"
                                or method_name == "create"
                                or method_name == "destroy"
                                or method_name == "configure"
                                or method_name == "cget"
                            ):
                                found = True
                                break
                            # If the class defines 'unknown', any method is valid.
                            if "unknown" in cd.methods:
                                found = True
                                break
                    # Check for inherited 'unknown' handler via the MRO.
                    if not found and has_local_class:
                        for cls in class_names:
                            if hierarchy.method_target(cls, "unknown") is not None:
                                found = True
                                break
                    # If the class has an external superclass the method
                    # might be inherited — skip W308.
                    if not found and has_local_class:
                        for cls in class_names:
                            cd = self.result.all_classes.get(cls)
                            if cd is not None and cd.superclasses:
                                _OO_BASE = {"oo::object", "oo::class"}
                                if any(
                                    s not in self.result.all_classes and s not in _OO_BASE
                                    for s in cd.superclasses
                                ):
                                    found = True
                                    break
                    # Suppress W308 if the variable had oo::objdefine applied
                    # (may have per-instance methods not in the class).
                    if not found and var_name in self._objdefined_vars:
                        found = True
                    if not found and has_local_class:
                        cls_display = ", ".join(sorted(class_names))
                        self.result.diagnostics.append(
                            Diagnostic(
                                range=site_range,
                                message=f"Unknown method '{method_name}' on class '{cls_display}'",
                                severity=Severity.WARNING,
                                code="W308",
                            )
                        )
            else:
                # Variable is not a known TclOO object — check if SCCP
                # resolved it to a finite set of known command names.
                # Use per-function scoping to avoid cross-procedure conflation.
                scoped_cs = _constsets_for_offset(site_range.start.offset)
                constset_vals = scoped_cs.get(var_name)
                if constset_vals is not None and all(
                    isinstance(v, str)
                    and (
                        v in _known_cmds
                        or v in _known_procs
                        or v in _known_proc_bare
                        or f"::{v}" in _known_procs
                        or v in all_typed_vars
                        or v in _class_tail_names
                        or f"::{v}" in self.result.all_classes
                    )
                    for v in constset_vals
                ):
                    # All possible command names are statically known —
                    # suppress W307.
                    continue

                # Emit W307 unless inside a method body or a function with
                # dict-with where $var is very likely an object from dict
                # unpacking.
                in_dict_with = any(s <= site_range.start.offset <= e for s, e in dict_with_ranges)
                if not in_method and not in_dict_with and "W307" not in self._disabled_diagnostics:
                    self.result.diagnostics.append(
                        Diagnostic(
                            range=site_range,
                            message="Non-literal command name \u2014 cannot statically analyze",
                            severity=Severity.WARNING,
                            code="W307",
                        )
                    )

        # Resolve [cmd]-as-command sites: suppress W307 when the command
        # substitution returns a TclOO object (e.g. ``[Dog new] bark``).
        if self._cmd_command_sites:
            from ...compiler.core_analyses import _return_type_for_command

            _known_classes = frozenset(self.result.all_classes)
            # Build set of W307 ranges from the check pipeline that we may
            # need to remove.
            w307_indices: dict[tuple[int, int], int] = {}
            for i, d in enumerate(self.result.diagnostics):
                if d.code == "W307":
                    w307_indices[(d.range.start.offset, d.range.end.offset)] = i

            remove_indices: list[int] = []
            for cmd_text, method_name, site_range, in_method in self._cmd_command_sites:
                # Parse the command substitution: [Dog new] → ("Dog", ("new",))
                inner = cmd_text.strip()
                if inner.startswith("[") and inner.endswith("]"):
                    inner = inner[1:-1].strip()
                parts = inner.split(None, 1)
                if not parts:
                    continue
                cmd_name_ = parts[0]
                cmd_args_ = tuple(parts[1].split()) if len(parts) > 1 else ()
                ret_type = _return_type_for_command(cmd_name_, cmd_args_, _known_classes)
                # ``my`` and ``self`` are TclOO self-dispatch — the return
                # value is very likely an object when used in chained calls.
                is_oo_self_dispatch = cmd_name_ in ("my", "self")
                # Inside an OO method body, [cmd] method chaining is common
                # for accessing objects stored in instance variables.
                if (
                    in_method
                    or is_oo_self_dispatch
                    or (ret_type.kind is TypeKind.KNOWN and ret_type.tcl_type is TclType.OBJECT)
                ):
                    # Command returns an object — suppress W307.
                    key = (site_range.start.offset, site_range.end.offset)
                    idx = w307_indices.get(key)
                    if idx is not None:
                        remove_indices.append(idx)
                    # Optionally validate the method name (W308).
                    if (
                        hierarchy is not None
                        and method_name is not None
                        and ret_type.class_name
                        and "W308" not in self._disabled_diagnostics
                    ):
                        cls = ret_type.class_name
                        cd = self.result.all_classes.get(cls)
                        method_ok = (
                            hierarchy.method_target(cls, method_name) is not None
                            or cd is None  # external class — can't validate
                            or method_name in cd.methods
                            or method_name in cd.class_methods
                            or method_name in ("new", "create", "destroy", "configure", "cget")
                            or "unknown" in cd.methods
                            or hierarchy.method_target(cls, "unknown") is not None
                        )
                        # If class has external superclass, skip W308.
                        if not method_ok and cd is not None and cd.superclasses:
                            _OO_BASE = {"oo::object", "oo::class"}
                            if any(
                                s not in self.result.all_classes and s not in _OO_BASE
                                for s in cd.superclasses
                            ):
                                method_ok = True
                        if not method_ok:
                            self.result.diagnostics.append(
                                Diagnostic(
                                    range=site_range,
                                    message=f"Unknown method '{method_name}' on class '{cls}'",
                                    severity=Severity.WARNING,
                                    code="W308",
                                )
                            )

            # Remove suppressed W307 diagnostics.
            if remove_indices:
                for i in sorted(remove_indices, reverse=True):
                    del self.result.diagnostics[i]

    def _resolve_interpolated_commands(self, cu: CompilationUnit) -> None:
        """Suppress W123 for interpolated command names resolvable via CONSTSET.

        When a command name like ``cmd_${var}`` contains variable references
        and those variables have CONSTSET (or CONST) values in the SCCP
        lattice, we compute all possible interpolated strings.  If every
        resolved name is a known command or proc, the W123 diagnostic is
        removed.

        SCCP values are resolved per-function to avoid cross-procedure
        variable name conflation.
        """
        # Quick exit if no W123 diagnostics to resolve.
        w123_diags = [d for d in self.result.diagnostics if d.code == "W123"]
        if not w123_diags:
            return

        from ...compiler.core_analyses import _fold_interpolation_set

        # Build per-function SCCP value maps.
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

        # Build function offset ranges for per-function lookup.
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
            # Fallback: top-level
            return _func_uses.get("::top", {}), _func_values.get("::top", {})

        # Build known command/proc sets.
        _known_cmds = known_command_names()
        _known_procs = frozenset(self.result.all_procs)
        _known_proc_bare = frozenset(qn.rsplit("::", 1)[-1] for qn in _known_procs if "::" in qn)

        resolved_ranges: set[tuple[int, int]] = set()
        for w123_diag in w123_diags:
            # Extract the command name from the message.
            # Format: "Unknown command 'cmd_${var}'"
            msg = w123_diag.message
            if "'" not in msg:
                continue
            start = msg.index("'") + 1
            end = msg.index("'", start)
            cmd_name = msg[start:end]

            # Only process interpolated names (contain $ references).
            if "$" not in cmd_name:
                continue

            site_uses, site_values = _sccp_for_offset(w123_diag.range.start.offset)
            resolved = _fold_interpolation_set(cmd_name, site_uses, site_values)
            if resolved is None:
                continue

            # Check if all resolved names are known.
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
        """Return names of global variables that any proc in ``cu`` writes.

        A proc writes to a global when it either declares ``global X`` and
        then sets ``X`` (via ``set``/``incr``/``append``/``lappend``/...),
        or writes through a fully-qualified name (``set ::X ...``).  These
        globals may be populated at runtime by calls to those procs —
        including indirect calls via ``source`` — so top-level reads of
        such variables should not trigger W210.  Names are returned without
        the leading ``::`` since the read-before-set analysis skips
        qualified reads and reports only bare names.
        """
        result: set[str] = set()
        for fu in cu.procedures.values():
            global_aliases: set[str] = set()
            written: set[str] = set()
            for block in fu.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, IRCall):
                        if stmt.command == "global":
                            global_aliases.update(stmt.defs)
                            continue
                        if stmt.command in ("variable", "upvar"):
                            continue
                        if REGISTRY.is_destroys_variable(stmt.command):
                            # ``unset`` and similar destroy the variable;
                            # they don't populate it.
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
            if isinstance(stmt, IRCall) and stmt.command == "unset":
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
        for param_name in analysis.unused_params:
            self.result.diagnostics.append(
                Diagnostic(
                    range=ir_proc.range,
                    message=(f"Parameter '{param_name}' of proc '{ir_proc.name}' is unused"),
                    severity=Severity.HINT,
                    code="W214",
                )
            )

    # Standard Tcl channel names that are always valid.
    _STANDARD_CHANNELS = frozenset({"stdout", "stderr", "stdin"})

    def _emit_channel_diagnostics(
        self,
        ssa: "SSAFunction",
        analysis: FunctionAnalysis,
    ) -> None:
        """W126: flag non-channel values passed to channel argument positions.

        Walks SSA statements for commands that declare ``ArgRole.CHANNEL``
        arguments.  For each channel arg, checks the SSA value's type:
        if it's a known constant that isn't a standard channel name and
        the variable type is not ``TclType.CHANNEL``, emit a warning.
        """
        from ...compiler.ir import IRCall
        from ...compiler.types import TclType, TypeKind

        for block in ssa.blocks.values():
            for ssa_stmt in block.statements:
                ir_stmt = ssa_stmt.statement
                if not isinstance(ir_stmt, IRCall):
                    continue
                cmd = ir_stmt.command
                args = list(ir_stmt.args)
                channel_indices = arg_indices_for_role(cmd, args, ArgRole.CHANNEL)
                if not channel_indices:
                    continue

                for idx in channel_indices:
                    if idx >= len(args):
                        continue
                    arg_text = args[idx]

                    # Extract variable name from ${var} or $var form
                    var_name: str | None = None
                    if arg_text.startswith("${") and arg_text.endswith("}"):
                        var_name = arg_text[2:-1]
                    elif arg_text.startswith("$"):
                        var_name = arg_text[1:]

                    if var_name and var_name in ssa_stmt.uses:
                        # Variable reference — check its type in the SSA
                        version = ssa_stmt.uses[var_name]
                        key = (var_name, version)
                        var_type = analysis.types.get(key)
                        if var_type is not None and var_type.kind == TypeKind.KNOWN:
                            if var_type.tcl_type == TclType.CHANNEL:
                                continue  # Confirmed channel — ok
                            # Known non-channel type → warn
                            stmt_range = getattr(ir_stmt, "range", None)
                            if stmt_range is not None:
                                self.result.diagnostics.append(
                                    Diagnostic(
                                        range=stmt_range,
                                        message=(
                                            "Variable '$"
                                            + var_name
                                            + f"' passed as channel to '{cmd}'"
                                            f" has type {var_type.tcl_type.name if var_type.tcl_type else 'UNKNOWN'},"
                                            " not CHANNEL."
                                        ),
                                        severity=Severity.WARNING,
                                        code="W126",
                                    )
                                )
                        # UNKNOWN or OVERDEFINED — could be anything, don't warn
                    elif not var_name:
                        # Literal string — check if it's a standard channel
                        literal = arg_text.strip('"').strip("{").strip("}")
                        if literal in self._STANDARD_CHANNELS:
                            continue  # stdout/stderr/stdin — ok
                        # Non-standard literal in channel position
                        # Only warn if it's clearly not a variable ref
                        if "$" not in arg_text and "[" not in arg_text:
                            stmt_range = getattr(ir_stmt, "range", None)
                            if stmt_range is not None:
                                self.result.diagnostics.append(
                                    Diagnostic(
                                        range=stmt_range,
                                        message=(
                                            f"String literal '{literal}' used as channel "
                                            f"argument to '{cmd}' — expected a channel "
                                            f"from open/socket/chan create."
                                        ),
                                        severity=Severity.WARNING,
                                        code="W126",
                                    )
                                )

    def _emit_invalid_ip_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        """W124: flag invalid IP address literals discovered via SCCP constants.

        Walks all SSA constants looking for IPv4/IPv6 candidates via regex,
        validates them with ``ip_utils.parse_ip()``, and emits diagnostics at
        the definition site.  Use sites get ``related_ranges`` links.
        """
        from ...common.ip_utils import IPV6_RE, parse_ip

        if analysis.def_use_chains is None:
            return

        _DOTTED_QUAD_LOOSE = re.compile(r"\b(\d{1,4})\.(\d{1,4})\.(\d{1,4})\.(\d{1,4})\b")
        # Track emitted definition offsets to avoid duplicates when
        # multiple SSA values point at the same assignment.
        seen_offsets: set[int] = set()

        for key, lattice_val in analysis.values.items():
            if lattice_val.kind is not LatticeKind.CONST:
                continue
            val = lattice_val.value
            if not isinstance(val, str):
                continue

            # --- IPv4 candidates ---
            for m in _DOTTED_QUAD_LOOSE.finditer(val):
                # Skip version-number patterns: preceded by '/'
                if m.start() > 0 and val[m.start() - 1] == "/":
                    continue
                octets_str = [m.group(i) for i in range(1, 5)]
                msg: str | None = None
                severity = Severity.ERROR
                for i, octet_s in enumerate(octets_str):
                    v = int(octet_s)
                    if v > 255:
                        msg = (
                            f"IPv4 octet {i + 1} ({octet_s}) exceeds 255 "
                            "— this is not a valid IP address."
                        )
                        break
                    if (
                        len(octet_s) > 1
                        and octet_s[0] == "0"
                        and all(c in "01234567" for c in octet_s)
                    ):
                        msg = (
                            f"IPv4 octet {i + 1} ({octet_s}) has a leading zero "
                            "— may be interpreted as octal in some contexts."
                        )
                        severity = Severity.WARNING
                        break
                if msg is not None:
                    self._emit_ip_diag(cfg, analysis, key, msg, severity, seen_offsets)
                    break  # one diagnostic per SSA value

            # --- IPv6 candidates ---
            for m in IPV6_RE.finditer(val):
                candidate = m.group(1)
                if parse_ip(candidate) is None:
                    self._emit_ip_diag(
                        cfg,
                        analysis,
                        key,
                        f"Invalid IPv6 address '{candidate}'.",
                        Severity.ERROR,
                        seen_offsets,
                    )
                    break  # one diagnostic per SSA value

    def _emit_ip_diag(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        key: tuple[str, int],
        message: str,
        severity: Severity,
        seen_offsets: set[int],
    ) -> None:
        """Emit a W124 diagnostic at the definition site with related-info on uses."""
        assert analysis.def_use_chains is not None
        var_name, version = key
        chain = analysis.def_use_chains.chain_for(var_name, version)
        if chain is None:
            return

        # Find definition range
        def_site = chain.definition
        block = cfg.blocks.get(def_site.block)
        if block is None:
            return
        if def_site.statement_index < 0 or def_site.statement_index >= len(block.statements):
            return
        stmt = block.statements[def_site.statement_index]
        def_range = getattr(stmt, "range", None)
        if def_range is None:
            return

        # Skip if we already emitted a W124 for this exact source location
        if def_range.start.offset in seen_offsets:
            return
        seen_offsets.add(def_range.start.offset)

        # Collect use-site ranges for related information
        related: list[tuple[Range, str]] = []
        for use in chain.uses:
            use_block = cfg.blocks.get(use.block)
            if use_block is None:
                continue
            if 0 <= use.statement_index < len(use_block.statements):
                use_stmt = use_block.statements[use.statement_index]
                use_range = getattr(use_stmt, "range", None)
                if use_range is not None:
                    related.append((use_range, f"'{var_name}' used here"))

        self.result.diagnostics.append(
            Diagnostic(
                range=def_range,
                message=message,
                severity=severity,
                code="W124",
                related_ranges=tuple(related),
            )
        )

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
