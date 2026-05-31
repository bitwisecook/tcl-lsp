"""Tcl ``scan`` format-string parser and static no-match analysis.

Tcl's ``scan STRING FORMAT ?varName...?`` is the analogue of C's
``sscanf``: it walks *FORMAT* and consumes characters from *STRING*,
binding each successful conversion to the next varName.  When the very
first conversion can't be satisfied, no varName is written -- the
caller observes ``v`` as unset.

This module's :func:`scan_provably_no_match` returns ``True`` only when
the first conversion of *format* is *statically* known to fail on
*input* (no characters could be consumed); callers can then treat the
output var as ``provably_unset`` and fire W210 on subsequent reads.

The parser is structurally faithful to the Tcl 9.0.3 ``scan`` man
page (``scan.n``): literal characters in the format must match the
input verbatim; whitespace in the format matches any run of input
whitespace; ``%`` introduces a conversion with optional ``*``
(suppress assignment), width, and size modifier, followed by the
conversion character (``d``, ``o``, ``x``, ``c``, ``s``, ``e``,
``f``, ``g``, ``[set]``).  This module models that walk for the
*first* conversion only, which is sufficient for the W210 use case.
"""

from __future__ import annotations

# Characters that are valid conversion specifiers in Tcl scan.
# Each maps to the input-prefix predicate that decides whether the
# conversion can consume at least one character.
_CONV_LEADERS: dict[str, str] = {
    # Integers (any base if no width): leading sign + digit(s).
    # Decimal-only here; ``%i`` is more permissive but treated
    # conservatively as a superset of %d so that no-match proofs
    # under %d also hold under %i.
    "d": "decimal-int",
    "i": "decimal-int",  # 0x.../ 0b.../ 0o.../ decimal
    "o": "octal-int",
    "x": "hex-int",
    "X": "hex-int",
    "b": "binary-int",
    "u": "decimal-int",
    "c": "any",  # single character -- always consumes one
    "s": "non-space",  # run of non-whitespace
    "e": "float",
    "E": "float",
    "f": "float",
    "g": "float",
    "G": "float",
    # ``%n`` writes the count of characters consumed so far -- it does
    # NOT consume input and ALWAYS succeeds (tclsh: ``scan "" %n n``
    # sets ``n`` to 0).  Distinct from "any" (which requires input)
    # because empty input is fine for %n but not for, say, %c.
    "n": "always",
}


def _skip_leading_whitespace(s: str, i: int) -> int:
    """Advance over input whitespace -- mirrors scanf's ws handling."""
    while i < len(s) and s[i] in " \t\n\r\f\v":
        i += 1
    return i


def _can_consume_first_conversion(format_str: str, input_str: str) -> bool | None:
    """Return True/False if we can prove the first conversion will / will
    not consume any input.  Return None when the format / input shape
    is outside our modelled subset (caller should NOT treat as no-match).
    """
    # Walk the format until the first conversion specifier.
    fi = 0
    si = 0
    n_fmt = len(format_str)
    n_inp = len(input_str)
    while fi < n_fmt:
        fc = format_str[fi]
        if fc == "%":
            # Parse the conversion: ``%[*][width][modifier]CONV``.
            fi += 1
            if fi >= n_fmt:
                return None  # malformed -- be conservative
            if format_str[fi] == "%":
                # ``%%`` matches a literal ``%`` in the input.
                if si >= n_inp or input_str[si] != "%":
                    return False
                si += 1
                fi += 1
                continue
            # Skip assignment-suppression ``*``.
            if format_str[fi] == "*":
                fi += 1
                if fi >= n_fmt:
                    return None
            # Width digits.
            while fi < n_fmt and format_str[fi].isdigit():
                fi += 1
            if fi >= n_fmt:
                return None
            # Size modifier (``h``, ``l``, ``ll``).  We just skip it.
            while fi < n_fmt and format_str[fi] in "hl":
                fi += 1
            if fi >= n_fmt:
                return None
            conv = format_str[fi]
            # Character set ``%[abc]`` / ``%[^abc]`` -- read until ``]``.
            if conv == "[":
                # Parse the set spec but don't need it for the
                # consume/no-consume decision: a ``[set]`` conversion
                # consumes if the next input char is in the (possibly
                # negated) set.  Without parsing the set we can't be
                # sure, so return None to stay conservative.
                return None
            # Non-``[`` conversions skip leading whitespace EXCEPT
            # for ``%c``, ``%[``, and ``%n``.
            if conv not in ("c", "n"):
                si = _skip_leading_whitespace(input_str, si)
            return _input_satisfies_conv(conv, input_str, si)
        # Literal in format: any whitespace run matches input ws run.
        # Tcl scan treats every C-ws character in the FORMAT as a
        # whitespace directive (matches the input's ws run), not only
        # space/tab/newline.  The previous version missed ``\r\f\v``
        # and would falsely classify ``scan " 123" "\r%d" n`` as no-match.
        if fc in " \t\n\r\f\v":
            fi += 1
            si = _skip_leading_whitespace(input_str, si)
            continue
        # Plain literal character: must match input verbatim.
        if si >= n_inp or input_str[si] != fc:
            return False
        si += 1
        fi += 1
    # Walked the whole format without seeing a conversion -- there's
    # no output var to write, so no-match is meaningless.  Return None.
    return None


