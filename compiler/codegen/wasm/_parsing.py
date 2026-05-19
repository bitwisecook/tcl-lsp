"""Small parsing helpers for the WASM codegen.

These are pure-string utilities that match Tcl source patterns the
emitter needs at compile time (array references, interpolated strings,
namespace qualification). Kept here so the main emitter module stays
focused on code emission rather than token parsing.
"""

from __future__ import annotations


def _parse_array_ref(name: str) -> tuple[str, str] | None:
    """Return ``(array_name, key_text)`` if *name* looks like ``arr(key)``.

    Returns ``None`` for scalar names.  The key may be a literal, a
    variable reference (``$i``), an interpolated string
    (``counter-$tag``), or a command substitution — the caller is
    responsible for emitting it as a TclObj value via ``_emit_value``.

    Array names can themselves contain ``::`` (``::ns::arr(key)``) and
    the key may contain ``)``-balanced nested parens — we require
    properly nested parens with the final ``)`` matching the first
    unescaped ``(``.
    """
    # Strip Tcl's ``$=`` scalar-disambiguator prefix that the lowerer
    # sometimes applies to braced ``${x(1)}`` scalars so we don't
    # misidentify those as array refs.
    if not name or "(" not in name or not name.endswith(")"):
        return None
    if name.startswith("$={"):
        return None
    # Find the first '(' that isn't escaped.
    i = 0
    n = len(name)
    while i < n:
        if name[i] == "\\" and i + 1 < n:
            i += 2
            continue
        if name[i] == "(":
            break
        i += 1
    else:
        return None
    arr = name[:i]
    if not arr:
        return None
    # The key runs from i+1 to the matching ')', which is the last char.
    key = name[i + 1 : -1]
    return (arr, key)


def _derive_proc_namespace(qname: str) -> str:
    """Return the namespace containing *qname*.

    ``::counter::init`` → ``::counter``; ``::main`` → ``::``. Used to
    resolve ``variable x`` inside a proc to ``::counter::x``.
    """
    if not qname:
        return "::"
    # Strip leading ``::`` for the search, then re-add.
    bare = qname[2:] if qname.startswith("::") else qname
    idx = bare.rfind("::")
    if idx < 0:
        return "::"
    return "::" + bare[:idx]


def _has_embedded_subst_scan(value: str) -> bool:
    """Detect embedded $var / ${var} / [cmd] substitutions mixed with literal text.

    Returns True when *value* contains both literal characters and a
    substitution — a pure ``$x`` / ``${x}`` / ``[cmd]`` reference returns
    False (handled by the single-token paths in _emit_value).
    """
    # Strip off a pure variable or command substitution to see if there's
    # anything else present.
    if value.startswith("${") and value.endswith("}"):
        # A single ``${name}`` is pure only when the inner text has no
        # further substitutions or braces.  ``${a}${b}`` starts with
        # ``${`` and ends with ``}`` but is NOT pure.
        inner = value[2:-1]
        if "$" not in inner and "[" not in inner and "}" not in inner:
            return False
    if value.startswith("$") and not value.startswith("$["):
        # Check whether the entire string is just $name (possibly with
        # :: namespace separators, e.g. $::ns::var).
        j = 1
        n = len(value)
        while j < n:
            ch = value[j]
            if ch.isalnum() or ch == "_":
                j += 1
            elif ch == ":" and j + 1 < n and value[j + 1] == ":":
                j += 2
            else:
                break
        if j == n:
            return False
    if value.startswith("[") and value.endswith("]"):
        return False
    i = 0
    n = len(value)
    while i < n:
        c = value[i]
        if c == "\\" and i + 1 < n:
            i += 2
            continue
        if c == "$" and i + 1 < n:
            nxt = value[i + 1]
            if nxt == "{" or nxt.isalpha() or nxt == "_" or nxt == ":":
                return True
        if c == "[":
            return True
        i += 1
    return False
