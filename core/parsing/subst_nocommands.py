"""Compile-time evaluator for ``[subst -nocommands {template}]``.

Used by P7.3's lowering hook when we see a ``proc \$var [subst
-nocommands {…}]`` shape and want to materialise the body string
at compile time instead of deferring to the runtime interpreter.

Semantics (matching ``tclsh 9.0``'s ``subst -nocommands``):

* ``\$var`` / ``\${var}`` — variable substitution.  The name is
  looked up in the supplied const-map; a miss refuses the whole
  evaluation by returning ``None`` (the caller keeps the dynamic
  dispatch path in that case).
* ``\\…`` — standard backslash processing via
  :func:`core.parsing.substitution.backslash_subst`.  Handles
  ``\\n \\t \\xNN \\uNNNN`` and octal / continuation-line forms.
* ``[…]`` — left as a literal ``[…]`` string (the ``-nocommands``
  flag is exactly this: skip command substitution).  Unbalanced
  ``[`` / ``]`` pairs are a user error in real Tcl; we reject them
  by returning ``None`` rather than emitting silently-wrong output.
* ``\$a(b)`` — array references are refused.  The tcltest
  ``Option`` factory template never uses them; adding full array-
  ref support would widen the trust surface without payoff today.
* ``\$::ns::var`` — namespace-qualified var refs are refused for
  the same reason.

This is a *compile-time* string→string transform — no side
effects, no IR interaction.  Pure function.
"""

from __future__ import annotations

from typing import Mapping

from .substitution import backslash_subst

# Character classes used for bare ``$name`` scanning.  Tcl's
# variable-name parser accepts ASCII alphanumerics plus underscore;
# anything else terminates the name.  We keep to the ASCII subset
# here to match the tokeniser in ``core/parsing/lexer.py``.
_NAME_CHARS = frozenset("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")


def subst_nocommands(template: str, const_map: Mapping[str, str]) -> str | None:
    """Evaluate a ``subst -nocommands`` template at compile time.

    Returns the substituted string on success, or ``None`` if any
    condition above refuses the evaluation.  Refusal is always safe
    — the caller falls back to runtime dispatch, preserving the
    original semantics.

    *const_map* maps variable names (without the leading ``$``) to
    their literal string values.  Names are looked up with their
    ``{…}``-stripped form, so ``\${foo}`` and ``\$foo`` resolve the
    same entry.
    """
    out: list[str] = []
    i = 0
    n = len(template)
    while i < n:
        c = template[i]
        if c == "\\":
            # Defer to the shared backslash processor for the
            # single following escape (plus continuation-line and
            # octal / hex / unicode forms).  We feed it just the
            # ``\\X`` slice it needs, letting it compute how many
            # source characters the escape consumed.
            j = _backslash_end(template, i)
            decoded = backslash_subst(template[i:j])
            out.append(decoded)
            i = j
            continue
        if c == "$":
            # Variable substitution.  ``$$`` is not a Tcl escape
            # — Tcl treats the first ``$`` as starting a variable
            # name and if no name follows (e.g. ``$$``), leaves the
            # ``$`` literal and continues from the next character.
            # Matches that behaviour: look at the next character to
            # decide.
            if i + 1 >= n:
                out.append("$")
                i += 1
                continue
            nxt = template[i + 1]
            if nxt == "{":
                close = template.find("}", i + 2)
                if close < 0:
                    return None
                name = template[i + 2 : close]
                if _is_complex_var_name(name):
                    return None
                if name not in const_map:
                    return None
                out.append(const_map[name])
                i = close + 1
                continue
            if nxt in _NAME_CHARS:
                j = i + 1
                while j < n and template[j] in _NAME_CHARS:
                    j += 1
                name = template[i + 1 : j]
                # Array reference ``$name(index)`` — refuse.
                if j < n and template[j] == "(":
                    return None
                # Namespace qualifier ``$a::b`` — refuse (parser
                # treats ``::`` inside a bare ``$name`` as part of
                # the name, but we don't carry qualified entries in
                # the const-map).
                if j + 1 < n and template[j] == ":" and template[j + 1] == ":":
                    return None
                if name not in const_map:
                    return None
                out.append(const_map[name])
                i = j
                continue
            # ``$`` followed by a non-name character — emit literal
            # ``$`` and continue from the next character.
            out.append("$")
            i += 1
            continue
        if c == "[":
            # ``-nocommands``: ``[`` introduces a bracket-quoted run
            # that's kept literal (including the brackets).  We need
            # the matching ``]`` to locate the end; unbalanced
            # brackets are a user error.
            close = _match_bracket(template, i)
            if close < 0:
                return None
            out.append(template[i : close + 1])
            i = close + 1
            continue
        if c == "]":
            # A stray ``]`` without a matching ``[`` — just output
            # it.  Tcl's ``subst -nocommands`` accepts this shape.
            out.append(c)
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def _backslash_end(template: str, start: int) -> int:
    """Return the index one past the last character consumed by the
    backslash escape starting at *start*.  Matches the scan
    ``backslash_subst`` would do internally, returning enough range
    for a single-escape decode.
    """
    n = len(template)
    if start + 1 >= n:
        return n
    c = template[start + 1]
    # Single-char escapes the shared helper knows about all consume
    # two characters.  The multi-char variants (``\\x``, ``\\u``,
    # ``\\U``, octal, continuation line) need explicit scanning.
    if c == "x":
        j = start + 2
        while j < n and j < start + 4 and template[j] in "0123456789abcdefABCDEF":
            j += 1
        return j if j > start + 2 else start + 2
    if c == "u":
        j = start + 2
        while j < n and j < start + 6 and template[j] in "0123456789abcdefABCDEF":
            j += 1
        return j if j > start + 2 else start + 2
    if c == "U":
        j = start + 2
        while j < n and j < start + 10 and template[j] in "0123456789abcdefABCDEF":
            j += 1
        return j if j > start + 2 else start + 2
    if c in "01234567":
        j = start + 1
        while j < n and j < start + 4 and template[j] in "01234567":
            j += 1
        return j
    if c == "\n" or c == "\r":
        j = start + 2
        if c == "\r" and j < n and template[j] == "\n":
            j += 1
        while j < n and template[j] in " \t":
            j += 1
        return j
    return start + 2


def _match_bracket(template: str, start: int) -> int:
    """Return the index of the matching ``]`` for the ``[`` at
    *start*, or ``-1`` if unbalanced.  Skips nested ``[…]`` pairs
    and honours backslash escapes."""
    depth = 1
    i = start + 1
    n = len(template)
    while i < n:
        c = template[i]
        if c == "\\" and i + 1 < n:
            i += 2
            continue
        if c == "[":
            depth += 1
        elif c == "]":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def _is_complex_var_name(name: str) -> bool:
    """True if ``name`` has a form we refuse — namespace
    qualifiers, array indices, or embedded substitutions.  We only
    accept plain identifiers so the const-map key matches what
    :meth:`core.compiler.lowering.Lowering._set_literal_body` tracks.
    """
    if not name:
        return True
    if "::" in name or "(" in name or ")" in name:
        return True
    if "$" in name or "[" in name:
        return True
    if not all(ch in _NAME_CHARS for ch in name):
        return True
    return False
