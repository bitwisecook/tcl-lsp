# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Consistent naming helpers for Tcl identifiers."""

from __future__ import annotations

from typing import NewType

# Typed wrappers (NewType, runtime-equivalent to ``str``) that distinguish
# raw token text — what the user typed, with no resolution applied — from
# resolved canonical names produced by the lowering-time canonicalisation
# helpers.  Static type checkers (mypy, ty) flag accidental mixing; at
# runtime both are plain strings, so existing string-typed APIs (e.g.
# ``IRCall.canonical_command: str``) keep working without conversion.
# Adopt incrementally — annotating a new helper with ``CanonicalCommand``
# documents its contract to readers and to the type checker.
# See issue #246 for the canonicalisation contract.
CanonicalCommand = NewType("CanonicalCommand", str)
"""``::ns::cmd`` form of a Tcl command name, post alias / namespace-import
/ rename resolution.  Stamped on ``IRCall.canonical_command`` /
``IRBarrier.canonical_command`` at lowering."""

CanonicalVar = NewType("CanonicalVar", str)
"""Canonical Tcl variable name — bare locals stay bare, qualified
``::ns::v`` stays qualified.  The bare/qualified distinction is
semantically meaningful (local vs global) and is preserved by
:func:`to_canonical_var`; per-call-site scope resolution (``global`` /
``variable`` / ``upvar`` declarations) is the analyser's job and is
not folded into this form."""

RawCommandText = NewType("RawCommandText", str)
"""Raw command word as written at the call site.  Stamped on
``IRCall.command`` / ``IRBarrier.command`` for diagnostic rendering and
source-fidelity passes (formatter, refactor)."""


def is_bare_var_name(name: str) -> bool:
    """Return True when ``$name`` would lex as a single bare variable token.

    Mirrors ``compiler.parsing.lexer._parse_var``'s bare-form rule: a name is
    one or more ``::``-separated segments, each consisting of Unicode
    alnum or ``_`` characters.  An optional leading ``::`` is allowed.

    Used to decide between the bare ``$name`` and brace ``${name}`` forms
    in completion / quick fixes -- and as the inverse of "needs braces".
    Centralised here so both the completion provider and the W216
    analyser pass derive their decision from a single source of truth
    rather than independent regexes that may drift from the lexer rule.
    """
    if not name:
        return False
    s = name[2:] if name.startswith("::") else name
    if not s:
        return False
    for segment in s.split("::"):
        if not segment:
            return False
        for ch in segment:
            if not (ch.isalnum() or ch == "_"):
                return False
    return True


def is_unqualified_var_name(name: str) -> bool:
    """Return True when *name* is an unqualified single variable name.

    That is, a plain local scalar: a bare identifier with no ``::`` namespace
    qualifier and no array ``(idx)`` or glob characters.  Derived from
    :func:`is_bare_var_name` (the lexer's bare-name rule) so callers do not
    hand-roll a regex that can drift from it.
    """
    return "::" not in name and is_bare_var_name(name)


def is_brace_substitutable(name: str) -> bool:
    """Return True when ``${name}`` would successfully look up ``name``.

    Mirrors Tcl 9.0.3's ``Tcl_ParseVarName`` brace-form parser
    (tclParse.c §1383+):

    - ``\\X`` (backslash + any char) consumes 2 source chars, both of
      which stay in the lookup name.  So a closing ``}`` preceded by
      ``\\`` does not end the var-name span -- but the ``\\`` is also
      preserved in the looked-up name.
    - ``{`` / ``}`` are tracked with a depth counter; only a ``}`` at
      depth 0 ends the var-name span.

    Verified empirically against ``tclsh`` 9.0.3 in
    ``docs/kcs/kcs-tcl-corner-cases.md``.

    Strategy: simulate the parser using ``source = name``.  If it
    produces ``name`` and closes successfully, ``name`` is reachable.
    The function returns ``False`` for:

    - any ``}`` in ``name`` not preceded by a balanced inner ``{``
      (the ``}`` would close the brace before all chars are read);
    - a trailing single ``\\`` (the parser's ``\\X``-consumes-2 rule
      can't close because there's no next char);
    - unbalanced ``{`` (depth > 0 at end -- missing close-brace).

    Names that pass this check are reachable via ``${name}`` exactly.
    Bare ``$name`` reachability is a strict subset (covered by
    :func:`is_bare_var_name`).
    """
    if not name:
        return True  # ``${}`` looks up the var literally named ""
    n = len(name)
    depth = 0
    i = 0
    while i < n:
        ch = name[i]
        if ch == "}" and depth == 0:
            return False
        if ch == "\\":
            if i + 1 >= n:
                return False
            i += 2
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        i += 1
    return depth == 0


