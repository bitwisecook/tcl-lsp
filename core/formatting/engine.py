"""Core formatting engine: structural parse + reconstruction.

Parses Tcl source into commands, identifies body arguments via the SIGNATURES
registry, recursively formats bodies, and reconstructs the output.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum, auto

from core.commands.registry import REGISTRY
from core.commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    body_arg_indices,
    expr_arg_indices,
)
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType

from .config import FormatterConfig, IndentStyle

# Structural model


class ArgKind(Enum):
    """What kind of argument this is for formatting purposes."""

    WORD = auto()  # plain argument -- reconstruct from tokens
    BODY = auto()  # script body -- recursively format, wrap in {}
    KEYWORD = auto()  # structural keyword (else, elseif, then, finally, on, trap)
    PARAM_LIST = auto()  # parameter list -- normalize internal whitespace


@dataclass
class CommandArg:
    """A single argument to a command."""

    kind: ArgKind
    tokens: list[Token]
    text: str  # concatenated token text (stripped of delimiters by lexer)
    is_braced: bool  # first token was STR type (i.e. {wrapped})
    is_quoted: bool  # argument was "quoted"
    formatted_body: str | None = None


@dataclass
class ParsedCommand:
    """A single Tcl command with its arguments, ready for reformatting."""

    name: str
    args: list[CommandArg]  # args[0] is the command name
    preceding_comments: list[str]
    preceding_blank_lines: int


# Token → raw source reconstruction


def _reconstruct_raw(tok: Token) -> str:
    """Rebuild source text from a single token, re-adding delimiters."""
    match tok.type:
        case TokenType.STR:
            return "{" + tok.text + "}"
        case TokenType.CMD:
            # Normalise backslash-newline continuations to plain spaces.
            # Inside [], \<newline> is just whitespace in Tcl, so this is
            # semantics-preserving and keeps formatting idempotent.
            text = tok.text
            if "\\\n" in text:
                text = re.sub(r"[ \t]*\\\n[ \t]*", " ", text)
            return "[" + text + "]"
        case TokenType.VAR:
            return "$" + tok.text
        case _:
            return tok.text


def _reconstruct_arg(arg: CommandArg) -> str:
    """Rebuild the source text of an argument from its tokens."""
    raw = "".join(_reconstruct_raw(t) for t in arg.tokens)
    if arg.is_quoted:
        return '"' + raw + '"'
    return raw


def _normalize_param_list(arg: CommandArg) -> str:
    """Normalize whitespace in a braced parameter list.

    Each parameter (possibly a {name default} pair) is separated by a
    single space and leading/trailing whitespace inside the braces is
    removed.  For example ``{a    b   c}`` becomes ``{a b c}`` and
    ``{a  {b default}}`` becomes ``{a {b default}}``.
    """
    # The lexer gives us the text *inside* the braces.  We need to
    # split it into top-level list elements (respecting nested braces)
    # and rejoin with single spaces.
    text = arg.text
    elements: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        # skip whitespace between elements
        if text[i] in (" ", "\t", "\n", "\r"):
            i += 1
            continue
        if text[i] == "{":
            # scan for matching close brace
            level = 1
            start = i
            i += 1
            while i < n and level > 0:
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            elements.append(text[start:i])
        else:
            # bare word
            start = i
            while i < n and text[i] not in (" ", "\t", "\n", "\r"):
                i += 1
            elements.append(text[start:i])
    return "{" + " ".join(elements) + "}"


# Command parsing


def _count_newlines(text: str) -> int:
    """Count newlines in an EOL token's text (for blank line tracking)."""
    # An EOL token contains newlines, semicolons, whitespace.
    # Each newline beyond the first represents a blank line.
    nc = text.count("\n")
    return max(0, nc - 1)


