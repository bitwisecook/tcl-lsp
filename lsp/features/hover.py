"""Hover provider -- proc signatures, command help, variable info."""

from __future__ import annotations

import logging

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef, Scope, VarDef
from core.commands.registry import REGISTRY
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.operators import operator_hover
from core.commands.registry.runtime import SIGNATURES, SubcommandSig
from core.common.dialect import active_dialect
from core.common.ip_utils import format_ip_hover, parse_ip
from core.compiler.core_analyses import analyse_source
from core.compiler.taint import TaintLattice
from core.compiler.types import TypeKind
from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType

from .semantic_tokens import (
    _BINARY_FORMAT_SPECIFIERS,
    _CLOCK_FORMAT_RE,
    _GLOB_META_RE,
    _REGEX_PART_RE,
    _REGSUB_BACKREF_RE,
    _SPRINTF_RE,
    _binary_format_arg_index,
    _clock_format_arg_index,
    _glob_pattern_arg_indices,
    _regex_pattern_arg_index,
    _regsub_subspec_arg_index,
    _sprintf_format_arg_index,
)
from .symbol_resolution import (
    find_command_context_in_line,
    find_scope_at_line,
    find_var_at_position,
    find_word_span_at_position,
)

log = logging.getLogger(__name__)


def _proc_hover_text(proc_def: ProcDef) -> str:
    """Format hover text for a proc definition."""
    params = []
    for p in proc_def.params:
        if p.has_default:
            params.append(f"{{{p.name} {p.default_value}}}")
        else:
            params.append(p.name)

    sig = f"proc {proc_def.qualified_name} {{{' '.join(params)}}} {{...}}"
    parts = [f"```tcl\n{sig}\n```"]
    if proc_def.doc:
        parts.append(proc_def.doc)
    return "\n\n".join(parts)


def _var_hover_text(
    var_def: VarDef,
    type_info: str | None = None,
    taint_info: str | None = None,
) -> str:
    """Format hover text for a variable."""
    ref_count = len(var_def.references)
    text = f"**Variable** `{var_def.name}`\n\n{ref_count} reference(s)"
    if type_info:
        text = f"{text}\n\n**Inferred intrep**: {type_info}"
    if taint_info:
        text = f"{text}\n\n**Taint**: {taint_info}"
    return text


def _infer_var_type(source: str, var_name: str) -> str | None:
    """Try to infer the dominant type for a variable from compiler analysis.

    Returns a human-readable string if a consistent type is found, or None.
    """
    try:
        module_analysis = analyse_source(source)
    except Exception:
        log.debug("hover: analysis failed for type inference", exc_info=True)
        return None

    # Check top-level and all procedures.
    for analysis in [module_analysis.top_level, *module_analysis.procedures.values()]:
        type_entries = [tl for (name, _ver), tl in analysis.types.items() if name == var_name]
        if not type_entries:
            continue

        # All versions agree on a single known type.
        known = [t for t in type_entries if t.kind is TypeKind.KNOWN and t.tcl_type is not None]
        if known and all(t.tcl_type is known[0].tcl_type for t in known):
            if len(known) == len(type_entries):
                kt = known[0].tcl_type
                assert kt is not None
                return kt.name.lower()

        # All versions agree on SHIMMERED with the same pair.
        shimmered = [
            t
            for t in type_entries
            if t.kind is TypeKind.SHIMMERED and t.from_type is not None and t.tcl_type is not None
        ]
        if shimmered and len(shimmered) == len(type_entries):
            s = shimmered[0]
            if all(t == s for t in shimmered):
                assert s.from_type is not None and s.tcl_type is not None
                return f"shimmered ({s.from_type.name.lower()} / {s.tcl_type.name.lower()})"

        # Mixed but has a dominant type.
        if known:
            kt = known[0].tcl_type
            assert kt is not None
            return kt.name.lower()

    return None


_COLOUR_LABELS: list[tuple[int, str]] = [
    # Listed in display priority order.
    # Uses raw flag values to avoid importing TaintColour into the hot path.
]


