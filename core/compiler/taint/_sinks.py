"""Sink detection and warning generation for taint analysis."""

# canonicalisation: audited #246


from __future__ import annotations

from functools import lru_cache

from ...commands.registry import REGISTRY
from ...commands.registry.runtime import (
    TAINT_HINTS,
    regex_pattern_commands,
    regexp_pattern_index,
    taint_double_encode_map,
    taint_sink_safe_colours,
)
from ...commands.registry.taint_hints import TaintColour
from ...common.codes import diag
from ...common.dialect import active_dialect
from ...common.naming import normalise_var_name as _normalise_var_name
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..cfg import CFGBranch, CFGFunction, CFGGoto
from ..ir import (
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRExprEval,
)
from ..ssa import SSAFunction, SSAValueKey
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._lattice import (
    _CRLF_SAFE,
    _T102_SAFE,
    _UNTAINTED,
    TaintLattice,
)
from ._propagation import _COLOUR_LABELS
from ._types import TaintWarning

# W313: destructive file operations with tainted path (taint-aware).
# Suppressed only when the path is both normalised (PATH_NORMALISED)
# AND bounds-checked (PATH_BOUNDED) — i.e. the developer called
# [file normalize] and verified the result stays within an intended
# directory via string match/first/equal.
diag(
    "W313",
    "Destructive file operation with variable path — path-traversal risk.",
    section="security",
)

# iRules taint sink diagnostic codes
diag("IRULE3001", "Tainted data in HTTP response body.", section="irules_security")
diag("IRULE3002", "Tainted data in HTTP header or cookie value.", section="irules_security")
diag(
    "IRULE3003",
    "Tainted data in `log` command — log injection risk.",
    section="irules_security",
)

# Taint diagnostic codes (T-series) — co-registered with codes_taint.py
diag(
    "T100",
    "Tainted data flows into a dangerous code-execution sink (`eval`, `expr`, `exec`, `uplevel`, `subst`).",
    section="taint",
)
diag("T101", "Tainted data flows into an output command (`puts`).", section="taint")
diag(
    "T102",
    "Tainted data in option position without `--` terminator — option injection risk.",
    section="taint",
)
diag("T103", "Taint propagation through variable.", section="taint", internal=True)
diag("T106", "Taint propagation through command return.", section="taint", internal=True)

# Diagnostic messages

_OUTPUT_MESSAGES: dict[str, str] = {
    "T101": "Tainted variable ${var} flows into {cmd}; output may contain injected content",
    "T103": "Tainted variable ${var} in regexp pattern position ({cmd}); risk of regex injection or ReDoS",
    "IRULE3001": "Tainted variable ${var} in HTTP response body ({cmd}); risk of XSS or content injection",
    "IRULE3002": "Tainted variable ${var} in HTTP header/cookie value ({cmd}); risk of header injection",
    "IRULE3003": "Tainted variable ${var} in log output ({cmd}); risk of log injection or log forging",
    "IRULE3004": "Tainted variable ${var} in redirect URL ({cmd}); risk of open redirect",
    "T102": "Tainted variable ${var} in option position of '{cmd}' without '--' terminator; risk of option injection",
    "T104": "Tainted variable ${var} in network address argument of {cmd}; risk of SSRF (server-side request forgery)",
    "T105": "Tainted variable ${var} in {cmd} script argument; risk of cross-interpreter code injection",
    "T106": "Variable ${var} is already {colour}; passing through {cmd} double-encodes the value",
}


def _has_option_terminator(args: tuple[str, ...], scan_start: int) -> bool:
    """Return True if ``--`` appears at or after *scan_start* in *args*."""
    for i in range(scan_start, len(args)):
        if args[i] == "--":
            return True
    return False


def _stmt_command_args(stmt) -> tuple[str, tuple[str, ...]] | None:
    """Return ``(command, args)`` for sink classification and arg inspection."""
    if isinstance(stmt, (IRCall, IRBarrier)):
        return stmt.command, stmt.args
    if isinstance(stmt, IRAssignValue):
        return parse_command_substitution(stmt.value)
    return None


@lru_cache(maxsize=8192)
def _arg_var_names(arg: str) -> frozenset[str]:
    """Return normalised variable names referenced by *arg*."""
    names: set[str] = set()
    lexer = TclLexer(arg)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            return frozenset(names)
        if tok.type is TokenType.VAR:
            name = _normalise_var_name(tok.text)
            if name:
                names.add(name)


