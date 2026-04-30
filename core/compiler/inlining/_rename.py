"""S4.2 v3 — α-renaming for inlined IR bodies.

When a callee's body is spliced into a caller's IR, the callee's
parameter and local names must be rewritten so they don't
collide with the caller's locals.  This module walks every
:class:`~core.compiler.ir.IRStatement` shape and produces a
clone with the rename map applied to:

* ``name`` fields on every :class:`IRAssign*` / :class:`IRIncr` /
  :class:`IRCall` ``defs`` / ``reads`` / :class:`IRForeach`
  iterator var lists / :class:`IRCatch` / :class:`IRTry`
  exception bindings;

* every embedded ``$name`` and ``${name}`` substitution in
  string-valued fields (``IRAssignValue.value``,
  ``IRCall.args``, ``IRReturn.value``, ``IRSwitch.subject``,
  etc.);

* every :class:`~core.compiler.expr_ast.ExprVar` node in
  embedded expression ASTs (``IRAssignExpr.expr``,
  :class:`IRIf` / :class:`IRFor` / :class:`IRWhile`
  conditions, :class:`IRReturn.expr`, :class:`IRExprEval.expr`).

The rename map applies only to names listed as keys.  Any name
not in the map (e.g. a global ``::ns::var`` reference, a
namespace variable, a name that's neither parameter nor local
of the callee) passes through verbatim.
"""

from __future__ import annotations

import re
from dataclasses import replace

from ..expr_ast import (
    ExprBinary,
    ExprCall,
    ExprTernary,
    ExprUnary,
    ExprVar,
)
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
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)

# ``$name`` and ``${name}`` substitutions.  Same regex used by
# the DCE read scanner — kept consistent so both passes agree on
# what counts as a variable reference.
_VAR_REF_RE = re.compile(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*(?:\([^)]*\))?)")


def rewrite_script(script: IRScript, rename: dict[str, str]) -> IRScript:
    """Return a new :class:`IRScript` with ``rename`` applied
    everywhere reachable.

    The original script and every nested node are left untouched —
    every IR type is frozen, so the walker rebuilds via
    :func:`dataclasses.replace`.  When the rename map doesn't
    affect a sub-tree the original instance is returned unchanged
    (cheap structural sharing for partial renames).
    """
    if not rename:
        return script
    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        new_stmt = rewrite_stmt(stmt, rename)
        if new_stmt is not stmt:
            changed = True
        new_stmts.append(new_stmt)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _rename_var_name(name: str, rename: dict[str, str]) -> str:
    """Apply ``rename`` to a variable-name field, handling array
    element references.

    For ``arr(idx)`` shapes, the array base is what carries the
    binding identity; the ``(idx)`` suffix is preserved verbatim.
    A bare name routes through the rename map directly.
    """
    paren = name.find("(")
    if paren >= 0:
        base = name[:paren]
        tail = name[paren:]
        renamed_base = rename.get(base, base)
        if renamed_base is base:
            return name
        return renamed_base + tail
    return rename.get(name, name)