def parse_commands(source: str) -> tuple[list[ParsedCommand], list[str]]:
    """Parse Tcl source into a list of structured commands.

    Walks the TclLexer token stream, groups tokens into commands
    (same pattern as Analyser._analyse_body), and tracks preceding
    comments and blank lines.

    Returns (commands, trailing_comments) where trailing_comments are
    any comments after the last command.
    """
    lexer = TclLexer(source)
    commands: list[ParsedCommand] = []
    pending_comments: list[str] = []
    pending_blank_lines = 0

    # Per-argument accumulation
    argv_tokens: list[list[Token]] = []  # tokens per argument
    argv_texts: list[str] = []  # concatenated text per argument
    argv_braced: list[bool] = []  # whether first token was STR
    argv_quoted: list[bool] = []  # whether argument was quoted
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break

        match tok.type:
            case TokenType.COMMENT:
                pending_comments.append(tok.text)
                continue
            case TokenType.SEP:
                prev_type = tok.type
                continue
            case TokenType.EOL:
                if argv_texts:
                    cmd = _build_command(
                        argv_tokens,
                        argv_texts,
                        argv_braced,
                        argv_quoted,
                        pending_comments,
                        pending_blank_lines,
                    )
                    commands.append(cmd)
                    pending_comments = []
                    # Count blank lines in this EOL for the next command
                    pending_blank_lines = _count_newlines(tok.text)
                else:
                    pending_blank_lines += _count_newlines(tok.text)
                argv_tokens = []
                argv_texts = []
                argv_braced = []
                argv_quoted = []
                prev_type = tok.type
                continue
            case _:
                pass

        # Detect quoted context: if this is the start of a new arg and the
        # source character at the token start is a quote mark.
        # The lexer captures start_pos before advancing past the opening quote,
        # so the start offset points at the '"' character itself.
        is_start_of_new_arg = prev_type in (TokenType.SEP, TokenType.EOL)
        detected_quoted = False
        if is_start_of_new_arg and tok.start.offset < len(source):
            if source[tok.start.offset] == '"':
                detected_quoted = True

        if is_start_of_new_arg:
            argv_tokens.append([tok])
            argv_texts.append(tok.text)
            argv_braced.append(tok.type == TokenType.STR)
            argv_quoted.append(detected_quoted)
        else:
            # Concatenation within same argument (e.g. "hello $name")
            if argv_texts:
                argv_tokens[-1].append(tok)
                argv_texts[-1] += tok.text
            else:
                argv_tokens.append([tok])
                argv_texts.append(tok.text)
                argv_braced.append(tok.type == TokenType.STR)
                argv_quoted.append(detected_quoted)

        prev_type = tok.type

    # Handle trailing command without final EOL
    if argv_texts:
        cmd = _build_command(
            argv_tokens,
            argv_texts,
            argv_braced,
            argv_quoted,
            pending_comments,
            pending_blank_lines,
        )
        commands.append(cmd)
        pending_comments = []

    return commands, pending_comments


def _build_command(
    argv_tokens: list[list[Token]],
    argv_texts: list[str],
    argv_braced: list[bool],
    argv_quoted: list[bool],
    preceding_comments: list[str],
    preceding_blank_lines: int,
) -> ParsedCommand:
    """Build a ParsedCommand from accumulated argument data."""
    args: list[CommandArg] = []
    for i, (tokens, text, braced, quoted) in enumerate(
        zip(argv_tokens, argv_texts, argv_braced, argv_quoted)
    ):
        args.append(
            CommandArg(
                kind=ArgKind.WORD,
                tokens=tokens,
                text=text,
                is_braced=braced,
                is_quoted=quoted,
            )
        )

    name = argv_texts[0] if argv_texts else ""
    return ParsedCommand(
        name=name,
        args=args,
        preceding_comments=list(preceding_comments),
        preceding_blank_lines=preceding_blank_lines,
    )


# Body argument identification


def _identify_body_args(cmd: ParsedCommand) -> None:
    """Mark arguments that are script bodies, using the runtime's body_arg_indices.

    Mutates cmd.args in place, setting kind=ArgKind.BODY for body args,
    kind=ArgKind.KEYWORD for structural keywords, and kind=ArgKind.PARAM_LIST
    for parameter lists.
    """
    name = cmd.name
    args = cmd.args[1:]  # skip command name

    # Formatter-specific override: for {init} {expr} {next} {body} --
    # only the main body (arg 3) should expand; init and next stay inline.
    if name == "for" and len(args) >= 4:
        if args[3].is_braced:
            args[3].kind = ArgKind.BODY
        return

    # Get body indices from the canonical runtime implementation
    arg_texts = [arg.text for arg in args]
    body_indices = body_arg_indices(name, arg_texts)
    for idx in body_indices:
        if idx < len(args) and args[idx].is_braced:
            args[idx].kind = ArgKind.BODY

    # Formatter-specific: mark structural keywords for formatting
    if name == "if":
        _identify_if_keywords(args)
    elif name == "try":
        _identify_try_keywords(args)

    # Formatter-specific: identify PARAM_LIST args from signatures
    _identify_param_list_args(name, args)


def _identify_if_keywords(args: list[CommandArg]) -> None:
    """Mark 'then', 'else', 'elseif' as KEYWORD for formatting."""
    for arg in args:
        if arg.text in ("then", "else", "elseif"):
            arg.kind = ArgKind.KEYWORD


def _identify_try_keywords(args: list[CommandArg]) -> None:
    """Mark 'finally', 'on', 'trap' as KEYWORD for formatting."""
    for arg in args:
        if arg.text in ("finally", "on", "trap"):
            arg.kind = ArgKind.KEYWORD


