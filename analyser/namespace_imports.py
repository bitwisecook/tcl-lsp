"""Helpers for resolving namespace-imported command references.

When a Tcl script contains ``namespace import ::foo::bar::*`` inside
namespace ``baz`` (or at the global level), the command names in the
source namespace become available in the importing namespace under
their tail names.  So a later call to ``baz::thing`` actually invokes
``::foo::bar::thing``.  This module rewrites a referenced word through
a list of :class:`NamespaceImport` declarations so cross-file lookup
can find the real target.
"""

from __future__ import annotations

from collections.abc import Iterable

from .semantic_model import NamespaceImport


def rewrite_via_imports(word: str, imports: Iterable[NamespaceImport]) -> list[str]:
    """Return candidate fully-qualified names for *word*.

    The input *word* is the text the user wrote (e.g. ``vt::showat``).
    Each candidate is a fully-qualified name derived by applying one of
    the *imports* declarations.  Non-conjectured imports are listed
    first so the caller can prefer them.
    """
    if not word:
        return []
    # Split the reference into ``(importing_ns, tail)``.  A bare name
    # matches global imports (``namespace import`` at the ``::`` level),
    # while a qualified name ``vt::showat`` matches imports into
    # namespace ``::vt``.
    if word.startswith("::"):
        body = word[2:]
    else:
        body = word
    if "::" in body:
        alias_ns_body, tail = body.rsplit("::", 1)
        alias_ns = f"::{alias_ns_body}" if alias_ns_body else "::"
    else:
        alias_ns = "::"
        tail = body

    non_conjectured: list[str] = []
    conjectured: list[str] = []
    for imp in imports:
        if imp.ns != alias_ns:
            continue
        rewritten = _apply_pattern(imp.pattern, tail)
        if rewritten is None:
            continue
        (conjectured if imp.conjectured else non_conjectured).append(rewritten)
    # Deduplicate while preserving order.
    seen: set[str] = set()
    out: list[str] = []
    for cand in non_conjectured + conjectured:
        if cand not in seen:
            seen.add(cand)
            out.append(cand)
    return out


def _apply_pattern(pattern: str, tail: str) -> str | None:
    """Resolve ``namespace import`` *pattern* applied to *tail*.

    ``::foo::bar::*`` + ``baz`` → ``::foo::bar::baz``.
    ``::foo::bar::baz`` + ``baz`` → ``::foo::bar::baz`` (exact match).
    Glob characters other than a trailing ``*`` are not supported; Tcl's
    own ``namespace import`` accepts them but tcllib / user code almost
    never uses that form.
    """
    if pattern.endswith("::*"):
        return f"{pattern[:-1]}{tail}"
    if "::" in pattern:
        head, pat_tail = pattern.rsplit("::", 1)
        if pat_tail == tail:
            return pattern
        if pat_tail == "*":
            return f"{head}::{tail}"
    return None
