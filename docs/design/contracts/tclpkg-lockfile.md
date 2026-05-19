# tclpkg lockfile schema

## Symptom

The lockfile differs between two runs, or a newer tool version rejects
an existing lockfile.

## Decision rules / contracts

1. Canonical JSON: sorted keys, 2-space indent, LF endings, final newline.
2. ``packages[]`` sorted by ``(name, version)``.
3. Two invocations against the same manifest + registry produce byte-identical
   output (``generated`` timestamp is the only non-deterministic field).
4. ``--frozen`` preserves the existing ``generated`` timestamp.
5. Schema version (``"version": 1``) bumped only on incompatible changes.
6. Schema version > supported raises ``TclPkgError`` with upgrade hint.
7. Atomic writes via temp-file + rename (no partial lockfiles on crash).
8. ``integrity`` field format: ``sha256-<base64url-no-pad>``.

## File-path anchors

- ``tooling/tclpkg/lockfile.py`` — ``LockFile``, ``serialise()``, ``deserialise()``, ``write_lockfile()``

## Test anchors

- ``tests/tooling/tclpkg/test_lockfile.py`` — 23 tests covering serialisation, round-tripping, and error cases