def _identify_param_list_args(name: str, args: list[CommandArg]) -> None:
    """Identify PARAM_LIST args from SIGNATURES registry."""
    sig = SIGNATURES.get(name)
    if isinstance(sig, SubcommandSig) and args:
        sub_sig = sig.subcommands.get(args[0].text)
        if isinstance(sub_sig, CommandSig):
            for idx, role in sub_sig.arg_roles.items():
                actual_idx = idx + 1
                if (
                    role is ArgRole.PARAM_LIST
                    and actual_idx < len(args)
                    and args[actual_idx].is_braced
                ):
                    args[actual_idx].kind = ArgKind.PARAM_LIST
    elif isinstance(sig, CommandSig):
        for idx, role in sig.arg_roles.items():
            if role is ArgRole.PARAM_LIST and idx < len(args) and args[idx].is_braced:
                args[idx].kind = ArgKind.PARAM_LIST


# Indentation helpers


def _make_indent(config: FormatterConfig, level: int) -> str:
    """Build the indentation string for a given nesting level."""
    if config.indent_style == IndentStyle.TABS:
        return "\t" * level
    return " " * (config.indent_size * level)


# Comment formatting


def _format_comment(comment_text: str, config: FormatterConfig) -> str:
    """Format a comment according to config.

    The lexer's COMMENT token text starts with '#' and runs to end of line.
    """
    if not comment_text or comment_text == "#":
        return "#"

    # Check if this looks like commented-out code: #command (no space after #)
    after_hashes = comment_text.lstrip("#")
    num_hashes = len(comment_text) - len(after_hashes)

    if not after_hashes:
        return "#" * num_hashes

    # Preserve commented-out code style: #command (no space)
    is_commented_code = not after_hashes[0].isspace()

    if is_commented_code:
        # Normalise \<newline><whitespace> continuations back to single
        # spaces so the formatter always starts from a canonical single-
        # line form.  This keeps splitting idempotent.
        normalised = re.sub(r"[ \t]*\\\n[ \t]*", " ", after_hashes)
        return "#" * num_hashes + normalised
    else:
        # Preserve existing whitespace between # and content so that
        # commented-out code formatting and ASCII diagrams are not
        # mangled.  Only ensure a minimum single space when the config
        # option is enabled; never collapse intentional extra spacing.
        if config.space_after_comment_hash:
            return "#" * num_hashes + after_hashes.rstrip()
        else:
            content = after_hashes.strip()
            return "#" * num_hashes + content


# Blank line computation


def _compute_blank_lines(
    commands: list[ParsedCommand],
    index: int,
    config: FormatterConfig,
) -> int:
    """Compute how many blank lines to insert before commands[index]."""
    if index == 0:
        return 0

    current = commands[index]
    prev = commands[index - 1]

    # Between proc definitions
    if prev.name == "proc" and current.name == "proc":
        return config.blank_lines_between_procs

    # After proc before non-proc (or non-proc before proc)
    if prev.name == "proc" or current.name == "proc":
        return config.blank_lines_between_blocks

    # Preserve existing blank lines up to max
    actual = current.preceding_blank_lines
    return min(actual, config.max_consecutive_blank_lines)


# Switch body formatting


def _format_switch_body(
    body_text: str,
    config: FormatterConfig,
    indent_level: int,
) -> str:
    """Format the braced body of a switch command.

    The body contains pattern/body pairs. Parse them and format each body
    recursively.
    """
    lexer = TclLexer(body_text)
    elements: list[tuple[list[Token], str, bool]] = []  # (tokens, text, is_braced)
    prev_type = TokenType.EOL

    current_tokens: list[Token] = []
    current_text = ""
    current_braced = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL):
            if current_text:
                elements.append((current_tokens, current_text, current_braced))
                current_tokens = []
                current_text = ""
                current_braced = False
            prev_type = tok.type
            continue
        if tok.type == TokenType.COMMENT:
            continue

        if prev_type in (TokenType.SEP, TokenType.EOL):
            if current_text:
                elements.append((current_tokens, current_text, current_braced))
            current_tokens = [tok]
            current_text = tok.text
            current_braced = tok.type == TokenType.STR
        else:
            current_tokens.append(tok)
            current_text += tok.text

        prev_type = tok.type

    if current_text:
        elements.append((current_tokens, current_text, current_braced))

    # elements should be alternating pattern/body pairs
    indent = _make_indent(config, indent_level)
    inner_indent_level = indent_level + 1
    lines: list[str] = []

    i = 0
    while i < len(elements):
        tokens, text, is_braced = elements[i]
        pattern_raw = "".join(_reconstruct_raw(t) for t in tokens)

        if i + 1 < len(elements):
            body_tokens, body_text, body_braced = elements[i + 1]
            if body_text == "-":
                # Fall-through
                lines.append(f"{indent}{pattern_raw} -")
            elif body_braced:
                formatted_body = format_body(body_text, config, inner_indent_level)
                if formatted_body.strip():
                    lines.append(f"{indent}{pattern_raw} {{")
                    lines.append(formatted_body)
                    lines.append(f"{indent}}}")
                else:
                    lines.append(f"{indent}{pattern_raw} {{}}")
            else:
                body_raw = "".join(_reconstruct_raw(t) for t in body_tokens)
                lines.append(f"{indent}{pattern_raw} {body_raw}")
            i += 2
        else:
            # Odd trailing element
            lines.append(f"{indent}{pattern_raw}")
            i += 1

    return "\n".join(lines)


