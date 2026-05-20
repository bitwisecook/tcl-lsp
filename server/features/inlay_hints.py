"""Inlay hint provider -- show inferred types and format string annotations."""

from __future__ import annotations

import logging
from collections.abc import Callable

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ProcDef
from compiler.core_analyses import analyse_source
from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_positions import token_content_shift
from compiler.registry import REGISTRY
from compiler.registry.models import OptionSpec
from compiler.types import TypeKind
from shared.tokens import TokenType

from ._semantic_tokens import (
    _BINARY_FORMAT_SPECIFIERS,
    _CLOCK_FORMAT_RE,
    _REGSUB_BACKREF_RE,
    _SPRINTF_RE,
    _binary_format_arg_index,
    _clock_format_arg_index,
    _regsub_subspec_arg_index,
    _sprintf_format_arg_index,
)

log = logging.getLogger(__name__)

# Short display names for Tcl internal representation types.
_TYPE_SHORT: dict[str, str] = {
    "string": "str",
    "int": "int",
    "double": "double",
    "boolean": "bool",
    "list": "list",
    "dict": "dict",
    "bytearray": "bytes",
    "numeric": "num",
}


def _short_type(tcl_type_name: str) -> str:
    """Return the abbreviated display name for a TclType enum name."""
    low = tcl_type_name.lower()
    return _TYPE_SHORT.get(low, low)


def _collect_type_hints(
    source: str,
    analysis: AnalysisResult,
    range_: types.Range,
) -> list[types.InlayHint]:
    """Collect type inlay hints for variable definitions in the given range."""
    hints: list[types.InlayHint] = []

    try:
        module_analysis = analyse_source(source)
    except Exception:
        log.debug("inlay_hints: analysis failed for type hints", exc_info=True)
        return hints

    # Collect types from all scopes
    type_map: dict[str, str] = {}
    for fa in [module_analysis.top_level, *module_analysis.procedures.values()]:
        for (name, _ver), tl in fa.types.items():
            if tl.kind is TypeKind.KNOWN and tl.tcl_type is not None:
                type_map[name] = _short_type(tl.tcl_type.name)
            elif tl.kind is TypeKind.SHIMMERED and tl.from_type and tl.tcl_type:
                type_map[name] = (
                    f"{_short_type(tl.from_type.name)} \u2192 {_short_type(tl.tcl_type.name)}"
                )

    if not type_map:
        return hints

    # Walk all variable definitions in scope tree and emit hints
    def _walk_scope(scope):
        for var_def in scope.variables.values():
            dr = var_def.definition_range
            # Check if within requested range
            if dr.end.line < range_.start.line or dr.start.line > range_.end.line:
                continue
            type_str = type_map.get(var_def.name)
            if type_str is None:
                continue
            hints.append(
                types.InlayHint(
                    position=types.Position(
                        line=dr.end.line,
                        character=dr.end.character + 1,
                    ),
                    label=f": {type_str}",
                    kind=types.InlayHintKind.Type,
                    padding_left=True,
                )
            )
        for child in scope.children:
            _walk_scope(child)

    _walk_scope(analysis.global_scope)
    return hints


# Format-string inlay hint labels (short annotations)

_SPRINTF_SHORT: dict[str, str] = {
    "s": "str",
    "d": "int",
    "i": "int",
    "u": "uint",
    "o": "oct",
    "x": "hex",
    "X": "HEX",
    "f": "float",
    "e": "exp",
    "E": "EXP",
    "g": "num",
    "G": "NUM",
    "c": "char",
    "%": "%%",
    "b": "bin",
    "B": "BIN",
    "a": "hexf",
    "A": "HEXF",
}

