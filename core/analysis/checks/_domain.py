"""Domain-specific and iRules checks."""

from __future__ import annotations

from ...commands.registry import REGISTRY
from ...commands.registry.models import DialectStatus, FormKind
from ...commands.registry.runtime import (
    ArgRole,
    arg_indices_for_role,
)
from ...common.dialect import active_dialect
from ...common.ranges import range_from_token
from ...parsing.tokens import Token, TokenType
from ..semantic_model import Diagnostic, Severity
from ._helpers import (
    _first_token_is_braced,
    _has_substitution,
    _tok_is_quoted,
)

# W002: Disabled command in active dialect


def check_disabled_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W002: Warn when a command is disabled in the active dialect profile."""
    dialect = active_dialect()
    status = REGISTRY.command_status(cmd_name, dialect)
    if status is DialectStatus.DISALLOWED:
        msg = f"'{cmd_name}' is disabled in the active dialect profile"
        # If the command exists in iRules, suggest selecting that dialect.
        if cmd_name == "when":
            msg += ". Select iRules as the language to enable F5 iRules support"
        elif REGISTRY.command_status(cmd_name, "f5-irules") is DialectStatus.EXISTS:
            msg += " (available in the iRules dialect)"
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=msg,
                severity=Severity.WARNING,
                code="W002",
            )
        ]
    return []


# IRULE2002: Deprecated iRules command


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

# Only these commands actually support the -normalized flag.
_NORMALIZABLE_HTTP_COMMANDS = frozenset(
    {
        "HTTP::uri",
        "HTTP::path",
        "HTTP::query",
    }
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
    if cmd_name not in _NORMALIZABLE_HTTP_COMMANDS:
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


def check_non_literal_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W307: Warn when a command name is a variable or command substitution."""
    cmd_tok = all_tokens[0] if all_tokens else None
    if cmd_tok is not None and cmd_tok.type in (TokenType.VAR, TokenType.CMD):
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
                    severity=Severity.ERROR,
                    code="W306",
                )
            )

    # class match / class search: class name argument should be literal
    if cmd_name == "class" and args:
        sub = args[0]
        if sub in ("match", "search"):
            # Skip options after subcommand to find class name arg.
            # class match ?options? ?--? item operator class_obj
            # The class object is the last argument.
            if len(args) >= 4:
                cls_idx = len(args) - 1
                if cls_idx < len(arg_tokens):
                    tok = arg_tokens[cls_idx]
                    text = args[cls_idx]
                    if not _first_token_is_braced(tok) and _has_substitution(text, tok):
                        diagnostics.append(
                            Diagnostic(
                                range=range_from_token(tok),
                                message=(
                                    f"Literal expected for class name in 'class {sub}' \u2014 "
                                    "found substitution. Use braces or a literal class name."
                                ),
                                severity=Severity.ERROR,
                                code="W306",
                            )
                        )

    return diagnostics
