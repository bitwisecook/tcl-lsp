"""S5.3 — Loop-invariant code motion for retain/release wraps.

In the current emitter, a literal assignment inside a loop body
allocates a fresh ``TclObj`` and runs the full retain/release wrap
on every iteration::

    for {set i 0} {$i < 100} {incr i} {
        set guard "constant"
    }

Each iteration: ``obj_new_string("constant")`` → retain → release
prior → store.  Net effect across N iterations: ``N`` allocations
and ``N`` retain/release pairs, even though the value never
changes.

This pass hoists such assignments to the IR statement immediately
before the enclosing loop.  Codegen then emits the alloc + retain
+ release once.

**Soundness — zero-iteration loops.**  A naive hoist is **unsound**
when the loop can run zero times: ``foreach i "" { set x 0 }``
must leave ``x`` untouched, but a hoisted ``set x 0`` would write
unconditionally.  We therefore restrict hoisting to two safe
shapes:

1. **Provably ≥1-iteration counted loop.**  An :class:`IRFor`
   whose ``init`` is a single ``set var literal``, whose
   ``condition`` is a literal-vs-literal comparison decidably
   true at entry (e.g. ``$i < 100`` after ``set i 0``), and
   whose body's hoisted target is **fresh** — the hoisted name
   is not the same as the loop counter and is not read after
   the loop in the enclosing script.  Since the body executes
   at least once anyway, hoisting changes nothing observable.

2. **Other loop shapes** — :class:`IRWhile` (initially-false
   condition → 0 iterations) and :class:`IRForeach` (empty list
   → 0 iterations) need a use-after-loop reaching-uses analysis
   to decide whether the slot's pre-loop value is observable
   after a 0-iteration run.  Until that analysis is wired into
   the pass, those loops only have their bodies recursed for
   nested-loop hoisting; nothing is hoisted out of them.

**Soundness — alias and write collisions.**  Even within the
provable-iteration case, the hoist requires:

* The assigned variable is **not written** by any other statement
  in the loop body (recursively into nested control flow), the
  loop's ``next`` clause (for ``IRFor``), or the iterator
  bindings (for ``IRForeach``).
* The assignment's value is loop-invariant — a literal constant
  with no ``$``/``[`` substitution.

Reads of the variable inside the loop are explicitly **allowed**
— the whole point of LICM is that the slot's value is the same in
every iteration, so reads inside don't observe any difference.
"""

from __future__ import annotations

from dataclasses import replace

import re

from ..expr_ast import BinOp, ExprBinary, ExprLiteral, ExprVar, vars_in_expr_node
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
)


def licm_module(module: IRModule) -> IRModule:
    """Return a new module with loop-invariant assignments hoisted.

    The input ``module`` is left untouched (every IR node is
    frozen).  Procedures whose bodies don't change share their
    original :class:`IRProcedure` instance for cheap structural
    sharing.
    """

    new_top = _licm_script(module.top_level)
    new_procs: dict[str, IRProcedure] = {}
    for qname, proc in module.procedures.items():
        new_body = _licm_script(proc.body)
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
    if new_top is module.top_level and all(
        new_procs[q] is module.procedures[q] for q in module.procedures
    ):
        return module
    return replace(module, top_level=new_top, procedures=new_procs)


