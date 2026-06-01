"""Canonical Tcl list manipulation — quoting, splitting, joining.

The single shared home for Tcl list semantics, so every concern (compiler
pipeline, codegen, dialect const-folding, the bytecode VM, tooling) shares one
implementation instead of re-deriving brace/quote/backslash rules.

* :func:`tcl_list_quote` renders a string value as one Tcl list element — a
  1:1 port of Tcl's own ``TclScanElement`` / ``TclConvertElement``
  (``tclUtil.c``, Tcl 9.0, ``COMPAT=1``): bare when safe, brace-quoted when
  brace-balanced, backslash-escaped otherwise.
* :func:`tcl_list_split` parses a list string into its element strings,
  mirroring ``Tcl_SplitList`` (braces stripped, backslash-aware).
* :func:`tcl_list_join` is the inverse of split — quote each element and join
  with single spaces (what Tcl's ``list`` builtin produces).

This lives in ``shared/`` (a graph leaf): the algorithms are pure and
dialect-agnostic, depending on nothing outside the standard library.
"""

from __future__ import annotations

from collections.abc import Iterable

from shared.tcl_subst import backslash_subst

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


class TclListError(ValueError):
    """Raised by :func:`tcl_list_split` on malformed list syntax.

    A :class:`ValueError` subclass so ``shared`` carries no dependency on any
    runtime error type; callers that need a dialect/VM-specific error (e.g. the
    bytecode VM's ``TclError``) catch this and re-raise.
    """


def tcl_list_split(s: str, *, strict: bool = False, unescape: bool = False) -> list[str]:
    """Split a Tcl list string into its element strings (``Tcl_SplitList``).

    Elements are separated by whitespace; brace- and quote-grouped elements
    are returned with their outer delimiters removed.  Backslashes are honoured
    for grouping (``\\{`` / ``\\"`` / ``\\ `` don't open/close/separate).

    By default each element is returned in *source* form minus delimiters
    (backslashes preserved) — enough for compile-time folders that compare
    against known-good literals.  Pass ``unescape=True`` to recover element
    *values* (what ``lindex`` yields at runtime): :func:`backslash_subst` is
    applied to bare and quoted elements (never braced ones, whose content is
    literal).

    With ``strict=True`` malformed input (unmatched brace/quote, or trailing
    junk glued to a close delimiter) raises :class:`TclListError`, matching
    ``Tcl_SplitList``.  With ``strict=False`` (the default) parsing is lenient:
    an unterminated group simply runs to end-of-string and trailing junk is
    absorbed into the element — the historic behaviour relied on by const
    folding over known-good literals.
    """

    def _subst(text: str) -> str:
        return backslash_subst(text) if unescape else text

    result: list[str] = []
    i = 0
    n = len(s)
    while i < n:
        while i < n and s[i] in _LIST_ELEM_WHITESPACE:
            i += 1
        if i >= n:
            break
        ch = s[i]
        if ch == "{":
            level = 1
            i += 1
            start = i
            while i < n and level > 0:
                if s[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                if s[i] == "{":
                    level += 1
                elif s[i] == "}":
                    level -= 1
                i += 1
            if level > 0:
                if strict:
                    raise TclListError("unmatched open brace in list")
                # Braced content is literal — never backslash-substituted.
                result.append(s[start:i])
                continue
            result.append(s[start : i - 1])
            if strict and i < n and s[i] not in _LIST_ELEM_WHITESPACE:
                raise TclListError(
                    f'list element in braces followed by "{s[i]}" instead of space'
                )
        elif ch == '"':
            i += 1
            start = i
            while i < n and s[i] != '"':
                if s[i] == "\\":
                    i += 1
                i += 1
            if i >= n:
                if strict:
                    raise TclListError("unmatched open quote in list")
                result.append(_subst(s[start:i]))
                continue
            result.append(_subst(s[start:i]))
            i += 1
            if strict and i < n and s[i] not in _LIST_ELEM_WHITESPACE:
                raise TclListError(
                    f'list element in quotes followed by "{s[i]}" instead of space'
                )
        else:
            start = i
            while i < n and s[i] not in _LIST_ELEM_WHITESPACE:
                if s[i] == "\\":
                    i += 1
                i += 1
            result.append(_subst(s[start:i]))
    return result


def tcl_list_join(elements: Iterable[str]) -> str:
    """Join *elements* into a canonical Tcl list string.

    Each element is rendered with :func:`tcl_list_quote` and joined with single
    spaces — exactly what Tcl's ``list`` builtin produces.  The first element
    quotes a leading ``#``; later ones don't (``UpdateStringOfList`` rule).
    """
    return " ".join(tcl_list_quote(e, first=(i == 0)) for i, e in enumerate(elements))
