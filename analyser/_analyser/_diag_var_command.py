# canonicalisation: audited #246
from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.compilation_unit import CompilationUnit
from compiler.ir import (
    IRAssignValue,
    IRBarrier,
    IRCall,
)
from compiler.parsing.known_commands import known_command_names
from compiler.value_shapes import parse_command_substitution

from ..semantic_model import Diagnostic, Severity

# snit reserved object/type self-references — ``$self``/``$type``/``$selfns``/
# ``$win``/``$hull`` used as a command word are object dispatch, but *only*
# inside a snit type body (``$hull configure`` is the widgetadaptor delegation
# idiom).  Outside a snit body these are ordinary variable names, so the
# exemption must be scoped (see the membership check at the use site) — a
# vanilla ``proc f {} { set self …; $self foo }`` must still get W307.
_OO_SELF_REFS = frozenset({"self", "type", "selfns", "win", "hull"})


class _AnalyserDiagVarCommandMixin(_Base):
    """W307/W308 diagnostics: variable-as-command patterns."""

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
            import re as _re

            _CMD_SUB_RE = _re.compile(r"^\[(\S+?)(?:\s.*)?\]$")

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
                        and stmt.canonical_command == "::dict"
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
        factory_object_ranges: list[tuple[int, int, set[str]]] = []
        for qname, fu_unit in _all_fus:
            names: set[str] = set()
            for block in fu_unit.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, IRAssignValue) and stmt.name:
                        parsed = parse_command_substitution(stmt.value)
                        if parsed is not None and "::" in parsed[0]:
                            names.add(stmt.name)
            if names:
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

        def _enclosing_proc_params(off: int) -> tuple[str, frozenset[str]] | None:
            for s, e, qname, params in reversed(proc_body_ranges):
                if s <= off <= e:
                    return qname, params
            return None

        # First pass over var-command sites to identify, per proc, which
        # var names are dispatchers and how many times each is dispatched.
        # A param + ANY dispatch is suppressed (the param itself signals
        # the contract); a non-param LOCAL + ≥2 dispatches is suppressed
        # (multiple uses on the same var demonstrate intent — a single
        # dispatch could be a typo, multiple is clearly designed).
        proc_dispatcher_vars: dict[str, set[str]] = {}
        proc_dispatch_counts: dict[str, dict[str, int]] = {}
        for var_name, _mn, site_range, _im, _cws in self._var_command_sites:
            enc = _enclosing_proc_params(site_range.start.offset)
            if enc is None:
                continue
            qname, _params = enc
            proc_dispatcher_vars.setdefault(qname, set()).add(var_name)
            counts = proc_dispatch_counts.setdefault(qname, {})
            counts[var_name] = counts.get(var_name, 0) + 1

        # snit instance-variable / component dispatch (W307 suppression).  A
        # snit type's instance variables and components frequently hold object
        # handles (``component myparser`` / ``variable myparser`` assigned
        # ``[pt::rde …]`` in the constructor).  Dispatch on them — including
        # from type-private procs that ``upvar`` the instance var — is object
        # dispatch.  Their factory assignment lives inside the snit body (an IR
        # barrier), so it never reaches the compiler CU; recover it from the
        # snit ClassDefs the analyser built.  Scoped to each type's body range
        # so a same-named scalar elsewhere is unaffected.
        snit_var_ranges: list[tuple[int, int, frozenset[str]]] = []
        snit_body_ranges: list[tuple[int, int]] = []
        for class_def in self.result.all_classes.values():
            if "snit::" in class_def.metaclass:
                br = class_def.body_range
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
                # unpacking.
                in_dict_with = any(s <= site_range.start.offset <= e for s, e in dict_with_ranges)
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
                is_proc_param_dispatcher = False
                _enc = _enclosing_proc_params(_off)
                if _enc is not None:
                    _qname, _params = _enc
                    if var_name in proc_dispatcher_vars.get(_qname, ()):
                        if var_name in _params:
                            is_proc_param_dispatcher = True
                        elif proc_dispatch_counts.get(_qname, {}).get(var_name, 0) >= 2:
                            is_proc_param_dispatcher = True
                if (
                    not in_method
                    and not in_dict_with
                    and not is_factory_object
                    and not is_snit_member
                    and not is_proc_param_dispatcher
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
                if not cmd_word_single:
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

            # Remove suppressed W307 diagnostics.  Rebuild in one pass
            # (O(N)) instead of repeated del-by-index (each del shifts the
            # tail — O(R·N) when many sites are suppressed).
            if remove_indices:
                drop = set(remove_indices)
                self.result.diagnostics[:] = [
                    d for i, d in enumerate(self.result.diagnostics) if i not in drop
                ]
