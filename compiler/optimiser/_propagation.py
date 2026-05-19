"""Statement-loop propagation passes for the optimiser."""

from __future__ import annotations

import re

from compiler.parsing.expr_lexer import ExprTokenType, tokenise_expr
from compiler.parsing.tokens import Token, TokenType
from core.commands.registry import REGISTRY
from core.commands.registry.runtime import ArgRole, arg_indices_for_role
from shared.codes import opt
from shared.dialect import active_dialect
from shared.naming import (
    normalise_var_name as _normalise_var_name,
)
from shared.ranges import range_from_token

from ..interprocedural import (
    evaluate_proc_with_constants,
    fold_static_proc_call,
)
from ..token_helpers import parse_decimal_int as _parse_decimal_int
from ..types import TypeLattice
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
    _braced_token_range,
    _braced_token_range_from_range,
    _command_subst_range,
    _constants_from_exit_versions,
    _expr_arg_from_expr_command,
    _literal_from_constant_str,
    _parse_command_words,
    _parse_single_command_from_range,
    _render_folded_literal,
    _resolve_summary_proc_name,
)
from ._types import Optimisation, PassContext

# O-code registrations for codes primarily emitted from this module
opt(
    code="O102",
    description="Fold constant `[expr {...}]` command substitutions.",
    opt_category="constant_folding",
)
opt(
    code="O103",
    description="Fold static procedure calls using interprocedural summaries.",
    opt_category="constant_folding",
)
opt(
    code="O105",
    description=(
        "Propagate constants into variable references and detect redundant computations (GVN/CSE)."
    ),
    opt_category="constant_folding",
)
opt(
    code="O110",
    description="Canonicalise expressions (InstCombine).",
    opt_category="constant_folding",
)
opt(
    code="O111",
    description="Brace expression performance hints (paired with W100).",
    opt_category="readability",
)
opt(
    code="O113",
    description="Strength-reduce expressions (`x**2` → `x*x`, `x%8` → `x&7`).",
    opt_category="constant_folding",
)
opt(
    code="O116",
    description="Fold constant `[list a b c]` to literal value.",
    opt_category="constant_folding",
)
opt(
    code="O117",
    description='Simplify `[string length $s] == 0` → `$s eq ""`.',
    opt_category="readability",
)
opt(
    code="O118",
    description="Fold constant `[lindex {a b c} 1]` to element.",
    opt_category="constant_folding",
)
opt(
    code="O120",
    description="Prefer `eq`/`ne` over `==`/`!=` for string comparisons.",
    opt_category="readability",
)


