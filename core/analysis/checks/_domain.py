"""Domain-specific and iRules checks."""

from __future__ import annotations

from collections.abc import Mapping

from shared.codes import diag
from shared.dialect import active_dialect
from shared.naming import normalise_qualified_name
from shared.ranges import range_from_token

from ...commands.registry import REGISTRY
from ...commands.registry.models import DialectStatus, FormKind
from ...commands.registry.runtime import (
    ArgRole,
    arg_indices_for_role,
    normalized_flag_commands,
)
from ...parsing.tokens import Token, TokenType
from ..semantic_model import Diagnostic, Severity
from ._helpers import (
    _first_token_is_braced,
    _has_substitution,
    _is_bare_single_var_substitution,
    _tok_is_quoted,
)

# W002: Disabled command in active dialect

_EMPTY_USER_PROCS: Mapping[str, int] = {}


@diag("W002", "Command is disabled in active dialect profile.", section="warning")
def check_disabled_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    user_procs: Mapping[str, int] = _EMPTY_USER_PROCS,
) -> list[Diagnostic]:
    """W002: Warn when a command is disabled in the active dialect profile.

    ``user_procs`` maps a qualified proc name to the source offset of its
    earliest unconditional top-level definition. W002 is suppressed only
    when the call site appears strictly after such a definition — this
    mirrors Tcl's runtime command resolution, where a proc defined
    earlier on the unconditional script path shadows the would-be
    disallowed built-in. Procs defined inside conditionals, loops, or
    another proc body are not counted, so their shadowing is not assumed.
    """
    if not cmd_name or not all_tokens:
        return []

    # Variable- or command-substitution as the command name (``$obj method``,
    # ``[lookup] arg``) cannot be resolved statically to any built-in. The
    # variable's name happens to coincide with a registry entry (e.g. a Tk
    # widget handle named ``$table`` versus the iRules ``table`` command),
    # but the actual command dispatched at runtime is whatever string the
    # variable holds. W307 already flags non-literal command names.
    if all_tokens[0].type in (TokenType.VAR, TokenType.CMD):
        return []

    qualified = normalise_qualified_name(cmd_name)
    def_offset = user_procs.get(qualified)
    call_offset = all_tokens[0].start.offset
    if def_offset is not None and def_offset < call_offset:
        return []

    # Registry is keyed without a leading ``::`` — strip it so both
    # bare (``log``) and fully-qualified (``::log``) call sites
    # resolve to the same spec.
    lookup = qualified.lstrip(":")

    dialect = active_dialect()
    status = REGISTRY.command_status(lookup, dialect)
    if status is DialectStatus.DISALLOWED:
        # The base "disabled in active profile" wording is dialect-agnostic.
        # Older revisions appended "(available in the iRules dialect)" /
        # "Select iRules as the language" hints when the command happened
        # to exist in f5-irules, but those hints only ever fired when the
        # active dialect was NOT iRules — so they were guaranteed-noise
        # for users who have no interest in iRules.  Per #407 feedback:
        # iRules-specific messaging is irrelevant outside iRules mode.
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=f"'{cmd_name}' is disabled in the active dialect profile",
                severity=Severity.WARNING,
                code="W002",
            )
        ]
    return []


# W004: Option not available in active dialect


