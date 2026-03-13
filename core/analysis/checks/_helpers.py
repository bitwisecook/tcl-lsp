"""Shared helper utilities for check functions."""

from __future__ import annotations

import re

from ...commands.registry import REGISTRY
from ...commands.registry.models import OptionTerminatorSpec
from ...commands.registry.runtime import regexp_pattern_index
from ...common.dialect import active_dialect
from ...common.ranges import range_from_token
from ...parsing.command_segmenter import segment_commands
from ...parsing.expr_lexer import ExprTokenType, tokenise_expr
from ...parsing.lexer import TclLexer
from ...parsing.tokens import SourcePosition, Token, TokenType
from ..semantic_model import CodeFix, Range


def _pos_in_cmd_text(tok: Token, text_idx: int) -> SourcePosition:
    """Compute full-file SourcePosition for ``tok.text[text_idx]``."""
    line = tok.start.line
    col = tok.start.character + 1  # +1 for the opening [
    for c in tok.text[:text_idx]:
        if c == "\n":
            line += 1
            col = 0
        else:
            col += 1
    return SourcePosition(line=line, character=col, offset=tok.start.offset + 1 + text_idx)


def _tok_is_quoted(tok: Token, source: str) -> bool:
    """Return True if the token started with a double-quote in source."""
    offset = tok.start.offset
    return offset < len(source) and source[offset] == '"'


def _first_token_is_braced(tok: Token) -> bool:
    """Check if the first token in an argument position is a braced string."""
    return tok.type == TokenType.STR


# Option-terminator profiles -- loaded from the command registry


def _resolve_option_terminator_profile(
    cmd_name: str,
    args: list[str],
) -> tuple[OptionTerminatorSpec | None, str | None]:
    """Find the matching option-terminator profile from the registry.

    Returns ``(profile, subcommand)`` where *subcommand* is the matched
    subcommand word (or ``None`` for top-level profiles).
    """
    profiles = REGISTRY.option_terminator_profiles(cmd_name)
    if not profiles:
        return None, None
    # Check subcommand-scoped profiles first.
    if args:
        for profile in profiles:
            if profile.subcommand is not None and profile.subcommand == args[0]:
                return profile, args[0]
    # Fall back to top-level profile (subcommand=None).
    for profile in profiles:
        if profile.subcommand is None:
            return profile, None
    return None, None


def _first_positional_without_terminator(
    args: list[str],
    profile: OptionTerminatorSpec,
) -> int | None:
    """Return first positional arg index when '--' is missing, else None."""
    i = profile.scan_start
    while i < len(args):
        arg = args[i]
        if arg == "--":
            return None
        if arg.startswith("-"):
            i += 1
            if arg in profile.options_with_values and i < len(args):
                i += 1
            continue
        return i
    return None


def _has_substitution(text: str, tok: Token | None = None) -> bool:
    """Return True if *text* contains substitution (``$``, ``[``, or VAR/CMD token type)."""
    if "$" in text or "[" in text:
        return True
    if tok is not None and tok.type in (TokenType.VAR, TokenType.CMD):
        return True
    return False


def _rewrite_string_compare_ops(expr_text: str) -> str:
    """Rewrite expr operators for string comparison diagnostics."""
    _EXPR_EQ_RE = re.compile(r"(?<![=!])==(?!=)")
    _EXPR_NE_RE = re.compile(r"!=")
    rewritten = _EXPR_EQ_RE.sub(" eq ", expr_text)
    rewritten = _EXPR_NE_RE.sub(" ne ", rewritten)
    return re.sub(r"[ \t]{2,}", " ", rewritten)


def _is_safe_literal(text: str) -> bool:
    """Check if text is a simple literal that doesn't need bracing."""
    # Pure numeric
    try:
        float(text)
        return True
    except ValueError:
        pass
    # Boolean constants
    if text.lower() in ("true", "false", "yes", "no", "on", "off"):
        return True
    return False


def _is_safe_literal_expr(text: str) -> bool:
    """Check whether an expr string is substitution-free numeric/boolean text."""
    if _is_safe_literal(text):
        return True
    if "$" in text or "[" in text:
        return False

    allowed = {
        ExprTokenType.NUMBER,
        ExprTokenType.BOOL,
        ExprTokenType.OPERATOR,
        ExprTokenType.PAREN_OPEN,
        ExprTokenType.PAREN_CLOSE,
        ExprTokenType.WHITESPACE,
        ExprTokenType.TERNARY_Q,
        ExprTokenType.TERNARY_C,
        ExprTokenType.COMMA,
    }
    tokens = tokenise_expr(text, dialect=active_dialect())
    if not tokens:
        return False
    return all(tok.type in allowed for tok in tokens)


