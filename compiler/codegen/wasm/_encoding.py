"""WASM binary encoding helpers + Tcl list-element quoting.

These utilities are pure functions with no dependence on the emitter
state. They're the input side of every byte the emitter writes:
LEB128 for integers, UTF-8 strings with length prefix, Tcl
list-element quoting for literal values the runtime will parse via
list semantics.
"""

from __future__ import annotations

from compiler.parsing.substitution import backslash_subst as _tcl_backslash_subst


def _leb128_unsigned(value: int) -> bytes:
    """Encode an unsigned integer as LEB128."""
    if value < 0:
        msg = f"unsigned LEB128 requires non-negative value, got {value}"
        raise ValueError(msg)
    result = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            byte |= 0x80
        result.append(byte)
        if not value:
            break
    return bytes(result)


def _leb128_signed(value: int) -> bytes:
    """Encode a signed integer as LEB128."""
    result = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if (value == 0 and not (byte & 0x40)) or (value == -1 and (byte & 0x40)):
            result.append(byte)
            break
        result.append(byte | 0x80)
    return bytes(result)


def _tcl_token_value(token: str) -> str:
    """Return the runtime VALUE of a Tcl source token (word).

    Mirrors the C Tcl parser's substitution rules:
    - Braced words ``{content}``: strip outer braces; content is literal
      (no substitution of any kind).
    - All other words: apply backslash substitution (``\\n`` → newline,
      ``\\\\`` → ``\\``, etc.).  Variable and command substitutions are
      left as-is because they must be resolved at runtime.

    Use this before :func:`~shared.tcl_quoting.tcl_list_quote` whenever *token* comes
    directly from IR source tokens rather than from a previously-
    evaluated value.

    Inverse: ``tcl_list_quote(_tcl_token_value(token))`` gives the
    canonical list-element representation of the word's value.
    """
    if token.startswith("{") and token.endswith("}") and len(token) >= 2:
        return token[1:-1]
    if "\\" in token:
        return _tcl_backslash_subst(token)
    return token


def _encode_string(s: str) -> bytes:
    """Encode a UTF-8 string with length prefix."""
    encoded = s.encode("utf-8")
    return _leb128_unsigned(len(encoded)) + encoded


def _encode_vector(items: list[bytes]) -> bytes:
    """Encode a WASM vector (count + concatenated items)."""
    return _leb128_unsigned(len(items)) + b"".join(items)