@diag(
    "W004",
    "Command option is not available in the active dialect.",
    section="warning",
)
def check_dialect_invalid_option(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W004: Warn when a switch is used in a dialect that doesn't accept it.

    Looks for ``-foo`` flags in the args, then asks the registry whether
    each is recognised in the active dialect.  Fires when the registry
    reports :class:`DialectStatus.DISALLOWED` — i.e. the option exists
    on the command but its ``dialects`` frozenset excludes the active
    one.  Examples: ``lsearch -stride`` in tcl8.6, ``regsub -command``
    in tcl8.x, ``vwait -all`` in tcl8.x, ``socket -reuseaddr`` in
    tcl8.x.

    Skips substituted argument values (``-foo $bar``), command-only
    invocations (``cmd``), and non-flag tokens.  Subcommand-scoped
    options consult the subcommand's dialect set.
    """
    if not args or not arg_tokens:
        return []

    spec = REGISTRY.get(cmd_name, active_dialect())
    if spec is None:
        return []

    # Identify the subcommand (if any) so the registry can pick the
    # right options table.
    sub: str | None = None
    if spec.subcommands and args:
        first = args[0]
        if first in spec.subcommands:
            sub = first

    dialect = active_dialect()
    diagnostics: list[Diagnostic] = []
    seen_at_offset: set[int] = set()

    # Scan every arg that looks like a switch.  Many commands take
    # options after a positional (``fconfigure $chan -nodelay 1``,
    # ``clock scan now -validate 1``) so we don't stop at the first
    # non-flag.  False positives are limited because we only flag
    # options the registry recognises on this command/subcommand —
    # patterns and unrelated literals won't be in the option table.
    # ``--`` terminates the scan (literal arg, no further options).
    start_idx = 1 if sub is not None else 0
    options_with_value = REGISTRY.options_with_values(cmd_name, sub, dialect)
    i = start_idx
    while i < len(args):
        arg = args[i]
        if arg == "--":
            break
        if not arg.startswith("-") or len(arg) < 2:
            i += 1
            continue
        # Skip pure numeric literals (negative numbers).
        if arg[1:].lstrip("-").replace(".", "", 1).isdigit():
            i += 1
            continue
        if i >= len(arg_tokens):
            i += 1
            continue
        tok = arg_tokens[i]
        if tok.type in (TokenType.VAR, TokenType.CMD):
            i += 1
            continue
        if tok.start.offset in seen_at_offset:
            i += 1
            continue
        seen_at_offset.add(tok.start.offset)
        status = REGISTRY.option_status(cmd_name, sub, arg, dialect)
        if status is DialectStatus.DISALLOWED:
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"Option '{arg}' on '{cmd_name}'"
                        + (f" {sub}" if sub else "")
                        + f" is not available in the active dialect ({dialect})."
                    ),
                    severity=Severity.WARNING,
                    code="W004",
                )
            )
        # Skip the option's value to avoid scanning it.
        i += 2 if arg in options_with_value else 1
    return diagnostics


# W003: Expression operator not available in active dialect (lt/le/gt/ge)


def _find_dialect_invalid_ops(node, dialect: str):
    """Walk expression AST and collect operators whose dialect gate doesn't
    include *dialect*.

    Returns a list of ``(BinOp, op_name)`` pairs.  Catches ``in`` / ``ni``
    in pre-Tcl-8.5 dialects (TIP 201) and ``lt`` / ``le`` / ``gt`` / ``ge``
    in pre-Tcl-9.0 dialects (TIP 461).  New gated operators just need
    their entry in :data:`TCL_OPERATOR_DIALECTS`.
    """
    from ...commands.registry.operators import operator_supports_dialect
    from ...compiler.expr_ast import (
        BinOp,
        ExprBinary,
        ExprCall,
        ExprTernary,
        ExprUnary,
    )

    found: list[tuple[BinOp, str]] = []
    # Operators with version-dependent dialect availability.  ``in`` / ``ni``
    # were added in Tcl 8.5 (TIP 201); ``lt`` / ``le`` / ``gt`` / ``ge``
    # were added in Tcl 9.0 (TIP 461).  Both groups are gated through
    # :data:`TCL_OPERATOR_DIALECTS` in ``commands.registry.operators``.
    _GATED_OP_NAMES = {
        BinOp.IN: "in",
        BinOp.NI: "ni",
        BinOp.STR_LT: "lt",
        BinOp.STR_LE: "le",
        BinOp.STR_GT: "gt",
        BinOp.STR_GE: "ge",
    }

    match node:
        case ExprBinary(op=op, left=left, right=right):
            found.extend(_find_dialect_invalid_ops(left, dialect))
            found.extend(_find_dialect_invalid_ops(right, dialect))
            name = _GATED_OP_NAMES.get(op)
            if name is not None and not operator_supports_dialect(name, dialect):
                found.append((op, name))
        case ExprUnary(operand=operand):
            found.extend(_find_dialect_invalid_ops(operand, dialect))
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            found.extend(_find_dialect_invalid_ops(cond, dialect))
            found.extend(_find_dialect_invalid_ops(tb, dialect))
            found.extend(_find_dialect_invalid_ops(fb, dialect))
        case ExprCall(args=args):
            for arg in args:
                found.extend(_find_dialect_invalid_ops(arg, dialect))
    return found


@diag(
    "W003",
    "Expression operator not available in active dialect.",
    section="warning",
)
def check_dialect_invalid_expr_operator(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W003: Warn when an expr operator unavailable in the active dialect is used.

    Currently fires for:

    * ``in`` / ``ni`` (TIP 201, Tcl 8.5+) — tclsh 8.4 and iRules
      (which is locked to 8.4.6) reject them with ``syntax error in
      expression``.
    * ``lt`` / ``le`` / ``gt`` / ``ge`` (TIP 461, Tcl 9.0+) — tclsh
      8.5 / 8.6 reject them with ``invalid bareword``.

    The :data:`TCL_OPERATOR_DIALECTS` table is the source of truth;
    adding a new gated operator there is enough to extend this check.
    """
    from ...compiler.expr_ast import ExprRaw
    from ...parsing.expr_parser import parse_expr

    dialect = active_dialect()
    expr_indices = arg_indices_for_role(cmd_name, args, ArgRole.EXPR)
    if not expr_indices:
        return []

    diagnostics: list[Diagnostic] = []
    for idx in sorted(expr_indices):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue

        tok = arg_tokens[idx]
        # ``expr a + b c + d`` form: concatenate the operand args.
        if cmd_name == "expr" and len(args) > 1:
            expr_text = " ".join(args)
        else:
            expr_text = args[idx]

        parsed = parse_expr(expr_text.strip(), dialect=dialect)
        if isinstance(parsed, ExprRaw):
            continue

        offenders = _find_dialect_invalid_ops(parsed, dialect)
        # Deduplicate (we only need one diagnostic per op name per call).
        seen: set[str] = set()
        # Per-op introduction hint so the message points the reader at the
        # right Tcl version / TIP without us having to special-case dialects
        # at every call site.
        intro_hint = {
            "in": "Tcl 8.5+ (TIP 201)",
            "ni": "Tcl 8.5+ (TIP 201)",
            "lt": "Tcl 9.0+ (TIP 461)",
            "le": "Tcl 9.0+ (TIP 461)",
            "gt": "Tcl 9.0+ (TIP 461)",
            "ge": "Tcl 9.0+ (TIP 461)",
        }
        for _op, name in offenders:
            if name in seen:
                continue
            seen.add(name)
            hint = intro_hint.get(name, "a later Tcl version")
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"Expression operator '{name}' is not available in the "
                        f"active dialect ({dialect}); requires {hint}."
                    ),
                    severity=Severity.WARNING,
                    code="W003",
                )
            )

    return diagnostics


