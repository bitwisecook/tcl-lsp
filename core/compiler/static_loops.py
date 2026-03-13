"""Conservative static evaluation helpers for simple Tcl for-loops.

This module intentionally supports a narrow, side-effect-free subset so callers
can safely infer constants without changing semantics.
"""

from __future__ import annotations

import logging
import math
import re

from ..common.naming import normalise_var_name as _normalise_var_name
from .eval_helpers import DECIMAL_INT_RE as _DECIMAL_INT_RE
from .expr_ast import ExprNode, expr_text
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRFor,
    IRIf,
    IRIncr,
    IRReturn,
    IRScript,
    IRSwitch,
)
from .lowering import lower_to_ir
from .tcl_expr_eval import eval_tcl_expr_str

log = logging.getLogger(__name__)

_SIMPLE_VAR_WORD_RE = re.compile(r"\$(?:\{[A-Za-z_][A-Za-z0-9_:]*\}|[A-Za-z_][A-Za-z0-9_:]*)\Z")
_DEFAULT_MAX_STATIC_LOOP_ITERS = 4096


def evaluate_expr_with_constants(
    expr: str,
    constants: dict[str, int | float | bool | str],
) -> int | bool | None:
    result = eval_tcl_expr_str(expr, constants)
    if result is None:
        return None
    if isinstance(result, float):
        if not math.isfinite(result):
            return None
        if result == int(result):
            return int(result)
        return None
    return result


def _parse_literal_value(text: str) -> int | bool | str:
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        try:
            return int(stripped)
        except ValueError:
            pass
    lowered = stripped.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    return stripped


def _simple_var_ref(text: str) -> str | None:
    stripped = text.strip()
    if not _SIMPLE_VAR_WORD_RE.fullmatch(stripped):
        return None
    return _normalise_var_name(stripped)


def _strip_word_delimiters(text: str) -> str:
    stripped = text.strip()
    if (stripped.startswith('"') and stripped.endswith('"')) or (
        stripped.startswith("{") and stripped.endswith("}")
    ):
        return stripped[1:-1]
    return stripped


def _resolve_switch_subject(
    text: str,
    env: dict[str, int | float | bool | str],
) -> str | None:
    stripped = text.strip()
    if "$" in stripped or "[" in stripped:
        var_ref = _simple_var_ref(stripped)
        if var_ref is None:
            return None
        value = env.get(var_ref)
        if value is None:
            return None
        return str(value)
    return _strip_word_delimiters(stripped)


def _resolve_switch_pattern(pattern: str) -> str:
    return _strip_word_delimiters(pattern)


def _lower_script_if_simple(script_text: str) -> IRScript | None:
    try:
        module = lower_to_ir(script_text)
    except Exception:
        log.debug("static_loops: failed to lower script to IR", exc_info=True)
        return None
    if module.procedures:
        return None
    return module.top_level