def optimise_expression_args(
    ctx: PassContext,
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    arg_single: list[bool],
    constants: dict[str, str],
    *,
    namespace: str = "::",
    ssa_uses: dict[str, int] | None = None,
    types: dict[tuple[str, int], TypeLattice] | None = None,
) -> None:
    for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.EXPR)):
        if idx >= len(args) or idx >= len(arg_tokens) or idx >= len(arg_single):
            continue
        if not arg_single[idx]:
            continue

        expr_tok = arg_tokens[idx]
        if expr_tok.type not in (TokenType.STR, TokenType.ESC):
            continue

        expr_text = args[idx]

        if expr_tok.type is TokenType.STR:
            replacement_prefix = "{"
            replacement_suffix = "}"
            tok_range = _braced_token_range(expr_tok)
        else:
            replacement_prefix = ""
            replacement_suffix = ""
            tok_range = range_from_token(expr_tok)

        # O115: unwrap redundant nested expr -- [expr {E}] in expr ctx -> E
        unwrapped = _try_unwrap_expr_in_expr(expr_text)
        if unwrapped is not None and unwrapped != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O115",
                    message="Remove redundant nested expr",
                    range=tok_range,
                    replacement=f"{replacement_prefix}{unwrapped}{replacement_suffix}",
                )
            )
            continue

        substituted_expr, var_changed, _subst = _substitute_expr_constants(expr_text, constants)
        substituted_expr, proc_changed = _substitute_expr_proc_calls(
            ctx,
            substituted_expr,
            constants,
            namespace=namespace,
        )
        changed = var_changed or proc_changed

        # O115/O113/O117/O120 pre-checks on the original expression text
        sr_detected = _try_strength_reduce_expr(expr_text)[1]
        sl_detected = _try_strlen_simplify_expr(expr_text)[1]
        sc_detected = _try_eq_ne_string_compare_simplify_expr(
            expr_text,
            ssa_uses=ssa_uses,
            types=types,
        )[1]

        compared_expr, compare_changed = _try_eq_ne_string_compare_simplify_expr(
            substituted_expr,
            ssa_uses=ssa_uses,
            types=types,
        )

        is_bool_ctx = REGISTRY.has_boolean_condition(cmd_name) or cmd_name == "elseif"
        combined_expr, combine_changed = _instcombine_expr(
            compared_expr,
            bool_context=is_bool_ctx,
        )
        folded_expr = _try_fold_expr(combined_expr)

        if folded_expr is not None and folded_expr != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O101",
                    message="Fold constant expression",
                    range=tok_range,
                    replacement=f"{replacement_prefix}{folded_expr}{replacement_suffix}",
                )
            )
            continue

        if (compare_changed or combine_changed) and combined_expr != expr_text:
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
                    range=tok_range,
                    replacement=f"{replacement_prefix}{combined_expr}{replacement_suffix}",
                )
            )
            continue

        if changed and substituted_expr != expr_text:
            ctx.optimisations.append(
                Optimisation(
                    code="O100",
                    message="Propagate constant into expression",
                    range=tok_range,
                    replacement=f"{replacement_prefix}{substituted_expr}{replacement_suffix}",
                )
            )


def optimise_expr_substitutions(
    ctx: PassContext,
    arg_tokens: list[Token],
    arg_single: list[bool],
    constants: dict[str, str],
    *,
    ssa_uses: dict[str, int] | None = None,
    types: dict[tuple[str, int], TypeLattice] | None = None,
) -> None:
    from ._helpers import _try_fold_lindex_command, _try_fold_list_command

    for idx, tok in enumerate(arg_tokens):
        if idx >= len(arg_single) or not arg_single[idx]:
            continue
        if tok.type is not TokenType.CMD:
            continue

        # O116: fold [list a b c] -> a b c
        list_result = _try_fold_list_command(tok.text)
        if list_result is not None:
            ctx.optimisations.append(
                Optimisation(
                    code="O116",
                    message="Fold constant list command",
                    range=_command_subst_range(tok),
                    replacement=list_result,
                )
            )
            continue

        # O118: fold [lindex {a b c} 1] -> b
        lindex_result = _try_fold_lindex_command(tok.text)
        if lindex_result is not None:
            ctx.optimisations.append(
                Optimisation(
                    code="O118",
                    message="Fold constant lindex command",
                    range=_command_subst_range(tok),
                    replacement=lindex_result,
                )
            )
            continue

        expr_arg = _expr_arg_from_expr_command(tok.text)
        if expr_arg is None:
            continue

        substituted_expr, changed, _subst = _substitute_expr_constants(expr_arg, constants)
        sc_detected = _try_eq_ne_string_compare_simplify_expr(
            expr_arg,
            ssa_uses=ssa_uses,
            types=types,
        )[1]
        compared_expr, compare_changed = _try_eq_ne_string_compare_simplify_expr(
            substituted_expr,
            ssa_uses=ssa_uses,
            types=types,
        )
        combined_expr, combine_changed = _instcombine_expr(compared_expr)
        folded_expr = _try_fold_expr(combined_expr)

        if folded_expr is not None:
            ctx.optimisations.append(
                Optimisation(
                    code="O102",
                    message="Fold constant expr command substitution",
                    range=_command_subst_range(tok),
                    replacement=folded_expr,
                )
            )
            continue

        if (compare_changed or combine_changed) and combined_expr != expr_arg:
            ctx.optimisations.append(
                Optimisation(
                    code="O120" if sc_detected else "O110",
                    message=(
                        "Use eq/ne for string comparison"
                        if sc_detected
                        else "Canonicalise expr command substitution (InstCombine)"
                    ),
                    range=_command_subst_range(tok),
                    replacement=f"[expr {{{combined_expr}}}]",
                )
            )
            continue

        if changed and substituted_expr != expr_arg:
            ctx.optimisations.append(
                Optimisation(
                    code="O100",
                    message="Propagate constant variable into expression",
                    range=_command_subst_range(tok),
                    replacement=f"[expr {{{substituted_expr}}}]",
                )
            )