def _taint_colour_labels(taint: TaintLattice) -> list[str]:
    """Return human-readable labels for the colour flags present in *taint*."""
    from core.commands.registry.taint_hints import TaintColour

    _flag_labels: list[tuple[TaintColour, str]] = [
        (TaintColour.PATH_PREFIXED, "path-prefixed"),
        (TaintColour.NON_DASH_PREFIXED, "non-dash-prefixed"),
        (TaintColour.CRLF_FREE, "CRLF-free"),
        (TaintColour.SHELL_ATOM, "shell-atom"),
        (TaintColour.LIST_CANONICAL, "list-canonical"),
        (TaintColour.REGEX_LITERAL, "regex-literal"),
        (TaintColour.PATH_NORMALISED, "path-normalised"),
        (TaintColour.HEADER_TOKEN_SAFE, "header-token-safe"),
        (TaintColour.HTML_ESCAPED, "HTML-escaped"),
        (TaintColour.URL_ENCODED, "URL-encoded"),
        (TaintColour.IP_ADDRESS, "IP-address"),
        (TaintColour.PORT, "port"),
        (TaintColour.FQDN, "FQDN"),
    ]
    labels: list[str] = []
    for flag, label in _flag_labels:
        if bool(taint.colour & flag):
            labels.append(label)
    return labels


def _infer_var_taint(source: str, var_name: str) -> str | None:
    """Check if a variable carries tainted data from compiler analysis."""
    try:
        module_analysis = analyse_source(source)
    except Exception:
        log.debug("hover: analysis failed for taint inference", exc_info=True)
        return None

    for analysis in [module_analysis.top_level, *module_analysis.procedures.values()]:
        taint_entries = [tl for (name, _ver), tl in analysis.taints.items() if name == var_name]
        if not taint_entries:
            continue
        tainted = [t for t in taint_entries if t.tainted]
        if tainted:
            # Join all versions to get the most conservative colour set.
            from core.compiler.taint import taint_join

            combined = tainted[0]
            for t in tainted[1:]:
                combined = taint_join(combined, t)
            labels = _taint_colour_labels(combined)
            if labels:
                return f"tainted (from I/O); {', '.join(labels)}"
            return "tainted (from I/O)"
    return None


# Format-string specifier descriptions

_SPRINTF_SPEC_DESC: dict[str, str] = {
    "d": "Signed decimal integer",
    "i": "Signed decimal integer",
    "u": "Unsigned decimal integer",
    "o": "Unsigned octal integer",
    "x": "Unsigned hexadecimal (lowercase)",
    "X": "Unsigned hexadecimal (uppercase)",
    "f": "Floating-point (fixed notation)",
    "e": "Floating-point (scientific, lowercase)",
    "E": "Floating-point (scientific, uppercase)",
    "g": "Shorter of %e or %f",
    "G": "Shorter of %E or %f",
    "s": "String",
    "c": "Character (by Unicode code point)",
    "%": "Literal percent sign",
    "b": "Unsigned binary integer",
    "B": "Unsigned binary integer (alternate form)",
    "a": "Double hex fraction (lowercase)",
    "A": "Double hex fraction (uppercase)",
}

_CLOCK_SPEC_DESC: dict[str, str] = {
    "a": "Abbreviated weekday name",
    "A": "Full weekday name",
    "b": "Abbreviated month name",
    "B": "Full month name",
    "c": "Locale date and time",
    "C": "Century (00\u201399)",
    "d": "Day of month (01\u201331)",
    "D": "Date as %m/%d/%Y",
    "e": "Day of month (1\u201331, no leading zero)",
    "g": "ISO 8601 2-digit year",
    "G": "ISO 8601 4-digit year",
    "h": "Abbreviated month name (same as %b)",
    "H": "Hour (00\u201323)",
    "I": "Hour (01\u201312)",
    "j": "Day of year (001\u2013366)",
    "J": "Julian day number",
    "k": "Hour (0\u201323, no leading zero)",
    "l": "Hour (1\u201312, no leading zero)",
    "m": "Month (01\u201312)",
    "M": "Minute (00\u201359)",
    "N": "Month number (1\u201312, no leading zero)",
    "p": "AM/PM indicator (uppercase)",
    "P": "AM/PM indicator (lowercase)",
    "s": "Seconds since Unix epoch",
    "S": "Second (00\u201359)",
    "u": "Day of week (1=Monday\u20137=Sunday)",
    "U": "Week number (Sunday start, 00\u201353)",
    "V": "ISO 8601 week number (01\u201353)",
    "w": "Day of week (0=Sunday\u20136=Saturday)",
    "W": "Week number (Monday start, 00\u201353)",
    "x": "Locale date representation",
    "X": "Locale time representation",
    "y": "2-digit year (00\u201399)",
    "Y": "4-digit year",
    "z": "Timezone offset (+hhmm)",
    "Z": "Timezone abbreviation",
    "%": "Literal percent sign",
}