def is_braced_indirect_array_ref(word: str) -> bool:
    """Return True when *word* is the braced indirect-array-element idiom
    ``${name}(index)``.

    Tcl parses ``${name}(index)`` as the brace-form substitution ``${name}``
    (which the lexer ends at the ``}``) concatenated with the *literal* text
    ``(index)``.  When this word sits in a **variable-name position**
    (the target of ``set`` / ``incr`` / ``append`` / ``lappend`` / ``unset`` /
    ``info exists`` / ``vwait`` …) the resulting string
    ``<value-of-name>(index)`` names element ``index`` of the array whose name
    is held in the scalar ``name`` — the standard "array name kept in a
    variable" access idiom (e.g. Tcl's ``http`` package:
    ``set ${token}(status) eof``, where ``token`` holds ``::http::1``).

    The braces are essential: only ``${name}`` substitutes the *whole* name
    before the literal ``(index)`` is appended.  The bare ``$name(index)`` is a
    *direct* array reference (an array literally named ``name`` with the index
    substituted), a different construct — so this returns False for it.

    This is the discriminator that keeps the W216 (brace-then-paren) and W212
    (substitution-where-name-expected) checks from false-positiving on the
    indirect idiom while still firing on a genuine value-position
    ``${arr}(x)`` typo for ``$arr(x)``.
    """
    if not word.startswith("${"):
        return False
    # Walk to the depth-0 ``}`` that closes the brace-form variable name,
    # mirroring the brace parser in :func:`is_brace_substitutable`.
    i = 2
    n = len(word)
    depth = 0
    close = -1
    while i < n:
        ch = word[i]
        if ch == "\\" and i + 1 < n:
            i += 2
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            if depth == 0:
                close = i
                break
            depth -= 1
        i += 1
    # No closing brace, or an empty name (``${}(…)``) — not the idiom.
    if close <= 2:
        return False
    # The closing ``}`` must be immediately followed by ``(…)`` that runs to
    # the end of the word.
    rest = word[close + 1 :]
    return len(rest) >= 2 and rest[0] == "(" and rest[-1] == ")"


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


def split_array_name(name: str) -> tuple[str, str | None]:
    """Split a Tcl variable reference into ``(base, element)``.

    Strips ``$`` / ``${...}`` substitution sigils first, then separates
    the optional ``(element)`` array-index suffix from the base name.
    Returns ``(base, None)`` for scalar references.

    Brace and bare forms differ on what's "part of the reference":

    - ``${name}`` reads exactly the chars inside the braces.  Anything
      after the closing ``}`` is unrelated literal text -- e.g. Tcl
      parses ``${arr}(foo)`` as scalar ``${arr}`` followed by the
      literal characters ``(foo)``, *not* as an array reference.
    - Inside a brace form, ``(idx)`` *is* an array reference, because
      ``${arr(foo)}`` is just the brace form of ``$arr(foo)`` -- the
      runtime treats both as the literal name ``arr(foo)``.

    Examples::

        split_array_name("arr")            -> ("arr", None)
        split_array_name("arr(foo)")       -> ("arr", "foo")
        split_array_name("$arr(foo)")      -> ("arr", "foo")
        split_array_name("${arr(foo)}")    -> ("arr", "foo")
        split_array_name("${arr}(foo)")    -> ("arr", None)   # not an array
        split_array_name("${arr}")         -> ("arr", None)
    """
    base = name
    if base.startswith("${") and "}" in base:
        end = base.index("}")
        inner = base[2:end]
        # Anything after ``}`` is unrelated literal text -- ignore it.
        if "(" in inner and inner.endswith(")"):
            idx = inner.index("(")
            return inner[:idx], inner[idx + 1 : -1]
        return inner, None
    if base.startswith("$"):
        base = base[1:]
    if "(" in base and base.endswith(")"):
        idx = base.index("(")
        return base[:idx], base[idx + 1 : -1]
    return base, None


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