def optimise_static_proc_calls(
    ctx: PassContext,
    arg_tokens: list[Token],
    arg_single: list[bool],
    constants: dict[str, str],
    *,
    namespace: str = "::",
) -> None:
    """Fold [procName args...] command substitutions using interprocedural summaries."""
    if not ctx.interproc.procedures:
        return

    for idx, tok in enumerate(arg_tokens):
        if idx >= len(arg_single) or not arg_single[idx]:
            continue
        if tok.type is not TokenType.CMD or not tok.text:
            continue

        parsed = _parse_command_words(tok.text)
        if parsed is None:
            continue
        cmd_texts, cmd_tokens, cmd_single = parsed
        if not cmd_texts:
            continue

        # See through iRules ``call proc_name arg...`` indirection.
        if cmd_texts[0] == "call" and len(cmd_texts) >= 2:
            proc_word = cmd_texts[1]
            arg_start = 2
        else:
            proc_word = cmd_texts[0]
            arg_start = 1

        resolved = _resolve_summary_proc_name(
            proc_word,
            namespace=namespace,
            interproc=ctx.interproc,
        )
        if resolved is None:
            continue

        # Don't fold calls to redefined procedures.
        if ctx.ir_module is not None and resolved in ctx.ir_module.redefined_procedures:
            continue

        static_args: list[int | bool | str] = []
        all_static = True
        for i in range(arg_start, len(cmd_texts)):
            if i >= len(cmd_tokens) or i >= len(cmd_single):
                all_static = False
                break
            literal = _parse_static_call_arg(
                cmd_texts[i],
                cmd_tokens[i],
                single_token=cmd_single[i],
                constants=constants,
            )
            if literal is None:
                all_static = False
                break
            static_args.append(literal)

        if not all_static:
            continue

        folded = fold_static_proc_call(ctx.interproc, resolved, tuple(static_args))
        if folded is None:
            summary = ctx.interproc.procedures.get(resolved)
            proc_entry = ctx.proc_cfgs.get(resolved)
            if summary is not None and proc_entry is not None and summary.pure:
                cfg_func, proc_params = proc_entry
                folded = evaluate_proc_with_constants(cfg_func, proc_params, tuple(static_args))
        if folded is None:
            continue
        replacement = _render_folded_literal(folded)
        if replacement is None:
            continue

        ctx.optimisations.append(
            Optimisation(
                code="O103",
                message="Fold static procedure call from interprocedural summary",
                range=_command_subst_range(tok),
                replacement=replacement,
            )
        )


_UNSAFE_IN_WORD_RE = re.compile(r'[\$\[\]\\"\s{}]')


def _is_braced_var_token(tok: Token) -> bool:
    """Return True when ``tok`` is a ``${name}`` style variable token."""
    if tok.type is not TokenType.VAR:
        return False
    span_len = tok.end.offset - tok.start.offset + 1
    # Lexer VAR token text is the normalised var name (without "$"/braces).
    # Unbraced refs are "$name" (span = len(name) + 1), braced refs are
    # "${name}" with the closing brace excluded from token.end
    # (span = len(name) + 2).
    return span_len > len(tok.text) + 1


def _var_token_range(tok: Token):
    """Return the replacement range for a variable token."""
    token_range = range_from_token(tok)
    if _is_braced_var_token(tok):
        return _braced_token_range_from_range(token_range)
    return token_range


