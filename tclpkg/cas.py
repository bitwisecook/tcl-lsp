"""Content-addressable store (CAS) and integrity hashing.

The CAS lives at ``<cache_dir>/tclpkg/cas/sha256/<ab>/<full_hash>/tree/``
where ``<ab>`` is the first two hex characters of the SHA-256 digest.  Each
entry is immutable once written — a later ``tcl pkg sync`` can recreate
``./lib/<pkg>-<ver>/`` symlinks from the CAS without a network round-trip.

The integrity string has the shape ``sha256-<base64url-no-pad>`` (the same
format as Subresource Integrity and Zig's package hashes).  It is computed
over a **canonicalised worktree** — not the raw tarball — so a
re-compressed archive still hashes identically.

Canonicalisation rules:

1. Strip ``.git/``, ``.hg/``, ``.svn/``, ``.fossil/``, ``.DS_Store``,
   ``Thumbs.db``, and anything listed in a top-level ``.tclpkgignore``.
2. Sort entries by POSIX path (byte-order).
3. For each file emit path, mode, size, per-file SHA-256, and content.
4. Feed the concatenation through SHA-256.
5. Encode as base64url without trailing ``=``.
"""

from __future__ import annotations

import base64
import hashlib
import os
import shutil
import stat
from pathlib import Path

from .errors import IntegrityError

_IGNORED_NAMES: frozenset[str] = frozenset(
    {
        ".git",
        ".hg",
        ".svn",
        ".fossil",
        ".DS_Store",
        "Thumbs.db",
    }
)


def _load_tclpkgignore(root: Path) -> set[str]:
    """Read ``.tclpkgignore`` patterns (one plain name per line)."""
    ignore_file = root / ".tclpkgignore"
    if not ignore_file.is_file():
        return set()
    patterns: set[str] = set()
    for line in ignore_file.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            patterns.add(stripped)
    return patterns


def _walk_tree(root: Path, extra_ignore: set[str] | None = None) -> list[Path]:
    """Return sorted relative paths for all files under *root*.

    Directories matching ``_IGNORED_NAMES`` or *extra_ignore* are pruned.
    Symlinks are followed (files only).
    """
    ignored = _IGNORED_NAMES | (extra_ignore or set())
    result: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        # Prune ignored directories in-place so os.walk skips them.
        dirnames[:] = [d for d in dirnames if d not in ignored]
        rel_dir = Path(dirpath).relative_to(root)
        for fname in filenames:
            if fname in ignored:
                continue
            result.append(rel_dir / fname)
    result.sort(key=lambda p: p.as_posix().encode())
    return result


def integrity_of_tree(root: Path) -> str:
    """Compute the canonical SHA-256 integrity hash of a package worktree.

    Returns a string ``"sha256-<base64url_no_pad>"`` that is stable across
    machines (timestamps, uid, gid, xattrs are ignored; only permission
    bits + execute bit and file content matter).
    """
    extra = _load_tclpkgignore(root)
    hasher = hashlib.sha256()
    for rel_path in _walk_tree(root, extra):
        abs_path = root / rel_path
        try:
            st = abs_path.stat()  # follows symlinks
        except OSError:
            continue
        mode = stat.S_IMODE(st.st_mode)
        # Derive executability from mode bits, not os.access(), for
        # cross-machine determinism (os.access checks effective UID/ACL).
        if mode & 0o111:
            mode |= 0o111
        try:
            content = abs_path.read_bytes()
        except OSError:
            continue
        file_hash = hashlib.sha256(content).hexdigest()
        hasher.update(rel_path.as_posix().encode() + b"\n")
        hasher.update(f"{mode:o}\n".encode())
        hasher.update(f"{len(content)}\n".encode())
        hasher.update(file_hash.encode() + b"\n")
        hasher.update(content)
    digest = hasher.digest()
    encoded = base64.urlsafe_b64encode(digest).rstrip(b"=").decode()
    return f"sha256-{encoded}"


