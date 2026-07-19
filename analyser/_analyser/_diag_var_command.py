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

# canonicalisation: audited #246
from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.compilation_unit import CompilationUnit
from compiler.core_analyses import LatticeKind
from compiler.ir import (
    IRAssignValue,
    IRBarrier,
    IRCall,
)
from compiler.parsing.known_commands import known_command_names
from compiler.value_shapes import parse_command_substitution

from ..semantic_model import Diagnostic, MethodInvocation, Severity

# snit reserved object/type self-references — ``$self``/``$type``/``$selfns``/
# ``$win``/``$hull`` used as a command word are object dispatch, but *only*
# inside a snit type body (``$hull configure`` is the widgetadaptor delegation
# idiom).  Outside a snit body these are ordinary variable names, so the
# exemption must be scoped (see the membership check at the use site) — a
# vanilla ``proc f {} { set self …; $self foo }`` must still get W307.
_OO_SELF_REFS = frozenset({"self", "type", "selfns", "win", "hull"})


def _last_return_var(cfg) -> str | None:
    """Return the name of the variable returned by the proc, or None.

    Walks the CFG looking for the LAST IRReturn whose value is a single
    ``$var`` or ``${var}`` substitution.  Used by the object-returning
    -proc inference in the W307 suppression pass: a proc returning
    ``$X`` where ``X`` was assigned from a factory is itself an object
    factory.
    """
    from compiler.cfg import CFGReturn
    from compiler.ir import IRReturn

    def _extract(v: str | None) -> str | None:
        if not v:
            return None
        v = v.strip()
        if not v.startswith("$"):
            return None
        rest = v[1:]
        # Braced form ``${name}``.
        if rest.startswith("{") and rest.endswith("}") and rest.count("{") == 1:
            inner = rest[1:-1]
            if inner and all(c.isalnum() or c in "_:" for c in inner):
                return inner
            return None
        # Bare ``$name``.
        if rest and all(c.isalnum() or c in "_:" for c in rest):
            return rest
        return None

    last: str | None = None
    for block in cfg.blocks.values():
        for stmt in block.statements:
            if isinstance(stmt, IRReturn):
                name = _extract(stmt.value)
                if name is not None:
                    last = name
        term = block.terminator
        if isinstance(term, CFGReturn):
            v = term.value if isinstance(term.value, str) else None
            name = _extract(v)
            if name is not None:
                last = name
    return last


def _scan_tcl_command_name(source: str, start: int) -> str:
    """Return the longest Tcl command-name token at *source[start:]*.

    Walks contiguous Tcl identifier characters (alphanumeric + ``_`` +
    ``:``) -- the canonical class used by
    ``compiler.parsing.lexer._is_valid_tcl_name`` and the Tcl expr
    lexer.  Returns the empty string when the start position is past
    end-of-source or doesn't begin with an identifier character.
    """
    i = start
    n = len(source)
    while i < n and (source[i].isalnum() or source[i] in "_:"):
        i += 1
    return source[start:i]


# snit reserved object/type self-references — ``$self``/``$type``/``$selfns``/
# ``$win``/``$hull`` used as a command word are object dispatch, but *only*
# inside a snit type body (``$hull configure`` is the widgetadaptor delegation
# idiom).  Outside a snit body these are ordinary variable names, so the
# exemption must be scoped (see the membership check at the use site) — a
# vanilla ``proc f {} { set self …; $self foo }`` must still get W307.
_OO_SELF_REFS = frozenset({"self", "type", "selfns", "win", "hull"})


def _last_return_var(cfg) -> str | None:
    """Return the name of the variable returned by the proc, or None.

    Walks the CFG looking for the LAST IRReturn whose value is a single
    ``$var`` or ``${var}`` substitution.  Used by the object-returning
    -proc inference in the W307 suppression pass: a proc returning
    ``$X`` where ``X`` was assigned from a factory is itself an object
    factory.
    """
    from compiler.cfg import CFGReturn
    from compiler.ir import IRReturn

    def _extract(v: str | None) -> str | None:
        if not v:
            return None
        v = v.strip()
        if not v.startswith("$"):
            return None
        rest = v[1:]
        # Braced form ``${name}``.
        if rest.startswith("{") and rest.endswith("}") and rest.count("{") == 1:
            inner = rest[1:-1]
            if inner and all(c.isalnum() or c in "_:" for c in inner):
                return inner
            return None
        # Bare ``$name``.
        if rest and all(c.isalnum() or c in "_:" for c in rest):
            return rest
        return None

    last: str | None = None
    for block in cfg.blocks.values():
        for stmt in block.statements:
            if isinstance(stmt, IRReturn):
                name = _extract(stmt.value)
                if name is not None:
                    last = name
        term = block.terminator
        if isinstance(term, CFGReturn):
            v = term.value if isinstance(term.value, str) else None
            name = _extract(v)
            if name is not None:
                last = name
    return last


def _object_var_type_map(cu: CompilationUnit) -> dict[str, set[str]]:
    """Map each variable to the TclOO class(es) the SSA lattice typed it as.

    Reads ``TclType.OBJECT`` entries carrying a known ``class_name`` from every
    function's type analysis (top level and each proc/method).  This is the
    single ``$obj`` → class resolution shared by the W308 unknown-method check
    and method-reference recording, so both agree on which variables hold
    objects — and, crucially, non-objects such as channels never appear here.
    """
    from compiler.types import TclType, TypeKind

    all_types: dict[str, set[str]] = {}
    for _qname, fu_unit in [("::top", cu.top_level), *cu.procedures.items()]:
        for (var_name, _ver), tl in fu_unit.analysis.types.items():
            if tl.kind is TypeKind.KNOWN and tl.tcl_type is TclType.OBJECT and tl.class_name:
                all_types.setdefault(var_name, set()).add(tl.class_name)
    return all_types