def optimise_constant_var_refs(
    ctx: PassContext,
    arg_tokens: list[Token],
    arg_single: list[bool],
    constants: dict[str, str],
) -> None:
    """Replace single-token $var references with known constant values (O100)."""
    for idx, tok in enumerate(arg_tokens):
        if idx >= len(arg_single) or not arg_single[idx]:
            continue
        if tok.type is not TokenType.VAR:
            continue
        name = _normalise_var_name(tok.text)
        value = constants.get(name)
        if value is None:
            continue
        # String constants with Tcl metacharacters ($, [, \, etc.) would
        # change the interpretation if substituted as a bare word.
        if _UNSAFE_IN_WORD_RE.search(value):
            continue
        ctx.optimisations.append(
            Optimisation(
                code="O100",
                message="Propagate constant into command argument",
                range=_var_token_range(tok),
                replacement=value,
            )
        )


# Characters that would introduce new Tcl substitutions inside a double-quoted
# string and therefore must not appear in inlined constant values.
_UNSAFE_IN_STRING_RE = re.compile(r'[\$\[\]\\"]')


def optimise_string_interpolation_var_refs(
    ctx: PassContext,
    arg_tokens: list[Token],
    arg_single: list[bool],
    constants: dict[str, str],
    all_tokens: tuple[Token, ...],
) -> None:
    """Replace $var inside multi-token string arguments with constants (O105).

    This extends ``optimise_constant_var_refs`` to handle the common pattern
    of variable references embedded in double-quoted strings, e.g.::

        set x 42
        puts "value is $x"   ;# -> puts "value is 42"
    """
    if not all_tokens or not constants:
        return

    # Build a set of offsets for VAR tokens that are single-token words
    # (already handled by optimise_constant_var_refs).
    single_var_offsets: set[int] = set()
    for idx, tok in enumerate(arg_tokens):
        if idx < len(arg_single) and arg_single[idx] and tok.type is TokenType.VAR:
            single_var_offsets.add(tok.start.offset)

    # Also collect the command-name token offset so we don't touch it.
    # arg_tokens is argv[1:], but the first token of the command (argv[0])
    # can appear in all_tokens.  We skip it by only looking at VAR tokens
    # that are NOT single-token words and not the command name itself.

    for atk in all_tokens:
        if atk.type is not TokenType.VAR:
            continue
        if atk.start.offset in single_var_offsets:
            continue
        name = _normalise_var_name(atk.text)
        value = constants.get(name)
        if value is None:
            continue
        # Safety: don't inline values containing Tcl special characters that
        # would introduce new substitutions inside the surrounding string.
        if _UNSAFE_IN_STRING_RE.search(value):
            continue
        ctx.optimisations.append(
            Optimisation(
                code="O105",
                message="Propagate constant into string interpolation",
                range=_var_token_range(atk),
                replacement=value,
            )
        )


def optimise_return_terminator(
    ctx: PassContext, source: str, block, ssa_block, analysis, *, namespace: str = "::"
) -> None:
    """Check return values for foldable command substitutions."""
    from ..cfg import CFGReturn

    term = block.terminator
    if not isinstance(term, CFGReturn) or not term.value or not term.range:
        return

    # Build constants from exit versions of this block.
    constants = _constants_from_exit_versions(ssa_block.exit_versions, analysis.values)

    # Parse the return command text to find command substitution tokens.
    ret_range = term.range
    start = ret_range.start.offset
    end = ret_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return

    parsed = _parse_single_command_from_range(source, ret_range)
    if parsed is None:
        return
    argv_texts, argv_tokens, argv_single = parsed
    if len(argv_texts) < 2:
        return

    arg_tokens_slice = argv_tokens[1:]
    arg_single_slice = argv_single[1:]
    optimise_expr_substitutions(ctx, arg_tokens_slice, arg_single_slice, constants)
    optimise_static_proc_calls(
        ctx, arg_tokens_slice, arg_single_slice, constants, namespace=namespace
    )
    optimise_constant_var_refs(ctx, arg_tokens_slice, arg_single_slice, constants)


