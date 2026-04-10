"""Go-style Minimum Version Selection (MVS) resolver.

MVS is deterministic and solverless: every ``require <pkg> <minver>``
declares a minimum, and the resolver picks the maximum-of-minimums for
each package across the whole graph.  No upper bounds, no backtracking,
no SAT solver.

The resolver itself is a pure function over data — it does not fetch or
cache anything.  Callers supply a ``PackageManifestProvider`` callback
that maps ``(name, version)`` to the package's own transitive
requirements.  During a real install the provider reads from the CAS
(after fetching); during tests it is a simple dict lookup.

Replace directives from the root manifest override resolved versions.
Exclude directives from the root manifest reject specific versions.
Both operate only from the root — transitive replace/exclude is
deliberately ignored (matching Go's behaviour) so the top of the graph
has final authority.
"""

from __future__ import annotations

from collections import deque
from collections.abc import Callable
from dataclasses import dataclass, field

from .errors import ResolutionError
from .version import Version


@dataclass(frozen=True)
class PackageRef:
    """A named package at a specific version."""

    name: str
    version: Version

    def __str__(self) -> str:
        return f"{self.name}@{self.version}"


@dataclass
class ResolvedPackage:
    """One entry in the resolved dependency graph."""

    ref: PackageRef
    requires: list[PackageRef] = field(default_factory=list)
    dev: bool = False

    def __str__(self) -> str:
        return str(self.ref)


@dataclass(frozen=True)
class ReplaceSpec:
    """Root manifest ``replace`` directive: force *name* to *version*."""

    name: str
    version: Version


@dataclass(frozen=True)
class ExcludeSpec:
    """Root manifest ``exclude`` directive: refuse this exact version."""

    name: str
    version: Version


# The provider maps (name, version) → list of PackageRef requirements.
# For the initial resolve (before packages are fetched), callers either
# return an empty list (leaf package) or the real requirements from the
# package's own manifest.
PackageManifestProvider = Callable[[str, Version], list[PackageRef]]


def resolve(
    direct: list[PackageRef],
    dev_direct: list[PackageRef] | None = None,
    *,
    replaces: list[ReplaceSpec] | None = None,
    excludes: list[ExcludeSpec] | None = None,
    provider: PackageManifestProvider | None = None,
    include_dev: bool = True,
) -> list[ResolvedPackage]:
    """Run MVS over the dependency graph and return the resolved set.

    Parameters
    ----------
    direct:
        Production ``require`` refs from the root manifest.
    dev_direct:
        Development-only ``dev-require`` refs (optional).
    replaces:
        Root-level ``replace`` directives.
    excludes:
        Root-level ``exclude`` directives.
    provider:
        Callback ``(name, version) -> [PackageRef]`` returning the
        transitive requirements of a package.  ``None`` means every
        package is a leaf (no transitive deps).
    include_dev:
        Whether to pull in ``dev_direct`` packages.

    Returns
    -------
    Sorted list of :class:`ResolvedPackage` (topological by name).

    Raises
    ------
    ResolutionError
        If an excluded version is selected or a provider raises.
    """
    replace_map: dict[str, ReplaceSpec] = {}
    if replaces:
        for r in replaces:
            replace_map[r.name] = r

    exclude_set: set[tuple[str, Version]] = set()
    if excludes:
        for e in excludes:
            exclude_set.add((e.name, e.version))

    dev_names: set[str] = set()
    if dev_direct:
        for r in dev_direct:
            dev_names.add(r.name)

    _provider = provider or (lambda _n, _v: [])

    # Phase 1: collect max-of-minimums via BFS.
    minimums: dict[str, Version] = {}
    pending: deque[PackageRef] = deque()

    # Seed with direct deps.
    for ref in direct:
        pending.append(ref)
    if include_dev and dev_direct:
        for ref in dev_direct:
            pending.append(ref)

    # Track what each package requires so we can build the graph later.
    requires_map: dict[str, list[PackageRef]] = {}

    iterations = 0
    max_iterations = 10_000  # safety valve

    while pending:
        iterations += 1
        if iterations > max_iterations:
            raise ResolutionError("resolver exceeded maximum iterations (possible cycle)")

        ref = pending.popleft()
        name = ref.name
        version = ref.version

        # Apply replace.
        if name in replace_map:
            rep = replace_map[name]
            version = rep.version

        # Check exclude.
        if (name, version) in exclude_set:
            raise ResolutionError(
                f"package {name} {version} is excluded by the root manifest",
                hint=f"remove the exclude directive or adjust the version constraint for {name}",
            )

        # Update minimum if this ref bumps it.
        current = minimums.get(name)
        if current is not None and version <= current:
            continue  # already satisfied
        minimums[name] = version

        # Fetch this package's own requirements.
        try:
            child_refs = _provider(name, version)
        except ResolutionError:
            raise
        except Exception as exc:
            raise ResolutionError(
                f"failed to read requirements of {name}@{version}: {exc}",
            ) from exc

        requires_map[name] = child_refs

        for child in child_refs:
            pending.append(child)

    # Phase 2: convergence — if we bumped a minimum after already
    # processing a package, re-process it at the new version.
    dirty = True
    while dirty:
        dirty = False
        for name, wanted in list(minimums.items()):
            recorded = requires_map.get(name)
            if recorded is None:
                # Not yet processed — this shouldn't happen after BFS,
                # but handle it defensively.
                try:
                    child_refs = _provider(name, wanted)
                except Exception as exc:
                    raise ResolutionError(
                        f"failed to read requirements of {name}@{wanted}: {exc}",
                    ) from exc
                requires_map[name] = child_refs
                for child in child_refs:
                    pending.append(child)
                dirty = True
                break

    # Drain any leftover pending from phase 2.
    while pending:
        ref = pending.popleft()
        name = ref.name
        version = ref.version
        if name in replace_map:
            version = replace_map[name].version
        if (name, version) in exclude_set:
            raise ResolutionError(
                f"package {name} {version} is excluded by the root manifest",
            )
        current = minimums.get(name)
        if current is not None and version <= current:
            continue
        minimums[name] = version
        try:
            child_refs = _provider(name, version)
        except Exception as exc:
            raise ResolutionError(
                f"failed to read requirements of {name}@{version}: {exc}",
            ) from exc
        requires_map[name] = child_refs
        for child in child_refs:
            pending.append(child)

    # Phase 3: build the result list.
    result: list[ResolvedPackage] = []
    for name in sorted(minimums):
        ver = minimums[name]
        reqs = requires_map.get(name, [])
        is_dev = name in dev_names
        result.append(
            ResolvedPackage(
                ref=PackageRef(name=name, version=ver),
                requires=reqs,
                dev=is_dev,
            )
        )

    return result


__all__ = [
    "ExcludeSpec",
    "PackageManifestProvider",
    "PackageRef",
    "ReplaceSpec",
    "ResolvedPackage",
    "resolve",
]
