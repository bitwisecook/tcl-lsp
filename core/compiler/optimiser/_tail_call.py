"""Tail-call and recursion optimisation passes (O121–O123).

O121: Detect self-recursive calls in tail position → suggest ``tailcall``.
O122: Convert fully tail-recursive proc to iterative ``while`` loop.
O123: Detect non-tail recursion eligible for accumulator introduction.

Both O121 and O122 check recursively through ``if``/``switch`` branches
so that self-calls in any tail-position branch are reported.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from shared.codes import opt
from shared.dialect import active_dialect
from shared.naming import normalise_qualified_name as _normalise_qualified_name

from ...analysis.semantic_model import Range
from ...commands.registry.dialects import dialects_since
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..ir import (
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ._helpers import _full_command_range, _parse_command_words, _parse_single_command_from_range
from ._types import Optimisation, PassContext

_CMD_SUBST_RE = re.compile(r"^\[(.+)\]\Z", re.DOTALL)


@dataclass(frozen=True, slots=True)
class _TailCallSite:
    """A self-recursive call detected in tail position."""

    stmt_range: Range
    full_range: Range
    args: tuple[str, ...]
    kind: str  # "return_subst" | "bare_call"


# Entry point


_TAILCALL_DIALECTS = dialects_since("tcl8.6")


def optimise_tail_calls(ctx: PassContext) -> None:
    """Scan all procedures for recursion patterns (O121–O123).

    ``tailcall`` is Tcl 8.6+ (TIP 327), so O121 is gated to 8.6+
    dialects.  In Tcl 8.4 / 8.5 the recursion-to-loop hint (O122)
    is still useful and remains active.
    """
    if ctx.ir_module is None:
        return
    tailcall_supported = active_dialect() in _TAILCALL_DIALECTS
    for qname, proc in ctx.ir_module.procedures.items():
        short_name = qname.rsplit("::", 1)[-1] if "::" in qname else qname
        if not short_name:
            continue
        self_names = _self_name_variants(qname)

        # Collect tail-call sites with their arguments.
        sites = _collect_tail_call_sites(ctx.source, proc.body, self_names)

        # O121: suggest tailcall for each site (Tcl 8.6+ only).
        if tailcall_supported:
            for site in sites:
                _emit_o121(ctx, site, short_name)

        # O122: convert to loop if ALL self-calls are in tail position.
        if sites and proc.params:
            total = _count_all_self_calls(proc.body, self_names, ctx.source)
            # Also count self-calls in conditions/subjects (non-tail by definition).
            total += _count_condition_self_calls(proc.body, self_names)
            if total > 0 and total == len(sites):
                _suggest_loop_conversion(ctx, proc, sites, short_name)

        # O123: detect accumulator-eligible non-tail recursion.
        _detect_accumulator_candidate(ctx, proc, self_names, short_name, sites, ctx.source)


# Name resolution


def _self_name_variants(qname: str) -> frozenset[str]:
    """Return the set of command names that refer to *qname*."""
    names: set[str] = set()
    normalised = _normalise_qualified_name(qname)
    names.add(normalised)
    short = normalised.rsplit("::", 1)[-1]
    if short:
        names.add(short)
    if normalised.startswith("::"):
        names.add(normalised[2:])
    return frozenset(names)


# Tail-call site collection


def _collect_tail_call_sites(
    source: str,
    script: IRScript,
    self_names: frozenset[str],
) -> list[_TailCallSite]:
    """Collect all tail-position self-calls with their arguments."""
    sites: list[_TailCallSite] = []
    if not script.statements:
        return sites
    last = script.statements[-1]
    match last:
        case IRReturn(range=r, value=value) if value is not None:
            args = _extract_return_subst_args(source, r, self_names)
            if args is not None:
                full = _full_command_range(source, r) or r
                sites.append(
                    _TailCallSite(stmt_range=r, full_range=full, args=args, kind="return_subst")
                )

        case IRCall(range=r, command=cmd, args=call_args) if cmd in self_names:
            full = _full_command_range(source, r) or r
            sites.append(
                _TailCallSite(stmt_range=r, full_range=full, args=call_args, kind="bare_call")
            )

        case IRIf(clauses=clauses, else_body=else_body):
            for clause in clauses:
                sites.extend(_collect_tail_call_sites(source, clause.body, self_names))
            if else_body is not None:
                sites.extend(_collect_tail_call_sites(source, else_body, self_names))

        case IRSwitch(arms=arms, default_body=default_body):
            for arm in arms:
                if arm.body is not None:
                    sites.extend(_collect_tail_call_sites(source, arm.body, self_names))
            if default_body is not None:
                sites.extend(_collect_tail_call_sites(source, default_body, self_names))
    return sites


def _extract_return_subst_args(
    source: str,
    stmt_range: Range,
    self_names: frozenset[str],
) -> tuple[str, ...] | None:
    """Extract args from ``return [procName arg1 ...]`` in source text."""
    inner = _return_command_subst_text(source, stmt_range)
    if inner is None:
        return None
    parsed = _parse_command_words(inner)
    if parsed is None:
        return None
    argv_texts, _, _ = parsed
    if not argv_texts or argv_texts[0] not in self_names:
        return None
    return tuple(argv_texts[1:])


def _return_command_subst_text(source: str, stmt_range: Range) -> str | None:
    """Return inner text for ``return [ ... ]`` when the arg is a CMD token.

    Braced/quoted forms are intentionally excluded because lexical shape matters
    for substitution semantics.
    """
    parsed = _parse_single_command_from_range(source, stmt_range)
    if parsed is None:
        return None
    argv_texts, argv_tokens, argv_single = parsed
    if len(argv_texts) != 2 or argv_texts[0] != "return":
        return None
    if not argv_single[1] or argv_tokens[1].type is not TokenType.CMD:
        return None
    return argv_texts[1].strip()


# Self-call counting (all positions, not just tail)


def _count_all_self_calls(
    script: IRScript,
    self_names: frozenset[str],
    source: str,
) -> int:
    """Count every self-call occurrence in *script* (tail and non-tail)."""
    count = 0
    for stmt in script.statements:
        count += _count_self_calls_in_stmt(stmt, self_names, source)
    return count


def _count_self_calls_in_stmt(stmt, self_names: frozenset[str], source: str) -> int:
    """Count self-call occurrences in a single IR statement."""
    count = 0

    # Command-level statements: count only executable substitutions from source.
    if isinstance(stmt, (IRCall, IRReturn, IRAssignValue, IRBarrier)):
        stmt_range = getattr(stmt, "range", None)
        if stmt_range is not None:
            count += _count_self_calls_in_command_range(source, stmt_range, self_names)

    # Expression evaluation (may embed command substitutions).
    elif isinstance(stmt, IRExprEval):
        expr_text = getattr(stmt.expr, "text", "") or ""
        count += _count_name_as_command(expr_text, self_names)

    # Recurse into control structures.
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            count += _count_all_self_calls(clause.body, self_names, source)
        if stmt.else_body is not None:
            count += _count_all_self_calls(stmt.else_body, self_names, source)
    elif isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if arm.body is not None:
                count += _count_all_self_calls(arm.body, self_names, source)
        if stmt.default_body is not None:
            count += _count_all_self_calls(stmt.default_body, self_names, source)
    elif isinstance(stmt, IRFor):
        count += _count_all_self_calls(stmt.body, self_names, source)
        count += _count_all_self_calls(stmt.init, self_names, source)
        count += _count_all_self_calls(stmt.next, self_names, source)
    elif isinstance(stmt, IRWhile):
        count += _count_all_self_calls(stmt.body, self_names, source)
    elif isinstance(stmt, IRForeach):
        count += _count_all_self_calls(stmt.body, self_names, source)
    elif isinstance(stmt, IRCatch):
        count += _count_all_self_calls(stmt.body, self_names, source)
    elif isinstance(stmt, IRTry):
        count += _count_all_self_calls(stmt.body, self_names, source)
        for handler in stmt.handlers:
            count += _count_all_self_calls(handler.body, self_names, source)

    return count


def _count_self_calls_in_command_range(
    source: str,
    command_range: Range,
    self_names: frozenset[str],
) -> int:
    """Count self-calls in one command range using lexer token shapes."""
    count = 0

    parsed = _parse_single_command_from_range(source, command_range)
    if parsed is not None:
        argv_texts, _argv_tokens, _argv_single = parsed
        if argv_texts and argv_texts[0] in self_names:
            count += 1

    start = command_range.start.offset
    end = command_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return count

    lexer = TclLexer(
        source[start : end + 1],
        base_offset=start,
        base_line=command_range.start.line,
        base_col=command_range.start.character,
    )
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is not TokenType.CMD:
            continue
        inner = tok.text.strip()
        parsed = _parse_command_words(inner)
        if parsed is not None:
            argv_texts, _, _ = parsed
            if argv_texts and argv_texts[0] in self_names:
                count += 1
        # Count nested substitutions inside this command-substitution token.
        count += _count_name_as_command(inner, self_names)
    return count


def _count_name_as_command(text: str, self_names: frozenset[str]) -> int:
    """Count occurrences of self_names used as commands in *text*."""
    total = 0
    for name in self_names:
        # Match [name ...] or [name] — command substitution invocation.
        total += text.count(f"[{name} ")
        total += text.count(f"[{name}]")
    return total


def _count_condition_self_calls(
    script: IRScript,
    self_names: frozenset[str],
) -> int:
    """Count self-calls embedded in control-flow conditions and subjects.

    These are non-tail positions that ``_count_all_self_calls`` misses
    because it only walks statement-level IR nodes.
    """
    count = 0
    for stmt in script.statements:
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                cond_text = getattr(clause.condition, "text", "") or ""
                count += _count_name_as_command(cond_text, self_names)
                count += _count_condition_self_calls(clause.body, self_names)
            if stmt.else_body is not None:
                count += _count_condition_self_calls(stmt.else_body, self_names)
        elif isinstance(stmt, IRSwitch):
            count += _count_name_as_command(stmt.subject, self_names)
            for arm in stmt.arms:
                if arm.body is not None:
                    count += _count_condition_self_calls(arm.body, self_names)
            if stmt.default_body is not None:
                count += _count_condition_self_calls(stmt.default_body, self_names)
        elif isinstance(stmt, IRWhile):
            cond_text = getattr(stmt.condition, "text", "") or ""
            count += _count_name_as_command(cond_text, self_names)
            count += _count_condition_self_calls(stmt.body, self_names)
        elif isinstance(stmt, IRFor):
            cond_text = getattr(stmt.condition, "text", "") or ""
            count += _count_name_as_command(cond_text, self_names)
            count += _count_condition_self_calls(stmt.body, self_names)
            count += _count_condition_self_calls(stmt.init, self_names)
            count += _count_condition_self_calls(stmt.next, self_names)
        elif isinstance(stmt, IRForeach):
            count += _count_condition_self_calls(stmt.body, self_names)
        elif isinstance(stmt, IRCatch):
            count += _count_condition_self_calls(stmt.body, self_names)
        elif isinstance(stmt, IRTry):
            count += _count_condition_self_calls(stmt.body, self_names)
            for handler in stmt.handlers:
                count += _count_condition_self_calls(handler.body, self_names)
    return count


# O121: tailcall suggestion


@opt(
    code="O121",
    description="Rewrite self-recursive tail calls to `tailcall`.",
    opt_category="recursion",
)
def _emit_o121(ctx: PassContext, site: _TailCallSite, short_name: str) -> None:
    """Emit O121 for a tail-position self-call."""
    if site.kind == "return_subst":
        # return [procName args...] → tailcall procName args...
        value_text = ctx.source[site.stmt_range.start.offset : site.stmt_range.end.offset + 1]
        # Strip 'return ' prefix and extract inner command.
        m = _CMD_SUBST_RE.match(value_text.partition(" ")[2].strip() if " " in value_text else "")
        if m is None:
            # Use the already-parsed inner command.
            inner_parts = []
            for name in _self_name_variants_from_short(short_name):
                inner_parts.append(name)
                break
            inner_parts.extend(site.args)
            replacement = "tailcall " + " ".join(inner_parts)
        else:
            replacement = f"tailcall {m.group(1).strip()}"
    else:
        # Bare call: prepend tailcall.
        cmd_text = ctx.source[site.full_range.start.offset : site.full_range.end.offset + 1]
        replacement = f"tailcall {cmd_text}"

    ctx.optimisations.append(
        Optimisation(
            code="O121",
            message=f"Use tailcall for self-recursive call to {short_name}",
            range=site.full_range,
            replacement=replacement,
        )
    )


def _self_name_variants_from_short(short_name: str) -> frozenset[str]:
    """Minimal name set from just the short name (for replacement text)."""
    return frozenset({short_name})


# O122: recursion-to-loop conversion


@opt(
    code="O122",
    description="Convert fully tail-recursive proc to iterative `while` loop.",
    opt_category="recursion",
)
def _suggest_loop_conversion(
    ctx: PassContext,
    proc,
    sites: list[_TailCallSite],
    short_name: str,
) -> None:
    """Suggest converting a fully tail-recursive proc to a while loop."""
    params = proc.params
    body_source = proc.body_source
    if body_source is None or not params:
        return

    # Verify all tail-call sites pass the right number of arguments.
    for site in sites:
        if len(site.args) != len(params):
            return

    # Locate body_source within the full source.
    source = ctx.source
    proc_start = proc.range.start.offset
    proc_end = proc.range.end.offset
    proc_text = source[proc_start : proc_end + 1]
    body_idx = proc_text.find(body_source)
    if body_idx < 0:
        return
    body_start_in_source = proc_start + body_idx

    # Replace tail-call sites with parameter reassignment (bottom-up).
    modified = body_source
    for site in sorted(sites, key=lambda s: s.full_range.start.offset, reverse=True):
        rel_start = site.full_range.start.offset - body_start_in_source
        rel_end = site.full_range.end.offset - body_start_in_source + 1
        if rel_start < 0 or rel_end > len(modified):
            return
        reassign = _make_reassignment(params, site.args)
        modified = modified[:rel_start] + reassign + modified[rel_end:]

    # Re-indent body for the while loop nesting (add 4 spaces).
    lines = modified.rstrip().split("\n")
    reindented_lines: list[str] = []
    for line in lines:
        if line.strip():
            reindented_lines.append("    " + line)
        else:
            reindented_lines.append(line)
    reindented = "\n".join(reindented_lines)

    # Build the complete replacement proc.
    params_str = proc.params_raw or " ".join(params)
    replacement = (
        f"proc {short_name} {{{params_str}}} {{\n    while {{1}} {{{reindented}\n    }}\n}}"
    )

    full_range = _full_command_range(ctx.source, proc.range) or proc.range
    ctx.optimisations.append(
        Optimisation(
            code="O122",
            message=f"Convert tail-recursive {short_name} to iterative loop",
            range=full_range,
            replacement=replacement,
        )
    )


_LASSIGN_DIALECTS = dialects_since("tcl8.5")


def _make_reassignment(params: tuple[str, ...], args: tuple[str, ...]) -> str:
    """Generate parameter reassignment text for loop conversion.

    Uses ``lassign`` for the multi-parameter case so the assignments
    happen simultaneously (avoiding evaluation-order bugs when a
    later arg refers to an earlier param's old value).  ``lassign``
    was introduced in Tcl 8.5; for older dialects (notably
    ``f5-irules`` which is locked to 8.4.6) we fall back to a temp-
    list expansion that uses only ``set`` / ``lindex``.
    """
    if len(params) == 1:
        return f"set {params[0]} {args[0]}"
    arg_list = " ".join(args)
    param_list = " ".join(params)
    if active_dialect() in _LASSIGN_DIALECTS:
        return f"lassign [list {arg_list}] {param_list}"
    # 8.4-compatible fallback — capture into a temp list and extract by
    # index so the assignments are simultaneous (avoids the
    # evaluation-order bug a sequence of bare ``set`` calls would have).
    pieces = [f"set __tcllsp_tmp [list {arg_list}]"]
    for i, p in enumerate(params):
        pieces.append(f"set {p} [lindex $__tcllsp_tmp {i}]")
    pieces.append("unset __tcllsp_tmp")
    return "; ".join(pieces)


# O123: accumulator-eligible non-tail recursion detection


@opt(
    code="O123",
    description="Detect non-tail recursion eligible for accumulator introduction (hint only).",
    opt_category="recursion",
)
def _detect_accumulator_candidate(
    ctx: PassContext,
    proc,
    self_names: frozenset[str],
    short_name: str,
    tail_sites: list[_TailCallSite],
    source: str,
) -> None:
    """Detect non-tail self-recursion that could use an accumulator.

    Pattern: ``return [expr {$param OP [self ...]}]`` where OP is an
    associative operation (+, *, etc.).
    """
    if not proc.body.statements:
        return

    # Collect ranges already covered by tail-call sites (O121/O122).
    tail_offsets = {s.stmt_range.start.offset for s in tail_sites}

    # Search all return statements for embedded self-calls in expressions.
    for site_range in _find_accumulator_sites(source, proc.body, self_names):
        # Skip if this return statement is already covered by a tail-call site.
        if site_range.start.offset in tail_offsets:
            continue
        full_range = _full_command_range(ctx.source, site_range) or site_range
        ctx.optimisations.append(
            Optimisation(
                code="O123",
                message=(
                    f"Non-tail recursion in {short_name} could be eliminated by "
                    f"introducing an accumulator parameter and using tailcall"
                ),
                range=full_range,
                replacement="",
                hint_only=True,
            )
        )


def _find_accumulator_sites(
    source: str,
    script: IRScript,
    self_names: frozenset[str],
) -> list[Range]:
    """Find return statements with non-tail self-calls inside expressions."""
    sites: list[Range] = []
    if not script.statements:
        return sites

    for stmt in script.statements:
        if isinstance(stmt, IRReturn) and stmt.value is not None:
            inner = _return_command_subst_text(source, stmt.range)
            if inner is not None:
                first_word = inner.split(None, 1)[0] if inner else ""
                if first_word in self_names:
                    continue  # plain tail call, not accumulator pattern
                # Restrict O123 to expression wrappers for clearer signal.
                if first_word != "expr":
                    continue
                if _is_accumulator_pattern(inner, self_names):
                    sites.append(stmt.range)

        # Recurse into branches.
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                sites.extend(_find_accumulator_sites(source, clause.body, self_names))
            if stmt.else_body is not None:
                sites.extend(_find_accumulator_sites(source, stmt.else_body, self_names))
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    sites.extend(_find_accumulator_sites(source, arm.body, self_names))
            if stmt.default_body is not None:
                sites.extend(_find_accumulator_sites(source, stmt.default_body, self_names))

    return sites


_ACCUMULATOR_OPS_RE = re.compile(r"[\+\*]")


def _is_accumulator_pattern(text: str, self_names: frozenset[str]) -> bool:
    """Check if *text* is an accumulator-eligible expression.

    Requirements:
    1. Contains exactly one embedded self-call (not doubly-recursive).
    2. The enclosing ``[expr {...}]`` contains an associative operator
       (``+`` or ``*``) so an accumulator introduction is meaningful.
    """
    total = 0
    for name in self_names:
        total += text.count(f"[{name} ")
        total += text.count(f"[{name}]")
    if total != 1:
        return False
    # Require an associative operator in the expression.
    return bool(_ACCUMULATOR_OPS_RE.search(text))