@lru_cache(maxsize=32768)
def _args_var_indexes(
    args: tuple[str, ...],
    var_name: str,
) -> tuple[int, ...]:
    """Return indexes in *args* where ``$var_name`` is referenced."""
    return tuple(i for i, arg in enumerate(args) if var_name in _arg_var_names(arg))


def _stmt_var_arg_indexes(stmt, var_name: str) -> tuple[int, ...]:
    """Return argument indexes where *var_name* appears for the statement sink."""
    parsed = _stmt_command_args(stmt)
    if parsed is None:
        return ()
    _, args = parsed
    return _args_var_indexes(args, var_name)


def _classify_sink(
    stmt,
    is_irules: bool,  # noqa: ARG001 – kept for call-site compat; dialect subsumes it
    dialect: str | None = None,
) -> list[tuple[str, str]]:
    """Return a list of ``(code, command_label)`` sink matches for *stmt*.

    A single statement can match multiple sink categories (e.g. ``exec``
    is both a dangerous sink and an output sink).  We return all matches.
    """
    results: list[tuple[str, str]] = []

    parsed = _stmt_command_args(stmt)
    if parsed is None:
        return results
    command, args = parsed

    sub = args[0] if args else None

    # Single-pass taint sink classification (one dict lookup + dialect filter).
    sink = REGISTRY.classify_taint_sinks(command, sub, dialect)

    # T100: dangerous code-execution sinks
    if sink.is_code_sink:
        results.append(("T100", command))

    # T101 / IRULE3001 / IRULE3002 / IRULE3004: output sinks
    if sink.output_sink is not None:
        label = f"{command} {sub}" if sub and sink.output_sink_is_subcommand_qualified else command
        results.append((sink.output_sink, label))

    # IRULE3003: log injection
    if sink.log_sink is not None:
        results.append((sink.log_sink, command))

    # T102: option injection via tainted input (colour-suppressed below)
    profile = REGISTRY.resolve_option_terminator(command, args)
    if profile is not None and not _has_option_terminator(args, profile.scan_start):
        cmd_label = command
        if profile.subcommand is not None:
            cmd_label = f"{command} {profile.subcommand}"
        results.append(("T102", cmd_label))

    # T104: network address sinks (SSRF)
    if sink.is_network_sink:
        results.append(("T104", command))

    # T105: cross-interpreter code execution
    if (
        sink.interp_eval_subcommands is not None
        and args
        and args[0] in sink.interp_eval_subcommands
    ):
        results.append(("T105", f"{command} {args[0]}"))

    return results


# Suppression logic


def _should_suppress_t100(stmt, taint: TaintLattice) -> bool:
    """Return True if T100 should be suppressed for this taint colour + sink.

    The suppression colour for each sink command is declared on its
    ``CommandSpec.taint_sink_safe_colour`` field (e.g. ``exec`` →
    ``SHELL_ATOM``, ``eval``/``uplevel`` → ``LIST_CANONICAL``).
    """
    if not taint.tainted:
        return False
    parsed = _stmt_command_args(stmt)
    if parsed is None:
        return False
    command, _ = parsed
    safe_colour = taint_sink_safe_colours().get(command)
    if safe_colour is not None and bool(taint.colour & safe_colour):
        return True
    return False


def _should_suppress_t102(taint: TaintLattice) -> bool:
    """Return True if T102 should be suppressed for this taint colour.

    Values with PATH_PREFIXED, NON_DASH_PREFIXED, IP_ADDRESS, PORT, or FQDN
    colours provably cannot start with ``-`` and are safe from option injection.
    """
    if not taint.tainted:
        return False
    return bool(taint.colour & _T102_SAFE)


def _should_suppress_irule3002_for_var(
    stmt,
    var_name: str,
    taint: TaintLattice,
) -> bool:
    """Return True if IRULE3002 is not actionable for this var/position."""
    if not taint.tainted:
        return False

    # CRLF-safe values cannot inject header/cookie line breaks.
    if bool(taint.colour & _CRLF_SAFE):
        return True

    # Header name position with token-safe value is acceptable.
    if not bool(taint.colour & TaintColour.HEADER_TOKEN_SAFE):
        return False

    parsed = _stmt_command_args(stmt)
    if parsed is None:
        return False
    command, args = parsed
    if command not in {"HTTP::header", "HTTP::cookie"}:
        return False
    if not args or args[0] not in {"insert", "replace"}:
        return False
    return 1 in _stmt_var_arg_indexes(stmt, var_name)