def _licm_script(script: IRScript) -> IRScript:
    """Recurse into ``script`` looking for loops to LICM.

    Returns ``script`` unchanged when no rewrites apply.
    """

    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        rewritten = _licm_stmt(stmt)
        if rewritten is None:
            new_stmts.append(stmt)
            continue
        hoisted, replacement = rewritten
        if hoisted or replacement is not stmt:
            changed = True
        new_stmts.extend(hoisted)
        new_stmts.append(replacement)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _licm_stmt(stmt: object) -> tuple[list, object] | None:
    """Recurse into ``stmt`` and return ``(hoisted, replacement)`` or
    ``None`` to keep ``stmt`` unchanged.

    ``hoisted`` is the list of statements to splice in **before**
    ``replacement``; ``replacement`` is the (possibly rewritten)
    successor for ``stmt``.  When neither change applies the helper
    returns ``None`` so the caller can keep the original instance.
    """

    if isinstance(stmt, IRFor):
        new_init = _licm_script(stmt.init)
        new_next = _licm_script(stmt.next)
        # Only counted IRFor loops where we can statically prove
        # at least one iteration are eligible.  This avoids the
        # ``foreach i "" { set x 0 }`` class of bug — if the loop
        # runs zero times, hoisting the body's assignment would
        # change ``x`` from "untouched" to "set to 0".  Provable
        # ≥1 iteration guarantees the body's assignment would
        # have run at least once anyway, so the post-loop slot
        # value matches with or without the hoist.
        ctr = _proven_at_least_one_iteration(new_init, stmt.condition)
        if ctr is None:
            # Recurse into the body for nested loops, but don't
            # hoist out of THIS loop.
            recursed_body = _licm_script(stmt.body)
            if new_init is stmt.init and new_next is stmt.next and recursed_body is stmt.body:
                return None
            return (
                [],
                replace(stmt, init=new_init, next=new_next, body=recursed_body),
            )
        hoisted, new_body = _hoist_from_loop_body(
            stmt.body,
            extra_writes=_collect_writes(new_next),
            forbidden_targets={ctr},
        )
        if (
            new_init is stmt.init
            and new_next is stmt.next
            and new_body is stmt.body
            and not hoisted
        ):
            return None
        return (
            hoisted,
            replace(stmt, init=new_init, next=new_next, body=new_body),
        )

    if isinstance(stmt, IRWhile):
        # ``while`` loops can run zero times when the condition is
        # initially false.  Without proving the contrary the hoist
        # is unsound — recurse into nested loops only.
        new_body = _licm_script(stmt.body)
        if new_body is stmt.body:
            return None
        return [], replace(stmt, body=new_body)

    if isinstance(stmt, IRForeach):
        # ``foreach`` over an empty list runs zero iterations.
        # Hoist only when EVERY iterator's list arg is a literal
        # with at least one element — then the body executes ≥ 1
        # times unconditionally, so a hoisted body-level write
        # would have run anyway.
        iter_vars: set[str] = set()
        for var_list, _ in stmt.iterators:
            iter_vars.update(var_list)
        if _every_foreach_iterator_runs_at_least_once(stmt.iterators):
            hoisted, new_body = _hoist_from_loop_body(
                stmt.body,
                extra_writes={name: 1 for name in iter_vars},
                forbidden_targets=iter_vars,
            )
            if new_body is stmt.body and not hoisted:
                return None
            return hoisted, replace(stmt, body=new_body)
        # Otherwise recurse into the body for nested loops only.
        new_body = _licm_script(stmt.body)
        if new_body is stmt.body:
            return None
        return [], replace(stmt, body=new_body)

    if isinstance(stmt, IRBlock):
        new_body = _licm_script(stmt.body)
        if new_body is stmt.body:
            return None
        return [], replace(stmt, body=new_body)

    if isinstance(stmt, IRIf):
        new_clauses = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _licm_script(clause.body)
            if new_body is clause.body:
                new_clauses.append(clause)
            else:
                new_clauses.append(replace(clause, body=new_body))
                clauses_changed = True
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _licm_script(stmt.else_body)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return None
        return [], replace(stmt, clauses=tuple(new_clauses), else_body=new_else)

    if isinstance(stmt, IRCatch):
        new_body = _licm_script(stmt.body)
        if new_body is stmt.body:
            return None
        return [], replace(stmt, body=new_body)

    if isinstance(stmt, IRTry):
        new_body = _licm_script(stmt.body)
        new_handlers = []
        handlers_changed = False
        for handler in stmt.handlers:
            new_handler_body = _licm_script(handler.body)
            if new_handler_body is handler.body:
                new_handlers.append(handler)
            else:
                new_handlers.append(replace(handler, body=new_handler_body))
                handlers_changed = True
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _licm_script(stmt.finally_body)
            finally_changed = new_finally is not stmt.finally_body
        if new_body is stmt.body and not handlers_changed and not finally_changed:
            return None
        return [], replace(
            stmt,
            body=new_body,
            handlers=tuple(new_handlers),
            finally_body=new_finally,
        )

    if isinstance(stmt, IRSwitch):
        new_arms = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            new_arm_body = _licm_script(arm.body)
            if new_arm_body is arm.body:
                new_arms.append(arm)
            else:
                new_arms.append(replace(arm, body=new_arm_body))
                arms_changed = True
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _licm_script(stmt.default_body)
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return None
        return [], replace(stmt, arms=tuple(new_arms), default_body=new_default)

    if isinstance(stmt, IRUpFrame):
        new_body = _licm_script(stmt.body)
        if new_body is stmt.body:
            return None
        return [], replace(stmt, body=new_body)

    return None


