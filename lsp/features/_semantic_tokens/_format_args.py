from __future__ import annotations

import logging
import re

from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_positions import token_content_base
from compiler.parsing.tokens import Token, TokenType
from core.commands.registry.command_registry import REGISTRY
from core.commands.registry.runtime import (
    SIGNATURES,
    SubcommandSig,
    options_with_value,
    regexp_pattern_index,
    skip_options,
)
from shared.dialect import active_dialect
from shared.ranges import position_from_offset

from ._constants import (
    _BINARY_FORMAT_SPECIFIERS,
    _BINARY_INT_SPECIFIERS,
    _MOD_INDEX,
    _TYPE_INDEX,
)
from ._primitives import (
    _append_text_token,
)

log = logging.getLogger(__name__)


def _split_words(
    source: str,
    *,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> tuple[list[str], list[Token]]:
    """Split Tcl words and return (word_texts, first_token_per_word)."""
    lexer = TclLexer(source, base_offset=base_offset, base_line=base_line, base_col=base_col)
    words: list[str] = []
    word_tokens: list[Token] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL):
            prev_type = tok.type
            continue

        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(tok.text)
            word_tokens.append(tok)
        elif words:
            words[-1] += tok.text
        else:
            words.append(tok.text)
            word_tokens.append(tok)
        prev_type = tok.type

    return words, word_tokens


def _collect_param_list_tokens(
    out: list[tuple[int, int, int, int, int]],
    param_token: Token,
) -> bool:
    """Emit semantic parameter tokens from a Tcl parameter-list argument."""
    if param_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    base_offset, base_line, base_col = token_content_base(param_token)
    words, word_tokens = _split_words(
        param_token.text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )

    emitted = False
    for word, tok in zip(words, word_tokens):
        param_name = word
        param_start = tok.start

        # Braced parameter form: {name ?default?}
        if tok.type is TokenType.STR:
            inner_offset, inner_line, inner_col = token_content_base(tok)
            inner_words, inner_tokens = _split_words(
                tok.text,
                base_offset=inner_offset,
                base_line=inner_line,
                base_col=inner_col,
            )
            if not inner_words or not inner_tokens:
                continue
            param_name = inner_words[0]
            param_start = inner_tokens[0].start

        if not param_name:
            continue

        _append_text_token(
            out,
            start=param_start,
            text=param_name,
            type_idx=_TYPE_INDEX["parameter"],
            modifiers=1 << _MOD_INDEX["declaration"],
        )
        emitted = True

    return emitted


