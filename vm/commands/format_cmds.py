"""Formatting commands: format, scan."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

# Tcl format specifiers — supports flags, star-width, star-precision,
# length modifiers (h/l/ll), and all Tcl conversion characters.
_FMT_RE = re.compile(
    r"%"
    r"(?P<flags>[-+ #0]*)"
    r"(?P<width>\*|\d+)?"
    r"(?:\.(?P<prec>\*|\d+))?"
    r"(?:h{1,2}|l{1,2})?"  # length modifiers — ignored in our bignum VM
    r"(?P<conv>[diouxXeEfgGcsb%])"
)


def _format_int(n: int, conv: str, flags: str, width: int, prec: int) -> str:
    """Format a single integer value according to Tcl format rules."""
    if conv == "b":
        # Tcl %b — binary representation
        if n < 0:
            n = n & 0xFFFFFFFF
        raw = bin(n)[2:]
        prefix = "0b" if "#" in flags else ""
    elif conv in "di":
        raw = str(abs(n))
        sign = "-" if n < 0 else ("+" if "+" in flags else (" " if " " in flags else ""))
        if prec > 0:
            raw = raw.zfill(prec)
        total = len(sign) + len(raw)
        if width > total:
            pad = width - total
            if "-" in flags:
                return sign + raw + " " * pad
            if "0" in flags and prec < 0:
                return sign + "0" * pad + raw
            return " " * pad + sign + raw
        return sign + raw
    elif conv == "o":
        if n < 0:
            n = n & 0xFFFFFFFF
        raw = oct(n)[2:]
        prefix = "0o" if "#" in flags and n != 0 else ""
    elif conv in "xX":
        if n < 0:
            n = n & 0xFFFFFFFF
        raw = hex(n)[2:]
        if conv == "X":
            raw = raw.upper()
        # Tcl always uses lowercase prefix (0x/0X → 0x)
        if "#" in flags and n != 0:
            prefix = "0x"
        else:
            prefix = ""
    elif conv == "u":
        if n < 0:
            n = n & 0xFFFFFFFF
        raw = str(n)
        prefix = ""
    else:
        raw = str(n)
        prefix = ""

    total = len(prefix) + len(raw)
    if width > total:
        pad = width - total
        if "-" in flags:
            return prefix + raw + " " * pad
        if "0" in flags:
            return prefix + "0" * pad + raw
        return " " * pad + prefix + raw
    return prefix + raw


def _cmd_format(interp: TclInterp, args: list[str]) -> TclResult:
    """format formatString ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "format formatString ?arg ...?"')

    fmt = args[0]
    values = args[1:]
    result: list[str] = []
    val_idx = 0
    i = 0

    while i < len(fmt):
        if fmt[i] == "%" and i + 1 < len(fmt):
            if fmt[i + 1] == "%":
                result.append("%")
                i += 2
                continue

            m = _FMT_RE.match(fmt, i)
            if m:
                conv = m.group("conv")
                if conv == "%":
                    result.append("%")
                    i = m.end()
                    continue

                flags = m.group("flags") or ""

                # Resolve width (may be * = from next arg)
                w = m.group("width")
                if w == "*":
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    width = int(values[val_idx], 0)
                    val_idx += 1
                    # Negative star-width means left-justify
                    if width < 0:
                        flags += "-"
                        width = -width
                else:
                    width = int(w) if w else 0

                # Resolve precision (may be * = from next arg)
                p = m.group("prec")
                if p == "*":
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    prec = int(values[val_idx], 0)
                    val_idx += 1
                else:
                    prec = int(p) if p else -1

                if val_idx >= len(values):
                    raise TclError("not enough arguments for all format specifiers")
                val = values[val_idx]
                val_idx += 1

                if conv in "diouxXb":
                    n = int(val, 0)
                    result.append(_format_int(n, conv, flags, width, prec))
                elif conv in "eEfgG":
                    # Build Python format spec
                    py_spec = "%" + flags
                    if width:
                        py_spec += str(width)
                    if prec >= 0:
                        py_spec += "." + str(prec)
                    py_spec += conv
                    result.append(py_spec % float(val))
                elif conv == "c":
                    ch = chr(int(val, 0))
                    if width > len(ch):
                        pad = width - len(ch)
                        if "-" in flags:
                            ch = ch + " " * pad
                        else:
                            ch = " " * pad + ch
                    result.append(ch)
                elif conv == "s":
                    s = val
                    if prec >= 0:
                        s = s[:prec]
                    if width > len(s):
                        pad = width - len(s)
                        if "-" in flags:
                            s = s + " " * pad
                        else:
                            s = " " * pad + s
                    result.append(s)

                i = m.end()
                continue
        result.append(fmt[i])
        i += 1

    return TclResult(value="".join(result))


