"""The ``binary`` command — format and scan binary data."""

from __future__ import annotations

import struct
from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_binary(interp: TclInterp, args: list[str]) -> TclResult:
    """binary subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "binary subcommand ?arg ...?"')

    match args[0]:
        case "format":
            return _binary_format(interp, args[1:])
        case "scan":
            return _binary_scan(interp, args[1:])
        case "encode":
            return _binary_encode(interp, args[1:])
        case "decode":
            return _binary_decode(interp, args[1:])
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be '
                "decode, encode, format, or scan"
            )


def _parse_count(fmt: str, pos: int) -> tuple[int | None, bool, int]:
    """Parse the count field after a format character.

    Returns (count, is_star, new_pos).
    *count* is ``None`` when ``*`` or no count is given (meaning "all
    remaining" or "default for the specifier").  An explicit ``0`` is
    preserved so callers can distinguish "no items" from "default".
    """
    if pos >= len(fmt):
        return None, False, pos
    if fmt[pos] == "*":
        return None, True, pos + 1
    count = 0
    has_digits = False
    while pos < len(fmt) and fmt[pos].isdigit():
        count = count * 10 + int(fmt[pos])
        has_digits = True
        pos += 1
    if has_digits:
        return count, False, pos
    return None, False, pos


def _binary_format(interp: TclInterp, args: list[str]) -> TclResult:
    """binary format formatString ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "binary format formatString ?arg ...?"')

    fmt = args[0]
    values = args[1:]
    result = bytearray()
    val_idx = 0
    pos = 0

    while pos < len(fmt):
        ch = fmt[pos]
        pos += 1

        if ch == " " or ch == "\t":
            continue

        count, is_star, pos = _parse_count(fmt, pos)

        match ch:
            case "a" | "A":
                if val_idx >= len(values):
                    raise TclError("not enough arguments for all format specifiers")
                data = values[val_idx].encode("latin-1", errors="replace")
                val_idx += 1
                if is_star:
                    result.extend(data)
                else:
                    n = 0 if count is None else count
                    pad = b"\x00" if ch == "a" else b" "
                    if len(data) >= n:
                        result.extend(data[:n])
                    else:
                        result.extend(data)
                        result.extend(pad * (n - len(data)))

            case "b" | "B":
                if val_idx >= len(values):
                    raise TclError("not enough arguments for all format specifiers")
                bits = values[val_idx]
                val_idx += 1
                n = len(bits) if is_star else (0 if count is None else count)
                byte_count = (n + 7) // 8
                for i in range(byte_count):
                    byte_val = 0
                    for bit_j in range(8):
                        bit_idx = i * 8 + bit_j
                        if bit_idx < n and bit_idx < len(bits) and bits[bit_idx] == "1":
                            if ch == "b":
                                byte_val |= 1 << bit_j
                            else:
                                byte_val |= 1 << (7 - bit_j)
                    result.append(byte_val)

            case "h" | "H":
                if val_idx >= len(values):
                    raise TclError("not enough arguments for all format specifiers")
                hex_str = values[val_idx]
                val_idx += 1
                n = len(hex_str) if is_star else (0 if count is None else count)
                byte_count = (n + 1) // 2
                for i in range(byte_count):
                    hi_idx = i * 2
                    lo_idx = i * 2 + 1
                    if ch == "H":
                        hi = int(hex_str[hi_idx], 16) if hi_idx < n and hi_idx < len(hex_str) else 0
                        lo = int(hex_str[lo_idx], 16) if lo_idx < n and lo_idx < len(hex_str) else 0
                        result.append((hi << 4) | lo)
                    else:
                        lo = int(hex_str[hi_idx], 16) if hi_idx < n and hi_idx < len(hex_str) else 0
                        hi = int(hex_str[lo_idx], 16) if lo_idx < n and lo_idx < len(hex_str) else 0
                        result.append((hi << 4) | lo)

            case "c":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.append(v & 0xFF)

            case "s":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack("<H", v & 0xFFFF))

            case "S":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">H", v & 0xFFFF))

            case "t":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack("<H", v & 0xFFFF))

            case "i":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack("<I", v & 0xFFFFFFFF))

            case "I":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">I", v & 0xFFFFFFFF))

            case "n":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">I", v & 0xFFFFFFFF))

            case "w":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack("<q", v))

            case "W":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">q", v))

            case "m":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = _to_int(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">Q", v))

            case "r" | "R":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = float(values[val_idx])
                    val_idx += 1
                    endian = "<" if ch == "r" else ">"
                    result.extend(struct.pack(f"{endian}f", v))

            case "d" | "q":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = float(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack("<d", v))

            case "Q":
                n = 1 if is_star else (1 if count is None else count)
                for _ in range(n):
                    if val_idx >= len(values):
                        raise TclError("not enough arguments for all format specifiers")
                    v = float(values[val_idx])
                    val_idx += 1
                    result.extend(struct.pack(">d", v))

            case "x":
                n = 1 if is_star else (1 if count is None else count)
                result.extend(b"\x00" * n)

            case "X":
                n = 1 if is_star else (1 if count is None else count)
                if n > len(result):
                    n = len(result)
                del result[-n:]

            case "@":
                n = 0 if count is None else count
                if n > len(result):
                    result.extend(b"\x00" * (n - len(result)))
                else:
                    del result[n:]

            case _:
                raise TclError(f'bad field specifier "{ch}"')

    # Return as a binary string (latin-1 round-trips bytes)
    return TclResult(value=bytes(result).decode("latin-1"))