def _exec_statement(
    stmt,
    env: dict[str, int | float | bool | str],
) -> bool:
    match stmt:
        case IRAssignConst(name=name, value=value):
            env[name] = _parse_literal_value(value)
            return True

        case IRAssignExpr(name=name, expr=expr):
            result = evaluate_expr_with_constants(expr_text(expr), env)
            if result is None:
                return False
            env[name] = int(result) if isinstance(result, bool) else result
            return True

        case IRAssignValue(name=name, value=value):
            if "[" in value:
                return False
            var_ref = _simple_var_ref(value)
            if var_ref is not None:
                if var_ref not in env:
                    return False
                env[name] = env[var_ref]
                return True
            env[name] = _parse_literal_value(value)
            return True

        case IRIncr(name=name, amount=amount_raw):
            base = env.get(name)
            if not isinstance(base, int):
                return False
            if amount_raw is None:
                amount = 1
            else:
                amount_text = amount_raw.strip()
                parsed = _parse_literal_value(amount_text)
                if isinstance(parsed, bool):
                    amount = int(parsed)
                elif isinstance(parsed, int):
                    amount = parsed
                else:
                    var_ref = _simple_var_ref(amount_text)
                    if var_ref is None:
                        return False
                    value = env.get(var_ref)
                    if not isinstance(value, int):
                        return False
                    amount = value
            env[name] = base + amount
            return True

        case IRIf(clauses=clauses, else_body=else_body):
            for clause in clauses:
                cond = evaluate_expr_with_constants(expr_text(clause.condition), env)
                if cond is None:
                    return False
                cond_bool = cond if isinstance(cond, bool) else (cond != 0)
                if cond_bool:
                    return _exec_script(clause.body, env)
            if else_body is None:
                return True
            return _exec_script(else_body, env)

        case IRSwitch(subject=subject, arms=arms, default_body=default_body):
            subject_value = _resolve_switch_subject(subject, env)
            if subject_value is None:
                return False

            selected_body: IRScript | None = None
            pending_fallthrough = False
            for arm in arms:
                pattern = _resolve_switch_pattern(arm.pattern)
                matches = pattern == subject_value
                if not (matches or pending_fallthrough):
                    continue
                if arm.body is not None:
                    selected_body = arm.body
                    break
                # Fallthrough arm ("-") continues into the next body arm.
                pending_fallthrough = True

            if selected_body is None:
                selected_body = default_body
            if selected_body is None:
                return True
            return _exec_script(selected_body, env)

        case IRBarrier() | IRCall() | IRReturn():
            return False

        case _:
            # Structured statements not explicitly handled remain unsupported.
            return False


def _exec_script(script: IRScript, env: dict[str, int | float | bool | str]) -> bool:
    for stmt in script.statements:
        if not _exec_statement(stmt, env):
            return False
    return True


def summarise_static_for_loop(
    args: list[str] | tuple[str, ...],
    *,
    initial_constants: dict[str, int | float | bool | str] | None = None,
    max_iterations: int = _DEFAULT_MAX_STATIC_LOOP_ITERS,
) -> dict[str, int | float | bool | str] | None:
    """Return post-loop constants for simple static `for` loops.

    Expected args shape: [init_script, test_expr, next_script, body_script].
    Returns None if the loop is not in the supported safe subset.
    """

    if len(args) < 4:
        return None

    init_script = _lower_script_if_simple(args[0])
    next_script = _lower_script_if_simple(args[2])
    body_script = _lower_script_if_simple(args[3])
    if init_script is None or next_script is None or body_script is None:
        return None

    env: dict[str, int | float | bool | str] = {}
    if initial_constants is not None:
        env.update(initial_constants)

    return _summarise_static_for_components(
        init=init_script,
        condition=args[1],
        next_script=next_script,
        body=body_script,
        env=env,
        max_iterations=max_iterations,
    )


def summarise_static_for_ir(
    stmt: IRFor,
    *,
    initial_constants: dict[str, int | float | bool | str] | None = None,
    max_iterations: int = _DEFAULT_MAX_STATIC_LOOP_ITERS,
) -> dict[str, int | float | bool | str] | None:
    env: dict[str, int | float | bool | str] = {}
    if initial_constants is not None:
        env.update(initial_constants)
    return _summarise_static_for_components(
        init=stmt.init,
        condition=stmt.condition,
        next_script=stmt.next,
        body=stmt.body,
        env=env,
        max_iterations=max_iterations,
    )


def _summarise_static_for_components(
    *,
    init: IRScript,
    condition: str | ExprNode,
    next_script: IRScript,
    body: IRScript,
    env: dict[str, int | float | bool | str],
    max_iterations: int,
) -> dict[str, int | float | bool | str] | None:
    if not _exec_script(init, env):
        return None

    cond_str = expr_text(condition) if isinstance(condition, ExprNode) else condition

    iterations = 0
    while True:
        cond = evaluate_expr_with_constants(cond_str, env)
        if cond is None:
            return None
        cond_bool = cond if isinstance(cond, bool) else (cond != 0)
        if not cond_bool:
            break

        iterations += 1
        if iterations > max_iterations:
            return None

        if not _exec_script(body, env):
            return None
        if not _exec_script(next_script, env):
            return None

    return env