def rewrite_stmt(stmt: object, rename: dict[str, str]) -> object:
    """Return a :func:`replace`-cloned ``stmt`` with renames applied,
    or ``stmt`` unchanged if no rename touches it."""

    if isinstance(stmt, IRAssignConst):
        new_name = _rename_var_name(stmt.name, rename)
        if new_name is stmt.name:
            return stmt
        return replace(stmt, name=new_name)

    if isinstance(stmt, IRAssignValue):
        new_name = _rename_var_name(stmt.name, rename)
        new_value = _rewrite_value_string(stmt.value, rename)
        if new_name is stmt.name and new_value is stmt.value:
            return stmt
        return replace(stmt, name=new_name, value=new_value)

    if isinstance(stmt, IRAssignExpr):
        new_name = _rename_var_name(stmt.name, rename)
        new_expr = _rewrite_expr(stmt.expr, rename)
        if new_name is stmt.name and new_expr is stmt.expr:
            return stmt
        return replace(stmt, name=new_name, expr=new_expr)

    if isinstance(stmt, IRIncr):
        new_name = _rename_var_name(stmt.name, rename)
        new_amount = stmt.amount
        if stmt.amount is not None:
            new_amount = _rewrite_value_string(stmt.amount, rename)
        if new_name is stmt.name and new_amount is stmt.amount:
            return stmt
        return replace(stmt, name=new_name, amount=new_amount)

    if isinstance(stmt, IRCall):
        new_args = tuple(_rewrite_value_string(a, rename) for a in stmt.args)
        new_defs = tuple(_rename_var_name(d, rename) for d in stmt.defs)
        new_reads = tuple(_rename_var_name(r, rename) for r in stmt.reads)
        if new_args == stmt.args and new_defs == stmt.defs and new_reads == stmt.reads:
            return stmt
        return replace(stmt, args=new_args, defs=new_defs, reads=new_reads)

    if isinstance(stmt, IRReturn):
        new_value = stmt.value
        if stmt.value is not None:
            new_value = _rewrite_value_string(stmt.value, rename)
        new_expr = stmt.expr
        if stmt.expr is not None:
            new_expr = _rewrite_expr(stmt.expr, rename)
        if new_value is stmt.value and new_expr is stmt.expr:
            return stmt
        return replace(stmt, value=new_value, expr=new_expr)

    if isinstance(stmt, IRExprEval):
        new_expr = _rewrite_expr(stmt.expr, rename)
        if new_expr is stmt.expr:
            return stmt
        return replace(stmt, expr=new_expr)

    if isinstance(stmt, IRBlock):
        new_body = rewrite_script(stmt.body, rename)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)

    if isinstance(stmt, IRIf):
        new_clauses = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_cond = _rewrite_expr(clause.condition, rename)
            new_body = rewrite_script(clause.body, rename)
            if new_cond is clause.condition and new_body is clause.body:
                new_clauses.append(clause)
            else:
                new_clauses.append(replace(clause, condition=new_cond, body=new_body))
                clauses_changed = True
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = rewrite_script(stmt.else_body, rename)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return stmt
        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else)

    if isinstance(stmt, IRFor):
        new_init = rewrite_script(stmt.init, rename)
        new_cond = _rewrite_expr(stmt.condition, rename)
        new_next = rewrite_script(stmt.next, rename)
        new_body = rewrite_script(stmt.body, rename)
        if (
            new_init is stmt.init
            and new_cond is stmt.condition
            and new_next is stmt.next
            and new_body is stmt.body
        ):
            return stmt
        return replace(
            stmt,
            init=new_init,
            condition=new_cond,
            next=new_next,
            body=new_body,
        )

    if isinstance(stmt, IRWhile):
        new_cond = _rewrite_expr(stmt.condition, rename)
        new_body = rewrite_script(stmt.body, rename)
        if new_cond is stmt.condition and new_body is stmt.body:
            return stmt
        return replace(stmt, condition=new_cond, body=new_body)

    if isinstance(stmt, IRForeach):
        new_iters = []
        iters_changed = False
        for var_list, list_arg in stmt.iterators:
            new_vars = tuple(rename.get(v, v) for v in var_list)
            new_list = _rewrite_value_string(list_arg, rename)
            if new_vars == var_list and new_list is list_arg:
                new_iters.append((var_list, list_arg))
            else:
                new_iters.append((new_vars, new_list))
                iters_changed = True
        new_body = rewrite_script(stmt.body, rename)
        if not iters_changed and new_body is stmt.body:
            return stmt
        return replace(stmt, iterators=tuple(new_iters), body=new_body)

    if isinstance(stmt, IRCatch):
        new_body = rewrite_script(stmt.body, rename)
        new_result = stmt.result_var
        if stmt.result_var is not None:
            new_result = rename.get(stmt.result_var, stmt.result_var)
        new_options = stmt.options_var
        if stmt.options_var is not None:
            new_options = rename.get(stmt.options_var, stmt.options_var)
        if (
            new_body is stmt.body
            and new_result is stmt.result_var
            and new_options is stmt.options_var
        ):
            return stmt
        return replace(
            stmt,
            body=new_body,
            result_var=new_result,
            options_var=new_options,
        )

    if isinstance(stmt, IRTry):
        new_body = rewrite_script(stmt.body, rename)
        new_handlers = []
        handlers_changed = False
        for handler in stmt.handlers:
            new_handler_body = rewrite_script(handler.body, rename)
            new_var = handler.var_name
            if handler.var_name is not None:
                new_var = rename.get(handler.var_name, handler.var_name)
            new_options = handler.options_var
            if handler.options_var is not None:
                new_options = rename.get(handler.options_var, handler.options_var)
            if (
                new_handler_body is handler.body
                and new_var is handler.var_name
                and new_options is handler.options_var
            ):
                new_handlers.append(handler)
            else:
                new_handlers.append(
                    replace(
                        handler,
                        body=new_handler_body,
                        var_name=new_var,
                        options_var=new_options,
                    )
                )
                handlers_changed = True
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = rewrite_script(stmt.finally_body, rename)
            finally_changed = new_finally is not stmt.finally_body
        if new_body is stmt.body and not handlers_changed and not finally_changed:
            return stmt
        return replace(
            stmt,
            body=new_body,
            handlers=tuple(new_handlers),
            finally_body=new_finally,
        )

    if isinstance(stmt, IRSwitch):
        new_subject = _rewrite_value_string(stmt.subject, rename)
        new_arms = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            new_arm_body = rewrite_script(arm.body, rename)
            if new_arm_body is arm.body:
                new_arms.append(arm)
            else:
                new_arms.append(replace(arm, body=new_arm_body))
                arms_changed = True
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = rewrite_script(stmt.default_body, rename)
            default_changed = new_default is not stmt.default_body
        if new_subject is stmt.subject and not arms_changed and not default_changed:
            return stmt
        return replace(
            stmt,
            subject=new_subject,
            arms=tuple(new_arms),
            default_body=new_default,
        )

    if isinstance(stmt, IRUpFrame):
        new_body = rewrite_script(stmt.body, rename)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)

    return stmt