def _input_satisfies_conv(conv: str, s: str, i: int) -> bool | None:
    """Can the input at offset *i* satisfy a single conversion *conv*?"""
    kind = _CONV_LEADERS.get(conv)
    if kind is None:
        return None  # unknown conversion -- conservative
    if kind == "always":
        # ``%n`` writes the consumed count and never consumes input;
        # the conversion always succeeds, even on empty input.
        return True
    if kind == "any":
        return i < len(s)
    if kind == "non-space":
        return i < len(s) and s[i] not in " \t\n\r\f\v"
    if kind == "decimal-int":
        return _starts_with_int(s, i, "0123456789")
    if kind == "octal-int":
        return _starts_with_int(s, i, "01234567")
    if kind == "binary-int":
        return _starts_with_int(s, i, "01")
    if kind == "hex-int":
        return _starts_with_int(s, i, "0123456789abcdefABCDEF")
    if kind == "float":
        # Float accepts sign + (digits | leading '.' digits | Inf | NaN).
        # Tcl ``scan %f`` matches the same surface as ``Tcl_GetDouble``,
        # which includes the IEEE special-value spellings ``Inf``,
        # ``Infinity``, ``NaN`` (case-insensitive).  Missing these
        # (the original implementation) caused ``scan Inf %f f`` to be
        # falsely classified as no-match.
        if i >= len(s):
            return False
        c = s[i]
        if c in "+-":
            i += 1
            if i >= len(s):
                return False
            c = s[i]
        if c.isdigit():
            return True
        if c == "." and i + 1 < len(s) and s[i + 1].isdigit():
            return True
        # ``Inf``/``Infinity``/``NaN`` (case-insensitive prefix match).
        tail = s[i:].lower()
        if tail.startswith("inf") or tail.startswith("nan"):
            return True
        return False
    return None


def _starts_with_int(s: str, i: int, digit_set: str) -> bool:
    """Return True iff *s[i:]* starts with an optional sign followed by
    at least one digit from *digit_set*."""
    if i >= len(s):
        return False
    if s[i] in "+-":
        i += 1
    return i < len(s) and s[i] in digit_set


def scan_provably_no_match(format_str: str, input_str: str) -> bool:
    """Return True iff Tcl's ``scan INPUT_STR FORMAT_STR ?vars...?`` is
    statically guaranteed to leave at least the FIRST output variable
    unset (the first conversion can't consume any input).

    Only returns True when the analysis is fully confident.  Returns
    False on any uncertain case (unknown conversion, character set,
    malformed format) so callers don't fire spurious W210.
    """
    # Conservative bail-out: the analyser passes the RAW source text
    # (``argv_texts``) which has NOT had Tcl's backslash escapes
    # interpreted.  A backslash in the format / input means we'd be
    # walking the literal ``\r`` instead of the runtime CR character,
    # producing a wrong no-match verdict (D4-F1 closure).  Similarly
    # bail when the format or input contains a ``$`` / ``[`` -- those
    # are substitution markers whose runtime value we can't see here.
    if any(c in format_str for c in "\\$[") or any(c in input_str for c in "\\$["):
        return False
    verdict = _can_consume_first_conversion(format_str, input_str)
    return verdict is False