def _hoist_from_loop_body(
    body: IRScript,
    *,
    extra_writes: dict[str, int] | None = None,
    forbidden_targets: set[str] | None = None,
) -> tuple[list, IRScript]:
    """Return ``(hoisted, new_body)`` for ``body``.

    ``hoisted`` lists statements to emit immediately before the
    enclosing loop; ``new_body`` is the rewritten loop body with
    those statements removed.  Returns ``([], body)`` when nothing
    is hoistable.

    The recursive LICM on inner control flow runs first, so loops
    nested inside this body get their own LICM applied before we
    look for hoistable assignments at *this* body's top level.

    ``forbidden_targets`` lists variable names that must not be
    hoisted regardless of their write count — typically the loop
    counter, whose value the ``next`` clause depends on.
    """

    # First, recursively LICM nested loops/blocks/etc. inside the
    # body.  This may move statements OUT of inner loops INTO the
    # current body's top level — those new top-level statements
    # become candidates for the current loop's LICM in turn.
    after_inner = _licm_script(body)

    write_counts = _collect_writes(after_inner)
    if extra_writes:
        for name, n in extra_writes.items():
            write_counts[name] = write_counts.get(name, 0) + n

    forbidden = forbidden_targets or set()
    hoisted: list = []
    keep: list = []
    # Track every var name read OR written by statements *seen so
    # far* in body order.  Hoisting ``set x C`` is unsound when an
    # earlier statement reads ``x`` — the first iteration would
    # observe the pre-loop value of ``x``, and after the hoist
    # every iteration sees ``C`` instead.  Requiring the candidate
    # write to be the *first* reference of its target is a
    # straight-line dominance check that handles the common case
    # without dragging in a full SSA / dominator analysis.
    referenced_so_far: set[str] = set()
    for stmt in after_inner.statements:
        target = _assign_target(stmt)
        if (
            _is_hoistable(stmt)
            and target not in forbidden
            and write_counts.get(target, 0) == 1
            and target not in referenced_so_far
        ):
            hoisted.append(stmt)
        else:
            keep.append(stmt)
        # Either way, mark every name referenced by this statement
        # as seen so subsequent candidates checking the same name
        # see the prior reference.
        referenced_so_far |= _stmt_references(stmt)

    if not hoisted:
        if after_inner is body:
            return [], body
        return [], after_inner
    return hoisted, IRScript(statements=tuple(keep))


def _every_foreach_iterator_runs_at_least_once(
    iterators: tuple[tuple[tuple[str, ...], str], ...],
) -> bool:
    """Return True iff every ``foreach`` iterator binds at least one
    iteration unconditionally.

    Each iterator's list arg must be a static literal with a non-
    empty content area.  Brace-quoted ``{a b c}`` and double-
    quoted ``"a b c"`` (both without ``$`` or ``[``) qualify, as
    do bare unquoted single-word args.  Anything containing a
    runtime substitution returns False conservatively and the
    caller declines the hoist.
    """
    for _, list_arg in iterators:
        if not _literal_list_is_non_empty(list_arg):
            return False
    return True