def _last_literal_set_value_for_var(
    source: str,
    var_name: str,
    *,
    before_offset: int,
) -> tuple[str, Range] | None:
    """Return most recent literal ``set var value`` before *before_offset*.

    If the latest assignment is dynamic/non-literal, return ``None`` because
    the runtime value cannot be proven statically.
    """
    if not var_name or before_offset <= 0:
        return None

    for cmd in reversed(segment_commands(source[:before_offset])):
        if not cmd.texts or cmd.texts[0] != "set" or len(cmd.texts) < 3:
            continue
        if cmd.texts[1] != var_name:
            continue

        # Most recent assignment wins. If it's dynamic, value is unknown.
        if len(cmd.single_token_word) < 3 or not cmd.single_token_word[2]:
            return None
        if len(cmd.argv) < 3:
            return None
        value_tok = cmd.argv[2]
        if value_tok.type not in (TokenType.ESC, TokenType.STR):
            return None
        return cmd.texts[2], range_from_token(value_tok)

    return None


def _first_arg_name(args: list[str]) -> list[int]:
    """set, incr, append, lappend -- first arg is the variable name."""
    return [0] if args else []


def _unset_name_args(args: list[str]) -> list[int]:
    """unset ?-nocomplain? ?--? varName ?varName ...?"""
    start = 0
    for i, a in enumerate(args):
        if a == "--":
            start = i + 1
            break
        if a.startswith("-"):
            start = i + 1
            continue
        start = i
        break
    return list(range(start, len(args)))


def _info_exists_arg(args: list[str]) -> list[int]:
    """info exists varName -- only the ``exists`` subcommand takes a name."""
    if len(args) >= 2 and args[0] == "exists":
        return [1]
    return []


def _upvar_local_name_args(args: list[str]) -> list[int]:
    """upvar ?level? otherVar myVar ?otherVar myVar ...?

    The *local* binding names (myVar positions) must be plain names.
    If the first arg looks like a level (``#N`` or a digit), skip it.
    Then every odd-indexed arg (0-based from the remaining) is a local name.
    """
    if not args:
        return []
    start = 0
    if args[0].lstrip("-").isdigit() or args[0].startswith("#"):
        start = 1
    # local names are at start+1, start+3, start+5, ...
    return list(range(start + 1, len(args), 2))


def _stray_brace_fix(tok: Token, source: str) -> CodeFix | None:
    """Build a CodeFix that removes a stray ``}`` and its enclosing line.

    Returns *None* when the ``}`` shares a line with other code (not safe to
    delete the whole line).
    """
    # Locate the line boundaries around the token.
    prev_nl = source.rfind("\n", 0, tok.start.offset)
    line_content_start = prev_nl + 1 if prev_nl >= 0 else 0

    next_nl = source.find("\n", tok.end.offset + 1)
    line_end_off = next_nl + 1 if next_nl >= 0 else len(source)

    # Only auto-fix if the line contains nothing but optional whitespace + '}'.
    line_text = source[line_content_start:line_end_off]
    if line_text.strip() != "}":
        return None

    # Preferred: delete from line start through trailing '\n' (keeps the
    # previous line's newline intact).  Fallback when there is no trailing
    # '\n': delete the preceding '\n' through EOF instead.
    if next_nl >= 0:
        del_start = SourcePosition(
            line=tok.start.line,
            character=0,
            offset=line_content_start,
        )
        del_end = SourcePosition(
            line=tok.start.line + 1,
            character=0,
            offset=next_nl + 1,
        )
    elif prev_nl >= 0:
        del_start = SourcePosition(
            line=tok.start.line - 1,
            character=prev_nl - source.rfind("\n", 0, prev_nl) - 1,
            offset=prev_nl,
        )
        del_end = SourcePosition(
            line=tok.start.line,
            character=line_end_off - line_content_start,
            offset=line_end_off,
        )
    else:
        # Only line in the file.
        del_start = SourcePosition(line=0, character=0, offset=0)
        del_end = SourcePosition(
            line=0,
            character=line_end_off,
            offset=line_end_off,
        )

    return CodeFix(
        range=Range(start=del_start, end=del_end),
        new_text="",
        description="Remove extra '}'",
    )


def _find_bracket_insertion_point(
    cmd_name: str,
    all_tokens: list[Token],
    arg_tokens: list[Token],
    bracket_tok_index: int,
) -> SourcePosition | None:
    """Find where the missing ``[`` should be inserted.

    Uses two heuristics in priority order:

    1. **Known command name**: scan backward from the ``]`` token for an ESC
       token whose text matches a registered command name.
    2. **Arity overflow**: if the enclosing command has a bounded max arity
       and the actual argument count exceeds it, the ``[`` should go before
       the last expected argument position (e.g. ``set`` expects 2 args, so
       the ``[`` goes before the second argument).
    """
    known = REGISTRY.command_names()
    tok = all_tokens[bracket_tok_index]
    bracket_offset = tok.start.offset + max(tok.text.find("]"), 0)

    # Heuristic 1a: text before ']' in the same token is a command name.
    bracket_idx = tok.text.find("]")
    prefix = tok.text[:bracket_idx] if bracket_idx > 0 else ""
    if prefix in known:
        return tok.start

    # Heuristic 1b: backward scan for a known command name (skip cmd at [0]).
    for i in range(bracket_tok_index - 1, 0, -1):
        t = all_tokens[i]
        if t.type is TokenType.ESC and t.text in known:
            return t.start

    # Heuristic 2: arity overflow -- enclosing command has bounded max arity.
    validation = REGISTRY.validation(cmd_name)
    if validation is not None and not validation.arity.is_unlimited:
        max_args = validation.arity.max
        if len(arg_tokens) > max_args >= 1:
            # The [ should go before the last expected argument position.
            # arg_tokens excludes the command name, so index (max_args - 1)
            # is the start of the last expected argument.
            insert_tok = arg_tokens[max_args - 1]
            if insert_tok.start.offset < bracket_offset:
                return insert_tok.start

    return None


