"""Standalone helper functions for the optimiser package."""

from __future__ import annotations

import logging
import re

from ...analysis.semantic_model import Range
from ...commands.registry import REGISTRY
from ...common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...parsing.command_shapes import extract_single_expr_argument
from ...parsing.lexer import TclLexer
from ...parsing.tokens import SourcePosition, Token, TokenType
from ..core_analyses import LatticeKind, LatticeValue
from ..interprocedural import InterproceduralAnalysis
from ..ir import CommandTokens
from ..token_helpers import parse_decimal_int as _parse_decimal_int
from ..token_helpers import word_piece as _word_piece
from ._types import _OPT_PRIORITY, Optimisation

log = logging.getLogger(__name__)

_SAFE_WORD_RE = re.compile(r"[A-Za-z0-9_./:+-]+\Z")
_STATIC_VAR_WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_:]*\Z")
_DYNAMIC_BARRIER_COMMANDS = REGISTRY.dynamic_barrier_commands()


def _braced_token_range(tok: Token) -> Range:
    """Extend a STR token's range to include the closing ``}``."""
    end = tok.end
    return Range(
        start=tok.start,
        end=SourcePosition(
            line=end.line,
            character=end.character + 1,
            offset=end.offset + 1,
        ),
    )


def _braced_token_range_from_range(r: Range) -> Range:
    """Extend a range by one character to include a closing ``}``."""
    end = r.end
    return Range(
        start=r.start,
        end=SourcePosition(
            line=end.line,
            character=end.character + 1,
            offset=end.offset + 1,
        ),
    )


def _command_subst_range(tok: Token) -> Range:
    return Range(
        start=tok.start,
        end=_advance_position(tok.start, "[" + tok.text + "]"),
    )


def _advance_position(start: SourcePosition, text: str) -> SourcePosition:
    """Return position at the last character of text starting at *start*."""
    line = start.line
    col = start.character
    offset = start.offset
    if not text:
        return start
    for ch in text[1:]:
        if ch == "\n":
            line += 1
            col = 0
        else:
            col += 1
        offset += 1
    return SourcePosition(line=line, character=col, offset=offset)


def _namespace_parts(namespace: str) -> list[str]:
    return [p for p in _normalise_qualified_name(namespace).split("::") if p]


def _namespace_from_qualified(qname: str) -> str:
    """Extract the namespace portion of a fully-qualified proc name."""
    parts = _namespace_parts(qname)
    if len(parts) <= 1:
        return "::"
    return "::" + "::".join(parts[:-1])


def _resolve_summary_proc_name(
    command: str,
    *,
    namespace: str,
    interproc: InterproceduralAnalysis,
) -> str | None:
    if not command:
        return None

    names = interproc.procedures.keys()
    if command.startswith("::"):
        qname = _normalise_qualified_name(command)
        return qname if qname in names else None
    if "::" in command:
        qname = _normalise_qualified_name(f"::{command}")
        return qname if qname in names else None

    ns_parts = _namespace_parts(namespace)
    for depth in range(len(ns_parts), -1, -1):
        prefix = ns_parts[:depth]
        if prefix:
            candidate = "::" + "::".join(prefix + [command])
        else:
            candidate = f"::{command}"
        if candidate in names:
            return candidate
    return None


def _literal_from_constant_str(value: str) -> int | bool | str | None:
    parsed = _parse_decimal_int(value)
    if parsed is not None:
        return int(parsed)
    lowered = value.strip().lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if _SAFE_WORD_RE.fullmatch(value):
        return value
    return None


def _render_folded_literal(value: int | float | bool | str) -> str | None:
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, float):
        return str(value) if value == int(value) else None
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        if _SAFE_WORD_RE.fullmatch(value):
            return value
    return None


def _render_static_string_word(value: str) -> str | None:
    if value == "":
        return "{}"
    if _SAFE_WORD_RE.fullmatch(value):
        return value
    if any(ch in value for ch in "{}\\\n\r"):
        return None
    return "{" + value + "}"


def _parse_static_string_arg(
    arg_text: str,
    arg_token: Token,
    *,
    single_token: bool,
) -> str | None:
    if not single_token:
        return None
    if arg_token.type is TokenType.STR:
        return arg_text
    if arg_token.type is TokenType.ESC:
        # Preserve semantics only when no backslash substitution is involved.
        if "\\" in arg_text:
            return None
        return arg_text
    return None


def _is_static_var_word(
    word: str,
    tok: Token,
    *,
    single_token: bool,
) -> bool:
    if not single_token or tok.type is not TokenType.ESC:
        return False
    return _STATIC_VAR_WORD_RE.fullmatch(word) is not None