# IRULE2002: Deprecated iRules command


@diag("IRULE2002", "Deprecated iRules command.", section="irules")
def check_deprecated_irules_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """IRULE2002: Warn when a deprecated iRules command is used."""
    dialect = active_dialect()
    if dialect != "f5-irules":
        return []
    spec = REGISTRY.get(cmd_name, dialect)
    if spec is not None and spec.deprecated_replacement is not None:
        replacement = spec.deprecated_replacement_name
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=f"'{cmd_name}' is deprecated in iRules. Use '{replacement}' instead.",
                severity=Severity.WARNING,
                code="IRULE2002",
            )
        ]
    return []


# IRULE2003: Unsafe iRules command


@diag("IRULE2003", "Unsafe iRules command.", section="irules")
def check_unsafe_irules_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """IRULE2003: Warn when an unsafe iRules command is used."""
    if active_dialect() != "f5-irules":
        return []
    if REGISTRY.is_unsafe(cmd_name):
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=f"'{cmd_name}' is unsafe in iRules and may allow context escalation",
                severity=Severity.ERROR,
                code="IRULE2003",
            )
        ]
    return []


# IRULE3102: HTTP::path / HTTP::uri / HTTP::query should use -normalized


@diag(
    "IRULE3102",
    "`HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized`.",
    section="irules_security",
)
def check_irules_unnormalized_http_getter(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """IRULE3102: Warn when HTTP URI/path/query getters omit ``-normalized``."""
    if active_dialect() != "f5-irules":
        return []
    if cmd_name not in normalized_flag_commands():
        return []
    # Already using -normalized — nothing to warn about.
    if "-normalized" in args:
        return []
    spec = REGISTRY.get(cmd_name, "f5-irules")
    if spec is None:
        return []
    form = spec.resolve_form(tuple(args))
    if form is None or form.kind is not FormKind.GETTER:
        return []

    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                f"Use '{cmd_name} -normalized' for canonicalized request data; "
                "non-normalized values may allow URL evasion patterns."
            ),
            severity=Severity.WARNING,
            code="IRULE3102",
        )
    ]


# IRULE1002: Unknown iRules event