def _should_suppress_sink_warning(
    code: str,
    stmt,
    var_name: str,
    taint: TaintLattice,
) -> bool:
    """Return True when a sink warning is mitigated by taint colour."""
    if code == "T100":
        return _should_suppress_t100(stmt, taint)
    if code == "T102":
        return _should_suppress_t102(taint)
    if code == "T103":
        return bool(taint.tainted and (taint.colour & TaintColour.REGEX_LITERAL))
    if code == "IRULE3002":
        return _should_suppress_irule3002_for_var(stmt, var_name, taint)
    if code == "IRULE3003":
        return bool(taint.tainted and (taint.colour & _CRLF_SAFE))
    if code == "IRULE3001":
        return bool(taint.tainted and (taint.colour & TaintColour.HTML_ESCAPED))
    if code == "IRULE3004":
        # Relative redirect (starts with "/") is same-origin and safe.
        return bool(
            taint.tainted
            and (taint.colour & (TaintColour.PATH_PREFIXED | TaintColour.PATH_NORMALISED))
        )
    if code == "T104":
        # IP_ADDRESS, PORT, or FQDN colours prove the value is a valid
        # network address from a trusted source (e.g. allowlist lookup).
        return bool(
            taint.tainted
            and (taint.colour & (TaintColour.IP_ADDRESS | TaintColour.PORT | TaintColour.FQDN))
        )
    if code == "T105":
        # LIST_CANONICAL preserves element boundaries, same as eval suppression.
        return bool(taint.tainted and (taint.colour & TaintColour.LIST_CANONICAL))
    return False


def _regexp_pattern_arg_index(command: str, args: tuple[str, ...]) -> int | None:
    """Return the 0-based arg index of the regex pattern in *args*, or None."""
    if command not in regex_pattern_commands():
        return None
    return regexp_pattern_index(args)


# Main sink-finding functions


def _find_taint_sinks(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice],
    executable_blocks: set[str],
) -> list[TaintWarning]:
    """Find tainted variables flowing into dangerous or output commands."""
    warnings: list[TaintWarning] = []
    dialect = active_dialect()
    is_irules = dialect == "f5-irules"

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]
            uses = ssa_stmt.uses

            # Special case: IRAssignExpr / IRExprEval (expr with parsed AST).
            if isinstance(stmt, (IRAssignExpr, IRExprEval)):
                for name, ver in uses.items():
                    t = taints.get((name, ver), _UNTAINTED)
                    if t.tainted:
                        warnings.append(
                            TaintWarning(
                                range=stmt.range,
                                variable=name,
                                sink_command="expr",
                                code="T100",
                                message=(
                                    f"Tainted variable ${name} used in expr; "
                                    f"possible code injection"
                                ),
                            )
                        )
                continue

            # Classify sinks for this statement.
            sinks = _classify_sink(stmt, is_irules, dialect)

            # T103: tainted data in regexp/regsub pattern position.
            parsed = _stmt_command_args(stmt)
            if parsed is not None:
                cmd, cmd_args = parsed
                pattern_idx = _regexp_pattern_arg_index(cmd, cmd_args)
                if pattern_idx is not None:
                    for name, ver in uses.items():
                        t = taints.get((name, ver), _UNTAINTED)
                        if not t.tainted:
                            continue
                        if pattern_idx in _stmt_var_arg_indexes(stmt, name):
                            if not _should_suppress_sink_warning(
                                "T103",
                                stmt,
                                name,
                                t,
                            ):
                                template = _OUTPUT_MESSAGES["T103"]
                                warnings.append(
                                    TaintWarning(
                                        range=stmt.range,
                                        variable=name,
                                        sink_command=cmd,
                                        code="T103",
                                        message=template.format(
                                            var=name,
                                            cmd=cmd,
                                        ),
                                    )
                                )

                # T106: double-encoding detection.
                dup_colour = taint_double_encode_map().get(cmd)
                if dup_colour is not None:
                    for name, ver in uses.items():
                        t = taints.get((name, ver), _UNTAINTED)
                        if t.tainted and bool(t.colour & dup_colour):
                            label = _COLOUR_LABELS.get(dup_colour, str(dup_colour))
                            template = _OUTPUT_MESSAGES["T106"]
                            warnings.append(
                                TaintWarning(
                                    range=stmt.range,
                                    variable=name,
                                    sink_command=cmd,
                                    code="T106",
                                    message=template.format(
                                        var=name,
                                        cmd=cmd,
                                        colour=label,
                                    ),
                                )
                            )

            if not sinks:
                continue

            # Check each used variable for taint.
            for name, ver in uses.items():
                t = taints.get((name, ver), _UNTAINTED)
                if t.tainted:
                    for code, cmd_label in sinks:
                        if _should_suppress_sink_warning(code, stmt, name, t):
                            continue
                        template = _OUTPUT_MESSAGES.get(code)
                        if template is not None:
                            message = template.format(var=name, cmd=cmd_label)
                        else:
                            message = (
                                f"Tainted variable ${name} flows into {cmd_label}; "
                                f"possible code injection"
                            )
                        warnings.append(
                            TaintWarning(
                                range=stmt.range,
                                variable=name,
                                sink_command=cmd_label,
                                code=code,
                                message=message,
                            )
                        )

    return warnings


