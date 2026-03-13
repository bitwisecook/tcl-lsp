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
            elif c == "\n":
                # continuation line — skip newline and leading whitespace
                i += 2
                while i < n and text[i] in " \t":
                    i += 1
                result.append(" ")
            elif c == "x":
                # hex escape: \xNN (1-2 hex digits)
                j = i + 2
                while j < n and j < i + 4 and text[j] in "0123456789abcdefABCDEF":
                    j += 1
                if j > i + 2:
                    result.append(chr(int(text[i + 2 : j], 16)))
                    i = j
                else:
                    result.append("x")
                    i += 2
            elif c == "u":
                # unicode escape: \uNNNN (1-4 hex digits)
                j = i + 2
                while j < n and j < i + 6 and text[j] in "0123456789abcdefABCDEF":
                    j += 1
                if j > i + 2:
                    result.append(chr(int(text[i + 2 : j], 16)))
                    i = j
                else:
                    result.append("u")
                    i += 2
            elif c == "U":
                # wide unicode escape: \UNNNNNNNN (1-8 hex digits)
                j = i + 2
                while j < n and j < i + 10 and text[j] in "0123456789abcdefABCDEF":
                    j += 1
                if j > i + 2:
                    result.append(chr(int(text[i + 2 : j], 16)))
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