@diag("IRULE1002", "Unknown iRules event name.", section="irules")
def check_unknown_irules_event(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """IRULE1002: Warn when ``when`` references an unknown iRules event name."""
    if cmd_name != "when" or active_dialect() != "f5-irules":
        return []
    if not args or not arg_tokens:
        return []

    tok = arg_tokens[0]
    event_name = args[0]

    # Only validate literal event names (skip $var / [cmd]).
    if tok.type in (TokenType.VAR, TokenType.CMD):
        return []

    from ...commands.registry.namespace_data import EVENT_PROPS

    if event_name in EVENT_PROPS:
        return []

    return [
        Diagnostic(
            range=range_from_token(tok),
            message=f"Unknown iRules event '{event_name}'. Check the event name spelling.",
            severity=Severity.WARNING,
            code="IRULE1002",
        )
    ]


# W307: Non-literal command name


@diag(
    "W307",
    "Non-literal command name — variable or command substitution as command.",
    section="security",
)
def check_non_literal_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W307: Warn when a command name is a command substitution.

    Variable-as-command (``$obj method``) is deferred to the analyser's
    post-analysis pass which checks the type lattice for TclOO objects.
    """
    cmd_tok = all_tokens[0] if all_tokens else None
    if cmd_tok is not None and cmd_tok.type is TokenType.CMD:
        return [
            Diagnostic(
                range=range_from_token(cmd_tok),
                message="Non-literal command name \u2014 cannot statically analyze",
                severity=Severity.WARNING,
                code="W307",
            )
        ]
    return []


# W306: Literal-expected position contains substitution


@diag("W306", "Substitution in literal-expected argument position.", section="security")
def check_literal_expected(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W306: Warn when a literal-expected argument contains substitution.

    Certain command arguments (regexp patterns, class names) should be
    literal values.  Unbraced ``$`` or ``[`` in these positions can
    cause unexpected substitution or injection.
    """
    diagnostics: list[Diagnostic] = []

    # regexp / regsub: pattern argument should be literal
    if cmd_name in ("regexp", "regsub"):
        pattern_indices = arg_indices_for_role(cmd_name, args, ArgRole.PATTERN)
        for idx in sorted(pattern_indices):
            if idx >= len(args) or idx >= len(arg_tokens):
                continue
            tok = arg_tokens[idx]
            text = args[idx]
            if _first_token_is_braced(tok):
                continue
            if not _has_substitution(text, tok):
                continue
            # A bare single ``$var``/``${var}`` is the canonical idiom for
            # a parameterised regex pattern.  There is no equivalent
            # ``{...}``-braced form (bracing would suppress substitution),
            # so flagging it would be a false positive (issue #235).  Bare
            # command substitutions like ``[a-z]`` are intentionally not
            # exempt — that case looks like a regex character class but is
            # parsed as command substitution, which is exactly the foot-gun
            # W306 is meant to catch.
            if _is_bare_single_var_substitution(tok, source):
                continue
            is_quoted = _tok_is_quoted(tok, source)
            is_var = tok.type == TokenType.VAR or "$" in text
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"Literal expected in {cmd_name} pattern \u2014 found "
                        + ("'$'" if is_var else "'['")
                        + (
                            ". Use braces '{...}' instead of quotes."
                            if is_quoted
                            else ". Use braces '{...}' to prevent substitution."
                        )
                    ),
                    severity=Severity.WARNING,
                    code="W306",
                )
            )

    # class match / class search: class name argument should be literal
    if cmd_name == "class" and args:
        sub = args[0]
        if sub in ("match", "search"):
            # class match ?options? ?--? item operator class_obj → LAST arg
            # class search ?options? ?--? class_name operator string → 3rd-from-last
            # Skip the subcommand (args[0]) and options (-value, -nocase, --).
            first_positional = 1
            while first_positional < len(args) and (
                args[first_positional].startswith("-") or args[first_positional] == "--"
            ):
                first_positional += 1
                if first_positional > 1 and args[first_positional - 1] == "--":
                    break
            if sub == "match":
                # class_obj is the last positional argument
                cls_idx = len(args) - 1
            else:
                # class search: class_name is the first positional argument
                cls_idx = first_positional
            if cls_idx < len(args) and cls_idx < len(arg_tokens):
                tok = arg_tokens[cls_idx]
                text = args[cls_idx]
                if (
                    not _first_token_is_braced(tok)
                    and _has_substitution(text, tok)
                    and not _is_bare_single_var_substitution(tok, source)
                ):
                    diagnostics.append(
                        Diagnostic(
                            range=range_from_token(tok),
                            message=(
                                f"Literal expected for class name in 'class {sub}' \u2014 "
                                "found substitution. Use braces or a literal class name."
                            ),
                            severity=Severity.WARNING,
                            code="W306",
                        )
                    )

    return diagnostics
