# tclpkg cache and integrity

## Symptom

A package fails integrity verification, or the CAS does not contain an
expected entry.

## Decision rules / contracts

1. Cache location: ``~/.cache/tcl-lsp/tclpkg/cas/sha256/<ab>/<hash>/tree/``.
2. ``<ab>`` is the first two hex characters of the SHA-256 digest (sharding).
3. Integrity string format: ``sha256-<base64url-no-pad>`` (SRI-compatible).
4. Hash computed over the canonicalised worktree, NOT raw archive bytes.
5. Canonicalisation: strip ``.git/``, ``.hg/``, ``.DS_Store``, ``.tclpkgignore``
   entries; sort files by POSIX path byte-order; hash path + mode + size +
   per-file SHA-256 + content.
6. Timestamps, uid, gid, xattrs are deliberately ignored for cross-machine
   stability.
7. CAS entries are immutable once written — a later ``store()`` with the same
   hash is a no-op.
8. Materialisation into ``lib/`` uses symlinks by default; falls back to copy
   on platforms that restrict symlinks.
9. ``_cache_dir()`` at ``shared/user_config.py`` follows
   ``$XDG_CACHE_HOME`` / macOS / Windows conventions.

## File-path anchors

- ``tclpkg/cas.py`` — ``integrity_of_tree()``, ``ContentAddressableStore``
- ``shared/user_config.py`` — ``_cache_dir()``

## Test anchors

- ``tests/tclpkg/test_cas.py`` — 18 tests: hashing, ignore, CAS store/materialise
