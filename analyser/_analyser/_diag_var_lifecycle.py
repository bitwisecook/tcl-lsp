# canonicalisation: audited #246
from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.cfg import CFGFunction
from compiler.core_analyses import FunctionAnalysis
from compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
    IRProcedure,
    IRStatement,
)
from shared.document_buffer import DocumentBuffer
from shared.naming import normalise_qualified_name as _normalise_qualified_name
from shared.naming import normalise_var_name as _normalise_var_name
from shared.proc_traits import ProcArgTrait

from ..semantic_model import Diagnostic, Severity
from ._utils import (
    _UNUSED_VAR_RE,
    _format_literal_for_message,
    _possible_paste_fingerprint,
    narrow_to_variable,
)


class _AnalyserDiagVarLifecycleMixin(_Base):
    """W210/211/213/214/220 + H300 diagnostics: variable lifecycle."""

    def _lifecycle_buffer(self) -> DocumentBuffer:
        """Cached ``DocumentBuffer`` over the current source for range narrowing."""
        src = self._source
        cached = getattr(self, "_lifecycle_buf_cache", None)
        if cached is None or cached[0] is not src:
            buffer = DocumentBuffer.from_source(src)
            self._lifecycle_buf_cache = (src, buffer)
            return buffer
        return cached[1]

    def _collect_call_by_name_reads(self, cfg: CFGFunction) -> set[str]:
        """Return caller-local variable names passed *by name* to a user proc
        that consumes them via ``upvar`` (call-by-name).

        The trait inference in ``compiler.proc_arg_traits`` already detects
        when a user proc's parameter is a name-reference (``VAR_READ`` /
        ``VAR_WRITE``) by recognising bodies like
        ``upvar 1 $paramN local; set local …``.  Those traits live on
        ``ProcDef.param_traits``.  At a call site like
        ``validate_imodules_cmp im dm`` where the receiver's params have
        those traits, the literal-name args ``im``/``dm`` are an *indirect
        read/write* of the caller's variables of those names.

        Without this view the analyser treats ``im``/``dm`` as set-but-never-
        used (W211) and the preceding writes as dead stores (W220).  By
        consuming the existing ``param_traits`` lattice we close that gap.

        Sound under-approximation: only literal-name args matter; a
        substituted (``$varname``) or array-element (``foo(key)``) arg
        names a variable we can't statically identify, so it is *not*
        suppressed — that preserves real W211/W220 cases where the user
        accidentally passed a literal string instead of a name.
        """
        all_procs = self.result.all_procs
        if not all_procs:
            return set()
        # Build a lookup keyed by both qualified and bare names so calls
        # like ``validate_imodules_cmp`` resolve to ``::validate_imodules_cmp``.
        bare_index: dict[str, list] = {}
        for qname, pdef in all_procs.items():
            if not pdef.param_traits:
                continue
            bare = qname.split("::")[-1]
            bare_index.setdefault(bare, []).append(pdef)
            bare_index.setdefault(qname, []).append(pdef)
            bare_index.setdefault(qname.lstrip(":"), []).append(pdef)

        by_name_reads: set[str] = set()
        for block in cfg.blocks.values():
            for stmt in block.statements:
                if not isinstance(stmt, (IRCall, IRBarrier)):
                    continue
                cmd = stmt.command
                if not cmd or "$" in cmd or "[" in cmd:
                    continue
                # Try the call name directly, then qualified, then bare.
                cand = bare_index.get(cmd) or bare_index.get(_normalise_qualified_name(cmd))
                if cand is None:
                    cand = bare_index.get(cmd.lstrip(":"))
                if cand is None:
                    continue
                # If multiple procs share the bare name, conservatively suppress
                # only when ALL of them mark the arg as a name-receiver — that
                # way we never over-suppress on a coincidental name collision.
                for pdef in cand:
                    params = [p.name for p in pdef.params]
                    for i, arg in enumerate(stmt.args):
                        if i >= len(params):
                            break
                        pname = params[i]
                        traits = pdef.param_traits.get(pname, frozenset())
                        if (
                            ProcArgTrait.VAR_READ not in traits
                            and ProcArgTrait.VAR_WRITE not in traits
                        ):
                            continue
                        if not arg or "$" in arg or "[" in arg:
                            continue
                        # Must be a plain identifier (Tcl-local naming rules).
                        name = _normalise_var_name(arg)
                        if name and all(ch.isalnum() or ch in "_:" for ch in name):
                            by_name_reads.add(name)
        return by_name_reads

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
        call_by_name = self._collect_call_by_name_reads(cfg)
        for dead in analysis.dead_stores:
            if dead.variable in cross_event_vars:
                continue
            if dead.variable in call_by_name:
                # The variable is passed by literal name to a user proc that
                # upvar's it — that is an indirect read/write the per-function
                # SSA can't see.  Suppress (interprocedural call-by-name lattice).
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
            narrowed = narrow_to_variable(
                self._source,
                self._lifecycle_buffer(),
                stmt_range,
                variable=dead.variable,
                kind="assigned_name",
            )
            self.result.diagnostics.append(
                Diagnostic(
                    range=narrowed or stmt_range,
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
                narrowed = narrow_to_variable(
                    self._source,
                    self._lifecycle_buffer(),
                    r,
                    variable=rbs.variable,
                    kind="named_arg",
                )
                self.result.diagnostics.append(
                    Diagnostic(
                        range=narrowed or r,
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
                narrowed = (
                    narrow_to_variable(
                        self._source,
                        self._lifecycle_buffer(),
                        r,
                        variable=rbs.variable,
                        kind="read_var",
                    )
                    if stmt is not None
                    else None
                )
                self.result.diagnostics.append(
                    Diagnostic(
                        range=narrowed or r,
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
        call_by_name = self._collect_call_by_name_reads(cfg)
        for unused in analysis.unused_variables:
            if unused.variable in cross_event_vars:
                continue
            if unused.variable in call_by_name:
                # Same as W220: passed by literal name to a name-receiver
                # user proc — the upvar makes it an indirect read.
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
            narrowed = narrow_to_variable(
                self._source,
                self._lifecycle_buffer(),
                stmt_range,
                variable=unused.variable,
                kind="assigned_name",
            )
            self.result.diagnostics.append(
                Diagnostic(
                    range=narrowed or stmt_range,
                    message=msg,
                    severity=Severity.HINT,
                    code="W211",
                )
            )

    def _dispatch_protocol_signatures(self) -> set[tuple[str, tuple[str, ...]]]:
        """Identify ``(namespace, leading-param-list)`` pairs that look like a
        **dispatch protocol** — a family of peer procs in the same namespace
        all accepting an identical leading-param signature dictated by an
        external dispatcher (parser-rule visitor, snit method protocol, etc.).

        For procs in such a family, the protocol params are part of an
        external contract and most rules genuinely don't use them; firing
        W214 on every rule is noise.  Concrete tcllib example: ~140 procs
        in ``pt::peg::from::peg::GEN::*`` all accept ``{s e}`` (start/end
        positions); most pure-pattern rules don't read either.

        Heuristic (sound for noise reduction; not for hiding real findings):
        a ``(namespace, tuple-of-leading-param-names)`` qualifies when AT
        LEAST 3 peer procs in the same namespace share that exact
        leading-param prefix.  ``args`` is excluded from the prefix
        (variadic catch-all should not count toward shape identity).

        Lazy: computed once per AnalysisResult and cached on the mixin.
        Returns the set of qualifying ``(namespace, params_tuple)`` keys.
        """
        cached = getattr(self, "_dispatch_proto_cache", None)
        if cached is not None and cached[0] is self.result:
            return cached[1]
        # Group user procs by (namespace, leading-param-tuple).
        groups: dict[tuple[str, tuple[str, ...]], int] = {}
        for qname, pdef in self.result.all_procs.items():
            # Namespace = everything up to the last ``::`` (or top-level).
            ns = qname.rsplit("::", 1)[0] or "::"
            # Build the leading-param tuple, stopping at ``args`` (variadic).
            params: list[str] = []
            for p in pdef.params:
                if p.name == "args":
                    break
                params.append(p.name)
            if not params:
                continue
            key = (ns, tuple(params))
            groups[key] = groups.get(key, 0) + 1
        # A protocol needs ≥3 peer procs sharing the signature shape.
        proto = {key for key, n in groups.items() if n >= 3}
        self._dispatch_proto_cache = (self.result, proto)
        return proto

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
        # Dispatch-protocol suppression: when ≥3 peer procs in the same
        # namespace share the same leading-param signature, those params are
        # an external contract (parser-rule visitor, snit method protocol,
        # tcllib pt::peg::from::peg::GEN::* rules — every rule gets ``{s e}``
        # whether it uses them or not).  Firing W214 on every rule that
        # ignores its protocol params is noise.  The shared-signature
        # detection is structural and sound: a contract is only inferred
        # when multiple peers carry it.
        if proc_def is not None:
            ns = ir_proc.qualified_name.rsplit("::", 1)[0] or "::"
            params: list[str] = []
            for p in proc_def.params:
                if p.name == "args":
                    break
                params.append(p.name)
            proto = self._dispatch_protocol_signatures()
            if (ns, tuple(params)) in proto:
                # All leading (non-``args``) params are protocol-dictated.
                # Suppress W214 only on those — if the proc has additional
                # params beyond the protocol, those would still fire.
                protocol_params = frozenset(params)
                remaining = [p for p in analysis.unused_params if p not in protocol_params]
            else:
                remaining = list(analysis.unused_params)
        else:
            remaining = list(analysis.unused_params)
        for param_name in remaining:
            self.result.diagnostics.append(
                Diagnostic(
                    range=ir_proc.range,
                    message=(f"Parameter '{param_name}' of proc '{ir_proc.name}' is unused"),
                    severity=Severity.HINT,
                    code="W214",
                )
            )