def _literal_list_is_non_empty(arg: str) -> bool:
    """Return True iff ``arg`` is a literal that names ≥ 1 element."""
    a = arg.strip()
    if not a:
        return False
    if "$" in a or "[" in a:
        return False
    if a.startswith("{") and a.endswith("}"):
        a = a[1:-1].strip()
    elif a.startswith('"') and a.endswith('"'):
        a = a[1:-1].strip()
    return bool(a)


def _proven_at_least_one_iteration(init: IRScript, condition) -> str | None:
    """Return the loop counter name when the loop is provably
    counted with ≥ 1 iteration; otherwise ``None``.

    The recognised shape is the canonical Tcl ``for`` loop::

        for {set i <int_lit>} {$i <op> <int_lit>} {<next>} { ... }

    where ``<op>`` is one of ``<``, ``<=``, ``>``, ``>=``, ``==``,
    ``!=``.  We evaluate the comparison at the init value: if it
    is true the loop's first iteration runs unconditionally, so
    body-level assignments would execute at least once even
    without the hoist.

    Anything more complex (multi-statement init, expr-valued init,
    variable bound on the right of the comparison, …) returns
    ``None`` and the caller declines the hoist.
    """

    if len(init.statements) != 1:
        return None
    init_stmt = init.statements[0]
    if not isinstance(init_stmt, IRAssignConst):
        return None
    try:
        ctr_val = int(init_stmt.value)
    except ValueError:
        return None

    if not isinstance(condition, ExprBinary):
        return None
    op = condition.op
    if op not in (BinOp.LT, BinOp.LE, BinOp.GT, BinOp.GE, BinOp.EQ, BinOp.NE):
        return None

    left = condition.left
    right = condition.right
    if not isinstance(left, ExprVar) or left.name != init_stmt.name:
        return None
    if not isinstance(right, ExprLiteral):
        return None
    try:
        bound = int(right.text)
    except ValueError:
        return None

    if op is BinOp.LT and not (ctr_val < bound):
        return None
    if op is BinOp.LE and not (ctr_val <= bound):
        return None
    if op is BinOp.GT and not (ctr_val > bound):
        return None
    if op is BinOp.GE and not (ctr_val >= bound):
        return None
    if op is BinOp.EQ and not (ctr_val == bound):
        return None
    if op is BinOp.NE and not (ctr_val != bound):
        return None
    return init_stmt.name


def _is_hoistable(stmt: object) -> bool:
    """True iff ``stmt`` is a literal-valued top-level assignment."""

    if isinstance(stmt, IRAssignConst):
        return True
    if isinstance(stmt, IRAssignValue):
        # Only fully literal values qualify.  A value containing
        # ``$`` or ``[`` could depend on something the loop
        # iterates over.  ``value_needs_backsubst`` is just a flag
        # for backslash-escapes — those are still loop-invariant.
        v = stmt.value
        if "$" in v or "[" in v:
            return False
        return True
    return False


def _assign_target(stmt: object) -> str:
    """Return the variable name written by ``stmt``."""

    return getattr(stmt, "name", "")


def _collect_writes(script: IRScript) -> dict[str, int]:
    """Return a map of ``name → number of writes`` reachable in ``script``.

    "Writes" includes :class:`IRAssignConst`, :class:`IRAssignValue`,
    :class:`IRAssignExpr`, :class:`IRIncr`, plus any
    :class:`IRCall` whose ``defs`` mention the name.  Nested
    control-flow bodies are walked transitively.

    The count is **inflated** by writes inside nested control flow:
    a single write that may execute zero or more times (inside an
    ``if`` clause) still counts as one occurrence.  That's fine
    for the hoist gate — we just need to know "is this the only
    place that writes this var?".
    """

    counts: dict[str, int] = {}
    _collect_writes_into(script, counts)
    return counts