# Setter constraint violations (IRULE3101)


def _find_setter_constraint_violations(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice],
    executable_blocks: set[str],
) -> list[TaintWarning]:
    """Find setter calls that violate required-prefix constraints."""
    warnings: list[TaintWarning] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]

            if not isinstance(stmt, IRCall):
                continue

            hint = TAINT_HINTS.get(stmt.command)
            if hint is None or not hint.setter_constraints:
                continue

            for constraint in hint.setter_constraints:
                ai = constraint.arg_index
                if ai >= len(stmt.args):
                    continue
                arg_val = stmt.args[ai]

                # Literal check: if it's a pure literal, check the prefix.
                if not arg_val.startswith("$") and "[" not in arg_val:
                    if not arg_val.startswith(constraint.required_prefix):
                        warnings.append(
                            TaintWarning(
                                range=stmt.range,
                                variable="",
                                sink_command=stmt.command,
                                code=constraint.code,
                                message=constraint.message,
                            )
                        )
                    continue

                # Variable reference: check taint colour.
                if is_pure_var_ref(arg_val):
                    var_name = _normalise_var_name(arg_val)
                    ver = ssa_stmt.uses.get(var_name, 0)
                    t = taints.get((var_name, ver), _UNTAINTED)
                    if t.tainted and bool(
                        t.colour
                        & (
                            TaintColour.PATH_PREFIXED
                            | TaintColour.PATH_NORMALISED
                            | TaintColour.PATH_BOUNDED
                        )
                    ):
                        # PATH_PREFIXED → provably starts with "/".
                        # PATH_NORMALISED → canonicalised path (traversal-safe).
                        # PATH_BOUNDED → normalised and verified within bounds.
                        continue
                    # Variable without safe path colour — warn.
                    warnings.append(
                        TaintWarning(
                            range=stmt.range,
                            variable=var_name if is_pure_var_ref(arg_val) else "",
                            sink_command=stmt.command,
                            code=constraint.code,
                            message=constraint.message,
                        )
                    )
                    continue

                # Dynamic expression (interpolation, command sub, etc.) — warn.
                warnings.append(
                    TaintWarning(
                        range=stmt.range,
                        variable="",
                        sink_command=stmt.command,
                        code=constraint.code,
                        message=constraint.message,
                    )
                )

    return warnings


# W313: Destructive file operations with tainted/variable path


_destructive_file_subs_cache: frozenset[str] | None = None


def _destructive_file_subs() -> frozenset[str]:
    global _destructive_file_subs_cache
    if _destructive_file_subs_cache is None:
        from core.commands.registry import REGISTRY

        spec = REGISTRY.get_any("file")
        if spec is not None:
            _destructive_file_subs_cache = frozenset(
                name for name, sub in spec.subcommands.items() if sub.destructive
            )
        else:
            _destructive_file_subs_cache = frozenset()
    return _destructive_file_subs_cache


_DESTRUCTIVE_SKIP_ARGS = frozenset({"-force", "--"})


def _is_normalised_def(
    var_name: str,
    version: int,
    ssa: SSAFunction,
    cfg: CFGFunction,
) -> bool:
    """Return True if the SSA def of *var_name*@*version* is ``[file normalize ...]``."""
    for bn, ssa_block in ssa.blocks.items():
        ir_block = cfg.blocks.get(bn)
        if ir_block is None:
            continue
        for i, ssa_stmt in enumerate(ssa_block.statements):
            if i >= len(ir_block.statements):
                continue
            defs = ssa_stmt.defs
            if defs.get(var_name) != version:
                continue
            ir_stmt = ir_block.statements[i]
            if isinstance(ir_stmt, IRAssignValue):
                val = ir_stmt.value.strip()
                if val.startswith("[file normalize "):
                    return True
            return False
    return False


