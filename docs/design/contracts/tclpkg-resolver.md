# tclpkg MVS resolver

## Symptom

Dependency resolution produces unexpected versions, or a replace/exclude
directive does not take effect.

## Decision rules / contracts

1. BFS walk over the dependency graph picks max-of-minimums for each package.
2. No upper bounds, no backtracking, no SAT solver.
3. ``replace`` from the root manifest forces a specific version for a named
   package. Transitive ``replace`` is ignored (root has final authority).
4. ``exclude`` from the root manifest rejects an exact ``(name, version)``
   pair. If MVS would select the excluded version, ``ResolutionError`` is
   raised with the chain that selected it.
5. A convergence pass re-processes packages whose minimums were bumped after
   initial processing.
6. Dev dependencies are included by default; ``--no-dev`` excludes them.
7. Result is sorted alphabetically by package name.
8. Maximum 10,000 iterations (safety valve against cycles).

## File-path anchors

- ``tclpkg/resolver.py`` — ``resolve()``, ``PackageRef``, ``ResolvedPackage``

## Test anchors

- ``tests/tclpkg/test_resolver.py`` — 15 tests: diamond, MVS max, replace, exclude, dev