_CLOCK_SHORT: dict[str, str] = {
    "Y": "year",
    "y": "yr",
    "m": "month",
    "d": "day",
    "e": "day",
    "H": "hour",
    "I": "hour12",
    "k": "hour",
    "l": "hour12",
    "M": "min",
    "S": "sec",
    "s": "epoch",
    "p": "AM/PM",
    "P": "am/pm",
    "a": "wday",
    "A": "weekday",
    "b": "mon",
    "B": "month",
    "c": "datetime",
    "D": "date",
    "j": "yday",
    "u": "wday#",
    "w": "wday#",
    "U": "week",
    "W": "week",
    "V": "isoweek",
    "z": "tz",
    "Z": "tz",
    "x": "date",
    "X": "time",
    "C": "century",
    "g": "isoyear",
    "G": "isoyear",
    "J": "julian",
    "%": "%%",
}

_BINARY_SHORT: dict[str, str] = {
    "a": "strN",
    "A": "strS",
    "b": "bits",
    "B": "BITS",
    "h": "hex",
    "H": "HEX",
    "c": "i8",
    "s": "i16le",
    "S": "i16be",
    "i": "i32le",
    "I": "i32be",
    "n": "i32",
    "w": "i64le",
    "W": "i64be",
    "m": "i64",
    "r": "f32le",
    "R": "f32be",
    "f": "f32",
    "d": "f64",
    "x": "pad",
    "X": "back",
    "@": "seek",
    "t": "rsv",
}

_REGSUB_SHORT: dict[str, str] = {
    "&": "match",
    "0": "match",
    "1": "grp1",
    "2": "grp2",
    "3": "grp3",
    "4": "grp4",
    "5": "grp5",
    "6": "grp6",
    "7": "grp7",
    "8": "grp8",
    "9": "grp9",
}


def _collect_format_string_hints(
    source: str,
    range_: types.Range,
    *,
    lines: list[str] | None = None,
) -> list[types.InlayHint]:
    """Collect inlay hints for format string specifiers."""
    hints: list[types.InlayHint] = []
    if lines is None:
        lines = source.split("\n")

    for line_no in range(range_.start.line, min(range_.end.line + 1, len(lines))):
        line_text = lines[line_no]
        if not line_text.strip():
            continue

        # Parse the line to find commands and their arguments
        lexer = TclLexer(line_text, base_line=line_no)
        argv_texts: list[str] = []
        argv_tokens: list = []
        prev_type = TokenType.EOL

        while True:
            tok = lexer.get_token()
            if tok is None or tok.type is TokenType.EOL:
                break
            if tok.type in (TokenType.SEP, TokenType.COMMENT):
                prev_type = tok.type
                continue
            if prev_type in (TokenType.SEP, TokenType.EOL):
                argv_texts.append(tok.text)
                argv_tokens.append(tok)
            else:
                if argv_texts:
                    argv_texts[-1] += tok.text
            prev_type = tok.type

        if not argv_texts:
            continue

        cmd_name = argv_texts[0]

        # Check each format type
        _emit_sprintf_hints(hints, cmd_name, argv_texts, argv_tokens, line_no)
        _emit_clock_hints(hints, cmd_name, argv_texts, argv_tokens, line_no)
        _emit_binary_hints(hints, cmd_name, argv_texts, argv_tokens, line_no)
        _emit_regsub_hints(hints, cmd_name, argv_texts, argv_tokens, line_no)

    return hints


def _token_content_start(tok) -> int:
    """Return the character offset where the token content begins (after delimiter)."""
    shift = token_content_shift(tok)
    return tok.start.character + shift


def _emit_sprintf_hints(hints, cmd_name, argv_texts, argv_tokens, line_no):
    idx = _sprintf_format_arg_index(cmd_name, argv_texts)
    if idx is None or idx >= len(argv_tokens):
        return
    text = argv_texts[idx]
    base_char = _token_content_start(argv_tokens[idx])
    for m in _SPRINTF_RE.finditer(text):
        type_char = m.group("type")
        label = _SPRINTF_SHORT.get(type_char)
        if label and label != "%%":
            hints.append(
                types.InlayHint(
                    position=types.Position(line=line_no, character=base_char + m.end()),
                    label=label,
                    kind=types.InlayHintKind.Type,
                    padding_left=True,
                )
            )