def _rewrite_value_string(text: str, rename: dict[str, str]) -> str:
    """Rewrite ``$name`` / ``${name}`` substitutions in ``text``.

    Returns the original string instance unchanged when no
    substitution matches the rename map.  Array element references
    (``$arr(idx)``) rename the array name only — the index
    expression is preserved verbatim, which is correct because
    array indices are evaluated as Tcl values at runtime and the
    rewrite never moves an array binding into the inlined scope
    (we conservatively decline to inline procs that touch arrays).
    """
    if not text or not rename:
        return text

    def repl(m: re.Match[str]) -> str:
        full = m.group(0)
        if m.group(1) is not None:
            # ${name} form
            name = m.group(1)
            paren = name.find("(")
            base = name[:paren] if paren >= 0 else name
            tail = name[paren:] if paren >= 0 else ""
            if base in rename:
                if tail:
                    return f"${{{rename[base]}{tail}}}"
                return f"${{{rename[base]}}}"
            return full
        # $name form
        name = m.group(2)
        paren = name.find("(")
        base = name[:paren] if paren >= 0 else name
        tail = name[paren:] if paren >= 0 else ""
        if base in rename:
            return f"${rename[base]}{tail}"
        return full

    new_text = _VAR_REF_RE.sub(repl, text)
    if new_text == text:
        return text
    return new_text


def _rewrite_expr(node, rename: dict[str, str]):
    """Walk an :class:`~core.compiler.expr_ast.ExprNode` tree and
    return a clone with ``rename`` applied to every ``ExprVar``
    name."""
    if isinstance(node, ExprVar):
        new_name = rename.get(node.name, node.name)
        if new_name is node.name:
            return node
        # ``text`` field carries the source token (``$x`` or
        # ``${x}``).  Update it textually so downstream emitters
        # that fall back on text get a coherent value.
        new_text = node.text
        if "{" in node.text:
            new_text = node.text.replace("{" + node.name + "}", "{" + new_name + "}")
        else:
            new_text = "$" + new_name
        return replace(node, name=new_name, text=new_text)

    if isinstance(node, ExprBinary):
        new_left = _rewrite_expr(node.left, rename)
        new_right = _rewrite_expr(node.right, rename)
        if new_left is node.left and new_right is node.right:
            return node
        return replace(node, left=new_left, right=new_right)

    if isinstance(node, ExprUnary):
        new_op = _rewrite_expr(node.operand, rename)
        if new_op is node.operand:
            return node
        return replace(node, operand=new_op)

    if isinstance(node, ExprTernary):
        new_c = _rewrite_expr(node.condition, rename)
        new_t = _rewrite_expr(node.true_branch, rename)
        new_f = _rewrite_expr(node.false_branch, rename)
        if new_c is node.condition and new_t is node.true_branch and new_f is node.false_branch:
            return node
        return replace(
            node,
            condition=new_c,
            true_branch=new_t,
            false_branch=new_f,
        )

    if isinstance(node, ExprCall):
        new_args = tuple(_rewrite_expr(a, rename) for a in node.args)
        if all(na is oa for na, oa in zip(new_args, node.args)):
            return node
        return replace(node, args=new_args)

    # ExprLiteral / ExprString / ExprCommand / ExprRaw — no
    # variable references inside (ExprCommand is opaque cmd subst
    # that runs in the caller's frame, which we already gate on
    # at the call-site eligibility check).
    return node