# Long-line expression wrapping


def _find_expr_break_points(text: str) -> list[tuple[str, int]]:
    """Find positions of top-level ``&&`` and ``||`` in an expression.

    Returns (operator, position) tuples where *position* is the index of
    the first character of the operator in *text*.  Only operators that
    are not nested inside ``[]``, ``{}``, ``()``, or ``""`` are returned.
    """
    breaks: list[tuple[str, int]] = []
    i = 0
    n = len(text)
    depth_bracket = 0
    depth_brace = 0
    depth_paren = 0
    in_quotes = False

    while i < n:
        ch = text[i]
        if ch == "\\":
            i += 2  # skip escaped character
            continue
        if ch == '"' and depth_brace == 0:
            in_quotes = not in_quotes
            i += 1
            continue
        if in_quotes:
            i += 1
            continue
        if ch == "[":
            depth_bracket += 1
        elif ch == "]":
            depth_bracket = max(0, depth_bracket - 1)
        elif ch == "{":
            depth_brace += 1
        elif ch == "}":
            depth_brace = max(0, depth_brace - 1)
        elif ch == "(":
            depth_paren += 1
        elif ch == ")":
            depth_paren = max(0, depth_paren - 1)
        elif depth_bracket == 0 and depth_brace == 0 and depth_paren == 0:
            if i + 1 < n:
                two = text[i : i + 2]
                if two in ("&&", "||"):
                    breaks.append((two, i))
                    i += 2
                    continue
        i += 1

    return breaks


def _wrap_braced_expr(
    text: str,
    config: FormatterConfig,
    start_col: int,
    indent_level: int,
    trailing: int = 0,
) -> str | None:
    """Try to wrap a braced expression at ``&&`` / ``||`` operators.

    *text* is the content inside the braces (no surrounding ``{}``).
    *start_col* is the column of the opening ``{``.
    *trailing* is the estimated length of content after the closing ``}``
    on the same line (e.g. 2 for the `` {`` body opener).

    Returns the wrapped expression text (without braces), or ``None`` if
    wrapping is not possible or would not help.

    The output uses a block style where the opening ``{`` stays on the
    command line and each operand gets its own indented line::

        if {
            $a == 1
            || $b == 2
        } {
            body
        }
    """
    # Normalise: collapse internal newlines and runs of whitespace that
    # may have been introduced by a previous format pass.
    stripped = " ".join(text.split())

    breaks = _find_expr_break_points(stripped)
    if not breaks:
        return None

    # Build chunks: first chunk has no operator prefix; subsequent chunks
    # start with their operator (e.g. "&& rest" or "|| rest").
    chunks: list[str] = []
    last = 0
    for _op, pos in breaks:
        before = stripped[last:pos].rstrip()
        chunks.append(before)
        last = pos
    chunks.append(stripped[last:])

    # Expression content is indented one level deeper than the command;
    # the closing brace sits at the command's own indent level.
    expr_indent = _make_indent(config, indent_level + 1)
    cmd_indent = _make_indent(config, indent_level)

    # Each chunk on its own line.
    inner = "\n".join(expr_indent + chunk for chunk in chunks)
    return "\n" + inner + "\n" + cmd_indent


def _identify_expr_args(cmd: ParsedCommand) -> set[int]:
    """Tag braced arguments that are Tcl expressions (e.g. ``if`` conditions).

    Sets ``arg.kind`` to ``ArgKind.WORD`` (unchanged) but records which
    args are expressions by returning a set of indices into ``cmd.args``.

    We piggy-back on the runtime ``expr_arg_indices`` which knows the
    signatures.
    """
    # expr_arg_indices works on args *after* the command name (index 0).
    arg_texts = [a.text for a in cmd.args[1:]]
    expr_indices = expr_arg_indices(cmd.name, arg_texts)
    # Convert to cmd.args indices (add 1 for command-name slot).
    return {idx + 1 for idx in expr_indices}


