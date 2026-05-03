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

from ..expr_ast import (
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprTernary,
    ExprUnary,
    vars_in_expr_node,
)
from ..ir import (
    InlineDecision,
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
)

# Matches both ``$name`` and ``${name}`` substitutions.  Tcl
# variable names are alphanumeric + underscore + ``::`` for fully
# qualified names.  ``$arr(idx)`` is also matched and contributes
# the array name (``arr``) — that's correct for our purposes.
_VAR_REF_RE = re.compile(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*(?:\([^)]*\))?)")

# Matches ``[set varname]`` (1-arg form = read).  The 2-arg form
# ``[set varname value]`` would have non-']' content after the name,
# so this pattern only matches the read form.
_SET_READ_RE = re.compile(r"\[set\s+([A-Za-z_][A-Za-z0-9_]*)\s*\]")


def dce_module(module: IRModule, summaries: object | None = None) -> IRModule:
    """Return a new module with dead local stores removed.

    Eligibility (PR #237 review): consult the per-proc
    ``safe_to_dce`` predicate from the post-fixpoint escape
    summary when available — that's the precise proof for
    "this proc's locals are unobservable externally".  When
    ``summaries`` is None (legacy callers) we fall back to the
    inline catalogue tag (``ALWAYS`` ⇒ ``pure_leaf`` ⇒
    over-conservatively safe), which is strictly stricter.

    Procs whose locals ARE observable (upvar source, info
    fallback, dynamic_barrier) keep their stores intact because
    the syntactic read-walk can't see those reach-ins.
    """

    new_procs: dict[str, IRProcedure] = {}
    changed_any = False
    for qname, proc in module.procedures.items():
        eligible = _proc_is_dce_eligible(proc, summaries, qname)
        if not eligible:
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


def _proc_is_dce_eligible(
    proc: IRProcedure,
    summaries: object | None,
    qname: str,
) -> bool:
    """Decide whether DCE may rewrite *proc*'s body.

    Prefers the per-proc ``safe_to_dce`` predicate when an
    interprocedural-resolved summaries map is supplied.  Otherwise
    falls back to the catalogue-tagged ``InlineDecision.ALWAYS``
    gate — strictly stricter, so the legacy path stays sound.
    """
    if summaries is not None:
        # Late import + duck-typed lookup so this module stays
        # decoupled from the var-escape implementation while
        # still consuming its precise predicate when present.
        try:
            summary = summaries[qname]  # type: ignore[index]
        except (KeyError, TypeError):
            summary = None
        if summary is not None and getattr(summary, "safe_to_dce", False):
            return True
        return False
    return proc.inline_decision is InlineDecision.ALWAYS


def _dce_script(script: IRScript, *, params: set[str]) -> IRScript:
    """Drop dead assignments from ``script`` and every nested
    control-flow body it contains.

    ``params`` lists names bound by the enclosing proc's parameter
    list — we must not delete writes to a parameter slot because
    callers may observe the parameter's input value through
    ``[info args]`` / debug paths even when the body never reads it.

    **Implicit-result protection (PR #237 review).**  Tcl proc
    semantics: the value of the *last* command in the body becomes
    the proc's return value when no explicit ``return`` fires.
    ``proc p {} {set y 42}`` returns ``42`` because ``set`` returns
    the value it stored.  If DCE deletes that ``set`` because
    ``y`` is unread, the proc's return value silently changes to
    ``""``.  Therefore the terminal statement of the *outermost*
    script is always preserved regardless of read/write counts.

    **Nested-body recursion (PR #237 review).**  Previously the
    pass walked only top-level statements, missing dead stores
    inside ``if {…} { set y 42 }``, ``for {…}`` etc.  We now
    recurse using the outer script's aggregate read/write counts
    as the eligibility gate — that's correct because
    ``_count_writes`` / ``_collect_reads`` walk the whole script
    transitively.  The terminal-protection rule applies only to
    the outermost script's last statement (a branch's last
    statement is *not* the proc's implicit return).
    """
    reads = _collect_reads(script)
    writes = _count_writes(script)
    return _dce_script_with_counts(
        script,
        params=params,
        reads=reads,
        writes=writes,
        outer_terminal=True,
    )