def _parse_subst_flags(args: list[str]) -> tuple[int | None, bool, bool, bool]:
    """Parse subst flags, return (template_idx, nocommands, novariables, nobackslashes)."""
    nocommands = False
    novariables = False
    nobackslashes = False
    template_idx = None
    for i, text in enumerate(args):
        if text == "-nocommands":
            nocommands = True
        elif text == "-novariables":
            novariables = True
        elif text == "-nobackslashes":
            nobackslashes = True
        elif text.startswith("-"):
            continue
        else:
            template_idx = i
            break
    return template_idx, nocommands, novariables, nobackslashes


# Regexes and constants used by _build_file_join_fix / _reconstruct_word_from_tokens
_SIMPLE_PATH_SEGMENT_RE = re.compile(r"^[A-Za-z0-9_.-]+$")
_SIMPLE_PATH_VAR_RE = re.compile(
    r"^\$(?:\{[A-Za-z_][A-Za-z0-9_:]*\}|[A-Za-z_][A-Za-z0-9_:]*)$",
)


def _build_file_join_fix(path_expr: str) -> str | None:
    """Build a conservative `[file join ...]` replacement when safe."""
    text = path_expr.strip()
    if not text:
        return None
    if (text.startswith('"') and text.endswith('"')) or (
        text.startswith("{") and text.endswith("}")
    ):
        text = text[1:-1]
    if not text or any(ch in text for ch in "[];"):
        return None
    if " " in text:
        return None

    parts = [part for part in re.split(r"[/\\\\]+", text) if part]
    if len(parts) < 2:
        return None

    for part in parts:
        if _SIMPLE_PATH_VAR_RE.fullmatch(part):
            continue
        if _SIMPLE_PATH_SEGMENT_RE.fullmatch(part):
            continue
        return None

    return "[file join " + " ".join(parts) + "]"


def _reconstruct_word_from_tokens(tokens: list[Token]) -> str:
    """Reconstruct a Tcl word from token pieces (including substitutions)."""
    pieces: list[str] = []
    for tok in tokens:
        if tok.type is TokenType.VAR:
            pieces.append(f"${tok.text}")
        elif tok.type is TokenType.CMD:
            pieces.append(f"[{tok.text}]")
        else:
            pieces.append(tok.text)
    return "".join(pieces)


def _find_regex_patterns_in_command(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
) -> list[tuple[str, Token]]:
    """Return (pattern_text, pattern_token) pairs for regex patterns in a command.

    Handles ``regexp``, ``regsub`` (first non-option arg) and ``switch -regexp``
    (all pattern arguments in the pattern/body pairs).
    """
    if not args or not arg_tokens:
        return []

    if cmd_name in ("regexp", "regsub"):
        i = regexp_pattern_index(args)
        if i is not None and i < len(arg_tokens):
            return [(args[i], arg_tokens[i])]
        return []

    if cmd_name == "switch":
        # Check for -regexp flag among options
        is_regexp = False
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "-regexp":
                is_regexp = True
            if args[i] == "--":
                i += 1
                break
            i += 1
        if not is_regexp:
            return []
        # Skip the string argument
        i += 1

        results: list[tuple[str, Token]] = []

        if i < len(args) and i == len(args) - 1:
            # Form 2: single braced case list -- re-lex to find pattern/body pairs
            case_tok = arg_tokens[i] if i < len(arg_tokens) else None
            if case_tok is not None:
                base_off = case_tok.start.offset + 1
                base_line = case_tok.start.line
                base_col = case_tok.start.character + 1
                lexer = TclLexer(
                    args[i],
                    base_offset=base_off,
                    base_line=base_line,
                    base_col=base_col,
                )
                elements: list[str] = []
                element_tokens: list[Token] = []
                prev = TokenType.EOL
                while True:
                    tok = lexer.get_token()
                    if tok is None:
                        break
                    if tok.type in (TokenType.SEP, TokenType.EOL):
                        prev = tok.type
                        continue
                    if prev in (TokenType.SEP, TokenType.EOL):
                        elements.append(tok.text)
                        element_tokens.append(tok)
                    elif elements:
                        elements[-1] += tok.text
                    else:
                        elements.append(tok.text)
                        element_tokens.append(tok)
                    prev = tok.type

                j = 0
                while j + 1 < len(elements):
                    if elements[j] != "default" and j < len(element_tokens):
                        results.append((elements[j], element_tokens[j]))
                    j += 2
        else:
            # Form 1: inline pattern/body pairs
            while i + 1 < len(args):
                if args[i] != "default" and i < len(arg_tokens):
                    results.append((args[i], arg_tokens[i]))
                i += 2

        return results

    return []