def _substitute_expr_proc_calls(
    ctx: PassContext,
    expr: str,
    constants: dict[str, str],
    *,
    namespace: str = "::",
) -> tuple[str, bool]:
    """Replace [procName ...] inside an expr with folded constants."""
    if not ctx.interproc.procedures:
        return expr, False

    pieces: list[str] = []
    cursor = 0
    changed = False

    for tok in tokenise_expr(expr, dialect=active_dialect()):
        if tok.start > cursor:
            pieces.append(expr[cursor : tok.start])

        if tok.type is ExprTokenType.COMMAND and len(tok.text) >= 2:
            cmd_text = tok.text[1:-1]  # strip [ and ]
            folded_text = _try_fold_proc_call_in_expr(
                ctx,
                cmd_text,
                constants,
                namespace=namespace,
            )
            if folded_text is not None:
                pieces.append(folded_text)
                changed = True
            else:
                pieces.append(tok.text)
        else:
            pieces.append(tok.text)

        cursor = tok.end + 1

    if cursor < len(expr):
        pieces.append(expr[cursor:])

    return "".join(pieces), changed


def _try_fold_proc_call_in_expr(
    ctx: PassContext,
    cmd_text: str,
    constants: dict[str, str],
    *,
    namespace: str = "::",
) -> str | None:
    """Try to fold a [procName ...] inside an expr to a numeric literal."""
    parsed = _parse_command_words(cmd_text)
    if parsed is None:
        return None
    cmd_texts, cmd_tokens, cmd_single = parsed
    if not cmd_texts:
        return None

    # See through iRules ``call proc_name arg...`` indirection.
    if cmd_texts[0] == "call" and len(cmd_texts) >= 2:
        proc_word = cmd_texts[1]
        arg_start = 2
    else:
        proc_word = cmd_texts[0]
        arg_start = 1

    resolved = _resolve_summary_proc_name(
        proc_word,
        namespace=namespace,
        interproc=ctx.interproc,
    )
    if resolved is None:
        return None

    static_args: list[int | bool | str] = []
    for i in range(arg_start, len(cmd_texts)):
        if i >= len(cmd_tokens) or i >= len(cmd_single):
            return None
        literal = _parse_static_call_arg(
            cmd_texts[i],
            cmd_tokens[i],
            single_token=cmd_single[i],
            constants=constants,
        )
        if literal is None:
            return None
        static_args.append(literal)

    # Don't fold calls to redefined procedures.
    if ctx.ir_module is not None and resolved in ctx.ir_module.redefined_procedures:
        return None

    folded = fold_static_proc_call(ctx.interproc, resolved, tuple(static_args))
    if folded is None:
        summary = ctx.interproc.procedures.get(resolved)
        proc_entry = ctx.proc_cfgs.get(resolved)
        if summary is not None and proc_entry is not None and summary.pure:
            cfg_func, proc_params = proc_entry
            folded = evaluate_proc_with_constants(
                cfg_func,
                proc_params,
                tuple(static_args),
            )
    if folded is None:
        return None

    # Only substitute numeric values into expr context.
    if isinstance(folded, bool):
        return "1" if folded else "0"
    if isinstance(folded, int):
        return str(folded)
    if isinstance(folded, str):
        p = _parse_decimal_int(folded)
        if p is not None:
            return str(p)
    return None


def _parse_static_call_arg(
    arg_text: str,
    arg_token: Token,
    *,
    single_token: bool,
    constants: dict[str, str],
) -> int | bool | str | None:
    """Parse a literal or SCCP-constant argument for static proc folding."""
    if not single_token:
        return None
    if arg_token.type in (TokenType.ESC, TokenType.STR):
        return _literal_from_constant_str(arg_text)
    if arg_token.type is TokenType.VAR:
        var_name = _normalise_var_name(arg_token.text)
        value = constants.get(var_name)
        if value is None:
            return None
        return _literal_from_constant_str(value)
    return None


