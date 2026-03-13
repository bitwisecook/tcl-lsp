"""Structure elimination pass (O112) for the optimiser."""

from __future__ import annotations

from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ..core_analyses import LatticeKind, LatticeValue
from ..ir import (
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ._helpers import (
    _extract_body_text,
    _full_command_range,
    _is_plain_literal,
)
from ._types import Optimisation, PassContext


def optimise_structure_elimination(
    ctx: PassContext,
    ir_script: IRScript | None,
    cfg=None,
    ssa=None,
    analysis=None,
) -> None:
    """Eliminate compound statements whose conditions are compile-time constants."""
    if ir_script is None:
        return
    sccp_constants: dict[str, int | float | str] = {}
    if analysis is not None:
        var_values: dict[str, list[LatticeValue]] = {}
        for key, lv in analysis.values.items():
            name = key[0] if isinstance(key, tuple) else key
            var_values.setdefault(name, []).append(lv)
        for name, lvs in var_values.items():
            if all(lv.kind is LatticeKind.CONST and lv.value is not None for lv in lvs):
                unique_vals = {lv.value for lv in lvs}
                if len(unique_vals) == 1:
                    val = next(iter(unique_vals))
                    assert val is not None
                    sccp_constants[name] = val
    _walk_ir_for_structure_elimination(ctx, ir_script, sccp_constants)


def _walk_ir_for_structure_elimination(
    ctx: PassContext,
    ir_script: IRScript,
    sccp_constants: dict[str, int | float | str] | None = None,
) -> None:
    from ..tcl_expr_eval import eval_tcl_expr

    source = ctx.source
    for stmt in ir_script.statements:
        match stmt:
            case IRIf(range=r, clauses=clauses, else_body=else_body, else_range=else_range):
                _try_eliminate_if(ctx, source, r, clauses, else_body, else_range, sccp_constants)
                for clause in clauses:
                    _walk_ir_for_structure_elimination(ctx, clause.body, sccp_constants)
                if else_body is not None:
                    _walk_ir_for_structure_elimination(ctx, else_body, sccp_constants)

            case IRWhile(range=r, condition=cond, body=body):
                val = eval_tcl_expr(cond, sccp_constants)
                if val is not None and not val:
                    full_range = _full_command_range(source, r) or r
                    ctx.optimisations.append(
                        Optimisation(
                            code="O112",
                            message="Eliminate dead while loop (condition is always false)",
                            range=full_range,
                            replacement="",
                        )
                    )
                _walk_ir_for_structure_elimination(ctx, body, sccp_constants)

            case IRFor(
                range=r,
                init=init,
                init_range=init_range,
                condition=cond,
                body=body,
                next=next_script,
            ):
                val = eval_tcl_expr(cond, sccp_constants)
                if val is not None and not val:
                    full_range = _full_command_range(source, r) or r
                    if init.statements:
                        replacement = _extract_body_text(source, init_range, r)
                        ctx.optimisations.append(
                            Optimisation(
                                code="O112",
                                message=(
                                    "Eliminate dead for loop (condition is always false); keep init"
                                ),
                                range=full_range,
                                replacement=replacement,
                            )
                        )
                    else:
                        ctx.optimisations.append(
                            Optimisation(
                                code="O112",
                                message="Eliminate dead for loop (condition is always false)",
                                range=full_range,
                                replacement="",
                            )
                        )
                _walk_ir_for_structure_elimination(ctx, init, sccp_constants)
                _walk_ir_for_structure_elimination(ctx, body, sccp_constants)
                _walk_ir_for_structure_elimination(ctx, next_script, sccp_constants)

            case IRSwitch(
                range=r,
                subject=subject,
                arms=arms,
                default_body=default_body,
                default_range=default_range,
            ):
                _try_eliminate_switch(
                    ctx,
                    source,
                    r,
                    subject,
                    arms,
                    default_body,
                    default_range,
                    sccp_constants=sccp_constants,
                )
                for arm in arms:
                    if arm.body is not None:
                        _walk_ir_for_structure_elimination(ctx, arm.body, sccp_constants)
                if default_body is not None:
                    _walk_ir_for_structure_elimination(ctx, default_body, sccp_constants)

            case IRCatch(body=body):
                _walk_ir_for_structure_elimination(ctx, body, sccp_constants)

            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                _walk_ir_for_structure_elimination(ctx, body, sccp_constants)
                for handler in handlers:
                    _walk_ir_for_structure_elimination(ctx, handler.body, sccp_constants)
                if finally_body is not None:
                    _walk_ir_for_structure_elimination(ctx, finally_body, sccp_constants)

            case IRForeach(body=body):
                _walk_ir_for_structure_elimination(ctx, body, sccp_constants)


def _try_eliminate_if(
    ctx: PassContext,
    source: str,
    stmt_range,
    clauses: tuple,
    else_body: IRScript | None,
    else_range,
    sccp_constants: dict[str, int | float | str] | None = None,
) -> None:
    """Try to eliminate an ``if`` whose clause conditions are all constant."""
    from ..tcl_expr_eval import eval_tcl_expr

    full_range = _full_command_range(source, stmt_range) or stmt_range
    for clause in clauses:
        val = eval_tcl_expr(clause.condition, sccp_constants)
        if val is None:
            return
        if val:
            replacement = _extract_body_text(source, clause.body_range, stmt_range)
            ctx.optimisations.append(
                Optimisation(
                    code="O112",
                    message="Eliminate constant if (condition is always true)",
                    range=full_range,
                    replacement=replacement,
                )
            )
            return
    # All clauses are false.
    if else_body is not None and else_range is not None:
        replacement = _extract_body_text(source, else_range, stmt_range)
        ctx.optimisations.append(
            Optimisation(
                code="O112",
                message="Eliminate constant if (all conditions false); keep else",
                range=full_range,
                replacement=replacement,
            )
        )
    else:
        ctx.optimisations.append(
            Optimisation(
                code="O112",
                message="Eliminate dead if (all conditions are always false)",
                range=full_range,
                replacement="",
            )
        )


def _try_eliminate_switch(
    ctx: PassContext,
    source: str,
    stmt_range,
    subject: str,
    arms: tuple,
    default_body: IRScript | None,
    default_range,
    *,
    sccp_constants: dict[str, int | float | str] | None = None,
) -> None:
    """Try to eliminate a ``switch`` with a literal subject."""
    resolved_subject = subject
    if not _is_plain_literal(resolved_subject) and sccp_constants:
        var = resolved_subject
        if var.startswith("${") and var.endswith("}"):
            var = var[2:-1]
        elif var.startswith("$"):
            var = var[1:]
        else:
            var = None
        if var is not None:
            val = sccp_constants.get(var) or sccp_constants.get(_normalise_var_name(var))
            if val is not None:
                resolved_subject = str(val)
    if not _is_plain_literal(resolved_subject):
        return
    subject = resolved_subject
    full_range = _full_command_range(source, stmt_range) or stmt_range
    for arm in arms:
        if arm.fallthrough:
            continue
        if arm.pattern == subject:
            if arm.body is not None and arm.body_range is not None:
                replacement = _extract_body_text(source, arm.body_range, stmt_range)
                ctx.optimisations.append(
                    Optimisation(
                        code="O112",
                        message=(
                            f"Eliminate switch (subject '{subject}' always matches "
                            f"pattern '{arm.pattern}')"
                        ),
                        range=full_range,
                        replacement=replacement,
                    )
                )
            return
    # No exact match -- try default.
    if default_body is not None and default_range is not None:
        replacement = _extract_body_text(source, default_range, stmt_range)
        ctx.optimisations.append(
            Optimisation(
                code="O112",
                message=f"Eliminate switch (subject '{subject}' matches no pattern); keep default",
                range=full_range,
                replacement=replacement,
            )
        )
    else:
        ctx.optimisations.append(
            Optimisation(
                code="O112",
                message=f"Eliminate dead switch (subject '{subject}' matches no pattern)",
                range=full_range,
                replacement="",
            )
        )
