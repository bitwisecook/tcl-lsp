"""Sink detection and warning generation for taint analysis."""

from __future__ import annotations

from functools import lru_cache

from ...commands.registry import REGISTRY
from ...commands.registry.runtime import TAINT_HINTS, regexp_pattern_index
from ...commands.registry.taint_hints import TaintColour
from ...common.dialect import active_dialect
from ...common.naming import normalise_var_name as _normalise_var_name
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..cfg import CFGFunction
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
from ._propagation import _COLOUR_LABELS, _DOUBLE_ENCODE_MAP
from ._types import TaintWarning

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
    profiles = REGISTRY.option_terminator_profiles(command)
    if profiles:
        profile = None
        for p in profiles:
            if p.subcommand is not None and args and p.subcommand == args[0]:
                profile = p
                break
        if profile is None:
            for p in profiles:
                if p.subcommand is None:
                    profile = p
                    break
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

    * ``exec`` with ``SHELL_ATOM``: the value is a single token with no
      shell metacharacters, so it cannot cause argument splitting or
      operator injection in exec.
    * ``eval`` / ``uplevel`` with ``LIST_CANONICAL``: the value is a
      canonical Tcl list, so element boundaries are preserved and the
      data cannot inject new command words.
    """
    if not taint.tainted:
        return False
    parsed = _stmt_command_args(stmt)
    if parsed is None:
        return False
    command, _ = parsed
    if command == "exec" and bool(taint.colour & TaintColour.SHELL_ATOM):
        return True
    if command in ("eval", "uplevel") and bool(taint.colour & TaintColour.LIST_CANONICAL):
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
    if command not in ("regexp", "regsub"):
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
                dup_colour = _DOUBLE_ENCODE_MAP.get(cmd)
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
                        t.colour & (TaintColour.PATH_PREFIXED | TaintColour.PATH_NORMALISED)
                    ):
                        # PATH_PREFIXED → provably starts with "/".
                        # PATH_NORMALISED → canonicalised path (traversal-safe).
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