def _compute_branch_guard_map(
    cfg: CFGFunction,
) -> dict[str, set[str]]:
    """Build a map of block → variable names guarded by bounds checks.

    For each ``CFGBranch`` whose condition is a ``string match``,
    ``string first``, or ``string equal`` call, the **true-target**
    block (and blocks only reachable through it) gains PATH_BOUNDED
    for the variable used as the path operand.

    Returns ``{block_name: {var_name, ...}}``.
    """
    guarded: dict[str, set[str]] = {}

    for bn, block in cfg.blocks.items():
        term = block.terminator
        if not isinstance(term, CFGBranch) or term.condition is None:
            continue

        # Extract variable names and negation status from the condition.
        negated, var_name = _extract_guard_var(term.condition)
        if var_name is None:
            continue

        # When the condition is negated (e.g. `if {![string match ...]}`),
        # the bounds check holds in the **false** branch (match succeeded).
        # When not negated, the bounds check holds in the **true** branch.
        if negated:
            guarded_target = term.false_target
            other_target = term.true_target
        else:
            guarded_target = term.true_target
            other_target = term.false_target
        _propagate_guard(cfg, guarded_target, other_target, var_name, guarded)

    return guarded


def _extract_guard_var(expr, *, _negated: bool = False) -> tuple[bool, str | None]:
    """Extract the path variable from a bounds-check condition expression.

    Returns ``(negated, var_name)`` where *negated* indicates the
    condition is inverted (e.g. ``![string match ...]``).

    Recognises ``[string match PATTERN $var]``, ``[string first NEEDLE $var]``,
    and ``[string equal ... $var ...]`` in branch conditions.
    """
    # ExprUnary(op=NOT, operand=...) — recurse, flipping negation.
    operand = getattr(expr, "operand", None)
    if operand is not None:
        return _extract_guard_var(operand, _negated=not _negated)

    # ExprCommand wraps [cmd ...].
    text = getattr(expr, "text", None)
    if not text:
        # ExprBinary — check both sides.
        left = getattr(expr, "left", None)
        right = getattr(expr, "right", None)
        if left:
            neg, result = _extract_guard_var(left, _negated=_negated)
            if result:
                return neg, result
        if right:
            return _extract_guard_var(right, _negated=_negated)
        return _negated, None

    parsed = parse_command_substitution(text.strip())
    if parsed is None:
        return _negated, None
    command, args = parsed
    if command != "string" or not args:
        return _negated, None
    subcmd = args[0]
    if subcmd == "match" and len(args) >= 3:
        return _negated, _extract_var_name(args[-1])
    if subcmd == "first" and len(args) >= 3:
        return _negated, _extract_var_name(args[2])
    if subcmd == "equal":
        filtered: list[str] = []
        skip_next = False
        for arg in args[1:]:
            if skip_next:
                skip_next = False
                continue
            if arg == "-length":
                skip_next = True
                continue
            if arg.startswith("-"):
                continue
            filtered.append(arg)
        for f_arg in filtered:
            name = _extract_var_name(f_arg)
            if name:
                return _negated, name
    return _negated, None


def _extract_var_name(arg: str) -> str | None:
    """Extract a variable name from ``$name`` or ``${name}``."""
    text = arg.strip()
    if text.startswith("${") and text.endswith("}"):
        return text[2:-1]
    if text.startswith("$") and text[1:].isidentifier():
        return text[1:]
    return None


_non_returning_cache: frozenset[str] | None = None


def _non_returning_commands() -> frozenset[str]:
    global _non_returning_cache
    if _non_returning_cache is None:
        from core.commands.registry import REGISTRY

        _non_returning_cache = REGISTRY.check_trait_commands("terminates_block")
    return _non_returning_cache


def _is_dead_end_block(cfg: CFGFunction, block_name: str) -> bool:
    """Return True if the block only contains non-returning commands.

    If the block executes ``error``, ``return``, ``throw``, or ``exit``
    as its sole command (with no other side effects), the successor
    blocks are effectively unreachable from this path.
    """
    block = cfg.blocks.get(block_name)
    if block is None or not block.statements:
        return False
    # Check if ALL statements are non-returning.
    non_returning = _non_returning_commands()
    for stmt in block.statements:
        if isinstance(stmt, IRCall) and stmt.command in non_returning:
            return True
    return False


