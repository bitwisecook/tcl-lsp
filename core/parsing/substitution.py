"""Tcl backslash escape processing.

Shared by the compiler (for expr string literals inside braces) and
the VM runtime substitution engine.
"""

from __future__ import annotations

# Backslash escape mapping

_BACKSLASH_MAP: dict[str, str] = {
    "a": "\a",
    "b": "\b",
    "f": "\f",
    "n": "\n",
    "r": "\r",
    "t": "\t",
    "v": "\v",
    "\\": "\\",
    "{": "{",
    "}": "}",
    "[": "[",
    "]": "]",
    "$": "$",
    '"': '"',
    " ": " ",
    ";": ";",
}

# Maximum Unicode codepoint (U+10FFFF).  Tcl 9 silently clamps overlong
# ``\xNN`` / ``\uNNNN`` / ``\UNNNNNNNN`` escapes that exceed this; we
# do the same so the lexer doesn't bail out on test files that
# intentionally exercise the boundary.
_MAX_UNICODE = 0x10FFFF


def _clamp_unicode(value: int) -> int:
    """Return *value* clamped to the Unicode range so :func:`chr` succeeds."""
    if value < 0:
        return 0
    if value > _MAX_UNICODE:
        return _MAX_UNICODE
    return value


# Hex digit lookup — maps each hex char to its numeric value.
_HEX_DIGITS: dict[str, int] = {
    **{c: int(c) for c in "0123456789"},
    **{c: 10 + ord(c) - ord("a") for c in "abcdef"},
    **{c: 10 + ord(c) - ord("A") for c in "ABCDEF"},
}


def _parse_hex_capped(text: str, start: int, max_digits: int) -> tuple[int, int]:
    """Mirror Tcl 9's ``ParseHex`` in tclParse.c:728.

    Scan up to ``max_digits`` hex characters starting at *start*, but
    stop early if the accumulated value would exceed U+10FFFF after one
    more shift.  The check is ``result > 0x10FFF`` BEFORE shifting, so
    after the shift the maximum representable value is exactly
    0x10FFFF (the Unicode upper bound).  This means ``\\U100000b``
    consumes only the 6 digits ``100000`` (= U+100000); the trailing
    ``b`` falls through to the next iteration as a literal.

    Returns ``(value, end_index)`` where *end_index* is the position
    one past the last consumed hex digit.  When no hex digits follow,
    returns ``(0, start)``.
    """
    n = len(text)
    value = 0
    i = start
    end = min(start + max_digits, n)
    while i < end:
        digit = _HEX_DIGITS.get(text[i])
        if digit is None or value > 0x10FFF:
            break
        value = (value << 4) | digit
        i += 1
    return value, i


def backslash_subst(text: str) -> str:
    """Process Tcl backslash escapes in *text*.

    Handles ``\\a \\b \\f \\n \\r \\t \\v \\\\ \\xNN \\uNNNN \\UNNNNNNNN``
    and octal escapes ``\\NNN``, plus ``\\<newline>`` (continuation lines).
    """
    result: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        if text[i] == "\\" and i + 1 < n:
            c = text[i + 1]
            if c in _BACKSLASH_MAP:
                result.append(_BACKSLASH_MAP[c])
                i += 2
            elif c == "\n" or c == "\r":
                # continuation line — skip newline and leading whitespace
                i += 2
                # Consume LF half of CRLF.
                if c == "\r" and i < n and text[i] == "\n":
                    i += 1
                while i < n and text[i] in " \t":
                    i += 1
                result.append(" ")
            elif c == "x":
                # ``\xNN`` — Tcl 9 reads up to 2 hex digits then masks
                # to UCHAR (low 8 bits).  Since 2 digits never exceeds
                # 0xFF, the ParseHex 0x10FFF cap never triggers here.
                value, j = _parse_hex_capped(text, i + 2, 2)
                if j > i + 2:
                    result.append(chr(value & 0xFF))
                    i = j
                else:
                    result.append("x")
                    i += 2
            elif c == "u":
                # ``\uNNNN`` — Tcl 9 reads up to 4 hex digits.  Tcl 9
                # does NOT combine adjacent surrogate pairs at parse
                # time — ``😂`` produces two isolated
                # surrogate codepoints (encoded as 3-byte WTF-8 each).
                # The runtime preserves them; do not combine here.
                value, j = _parse_hex_capped(text, i + 2, 4)
                if j > i + 2:
                    result.append(chr(value))
                    i = j
                else:
                    result.append("u")
                    i += 2
            elif c == "U":
                # ``\UNNNNNNNN`` — Tcl 9 reads up to 8 hex digits but
                # stops when the accumulator would exceed 0x10FFFF
                # (see tclParse.c:728 ParseHex).  Example:
                # ``\U100000b`` consumes the 6 digits ``100000``
                # → U+100000, leaving ``b`` as a literal.
                value, j = _parse_hex_capped(text, i + 2, 8)
                if j > i + 2:
                    result.append(chr(value))
                    i = j
                else:
                    result.append("U")
                    i += 2
            elif c in "01234567":
                # octal escape: \NNN (1-3 octal digits)
                j = i + 1
                while j < n and j < i + 4 and text[j] in "01234567":
                    j += 1
                result.append(chr(int(text[i + 1 : j], 8)))
                i = j
            else:
                # Unknown escape — just keep the character
                result.append(c)
                i += 2
        else:
            result.append(text[i])
            i += 1
    return "".join(result)
