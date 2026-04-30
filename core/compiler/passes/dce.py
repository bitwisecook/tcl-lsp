"""S5.4 — Dead-store elimination on IR.

Removes ``IRAssignConst`` / ``IRAssignValue`` / ``IRAssignExpr`` /
``IRIncr`` statements whose target variable is never read again in
the same procedure body.  The codegen otherwise emits the wasted
``obj_new_*`` + retain/release wrap on every iteration, even
though no observer can ever see the value.

**Soundness gate.**  The pass only operates on procs whose escape
summary is ``pure_leaf`` (no eval-fallback, no upvar-source, no
dynamic barrier).  A non-pure_leaf proc may have hidden reads via
``eval`` / ``uplevel`` / ``info`` that the syntactic walk can't
see; deleting an "unused" assignment in those cases would change
observable state.

**Single-write / zero-read rule.**  We only delete assignments
whose target is written exactly once and read zero times in the
same body.  Multi-write cases (a slot updated repeatedly with
only the final value mattering) need flow-sensitive reasoning the
pass doesn't have today and remain untouched.

**Read detection.**  String-valued fields are scanned for
``$name`` and ``${name}`` patterns; expression ASTs are walked for
``ExprVar(name=name)`` nodes; ``IRCall.reads`` and ``IRForeach``
iterator vars contribute explicit references.  The walker is
deliberately conservative — any plausible mention of the name
keeps the assignment alive — so false positives just decline a
deletion, never delete a live store.
"""

from __future__ import annotations

import re
from dataclasses import replace

from ..expr_ast import vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBlock,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRModule,
    IRProcedure,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
    InlineDecision,
)


# Matches both ``$name`` and ``${name}`` substitutions.  Tcl
# variable names are alphanumeric + underscore + ``::`` for fully
# qualified names.  ``$arr(idx)`` is also matched and contributes
# the array name (``arr``) — that's correct for our purposes.
_VAR_REF_RE = re.compile(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*(?:\([^)]*\))?)")


def dce_module(module: IRModule) -> IRModule:
    """Return a new module with dead local stores removed.

    Only procs that the inline catalogue tagged ``ALWAYS`` (i.e.
    ``pure_leaf`` after the interprocedural pass) are eligible.
    Other procs may have eval-fallback / upvar / info reads the
    syntactic walk can't see, so their assignments stay put.
    """

    new_procs: dict[str, IRProcedure] = {}
    changed_any = False
    for qname, proc in module.procedures.items():
        if proc.inline_decision is not InlineDecision.ALWAYS:
            new_procs[qname] = proc
            continue
        new_body = _dce_script(proc.body, params=set(proc.params))
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
            changed_any = True
    if not changed_any:
        return module
    return replace(module, procedures=new_procs)


def _dce_script(script: IRScript, *, params: set[str]) -> IRScript:
    """Drop dead top-level assignments from ``script``.

    ``params`` lists names bound by the enclosing proc's parameter
    list — we must not delete writes to a parameter slot because
    callers may observe the parameter's input value through
    ``[info args]`` / debug paths even when the body never reads it.
    """
    reads = _collect_reads(script)
    writes = _count_writes(script)

    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        target = _assign_target(stmt)
        if (
            target is not None
            and _is_dceable_name(target)
            and target not in params
            and writes.get(target, 0) == 1
            and reads.get(target, 0) == 0
        ):
            changed = True
            continue
        new_stmts.append(stmt)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _is_dceable_name(name: str) -> bool:
    """Return True iff ``name`` is a bare local that DCE may delete.

    Excludes:

    * ``::``-qualified names — those are globals, observable from
      outside the proc (other procs / interpreted scripts / the
      embedding host).
    * Array element names (``arr(idx)``) — the array's other
      elements may be read elsewhere; partial DCE on one element
      isn't sound without alias-aware reasoning.
    * Names containing namespace separators (``ns::var``) — same
      observability concern as ``::``-qualified.
    """
    if name.startswith("::"):
        return False
    if "(" in name:
        return False
    if "::" in name:
        return False
    return True


def _assign_target(stmt: object) -> str | None:
    """Return the local-var name the statement writes, or ``None``."""
    if isinstance(stmt, IRAssignConst):
        return stmt.name
    if isinstance(stmt, IRAssignValue):
        return stmt.name
    if isinstance(stmt, IRAssignExpr):
        return stmt.name
    if isinstance(stmt, IRIncr):
        return stmt.name
    return None


def _count_writes(script: IRScript) -> dict[str, int]:
    counts: dict[str, int] = {}
    _walk_writes(script, counts)
    return counts


