"""Canonical Tcl list-element / word quoting.

A faithful, dialect-agnostic port of Tcl's own ``TclScanElement`` /
``TclConvertElement`` (``tclUtil.c``, Tcl 9.0, ``COMPAT=1``).  Given an
arbitrary string *value*, :func:`tcl_list_quote` returns the source text
of a single Tcl word that parses back to exactly that value — bare when
safe, brace-quoted when the content is brace-balanced, backslash-escaped
otherwise.  Because it mirrors the C algorithm 1:1, the result is the
authoritative answer to "how do I write this value as a Tcl word", not a
regex approximation.

This lives in ``shared/`` (a graph leaf) so every concern that needs to
render a runtime string value back into Tcl source — the WASM emitter and
the source optimiser both do — shares one implementation.
"""

from __future__ import annotations

_LIST_ELEM_WHITESPACE = frozenset(" \t\n\x0b\x0c\r")


# Flag bits for :func:`tcl_scan_element` / :func:`tcl_convert_element`.
# Names and semantics mirror ``enum ConvertFlags`` in ``tclUtil.c`` so the
# port is 1:1 with the Zig runtime (``runtime/zig/tcl_obj.zig``) and with
# reference Tcl 9.0's ``TclScanElement`` / ``TclConvertElement``.
FLAG_CONVERT_NONE = 0
FLAG_DONT_USE_BRACES = 1
FLAG_CONVERT_BRACE = 2
FLAG_CONVERT_ESCAPE = 4
FLAG_DONT_QUOTE_HASH = 8
FLAG_CONVERT_MASK = FLAG_CONVERT_BRACE | FLAG_CONVERT_ESCAPE


def tcl_scan_element(s: str, flag_in: int = 0) -> int:
    """Port of ``TclScanElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).

    Chooses the ``CONVERT_*`` mode appropriate for *s* so that
    :func:`tcl_convert_element` produces a valid list-element
    representation.  Pass ``FLAG_DONT_QUOTE_HASH`` to skip the
    leading-``#`` quoting rule — matches ``UpdateStringOfList``'s
    ``i ? TCL_DONT_QUOTE_HASH : 0`` convention for non-first elements.
    """
    if s == "":
        return (flag_in & FLAG_DONT_QUOTE_HASH) | FLAG_CONVERT_BRACE
    forbid_none = False
    require_escape = False
    prefer_escape = False
    prefer_brace = False
    nesting = 0
    if s[0] == "{" or s[0] == '"':
        forbid_none = True
        prefer_brace = True
    if s[0] == "#" and (flag_in & FLAG_DONT_QUOTE_HASH) == 0:
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
    out_hash = flag_in & FLAG_DONT_QUOTE_HASH
    if require_escape:
        return out_hash | FLAG_CONVERT_ESCAPE
    if forbid_none:
        if prefer_escape and not prefer_brace:
            return out_hash | FLAG_CONVERT_MASK
        return out_hash | FLAG_CONVERT_BRACE
    return out_hash | FLAG_CONVERT_NONE


_ESCAPE_ONE_CHAR = frozenset('][$; \\"')
_ESCAPE_CTL_MAP = {
    "\n": "\\n",
    "\t": "\\t",
    "\r": "\\r",
    "\x0b": "\\v",
    "\x0c": "\\f",
}


def tcl_convert_element(s: str, flags: int) -> str:
    """Port of ``TclConvertElement`` (tclUtil.c, Tcl 9.0, COMPAT=1)."""
    conversion = flags & FLAG_CONVERT_MASK
    if (flags & FLAG_DONT_USE_BRACES) and (conversion & FLAG_CONVERT_BRACE):
        conversion = FLAG_CONVERT_ESCAPE
    if s == "":
        return "{}"
    prefix = ""
    if s[0] == "#" and (flags & FLAG_DONT_QUOTE_HASH) == 0:
        if conversion == FLAG_CONVERT_ESCAPE:
            prefix = "\\#"
            s = s[1:]
        else:
            conversion = FLAG_CONVERT_BRACE
    if conversion == FLAG_CONVERT_NONE:
        return prefix + s
    if conversion == FLAG_CONVERT_BRACE:
        return prefix + "{" + s + "}"
    # CONVERT_ESCAPE or CONVERT_MASK.
    out: list[str] = [prefix] if prefix else []
    for ch in s:
        if ch in _ESCAPE_ONE_CHAR:
            out.append("\\")
            out.append(ch)
        elif ch == "{" or ch == "}":
            if conversion == FLAG_CONVERT_ESCAPE:
                out.append("\\")
            out.append(ch)
        elif ch in _ESCAPE_CTL_MAP:
            out.append(_ESCAPE_CTL_MAP[ch])
        else:
            out.append(ch)
    return "".join(out)


def tcl_list_quote(s: str, first: bool = True) -> str:
    """Return *s* encoded as a single Tcl list element.

    ``first=True`` — the element is position 0 of the output list, so a
    leading ``#`` is quoted (``{#}`` or ``\\#``) to prevent later
    ``eval`` re-parses from treating it as a comment.
    ``first=False`` — adds ``TCL_DONT_QUOTE_HASH``, matching the
    second-and-later-element rule in ``UpdateStringOfList``.
    """
    flag_in = 0 if first else FLAG_DONT_QUOTE_HASH
    flags = tcl_scan_element(s, flag_in)
    return tcl_convert_element(s, flags)
