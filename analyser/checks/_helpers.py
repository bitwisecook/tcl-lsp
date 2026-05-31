"""Shared helper utilities for check functions."""

from __future__ import annotations

import re

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.expr_lexer import ExprTokenType, tokenise_expr
from compiler.parsing.lexer import TclLexer
from compiler.registry import REGISTRY
from compiler.registry.command_registry import ResolvedTerminator
from compiler.registry.dialect import active_dialect
from compiler.registry.runtime import regexp_pattern_index
from shared.ranges import range_from_token
from shared.tokens import SourcePosition, Token, TokenType

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
) -> ResolvedTerminator | None:
    """Find the matching option-terminator profile from the registry."""
    return REGISTRY.resolve_option_terminator(cmd_name, args)


def _first_positional_without_terminator(
    args: list[str],
    profile: ResolvedTerminator,
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


def _raw_has_live_substitution(raw: str) -> bool:
    """Return True if *raw* (the **source** slice of a token, backslashes
    intact) contains a *live* ``$`` / ``[`` substitution — i.e. one not
    backslash-escaped and, for ``$``, actually introducing a variable name.

    In a quoted/literal regexp pattern, ``\\[`` and ``\\$`` are literal
    characters (a regex character-class bracket, a literal dollar) — NOT a
    command/variable substitution.  tclsh-verified: ``regexp "\\[abc\\]+" …``
    matches the literal pattern ``[abc]+`` with no substitution, whereas an
    *unescaped* ``regexp "[abc]+" …`` triggers a command substitution of
    ``abc`` (errors ``invalid command name "abc"``) — the real foot-gun W306
    catches.  The resolved token text can't tell these apart (an unresolved
    cmd-sub resolves to empty), so the check passes the raw source here.

    Tcl rules applied to the raw slice:
      * ``\\X`` is a literal — both characters are skipped.
      * ``[`` (unescaped) is always a command substitution.
      * ``$`` is a substitution only when followed by a variable-name start
        (``[A-Za-z0-9_]``, ``{``, or ``::``).  A ``$`` before a quote, end of
        string, or other punctuation — e.g. the regex end-anchor in
        ``"(.*)$"`` — is a *literal* dollar (tclsh: ``regexp "(.*)$" …`` runs
        cleanly with no substitution).
    """
    i = 0
    n = len(raw)
    while i < n:
        ch = raw[i]
        if ch == "\\":
            i += 2  # the next char is escaped (literal) — skip both
            continue
        if ch == "[":
            return True
        if ch == "$":
            nxt = raw[i + 1] if i + 1 < n else ""
            if nxt and (nxt.isalnum() or nxt in "_{:"):
                return True
            # bare ``$`` (end-anchor / literal) — not a substitution
        i += 1
    return False


def _is_bare_single_var_substitution(tok: Token, source: str) -> bool:
    """Return True if *tok* spans exactly a bare ``$var`` or ``${var}``.

    This identifies the canonical Tcl idiom for parameterising an argument
    with a single variable's value, with no surrounding literal text or
    quotes.  For these cases there is no equivalent ``{...}``-braced form
    (bracing would suppress the substitution), so flagging substitution-
    in-literal-expected-position would be a false positive.

    Bare command substitutions (``[cmd]``) are intentionally **not**
    exempted: a literal like ``[a-z]`` looks like a regex character class
    but is parsed by Tcl as a command substitution, and that confusion is
    exactly what W306 is meant to catch.

    Note: ``tok.end.offset`` typically points at the last character of the
    substitution name and excludes the trailing delimiter (``}``), so the
    closing delimiter is checked separately at ``end + 1``.
    """
    if tok.type is not TokenType.VAR:
        return False
    start = tok.start.offset
    end = tok.end.offset + 1
    if start < 0 or end > len(source) or end <= start:
        return False
    word = source[start:end]
    if word == "$" + tok.text:
        return True
    if word == "${" + tok.text and end < len(source) and source[end] == "}":
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

    Stops the backward scan at any ``proc NAME {PARAMS} BODY`` declaration
    that *shadows* the search: if the use offset is inside the proc body
    AND ``var_name`` appears in the proc's parameter list, the outer scope
    is irrelevant and we must NOT attribute an outer ``set`` to the inner
    parameter use.  (PR #498 deep-review finding 9 / G13:
    ``set path -force; proc useit {path} { file delete $path }`` was
    wrongly attributing the outer ``path = -force`` to the inner param.)
    """  # noqa: D205
    if not var_name or before_offset <= 0:
        return None

    from compiler.tcl_expr_eval import _split_tcl_list

    for cmd in reversed(segment_commands(source[:before_offset])):
        # Cross-scope guard: if we encounter a proc whose body *contains*
        # the use offset AND whose params include var_name, the
        # parameter shadows any outer scope -- stop searching.
        # The proc body contains the use offset iff the proc command's
        # full range is INCOMPLETE in the truncated source[:before_offset]
        # view (i.e. the proc's closing brace is past the use, so the
        # segmenter saw a partial command).  When the proc command's
        # end offset is <= before_offset, the entire proc body has been
        # truncated-and-shown; the use comes AFTER the proc and the
        # parameter does NOT shadow.  (PR #498 deep-review follow-up
        # finding 7: top-level ``$path`` after ``proc p {path}`` must
        # use the top-level ``set path -force`` evidence.)
        body_offset_end = cmd.range.end.offset
        use_inside_proc = body_offset_end >= before_offset
        if (
            use_inside_proc
            and cmd.texts
            and cmd.texts[0] == "proc"
            and len(cmd.texts) >= 4
        ):
            # cmd.texts[2] is the param-list literal -- the segmenter
            # has already brace-stripped braced words so this is the
            # *contents* (e.g. ``'a b {c default}'``).
            param_text = cmd.texts[2]
            # Quick prefilter: var_name must appear in the param text.
            if var_name in param_text:
                # Parse via the shared Tcl list splitter (handles
                # braces, quotes, escapes).  Each list element is
                # either a bare name or ``{name default}``; for the
                # latter the splitter returns ``"name default"`` (the
                # outer braces stripped), so the param name is the
                # first whitespace-separated word.
                try:
                    elements = _split_tcl_list(param_text)
                except Exception:
                    elements = []
                param_names = []
                for el in elements:
                    s = el.strip()
                    if not s:
                        continue
                    # Split off the optional default value.
                    name = s.split(None, 1)[0]
                    param_names.append(name)
                if var_name in param_names:
                    # Use offset is INSIDE this proc body iff the proc's
                    # start is before us and its end is after us.  Since
                    # we truncated at ``before_offset``, the proc's body
                    # is incomplete in the truncated view; the use is
                    # inside the proc by construction (segment_commands
                    # placed this proc as one of our reversed commands).
                    # Stop the scan -- the param shadows outer scope.
                    return None

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


# Regexes/constants for _reconstruct_word_from_tokens
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