def _walk_writes(script: IRScript | None, counts: dict[str, int]) -> None:
    if script is None:
        return
    for stmt in script.statements:
        target = _assign_target(stmt)
        if target is not None:
            counts[target] = counts.get(target, 0) + 1
        if isinstance(stmt, IRCall):
            for d in stmt.defs:
                counts[d] = counts.get(d, 0) + 1
        # Recurse into nested control-flow bodies.
        if isinstance(stmt, IRBlock):
            _walk_writes(stmt.body, counts)
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _walk_writes(clause.body, counts)
            _walk_writes(stmt.else_body, counts)
        elif isinstance(stmt, IRFor):
            _walk_writes(stmt.init, counts)
            _walk_writes(stmt.next, counts)
            _walk_writes(stmt.body, counts)
        elif isinstance(stmt, IRWhile):
            _walk_writes(stmt.body, counts)
        elif isinstance(stmt, IRForeach):
            for var_list, _ in stmt.iterators:
                for v in var_list:
                    counts[v] = counts.get(v, 0) + 1
            _walk_writes(stmt.body, counts)
        elif isinstance(stmt, IRCatch):
            _walk_writes(stmt.body, counts)
            if stmt.result_var:
                counts[stmt.result_var] = counts.get(stmt.result_var, 0) + 1
            if stmt.options_var:
                counts[stmt.options_var] = counts.get(stmt.options_var, 0) + 1
        elif isinstance(stmt, IRTry):
            _walk_writes(stmt.body, counts)
            for handler in stmt.handlers:
                _walk_writes(handler.body, counts)
                if handler.var_name:
                    counts[handler.var_name] = counts.get(handler.var_name, 0) + 1
                if handler.options_var:
                    counts[handler.options_var] = counts.get(handler.options_var, 0) + 1
            _walk_writes(stmt.finally_body, counts)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                _walk_writes(arm.body, counts)
            _walk_writes(stmt.default_body, counts)
        elif isinstance(stmt, IRUpFrame):
            _walk_writes(stmt.body, counts)


def _collect_reads(script: IRScript) -> dict[str, int]:
    counts: dict[str, int] = {}
    _walk_reads(script, counts)
    return counts


def _walk_reads(script: IRScript | None, counts: dict[str, int]) -> None:
    if script is None:
        return
    for stmt in script.statements:
        _read_one(stmt, counts)


def _read_one(stmt: object, counts: dict[str, int]) -> None:
    if isinstance(stmt, IRAssignValue):
        _scan_string(stmt.value, counts)
        return
    if isinstance(stmt, IRAssignExpr):
        for name in vars_in_expr_node(stmt.expr):
            counts[name] = counts.get(name, 0) + 1
        return
    if isinstance(stmt, IRIncr):
        # ``incr x`` reads x before writing — the var is live.
        counts[stmt.name] = counts.get(stmt.name, 0) + 1
        if stmt.amount is not None:
            _scan_string(stmt.amount, counts)
        return
    if isinstance(stmt, IRAssignConst):
        # Pure literal write — no reads on the RHS.
        return
    if isinstance(stmt, IRCall):
        for arg in stmt.args:
            _scan_string(arg, counts)
        for r in stmt.reads:
            counts[r] = counts.get(r, 0) + 1
        if stmt.reads_own_defs:
            for d in stmt.defs:
                counts[d] = counts.get(d, 0) + 1
        return
    if isinstance(stmt, IRReturn):
        if stmt.value is not None:
            _scan_string(stmt.value, counts)
        if stmt.expr is not None:
            for name in vars_in_expr_node(stmt.expr):
                counts[name] = counts.get(name, 0) + 1
        return
    if isinstance(stmt, IRExprEval):
        for name in vars_in_expr_node(stmt.expr):
            counts[name] = counts.get(name, 0) + 1
        return
    if isinstance(stmt, IRBlock):
        _walk_reads(stmt.body, counts)
        return
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            for name in vars_in_expr_node(clause.condition):
                counts[name] = counts.get(name, 0) + 1
            _walk_reads(clause.body, counts)
        _walk_reads(stmt.else_body, counts)
        return
    if isinstance(stmt, IRFor):
        _walk_reads(stmt.init, counts)
        for name in vars_in_expr_node(stmt.condition):
            counts[name] = counts.get(name, 0) + 1
        _walk_reads(stmt.next, counts)
        _walk_reads(stmt.body, counts)
        return
    if isinstance(stmt, IRWhile):
        for name in vars_in_expr_node(stmt.condition):
            counts[name] = counts.get(name, 0) + 1
        _walk_reads(stmt.body, counts)
        return
    if isinstance(stmt, IRForeach):
        for _, list_arg in stmt.iterators:
            _scan_string(list_arg, counts)
        _walk_reads(stmt.body, counts)
        return
    if isinstance(stmt, IRCatch):
        _walk_reads(stmt.body, counts)
        return
    if isinstance(stmt, IRTry):
        _walk_reads(stmt.body, counts)
        for handler in stmt.handlers:
            _walk_reads(handler.body, counts)
        _walk_reads(stmt.finally_body, counts)
        return
    if isinstance(stmt, IRSwitch):
        _scan_string(stmt.subject, counts)
        for arm in stmt.arms:
            _walk_reads(arm.body, counts)
        _walk_reads(stmt.default_body, counts)
        return
    if isinstance(stmt, IRUpFrame):
        _walk_reads(stmt.body, counts)
        return


def _scan_string(text: str, counts: dict[str, int]) -> None:
    """Scan ``text`` for ``$var`` / ``${var}`` substitutions and bump the
    read count for each name found.

    Conservative: any match (even inside what might be a string
    literal a parser would not interpret as a substitution) keeps
    the variable alive.  False positives just decline a deletion.
    """
    if not text:
        return
    for m in _VAR_REF_RE.finditer(text):
        name = m.group(1) or m.group(2)
        if name is None:
            continue
        # Strip array element: ``arr(idx)`` → ``arr``.
        paren = name.find("(")
        if paren >= 0:
            name = name[:paren]
        if name:
            counts[name] = counts.get(name, 0) + 1