# ---------------------------------------------------------------------------
# O127: Inline single-use variable assignment (store-to-load forwarding)
# ---------------------------------------------------------------------------

opt(
    code="O127",
    description=(
        "Inline single-use variable assignment — eliminate redundant variable load by "
        "folding `set` into the use site."
    ),
    opt_category="code_motion",
)


def optimise_load_forwarding(
    ctx: PassContext,
    source: str,
    cfg,
    ssa,
    analysis,
    *,
    is_top_level: bool = False,
) -> None:
    """Detect single-use variables and suggest inlining the assignment at the use site.

    When ``set x [cmd]`` is followed by a single use of ``$x``, the variable
    load is redundant — the value was just computed.  This pass suggests
    rewriting to ``[set x [cmd]]`` at the use site (preserving the assignment)
    and deleting the standalone ``set`` statement.

    Inspired by ZJIT's store-to-load forwarding: instead of storing to a
    variable and immediately loading it back, forward the value directly.

    Uses the following SSA/SCCP infrastructure:

    - **Def-use chains** (``analysis.def_use_chains``): precise single-use
      detection — only fires when a definition has exactly one ``OPERAND``
      use and no phi/terminator uses.
    - **SCCP lattice** (``analysis.values``): skip constants already handled
      by O100 — only forward OVERDEFINED (non-constant) values.
    - **Memory-SSA aliases** (``analysis.memory_ssa``): skip variables
      involved in aliasing (upvar/global/variable) where writes through
      one name could be visible through another.
    - **Side-effect classification** (``classify_side_effects``): use the
      registry-backed effect model to determine intervening barriers,
      replacing a hardcoded command list.
    - **SSA versions** (``ssa_stmt.uses``/``defs``): verify that all
      variables read by the inlined expression have unchanged SSA versions
      between the def site and the use site.
    """
    from ..core_analyses import LatticeKind
    from ..def_use import UseKind
    from ..ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRBarrier, IRCall
    from ..side_effects import classify_side_effects
    from ._helpers import _full_command_range, _tokens_for_statement
    from ._pattern_recognition import _statement_delete_rewrite_range, _statement_rewrite_context

    if is_top_level:
        return

    # Require def-use chains for precise single-use detection.
    du = analysis.def_use_chains
    if du is None:
        return

    # Alias information from memory-SSA (pre-computed during analysis).
    aliased_names: frozenset[str] = frozenset()
    if analysis.memory_ssa is not None:
        aliased_names = analysis.memory_ssa.aliased_names
    else:
        # Fallback: compute aliases directly.
        from ..memory_ssa import compute_aliases

        for aset in compute_aliases(ssa):
            aliased_names = aliased_names | aset.names

    executable_blocks = set(cfg.blocks) - set(analysis.unreachable_blocks)

    # Hoist trace-aware classification context out of the per-statement
    # loop: ``traced_commands()`` walks ``IRModule.command_traces`` on
    # every call, and the answer is invariant across the def-use chain
    # walk below.
    if ctx.ir_module is not None:
        traced_commands = ctx.ir_module.traced_commands()
        has_dynamic_trace = ctx.ir_module.has_dynamic_trace()
    else:
        traced_commands = None
        has_dynamic_trace = False

    # Build statement rewrite ranges for deletion.
    range_by_stmt, next_start_by_stmt = _statement_rewrite_context(source, cfg)

    # Build a set of source ranges already targeted by prior optimisation
    # passes (pre-loop pattern recognition + statement-loop propagation).
    # O127 must not emit rewrites that overlap with these.
    rewritten_ranges: set[tuple[int, int]] = set()
    for prior in ctx.optimisations:
        rewritten_ranges.add((prior.range.start.offset, prior.range.end.offset))

    # Walk def-use chains looking for single-use, same-block forwarding candidates.
    for def_key, chain in du.chains.items():
        def_name, def_ver = def_key

        # Must have exactly one use, and it must be a statement operand
        # (not a phi incoming edge or terminator condition).
        if chain.use_count != 1:
            continue
        use = chain.uses[0]
        if use.kind is not UseKind.OPERAND:
            continue

        # Def must be a statement (not a phi or parameter).
        defsite = chain.definition
        if defsite.statement_index < 0:
            continue

        # Def and use must be in the same executable block.
        def_block = defsite.block
        use_block = use.block
        if def_block != use_block:
            continue
        if def_block not in executable_blocks:
            continue

        block = cfg.blocks.get(def_block)
        ssa_block = ssa.blocks.get(def_block)
        if block is None or ssa_block is None:
            continue

        def_idx = defsite.statement_index
        use_idx = use.statement_index
        if def_idx >= len(block.statements) or use_idx >= len(block.statements):
            continue
        if use_idx <= def_idx:
            continue

        stmt = block.statements[def_idx]
        ssa_stmt = ssa_block.statements[def_idx]

        # Only consider pure assignment statements.
        if not isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr)):
            continue

        # Skip constants — SCCP lattice says O100 handles those.
        lv = analysis.values.get(def_key)
        if lv is not None and getattr(lv, "kind", None) is LatticeKind.CONST:
            continue

        # Skip definitions whose statement was already rewritten by
        # SCCP/ICIP propagation (O100/O101/O102/O103).
        if (def_block, def_idx) in ctx.propagated_expr_stmts:
            continue

        # Skip definitions whose use was already consumed by constant
        # propagation into a branch condition.
        if def_key in ctx.propagated_branch_uses:
            continue

        # Skip the use-site statement if it was already rewritten by
        # propagation.
        if (use_block, use_idx) in ctx.propagated_expr_stmts:
            continue

        # Skip def statements already targeted by prior passes (O104,
        # O114, O119, etc.) — their rewrites would conflict with the
        # delete edit.
        def_stmt_range = range_by_stmt.get((def_block, def_idx))
        if def_stmt_range is not None:
            def_start = def_stmt_range.start.offset
            def_end = def_stmt_range.end.offset
            if any(not (e < def_start or s > def_end) for s, e in rewritten_ranges):
                continue

        # Skip cross-event variables.
        if def_name in ctx.cross_event_vars:
            continue

        # Skip aliased variables — both the defined variable and any
        # variable read by the expression.  Memory-SSA alias sets track
        # upvar/global/variable bindings; writes through one alias are
        # visible through another, so forwarding is unsafe.
        if def_name in aliased_names:
            continue
        def_read_names = set(ssa_stmt.uses.keys())
        if def_read_names & aliased_names:
            continue

        # Check intervening statements between def and use:
        # - No barriers (IRBarrier)
        # - No side-effectful commands or assignments with command
        #   substitutions (IRCall, IRAssignValue/IRAssignExpr with [])
        # - No redefinition of any variable read by the expression
        unsafe = False
        for between_idx in range(def_idx + 1, use_idx):
            if between_idx >= len(block.statements):
                break
            between_stmt = block.statements[between_idx]

            # IRBarrier is always a kill.
            if isinstance(between_stmt, IRBarrier):
                unsafe = True
                break

            # Barrier-relaxed eval / uplevel still count as kills —
            # their bodies run with full side-effect potential, just
            # with inline IR rather than a string round-trip.  Treat
            # them conservatively so O127 load-forwarding does not
            # chase through a scope that may mutate arbitrary names.
            from ..ir import IRBlock as _IRBlock
            from ..ir import IRUpFrame as _IRUpFrame

            if isinstance(between_stmt, _IRUpFrame) or (
                isinstance(between_stmt, _IRBlock)
                and between_stmt.source_tokens is not None
                and between_stmt.source_tokens.argv_texts
                and between_stmt.source_tokens.argv_texts[0] == "eval"
            ):
                unsafe = True
                break

            # Assignments with command substitutions execute side effects;
            # forwarding past them could reorder observable behaviour.
            if isinstance(between_stmt, IRAssignValue) and "[" in between_stmt.value:
                unsafe = True
                break
            if isinstance(between_stmt, IRAssignExpr):
                from ._expr_simplify import _expr_has_command_subst

                if _expr_has_command_subst(between_stmt.expr):
                    unsafe = True
                    break

            # Use the side-effect model for command invocations.
            if isinstance(between_stmt, IRCall):
                callee_summary = None
                if ctx.interproc is not None:
                    from ..interprocedural import resolve_call_target

                    known = set(ctx.interproc.procedures)
                    target = resolve_call_target(
                        between_stmt.command,
                        between_stmt.args,
                        cfg.name if hasattr(cfg, "name") else "::top",
                        known,
                    )
                    if target is not None:
                        callee_summary = ctx.interproc.procedures.get(target)
                effect = classify_side_effects(
                    between_stmt.command,
                    between_stmt.args,
                    callee_summary=callee_summary,
                    traced_commands=traced_commands,
                    has_dynamic_trace=has_dynamic_trace,
                )
                if not effect.pure:
                    unsafe = True
                    break

            # SSA version check: if any intervening statement redefines
            # a variable that the expression reads, inlining is unsafe.
            between_ssa = ssa_block.statements[between_idx]
            if def_read_names & set(between_ssa.defs.keys()):
                unsafe = True
                break
        if unsafe:
            continue

        # Extract the full command text from source.
        stmt_range = stmt.range
        full_stmt_range = _full_command_range(source, stmt_range) or stmt_range
        stmt_text = source[full_stmt_range.start.offset : full_stmt_range.end.offset + 1]

        # Build the inline replacement: [set name value_expr]
        inline_text = f"[{stmt_text}]"

        # Find the $var token in the use-site statement and verify
        # intra-statement evaluation order safety.
        use_stmt = block.statements[use_idx]
        parsed = _tokens_for_statement(use_stmt, source)
        if parsed is None:
            continue

        _, use_argv_tokens, use_argv_single = parsed
        var_token = None
        has_earlier_effect = False
        for tidx, tok in enumerate(use_argv_tokens):
            if tok.type is TokenType.VAR and _normalise_var_name(tok.text) == def_name:
                if tidx < len(use_argv_single) and not use_argv_single[tidx]:
                    continue
                var_token = tok
                break
            # Check for command substitutions or variable writes in tokens
            # that appear BEFORE our $var.  These execute left-to-right in
            # Tcl; inlining [set x ...] after them would change evaluation
            # order if the inlined RHS reads state they modify.
            if tok.type is TokenType.CMD:
                has_earlier_effect = True
            elif tok.type is TokenType.VAR:
                # A $var by itself is a read, not a write — safe.
                pass

        if var_token is None:
            continue

        # If there are command substitutions before $var in the use
        # statement, the inlined expression might evaluate in a
        # different context.  Skip unless the def expression has no
        # reads (pure constant-like) — conservative but correct.
        if has_earlier_effect and def_read_names:
            continue

        # Emit grouped rewrite: inline at use site + delete the set line.
        group_id = ctx.alloc_group()

        # 1. Replace $var with [set name value].
        var_range = _var_token_range(var_token)
        ctx.optimisations.append(
            Optimisation(
                code="O127",
                message=f"Inline single-use variable `${def_name}`",
                range=var_range,
                replacement=inline_text,
                group=group_id,
            )
        )

        # 2. Delete the original set statement.
        def_stmt_key = (def_block, def_idx)
        full_range = range_by_stmt.get(def_stmt_key)
        if full_range is not None:
            delete_range = _statement_delete_rewrite_range(
                source,
                full_range,
                next_start_by_stmt.get(def_stmt_key),
            )
            ctx.optimisations.append(
                Optimisation(
                    code="O127",
                    message="Remove inlined assignment",
                    range=delete_range,
                    replacement="",
                    group=group_id,
                )
            )