class _AnalyserDiagVarCommandMixin(_Base):
    """W307/W308 diagnostics: variable-as-command patterns."""

    if TYPE_CHECKING:
        # Provided by _AnalyserCommandsMixin when composed into Analyser.
        def _class_from_construction(self, cmd_text: str) -> str | None: ...

    def _method_provider(self, class_qname: str, method_name: str, hierarchy) -> str | None:
        """Return the class that provides *method_name* for a *class_qname* receiver.

        Resolves through the class hierarchy (so an inherited method is credited
        to the ancestor that defines it), falling back to a direct definition on
        the class.  Returns ``None`` when no user method matches — a built-in
        such as ``new`` / ``destroy`` is not a reference to anything.
        """
        if hierarchy is not None:
            target = hierarchy.method_target(class_qname, method_name)
            if target is not None:
                return target
        class_def = self.result.all_classes.get(class_qname)
        if class_def is not None and (
            method_name in class_def.methods or method_name in class_def.class_methods
        ):
            return class_qname
        return None

    def _record_method_invocations(self, cu: CompilationUnit) -> None:
        """Resolve captured method-dispatch sites into references (#956, #957).

        Types a ``$obj`` receiver from the SSA OBJECT lattice (shared with the
        W308 check), so only genuine objects resolve — a ``$chan gets`` channel
        call never does.  ``my`` self-dispatch uses its enclosing class and
        ``[Class new] method`` parses the construction.  Each resolved site
        records a :class:`MethodInvocation` naming the class that actually
        provides the method, which find-references and the reference code-lens
        consume.  Runs unconditionally — references must not depend on whether
        W307/W308 are enabled.
        """
        if not self._method_dispatch_sites:
            return
        from ..class_hierarchy import build_class_hierarchy

        all_types = _object_var_type_map(cu)
        hierarchy = (
            build_class_hierarchy(self.result.all_classes) if self.result.all_classes else None
        )
        for receiver, key, method_name, method_range, enclosing in self._method_dispatch_sites:
            if receiver == "my":
                candidates: set[str] = {enclosing} if enclosing else set()
            elif receiver == "obj":
                candidates = all_types.get(key, set()) if key is not None else set()
            elif receiver == "cmd":
                built = self._class_from_construction(key) if key is not None else None
                candidates = {built} if built else set()
            else:
                candidates = set()
            for class_qname in candidates:
                provider = self._method_provider(class_qname, method_name, hierarchy)
                if provider is not None:
                    self.result.method_invocations.append(
                        MethodInvocation(
                            method_name=method_name,
                            range=method_range,
                            receiver=receiver,
                            class_name=provider,
                        )
                    )
                    break

    def _emit_var_command_diagnostics(self, cu: CompilationUnit) -> None:
        """Resolve ``$var method`` patterns using the type lattice.

        For each recorded variable-as-command site:
        - If the variable has ``TclType.OBJECT`` with a known class, validate
          the method name against the class hierarchy.  Emit W308 if the
          method doesn't exist.
        - If the variable has a ``CONSTSET`` (or ``CONST``) SCCP value whose
          elements are all resolvable to known commands, procs, or TclOO
          objects, suppress W307 (the set of command names is statically known).
          When all resolved names are also disabled in the active dialect,
          emit W002 against the resolved literal name(s).
        - Otherwise emit W307 (non-literal command name).
        """
        if not self._var_command_sites and not self._cmd_command_sites:
            return
        if (
            "W307" in self._disabled_diagnostics
            and "W308" in self._disabled_diagnostics
            and "W002" in self._disabled_diagnostics
        ):
            return

        from analyser.compiler_checks import _collect_unconditional_top_level_procs
        from compiler.core_analyses import (
            _extract_foreach_elements,
            _parse_literal_value,
        )
        from compiler.core_analyses import (
            _to_set as _lattice_to_set,
        )
        from compiler.registry import REGISTRY
        from compiler.registry.dialect import active_dialect
        from compiler.registry.models import DialectStatus
        from compiler.types import TclType, TypeKind
        from shared.naming import normalise_qualified_name

        from ..class_hierarchy import build_class_hierarchy

        _w002_enabled = "W002" not in self._disabled_diagnostics
        _dialect = active_dialect() if _w002_enabled else None
        _user_procs = _collect_unconditional_top_level_procs(cu.ir_module) if _w002_enabled else {}

        def _maybe_emit_w002(values: frozenset[object], site_range) -> None:
            """Emit W002 when every resolved literal command name is disabled.

            Only runs when ``_w002_enabled``. Each value must be a string;
            we accept the resolution only if **all** possible names are
            DISALLOWED in the active dialect and none is shadowed by a
            user proc whose unconditional definition precedes the call.
            Conservative on purpose: a CONSTSET that mixes allowed and
            disallowed names cannot fire W002 without false positives.
            """
            if not _w002_enabled or not values:
                return
            disabled_names: list[str] = []
            for v in values:
                if not isinstance(v, str) or not v:
                    return
                qualified = normalise_qualified_name(v)
                offset = _user_procs.get(qualified)
                if offset is not None and offset < site_range.start.offset:
                    return
                lookup = qualified.lstrip(":")
                if REGISTRY.command_status(lookup, _dialect) is not DialectStatus.DISALLOWED:
                    return
                disabled_names.append(v)
            if not disabled_names:
                return
            unique_names = sorted(set(disabled_names))
            if len(disabled_names) == 1:
                msg = f"'{disabled_names[0]}' is disabled in the active dialect profile"
            else:
                quoted = ", ".join(f"'{n}'" for n in unique_names)
                msg = f"command may resolve to {quoted}, all disabled in the active dialect profile"
            # The iRules-suggestion suffixes (``Select iRules`` /
            # ``available in the iRules dialect``) used to be appended
            # here; they only fired outside iRules mode and so were
            # guaranteed-noise for non-iRules users.  Per #407 feedback:
            # iRules-specific messaging is irrelevant outside iRules.
            self.result.diagnostics.append(
                Diagnostic(
                    range=site_range,
                    message=msg,
                    severity=Severity.WARNING,
                    code="W002",
                )
            )

        # SSA OBJECT typing (shared with method-reference recording).
        all_types = _object_var_type_map(cu)  # var_name → set of class_names
        all_typed_vars: set[str] = set(all_types)  # vars known to be OBJECT
        # Per-function CONSTSET maps: keyed by function qname, then var_name.
        _func_constsets: dict[str, dict[str, frozenset[int | float | bool | str]]] = {}
        _all_fus_named = [("::top", cu.top_level)] + list(cu.procedures.items())
        for qname, fu_unit in _all_fus_named:
            analysis = fu_unit.analysis
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
            """Return the CONSTSET map for the function containing *offset*.

            ``_func_ranges`` includes ``::top`` (covering the whole source)
            and one entry per proc.  When an offset is inside a proc, BOTH
            ranges contain it -- but the proc's locals must win.  Pick the
            NARROWEST containing range so a proc's CONSTSETs aren't shadowed
            by the top-level fallback (which is empty for variables defined
            only inside a proc).
            """
            best: tuple[int, str] | None = None
            for qname, start, end in _func_ranges:
                if start <= offset <= end:
                    width = end - start
                    if best is None or width < best[0]:
                        best = (width, qname)
            if best is not None:
                return _func_constsets.get(best[1], {})
            return all_constsets  # fallback to merged

        # Map qname -> FunctionUnit for the per-SSA-version precise lookup.
        _fu_by_qname = dict(_all_fus_named)

        def _fu_qname_for_offset(offset: int) -> str | None:
            """Return the qname of the NARROWEST function range containing
            *offset* (mirrors ``_constsets_for_offset``'s scoping)."""
            best: tuple[int, str] | None = None
            for qname, start, end in _func_ranges:
                if start <= offset <= end:
                    width = end - start
                    if best is None or width < best[0]:
                        best = (width, qname)
            return best[1] if best is not None else None

        def _precise_cmd_values(
            offset: int, var_name: str
        ) -> frozenset[int | float | bool | str] | None:
            """The SCCP value set of *var_name* at the SSA use-version that
            reaches the dispatch statement at *offset*.

            This is the per-SSA-version refinement (post-stage2 §A): the
            merged ``_func_constsets`` map unions every version of a
            variable, so ``set c notacommand; set c parse; $c x`` wrongly
            keeps ``notacommand`` in the set even though only the ``parse``
            version reaches the dispatch.  Reading the value at the use
            site's exact version removes that false positive.

            Purely additive: returns a set only when an SSA statement that
            contains *offset* AND uses *var_name* is found and its version
            has a concrete CONST/CONSTSET value; otherwise ``None`` and the
            caller falls back to the merged-set logic.  Never broadens a
            fire into a suppression unsoundly — the value is the exact one
            flowing into the dispatch.
            """
            qname = _fu_qname_for_offset(offset)
            if qname is None:
                return None
            fu_unit = _fu_by_qname.get(qname)
            if fu_unit is None:
                return None
            # Find the narrowest CFG statement containing *offset* that uses
            # *var_name*, and read its SSA use-version (CFG/SSA blocks are
            # parallel-indexed, as the optimiser relies on).
            best_width: int | None = None
            best_version: int | None = None
            for block_name, block in fu_unit.cfg.blocks.items():
                ssa_block = fu_unit.ssa.blocks.get(block_name)
                if ssa_block is None:
                    continue
                for idx, stmt in enumerate(block.statements):
                    rng = getattr(stmt, "range", None)
                    if rng is None or not (rng.start.offset <= offset <= rng.end.offset):
                        continue
                    if idx >= len(ssa_block.statements):
                        continue
                    version = ssa_block.statements[idx].uses.get(var_name)
                    if version is None:
                        continue
                    width = rng.end.offset - rng.start.offset
                    if best_width is None or width < best_width:
                        best_width = width
                        best_version = version
            if best_version is None:
                return None
            lv = fu_unit.analysis.values.get((var_name, best_version))
            if lv is None:
                return None
            return _lattice_to_set(lv)

        # D3-P7 closure: harvest literal element values from
        # ``array set arr {k1 v1 k2 v2 ...}`` so the W307 callback-
        # array suppression can check the ACTUAL value of
        # ``$arr(-command)`` against the known-command set.  Without
        # this, the heuristic suppression on dash-prefixed / callback-
        # suffixed array keys fires even when SCCP-equivalent literal
        # evidence proves the value isn't a command.
        # D3-P8 closure helper -- also harvest dict-unpacked variable
        # values: when ``dict with d {body}`` runs with ``d`` known to
        # be a literal dict (via SCCP CONST -- usually from D3-P2
        # call-site propagation), the inner ``body`` sees each dict
        # key as a local variable bound to its value.  Register those
        # bindings in the CONSTSET map so the W307 check can fire on
        # ``$cmd hi`` when the dict's ``cmd`` -> ``notACommand``.
        for qname, fu_unit in _all_fus_named:
            func_cs = _func_constsets.setdefault(qname, {})
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if (
                        isinstance(stmt, IRBarrier)
                        and stmt.canonical_command == "::dict"
                        and stmt.args
                        and stmt.args[0] == "with"
                        and len(stmt.args) >= 2
                    ):
                        dict_var = stmt.args[1]
                        # Strip ``$`` / ``${...}`` if present (the IR
                        # arg form may include the substitution syntax).
                        if dict_var.startswith("${") and dict_var.endswith("}"):
                            dict_var = dict_var[2:-1]
                        elif dict_var.startswith("$"):
                            dict_var = dict_var[1:]
                        # Look up dict_var's SCCP value at v0 (param
                        # entry) -- D3-P2 propagation puts the literal
                        # dict here.
                        sccp_val = fu_unit.analysis.values.get((dict_var, 0))
                        if sccp_val is None or sccp_val.kind is not LatticeKind.CONST:
                            continue
                        dict_text = sccp_val.value
                        if not isinstance(dict_text, str):
                            continue
                        try:
                            from compiler.tcl_expr_eval import _split_tcl_list

                            items = _split_tcl_list(dict_text)
                        except Exception:
                            continue
                        if len(items) % 2 != 0:
                            continue
                        for i in range(0, len(items), 2):
                            key = items[i]
                            value = items[i + 1]
                            if key not in func_cs:
                                func_cs[key] = frozenset((value,))
                                all_constsets.setdefault(key, frozenset((value,)))

            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if not (
                        isinstance(stmt, IRCall)
                        and stmt.canonical_command == "::array"
                        and len(stmt.args) >= 2
                        and stmt.args[0] == "set"
                    ):
                        continue
                    # Args after ``set`` are: arr-name, key-value-list.
                    if len(stmt.args) < 3:
                        continue
                    arr_name = stmt.args[1]
                    kv_text = stmt.args[2]
                    try:
                        from compiler.tcl_expr_eval import _split_tcl_list

                        items = _split_tcl_list(kv_text)
                    except Exception:
                        continue
                    if len(items) % 2 != 0:
                        continue
                    for i in range(0, len(items), 2):
                        key = items[i]
                        value = items[i + 1]
                        elem_name = f"{arr_name}({key})"
                        if elem_name in func_cs:
                            continue
                        vs = frozenset((value,))
                        func_cs[elem_name] = vs
                        all_constsets.setdefault(elem_name, vs)

        # Fallback: directly extract foreach iteration elements from the CFG.
        # SCCP barriers (e.g. oo::class create) may have widened foreach
        # variables to OVERDEFINED even when the list is statically known.
        for qname, fu_unit in _all_fus_named:
            func_cs = _func_constsets.setdefault(qname, {})
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if (
                        isinstance(stmt, IRCall)
                        and stmt.canonical_command in ("::foreach", "::lmap")
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
            for qname, fu_unit in _all_fus_named:
                func_cs = _func_constsets.setdefault(qname, {})
                for block in fu_unit.cfg.blocks.values():
                    for stmt in block.statements:
                        if not isinstance(stmt, IRAssignValue):
                            continue
                        # Use the shared parser (lexer-backed) instead
                        # of a bespoke regex -- it handles braced /
                        # quoted / nested cmd-subs.  (Finding 9 of the
                        # PR #498 / PR #499 follow-up review.)
                        parsed_cs = parse_command_substitution(stmt.value)
                        if parsed_cs is None:
                            continue
                        called = parsed_cs[0]
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
        # ``dict with``/``dict update`` barriers, paired with the set
        # of LOCAL variable names that are EXPLICITLY assigned in the
        # same proc by ``set``/``foreach``/``lassign`` etc.  A dispatch
        # site whose target name is in the explicit-assignment set
        # cannot have been unpacked by the dict (the explicit local
        # def came from the source, not from the dict), so the
        # ``in_dict_with`` blanket suppression does NOT apply.
        # (Finding 3 of the PR #498 / PR #499 follow-up review.)
        # ``dict_with_ranges`` entries are tuples
        # ``(start, end, locally_defined_names)``.

        dict_with_ranges: list[tuple[int, int, frozenset[str]]] = []
        _all_fus = [("::top", cu.top_level)] + list(cu.procedures.items())
        for qname, fu_unit in _all_fus:
            func_has_dw = False
            local_defs: set[str] = set()
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if (
                        isinstance(stmt, IRBarrier)
                        and stmt.canonical_command == "::dict"
                        and stmt.args
                        and stmt.args[0] in ("with", "update")
                    ):
                        func_has_dw = True
                    # Collect explicit local defs from every IR statement
                    # type: ``IRAssignConst`` / ``IRAssignValue`` /
                    # ``IRAssignExpr`` / ``IRIncr`` carry a ``name``;
                    # ``IRCall`` / ``IRBarrier`` carry ``defs``.  These
                    # cover all the source-visible local-binding shapes
                    # (``set X ...``, ``incr X``, foreach-binding,
                    # regexp/scan output vars when written, etc.) and
                    # are exactly the names "unpacked from dict" cannot
                    # collide with.
                    name_attr = getattr(stmt, "name", None)
                    if name_attr:
                        local_defs.add(name_attr)
                    stmt_defs = getattr(stmt, "defs", ())
                    if stmt_defs:
                        for n in stmt_defs:
                            if n:
                                local_defs.add(n)
            if func_has_dw:
                ir_proc = cu.ir_module.procedures.get(qname)
                if ir_proc is not None:
                    dict_with_ranges.append(
                        (
                            ir_proc.range.start.offset,
                            ir_proc.range.end.offset,
                            frozenset(local_defs),
                        )
                    )
                else:
                    # Top-level: covers entire source.
                    dict_with_ranges.append((0, 2**31, frozenset(local_defs)))

        # Object-factory provenance (W307 suppression).  A variable assigned
        # from a *namespaced* command substitution (``set obj [::struct::tree
        # …]``, ``set p [pt::rde …]``, ``grammar::me::tcl``) overwhelmingly
        # holds an object/ensemble command name in tcllib idiom — dispatching on
        # it (``$obj method``) is object dispatch, not a stray non-literal
        # command.  Collected name-level (suppress-only, so over-approximation
        # is safe) and — unlike typing the result OBJECT in the lattice — it
        # does NOT perturb shimmer/type analysis (the value's real intrep when
        # used as data is untouched).
        # Scoped to the *defining* proc's body range (start, end, names): a
        # factory assignment in one proc must not suppress a same-named variable
        # in another proc, where it may hold anything (incl. user input).
        #
        # Two phases:
        # 1. Direct factory locals: ``set X [namespaced::cmd ...]`` — the RHS
        #    head contains ``::``, treating it as an object/ensemble.
        # 2. Object-returning user procs (TRANSITIVE): ``proc f {} { set X
        #    [struct::graph]; return $X }`` returns an object.  Callers
        #    ``set TGraph [f ...]`` also hold an object.  Iterate to fixpoint
        #    so a chain ``f -> g -> h`` propagates.
        # Build a set of user-proc qualified-name candidates so we can
        # distinguish user-proc calls from builtin/namespaced builtins.
        # User procs go through the fixpoint (only added to factory
        # locals if they're proven object-returning); builtins (or
        # unknown commands) get the shape-based heuristic (any
        # namespaced cmd-sub is treated as a factory).
        # G3 closure: a namespaced user proc that returns a plain string
        # (e.g. ``namespace eval ::ns { proc make {} { return foo } }``)
        # is no longer falsely treated as an object factory.
        _user_proc_qnames = set(cu.ir_module.procedures.keys())

        def _is_user_proc(cmd_head: str) -> bool:
            """Return True iff *cmd_head* refers to a user-defined proc
            in this compilation unit (rather than a Tcl builtin or
            unknown command).  Conservative: false negatives just mean
            we'll treat the cmd as a (possibly object-returning) builtin,
            which is the pre-G3 behaviour."""
            if cmd_head in _user_proc_qnames:
                return True
            if f"::{cmd_head}" in _user_proc_qnames:
                return True
            return False

        # Set of qualified TclOO class names + their bare-tail aliases
        # for object-factory recognition.
        _oo_class_qnames = set(self.result.all_classes.keys())
        _oo_class_tails = {qn.rsplit("::", 1)[-1] for qn in _oo_class_qnames if "::" in qn}

        def _is_object_returning_command_head(cmd_head: str) -> bool:
            """Return True iff *cmd_head* is a command whose value-returning
            invocation produces an object handle.

            Recognises:
            * Known TclOO class commands (``C new``, ``C create %AUTO%``)
              -- the class command is registered globally and its return
              value is the instance command name.
            * Namespaced commands (``::ns::cmd``) -- the documented
              tcllib / TclOO factory convention.  This is a heuristic
              (an unknown namespaced command MAY return a plain string),
              kept because removing it loses suppressions across the
              tcllib corpus (``struct::graph``, ``pt::*``, etc.) that
              aren't yet registry-modelled.  D4-F6 partial closure:
              the ``new``-spelling heuristic on bare names (handled
              in ``_return_type_for_command``) IS removed, and the
              ``::`` rule is tightened below by REQUIRING the head to
              not be a known non-object-returning user proc.

            User procs are intentionally excluded here: they go through
            the fixpoint (only added to factory locals if all their
            returns are themselves object-returning).
            """
            if cmd_head in _oo_class_tails or f"::{cmd_head}" in _oo_class_qnames:
                return True
            if "::" in cmd_head:
                # D4-F6 partial: when the namespaced command IS a known
                # user proc and the fixpoint has NOT classified it as
                # object-returning, that's evidence it returns a plain
                # value -- override the ``::`` heuristic.
                qualified = cmd_head if cmd_head.startswith("::") else f"::{cmd_head}"
                if qualified in self.result.all_procs:
                    # User proc -- defer to the fixpoint (which uses
                    # ``object_returning_procs`` membership).
                    return False
                # D3-P5 closure: a registered namespaced command whose
                # return type is EXPLICITLY known to not be OBJECT
                # (e.g. STRING / INT / LIST) overrides the heuristic
                # too.  Most tcllib factories don't have return_type
                # set in the registry (they default to None which is
                # treated as unknown), so this catches the small set
                # of namespaced commands that do (http::*, clock::*,
                # etc.) without losing the factory suppression on the
                # bulk of tcllib factory commands.
                from compiler.registry.runtime import REGISTRY as _REG_W307
                from compiler.types import TclType as _TT

                _spec = _REG_W307.get_any(cmd_head) or _REG_W307.get_any(qualified)
                if _spec is not None:
                    rt = getattr(_spec, "return_type", None)
                    if rt is not None and rt is not _TT.OBJECT:
                        return False
                # D3-P5 fully open: unregistered ``::pkg::plain``-style
                # namespaced commands can't be distinguished from
                # legitimate factories without per-command registry
                # data.  Documented in tracker; fix requires tcllib /
                # Tk factory return-type registry coverage.
                return True
            return False

        factory_locals_by_proc: dict[str, set[str]] = {}
        return_var_by_proc: dict[str, str | None] = {}
        for qname, fu_unit in _all_fus:
            names: set[str] = set()
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, IRAssignValue) and stmt.name:
                        parsed = parse_command_substitution(stmt.value)
                        if parsed is not None and _is_object_returning_command_head(parsed[0]):
                            # G3: defer to fixpoint when the head is a
                            # user proc -- only add as factory local if
                            # the user proc is later proven object-
                            # returning (handled by the extension pass).
                            if not _is_user_proc(parsed[0]):
                                names.add(stmt.name)
            factory_locals_by_proc[qname] = names
            # Find the LAST IRReturn statement and its returned var (if any).
            return_var_by_proc[qname] = _last_return_var(fu_unit.cfg)

        # Fixpoint: a proc is "object-returning" if it returns one of its
        # factory-locals (set X = [namespaced::cmd], return $X) OR directly
        # returns a namespaced command substitution (return [struct::graph])
        # OR its last assignment to the returned var is ``[other_user_proc]``
        # where other_user_proc is object-returning.
        object_returning_procs: set[str] = set()
        # Seed: a proc is object-returning when EVERY feasible return
        # value is itself a namespaced cmd-sub.
        # G4 closure: tracking only the LAST return (current pre-fix
        # behaviour) was wrong -- a wrapper that returns ``[factory]``
        # on one branch and ``foo`` (a plain string) on another isn't
        # an object factory; ``$x method`` on the string-branch result
        # is a runtime error and W307 should fire.  Collect ALL returns
        # and require ALL of them to be namespaced cmd-subs.
        direct_return_factory_by_proc: dict[str, bool] = {}
        for qname, fu_unit in _all_fus:
            from compiler.cfg import CFGReturn
            from compiler.ir import IRReturn as _IRRet

            return_values: list[str] = []
            for block in fu_unit.cfg.blocks.values():
                for s in block.statements:
                    if isinstance(s, _IRRet) and s.value:
                        return_values.append(s.value)
                term = block.terminator
                if isinstance(term, CFGReturn) and isinstance(term.value, str):
                    return_values.append(term.value)
            if return_values and all(
                (
                    (parsed := parse_command_substitution(rv.strip())) is not None
                    and _is_object_returning_command_head(parsed[0])
                )
                for rv in return_values
            ):
                object_returning_procs.add(qname)
                direct_return_factory_by_proc[qname] = True
        for qname, ret_var in return_var_by_proc.items():
            if ret_var is None:
                continue
            if ret_var in factory_locals_by_proc.get(qname, set()):
                object_returning_procs.add(qname)
        # Transitive propagation: track which RHS user-proc names assign to
        # the returned var, and propagate up.
        # Build assignment map: qname -> {var_name: rhs_cmd_head}
        assigns_by_proc: dict[str, dict[str, str]] = {}
        for qname, fu_unit in _all_fus:
            assigns: dict[str, str] = {}
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, IRAssignValue) and stmt.name:
                        parsed = parse_command_substitution(stmt.value)
                        if parsed is not None:
                            assigns[stmt.name] = parsed[0]
            assigns_by_proc[qname] = assigns
        # Iterate to fixpoint.
        bare_to_qnames: dict[str, list[str]] = {}
        for qname in cu.ir_module.procedures:
            bare = qname.rsplit("::", 1)[-1]
            bare_to_qnames.setdefault(bare, []).append(qname)
        changed = True
        while changed:
            changed = False
            for qname, ret_var in return_var_by_proc.items():
                if qname in object_returning_procs or ret_var is None:
                    continue
                rhs_cmd = assigns_by_proc.get(qname, {}).get(ret_var)
                if rhs_cmd is None:
                    continue
                # Match call to known object-returning user proc.
                for cand_qname in (rhs_cmd, "::" + rhs_cmd, *bare_to_qnames.get(rhs_cmd, [])):
                    if cand_qname in object_returning_procs:
                        object_returning_procs.add(qname)
                        changed = True
                        break

        # Now extend factory-locals: a ``set X [user_proc]`` where user_proc is
        # object-returning is just as much an object factory as a namespaced
        # builtin.
        for qname, fu_unit in _all_fus:
            names = factory_locals_by_proc.get(qname, set())
            assigns = assigns_by_proc.get(qname, {})
            for var_name, rhs_cmd in assigns.items():
                if var_name in names:
                    continue
                for cand_qname in (rhs_cmd, "::" + rhs_cmd, *bare_to_qnames.get(rhs_cmd, [])):
                    if cand_qname in object_returning_procs:
                        names.add(var_name)
                        break
            factory_locals_by_proc[qname] = names

        factory_object_ranges: list[tuple[int, int, set[str]]] = []
        for qname, fu_unit in _all_fus:
            names = factory_locals_by_proc.get(qname, set())
            if not names:
                continue
            ir_proc = cu.ir_module.procedures.get(qname)
            if ir_proc is not None:
                factory_object_ranges.append(
                    (ir_proc.range.start.offset, ir_proc.range.end.offset, names)
                )
            else:
                factory_object_ranges.append((0, 2**31, names))  # top-level

        # Proc-parameter object dispatch (W307 suppression).  When a user
        # defines ``proc walk {tree} {foreach n [\$tree leaves] {\$tree
        # visit \$n}}`` the parameter ``tree`` is unambiguously designed
        # to receive an object handle — every dispatch on ``\$tree`` in
        # the body proves it.  Flagging W307 on those dispatches is
        # noise; the user has documented the proc's API contract.
        #
        # Detection: pre-compute, per enclosing proc, the set of var
        # names that are used as the head of any ``\$var subcmd ...``
        # site in that proc's body.  At W307 emission, suppress when the
        # site's var is BOTH a param of the enclosing proc AND in that
        # proc's dispatcher set.
        #
        # Sound under-approximation: only matches when the proc's own
        # body has at least one dispatch on the var (so the trait is
        # evidenced by the proc, not just an external assumption).
        proc_body_ranges: list[tuple[int, int, str, frozenset[str]]] = []
        for qname, pdef in self.result.all_procs.items():
            br = pdef.body_range
            if br is None:
                continue
            param_names = {p.name for p in pdef.params}
            proc_body_ranges.append((br.start.offset, br.end.offset, qname, frozenset(param_names)))
        # Sort by start so the FIRST hit when scanning is the innermost
        # enclosing proc (procs don't nest in Tcl, but namespace eval
        # bodies can wrap multiple procs — innermost-first is robust).
        proc_body_ranges.sort(key=lambda r: (r[0], -r[1]))

        # Sentinel scope for top-level statements — has no parameters,
        # but the multi-dispatch rule still applies (a script-level
        # ``$cn`` dispatched 9 times in mainloop.tcl is unambiguous
        # object usage even outside any proc body).
        _TOP_SCOPE = ("::top", frozenset())

        def _enclosing_proc_params(off: int) -> tuple[str, frozenset[str]]:
            for s, e, qname, params in reversed(proc_body_ranges):
                if s <= off <= e:
                    return qname, params
            return _TOP_SCOPE

        # First pass over var-command sites to identify, per proc, which
        # var names are dispatchers and how many times each is dispatched.
        # A param + ANY dispatch is suppressed (the param itself signals
        # the contract); a non-param LOCAL + ≥2 dispatches is suppressed
        # (multiple uses on the same var demonstrate intent — a single
        # dispatch could be a typo, multiple is clearly designed).
        proc_dispatcher_vars: dict[str, set[str]] = {}
        proc_dispatch_counts: dict[str, dict[str, int]] = {}
        for var_name, _mn, site_range, _im, _cws, _argc in self._var_command_sites:
            qname, _params = _enclosing_proc_params(site_range.start.offset)
            proc_dispatcher_vars.setdefault(qname, set()).add(var_name)
            counts = proc_dispatch_counts.setdefault(qname, {})
            counts[var_name] = counts.get(var_name, 0) + 1

        # Taint-aware safety: never suppress W307 on a var that's tainted
        # in its enclosing proc, even if it's a param or dispatched ≥2x.
        # ``set usercmd [gets stdin]; \$usercmd op1; \$usercmd op2`` is a
        # genuine command-injection risk regardless of how many times
        # the user dispatches on it.  Build a per-proc set of tainted
        # var names — any version of the name carrying a tainted lattice
        # value disqualifies it from dispatcher-suppression.
        tainted_var_names: dict[str, set[str]] = {}
        for qname, fu_unit in _all_fus_named:
            tainted_names: set[str] = set()
            for (var_name, _ver), tl in fu_unit.analysis.taints.items():
                if tl.tainted:
                    tainted_names.add(var_name)
            if tainted_names:
                tainted_var_names[qname] = tainted_names

        # snit instance-variable / component dispatch (W307 suppression).  A
        # snit type's instance variables and components frequently hold object
        # handles (``component myparser`` / ``variable myparser`` assigned
        # ``[pt::rde …]`` in the constructor).  Dispatch on them — including
        # from type-private procs that ``upvar`` the instance var — is object
        # dispatch.  Their factory assignment lives inside the snit body (an IR
        # barrier), so it never reaches the compiler CU; recover it from the
        # snit ClassDefs the analyser built.  Scoped to each type's body range
        # so a same-named scalar elsewhere is unaffected.
        # Build per-class body ranges + their declared instance/type
        # variables for ALL class definitions, not just snit (oo::class
        # and oo::define classes use the same ``class_def.variables``
        # data from analyser/_analyser/_oo.py).  Both kinds of methods
        # legitimately dispatch on the class's instance vars without
        # the analyser seeing the var's binding via SCCP; the var-name
        # check below uses these ranges to suppress W307 on those
        # variables ONLY (no longer a blanket "anywhere in a method"
        # suppression).  (F4 of the PR #498/#499 follow-up review.)
        snit_var_ranges: list[tuple[int, int, frozenset[str]]] = []
        snit_body_ranges: list[tuple[int, int]] = []
        for class_def in self.result.all_classes.values():
            br = class_def.body_range
            # snit-style bodies keep the historical name; record their
            # range separately because some checks (snit reserved
            # self-refs ``$self`` etc.) only apply inside snit bodies.
            if "snit::" in class_def.metaclass:
                snit_body_ranges.append((br.start.offset, br.end.offset))
            if class_def.variables:
                snit_var_ranges.append(
                    (br.start.offset, br.end.offset, frozenset(class_def.variables))
                )

        for (
            var_name,
            method_name,
            site_range,
            in_method,
            cmd_word_single,
            _positional_argc,  # 6th tuple element -- used by W214 dispatcher check
        ) in self._var_command_sites:
            # snit's reserved object/type self-references (``$self foo``,
            # ``$type bar``, ``$selfns``, ``$win``, ``$hull configure``) are
            # object dispatch — but only inside a snit type body.  Scoped to the
            # snit body range (or a modelled method scope), so a same-named
            # variable in a vanilla proc / top-level script still gets W307.
            if var_name in _OO_SELF_REFS:
                _sr = site_range.start.offset
                if in_method or any(s <= _sr <= e for s, e in snit_body_ranges):
                    continue
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
                            # snit method resolution is too dynamic to validate
                            # soundly — instances respond to delegated methods,
                            # hull/component forwards, options-as-methods and
                            # snit built-ins (info/destroy/configure/cget) — so
                            # suppress W308 for snit types (W307 dispatch is
                            # still suppressed via the OBJECT typing).
                            if "snit::" in cd.metaclass:
                                found = True
                                break
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
                # Per-SSA-version refinement (post-stage2 §A): prefer the
                # value at the dispatch's exact use-version when available.
                # This drops the merged-set false positive on a variable
                # reassigned from a non-command to a known command before
                # the dispatch (``set c x; set c parse; $c ...``).  Falls
                # back to the merged set when no precise version is found.
                if cmd_word_single:
                    precise_vals = _precise_cmd_values(site_range.start.offset, var_name)
                    if precise_vals is not None:
                        constset_vals = precise_vals
                if (
                    cmd_word_single
                    and constset_vals is not None
                    and all(
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
                    )
                ):
                    # All possible command names are statically known
                    # *and* the entire command word is just ``$var`` —
                    # suppress W307.  If every resolved name happens to
                    # be disabled in the active dialect, fire W002
                    # instead.  Composite words like ``${cmd}x``
                    # dispatch to ``<value>x``, not the literal value,
                    # so neither the suppression nor the W002 lookup
                    # applies — those fall through to the generic
                    # W307 path below.
                    _maybe_emit_w002(constset_vals, site_range)
                    continue

                # Emit W307 unless inside a method body or a function with
                # dict-with where $var is very likely an object from dict
                # unpacking.  Finding 3 of the PR #498 / PR #499 follow-up:
                # the suppression only applies to var-names that DON'T
                # have an explicit local def in the same proc.  When the
                # source has ``set cmd nope; $cmd arg``, ``cmd`` has an
                # explicit local def, so the dict can't be its source
                # and W307 must still fire on the dispatch.  The base-
                # name check handles array-element dispatches like
                # ``$state(-foo)`` -- ``state`` is the local def.
                _site_off = site_range.start.offset
                _base_for_dw_check = var_name
                if "(" in var_name and var_name.endswith(")"):
                    _base_for_dw_check = var_name[: var_name.index("(")]
                in_dict_with = any(
                    s <= _site_off <= e and _base_for_dw_check not in locals_set
                    for s, e, locals_set in dict_with_ranges
                )
                _off = site_range.start.offset
                is_factory_object = any(
                    s <= _off <= e and var_name in names for s, e, names in factory_object_ranges
                )
                is_snit_member = any(
                    s <= _off <= e and var_name in names for s, e, names in snit_var_ranges
                )
                # Proc-parameter / multi-dispatch suppression: when this
                # site is in a proc that uses ``$var`` as a dispatcher,
                # suppress W307 when either:
                # (a) ``var`` is a parameter of the enclosing proc — the
                #     param itself documents the API contract; or
                # (b) ``var`` is a local (not a param) dispatched ≥2
                #     times in the same proc body — multiple uses on
                #     the same var demonstrate the user designed it as
                #     an object handle (a single dispatch on a local
                #     could be a typo, but multiple is firm intent).
                # Safety: a TAINTED var is never suppressed — even if
                # dispatched many times, ``set cmd [gets stdin]; \$cmd
                # op1; \$cmd op2`` is a real command-injection risk.
                is_proc_param_dispatcher = False
                _qname, _params = _enclosing_proc_params(_off)
                # Extract the base name for array-element dispatch.  When the
                # var_name is ``foo(key)`` (array element), the BASE is
                # ``foo`` — and if ``foo`` is itself a proc parameter, the
                # user designed the proc to receive a callback-table-style
                # array (eg ``proc f {Verify} { \$Verify(key) ... }``).  The
                # param contract is on ``foo``, not ``foo(key)``.
                _base_name = var_name
                if "(" in var_name and var_name.endswith(")"):
                    _base_name = var_name[: var_name.index("(")]
                if var_name in proc_dispatcher_vars.get(
                    _qname, ()
                ) and var_name not in tainted_var_names.get(_qname, ()):
                    if var_name in _params:
                        is_proc_param_dispatcher = True
                    elif proc_dispatch_counts.get(_qname, {}).get(var_name, 0) >= 2:
                        is_proc_param_dispatcher = True
                # Array-element dispatch on a PARAM-array: ``\$Verify(key)``
                # where ``Verify`` is a parameter — the param is the
                # callback-table contract.
                if (
                    not is_proc_param_dispatcher
                    and _base_name != var_name
                    and _base_name in _params
                    and _base_name not in tainted_var_names.get(_qname, ())
                ):
                    is_proc_param_dispatcher = True
                # Callback in an array element: ``$state(-command) $token``
                # (switch-style) and ``$state(openCmd) $arg`` /
                # ``$state(doneCallback)`` (suffix-style) are the documented
                # Tcl/Tk idioms for a user-configurable callback (widget
                # ``-command``, http ``-proxyfilter``, internal state with
                # registered command/callback/handler).  The user has
                # explicitly assigned a command name to this slot — flagging
                # W307 on the dispatch is noise.  Detect via array-element
                # form with EITHER a dash-prefixed key (switch) OR a key
                # whose final word is ``cmd``/``command``/``callback``/
                # ``handler``/``hook`` (case-insensitive suffix match).
                is_switch_callback_element = False
                if "(" in var_name and var_name.endswith(")"):
                    _open = var_name.index("(")
                    _key = var_name[_open + 1 : -1]
                    if _key.startswith("-"):
                        is_switch_callback_element = True
                    else:
                        _key_lower = _key.lower()
                        for _suffix in (
                            "cmd",
                            "command",
                            "callback",
                            "handler",
                            "hook",
                            "proc",
                        ):
                            if _key_lower.endswith(_suffix):
                                is_switch_callback_element = True
                                break
                # Namespaced ensemble dispatch: ``${log}::debug "msg"`` is
                # the documented "logger / namespaced ensemble" idiom —
                # the variable holds a namespace prefix, the literal
                # ``::tail`` appended makes a qualified command path.
                # tcllib's logger module + dns/spf/irc/multiplexer all
                # use this shape.  The ``::`` suffix after a ``${var}``
                # / ``$var`` is a strong signal the user explicitly
                # constructed a namespaced command path.  (The VAR token
                # range covers the var name only; for ``${log}`` the
                # closing ``}`` is the next char, so step past it.)
                is_namespaced_ensemble = False
                _src_off = site_range.end.offset + 1
                if _src_off < len(cu.source) and cu.source[_src_off] == "}":
                    _src_off += 1
                if _src_off + 1 < len(cu.source) and cu.source[_src_off : _src_off + 2] == "::":
                    is_namespaced_ensemble = True

                # SCCP evidence override (PR #498 deep-review gaps G5-G8):
                # the heuristic suppressions above (multi-dispatch local,
                # callback-shaped array key, dash-prefixed array key,
                # namespaced ensemble) are shape-based -- they assume the
                # user designed the slot as an object handle / callback
                # registration / namespaced command path.  But when SCCP
                # has concrete evidence the variable holds a CONST string
                # value that ISN'T a registered command/proc/class, the
                # heuristic is wrong and W307 should fire.  Compute the
                # SCCP-says-not-a-command predicate and use it to nullify
                # the heuristic suppressions for *this* site.
                _scoped_cs_w307 = _constsets_for_offset(site_range.start.offset)
                _const_vals_w307 = _scoped_cs_w307.get(var_name)
                # For array elements (``foo(key)``), SCCP often tracks
                # the value on the BASE name -- check it too.
                if _const_vals_w307 is None and _base_name != var_name:
                    _const_vals_w307 = _scoped_cs_w307.get(_base_name)

                def _const_value_is_known_command(v) -> bool:
                    if not isinstance(v, str):
                        return False
                    return (
                        v in _known_cmds
                        or v in _known_procs
                        or v in _known_proc_bare
                        or f"::{v}" in _known_procs
                        or v in all_typed_vars
                        or v in _class_tail_names
                        or f"::{v}" in self.result.all_classes
                    )

                sccp_says_not_a_command = (
                    _const_vals_w307 is not None
                    and len(_const_vals_w307) > 0
                    and not any(_const_value_is_known_command(v) for v in _const_vals_w307)
                )

                # D4-F7 closure -- composed-name check for the
                # ``${ns}::tail`` ensemble idiom.  The *prefix* var
                # holds the namespace name; the user is dispatching
                # ``$prefix::tail`` as a known qualified command path.
                # The previous gate (``not sccp_says_not_a_command``)
                # only ran the composition when the prefix alone was a
                # known command, which never matches real ensembles.
                # Run the composition unconditionally when:
                #
                #   1. The site is a namespaced-ensemble shape
                #      (``${var}::literal-tail``), AND
                #   2. SCCP knows the prefix value(s) (CONST or CONSTSET).
                #
                # If EVERY composed name resolves to a known command,
                # set ``sccp_says_not_a_command = False`` and keep
                # ``is_namespaced_ensemble = True`` -- the user's
                # composition is statically resolvable, so the W307 is
                # an FP.  If EVERY composition is unknown, set
                # ``sccp_says_not_a_command = True`` -- the user
                # constructed a path the analyser can't model, fire
                # W307.  Mixed: stay conservative (silent) -- the
                # namespaced ensemble suppression already applies.
                if is_namespaced_ensemble and _const_vals_w307:
                    _tail_start = _src_off + 2
                    _tail = _scan_tcl_command_name(cu.source, _tail_start)
                    if _tail:
                        all_composed_known = all(
                            isinstance(v, str)
                            and (
                                _const_value_is_known_command(f"{v}::{_tail}")
                                or _const_value_is_known_command(f"::{v}::{_tail}")
                            )
                            for v in _const_vals_w307
                        )
                        all_composed_unknown = all(
                            isinstance(v, str)
                            and not _const_value_is_known_command(f"{v}::{_tail}")
                            and not _const_value_is_known_command(f"::{v}::{_tail}")
                            for v in _const_vals_w307
                        )
                        if all_composed_known:
                            # The composed command is statically
                            # resolvable -- override the prefix-only
                            # not-a-command verdict.
                            sccp_says_not_a_command = False
                        elif all_composed_unknown:
                            sccp_says_not_a_command = True
                        # mixed -> leave sccp_says_not_a_command as the
                        # prefix-only verdict (conservative).

                if sccp_says_not_a_command:
                    is_proc_param_dispatcher = False
                    is_switch_callback_element = False
                    is_namespaced_ensemble = False
                    in_dict_with = False

                # F4 closure (PR #498/#499 follow-up): the old
                # ``in_method`` blanket suppression hid genuine W307
                # cases like ``oo::class create C { method m {} { set
                # cmd nope; \$cmd arg } }``.  Drop it as a standalone
                # gate -- object-dispatch evidence inside a method
                # body must come from a specific positive signal:
                # ``is_snit_member`` (which now also covers oo::class
                # instance vars via the widened class-vars range),
                # snit reserved self-ref (caught via the OO_SELF_REFS
                # exemption above), factory-object provenance,
                # multi-dispatch, switch-callback array element,
                # namespaced ensemble, or (via SCCP) a known-command
                # CONST value.  Locals set to non-command literals
                # inside a method body now correctly fire W307.
                if (
                    not in_dict_with
                    and not is_factory_object
                    and not is_snit_member
                    and not is_proc_param_dispatcher
                    and not is_switch_callback_element
                    and not is_namespaced_ensemble
                    and "W307" not in self._disabled_diagnostics
                ):
                    self.result.diagnostics.append(
                        Diagnostic(
                            range=site_range,
                            message="Non-literal command name — cannot statically analyze",
                            severity=Severity.WARNING,
                            code="W307",
                        )
                    )

        # Resolve [cmd]-as-command sites: suppress W307 when the command
        # substitution returns a TclOO object (e.g. ``[Dog new] bark``).
        if self._cmd_command_sites:
            from compiler.core_analyses import _return_type_for_command

            _known_classes = frozenset(self.result.all_classes)
            # Build set of W307 ranges from the check pipeline that we may
            # need to remove.
            w307_indices: dict[tuple[int, int], int] = {}
            for i, d in enumerate(self.result.diagnostics):
                if d.code == "W307":
                    w307_indices[(d.range.start.offset, d.range.end.offset)] = i

            remove_indices: list[int] = []
            for (
                cmd_text,
                method_name,
                site_range,
                in_method,
                cmd_word_single,
            ) in self._cmd_command_sites:
                # Composite words like ``[Dog new]x`` dispatch to the
                # *concatenated* string, not the substitution's return
                # value, so the substitution's return type tells us
                # nothing about the actual command.  Skip W307
                # suppression and W308 method validation entirely — the
                # generic W307 from the check pipeline correctly stands.
                # EXCEPT: ``[ns_func]::method`` is the same namespaced
                # ensemble idiom as ``${log}::method`` — the inner cmd
                # provides the namespace prefix and ``::tail`` literal
                # forms the qualified command path.  Verify by checking
                # the source: cmd-sub immediately followed by ``::``.
                if not cmd_word_single:
                    # CMD-sub token range covers content (no brackets),
                    # so step past the closing ``]`` to find what
                    # follows.
                    _src_off = site_range.end.offset + 1
                    if _src_off < len(cu.source) and cu.source[_src_off] == "]":
                        _src_off += 1
                    if _src_off + 1 < len(cu.source) and cu.source[_src_off : _src_off + 2] == "::":
                        key = (site_range.start.offset, site_range.end.offset)
                        idx = w307_indices.get(key)
                        if idx is not None:
                            remove_indices.append(idx)
                    continue
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
                # D3-P4 closure (partial): when ``my <method>`` /
                # ``self <method>`` resolves to a method in the current
                # class whose body is a simple ``return <literal>``, we
                # KNOW the return value is a plain string -- override
                # the self-dispatch object heuristic and fire W307.
                # For more complex method bodies (any cmd-sub, var
                # interpolation, multiple statements, ``return [...]``)
                # we conservatively keep the heuristic (chained-call
                # idiom dominates).
                if is_oo_self_dispatch and len(cmd_args_) >= 1:
                    method_name_ = cmd_args_[0]
                    site_off = site_range.start.offset
                    for class_def in self.result.all_classes.values():
                        cb = class_def.body_range
                        if not (cb.start.offset <= site_off <= cb.end.offset):
                            continue
                        md = class_def.methods.get(method_name_) or class_def.class_methods.get(
                            method_name_
                        )
                        if md is None:
                            break
                        # body_range may exclude the closing brace, so
                        # be lenient: try both as-is and brace-stripped.
                        body_text = cu.source[md.body_range.start.offset : md.body_range.end.offset]
                        bt = body_text.strip()
                        if bt.startswith("{"):
                            # Drop leading ``{`` (and trailing ``}`` if
                            # present) to get the inner body text.
                            bt = bt[1:].rstrip()
                            if bt.endswith("}"):
                                bt = bt[:-1].rstrip()
                        bt = bt.strip()
                        # Simple ``return <literal>`` -- no cmd-sub, no
                        # var interpolation, no other commands.
                        if bt.startswith("return ") and "\n" not in bt and ";" not in bt:
                            ret_arg = bt[len("return ") :].strip()
                            if ret_arg and "[" not in ret_arg and "$" not in ret_arg:
                                # Plain literal return -- string, not object.
                                is_oo_self_dispatch = False
                        break
                if is_oo_self_dispatch or (
                    ret_type.kind is TypeKind.KNOWN and ret_type.tcl_type is TclType.OBJECT
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

            # Remove suppressed W307 diagnostics.  Rebuild in one pass
            # (O(N)) instead of repeated del-by-index (each del shifts the
            # tail — O(R·N) when many sites are suppressed).
            if remove_indices:
                drop = set(remove_indices)
                self.result.diagnostics[:] = [
                    d for i, d in enumerate(self.result.diagnostics) if i not in drop
                ]

        # Dedup W307 vs W101 (eval-injection): both fire on the same
        # site for ``eval \$cmd`` / ``uplevel \$cmd``-style dynamic
        # dispatch.  W101 is the canonical eval-injection warning and
        # carries the specific message about double substitution; W307
        # is the generic "non-literal command name".  When both fire at
        # the same start offset, drop the W307 (more specific code
        # wins).  W101's end offset may differ from W307's by ±1 (token
        # vs argument range), so match on start offset only.
        if "W307" not in self._disabled_diagnostics:
            w101_starts: set[int] = {
                d.range.start.offset for d in self.result.diagnostics if d.code == "W101"
            }
            if w101_starts:
                self.result.diagnostics[:] = [
                    d
                    for d in self.result.diagnostics
                    if not (d.code == "W307" and d.range.start.offset in w101_starts)
                ]