def _emit_clock_hints(hints, cmd_name, argv_texts, argv_tokens, line_no):
    idx = _clock_format_arg_index(cmd_name, argv_texts)
    if idx is None or idx >= len(argv_tokens):
        return
    text = argv_texts[idx]
    base_char = _token_content_start(argv_tokens[idx])
    for m in _CLOCK_FORMAT_RE.finditer(text):
        spec = m.group()
        letter = spec[-1]
        label = _CLOCK_SHORT.get(letter)
        if label and label != "%%":
            hints.append(
                types.InlayHint(
                    position=types.Position(line=line_no, character=base_char + m.end()),
                    label=label,
                    kind=types.InlayHintKind.Type,
                    padding_left=True,
                )
            )


def _emit_binary_hints(hints, cmd_name, argv_texts, argv_tokens, line_no):
    idx = _binary_format_arg_index(cmd_name, argv_texts)
    if idx is None or idx >= len(argv_tokens):
        return
    text = argv_texts[idx]
    base_char = _token_content_start(argv_tokens[idx])
    i = 0
    while i < len(text):
        if text[i] in " \t\r\n":
            i += 1
            continue
        while i < len(text) and text[i].isdigit():
            i += 1
        if i >= len(text):
            break
        ch = text[i]
        if ch not in _BINARY_FORMAT_SPECIFIERS:
            i += 1
            continue
        spec_end = i + 1
        if spec_end < len(text) and text[spec_end] in ("u", "s"):
            spec_end += 1
        if spec_end < len(text) and text[spec_end] == "*":
            spec_end += 1
        label = _BINARY_SHORT.get(ch)
        if label:
            hints.append(
                types.InlayHint(
                    position=types.Position(line=line_no, character=base_char + spec_end),
                    label=label,
                    kind=types.InlayHintKind.Type,
                    padding_left=True,
                )
            )
        i = spec_end


def _emit_regsub_hints(hints, cmd_name, argv_texts, argv_tokens, line_no):
    idx = _regsub_subspec_arg_index(cmd_name, argv_texts)
    if idx is None or idx >= len(argv_tokens):
        return
    text = argv_texts[idx]
    base_char = _token_content_start(argv_tokens[idx])
    for m in _REGSUB_BACKREF_RE.finditer(text):
        char = m.group(1)
        label = _REGSUB_SHORT.get(char)
        if label:
            hints.append(
                types.InlayHint(
                    position=types.Position(line=line_no, character=base_char + m.end()),
                    label=label,
                    kind=types.InlayHintKind.Type,
                    padding_left=True,
                )
            )


def _synopsis_groups(synopsis: str) -> list[str]:
    """Split a synopsis into tokens, re-joining ``?...?`` optional groups
    that span multiple tokens (e.g. ``?-length length?`` → one group)."""
    groups: list[str] = []
    current: str | None = None
    for tok in synopsis.split():
        if current is not None:
            current += " " + tok
            if tok.endswith("?"):
                groups.append(current)
                current = None
        elif tok.startswith("?") and not tok.endswith("?"):
            current = tok
        else:
            groups.append(tok)
    if current is not None:
        groups.append(current)
    return groups


def _param_names_from_synopsis(synopsis: str, skip_words: int) -> list[str]:
    """Parse positional parameter names out of a command synopsis.

    ``name`` / ``?name?`` → emitted (optionals stripped); ``-flag`` /
    ``?-flag value?`` → skipped; ``...`` / ``?name ...?`` → stops the parse.
    """
    names: list[str] = []
    for group in _synopsis_groups(synopsis)[skip_words:]:
        if "..." in group:
            break
        if group.startswith("?") and group.endswith("?"):
            inner = group[1:-1].strip()
            if not inner or inner.startswith("-") or any(c.isspace() for c in inner):
                continue
            names.append(inner)
        elif group.startswith("-"):
            continue
        elif group:
            names.append(group)
    return names


