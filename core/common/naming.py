"""Consistent naming helpers for Tcl identifiers."""

from __future__ import annotations


def normalise_var_name(name: str) -> str:
    """Normalise Tcl variable forms to their base name."""
    base = name
    if base.startswith("${") and base.endswith("}"):
        base = base[2:-1]
    elif base.startswith("$"):
        base = base[1:]
    if "(" in base:
        base = base.split("(", 1)[0]
    return base


def normalise_qualified_name(name: str) -> str:
    """Normalise a possibly-qualified Tcl command/proc name."""
    if not name:
        return name
    parts = [part for part in name.split("::") if part]
    if not parts:
        return "::"
    return "::" + "::".join(parts)


def to_canonical_var(name: str, *, scope: str = "::") -> str:
    """Return the canonical form of a Tcl variable name.

    Strips substitution sigils (``$``, ``${...}``) and array-index
    suffixes via :func:`normalise_var_name`, then applies the standard
    "qualified stays qualified, bare stays bare" rule:

    - ``$x`` / ``${x}`` / ``x`` → ``x``  (local variable; bare form is
      already canonical relative to its scope).
    - ``::x`` → ``::x``  (global namespace variable).
    - ``::ns::x`` → ``::ns::x``  (namespace variable).

    *scope* is reserved for future per-call-site resolution that walks
    captured ``global`` / ``variable`` / ``upvar`` declarations and
    rewrites bare locals to their resolved fully-qualified form.  The
    current implementation accepts but does not use *scope* — bare
    names stay bare, mirroring the analyser's existing semantics where
    bare ``x`` and ``::x`` are deliberately distinct values (one local,
    one global) and only the analyser's scope tables decide they alias.
    See issue #246.
    """
    return normalise_var_name(name)


def from_canonical(name: str, *, display_namespace: str = "::") -> str:
    """Return a user-facing rendering of a canonical command/var name.

    Diagnostic messages should echo what the user wrote, not the
    canonical form analysis matched against — otherwise a hint that
    quoted ``dict for`` source said ``::tcl::dict::for``.  This helper
    strips a leading ``::`` for the user's current display namespace
    so canonical names render naturally:

    - ``from_canonical("::set")`` → ``"set"``  (global builtin).
    - ``from_canonical("::ns::foo", display_namespace="::ns")``
      → ``"foo"``  (visible from the current namespace).
    - ``from_canonical("::other::foo", display_namespace="::ns")``
      → ``"::other::foo"``  (out-of-namespace, render qualified).

    Diagnostic builders that already have access to the original
    ``CommandTokens.argv_texts[0]`` should prefer that — this helper
    is for consumers (hover, code-actions) that only carry the
    canonical name.  See issue #246.
    """
    if not name or not name.startswith("::"):
        return name
    if display_namespace == "::":
        # Global scope: strip the leading ``::`` for top-level builtins
        # (``::set`` → ``set``) but leave nested namespaces alone
        # (``::ns::foo`` keeps its qualifier so the reader sees the
        # cross-namespace reach).
        bare = name[2:]
        if "::" in bare:
            return name
        return bare
    # Inside ``::ns``: a name starting with ``::ns::`` reduces to its
    # tail; anything else is from a sibling namespace and stays
    # qualified so the rendered name preserves the cross-namespace
    # reach the user would see in their own source.
    prefix = display_namespace if display_namespace.endswith("::") else display_namespace + "::"
    if name.startswith(prefix):
        return name[len(prefix) :]
    return name


def is_canonical_command(name: str) -> bool:
    """Predicate: True when *name* is in canonical command form (``::cmd``).

    Used as an assertion on IR-bearing fields (``IRCall.canonical_command``)
    to catch passes that forget to canonicalise.  Empty strings are
    accepted for synthetic IR nodes (``<cond>``, ``<empty_clause>``)
    that have no user-source command.  See issue #246.
    """
    if name == "":
        return True
    if name.startswith("<") and name.endswith(">"):
        # Synthetic CFG node, never user-source.
        return True
    return name.startswith("::")