def _full_command_range(source: str, command_range: Range) -> Range | None:
    start = command_range.start.offset
    if start < 0 or start >= len(source):
        return None

    lexer = TclLexer(
        source[start:],
        base_offset=start,
        base_line=command_range.start.line,
        base_col=command_range.start.character,
    )

    saw_word = False
    end_offset = start

    while True:
        tok = lexer.get_token()
        if tok is None:
            if not saw_word:
                return None
            end_offset = len(source) - 1
            break
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            continue
        if tok.type is TokenType.EOL:
            if not saw_word:
                continue
            end_offset = tok.start.offset - 1
            break
        # A standalone "}" is a body-closing brace, not a command word.
        if tok.text == "}":
            if not saw_word:
                return None
            # End the command range before the closing brace.
            end_offset = tok.start.offset - 1
            break
        saw_word = True

    if end_offset < start:
        return None
    end_pos = _advance_position(command_range.start, source[start : end_offset + 1])
    return Range(start=command_range.start, end=end_pos)


def _extract_body_text(source: str, body_range: Range, stmt_range: Range) -> str:
    """Extract body content from braces, adjusting indentation to the statement level."""
    start = body_range.start.offset
    end = body_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return ""
    text = source[start : end + 1]
    # Strip outer braces.
    if text.startswith("{"):
        text = text[1:]
    if text.endswith("}"):
        text = text[:-1]
    # Strip a leading newline.
    if text.startswith("\n"):
        text = text[1:]
    elif text.startswith("\r\n"):
        text = text[2:]
    # Strip trailing whitespace before the closing brace.
    text = text.rstrip(" \t\r\n")
    if not text:
        return ""
    # Compute the target indent (column of the compound statement).
    target_col = stmt_range.start.character
    target_indent = " " * target_col
    # Compute the common leading whitespace of body lines.
    lines = text.split("\n")
    min_indent = None
    for line in lines:
        stripped = line.lstrip(" \t")
        if not stripped:
            continue
        leading = len(line) - len(stripped)
        if min_indent is None or leading < min_indent:
            min_indent = leading
    if min_indent is None:
        min_indent = 0
    # De-indent and re-indent to the target level.
    result_lines = []
    for i, line in enumerate(lines):
        stripped = line.lstrip(" \t")
        if not stripped:
            result_lines.append("")
        elif i == 0:
            result_lines.append(line[min_indent:])
        else:
            result_lines.append(target_indent + line[min_indent:])
    return "\n".join(result_lines)


def _is_plain_literal(text: str) -> bool:
    """Return ``True`` if *text* is a plain literal with no substitutions."""
    return "$" not in text and "[" not in text and "{" not in text and "\\" not in text


def _parse_command_words(text: str) -> tuple[list[str], list[Token], list[bool]] | None:
    lexer = TclLexer(text)
    commands: list[tuple[list[str], list[Token], list[bool]]] = []
    argv_texts: list[str] = []
    argv_tokens: list[Token] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL

    def flush() -> None:
        nonlocal argv_texts, argv_tokens, argv_single
        if argv_texts:
            commands.append((argv_texts, argv_tokens, argv_single))
        argv_texts = []
        argv_tokens = []
        argv_single = []

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            flush()
            prev_type = tok.type
            continue

        piece = _word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += piece
                argv_single[-1] = False
            else:
                argv_texts.append(piece)
                argv_tokens.append(tok)
                argv_single.append(True)
        prev_type = tok.type

    flush()
    if len(commands) != 1:
        return None
    return commands[0]


def _select_non_overlapping_optimisations(optimisations: list[Optimisation]) -> list[Optimisation]:
    """Apply simple pass-order/invalidation rules for overlapping rewrites."""
    # Hint-only diagnostics never conflict with rewrites — keep them all.
    hints = [o for o in optimisations if o.hint_only]
    rewrite_opts = [o for o in optimisations if not o.hint_only]
    sorted_opts = sorted(
        rewrite_opts,
        key=lambda o: (
            o.range.start.offset,
            -_OPT_PRIORITY.get(o.code, 0),
            -(o.range.end.offset - o.range.start.offset),
        ),
    )

    selected: list[Optimisation] = []
    dropped_groups: set[int] = set()
    for opt in sorted_opts:
        start = opt.range.start.offset
        end = opt.range.end.offset
        overlap = any(
            not (end < kept.range.start.offset or start > kept.range.end.offset)
            for kept in selected
        )
        if overlap:
            if opt.group is not None:
                dropped_groups.add(opt.group)
            continue
        selected.append(opt)

    # Clear group from surviving members whose group lost a sibling.
    if dropped_groups:
        selected = [
            Optimisation(
                code=opt.code,
                message=opt.message,
                range=opt.range,
                replacement=opt.replacement,
                group=None,
            )
            if opt.group is not None and opt.group in dropped_groups
            else opt
            for opt in selected
        ]

    return sorted(selected + hints, key=lambda o: o.range.start.offset)