def _current_col(parts: list[str], indent_len: int) -> int:
    """Compute current column from accumulated parts."""
    text = "".join(parts)
    last_nl = text.rfind("\n")
    if last_nl == -1:
        return indent_len + len(text)
    return len(text) - last_nl - 1


def _estimate_trailing_len(args: list[CommandArg], start: int) -> int:
    """Estimate the length of content after args[start] on the first line.

    This accounts for things like the `` {`` opening brace of a body arg
    or a ``then`` keyword that follows the expression.
    """
    length = 0
    for arg in args[start:]:
        if arg.kind == ArgKind.BODY:
            # Body opens with " {" on the same line (K&R style).
            length += 2  # " {"
            break
        elif arg.kind == ArgKind.KEYWORD:
            length += 1 + len(arg.text)  # " keyword"
        else:
            # A regular arg follows — add its reconstructed length.
            length += 1 + len(arg.text) + (2 if arg.is_braced else 0)
    return length


# Backslash-continuation line splitting


def _find_splittable_spaces(text: str, start: int = 0) -> list[tuple[int, int]]:
    r"""Find positions of spaces in *text* where ``\``-continuation is safe.

    Returns ``(index, bracket_depth)`` tuples.  Backslash-newline works
    identically inside ``[]`` command substitution as at the top level, so
    spaces inside brackets are included.  Spaces inside ``{}`` or ``""``
    are **not** safe (braces preserve literal text and ``\<newline>``
    inside double-quotes changes the string value).
    """
    spaces: list[tuple[int, int]] = []
    depth_bracket = 0
    depth_brace = 0
    in_quotes = False
    i = start
    n = len(text)

    while i < n:
        ch = text[i]
        if ch == "\\" and i + 1 < n:
            i += 2  # skip escaped character
            continue
        if ch == '"' and depth_brace == 0:
            in_quotes = not in_quotes
            i += 1
            continue
        if in_quotes:
            i += 1
            continue
        if ch == "[":
            depth_bracket += 1
        elif ch == "]":
            depth_bracket = max(0, depth_bracket - 1)
        elif ch == "{":
            depth_brace += 1
        elif ch == "}":
            depth_brace = max(0, depth_brace - 1)
        elif ch == " " and depth_brace == 0:
            spaces.append((i, depth_bracket))
        i += 1

    return spaces


def _find_quoted_string_spaces(text: str, start: int = 0) -> list[int]:
    r"""Find spaces inside double-quoted strings where ``\<newline>`` is safe.

    In Tcl, ``\<newline><whitespace>`` inside double-quotes is replaced by
    a single space.  So if we break at a space in the original string, the
    ``\<newline><indent>`` on the continuation line collapses to one space,
    preserving the string value.

    Only returns spaces that are NOT inside ``[]`` command substitution
    within the string (those would change command boundaries).
    """
    spaces: list[int] = []
    depth_bracket = 0
    depth_brace = 0
    in_quotes = False
    i = start
    n = len(text)

    while i < n:
        ch = text[i]
        if ch == "\\" and i + 1 < n:
            i += 2
            continue
        if not in_quotes:
            if ch == "{":
                depth_brace += 1
            elif ch == "}":
                depth_brace = max(0, depth_brace - 1)
            elif ch == '"' and depth_brace == 0:
                in_quotes = True
                depth_bracket = 0
            elif ch == "[":
                depth_bracket += 1
            elif ch == "]":
                depth_bracket = max(0, depth_bracket - 1)
        else:
            if ch == '"':
                in_quotes = False
            elif ch == "[":
                depth_bracket += 1
            elif ch == "]":
                depth_bracket = max(0, depth_bracket - 1)
            elif ch == " " and depth_bracket == 0:
                spaces.append(i)
        i += 1

    return spaces


def _greedy_split(
    line: str,
    spaces: list[int],
    max_len: int,
    cont_indent: str,
) -> list[str] | None:
    r"""Greedy line splitting at the given space positions.

    Returns the list of visual lines, or ``None`` if no split is possible.
    Lines end with ``" \"`` except the last one.  Continuation lines are
    prefixed with *cont_indent*.
    """
    if not spaces:
        return None

    indent_part = line[: len(line) - len(line.lstrip())]
    segments: list[str] = []
    seg_start = 0
    last_good: int | None = None

    def _commit_break(break_pos: int) -> None:
        """Commit a break at *break_pos* and start a new segment."""
        nonlocal seg_start, last_good
        text = line[seg_start:break_pos]
        if seg_start > 0 and seg_start > len(indent_part):
            text = cont_indent + text
        segments.append(text + " \\")
        seg_start = break_pos + 1
        last_good = None

    for sp in spaces:
        # Compute the visual length if the current segment ends at sp.
        if seg_start == 0:
            seg_len = sp + 2  # original indent is part of line
        else:
            seg_len = len(cont_indent) + (sp - seg_start) + 2

        if seg_len <= max_len:
            last_good = sp
        else:
            if last_good is not None:
                _commit_break(last_good)
                # Re-evaluate current space for the new segment.
                new_len = len(cont_indent) + (sp - seg_start) + 2
                last_good = sp if new_len <= max_len else None
            else:
                # No good break yet — force break here (may exceed limit).
                _commit_break(sp)

    # If we never broke, try the last good space.
    if not segments:
        if last_good is not None:
            _commit_break(last_good)
        else:
            return None

    # Append remainder with continuation indent.
    remainder = line[seg_start:]
    if seg_start > 0 and seg_start > len(indent_part):
        remainder = cont_indent + remainder
    segments.append(remainder)

    return segments


