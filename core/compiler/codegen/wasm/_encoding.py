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

    Use this before :func:`_tcl_list_quote` whenever *token* comes
    directly from IR source tokens rather than from a previously-
    evaluated value.

    Inverse: ``_tcl_list_quote(_tcl_token_value(token))`` gives the
    canonical list-element representation of the word's value.
    """
    if token.startswith("{") and token.endswith("}") and len(token) >= 2:
        return token[1:-1]
    if "\\" in token:
        return _tcl_backslash_subst(token)
    return token


_LIST_ELEM_WHITESPACE = frozenset(" \t\n\x0b\x0c\r")


# Flag bits for :func:`_tcl_scan_element` / :func:`_tcl_convert_element`.
# Names and semantics mirror ``enum ConvertFlags`` in ``tclUtil.c`` so the
# port is 1:1 with the Zig runtime (``runtime/zig/tcl_obj.zig``) and with
# reference Tcl 9.0's ``TclScanElement`` / ``TclConvertElement``.
_FLAG_CONVERT_NONE = 0
_FLAG_DONT_USE_BRACES = 1
_FLAG_CONVERT_BRACE = 2
_FLAG_CONVERT_ESCAPE = 4
_FLAG_DONT_QUOTE_HASH = 8
_FLAG_CONVERT_MASK = _FLAG_CONVERT_BRACE | _FLAG_CONVERT_ESCAPE


def _tcl_scan_element(s: str, flag_in: int = 0) -> int:
    """Port of ``TclScanElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).

    Chooses the ``CONVERT_*`` mode appropriate for *s* so that
    :func:`_tcl_convert_element` produces a valid list-element
    representation.  Pass ``_FLAG_DONT_QUOTE_HASH`` to skip the
    leading-``#`` quoting rule — matches ``UpdateStringOfList``'s
    ``i ? TCL_DONT_QUOTE_HASH : 0`` convention for non-first elements.
    """
    if s == "":
        return (flag_in & _FLAG_DONT_QUOTE_HASH) | _FLAG_CONVERT_BRACE
    forbid_none = False
    require_escape = False
    prefer_escape = False
    prefer_brace = False
    nesting = 0
    if s[0] == "{" or s[0] == '"':
        forbid_none = True
        prefer_brace = True
    if s[0] == "#" and (flag_in & _FLAG_DONT_QUOTE_HASH) == 0:
        prefer_brace = True
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch == "{":
            nesting += 1
        elif ch == "}":
            nesting -= 1
            if nesting < 0:
                require_escape = True
        elif ch == "]" or ch == '"':
            forbid_none = True
            prefer_escape = True
        elif ch == "[" or ch == "$" or ch == ";":
            forbid_none = True
            prefer_brace = True
        elif ch == "\\":
            if i + 1 >= n:
                require_escape = True
            elif s[i + 1] == "\n":
                require_escape = True
                i += 1
            elif s[i + 1] == "{" or s[i + 1] == "}" or s[i + 1] == "\\":
                i += 1
            forbid_none = True
            prefer_brace = True
        elif ch in _LIST_ELEM_WHITESPACE:
            forbid_none = True
            prefer_brace = True
        i += 1
    if nesting > 0:
        require_escape = True
    out_hash = flag_in & _FLAG_DONT_QUOTE_HASH
    if require_escape:
        return out_hash | _FLAG_CONVERT_ESCAPE
    if forbid_none:
        if prefer_escape and not prefer_brace:
            return out_hash | _FLAG_CONVERT_MASK
        return out_hash | _FLAG_CONVERT_BRACE
    return out_hash | _FLAG_CONVERT_NONE


_ESCAPE_ONE_CHAR = frozenset('][$; \\"')
_ESCAPE_CTL_MAP = {
    "\n": "\\n",
    "\t": "\\t",
    "\r": "\\r",
    "\x0b": "\\v",
    "\x0c": "\\f",
}


def _tcl_convert_element(s: str, flags: int) -> str:
    """Port of ``TclConvertElement`` (tclUtil.c, Tcl 9.0, COMPAT=1)."""
    conversion = flags & _FLAG_CONVERT_MASK
    if (flags & _FLAG_DONT_USE_BRACES) and (conversion & _FLAG_CONVERT_BRACE):
        conversion = _FLAG_CONVERT_ESCAPE
    if s == "":
        return "{}"
    prefix = ""
    if s[0] == "#" and (flags & _FLAG_DONT_QUOTE_HASH) == 0:
        if conversion == _FLAG_CONVERT_ESCAPE:
            prefix = "\\#"
            s = s[1:]
        else:
            conversion = _FLAG_CONVERT_BRACE
    if conversion == _FLAG_CONVERT_NONE:
        return prefix + s
    if conversion == _FLAG_CONVERT_BRACE:
        return prefix + "{" + s + "}"
    # CONVERT_ESCAPE or CONVERT_MASK.
    out: list[str] = [prefix] if prefix else []
    for ch in s:
        if ch in _ESCAPE_ONE_CHAR:
            out.append("\\")
            out.append(ch)
        elif ch == "{" or ch == "}":
            if conversion == _FLAG_CONVERT_ESCAPE:
                out.append("\\")
            out.append(ch)
        elif ch in _ESCAPE_CTL_MAP:
            out.append(_ESCAPE_CTL_MAP[ch])
        else:
            out.append(ch)
    return "".join(out)


def _tcl_list_quote(s: str, first: bool = True) -> str:
    """Return *s* encoded as a single Tcl list element.

    ``first=True`` — the element is position 0 of the output list, so a
    leading ``#`` is quoted (``{#}`` or ``\\#``) to prevent later
    ``eval`` re-parses from treating it as a comment.
    ``first=False`` — adds ``TCL_DONT_QUOTE_HASH``, matching the
    second-and-later-element rule in ``UpdateStringOfList``.
    """
    flag_in = 0 if first else _FLAG_DONT_QUOTE_HASH
    flags = _tcl_scan_element(s, flag_in)
    return _tcl_convert_element(s, flags)


def _encode_string(s: str) -> bytes:
    """Encode a UTF-8 string with length prefix."""
    encoded = s.encode("utf-8")
    return _leb128_unsigned(len(encoded)) + encoded


def _encode_vector(items: list[bytes]) -> bytes:
    """Encode a WASM vector (count + concatenated items)."""
    return _leb128_unsigned(len(items)) + b"".join(items)
