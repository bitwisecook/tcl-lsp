"""Wire-format markers shared by the compiler and the VM.

These three constants are not user-visible: they're embedded inside literal
strings during compilation and stripped out again by the runtime. They exist
because the bytecode argument vector is just `list[str]`, so the compiler has
no other way to tell the runtime "this argument was originally braced" or
"this string contains raw ``$`` / ``[`` characters that must NOT be
substituted".

``_BRACE_OPEN`` / ``_BRACE_CLOSE`` mark a braced word. They use NUL bytes
plus a brace so no legitimate Tcl source can produce them by accident.

``_RAW_PREFIX`` marks a literal whose ``$`` / ``[`` characters came from
backslash escapes and must be pushed verbatim.
"""

from __future__ import annotations

_BRACE_OPEN = "\x00\x01{"
_BRACE_CLOSE = "}\x01\x00"
_RAW_PREFIX = "\x00\x02"

__all__ = ["_BRACE_OPEN", "_BRACE_CLOSE", "_RAW_PREFIX"]