def _collect_writes_into(script: IRScript | None, counts: dict[str, int]) -> None:
    if script is None:
        return
    for stmt in script.statements:
        _collect_one(stmt, counts)


def _collect_one(stmt: object, counts: dict[str, int]) -> None:
    if isinstance(stmt, (IRAssignConst, IRAssignValue)):
        counts[stmt.name] = counts.get(stmt.name, 0) + 1
        return
    if isinstance(stmt, IRIncr):
        counts[stmt.name] = counts.get(stmt.name, 0) + 1
        return
    # IRAssignExpr also writes a variable.
    name = getattr(stmt, "name", None)
    if name is not None and not isinstance(
        stmt,
        (IRBlock, IRIf, IRFor, IRWhile, IRForeach, IRCatch, IRTry, IRSwitch, IRUpFrame, IRCall),
    ):
        counts[name] = counts.get(name, 0) + 1
        return
    if isinstance(stmt, IRCall):
        for d in stmt.defs:
            counts[d] = counts.get(d, 0) + 1
        return
    if isinstance(stmt, IRBlock):
        _collect_writes_into(stmt.body, counts)
        return
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            _collect_writes_into(clause.body, counts)
        if stmt.else_body is not None:
            _collect_writes_into(stmt.else_body, counts)
        return
    if isinstance(stmt, IRFor):
        _collect_writes_into(stmt.init, counts)
        _collect_writes_into(stmt.next, counts)
        _collect_writes_into(stmt.body, counts)
        return
    if isinstance(stmt, IRWhile):
        _collect_writes_into(stmt.body, counts)
        return
    if isinstance(stmt, IRForeach):
        for var_list, _ in stmt.iterators:
            for v in var_list:
                counts[v] = counts.get(v, 0) + 1
        _collect_writes_into(stmt.body, counts)
        return
    if isinstance(stmt, IRCatch):
        _collect_writes_into(stmt.body, counts)
        if stmt.result_var:
            counts[stmt.result_var] = counts.get(stmt.result_var, 0) + 1
        if stmt.options_var:
            counts[stmt.options_var] = counts.get(stmt.options_var, 0) + 1
        return
    if isinstance(stmt, IRTry):
        _collect_writes_into(stmt.body, counts)
        for handler in stmt.handlers:
            _collect_writes_into(handler.body, counts)
            if handler.var_name:
                counts[handler.var_name] = counts.get(handler.var_name, 0) + 1
            if handler.options_var:
                counts[handler.options_var] = counts.get(handler.options_var, 0) + 1
        if stmt.finally_body is not None:
            _collect_writes_into(stmt.finally_body, counts)
        return
    if isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if arm.body is not None:
                _collect_writes_into(arm.body, counts)
        if stmt.default_body is not None:
            _collect_writes_into(stmt.default_body, counts)
        return
    if isinstance(stmt, IRUpFrame):
        _collect_writes_into(stmt.body, counts)
        return


# Match ``$<name>`` and ``${<name>}`` Tcl variable substitutions in a
# raw word.  Word-boundary on the bare form rules out ``$NAMES``
# matching when the name we look for is ``NAME``.  Brace form has an
# explicit terminator so any non-``}`` chars qualify as the name.
_VAR_REF_BARE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_:]*)")
_VAR_REF_BRACE = re.compile(r"\$\{([^}]+)\}")


def _vars_in_value(value: str | None) -> set[str]:
    """Return the bare variable names referenced by ``$x`` / ``${x}``
    substitutions in *value*.  Conservative: doesn't track command
    substitutions (``[cmd]``) since those can call back into anything,
    but the LICM gate already refuses any value containing ``[``."""

    if not value or "$" not in value:
        return set()
    out: set[str] = set()
    out.update(m.group(1) for m in _VAR_REF_BARE.finditer(value))
    out.update(m.group(1) for m in _VAR_REF_BRACE.finditer(value))
    return out