def _dce_script_with_counts(
    script: IRScript,
    *,
    params: set[str],
    reads: dict[str, int],
    writes: dict[str, int],
    outer_terminal: bool,
) -> IRScript:
    """Helper that consumes pre-computed read/write counts so the
    recursive walk doesn't re-scan the proc on every body."""

    new_stmts: list = []
    changed = False
    last_index = len(script.statements) - 1 if outer_terminal else -1
    for i, stmt in enumerate(script.statements):
        is_terminal = outer_terminal and i == last_index
        target = _assign_target(stmt)
        if (
            target is not None
            and _is_dceable_name(target)
            and target not in params
            and writes.get(target, 0) == 1
            and reads.get(target, 0) == 0
            and not _stmt_has_side_effects(stmt)
            and not is_terminal
        ):
            changed = True
            continue
        rewritten = _dce_recurse_nested(stmt, params=params, reads=reads, writes=writes)
        if rewritten is not stmt:
            changed = True
        new_stmts.append(rewritten)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _dce_recurse_nested(
    stmt: object,
    *,
    params: set[str],
    reads: dict[str, int],
    writes: dict[str, int],
) -> object:
    """Apply DCE recursively to nested control-flow bodies in
    ``stmt``.  Nested bodies use ``outer_terminal=False`` because
    only the proc's outermost trailing statement is the implicit
    return.  Returns ``stmt`` unchanged when no nested rewrite
    fires."""

    def _ds(body):
        return _dce_script_with_counts(
            body,
            params=params,
            reads=reads,
            writes=writes,
            outer_terminal=False,
        )

    if isinstance(stmt, IRBlock):
        new_body = _ds(stmt.body)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRIf):
        new_clauses: list = []
        any_clause_changed = False
        for clause in stmt.clauses:
            new_body = _ds(clause.body)
            if new_body is clause.body:
                new_clauses.append(clause)
            else:
                new_clauses.append(replace(clause, body=new_body))
                any_clause_changed = True
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _ds(stmt.else_body)
            else_changed = new_else is not stmt.else_body
        if not any_clause_changed and not else_changed:
            return stmt
        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else)
    if isinstance(stmt, IRFor):
        new_init = _ds(stmt.init)
        new_next = _ds(stmt.next)
        new_body = _ds(stmt.body)
        if new_init is stmt.init and new_next is stmt.next and new_body is stmt.body:
            return stmt
        return replace(stmt, init=new_init, next=new_next, body=new_body)
    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        new_body = _ds(stmt.body)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRTry):
        new_body = _ds(stmt.body)
        new_handlers: list = []
        any_handler_changed = False
        for h in stmt.handlers:
            new_h_body = _ds(h.body)
            if new_h_body is h.body:
                new_handlers.append(h)
            else:
                new_handlers.append(replace(h, body=new_h_body))
                any_handler_changed = True
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _ds(stmt.finally_body)
            finally_changed = new_finally is not stmt.finally_body
        if new_body is stmt.body and not any_handler_changed and not finally_changed:
            return stmt
        return replace(
            stmt,
            body=new_body,
            handlers=tuple(new_handlers),
            finally_body=new_finally,
        )
    if isinstance(stmt, IRSwitch):
        new_arms: list = []
        any_arm_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            new_arm_body = _ds(arm.body)
            if new_arm_body is arm.body:
                new_arms.append(arm)
            else:
                new_arms.append(replace(arm, body=new_arm_body))
                any_arm_changed = True
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _ds(stmt.default_body)
            default_changed = new_default is not stmt.default_body
        if not any_arm_changed and not default_changed:
            return stmt
        return replace(stmt, arms=tuple(new_arms), default_body=new_default)
    return stmt


def _stmt_has_side_effects(stmt: object) -> bool:
    """True iff *stmt* may have observable side effects beyond
    writing its target slot.

    Codex P1 (PR #237): ``set x [expr {[puts hi]}]`` writes ``x`` and
    *also* runs ``puts hi`` for its side effect.  Even if the body
    never reads ``x``, removing the statement also removes the
    ``puts`` call — which is observable.  The DCE pass must keep
    such stores.

    The check is conservative — false positives just decline a
    deletion, never delete a side-effecting store.

    * :class:`IRAssignConst` — value is a literal, no substitutions
      possible, never side-effecting.
    * :class:`IRAssignValue` — the raw string may contain ``[cmd]``
      command substitutions.  A ``[`` anywhere in the string flags
      it as side-effecting (we don't try to detect literal ``[`` in
      braced contexts; the false-positive cost is "we keep the
      assignment", which is fine).
    * :class:`IRAssignExpr` — recurse the :class:`ExprNode` AST and
      flag any embedded :class:`ExprCommand`.  Math ops, literals,
      and var refs are pure.
    * :class:`IRIncr` — the amount string can contain ``[cmd]``.
    """

    if isinstance(stmt, IRAssignConst):
        return False
    if isinstance(stmt, IRAssignValue):
        return "[" in stmt.value
    if isinstance(stmt, IRAssignExpr):
        return _expr_has_command_subst(stmt.expr)
    if isinstance(stmt, IRIncr):
        return stmt.amount is not None and "[" in stmt.amount
    # Other statement types aren't candidates for DCE here, so the
    # answer is moot.
    return False


def _expr_has_command_subst(node: object) -> bool:
    """True iff the :class:`ExprNode` tree contains an
    :class:`ExprCommand` node anywhere.  Walks recursively through
    binary / unary / ternary / call subtrees.  ``ExprLiteral`` /
    ``ExprString`` / ``ExprVar`` are leaves that never carry a
    command substitution."""

    if isinstance(node, ExprCommand):
        return True
    if isinstance(node, ExprBinary):
        return _expr_has_command_subst(node.left) or _expr_has_command_subst(node.right)
    if isinstance(node, ExprUnary):
        return _expr_has_command_subst(node.operand)
    if isinstance(node, ExprTernary):
        return (
            _expr_has_command_subst(node.condition)
            or _expr_has_command_subst(node.true_branch)
            or _expr_has_command_subst(node.false_branch)
        )
    if isinstance(node, ExprCall):
        # ``rand()`` / ``srand()`` are stateful but not command
        # substitutions per se.  We only flag command-substitution
        # side effects here; the GVN denylist is a separate axis.
        # However, a function arg can itself be an ExprCommand, so
        # walk arguments anyway.
        return any(_expr_has_command_subst(a) for a in node.args)
    return False


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
    read count for each name found.  Also recognises ``[set varname]``
    (the 1-arg read form) as a variable read.

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
    for m in _SET_READ_RE.finditer(text):
        name = m.group(1)
        if name:
            counts[name] = counts.get(name, 0) + 1