def _split_long_line(
    line: str,
    config: FormatterConfig,
    cont_indent: str,
) -> str | None:
    r"""Split a long line using ``\`` continuation, preferring shallow breaks.

    Uses a recursive strategy: first splits at the shallowest available
    bracket depth, then recursively re-splits any continuation segment
    that is still too long.  This naturally produces clean output where
    outer command arguments are broken first and inner ones only as needed.

    Returns the split text, or ``None`` if the line fits or cannot be split.
    """
    if len(line) <= config.max_line_length:
        return None

    indent_len = len(line) - len(line.lstrip())
    all_spaces = _find_splittable_spaces(line, indent_len)
    max_len = config.max_line_length
    segments: list[str] | None = None

    if all_spaces:
        max_depth = max(d for _, d in all_spaces)

        # Try splitting at the shallowest depth that produces a break.
        for target_depth in range(max_depth + 1):
            spaces = [pos for pos, d in all_spaces if d <= target_depth]
            segments = _greedy_split(line, spaces, max_len, cont_indent)
            if segments is not None and len(segments) > 1:
                break
        else:
            segments = None

    # If no command-level break was possible (or none found), try breaking
    # inside double-quoted strings as a last resort.  In Tcl,
    # \<newline><ws> inside "" collapses to a single space, preserving
    # the string value.
    if segments is None or len(segments) < 2:
        quoted_spaces = _find_quoted_string_spaces(line, indent_len)
        if quoted_spaces:
            # Combine with command-level spaces for optimal splitting.
            all_positions = sorted({pos for pos, _ in all_spaces} | set(quoted_spaces))
            segments = _greedy_split(line, all_positions, max_len, cont_indent)
        if segments is None or len(segments) < 2:
            return None

    # Recursively split any continuation segment still too long.
    final: list[str] = []
    for seg in segments:
        if seg.endswith(" \\"):
            final.append(seg)
        elif len(seg) > max_len:
            sub = _split_long_line(seg, config, cont_indent)
            final.append(sub if sub is not None else seg)
        else:
            final.append(seg)

    return "\n".join(final)


def _split_commented_code(
    comment_text: str,
    config: FormatterConfig,
    indent: str,
    indent_level: int,
) -> str | None:
    r"""Try to split a long commented-out command using ``\`` continuation.

    If the comment looks like commented-out code (``#command ...``) and the
    full line exceeds ``max_line_length``, we strip the ``#``, format the
    inner command as a single line (with ``\`` continuation if needed), and
    re-prefix only the first line with ``#``.  The ``\`` at end-of-line
    naturally continues the comment in Tcl, so subsequent lines belong to
    the same comment token — and uncommenting just the ``#`` on line 1
    produces valid Tcl.

    Returns the replacement lines (without trailing newline), or ``None``
    if no splitting was needed or possible.
    """
    if not comment_text or len(comment_text) < 2:
        return None

    # Only handle commented-out code (no space after #).
    after_hash = comment_text[1:]
    if not after_hash or after_hash[0].isspace():
        return None

    full_line = indent + comment_text
    if len(full_line) <= config.max_line_length:
        return None

    # Build the uncommented command line and try to split it.
    cmd_line = indent + after_hash
    cont_indent = _make_indent(config, indent_level + 1)
    split = _split_long_line(cmd_line, config, cont_indent)
    if split is None:
        return None

    # Re-comment: only the first line gets ``#``; continuation lines
    # are covered by the ``\`` at end of the previous line.
    split_lines = split.split("\n")
    split_lines[0] = indent + "#" + split_lines[0].removeprefix(indent)
    return "\n".join(split_lines)


# Inline body detection


# Commands whose bodies should never be inlined -- loaded from the registry.
_NEVER_INLINE_BODY_COMMANDS = REGISTRY.never_inline_body_commands()