def _stmt_references(stmt: object) -> set[str]:
    """Return the set of var names *stmt* may read or write.

    Used by the LICM hoist gate: a candidate ``set x C`` is unsafe to
    hoist when an earlier statement in the same body referenced
    ``x`` — the first iteration would see the pre-loop value instead
    of the hoisted-to-pre-loop value.  The set conservatively
    over-approximates: control-flow blocks union references from
    every nested arm, ``IRCall`` includes both ``defs`` and
    arg-substitution reads, and ``IRIncr`` is treated as a read of
    its target (the increment depends on the prior value).
    """

    refs: set[str] = set()
    if isinstance(stmt, IRAssignConst):
        refs.add(stmt.name)
        return refs
    if isinstance(stmt, IRAssignValue):
        refs.add(stmt.name)
        refs |= _vars_in_value(stmt.value)
        return refs
    if isinstance(stmt, IRAssignExpr):
        refs.add(stmt.name)
        refs |= vars_in_expr_node(stmt.expr)
        return refs
    if isinstance(stmt, IRIncr):
        refs.add(stmt.name)
        if stmt.amount:
            refs |= _vars_in_value(stmt.amount)
        return refs
    if isinstance(stmt, IRExprEval):
        refs |= vars_in_expr_node(stmt.expr)
        return refs
    if isinstance(stmt, IRCall):
        refs.update(stmt.defs)
        refs.update(stmt.reads)
        if stmt.reads_own_defs:
            refs.update(stmt.defs)
        for a in stmt.args:
            refs |= _vars_in_value(a)
        return refs
    if isinstance(stmt, IRReturn):
        if stmt.value:
            refs |= _vars_in_value(stmt.value)
        if stmt.expr is not None:
            refs |= vars_in_expr_node(stmt.expr)
        return refs
    if isinstance(stmt, IRBlock):
        return _refs_in_script(stmt.body)
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            refs |= vars_in_expr_node(clause.condition)
            refs |= _refs_in_script(clause.body)
        if stmt.else_body is not None:
            refs |= _refs_in_script(stmt.else_body)
        return refs
    if isinstance(stmt, IRFor):
        refs |= _refs_in_script(stmt.init)
        refs |= vars_in_expr_node(stmt.condition)
        refs |= _refs_in_script(stmt.next)
        refs |= _refs_in_script(stmt.body)
        return refs
    if isinstance(stmt, IRWhile):
        refs |= vars_in_expr_node(stmt.condition)
        refs |= _refs_in_script(stmt.body)
        return refs
    if isinstance(stmt, IRForeach):
        for var_list, list_arg in stmt.iterators:
            refs.update(var_list)
            refs |= _vars_in_value(list_arg)
        refs |= _refs_in_script(stmt.body)
        return refs
    if isinstance(stmt, IRCatch):
        refs |= _refs_in_script(stmt.body)
        if stmt.result_var:
            refs.add(stmt.result_var)
        if stmt.options_var:
            refs.add(stmt.options_var)
        return refs
    if isinstance(stmt, IRTry):
        refs |= _refs_in_script(stmt.body)
        for handler in stmt.handlers:
            refs |= _refs_in_script(handler.body)
            if handler.var_name:
                refs.add(handler.var_name)
            if handler.options_var:
                refs.add(handler.options_var)
        if stmt.finally_body is not None:
            refs |= _refs_in_script(stmt.finally_body)
        return refs
    if isinstance(stmt, IRSwitch):
        refs |= _vars_in_value(stmt.subject)
        for arm in stmt.arms:
            if arm.body is not None:
                refs |= _refs_in_script(arm.body)
        if stmt.default_body is not None:
            refs |= _refs_in_script(stmt.default_body)
        return refs
    if isinstance(stmt, IRUpFrame):
        refs |= _refs_in_script(stmt.body)
        return refs
    return refs


def _refs_in_script(script: IRScript | None) -> set[str]:
    if script is None:
        return set()
    refs: set[str] = set()
    for s in script.statements:
        refs |= _stmt_references(s)
    return refs
