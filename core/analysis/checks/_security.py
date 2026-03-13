"""Security and injection checks (W1xx, W3xx)."""

from __future__ import annotations

import re

from ...commands.registry import REGISTRY
from ...common.ranges import range_from_token
from ...parsing.tokens import Token, TokenType
from ..semantic_model import Diagnostic, Severity
from ._helpers import (
    _find_regex_patterns_in_command,
    _first_token_is_braced,
    _has_substitution,
    _parse_subst_flags,
)

# W101: eval with string concatenation


def check_eval_string_concat(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W101: Warn when eval is used with arguments that look like string-built commands.

    eval concatenates its args and re-parses as Tcl; if any argument contains
    variable or command substitutions, the result may be an injection vector.
    Prefer {*}$cmdList or direct invocation.
    """
    if cmd_name != "eval":
        return []
    if not args or not arg_tokens:
        return []

    diagnostics: list[Diagnostic] = []

    # If every argument is a braced string, that's fine (eval {script})
    if all(_first_token_is_braced(tok) for tok in arg_tokens):
        return []

    # Check if any token in the command is a VAR or CMD substitution
    # (skip the first token which is "eval" itself)
    has_substitution = any(
        tok.type in (TokenType.VAR, TokenType.CMD)
        for tok in all_tokens[1:]  # skip "eval" token
    )

    if has_substitution:
        diagnostics.append(
            Diagnostic(
                range=range_from_token(arg_tokens[0]),
                message=(
                    "eval with substituted arguments risks code injection. "
                    "Prefer direct invocation or {*}$cmdList to preserve "
                    "argument boundaries."
                ),
                severity=Severity.WARNING,
                code="W101",
            )
        )

    return diagnostics


# W102: subst on variable input


def check_subst_injection(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W102: Warn when subst is used with a variable argument.

    subst performs $ and [] substitutions on its argument.  If the argument
    comes from a variable (especially user input), this enables arbitrary
    command execution.  Prefer [format] or [string map].
    """
    if cmd_name != "subst":
        return []
    if not args or not arg_tokens:
        return []

    template_idx, nocommands, novariables, _ = _parse_subst_flags(args)

    if template_idx is None or template_idx >= len(arg_tokens):
        return []

    tok = arg_tokens[template_idx]

    # If the template argument is a variable, the variable content
    # will be evaluated by subst
    if tok.type == TokenType.VAR:
        if nocommands and novariables:
            # With both -nocommands -novariables, only backslash
            # substitution remains -- low risk.
            return []
        mitigations: list[str] = []
        if not nocommands:
            mitigations.append("-nocommands")
        if not novariables:
            mitigations.append("-novariables")
        mitigation_text = (
            f" Add {' '.join(mitigations)} to limit substitution scope,"
            " or use [format] / [string map] for safe templating."
        )
        return [
            Diagnostic(
                range=range_from_token(tok),
                message=(
                    "subst with a variable argument enables code injection: "
                    "any "
                    + ("" if nocommands else "[cmd]")
                    + (" and " if not nocommands and not novariables else "")
                    + ("" if novariables else "$var")
                    + " in the string will be evaluated."
                    + mitigation_text
                ),
                severity=Severity.WARNING,
                code="W102",
            )
        ]

    return []


# W309: eval/uplevel with subst -- double substitution


def check_eval_subst_double_decode(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W309: Warn when eval/uplevel uses subst, creating double substitution.

    Patterns like ``eval [subst $template]`` or ``eval [subst {...}]``
    perform substitution twice: once by subst (expanding $var and [cmd])
    and again by eval (re-parsing the result as Tcl).  This is almost
    always a security vulnerability -- an attacker who controls part of
    the template can inject arbitrary commands that survive the first
    round and execute in the second.
    """
    if cmd_name not in ("eval", "uplevel"):
        return []
    if not args or not arg_tokens:
        return []

    # Look for [subst ...] in any argument token
    diagnostics: list[Diagnostic] = []
    for tok in all_tokens[1:]:  # skip the eval/uplevel token itself
        if tok.type != TokenType.CMD:
            continue
        inner = tok.text.strip()
        if inner.startswith("subst ") or inner.startswith("subst\t") or inner == "subst":
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"{cmd_name} with [subst] creates double substitution: "
                        "subst expands $var and [cmd], then "
                        f"{cmd_name} re-parses the result as Tcl. "
                        "This is a code-injection risk. "
                        "Use [format] or [string map] for safe templating."
                    ),
                    severity=Severity.ERROR,
                    code="W309",
                )
            )
            break  # one diagnostic per command is enough

    return diagnostics


# W301: uplevel with string-built script


def check_uplevel_injection(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W301: Warn when uplevel is used with unbraced script arguments.

    uplevel evaluates a script in a caller's scope.  Like eval, passing
    string-concatenated arguments risks injection.
    """
    if cmd_name != "uplevel":
        return []
    if not args or not arg_tokens:
        return []

    # uplevel ?level? script ?arg ...?
    # Skip the optional level argument
    script_idx = 0
    if args[0].lstrip("#").isdigit() or args[0] == "#0":
        script_idx = 1

    if script_idx >= len(args) or script_idx >= len(arg_tokens):
        return []

    # If there are extra args after the script, uplevel concatenates them
    # (just like eval) -- that's the danger
    remaining_args = args[script_idx:]
    remaining_toks = arg_tokens[script_idx:]

    if len(remaining_args) > 1:
        # Multiple args = concat behaviour = danger
        has_substitution = any(t.type in (TokenType.VAR, TokenType.CMD) for t in all_tokens)
        if has_substitution:
            return [
                Diagnostic(
                    range=range_from_token(remaining_toks[0]),
                    message=(
                        "uplevel with multiple arguments concatenates them into "
                        "a script (like eval). Use a single braced body or "
                        "{*}$cmdList to avoid injection."
                    ),
                    severity=Severity.WARNING,
                    code="W301",
                )
            ]
    elif remaining_args:
        # Single arg -- check if it's unbraced with substitutions
        tok = remaining_toks[0]
        if not _first_token_is_braced(tok):
            has_substitution = any(
                t.type in (TokenType.VAR, TokenType.CMD)
                for t in all_tokens[1:]  # skip "uplevel" token
            )
            if has_substitution:
                return [
                    Diagnostic(
                        range=range_from_token(tok),
                        message=(
                            "uplevel with an unbraced script argument may cause "
                            "double substitution. Use braces: uplevel 1 {...}"
                        ),
                        severity=Severity.WARNING,
                        code="W301",
                    )
                ]

    return []


# W103: open with pipeline (|) from variable


def check_open_pipeline(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W103: Warn when open is used with a pipeline specification or variable.

    open "|cmd" executes a command pipeline.  If the argument is built from
    user input, this is a command-injection vector.
    """
    if cmd_name != "open":
        return []
    if not args or not arg_tokens:
        return []

    tok = arg_tokens[0]
    text = args[0]

    # Check if the open argument starts with |
    if text.startswith("|"):
        # If any token in arg 0 is VAR or CMD, it's dangerous
        has_subst = any(
            t.type in (TokenType.VAR, TokenType.CMD)
            for t in all_tokens[1:]  # tokens after "open"
        )
        if has_subst:
            return [
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        "open with a pipeline containing variable/command "
                        "substitution risks command injection. Validate and "
                        "sanitize the command before passing to open."
                    ),
                    severity=Severity.WARNING,
                    code="W103",
                )
            ]
        # Even a literal pipeline is worth a hint for awareness
        return [
            Diagnostic(
                range=range_from_token(tok),
                message=(
                    'open with a pipeline ("|") executes an external command. '
                    "Ensure the command is not influenced by untrusted input."
                ),
                severity=Severity.HINT,
                code="W103",
            )
        ]

    # If the arg is a variable that could contain |...
    if tok.type == TokenType.VAR:
        return [
            Diagnostic(
                range=range_from_token(tok),
                message=(
                    "open with a variable argument: if the value starts with "
                    '"|", it will execute a command pipeline. Validate input '
                    "or use explicit I/O commands."
                ),
                severity=Severity.WARNING,
                code="W103",
            )
        ]

    return []


# W300: source with variable argument


def check_source_variable(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W300: Warn when source uses a variable argument.

    source executes a file as Tcl code.  If the path comes from user input
    or an untrusted variable, this is a code-execution vector.
    """
    if cmd_name != "source":
        return []
    if not args or not arg_tokens:
        return []

    # Skip -encoding flag
    file_idx = 0
    if args[0] == "-encoding" and len(args) >= 3:
        file_idx = 2

    if file_idx >= len(args) or file_idx >= len(arg_tokens):
        return []

    tok = arg_tokens[file_idx]

    if tok.type == TokenType.VAR:
        return [
            Diagnostic(
                range=range_from_token(tok),
                message=(
                    "source with a variable path executes arbitrary Tcl code. "
                    "Ensure the path is not influenced by untrusted input."
                ),
                severity=Severity.WARNING,
                code="W300",
            )
        ]

    return []


# W303: regexp with suspicious quantifier nesting (ReDoS)

_REDOS_PATTERN = re.compile(
    r"""
    (?:                 # Match nested quantifiers like (a+)+ or (a*)*
        [+*]           # Inner quantifier
        \)             # Close group
        [+*{]          # Outer quantifier (+ * or {n,m})
    )
    |
    (?:                 # Overlapping alternation: (a|a)+ etc.
        \([^)]*\|[^)]*\)
        [+*{]
    )
    """,
    re.VERBOSE,
)


def check_redos(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W303: Warn when regexp uses patterns susceptible to catastrophic backtracking."""
    diagnostics: list[Diagnostic] = []
    for pattern, tok in _find_regex_patterns_in_command(cmd_name, args, arg_tokens):
        if _REDOS_PATTERN.search(pattern):
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        "Regular expression may be vulnerable to catastrophic "
                        "backtracking (ReDoS). Nested quantifiers like (a+)+ can "
                        "cause exponential matching time on crafted input."
                    ),
                    severity=Severity.WARNING,
                    code="W303",
                )
            )
    return diagnostics


# W310: Hardcoded credentials

# Patterns that look like API keys, tokens, passwords, or base64 secrets.
_CREDENTIAL_RE = re.compile(
    r"""(?x)
    # Bearer / Basic auth tokens
    (?:Bearer|Basic)\s+[A-Za-z0-9+/=_-]{8,}
    |
    # Common API-key patterns (hex or base64, 16+ chars)
    (?:^|[\s"'=])
    [A-Za-z0-9+/=_-]{32,}
    (?:$|[\s"'])
    """,
)

# Generic fallback: option flags that commonly carry secrets.  Applied only
# when the command has no ``credential_options`` in the registry.
_DEFAULT_PASSWORD_OPTIONS = frozenset({"-password", "-pass", "-secret", "-token", "-apikey"})


def _is_literal_value(tok: Token, text: str) -> bool:
    """Return True if *tok*/*text* is a literal (not a variable/command sub)."""
    return (
        tok.type not in (TokenType.VAR, TokenType.CMD)
        and not text.startswith("$")
        and "[" not in text
    )


def check_hardcoded_credentials(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W310: Warn when a literal string looks like a hardcoded credential.

    Flags bearer/basic tokens, long base64 strings, and passwords supplied
    directly as string literals to network or authentication commands.
    """
    if not args or not arg_tokens:
        return []

    diagnostics: list[Diagnostic] = []

    # Strategy 1: password/secret option flags with literal values.
    # Always check generic heuristic set; also check command-specific options.
    registry_opts = REGISTRY.credential_options(cmd_name)
    cred_opts = (
        _DEFAULT_PASSWORD_OPTIONS | registry_opts if registry_opts else _DEFAULT_PASSWORD_OPTIONS
    )
    for i, text in enumerate(args):
        if text.lower() in cred_opts and i + 1 < len(args):
            val_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
            if val_tok is not None and _is_literal_value(val_tok, args[i + 1]):
                diagnostics.append(
                    Diagnostic(
                        range=range_from_token(val_tok),
                        message=(
                            f"Hardcoded credential in {text} argument. "
                            "Store secrets in environment variables or a vault, "
                            "not in source code."
                        ),
                        severity=Severity.WARNING,
                        code="W310",
                    )
                )
                return diagnostics

    # Strategy 2: subcommand-level credential detection via registry.
    if len(args) >= 3:
        sub = args[0].lower()
        cred_arg, sensitive = REGISTRY.subcommand_credential_info(cmd_name, sub)
        if cred_arg is not None and sensitive is not None:
            header_name = args[1].lower() if len(args) > 1 else ""
            if header_name in sensitive and cred_arg < len(arg_tokens):
                val_tok = arg_tokens[cred_arg]
                val = args[cred_arg]
                if _is_literal_value(val_tok, val):
                    diagnostics.append(
                        Diagnostic(
                            range=range_from_token(val_tok),
                            message=(
                                f"Hardcoded credential in {header_name} header value. "
                                "Store secrets in environment variables or a vault, "
                                "not in source code."
                            ),
                            severity=Severity.WARNING,
                            code="W310",
                        )
                    )
                    return diagnostics

    return diagnostics


# W312: interp eval / interp invokehidden injection


def check_interp_eval_injection(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W312: Warn when interp eval/invokehidden uses dynamic script arguments.

    ``interp eval $child $script`` has the same injection risks as
    ``eval`` -- the script argument is parsed and executed as Tcl code.
    """
    if cmd_name != "interp":
        return []
    if not args or args[0] not in ("eval", "invokehidden"):
        return []

    sub = args[0]

    if sub == "eval":
        # interp eval path arg ?arg ...?
        if len(args) < 3 or len(arg_tokens) < 3:
            return []
        # Script arguments start at index 2 (after subcommand + path)
        script_args = args[2:]
        script_toks = arg_tokens[2:]
    elif sub == "invokehidden":
        # interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?
        if len(args) < 3 or len(arg_tokens) < 3:
            return []
        # Skip options, find the hidden command name
        i = 2
        while i < len(args) and args[i].startswith("-"):
            i += 1
        if i >= len(args):
            return []
        # Arguments after the hidden command name
        script_args = args[i:]
        script_toks = arg_tokens[i:] if i < len(arg_tokens) else []
    else:
        return []

    if not script_args or not script_toks:
        return []

    # Multiple arguments -> concat behaviour (like eval)
    if sub == "eval" and len(script_args) > 1:
        has_substitution = any(t.type in (TokenType.VAR, TokenType.CMD) for t in all_tokens[1:])
        if has_substitution:
            return [
                Diagnostic(
                    range=range_from_token(script_toks[0]),
                    message=(
                        f"interp {sub} with multiple arguments concatenates "
                        "them into a script (like eval). Use a single braced "
                        "body to avoid injection."
                    ),
                    severity=Severity.WARNING,
                    code="W312",
                )
            ]

    # Single script argument -- check if unbraced with substitutions
    tok = script_toks[0]
    if not _first_token_is_braced(tok):
        has_substitution = any(t.type in (TokenType.VAR, TokenType.CMD) for t in all_tokens[1:])
        if has_substitution:
            return [
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"interp {sub} with an unbraced script argument may "
                        "cause code injection. Use braces: "
                        f"interp {sub} $child {{...}}"
                    ),
                    severity=Severity.WARNING,
                    code="W312",
                )
            ]

    return []


# W313: Destructive file operations with tainted path


def check_destructive_file_ops(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W313: Warn when destructive file operations use variable paths.

    ``file delete``, ``file rename``, and ``file mkdir`` with a variable
    path argument risk path-traversal attacks.
    """
    destructive = REGISTRY.destructive_subcommands(cmd_name)
    if not destructive:
        return []
    if not args or args[0] not in destructive:
        return []

    sub = args[0]
    diagnostics: list[Diagnostic] = []

    # Skip -force / -- options
    path_start = 1
    while path_start < len(args) and args[path_start] in ("-force", "--"):
        path_start += 1

    for i in range(path_start, len(args)):
        if i >= len(arg_tokens):
            break
        tok = arg_tokens[i]
        text = args[i]

        # Check for variable or command substitution in path
        if tok.type in (TokenType.VAR, TokenType.CMD) or _has_substitution(text, tok):
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        f"file {sub} with a variable path risks path-traversal. "
                        "Validate the path with [file normalize] and verify it "
                        "stays within the intended directory."
                    ),
                    severity=Severity.WARNING,
                    code="W313",
                )
            )
            break  # one diagnostic per command

    return diagnostics