def _proc_param_list_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a formal parameter list, if known."""
    if cmd_name in ("proc", "method"):
        return 2
    if cmd_name == "constructor":
        return 1
    if cmd_name == "classmethod" and len(argv_texts) >= 4:
        return 2  # classmethod name argList bodyScript
    if cmd_name == "self" and len(argv_texts) >= 2:
        if argv_texts[1] == "method":
            return 3
        if argv_texts[1] == "constructor":
            return 2
    return None


# Tcl ARE/ERE/BRE regex components for sub-tokenization.
# Matches metacharacters, character classes, escape sequences, quantifiers,
# groups, anchors, and backreferences.
_REGEX_PART_RE = re.compile(
    r"(?:"
    r"\(\?[imnsxwpq]*(?:[-imnsxwpq]*)?\)"  # (?flags) embedded flags
    r"|\(\?(?:[:=!>])"  # non-capturing / lookahead / lookbehind group open
    r"|\("  # group open
    r"|\)"  # group close
    r"|\[(?:\^)?\]?(?:[^\]\\]|\\.)*\]"  # character class [...]
    r"|\\[AbBdDmMsSwWyYZ]"  # ARE class shortcuts
    r"|\\[0-9]"  # backreference
    r"|\\[.*+?(){}\[\]|^$\\]"  # escaped metachar
    r"|\\[aefnrtv]"  # escape sequences
    r"|\\x[0-9a-fA-F]{1,2}"  # hex escape
    r"|\\u[0-9a-fA-F]{1,4}"  # unicode escape
    r"|\\U[0-9a-fA-F]{1,8}"  # wide unicode escape
    r"|[*+?](?:\?)?|\\{\\d+(?:,\\d*)?\\}"  # quantifiers
    r"|\{(?:\d+)(?:,\d*)?\}"  # bounded quantifier {n,m}
    r"|[|]"  # alternation
    r"|[\^$]"  # anchors
    r"|[.]"  # any-char
    r")"
)


def _collect_regex_pattern_tokens(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Sub-tokenize a regex pattern into its components.

    Returns True when at least one sub-token was emitted.
    """
    if tok.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = tok.text
    matches = list(_REGEX_PART_RE.finditer(text))
    if not matches:
        # No metacharacters — just a literal. Emit as single regexp token.
        return False

    base_offset, _base_line, _base_col = token_content_base(tok)
    pos_in_text = 0

    for match in matches:
        # Literal text before this metacharacter
        if match.start() > pos_in_text:
            before_start = position_from_offset(
                base_offset + pos_in_text,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["regexp"],
            )

        matched = match.group()
        meta_start = position_from_offset(
            base_offset + match.start(),
            line_starts,
            source_len,
        )

        # Classify the component using dedicated ARE token types
        if matched.startswith("["):
            # Character class [...]
            type_idx = _TYPE_INDEX["regexpCharClass"]
        elif matched.startswith("\\") and len(matched) >= 2:
            ch = matched[1]
            if ch.isdigit():
                # Backreference \0–\9
                type_idx = _TYPE_INDEX["regexpBackref"]
            elif ch in "aefnrtv" or ch == "x" or ch == "u" or ch == "U":
                # Escape sequence \n \t \xHH etc.
                type_idx = _TYPE_INDEX["regexpEscape"]
            elif ch in "dDsSwW":
                # ARE class shortcut (\d, \s, \w and negated)
                type_idx = _TYPE_INDEX["regexpCharClass"]
            elif ch in "bBmMyYAZ":
                # ARE anchors (\b, \B, \m, \M, \y, \Y, \A, \Z)
                type_idx = _TYPE_INDEX["regexpAnchor"]
            else:
                # Escaped metachar
                type_idx = _TYPE_INDEX["regexpEscape"]
        elif matched in ("^", "$"):
            type_idx = _TYPE_INDEX["regexpAnchor"]
        elif matched in ("(", ")") or matched.startswith("(?"):
            type_idx = _TYPE_INDEX["regexpGroup"]
        elif matched in ("|",):
            type_idx = _TYPE_INDEX["regexpAlternation"]
        elif matched == ".":
            # Any-char dot is a character class
            type_idx = _TYPE_INDEX["regexpCharClass"]
        elif matched in ("*", "+", "?", "*?", "+?", "??"):
            type_idx = _TYPE_INDEX["regexpQuantifier"]
        elif matched.startswith("{") and matched.endswith("}"):
            # Bounded quantifier {n,m}
            type_idx = _TYPE_INDEX["regexpQuantifier"]
        else:
            type_idx = _TYPE_INDEX["regexpQuantifier"]

        _append_text_token(
            out,
            start=meta_start,
            text=matched,
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    # Remaining literal text
    if pos_in_text < len(text):
        rest_start = position_from_offset(
            base_offset + pos_in_text,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["regexp"],
        )

    return True


def _emit_regex_token(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> None:
    """Emit semantic tokens for a regex pattern, with sub-tokenization."""
    if _collect_regex_pattern_tokens(out, tok, line_starts=line_starts, source_len=source_len):
        return
    # Fallback: emit as a single regexp token
    rendered = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
    _append_text_token(
        out,
        start=tok.start,
        text=rendered,
        type_idx=_TYPE_INDEX["regexp"],
    )


def _procedure_name_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a procedure/method name definition."""
    if cmd_name == "proc" and len(argv_texts) >= 2:
        return 1
    if cmd_name in ("method", "classmethod") and len(argv_texts) >= 2:
        return 1
    if cmd_name in ("oo::define", "oo::objdefine") and len(argv_texts) >= 4:
        if argv_texts[2] in ("method", "classmethod"):
            return 3
        if argv_texts[2] == "self" and len(argv_texts) >= 5 and argv_texts[3] == "method":
            return 4
    if cmd_name == "self" and len(argv_texts) >= 3 and argv_texts[1] == "method":
        return 2
    return None


def _binary_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a binary format string argument."""
    if cmd_name != "binary" or len(argv_texts) < 3:
        return None
    if argv_texts[1] == "format":
        return 2
    if argv_texts[1] == "scan" and len(argv_texts) >= 4:
        return 3
    return None


def _string_map_mapping_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the string map mapping list."""
    if cmd_name != "string" or len(argv_texts) < 4:
        return None
    if argv_texts[1] != "map":
        return None
    i = 2
    if i < len(argv_texts) and argv_texts[i] == "-nocase":
        i += 1
    return i if i < len(argv_texts) else None


def _collect_string_map_pairs_tokens(
    out: list[tuple[int, int, int, int, int]],
    mapping_token: Token,
) -> bool:
    """Tokenise string map mapping list with alternating pair colours.

    Each key-value pair gets a distinct colour that alternates between
    two token types so the user can visually associate keys with their
    replacement values.
    """
    if mapping_token.type is not TokenType.STR:
        return False

    base_offset, base_line, base_col = token_content_base(mapping_token)
    elements, element_tokens = _split_words(
        mapping_token.text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )

    if len(elements) < 2:
        return False

    pair_types = [_TYPE_INDEX["string"], _TYPE_INDEX["parameter"]]

    for idx, (elem, tok) in enumerate(zip(elements, element_tokens)):
        pair_num = idx // 2
        type_idx = pair_types[pair_num & 1]
        rendered = f"{{{elem}}}" if tok.type is TokenType.STR else elem
        _append_text_token(
            out,
            start=tok.start,
            text=rendered,
            type_idx=type_idx,
        )

    return True


def _subcommand_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a known subcommand token."""
    sig = SIGNATURES.get(cmd_name)
    if not isinstance(sig, SubcommandSig):
        return None
    if len(argv_texts) < 2:
        return None
    return 1


def _is_known_subcommand(cmd_name: str, sub_name: str) -> bool:
    """Return True when *sub_name* is a known subcommand for *cmd_name*."""
    sig = SIGNATURES.get(cmd_name)
    if not isinstance(sig, SubcommandSig):
        return False
    return sub_name in sig.subcommands or sig.allow_unknown


def _sprintf_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a format/scan format string argument."""
    if cmd_name == "format" and len(argv_texts) >= 2:
        return 1
    if cmd_name == "scan" and len(argv_texts) >= 3:
        return 2
    return None


# Tcl format/scan specifier sequence (simplified)
# %[position$][flags][width][.precision][length_modifier]type
# The \\?\$ handles both raw $ and Tcl-escaped \$ in double-quoted strings.
_SPRINTF_RE = re.compile(
    r"%(?:(?P<position>\d+)\\?\$)?(?P<flags>[-+ 0#]*)?(?P<width>\*|\d+)?(?:.(?P<precision>\*|\d+))?(?P<length>[hlLzq])?(?P<type>[aAbBcdieEfgGosuxX%])"
)


def _collect_sprintf_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Tokenise format/scan specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_SPRINTF_RE.finditer(text))
    if not matches:
        return False

    base_offset, _base_line, _base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_offset(base_offset + pos_in_text, line_starts, source_len)
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        pos = match.start()

        def emit_part(end: int, tidx: int) -> None:
            nonlocal pos
            if end > pos:
                part_pos = position_from_offset(base_offset + pos, line_starts, source_len)
                _append_text_token(out, start=part_pos, text=text[pos:end], type_idx=tidx)
                pos = end

        # The '%'. It's always 1 character
        emit_part(match.start() + 1, _TYPE_INDEX["formatPercent"])

        # position (digits)
        if match.span("position")[0] != -1:
            emit_part(match.end("position"), _TYPE_INDEX["formatWidth"])
            # The '$' follows the position
            emit_part(match.end("position") + 1, _TYPE_INDEX["formatPercent"])

        # flags (+- 0#)
        if match.span("flags")[0] != -1:
            emit_part(match.end("flags"), _TYPE_INDEX["formatFlag"])

        # width (digits or *)
        if match.span("width")[0] != -1:
            tid = (
                _TYPE_INDEX["formatWidth"]
                if text[match.start("width")].isdigit()
                else _TYPE_INDEX["formatFlag"]
            )
            emit_part(match.end("width"), tid)

        # precision (starts with . then digits or *)
        if match.span("precision")[0] != -1:
            # The '.' is a flag/punctuation
            emit_part(match.start("precision"), _TYPE_INDEX["formatFlag"])
            tid = (
                _TYPE_INDEX["formatWidth"]
                if text[match.start("precision")].isdigit()
                else _TYPE_INDEX["formatFlag"]
            )
            emit_part(match.end("precision"), tid)

        # length modifier (h l L z q)
        if match.span("length")[0] != -1:
            emit_part(match.end("length"), _TYPE_INDEX["formatFlag"])

        # type specifier (s d f etc.)
        if match.span("type")[0] != -1:
            emit_part(match.end("type"), _TYPE_INDEX["formatSpec"])

        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_offset(base_offset + pos_in_text, line_starts, source_len)
        _append_text_token(
            out, start=rest_start, text=text[pos_in_text:], type_idx=_TYPE_INDEX["string"]
        )

    return True


def _clock_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a clock format string argument.

    The format string is the VALUE of the ``-format`` option in
    ``clock format $t -format "%Y-%m-%d"`` or ``clock scan $s -format "%Y-%m-%d"``.
    """
    if cmd_name != "clock" or len(argv_texts) < 3:
        return None
    if argv_texts[1] not in ("format", "scan"):
        return None
    for i in range(2, len(argv_texts)):
        if argv_texts[i] == "-format" and i + 1 < len(argv_texts):
            return i + 1
    return None


# Tcl clock format specifiers: %[E|O]<letter> or %%
_CLOCK_FORMAT_RE = re.compile(r"%(?:[EO])?[aAbBcCdDeEgGhHIjJklmMNOpPqQsSuUVwWxXyYzZ%]")


def _collect_clock_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Tokenise clock format specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_CLOCK_FORMAT_RE.finditer(text))
    if not matches:
        return False

    base_offset, _base_line, _base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_offset(
                base_offset + pos_in_text,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        # Emit the % as clockPercent
        pct_pos = position_from_offset(
            base_offset + match.start(),
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=pct_pos,
            text="%",
            type_idx=_TYPE_INDEX["clockPercent"],
        )

        # After % there may be an E/O locale modifier then the specifier letter
        spec_text = match.group()[1:]  # strip leading %
        if spec_text:
            off = match.start() + 1
            if spec_text[0] in ("E", "O") and len(spec_text) > 1:
                # Emit locale modifier separately
                mod_start = position_from_offset(
                    base_offset + off,
                    line_starts,
                    source_len,
                )
                _append_text_token(
                    out,
                    start=mod_start,
                    text=spec_text[0],
                    type_idx=_TYPE_INDEX["clockModifier"],
                )
                off += 1
                spec_text = spec_text[1:]
            # Emit the specifier letter
            if spec_text:
                spec_start = position_from_offset(
                    base_offset + off,
                    line_starts,
                    source_len,
                )
                _append_text_token(
                    out,
                    start=spec_start,
                    text=spec_text,
                    type_idx=_TYPE_INDEX["clockSpec"],
                )

        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_offset(
            base_offset + pos_in_text,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _regsub_subspec_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the regsub substitution spec.

    ``regsub ?switches? exp string subSpec ?varName?``
    """
    if cmd_name != "regsub" or len(argv_texts) < 4:
        return None
    pat_idx = regexp_pattern_index(argv_texts[1:])
    if pat_idx is None:
        return None
    # pattern is at pat_idx+1 in argv; subspec is pattern+2
    subspec_idx = pat_idx + 1 + 2
    if subspec_idx < len(argv_texts):
        return subspec_idx
    return None


def _regex_pattern_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the regex pattern argument.

    Works for both ``regexp`` and ``regsub``:
        regexp ?switches? exp string ?matchVar ...?
        regsub ?switches? exp string subSpec ?varName?
    The pattern (*exp*) is the first positional arg after option switches.
    """
    if cmd_name not in ("regexp", "regsub"):
        return None
    if len(argv_texts) < 3:
        return None
    pat_idx = regexp_pattern_index(argv_texts[1:])
    if pat_idx is None:
        return None
    return pat_idx + 1


# Regsub substitution backreferences: \0-\9, \&
_REGSUB_BACKREF_RE = re.compile(r"\\([0-9&])")


def _collect_regsub_subspec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Tokenise regsub substitution spec backreferences."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_REGSUB_BACKREF_RE.finditer(text))
    if not matches:
        return False

    base_offset, _base_line, _base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_offset(
                base_offset + pos_in_text,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        ref_start = position_from_offset(
            base_offset + match.start(),
            line_starts,
            source_len,
        )
        ref_char = match.group(1)
        # \& and \0 = whole match → operator; \1-\9 = capture group → number
        type_idx = _TYPE_INDEX["operator"] if ref_char == "&" else _TYPE_INDEX["number"]
        _append_text_token(
            out,
            start=ref_start,
            text=match.group(),
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_offset(
            base_offset + pos_in_text,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _glob_pattern_arg_indices(cmd_name: str, argv_texts: list[str]) -> set[int]:
    """Return argv indices (absolute) containing glob pattern arguments.

    Handles ``string match``, ``glob``, and ``lsearch`` (default/glob mode).
    """
    if cmd_name == "string" and len(argv_texts) >= 4 and argv_texts[1] == "match":
        i = 2
        while i < len(argv_texts) and argv_texts[i].startswith("-"):
            if argv_texts[i] == "--":
                i += 1
                break
            i += 1
        return {i} if i < len(argv_texts) else set()

    if cmd_name == "glob":
        i = skip_options(argv_texts[1:], options_with_value("glob")) + 1
        return set(range(i, len(argv_texts)))

    if cmd_name == "lsearch":
        has_regexp = any(a == "-regexp" for a in argv_texts)
        has_exact = any(a == "-exact" for a in argv_texts)
        if has_regexp or has_exact:
            return set()
        if len(argv_texts) >= 3:
            return {len(argv_texts) - 1}
        return set()

    return set()


# Glob metacharacters: *, ?, [...], \x
_GLOB_META_RE = re.compile(
    r"\\."  # escaped character
    r"|\[[^\]]*\]"  # character class [...]
    r"|\*"  # match any string
    r"|\?"  # match any single character
)


def _collect_glob_pattern_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Tokenise glob pattern metacharacters."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_GLOB_META_RE.finditer(text))
    if not matches:
        return False

    base_offset, _base_line, _base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_offset(
                base_offset + pos_in_text,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        meta_start = position_from_offset(
            base_offset + match.start(),
            line_starts,
            source_len,
        )
        matched = match.group()
        if matched.startswith("\\"):
            type_idx = _TYPE_INDEX["escape"]
        elif matched.startswith("["):
            type_idx = _TYPE_INDEX["regexp"]
        else:
            type_idx = _TYPE_INDEX["operator"]
        _append_text_token(
            out,
            start=meta_start,
            text=matched,
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_offset(
            base_offset + pos_in_text,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _option_arg_indices(cmd_name: str, argv_texts: list[str]) -> set[int]:
    """Return arg indices (0-based after command name) that are option flags.

    Uses ``resolve_option_terminator()`` to identify where options start,
    walking args that begin with ``-`` until ``--`` or the first positional
    argument.
    """
    profile = REGISTRY.resolve_option_terminator(cmd_name, argv_texts)
    if profile is None:
        return set()

    result: set[int] = set()
    i = profile.scan_start
    while i < len(argv_texts):
        arg = argv_texts[i]
        if arg == "--":
            break
        if not arg.startswith("-"):
            break
        result.add(i)
        if arg in profile.options_with_values and i + 1 < len(argv_texts):
            i += 1  # skip the option's value argument
        i += 1
    return result


def _collect_binary_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Tokenise binary format/scan specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    base_offset, _base_line, _base_col = token_content_base(spec_token)
    text = spec_token.text
    i = 0
    emitted = False

    while i < len(text):
        ch = text[i]
        if ch in " \t\r\n":
            i += 1
            continue

        count_start = i
        while i < len(text) and text[i].isdigit():
            i += 1
        if i > count_start:
            count_pos = position_from_offset(
                base_offset + count_start,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=count_pos,
                text=text[count_start:i],
                type_idx=_TYPE_INDEX["binaryCount"],
            )
            emitted = True

        if i >= len(text):
            break

        spec = text[i]
        if spec not in _BINARY_FORMAT_SPECIFIERS:
            i += 1
            continue

        spec_pos = position_from_offset(
            base_offset + i,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=spec_pos,
            text=spec,
            type_idx=_TYPE_INDEX["binarySpec"],
        )
        emitted = True
        i += 1

        # Signed/unsigned modifier suffix (Tcl 8.5+): e.g. su, iu
        if (
            i < len(text)
            and text[i] in ("u", "s")
            and spec in _BINARY_INT_SPECIFIERS
            and active_dialect() not in ("tcl8.4", "f5")
        ):
            mod_pos = position_from_offset(
                base_offset + i,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=mod_pos,
                text=text[i],
                type_idx=_TYPE_INDEX["binaryFlag"],
            )
            emitted = True
            i += 1

        if i < len(text) and text[i] == "*":
            star_pos = position_from_offset(
                base_offset + i,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=star_pos,
                text="*",
                type_idx=_TYPE_INDEX["binaryFlag"],
            )
            emitted = True
            i += 1

    return emitted