def _format_constant(value: object) -> str | None:
    from ..tcl_expr_eval import format_tcl_value

    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return format_tcl_value(value)
    if isinstance(value, str):
        return value
    return None


def _constants_from_uses(
    uses: dict[str, int],
    values: dict[tuple[str, int], LatticeValue],
) -> dict[str, str]:
    constants: dict[str, str] = {}
    for name, ver in uses.items():
        if ver <= 0:
            continue
        lv = values.get((name, ver))
        if lv is None or getattr(lv, "kind", None) is not LatticeKind.CONST:
            continue
        s = _format_constant(getattr(lv, "value", None))
        if s is not None:
            constants[name] = s
    return constants


def _constants_from_exit_versions(
    exit_versions: dict[str, int],
    values: dict[tuple[str, int], LatticeValue],
) -> dict[str, str]:
    """Build a constants dict from SSA block exit versions."""
    constants: dict[str, str] = {}
    for name, ver in exit_versions.items():
        if ver <= 0:
            continue
        lv = values.get((name, ver))
        if lv is None or getattr(lv, "kind", None) is not LatticeKind.CONST:
            continue
        s = _format_constant(getattr(lv, "value", None))
        if s is not None:
            constants[name] = s
    return constants


def _expr_arg_from_expr_command(cmd_text: str) -> str | None:
    """Return expr argument if command text is exactly: expr <one-arg>."""
    return extract_single_expr_argument(cmd_text)