def _lookup_user_proc(analysis: AnalysisResult, cmd_name: str) -> ProcDef | None:
    """Resolve a command head to a user-defined proc, if any."""
    bare = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == bare or qname == cmd_name or qname.endswith(f"::{bare}"):
            return proc_def
    return None


def _emit_positional_hints(
    hints: list[types.InlayHint],
    cmd,
    param_names: list[str],
    first_arg_idx: int,
    option_for: Callable[[str], OptionSpec | None],
    range_: types.Range,
) -> None:
    """Label each positional call argument with its parameter name.

    Whether a ``-``-prefixed token is an option is decided by the
    registry (``option_for``), not its spelling: only declared options
    are skipped (and their value too, when ``takes_value``).  This keeps
    a real positional like the ``-1`` in ``string index $s -1`` — which
    is no command's option — labelled correctly.  ``--`` ends options.
    """
    if not param_names:
        return
    slot = 0
    arg_idx = first_arg_idx
    options_ended = False
    while arg_idx < len(cmd.argv) and slot < len(param_names):
        arg_text = cmd.texts[arg_idx] if arg_idx < len(cmd.texts) else ""
        if not options_ended and arg_text.startswith("-") and arg_text != "-":
            if arg_text == "--":
                options_ended = True
                arg_idx += 1
                continue
            opt = option_for(arg_text)
            if opt is not None:
                arg_idx += 1
                if opt.takes_value and arg_idx < len(cmd.argv):
                    arg_idx += 1
                continue
        tok = cmd.argv[arg_idx]
        arg_idx += 1
        slot += 1
        pos = tok.start
        if range_.start.line <= pos.line <= range_.end.line:
            hints.append(
                types.InlayHint(
                    position=types.Position(line=pos.line, character=pos.character),
                    label=f"{param_names[slot - 1]}:",
                    kind=types.InlayHintKind.Parameter,
                    padding_right=True,
                )
            )


def _collect_param_name_hints(
    source: str,
    range_: types.Range,
    analysis: AnalysisResult,
) -> list[types.InlayHint]:
    """Parameter-name hints at each positional argument of a call.

    User procs take precedence; built-in commands fall back to their
    registry synopsis (and the ``cmd subcommand`` shape when the spec
    declares subcommands).
    """
    hints: list[types.InlayHint] = []
    for cmd in segment_commands(source):
        if cmd.is_partial or not cmd.argv or not cmd.texts:
            continue
        cmd_name = cmd.texts[0]

        proc_def = _lookup_user_proc(analysis, cmd_name)
        if proc_def is not None:
            names: list[str] = []
            for p in proc_def.params:
                if p.name in ("args", "::args"):
                    break
                names.append(p.name)
            _emit_positional_hints(hints, cmd, names, 1, lambda _n: None, range_)
            continue

        spec = REGISTRY.get(cmd_name)
        if spec is None:
            continue
        if not spec.subcommands:
            synopsis = ""
            if spec.hover and spec.hover.synopsis:
                synopsis = spec.hover.synopsis[0]
            elif spec.forms:
                synopsis = spec.forms[0].synopsis
            if not synopsis:
                continue
            names = _param_names_from_synopsis(synopsis, skip_words=1)
            _emit_positional_hints(hints, cmd, names, 1, spec.option, range_)
        else:
            if len(cmd.texts) < 2:
                continue
            sub = spec.subcommands.get(cmd.texts[1])
            if sub is None:
                continue
            names = _param_names_from_synopsis(sub.synopsis, skip_words=2)
            sub_opts = {o.name: o for o in sub.options}
            _emit_positional_hints(hints, cmd, names, 2, sub_opts.get, range_)
    return hints


def get_inlay_hints(
    source: str,
    range_: types.Range,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.InlayHint]:
    """Return inlay hints for a Tcl source file within the given range."""
    if analysis is None:
        analysis = analyse(source)

    hints = _collect_type_hints(source, analysis, range_)
    hints.extend(_collect_format_string_hints(source, range_, lines=lines))
    hints.extend(_collect_param_name_hints(source, range_, analysis))
    return hints