def _propagate_guard(
    cfg: CFGFunction,
    guarded_target: str,
    other_target: str,
    var_name: str,
    guarded: dict[str, set[str]],
) -> None:
    """Mark *guarded_target* and its exclusive successors as guarded.

    Stops at blocks that are also reachable from *other_target* (merge points).
    """
    # If the other branch is a non-returning block (error/return/throw),
    # the merge point is only reachable from the guarded branch — extend
    # the guard through the merge.
    other_is_dead_end = _is_dead_end_block(cfg, other_target)

    other_reachable: set[str] = set()
    if not other_is_dead_end:
        stack = [other_target]
        while stack:
            b = stack.pop()
            if b in other_reachable or b not in cfg.blocks:
                continue
            other_reachable.add(b)
            match cfg.blocks[b].terminator:
                case CFGGoto(target=t):
                    stack.append(t)
                case CFGBranch(true_target=tt, false_target=ft):
                    stack.extend([tt, ft])

    visit_stack = [guarded_target]
    visited: set[str] = set()
    while visit_stack:
        b = visit_stack.pop()
        if b in visited or b not in cfg.blocks:
            continue
        if b in other_reachable and b != guarded_target:
            continue
        visited.add(b)
        guarded.setdefault(b, set()).add(var_name)
        match cfg.blocks[b].terminator:
            case CFGGoto(target=t):
                visit_stack.append(t)
            case CFGBranch(true_target=tt, false_target=ft):
                visit_stack.extend([tt, ft])


def _find_destructive_file_warnings(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice],
    executable_blocks: set[str],
) -> list[TaintWarning]:
    """W313: Flag destructive file ops where path arguments carry taint.

    ``file delete``, ``file rename``, and ``file mkdir`` with a variable
    or substituted path argument risk path-traversal attacks.

    **Suppressed** when the path variable is both normalised
    (``PATH_NORMALISED``) *and* bounds-checked (``PATH_BOUNDED``).
    Bounds-checking is detected via branch-dependent guard analysis:
    when a ``CFGBranch`` condition is a ``string match/first/equal``
    on a normalised variable, the true-target block (and its dominated
    successors) treat that variable as ``PATH_BOUNDED``.
    """
    # Build the guard map: block → {var_names with PATH_BOUNDED}.
    guard_map = _compute_branch_guard_map(cfg)

    warnings: list[TaintWarning] = []
    destructive_subs = _destructive_file_subs()

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]

            if not isinstance(stmt, IRCall):
                continue
            if stmt.canonical_command != "::file":
                continue
            if not stmt.args or stmt.args[0] not in destructive_subs:
                continue

            sub = stmt.args[0]

            # Skip -force / -- options to find path arguments.
            path_start = 1
            while path_start < len(stmt.args) and stmt.args[path_start] in _DESTRUCTIVE_SKIP_ARGS:
                path_start += 1

            # Check each path argument for tainted/variable content.
            emitted = False
            for name, ver in ssa_stmt.uses.items():
                if emitted:
                    break

                # Only warn if the variable appears in a path-position argument.
                arg_indexes = _args_var_indexes(stmt.args, name)
                path_indexes = tuple(i for i in arg_indexes if i >= path_start)
                if not path_indexes:
                    continue

                # Determine normalisation and bounds-check status.
                t = taints.get((name, ver), _UNTAINTED)
                is_normalised = (
                    t.tainted and bool(t.colour & TaintColour.PATH_NORMALISED)
                ) or _is_normalised_def(name, ver, ssa, cfg)

                # PATH_BOUNDED: from lattice or from branch guard analysis.
                is_bounded = (t.tainted and bool(t.colour & TaintColour.PATH_BOUNDED)) or (
                    is_normalised and name in guard_map.get(bn, set())
                )

                # Suppress when both normalised AND bounds-checked.
                if is_bounded:
                    continue

                # Emit W313 — adjust message based on normalisation status.
                if is_normalised:
                    message = (
                        f"file {sub} with normalised path (${name}) — "
                        "verify it stays within the intended directory "
                        '(e.g. [string match "$base/*" ${name}]).'
                    )
                else:
                    message = (
                        f"file {sub} with a variable path (${name}) risks "
                        "path-traversal. Normalise with [file normalize] and "
                        "verify it stays within the intended directory."
                    )

                warnings.append(
                    TaintWarning(
                        range=stmt.range,
                        variable=name,
                        sink_command=f"file {sub}",
                        code="W313",
                        message=message,
                    )
                )
                emitted = True  # one warning per command

    return warnings
