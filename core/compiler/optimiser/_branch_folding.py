"""Branch folding passes for the optimiser."""

from __future__ import annotations

from ..cfg import CFGBranch
from ..expr_ast import BinOp, ExprBinary, vars_in_expr_node
from ._expr_simplify import (
    _instcombine_expr,
    _substitute_expr_constants,
    _try_eq_ne_string_compare_simplify_expr,
    _try_fold_expr,
    _try_strength_reduce_expr,
    _try_strlen_simplify_expr,
    _try_unwrap_expr_in_expr,
)
from ._helpers import (
    _braced_token_range_from_range,
    _constants_from_exit_versions,
)
from ._propagation import _substitute_expr_proc_calls
from ._types import Optimisation, PassContext


def optimise_branch_proc_calls(
    ctx: PassContext,
    cfg,
    ssa,
    analysis,
    *,
    namespace: str = "::",
) -> None:
    """Fold proc calls inside branch conditions that SCCP could not resolve."""
    source = ctx.source
    constant_blocks = {cb.block for cb in analysis.constant_branches}

    for block_name, block in cfg.blocks.items():
        term = block.terminator
        if not isinstance(term, CFGBranch) or not term.range:
            continue
        if block_name in constant_blocks:
            continue

        cond_range = term.range
        start = cond_range.start.offset
        end = cond_range.end.offset
        if start < 0 or end < start or end >= len(source):
            continue

        cond_text = source[start : end + 1]
        if not cond_text:
            continue

        if cond_text.startswith("{"):
            expr_text = cond_text[1:]
            replacement_prefix = "{"
            replacement_suffix = "}"
            cond_range = _braced_token_range_from_range(cond_range)
        else:
            expr_text = cond_text
            replacement_prefix = ""
            replacement_suffix = ""

        ssa_block = ssa.blocks.get(block_name)
        constants: dict[str, str] = {}
        if ssa_block is not None:
            constants = _constants_from_exit_versions(
                ssa_block.exit_versions,
                analysis.values,
            )

        substituted, var_changed, subst_names = _substitute_expr_constants(expr_text, constants)
        substituted, proc_changed = _substitute_expr_proc_calls(
            ctx,
            substituted,
            constants,
            namespace=namespace,
        )

        # O115: unwrap redundant nested expr in branch condition
        unwrapped = _try_unwrap_expr_in_expr(expr_text)
        if unwrapped is not None and unwrapped != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O115",
                    message="Remove redundant nested expr",
                    range=cond_range,
                    replacement=f"{replacement_prefix}{unwrapped}{replacement_suffix}",
                )
            )
            continue

        # O113/O117/O120 pre-detection on the original expression text.
        sr_detected = _try_strength_reduce_expr(expr_text)[1]
        sl_detected = _try_strlen_simplify_expr(expr_text)[1]
        sc_detected = _try_eq_ne_string_compare_simplify_expr(
            expr_text,
            ssa_uses=ssa_block.exit_versions if ssa_block is not None else None,
            types=analysis.types,
        )[1]

        compared, compare_changed = _try_eq_ne_string_compare_simplify_expr(
            substituted,
            ssa_uses=ssa_block.exit_versions if ssa_block is not None else None,
            types=analysis.types,
        )
        combined, combine_changed = _instcombine_expr(compared, bool_context=True)

        if not (var_changed or proc_changed or compare_changed or combine_changed):
            continue

        # Track propagated branch uses for post-propagation DSE.
        branch_group: int | None = None
        if var_changed and ssa_block is not None:
            branch_group = ctx.alloc_group()
            for name in subst_names:
                ver = ssa_block.exit_versions.get(name, 0)
                if ver > 0:
                    ctx.propagated_branch_uses.add((name, ver))
                    ctx.propagated_use_groups[(name, ver)] = branch_group

        folded = _try_fold_expr(combined)
        if folded is not None and folded != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O101",
                    message="Fold constant expression",
                    range=cond_range,
                    replacement=f"{replacement_prefix}{folded}{replacement_suffix}",
                    group=branch_group,
                )
            )
            continue

        if (compare_changed or combine_changed) and combined != expr_text:
            opt_code = (
                "O113"
                if sr_detected
                else ("O117" if sl_detected else ("O120" if sc_detected else "O110"))
            )
            opt_msg = (
                "Strength-reduce expression"
                if sr_detected
                else (
                    "Simplify string length zero-check"
                    if sl_detected
                    else (
                        "Use eq/ne for string comparison"
                        if sc_detected
                        else "Canonicalise expression (InstCombine)"
                    )
                )
            )
            ctx.optimisations.append(
                Optimisation(
                    code=opt_code,
                    message=opt_msg,
                    range=cond_range,
                    replacement=f"{replacement_prefix}{combined}{replacement_suffix}",
                    group=branch_group,
                )
            )
            continue

        if (var_changed or proc_changed) and substituted != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O100",
                    message="Propagate constants into branch expression",
                    range=cond_range,
                    replacement=f"{replacement_prefix}{substituted}{replacement_suffix}",
                    group=branch_group,
                )
            )


def optimise_constant_branches(ctx: PassContext, cfg, ssa, analysis) -> None:
    """Emit O101 for branch conditions that SCCP proved constant."""
    source = ctx.source
    for cb in analysis.constant_branches:
        block = cfg.blocks.get(cb.block)
        if block is None:
            continue
        term = block.terminator
        if not isinstance(term, CFGBranch) or not term.range:
            continue

        # Skip switch dispatch branches.
        if isinstance(term.condition, ExprBinary) and term.condition.op is BinOp.STR_EQ:
            if "switch_next" in cb.block or any(
                bn.startswith("switch_next") for bn in (cb.taken_target, cb.not_taken_target)
            ):
                continue

        cond_range = term.range
        start = cond_range.start.offset
        end = cond_range.end.offset
        if start < 0 or end < start or end >= len(source):
            continue

        cond_text = source[start : end + 1]
        if not cond_text:
            continue

        if cond_text.startswith("{"):
            replacement_prefix = "{"
            replacement_suffix = "}"
            cond_range = _braced_token_range_from_range(cond_range)
        else:
            replacement_prefix = ""
            replacement_suffix = ""

        # All variable uses in a fully-constant branch are consumed.
        cb_group: int | None = None
        ssa_block = ssa.blocks.get(cb.block)
        if ssa_block is not None:
            use_keys = list(_branch_use_versions(term, ssa_block.exit_versions))
            if use_keys:
                cb_group = ctx.alloc_group()
            for use_key in use_keys:
                ctx.propagated_branch_uses.add(use_key)
                if cb_group is not None:
                    ctx.propagated_use_groups[use_key] = cb_group

        folded = "1" if cb.value else "0"
        ctx.optimisations.append(
            Optimisation(
                code="O101",
                message="Fold constant expression",
                range=cond_range,
                replacement=f"{replacement_prefix}{folded}{replacement_suffix}",
                group=cb_group,
            )
        )


def _branch_use_versions(term, exit_versions: dict[str, int]) -> set[tuple[str, int]]:
    if not isinstance(term, CFGBranch):
        return set()
    uses: set[tuple[str, int]] = set()
    for name in vars_in_expr_node(term.condition):
        ver = exit_versions.get(name, 0)
        if ver > 0:
            uses.add((name, ver))
    return uses
