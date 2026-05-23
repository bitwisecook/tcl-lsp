"""Class Hierarchy Analysis (CHA) for TclOO.

Builds a complete class hierarchy from the workspace class index,
computes the MRO (two-pass DFS with late-placement deduplication)
for each class, and answers queries about
subtype relationships, method providers, and method resolution.

Inspired by the CHA techniques used in LLVM and JVM HotSpot for
devirtualisation and call graph construction.
"""

from __future__ import annotations

from dataclasses import dataclass

from .mro import MROError, tcloo_linearise
from .semantic_model import ClassDef


@dataclass(frozen=True, slots=True)
class ClassHierarchy:
    """Immutable snapshot of the complete class hierarchy."""

    classes: dict[str, ClassDef]
    """All known classes, keyed by qualified name."""

    mro_map: dict[str, tuple[str, ...]]
    """Class → linearised MRO (including self)."""

    subclasses: dict[str, frozenset[str]]
    """Class → direct subclasses."""

    transitive_subtypes: dict[str, frozenset[str]]
    """Class → all descendants (transitive closure)."""

    method_providers: dict[tuple[str, str], str]
    """(class, method_name) → qualified name of the class that provides the
    method in the MRO chain.  Used for ``next``/``nextto`` resolution and
    devirtualisation."""

    errors: tuple[str, ...]
    """Hierarchy errors (cycles, inconsistent linearisation)."""

    def is_subtype(self, child: str, parent: str) -> bool:
        """Return True if *child* is a subtype of *parent*."""
        mro = self.mro_map.get(child, ())
        return parent in mro

    def method_target(self, class_name: str, method_name: str) -> str | None:
        """Resolve which class provides *method_name* for *class_name*."""
        return self.method_providers.get((class_name, method_name))

    def all_implementations(self, method_name: str) -> list[tuple[str, str]]:
        """Return all (class, defining_class) pairs that implement *method_name*."""
        results: list[tuple[str, str]] = []
        for (cls, meth), provider in self.method_providers.items():
            if meth == method_name:
                results.append((cls, provider))
        return results


def build_class_hierarchy(classes: dict[str, ClassDef]) -> ClassHierarchy:
    """Build a ``ClassHierarchy`` from a dict of class definitions.

    Computes TclOO MRO, subclass maps, and method provider resolution
    for all classes in the index.
    """
    # Build separate superclasses and mixins maps for TclOO DFS algorithm
    supers_map: dict[str, list[str]] = {}
    mixins_map: dict[str, list[str]] = {}

    def _normalise_names(names: list[str]) -> list[str]:
        normalised: list[str] = []
        for p in names:
            if p.startswith("::"):
                normalised.append(p)
            elif f"::{p}" in classes:
                normalised.append(f"::{p}")
            else:
                normalised.append(p)
        return normalised

    for qname, cd in classes.items():
        supers_map[qname] = _normalise_names(list(cd.superclasses))
        if cd.mixins:
            mixins_map[qname] = _normalise_names(list(cd.mixins))

    # Compute MRO for each class using TclOO's DFS + late-placement algorithm
    mro_map: dict[str, tuple[str, ...]] = {}
    errors: list[str] = []
    for qname in classes:
        if qname in mro_map:
            continue
        try:
            mro = tcloo_linearise(qname, supers_map, mixins_map=mixins_map)
            mro_map[qname] = tuple(mro)
        except MROError as e:
            errors.append(str(e))
            mro_map[qname] = (qname,)

    # Build direct subclass map
    direct_subs: dict[str, set[str]] = {qname: set() for qname in classes}
    for qname, cd in classes.items():
        for parent in supers_map.get(qname, []):
            if parent in direct_subs:
                direct_subs[parent].add(qname)

    subclasses = {k: frozenset(v) for k, v in direct_subs.items()}

    # Build transitive subtypes via BFS
    transitive: dict[str, frozenset[str]] = {}
    for qname in classes:
        visited: set[str] = set()
        stack = list(direct_subs.get(qname, set()))
        while stack:
            child = stack.pop()
            if child in visited:
                continue
            visited.add(child)
            stack.extend(direct_subs.get(child, set()))
        transitive[qname] = frozenset(visited)

    # Build method provider map
    method_providers: dict[tuple[str, str], str] = {}
    for qname in classes:
        mro = mro_map.get(qname, (qname,))
        # Collect all method names for this class
        cd = classes[qname]
        all_methods = set(cd.methods.keys()) | set(cd.class_methods.keys())
        # Also include methods from ancestors
        for ancestor_name in mro:
            ancestor = classes.get(ancestor_name)
            if ancestor:
                all_methods |= set(ancestor.methods.keys()) | set(ancestor.class_methods.keys())

        for method_name in all_methods:
            # Walk MRO to find first provider
            for ancestor_name in mro:
                ancestor = classes.get(ancestor_name)
                if ancestor and (
                    method_name in ancestor.methods or method_name in ancestor.class_methods
                ):
                    method_providers[(qname, method_name)] = ancestor_name
                    break

    return ClassHierarchy(
        classes=classes,
        mro_map=mro_map,
        subclasses=subclasses,
        transitive_subtypes=transitive,
        method_providers=method_providers,
        errors=tuple(errors),
    )
