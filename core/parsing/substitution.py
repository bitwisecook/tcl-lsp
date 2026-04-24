"""Tcl backslash escape processing.

Shared by the compiler (for expr string literals inside braces) and
the VM runtime substitution engine.

The primary implementation is provided by the Rust `tcl_lsp_rust`
extension module (see `rust/tcl-lexer/src/substitution.rs`). A
pure-Python fallback is still kept in this file for developer
environments that have not yet built the Rust wheel; the broader
Python-to-Rust rewrite is described in `docs/rust-rewrite.md`.
"""

from __future__ import annotations

from typing import Callable

try:
    from tcl_lsp_rust import (
        backslash_subst as _backslash_subst_rust,  # ty: ignore[unresolved-import]
    )
except ImportError:
    _backslash_subst_rust = None

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


def _backslash_subst_python(text: str) -> str:
    """Pure-Python fallback for :func:`backslash_subst`.

    Kept in lock-step with the Rust implementation; update both in the
    same commit until the fallback is removed. See
    ``rust/tcl-lexer/src/substitution.rs`` for the source of truth.
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


#: Public entry point. Dispatches to the Rust implementation when the
#: `tcl_lsp_rust` extension is installed; otherwise falls back to the
#: pure-Python implementation above. Module-level assignment means
#: callers pay no per-call dispatch cost.
backslash_subst: Callable[[str], str] = (
    _backslash_subst_rust if _backslash_subst_rust is not None else _backslash_subst_python
)