_BINARY_SPEC_DESC: dict[str, str] = {
    "a": "Byte string, padded with nulls",
    "A": "Byte string, padded with spaces",
    "b": "Binary digits (low-to-high order)",
    "B": "Binary digits (high-to-low order)",
    "h": "Hexadecimal digits (low-to-high nibble)",
    "H": "Hexadecimal digits (high-to-low nibble)",
    "c": "8-bit signed integer",
    "s": "16-bit signed integer (little-endian)",
    "S": "16-bit signed integer (big-endian)",
    "i": "32-bit signed integer (little-endian)",
    "I": "32-bit signed integer (big-endian)",
    "n": "32-bit integer (native byte order)",
    "w": "64-bit signed integer (little-endian)",
    "W": "64-bit signed integer (big-endian)",
    "m": "64-bit integer (native byte order)",
    "r": "32-bit float (little-endian)",
    "R": "32-bit float (big-endian)",
    "f": "32-bit float (native byte order)",
    "d": "64-bit double (native byte order)",
    "x": "Null padding byte (format) / skip byte (scan)",
    "X": "Move cursor back one byte",
    "@": "Move cursor to absolute position",
    "t": "Reserved (Tcl 8.5+)",
}

# Unit byte size for fixed-width integer/float specifiers
_BINARY_UNIT_BYTES: dict[str, int] = {
    "c": 1,
    "s": 2,
    "S": 2,
    "i": 4,
    "I": 4,
    "n": 4,
    "w": 8,
    "W": 8,
    "m": 8,
    "r": 4,
    "R": 4,
    "f": 4,
    "d": 8,
}

# Specifiers that don't consume a variable
_BINARY_NO_VAR = frozenset("xX@")

# Compact type labels for the detail table
_BINARY_SHORT_TYPE: dict[str, str] = {
    "a": "str (null-pad)",
    "A": "str (space-pad)",
    "b": "bits lo→hi",
    "B": "bits hi→lo",
    "h": "hex lo→hi",
    "H": "hex hi→lo",
    "c": "int8",
    "s": "int16 LE",
    "S": "int16 BE",
    "i": "int32 LE",
    "I": "int32 BE",
    "n": "int32 native",
    "w": "int64 LE",
    "W": "int64 BE",
    "m": "int64 native",
    "r": "float32 LE",
    "R": "float32 BE",
    "f": "float32 native",
    "d": "float64 native",
    "x": "pad/skip",
    "X": "back",
    "@": "seek",
    "t": "reserved",
}