def _binary_scan(interp: TclInterp, args: list[str]) -> TclResult:
    """binary scan string formatString ?varName ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "binary scan value formatString ?varName ...?"')

    data = args[0].encode("latin-1", errors="replace")
    fmt = args[1]
    var_names = args[2:]
    var_idx = 0
    data_pos = 0
    conversions = 0

    pos = 0
    while pos < len(fmt):
        ch = fmt[pos]
        pos += 1

        if ch == " " or ch == "\t":
            continue

        count, is_star, pos = _parse_count(fmt, pos)

        match ch:
            case "a" | "A":
                if var_idx >= len(var_names):
                    break
                n = len(data) - data_pos if is_star else (0 if count is None else count)
                chunk = data[data_pos : data_pos + n]
                data_pos += n
                val = chunk.decode("latin-1")
                if ch == "A":
                    val = val.rstrip(" \x00")
                interp.current_frame.set_var(var_names[var_idx], val)
                var_idx += 1
                conversions += 1

            case "b" | "B":
                if var_idx >= len(var_names):
                    break
                n = (len(data) - data_pos) * 8 if is_star else (0 if count is None else count)
                byte_count = (n + 7) // 8
                bits = []
                for i in range(byte_count):
                    if data_pos + i < len(data):
                        byte_val = data[data_pos + i]
                    else:
                        byte_val = 0
                    for bit_j in range(8):
                        if len(bits) >= n:
                            break
                        if ch == "b":
                            bits.append("1" if byte_val & (1 << bit_j) else "0")
                        else:
                            bits.append("1" if byte_val & (1 << (7 - bit_j)) else "0")
                data_pos += byte_count
                interp.current_frame.set_var(var_names[var_idx], "".join(bits))
                var_idx += 1
                conversions += 1

            case "h" | "H":
                if var_idx >= len(var_names):
                    break
                n = (len(data) - data_pos) * 2 if is_star else (0 if count is None else count)
                byte_count = (n + 1) // 2
                hex_chars = []
                for i in range(byte_count):
                    if data_pos + i < len(data):
                        byte_val = data[data_pos + i]
                    else:
                        byte_val = 0
                    if ch == "H":
                        hex_chars.append(f"{(byte_val >> 4) & 0xF:x}")
                        if len(hex_chars) < n:
                            hex_chars.append(f"{byte_val & 0xF:x}")
                    else:
                        hex_chars.append(f"{byte_val & 0xF:x}")
                        if len(hex_chars) < n:
                            hex_chars.append(f"{(byte_val >> 4) & 0xF:x}")
                data_pos += byte_count
                interp.current_frame.set_var(var_names[var_idx], "".join(hex_chars[:n]))
                var_idx += 1
                conversions += 1

            case "c":
                n = len(data) - data_pos if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos >= len(data):
                        break
                    v = struct.unpack_from("b", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 1
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "s":
                n = (len(data) - data_pos) // 2 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 2 > len(data):
                        break
                    v = struct.unpack_from("<h", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 2
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "S":
                n = (len(data) - data_pos) // 2 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 2 > len(data):
                        break
                    v = struct.unpack_from(">h", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 2
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "t":
                n = (len(data) - data_pos) // 2 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 2 > len(data):
                        break
                    v = struct.unpack_from("<H", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 2
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "i":
                n = (len(data) - data_pos) // 4 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 4 > len(data):
                        break
                    v = struct.unpack_from("<i", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 4
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "I":
                n = (len(data) - data_pos) // 4 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 4 > len(data):
                        break
                    v = struct.unpack_from(">i", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 4
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "n":
                n = (len(data) - data_pos) // 4 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 4 > len(data):
                        break
                    v = struct.unpack_from(">i", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 4
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "w":
                n = (len(data) - data_pos) // 8 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 8 > len(data):
                        break
                    v = struct.unpack_from("<q", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 8
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "W":
                n = (len(data) - data_pos) // 8 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 8 > len(data):
                        break
                    v = struct.unpack_from(">q", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 8
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "m":
                n = (len(data) - data_pos) // 8 if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + 8 > len(data):
                        break
                    v = struct.unpack_from(">Q", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += 8
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx],
                        vals[0] if len(vals) == 1 else " ".join(vals),
                    )
                    var_idx += 1
                    conversions += 1

            case "r" | "R":
                sz = 4
                n = (len(data) - data_pos) // sz if is_star else (1 if count is None else count)
                vals = []
                endian = "<" if ch == "r" else ">"
                for _ in range(n):
                    if data_pos + sz > len(data):
                        break
                    v = struct.unpack_from(f"{endian}f", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += sz
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "d" | "q":
                sz = 8
                n = (len(data) - data_pos) // sz if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + sz > len(data):
                        break
                    v = struct.unpack_from("<d", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += sz
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "Q":
                sz = 8
                n = (len(data) - data_pos) // sz if is_star else (1 if count is None else count)
                vals = []
                for _ in range(n):
                    if data_pos + sz > len(data):
                        break
                    v = struct.unpack_from(">d", data, data_pos)[0]
                    vals.append(str(v))
                    data_pos += sz
                if var_idx < len(var_names):
                    interp.current_frame.set_var(
                        var_names[var_idx], vals[0] if len(vals) == 1 else " ".join(vals)
                    )
                    var_idx += 1
                    conversions += 1

            case "x":
                n = 1 if is_star else (1 if count is None else count)
                data_pos += n

            case "X":
                n = 1 if is_star else (1 if count is None else count)
                data_pos = max(0, data_pos - n)

            case "@":
                n = 0 if count is None else count
                data_pos = n

            case _:
                raise TclError(f'bad field specifier "{ch}"')

    return TclResult(value=str(conversions))


def _binary_encode(interp: TclInterp, args: list[str]) -> TclResult:
    """binary encode format ?options? data"""
    if len(args) < 2:
        raise TclError(
            'wrong # args: should be "binary encode format ?-maxlen len? ?-wrapchar char? data"'
        )
    encoding = args[0]
    # Parse options between format and data
    opts = args[1:-1]
    data = args[-1].encode("latin-1", errors="replace")
    maxlen = 76
    wrapchar = "\n"
    i = 0
    while i < len(opts):
        if opts[i] == "-maxlen" and i + 1 < len(opts):
            try:
                maxlen = int(opts[i + 1])
            except ValueError:
                raise TclError(f'expected integer but got "{opts[i + 1]}"') from None
            i += 2
        elif opts[i] == "-wrapchar" and i + 1 < len(opts):
            wrapchar = opts[i + 1]
            i += 2
        else:
            raise TclError(f'bad option "{opts[i]}": must be -maxlen or -wrapchar')

    match encoding:
        case "hex":
            return TclResult(value=data.hex())
        case "base64":
            import base64

            encoded = base64.b64encode(data).decode("ascii")
            if maxlen > 0 and wrapchar:
                lines = [encoded[j : j + maxlen] for j in range(0, len(encoded), maxlen)]
                encoded = wrapchar.join(lines)
            return TclResult(value=encoded)
        case _:
            raise TclError(f'unknown or ambiguous encoding "{encoding}": must be base64 or hex')


def _binary_decode(interp: TclInterp, args: list[str]) -> TclResult:
    """binary decode format ?-strict? data"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "binary decode format ?-strict? data"')
    encoding = args[0]
    strict = False
    rest = args[1:]
    if rest and rest[0] == "-strict":
        strict = True
        rest = rest[1:]
    if not rest:
        raise TclError('wrong # args: should be "binary decode format ?-strict? data"')
    data_str = rest[0]

    match encoding:
        case "hex":
            try:
                decoded = bytes.fromhex(data_str)
            except ValueError as e:
                raise TclError(f"invalid hexadecimal data: {e}") from e
            return TclResult(value=decoded.decode("latin-1"))
        case "base64":
            import base64

            try:
                decoded = base64.b64decode(data_str, validate=strict)
            except Exception as e:
                raise TclError(f"invalid base64 data: {e}") from e
            return TclResult(value=decoded.decode("latin-1"))
        case _:
            raise TclError(f'unknown or ambiguous encoding "{encoding}": must be base64 or hex')


def _to_int(s: str) -> int:
    """Convert a string to an integer, handling Tcl number formats."""
    s = s.strip()
    if not s:
        raise TclError('expected integer but got ""')
    try:
        return int(s, 0)
    except ValueError:
        try:
            return int(float(s))
        except (ValueError, OverflowError):
            raise TclError(f'expected integer but got "{s}"') from None


def register() -> None:
    """Register binary command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("binary", _cmd_binary)