def _body_can_be_inline(
    body_text: str,
    config: FormatterConfig,
    current_line_len: int = 0,
    command_name: str = "",
) -> bool:
    """Check if a body is short enough to keep on one line.

    A body can be inline if:
    - expand_single_line_bodies is False
    - The parent command is not in _NEVER_INLINE_BODY_COMMANDS
    - It contains exactly one simple command (no newlines, no nested braces)
    - The total line length with inline body fits within goal_line_length
    """
    if config.expand_single_line_bodies:
        return False

    stripped = body_text.strip()
    if not stripped:
        return True  # Empty body: {}

    # Certain commands always expand their bodies
    if command_name in _NEVER_INLINE_BODY_COMMANDS:
        return False

    # Check if the formatted body is a single line (no newlines)
    if "\n" in stripped:
        return False

    # Don't inline bodies that contain nested braces (indicate nested structure)
    if "{" in stripped or "}" in stripped:
        return False

    # Check if the total line with "{ body }" would fit
    inline_len = current_line_len + len("{ ") + len(stripped) + len(" }")
    if inline_len > config.goal_line_length:
        return False

    return True


# Command reconstruction


def _reconstruct_command(
    cmd: ParsedCommand,
    config: FormatterConfig,
    indent: str,
    indent_level: int,
) -> str:
    """Reconstruct a single command as formatted text.

    Returns one or more lines (joined by newlines) for the command.
    """
    # Special case: switch with braced body needs custom formatting
    if cmd.name == "switch":
        return _reconstruct_switch(cmd, config, indent, indent_level)

    # Pre-compute which argument indices are expression args (for wrapping).
    expr_arg_set = _identify_expr_args(cmd)

    parts: list[str] = []
    # Track whether we're in a "structural brace chain" where
    # space_between_braces controls spacing.  This is True after a
    # BODY closing brace or a KEYWORD (which bridges }keyword{).
    # Regular WORD args (like braced conditions) are NOT part of this
    # chain and always get normal spacing.
    in_brace_chain = False

    for i, arg in enumerate(cmd.args):
        if arg.kind == ArgKind.BODY and arg.formatted_body is not None:
            body = arg.formatted_body

            # Check if body can stay inline (single short command)
            current_line_len = len(indent) + len("".join(parts))
            if _body_can_be_inline(body, config, current_line_len, cmd.name):
                if in_brace_chain and not config.space_between_braces:
                    pass  # no space: }keyword{ or }{
                elif parts:
                    parts.append(" ")

                stripped = body.strip()
                if stripped:
                    inline = "{ " + stripped + " }"
                    parts.append(inline)
                else:
                    parts.append("{}")
                in_brace_chain = True
                continue

            if in_brace_chain and not config.space_between_braces:
                pass  # no space: }keyword{ or }{
            elif parts:
                parts.append(" ")

            if body.strip():
                parts.append("{\n")
                parts.append(body)
                parts.append("\n" + indent + "}")
            else:
                parts.append("{}")
            in_brace_chain = True

        elif arg.kind == ArgKind.PARAM_LIST:
            # Parameter list -- normalize internal whitespace
            raw = _normalize_param_list(arg)
            if parts:
                parts.append(" ")
            parts.append(raw)
            in_brace_chain = False

        elif arg.kind == ArgKind.KEYWORD:
            # Keywords (else, elseif, finally, etc.) always need spaces
            # around them for valid Tcl syntax -- }else{ is not valid Tcl.
            if parts:
                parts.append(" ")
            parts.append(arg.text)
            in_brace_chain = False

        else:
            # Regular argument -- always use normal spacing
            if config.enforce_braced_variables:
                raw = _reconstruct_arg_with_braced_vars(arg)
            else:
                raw = _reconstruct_arg(arg)

            # Skip backslash-newline continuation artifacts from input.
            # The lexer produces these as ESC tokens; the formatter's own
            # line-splitting logic will re-add continuations where needed.
            if "\n" in raw:
                raw = re.sub(r"[ \t]*\\\n[ \t]*", " ", raw).strip()
                if not raw:
                    continue

            # Try wrapping braced expression args that would exceed
            # max_line_length (newlines are valid whitespace in expr).
            if arg.is_braced and i in expr_arg_set:
                col = _current_col(parts, len(indent))
                if parts:
                    col += 1  # account for the space separator
                # Estimate trailing content on the same line after this
                # arg (e.g. " {" for the body opening brace).
                trailing = _estimate_trailing_len(cmd.args, i + 1)
                if col + len(raw) + trailing > config.max_line_length:
                    wrapped = _wrap_braced_expr(arg.text, config, col, indent_level, trailing)
                    if wrapped is not None:
                        if parts:
                            parts.append(" ")
                        parts.append("{" + wrapped + "}")
                        in_brace_chain = False
                        continue

            if parts:
                parts.append(" ")
            parts.append(raw)
            in_brace_chain = False

    line = indent + "".join(parts)

    # If the reconstructed command's *first line* exceeds max_line_length,
    # try to split it using backslash continuation.  Multi-line commands
    # (those with expanded bodies) may have a short first line followed by
    # body content — we only check and split the first line.
    first_nl = line.find("\n")
    first_line = line if first_nl == -1 else line[:first_nl]
    if len(first_line) > config.max_line_length:
        cont_indent = _make_indent(config, indent_level + 1)
        split = _split_long_line(first_line, config, cont_indent)
        if split is not None:
            if first_nl != -1:
                return split + line[first_nl:]
            return split

    return line