def _tokens_for_statement(
    stmt,
    source: str,
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Extract parsed token info from an IR statement."""
    ct: CommandTokens | None = getattr(stmt, "tokens", None)
    if ct is not None:
        return list(ct.argv_texts), list(ct.argv), list(ct.single_token_word)
    stmt_range = getattr(stmt, "range", None)
    if stmt_range is None:
        return None
    return _parse_single_command_from_range(source, stmt_range)


def _parse_single_command_from_range(
    source: str,
    command_range: Range,
) -> tuple[list[str], list[Token], list[bool]] | None:
    start = command_range.start.offset
    end = command_range.end.offset
    if start < 0 or end < start or end >= len(source):
        return None

    lexer = TclLexer(
        source[start : end + 1],
        base_offset=start,
        base_line=command_range.start.line,
        base_col=command_range.start.character,
    )

    argv_texts: list[str] = []
    argv_tokens: list[Token] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL
    saw_eol = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            if argv_texts:
                saw_eol = True
            prev_type = tok.type
            continue
        if saw_eol:
            return None

        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(tok.text)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += tok.text
                argv_single[-1] = False
            else:
                argv_texts.append(tok.text)
                argv_tokens.append(tok)
                argv_single.append(True)
        prev_type = tok.type

    if not argv_texts:
        return None
    return argv_texts, argv_tokens, argv_single


def _try_fold_list_command(cmd_text: str) -> str | None:
    """Fold ``[list literal1 literal2 ...]`` to a literal value."""
    parsed = _parse_command_words(cmd_text)
    if parsed is None:
        return None
    cmd_texts, cmd_tokens, cmd_single = parsed
    if not cmd_texts or cmd_texts[0] != "list":
        return None
    if len(cmd_texts) == 1:
        return ""  # [list] -> empty string
    for i in range(1, len(cmd_texts)):
        if i >= len(cmd_single) or not cmd_single[i]:
            return None
        if i >= len(cmd_tokens):
            return None
        tok = cmd_tokens[i]
        if tok.type not in (TokenType.STR, TokenType.ESC):
            return None
        if not _is_plain_literal(cmd_texts[i]):
            return None
        if not _SAFE_WORD_RE.fullmatch(cmd_texts[i]):
            return None
    elements = cmd_texts[1:]
    if len(elements) == 1:
        return elements[0]
    return "{" + " ".join(elements) + "}"


def _parse_string_length_arg(cmd_text: str) -> str | None:
    """If *cmd_text* is ``[string length <arg>]`` or ``string length <arg>``,
    return the ``<arg>`` text.
    """
    inner = cmd_text
    if inner.startswith("[") and inner.endswith("]"):
        inner = inner[1:-1]
    inner = inner.strip()
    if not inner.startswith("string"):
        return None
    lexer = TclLexer(inner)
    argv: list[str] = []
    prev_type = TokenType.EOL
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            break
        if tok.type in (TokenType.SEP, TokenType.COMMENT):
            prev_type = tok.type
            continue
        piece = _word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(piece)
        else:
            if argv:
                argv[-1] += piece
        prev_type = tok.type
    if len(argv) != 3 or argv[0] != "string" or argv[1] != "length":
        return None
    return argv[2]


def _try_fold_lindex_command(cmd_text: str) -> str | None:
    """Fold ``[lindex {a b c} N]`` to the element at index *N*."""
    parsed = _parse_command_words(cmd_text)
    if parsed is None:
        return None
    cmd_texts, cmd_tokens, cmd_single = parsed
    if len(cmd_texts) != 3 or cmd_texts[0] != "lindex":
        return None
    # List argument must be a braced literal.
    if not cmd_single[1] or cmd_tokens[1].type is not TokenType.STR:
        return None
    list_text = cmd_texts[1]
    # Only handle simple lists (no nested braces or backslashes).
    if any(ch in list_text for ch in "{}\\"):
        return None
    # Index must be a literal.
    if not cmd_single[2]:
        return None
    if cmd_tokens[2].type not in (TokenType.STR, TokenType.ESC):
        return None
    idx_text = cmd_texts[2].strip()
    if idx_text == "end":
        idx = -1
    elif idx_text.startswith("end-"):
        try:
            offset = int(idx_text[4:])
            idx = -(offset + 1)
        except ValueError:
            return None
    else:
        try:
            idx = int(idx_text)
        except ValueError:
            return None
    elements = list_text.split()
    if not elements:
        return ""
    if idx < 0:
        idx = len(elements) + idx
    if idx < 0 or idx >= len(elements):
        return ""
    result = elements[idx]
    if not _SAFE_WORD_RE.fullmatch(result):
        return None
    return result


def _try_incr_idiom(
    argv_texts: list[str],
    argv_tokens: list[Token],
    argv_single: list[bool],
) -> str | None:
    """Detect ``set var [expr {$var + N}]`` -> ``incr var N``."""
    from ...common.dialect import active_dialect
    from ...parsing.expr_parser import parse_expr
    from ..expr_ast import BinOp, ExprBinary, ExprLiteral, ExprRaw, ExprVar

    if len(argv_texts) != 3 or argv_texts[0] != "set":
        return None
    # Variable must be a simple name.
    if not argv_single[1] or argv_tokens[1].type not in (TokenType.ESC, TokenType.STR):
        return None
    var_name = argv_texts[1]
    if not _STATIC_VAR_WORD_RE.fullmatch(var_name):
        return None
    # Value must be a single CMD token.
    if not argv_single[2] or argv_tokens[2].type is not TokenType.CMD:
        return None
    expr_arg = _expr_arg_from_expr_command(argv_tokens[2].text)
    if expr_arg is None:
        return None
    parsed = parse_expr(expr_arg.strip(), dialect=active_dialect())
    if isinstance(parsed, ExprRaw) or not isinstance(parsed, ExprBinary):
        return None
    if parsed.op is BinOp.ADD:
        var_side: ExprVar | None = None
        lit_side: ExprLiteral | None = None
        if isinstance(parsed.left, ExprVar) and isinstance(parsed.right, ExprLiteral):
            var_side = parsed.left
            lit_side = parsed.right
        elif isinstance(parsed.right, ExprVar) and isinstance(parsed.left, ExprLiteral):
            var_side = parsed.right
            lit_side = parsed.left
        else:
            return None
        if _normalise_var_name(var_side.name) != _normalise_var_name(var_name):
            return None
        incr_val = _parse_decimal_int(lit_side.text)
        if incr_val is None:
            return None
        incr_int = int(incr_val)
        if incr_int == 1:
            return f"incr {var_name}"
        return f"incr {var_name} {incr_int}"
    if parsed.op is BinOp.SUB:
        if not isinstance(parsed.left, ExprVar) or not isinstance(parsed.right, ExprLiteral):
            return None
        if _normalise_var_name(parsed.left.name) != _normalise_var_name(var_name):
            return None
        incr_val = _parse_decimal_int(parsed.right.text)
        if incr_val is None:
            return None
        incr_int = int(incr_val)
        if incr_int == 0:
            return None
        if incr_int == -1:
            return f"incr {var_name}"
        return f"incr {var_name} {-incr_int}"
    return None