def _binary_field_bytes(ch: str, count: int, star: bool) -> int | None:
    """Total byte size for one binary format field, or *None* if unknown."""
    if star:
        return None
    if ch in _BINARY_UNIT_BYTES:
        return _BINARY_UNIT_BYTES[ch] * count
    if ch in ("a", "A"):
        return count
    if ch in ("b", "B"):
        return -(-count // 8)  # ceil division
    if ch in ("h", "H"):
        return -(-count // 2)
    if ch == "x":
        return count
    return None


_REGSUB_BACKREF_DESC: dict[str, str] = {
    "&": "Entire matched string",
    "0": "Entire matched string",
    "1": "First capture group",
    "2": "Second capture group",
    "3": "Third capture group",
    "4": "Fourth capture group",
    "5": "Fifth capture group",
    "6": "Sixth capture group",
    "7": "Seventh capture group",
    "8": "Eighth capture group",
    "9": "Ninth capture group",
}

_GLOB_META_DESC: dict[str, str] = {
    "*": "Matches any sequence of characters",
    "?": "Matches any single character",
    "[": "Character class \u2014 matches any character inside brackets",
}

_REGEX_META_DESC: dict[str, str] = {
    "^": "Start of line/string anchor",
    "$": "End of line/string anchor",
    ".": "Match any single character",
    "*": "Zero or more (greedy)",
    "+": "One or more (greedy)",
    "?": "Zero or one (greedy)",
    "*?": "Zero or more (lazy)",
    "+?": "One or more (lazy)",
    "??": "Zero or one (lazy)",
    "|": "Alternation (OR)",
}

_REGEX_ESCAPE_DESC: dict[str, str] = {
    "\\d": "Digit `[0-9]`",
    "\\D": "Non-digit",
    "\\s": "Whitespace",
    "\\S": "Non-whitespace",
    "\\w": "Word character `[a-zA-Z0-9_]`",
    "\\W": "Non-word character",
    "\\b": "Word boundary",
    "\\B": "Non-word boundary",
    "\\m": "Beginning of word (Tcl ARE)",
    "\\M": "End of word (Tcl ARE)",
    "\\y": "Word boundary (Tcl ARE)",
    "\\Y": "Non-word boundary (Tcl ARE)",
    "\\A": "Start of string (Tcl ARE)",
    "\\Z": "End of string (Tcl ARE)",
    "\\a": "Bell character",
    "\\e": "Escape character",
    "\\f": "Form feed",
    "\\n": "Newline",
    "\\r": "Carriage return",
    "\\t": "Tab",
    "\\v": "Vertical tab",
}

_REGEX_FLAG_DESC: dict[str, str] = {
    "i": "case-insensitive",
    "m": "multiline (`^` `$` match line boundaries)",
    "n": "newline-sensitive (`.` doesn't match `\\n`)",
    "s": "single-line (`.` matches `\\n`)",
    "x": "expanded syntax (whitespace/comments ignored)",
    "w": "inverse partial newline-sensitive",
    "p": "partial newline-sensitive",
    "q": "all characters are literals",
}


def _find_token_at_position(
    line_text: str,
    character: int,
) -> tuple[list[str], int, str | None]:
    """Parse *line_text* and return (argv_texts, arg_index, token_text).

    *argv_texts* is the list of argument texts (including command name at [0]).
    *arg_index* is the 0-based index of the argument containing the cursor.
    *token_text* is the full text of the Tcl token at the cursor, or None.
    """
    lexer = TclLexer(line_text)
    argv_texts: list[str] = []
    token_list: list = []  # (arg_idx, token)
    prev_type = TokenType.EOL
    arg_idx = -1

    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            break
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if prev_type in (TokenType.SEP, TokenType.EOL):
            arg_idx += 1
            argv_texts.append(tok.text)
        else:
            if argv_texts:
                argv_texts[-1] += tok.text
        token_list.append((arg_idx, tok))
        prev_type = tok.type

    # Find which token the cursor falls inside
    hit_arg = -1
    hit_text = None
    for a_idx, tok in token_list:
        start = tok.start.character
        end = tok.end.character + 1
        if start <= character < end:
            hit_arg = a_idx
            # Return the full argument text (from argv_texts)
            if 0 <= a_idx < len(argv_texts):
                hit_text = argv_texts[a_idx]
            break

    return argv_texts, hit_arg, hit_text


def _format_string_hover(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> types.Hover | None:
    """Check if the cursor is inside a format string and return a decoded hover."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return None
    line_text = lines[line]

    argv_texts, hit_arg, hit_text = _find_token_at_position(line_text, character)
    if hit_arg < 0 or hit_text is None or not argv_texts:
        return None

    cmd_name = argv_texts[0]

    # Determine which format type this argument is
    fmt_type = None
    if _sprintf_format_arg_index(cmd_name, argv_texts) == hit_arg:
        fmt_type = "sprintf"
    elif _binary_format_arg_index(cmd_name, argv_texts) == hit_arg:
        fmt_type = "binary"
    elif _clock_format_arg_index(cmd_name, argv_texts) == hit_arg:
        fmt_type = "clock"
    elif _regsub_subspec_arg_index(cmd_name, argv_texts) == hit_arg:
        fmt_type = "regsub"
    elif _regex_pattern_arg_index(cmd_name, argv_texts) == hit_arg:
        fmt_type = "regex"
    elif hit_arg in _glob_pattern_arg_indices(cmd_name, argv_texts):
        fmt_type = "glob"

    if fmt_type is None:
        return None

    # Build a full decoded breakdown of the format string
    if fmt_type == "sprintf":
        return _sprintf_hover(hit_text)
    if fmt_type == "binary":
        return _binary_hover(hit_text, argv_texts)
    if fmt_type == "clock":
        return _clock_hover(hit_text)
    if fmt_type == "regsub":
        return _regsub_hover(hit_text)
    if fmt_type == "regex":
        return _regex_hover(hit_text)
    if fmt_type == "glob":
        return _glob_hover(hit_text)

    return None


def _make_hover(value: str) -> types.Hover:
    return types.Hover(
        contents=types.MarkupContent(
            kind=types.MarkupKind.Markdown,
            value=value,
        ),
    )


def _sprintf_hover(text: str) -> types.Hover:
    parts = ["**Format string** (sprintf-style)\n"]
    matches = list(_SPRINTF_RE.finditer(text))
    if not matches:
        parts.append("No specifiers found.")
    else:
        parts.append("| Specifier | Meaning |")
        parts.append("|-----------|---------|")
        for m in matches:
            spec = m.group()
            type_char = m.group("type")
            desc = _SPRINTF_SPEC_DESC.get(type_char, "Unknown")
            extras = []
            if m.group("position"):
                extras.append(f"position {m.group('position')}")
            if m.group("flags"):
                extras.append(f"flags `{m.group('flags')}`")
            if m.group("width"):
                extras.append(f"width {m.group('width')}")
            if m.group("precision"):
                extras.append(f"precision .{m.group('precision')}")
            detail = f" ({', '.join(extras)})" if extras else ""
            parts.append(f"| `{spec}` | {desc}{detail} |")
    return _make_hover("\n".join(parts))


def _binary_hover(
    text: str,
    argv_texts: list[str] | None = None,
) -> types.Hover:
    """Rich hover for ``binary format``/``binary scan`` format strings.

    Produces a byte-ruler diagram followed by a detail table.
    """
    # 1. Parse the format string
    # Tcl binary format order: type [modifier] [count|*]
    fields: list[
        tuple[str, str, int, str, bool, int | None, bool]
    ] = []  # (full, ch, count, mod, star, byte_size, consumes_var)
    idx = 0
    while idx < len(text):
        if text[idx] in " \t\r\n":
            idx += 1
            continue
        ch = text[idx]
        if ch not in _BINARY_FORMAT_SPECIFIERS:
            idx += 1
            continue
        idx += 1
        mod = ""
        if idx < len(text) and text[idx] in ("u", "s"):
            mod = text[idx]
            idx += 1
        star = False
        count_str = ""
        if idx < len(text) and text[idx] == "*":
            star = True
            idx += 1
        else:
            while idx < len(text) and text[idx].isdigit():
                count_str += text[idx]
                idx += 1
        count = int(count_str) if count_str else 1
        full = f"{ch}{mod}{count_str}{'*' if star else ''}"
        byte_size = _binary_field_bytes(ch, count, star)
        consumes = ch not in _BINARY_NO_VAR
        fields.append((full, ch, count, mod, star, byte_size, consumes))

    if not fields:
        return _make_hover("**Binary format string**\n\nNo specifiers found.")

    # 2. Determine context (scan vs format, variable names)
    is_scan = argv_texts is not None and len(argv_texts) > 1 and argv_texts[1] == "scan"
    subcmd = "scan" if is_scan else "format"

    # Trailing args are variable names (scan) or value expressions (format).
    trailing: list[str] = []
    if argv_texts:
        fmt_idx = 3 if is_scan else 2
        trailing = list(argv_texts[fmt_idx + 1 :])

    # Map each consuming field → its variable / arg name (if known).
    var_idx = 0
    field_labels: list[str] = []
    for full, _ch, _cnt, _mod, _star, _bs, consumes in fields:
        if consumes and var_idx < len(trailing):
            field_labels.append(trailing[var_idx])
            var_idx += 1
        else:
            field_labels.append(full)

    # Resolve effective byte deltas, including absolute seek (@N).
    effective_bytes: list[int | None] = []
    cursor = 0
    has_backward_seek = False
    for _full, ch, count, _mod, star, byte_size, _consumes in fields:
        if ch == "@":
            if star:
                effective_bytes.append(None)
                continue
            target = count
            if target < cursor:
                effective_bytes.append(0)
                has_backward_seek = True
            else:
                effective_bytes.append(target - cursor)
            cursor = target
            continue
        if byte_size is None:
            effective_bytes.append(None)
            continue
        effective_bytes.append(byte_size)
        cursor += byte_size

    # 3. Summary line
    n_vars = sum(1 for f in fields if f[6])
    total_known = sum(bs for bs in effective_bytes if bs is not None and bs > 0)
    has_unknown = any(bs is None for bs in effective_bytes)
    size_str = f"{total_known}{'+ ' if has_unknown else ''} bytes"

    parts: list[str] = [
        f"**binary {subcmd}** — {n_vars} field{'s' if n_vars != 1 else ''}, {size_str}\n",
    ]

    # 4. Byte-ruler diagram (skip if unknowns or > 32 bytes)
    can_diagram = (
        not has_backward_seek
        and all(bs is not None and bs > 0 for bs in effective_bytes)
        and len(effective_bytes) > 0
    )
    if can_diagram and 0 < total_known <= 32:
        CPB = 4  # chars per byte in the ruler
        ruler = "      "
        for b in range(total_known):
            ruler += f"{b:<{CPB}}"

        top = "      "
        mid = "      "
        bot = "      "
        for j, (full, _ch, _cnt, _mod, _star, _bs, _con) in enumerate(fields):
            bs = effective_bytes[j]
            assert bs is not None
            w = CPB * bs - 1
            label = field_labels[j]
            sep_t = "┌" if j == 0 else "┬"
            sep_b = "└" if j == 0 else "┴"
            top += sep_t + "─" * w
            mid += "│" + label[:w].center(w)
            bot += sep_b + "─" * w
        top += "┐"
        mid += "│"
        bot += "┘"

        parts.append("```")
        parts.extend([ruler, top, mid, bot])
        parts.append("```\n")

    # 5. Detail table
    parts.append("| Spec | Variable | Type | Bytes |")
    parts.append("|------|----------|------|------:|")
    for j, (full, ch, count, mod, star, _bs, consumes) in enumerate(fields):
        var = field_labels[j] if consumes else "—"
        typ = _BINARY_SHORT_TYPE.get(ch, "?")
        if mod == "u":
            typ = typ.replace("int", "uint")
        if count > 1 and ch in _BINARY_UNIT_BYTES:
            typ += f" ×{count}"
        eff_bs = effective_bytes[j]
        bs_str = str(eff_bs) if eff_bs is not None else "?"
        if star:
            bs_str = "…"
        elif ch == "@" and eff_bs is not None:
            bs_str = str(eff_bs)
        parts.append(f"| `{full}` | {var} | {typ} | {bs_str} |")

    return _make_hover("\n".join(parts))


def _clock_hover(text: str) -> types.Hover:
    parts = ["**Clock format string** (strftime-style)\n"]
    matches = list(_CLOCK_FORMAT_RE.finditer(text))
    if not matches:
        parts.append("No specifiers found.")
    else:
        parts.append("| Specifier | Meaning |")
        parts.append("|-----------|---------|")
        for m in matches:
            spec = m.group()
            # Get the final letter for lookup
            letter = spec[-1]
            desc = _CLOCK_SPEC_DESC.get(letter, "Unknown")
            if len(spec) == 3:  # %E* or %O* locale modifier
                desc = f"{desc} (locale-modified)"
            parts.append(f"| `{spec}` | {desc} |")
    return _make_hover("\n".join(parts))


def _regsub_hover(text: str) -> types.Hover:
    parts = ["**Substitution spec** (regsub)\n"]
    matches = list(_REGSUB_BACKREF_RE.finditer(text))
    if not matches:
        parts.append("No backreferences found.")
    else:
        parts.append("| Reference | Meaning |")
        parts.append("|-----------|---------|")
        for m in matches:
            ref = m.group()
            char = m.group(1)
            desc = _REGSUB_BACKREF_DESC.get(char, "Unknown")
            parts.append(f"| `{ref}` | {desc} |")
    return _make_hover("\n".join(parts))


def _glob_hover(text: str) -> types.Hover:
    parts = ["**Glob pattern**\n"]
    matches = list(_GLOB_META_RE.finditer(text))
    if not matches:
        parts.append("Literal string (no metacharacters).")
    else:
        parts.append("| Pattern | Meaning |")
        parts.append("|---------|---------|")
        seen: set[str] = set()
        for m in matches:
            matched = m.group()
            if matched.startswith("\\"):
                key = "escape"
                desc = f"Escaped character `{matched[1:]}`"
            elif matched.startswith("["):
                key = matched
                desc = f"Character class: matches any of `{matched[1:-1]}`"
            else:
                key = matched
                desc = _GLOB_META_DESC.get(matched, "Unknown")
            if key not in seen:
                parts.append(f"| `{matched}` | {desc} |")
                seen.add(key)
    return _make_hover("\n".join(parts))


def _describe_regex_component(matched: str) -> tuple[str, str]:
    """Return (dedup_key, description) for a regex component."""
    if matched.startswith("["):
        inner = matched[1:-1] if matched.endswith("]") else matched[1:]
        return (matched, f"Character class: matches any of `{inner}`")
    # Embedded flags like (?imsx)
    if (
        matched.startswith("(?")
        and matched.endswith(")")
        and not matched.startswith("(?:")
        and not matched.startswith("(?=")
        and not matched.startswith("(?!")
        and not matched.startswith("(?>")
    ):
        flags = matched[2:-1]
        parts = []
        for f in flags:
            if f == "-":
                continue
            parts.append(_REGEX_FLAG_DESC.get(f, f))
        return ("(?flags)", f"Embedded flags: {', '.join(parts)}")
    if matched.startswith("(?:"):
        return ("(?:", "Non-capturing group")
    if matched.startswith("(?="):
        return ("(?=", "Positive lookahead")
    if matched.startswith("(?!"):
        return ("(?!", "Negative lookahead")
    if matched.startswith("(?>"):
        return ("(?>", "Atomic (possessive) group")
    if matched == "(":
        return ("(", "Capture group open")
    if matched == ")":
        return (")", "Group close")
    if matched.startswith("\\") and len(matched) >= 2:
        ch = matched[1]
        if ch.isdigit():
            return (matched, f"Backreference to group {ch}")
        desc = _REGEX_ESCAPE_DESC.get(matched)
        if desc is not None:
            return (matched, desc)
        if matched.startswith("\\x") or matched.startswith("\\u") or matched.startswith("\\U"):
            return (matched, f"Unicode/hex escape `{matched}`")
        if ch in ".*+?(){}[]|^$\\":
            return (f"\\{ch}", f"Escaped literal `{ch}`")
        return (matched, f"Escape sequence `{matched}`")
    if matched.startswith("{") and matched.endswith("}"):
        return (matched, f"Bounded quantifier: repeat `{matched[1:-1]}` times")
    desc = _REGEX_META_DESC.get(matched)
    if desc:
        return (matched, desc)
    return (matched, "Regex metacharacter")


def _regex_hover(text: str) -> types.Hover:
    parts = ["**Regex pattern**\n"]
    matches = list(_REGEX_PART_RE.finditer(text))
    if not matches:
        parts.append("Literal string (no metacharacters).")
    else:
        parts.append("| Component | Meaning |")
        parts.append("|-----------|---------|")
        seen: set[str] = set()
        for m in matches:
            matched = m.group()
            key, desc = _describe_regex_component(matched)
            if key not in seen:
                parts.append(f"| `{matched}` | {desc} |")
                seen.add(key)
    return _make_hover("\n".join(parts))


def _ip_address_hover(word: str) -> types.Hover | None:
    """Return hover info if *word* is a valid IPv4 or IPv6 address (with optional /prefix)."""
    # Quick reject — must contain a dot or colon
    if "." not in word and ":" not in word:
        return None
    info = parse_ip(word)
    if info is None:
        return None
    return _make_hover(format_ip_hover(info))


def get_hover(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> types.Hover | None:
    """Generate hover info for a position in source."""
    if analysis is None:
        analysis = analyse(source)

    # Check for variable hover ($var)
    var_name = find_var_at_position(source, line, character, lines=lines)
    if var_name:
        scope = find_scope_at_line(analysis.global_scope, line)
        # Walk up scope chain looking for variable
        current: Scope | None = scope
        while current is not None:
            if var_name in current.variables:
                type_info = _infer_var_type(source, var_name)
                taint_info = _infer_var_taint(source, var_name)
                return types.Hover(
                    contents=types.MarkupContent(
                        kind=types.MarkupKind.Markdown,
                        value=_var_hover_text(
                            current.variables[var_name],
                            type_info,
                            taint_info,
                        ),
                    ),
                )
            current = current.parent
        return None

    # Check for format-string hover (must come before word_span which splits on %)
    fmt_hover = _format_string_hover(source, line, character, lines=lines)
    if fmt_hover is not None:
        return fmt_hover

    word_span = find_word_span_at_position(source, line, character, lines=lines)
    if word_span is None:
        return None
    word, _word_start, word_end = word_span

    # Check user-defined procs first
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == word or qname == word or qname == f"::{word}":
            return types.Hover(
                contents=types.MarkupContent(
                    kind=types.MarkupKind.Markdown,
                    value=_proc_hover_text(proc_def),
                ),
            )

    # Check for IP address hover
    ip_hover = _ip_address_hover(word)
    if ip_hover is not None:
        return ip_hover

    dialect = active_dialect()
    active_packages = analysis.active_package_names()
    if lines is None:
        lines = source.split("\n")
    line_text = lines[line] if line < len(lines) else ""
    context_cmd, _context_word, word_index = find_command_context_in_line(line_text, word_end)

    # Command-position hover from registry docs.
    # Try package-filtered first; fall back without filtering for
    # package-gated commands so they still show docs (with an import
    # hint) when the corresponding ``package require`` is missing.
    # The fallback and hint are skipped when the dialect does not
    # support ``package require`` (e.g. iRules).
    cmd_spec = REGISTRY.get(word, dialect, active_packages=active_packages)
    _pkg_available = REGISTRY.get("package", dialect) is not None
    if cmd_spec is None and _pkg_available and REGISTRY.has_required_package(word):
        cmd_spec = REGISTRY.get(word, dialect)
    if cmd_spec and cmd_spec.hover is not None:
        value = cmd_spec.hover.render_hover_lean(word)
        # Import hint when the required package is not imported.
        if (
            _pkg_available
            and cmd_spec.required_package
            and cmd_spec.required_package not in active_packages
        ):
            value = f"{value}\n\n**Requires**: `package require {cmd_spec.required_package}`"
        # Append valid events info for iRules commands.
        if dialect == "f5-irules":
            if cmd_spec.event_requires is not None:
                matching = EVENT_REGISTRY.events_matching(cmd_spec.event_requires)
                if matching:
                    sample = matching[:8]
                    event_list = ", ".join(f"`{e}`" for e in sample)
                    if len(matching) > 8:
                        event_list += f", ... ({len(matching)} total)"
                    value = f"{value}\n\n**Valid events**: {event_list}"
                reqs: list[str] = []
                er = cmd_spec.event_requires
                if er.client_side:
                    reqs.append("client-side")
                if er.server_side:
                    reqs.append("server-side")
                if er.transport:
                    reqs.append(er.transport.upper())
                if er.profiles:
                    reqs.append("profile " + " or ".join(sorted(er.profiles)))
                if reqs:
                    value = f"{value}\n\n**Requires**: {', '.join(reqs)}"
        return types.Hover(
            contents=types.MarkupContent(
                kind=types.MarkupKind.Markdown,
                value=value,
            ),
        )

    # Expression operator hover (eq/ne/in/ni + iRules contains, starts_with, etc.)
    op_hover = operator_hover(word, dialect)
    if op_hover is not None:
        return types.Hover(
            contents=types.MarkupContent(
                kind=types.MarkupKind.Markdown,
                value=op_hover.render_hover_lean(word),
            ),
        )

    # Option hover snippets (e.g. socket -server) and value hover snippets
    if context_cmd:
        if word.startswith("-"):
            opt = REGISTRY.option(context_cmd, word, dialect, active_packages=active_packages)
            if opt and opt.hover is not None:
                value = opt.hover.render_hover_lean(f"{context_cmd} {word}")
                return types.Hover(
                    contents=types.MarkupContent(
                        kind=types.MarkupKind.Markdown,
                        value=value,
                    ),
                )

        arg_index = word_index - 1
        if arg_index >= 0:
            value_spec = REGISTRY.argument_value(
                context_cmd, arg_index, word, dialect, active_packages=active_packages
            )
            # Try subcommand-scoped value (e.g. "string is integer").
            if value_spec is None and arg_index >= 1:
                sig = SIGNATURES.get(context_cmd)
                if isinstance(sig, SubcommandSig):
                    from .symbol_resolution import find_command_context_details_at_position

                    _cmd_d, args_d, _, _ = find_command_context_details_at_position(
                        source,
                        line,
                        character,
                    )
                    if args_d:
                        sub_arg_index = arg_index - 1
                        value_spec = REGISTRY.subcommand_argument_value(
                            context_cmd,
                            args_d[0],
                            sub_arg_index,
                            word,
                            dialect,
                            active_packages=active_packages,
                        )
            if value_spec and value_spec.hover is not None:
                value = value_spec.hover.render_hover_lean(f"{context_cmd} {word}")
                # Add event ordering info for `when EVENT` hover
                if (
                    context_cmd == "when"
                    and arg_index == 0
                    and EVENT_REGISTRY.event_index(word) is not None
                ):
                    file_events = EVENT_REGISTRY.scan_file_events(source)
                    if len(file_events) > 1:
                        ordered = EVENT_REGISTRY.order_events(file_events)
                        if word in ordered:
                            pos = ordered.index(word) + 1
                            total = len(ordered)
                            value = f"{value}\n\n**Event order**: {pos} of {total} in this file"
                            if total <= 12:
                                chain_parts = []
                                for evt in ordered:
                                    if evt == word:
                                        chain_parts.append(f"**{evt}**")
                                    else:
                                        chain_parts.append(evt)
                                value = f"{value}  \n{' → '.join(chain_parts)}"
                    # Multiplicity note
                    _mult = EVENT_REGISTRY.event_multiplicity(word)
                    if _mult == "per_request":
                        value += (
                            "\n\n**Multiplicity**: per-request "
                            "— fires once per HTTP transaction "
                            "(repeats on keep-alive)"
                        )
                    elif _mult == "init":
                        value += (
                            "\n\n**Multiplicity**: once at iRule load "
                            "— use `static::` variables for "
                            "persistent state"
                        )
                    elif _mult == "once_per_connection":
                        value += (
                            "\n\n**Multiplicity**: once per connection "
                            "— variables set here persist for "
                            "the entire TCP session"
                        )
                return types.Hover(
                    contents=types.MarkupContent(
                        kind=types.MarkupKind.Markdown,
                        value=value,
                    ),
                )

    # Fallback: known command with subcommands
    if word in SIGNATURES:
        sig = SIGNATURES[word]
        if isinstance(sig, SubcommandSig):
            subs = ", ".join(sorted(sig.subcommands.keys()))
            return types.Hover(
                contents=types.MarkupContent(
                    kind=types.MarkupKind.Markdown,
                    value=f"**{word}** subcommand\n\nSubcommands: {subs}",
                ),
            )

    return None