def _reconstruct_switch(
    cmd: ParsedCommand,
    config: FormatterConfig,
    indent: str,
    indent_level: int,
) -> str:
    """Reconstruct a switch command, handling the braced body form specially."""
    inner_indent_level = indent_level + 1
    parts: list[str] = []
    args = cmd.args

    for i, arg in enumerate(args):
        if arg.kind == ArgKind.BODY and arg.is_braced and cmd.name == "switch":
            # This is the switch braced body -- format pattern/body pairs
            formatted = _format_switch_body(arg.text, config, inner_indent_level)
            if parts:
                parts.append(" ")
            if formatted.strip():
                parts.append("{\n")
                parts.append(formatted)
                parts.append("\n" + indent + "}")
            else:
                parts.append("{}")
        else:
            if config.enforce_braced_variables:
                raw = _reconstruct_arg_with_braced_vars(arg)
            else:
                raw = _reconstruct_arg(arg)
            if parts:
                parts.append(" ")
            parts.append(raw)

    return indent + "".join(parts)


# Enforce braced variables


def _enforce_braced_variables_in_arg(arg: CommandArg) -> None:
    """Rewrite $var tokens to ${var} in an argument's tokens."""
    for i, tok in enumerate(arg.tokens):
        if tok.type == TokenType.VAR:
            # The token text is just the name (no $ or braces).
            # _reconstruct_raw will produce $name. We need ${name}.
            # We achieve this by wrapping the text in braces and keeping
            # the type as VAR, then adjusting _reconstruct_raw to handle it.
            # Instead, we'll do the rewrite at reconstruction time.
            pass
    # This is handled in _reconstruct_arg_with_braced_vars
    pass


def _reconstruct_arg_with_braced_vars(arg: CommandArg) -> str:
    """Like _reconstruct_arg but rewrites $var to ${var}."""
    parts: list[str] = []
    for tok in arg.tokens:
        if tok.type == TokenType.VAR:
            # Check if already braced: ${name} vs $name
            # We can detect this if the original source had '{' after '$'
            parts.append("${" + tok.text + "}")
        else:
            parts.append(_reconstruct_raw(tok))
    raw = "".join(parts)
    if arg.is_quoted:
        return '"' + raw + '"'
    return raw


# Main formatting entry point


def format_body(
    source: str,
    config: FormatterConfig,
    indent_level: int = 0,
) -> str:
    """Format a Tcl script body at the given indent level.

    This is the core recursive function. It:
    1. Parses source into commands
    2. Identifies body arguments
    3. Recursively formats bodies at indent_level + 1
    4. Reconstructs the output
    """
    commands, trailing_comments = parse_commands(source)
    indent = _make_indent(config, indent_level)
    inner_indent_level = indent_level + 1

    lines: list[str] = []

    for i, cmd in enumerate(commands):
        # Blank lines
        blank_count = _compute_blank_lines(commands, i, config)
        for _ in range(blank_count):
            lines.append("")

        # Preceding comments
        for comment in cmd.preceding_comments:
            formatted = _format_comment(comment, config)
            # Try to split long commented-out code lines.
            split = _split_commented_code(formatted, config, indent, indent_level)
            if split is not None:
                lines.append(split)
            else:
                lines.append(indent + formatted)

        # Identify body arguments and recursively format
        _identify_body_args(cmd)

        for arg in cmd.args:
            if arg.kind == ArgKind.BODY and arg.is_braced:
                arg.formatted_body = format_body(
                    arg.text,
                    config,
                    inner_indent_level,
                )

        # Reconstruct the command (with braced vars if configured)
        line = _reconstruct_command(cmd, config, indent, indent_level)
        lines.append(line)

    # Trailing comments (comments after the last command)
    for comment in trailing_comments:
        formatted = _format_comment(comment, config)
        split = _split_commented_code(formatted, config, indent, indent_level)
        if split is not None:
            lines.append(split)
        else:
            lines.append(indent + formatted)

    return "\n".join(lines)