def verify_integrity(root: Path, expected: str) -> bool:
    """Recompute the integrity hash and return whether it matches *expected*."""
    actual = integrity_of_tree(root)
    return actual == expected


class ContentAddressableStore:
    """Manage the ``sha256/<ab>/<hash>/tree/`` CAS on disk."""

    def __init__(self, cache_dir: Path) -> None:
        self._base = cache_dir / "tclpkg" / "cas" / "sha256"

    def _entry_dir(self, integrity: str) -> Path:
        """Return the CAS directory for a given integrity string."""
        if not integrity.startswith("sha256-"):
            raise IntegrityError(f"unsupported integrity format: {integrity}")
        # Decode back to hex for the sharded path.
        b64 = integrity[len("sha256-") :]
        # Re-pad for base64 decoding.
        padded = b64 + "=" * (-len(b64) % 4)
        try:
            raw = base64.urlsafe_b64decode(padded)
        except Exception as exc:
            raise IntegrityError(
                f"malformed integrity hash: {integrity}",
                hint="the lockfile may be corrupted; delete tclpkg.lock and re-run 'tcl pkg install'",
            ) from exc
        hex_digest = raw.hex()
        shard = hex_digest[:2]
        return self._base / shard / hex_digest

    def has(self, integrity: str) -> bool:
        """Check whether the CAS contains an entry for *integrity*."""
        tree = self._entry_dir(integrity) / "tree"
        return tree.is_dir()

    def tree_path(self, integrity: str) -> Path:
        """Return the ``tree/`` directory for an existing CAS entry."""
        tree = self._entry_dir(integrity) / "tree"
        if not tree.is_dir():
            raise IntegrityError(
                f"CAS entry missing for {integrity}",
                hint="run 'tcl pkg install' to populate the cache",
            )
        return tree

    def store(
        self,
        source_dir: Path,
        *,
        name: str = "",
        version: str = "",
        expected_integrity: str | None = None,
    ) -> str:
        """Copy a worktree into the CAS and return its integrity string.

        If *expected_integrity* is provided and does not match the computed
        hash, raises :class:`IntegrityError`.
        """
        actual = integrity_of_tree(source_dir)
        if expected_integrity and actual != expected_integrity:
            raise IntegrityError(
                f"integrity mismatch for {name}@{version}",
                expected=expected_integrity,
                actual=actual,
                hint="the download may be corrupted; delete the cache and retry",
            )
        entry = self._entry_dir(actual)
        tree = entry / "tree"
        if tree.is_dir():
            return actual  # already cached
        entry.mkdir(parents=True, exist_ok=True)
        # Write metadata.
        if name:
            (entry / "package.txt").write_text(f"{name} {version}\n", encoding="utf-8")
        # Copy the tree.
        shutil.copytree(source_dir, tree, dirs_exist_ok=False)
        return actual

    def materialise(
        self,
        integrity: str,
        dest: Path,
        *,
        symlink: bool = True,
    ) -> None:
        """Create ``dest`` as a symlink (or copy) pointing into the CAS.

        If *symlink* is ``True`` (default), ``dest`` becomes a symlink to
        the CAS ``tree/`` directory.  On platforms where symlinks require
        elevated privileges, falls back to a full copy.
        """
        tree = self.tree_path(integrity)
        if dest.exists() or dest.is_symlink():
            if dest.is_symlink():
                dest.unlink()
            elif dest.is_dir():
                shutil.rmtree(dest)
            else:
                dest.unlink()
        dest.parent.mkdir(parents=True, exist_ok=True)
        if symlink:
            try:
                dest.symlink_to(tree)
                return
            except OSError:
                pass  # fall through to copy
        shutil.copytree(tree, dest)


__all__ = [
    "ContentAddressableStore",
    "integrity_of_tree",
    "verify_integrity",
]