def _cmd_scan(interp: TclInterp, args: list[str]) -> TclResult:
    """scan string format ?varName ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "scan string format ?varName ...?"')

    string = args[0]
    fmt = args[1]
    var_names = args[2:]

    # Simple scanf-like implementation
    values: list[str] = []
    str_pos = 0
    fmt_pos = 0

    while fmt_pos < len(fmt) and str_pos <= len(string):
        if fmt[fmt_pos] == "%" and fmt_pos + 1 < len(fmt):
            fmt_pos += 1
            if fmt[fmt_pos] == "%":
                if str_pos < len(string) and string[str_pos] == "%":
                    str_pos += 1
                fmt_pos += 1
                continue

            # Skip width specifier
            while fmt_pos < len(fmt) and fmt[fmt_pos].isdigit():
                fmt_pos += 1

            if fmt_pos >= len(fmt):
                break

            conv = fmt[fmt_pos]
            fmt_pos += 1

            if conv == "d" or conv == "i":
                start = str_pos
                if str_pos < len(string) and string[str_pos] in "+-":
                    str_pos += 1
                while str_pos < len(string) and string[str_pos].isdigit():
                    str_pos += 1
                if str_pos > start:
                    values.append(str(int(string[start:str_pos])))
            elif conv == "f" or conv in "eEgG":
                start = str_pos
                if str_pos < len(string) and string[str_pos] in "+-":
                    str_pos += 1
                while str_pos < len(string) and (
                    string[str_pos].isdigit() or string[str_pos] in ".eE+-"
                ):
                    str_pos += 1
                if str_pos > start:
                    values.append(str(float(string[start:str_pos])))
            elif conv == "s":
                start = str_pos
                while str_pos < len(string) and not string[str_pos].isspace():
                    str_pos += 1
                values.append(string[start:str_pos])
            elif conv == "c":
                if str_pos < len(string):
                    values.append(str(ord(string[str_pos])))
                    str_pos += 1
            elif conv in "xXo":
                start = str_pos
                if conv in "xX":
                    if str_pos + 1 < len(string) and string[str_pos : str_pos + 2] in ("0x", "0X"):
                        str_pos += 2
                    while str_pos < len(string) and string[str_pos] in "0123456789abcdefABCDEF":
                        str_pos += 1
                    if str_pos > start:
                        values.append(str(int(string[start:str_pos], 16)))
                else:
                    while str_pos < len(string) and string[str_pos] in "01234567":
                        str_pos += 1
                    if str_pos > start:
                        values.append(str(int(string[start:str_pos], 8)))
            elif conv == "n":
                values.append(str(str_pos))
        else:
            # Literal character match
            if fmt[fmt_pos].isspace():
                while str_pos < len(string) and string[str_pos].isspace():
                    str_pos += 1
                while fmt_pos < len(fmt) and fmt[fmt_pos].isspace():
                    fmt_pos += 1
            else:
                if str_pos < len(string) and string[str_pos] == fmt[fmt_pos]:
                    str_pos += 1
                fmt_pos += 1

    if var_names:
        for i, vn in enumerate(var_names):
            if i < len(values):
                interp.current_frame.set_var(vn, values[i])
        return TclResult(value=str(len(values)))

    # No var names: return as list
    from ..machine import _list_escape

    return TclResult(value=" ".join(_list_escape(v) for v in values))


def register() -> None:
    """Register format commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("format", _cmd_format)
    REGISTRY.register_handler("scan", _cmd_scan)
