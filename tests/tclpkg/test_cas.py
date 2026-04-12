"""Tests for tclpkg.cas — integrity hashing and content-addressable store."""

from __future__ import annotations

from pathlib import Path

import pytest

from tclpkg.cas import ContentAddressableStore, integrity_of_tree, verify_integrity
from tclpkg.errors import IntegrityError


def _make_tree(tmp_path: Path, files: dict[str, str]) -> Path:
    """Create a small file tree under *tmp_path* and return the root."""
    root = tmp_path / "tree"
    root.mkdir(parents=True)
    for rel, content in files.items():
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
    return root


class TestIntegrityOfTree:
    """Tests for the worktree hashing algorithm."""

    def test_stable_across_calls(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n", "b.tcl": "set y 2\n"})
        a = integrity_of_tree(root)
        b = integrity_of_tree(root)
        assert a == b

    def test_starts_with_sha256(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        h = integrity_of_tree(root)
        assert h.startswith("sha256-")

    def test_different_content_different_hash(self, tmp_path) -> None:
        root1 = _make_tree(tmp_path / "a", {"a.tcl": "set x 1\n"})
        root2 = _make_tree(tmp_path / "b", {"a.tcl": "set x 2\n"})
        assert integrity_of_tree(root1) != integrity_of_tree(root2)

    def test_same_content_same_hash(self, tmp_path) -> None:
        root1 = _make_tree(tmp_path / "a", {"a.tcl": "hello\n"})
        root2 = _make_tree(tmp_path / "b", {"a.tcl": "hello\n"})
        assert integrity_of_tree(root1) == integrity_of_tree(root2)

    def test_ignores_dot_git(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "ok\n"})
        git_dir = root / ".git"
        git_dir.mkdir()
        (git_dir / "HEAD").write_text("ref: refs/heads/main\n")
        h_with = integrity_of_tree(root)
        # Hash should be same regardless of .git
        root2 = _make_tree(tmp_path / "clean", {"a.tcl": "ok\n"})
        h_without = integrity_of_tree(root2)
        assert h_with == h_without

    def test_ignores_ds_store(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "ok\n", ".DS_Store": "binary"})
        root2 = _make_tree(tmp_path / "clean", {"a.tcl": "ok\n"})
        assert integrity_of_tree(root) == integrity_of_tree(root2)

    def test_tclpkgignore(self, tmp_path) -> None:
        root = _make_tree(
            tmp_path,
            {"a.tcl": "ok\n", "build": "junk\n", ".tclpkgignore": "build\n"},
        )
        root2 = _make_tree(
            tmp_path / "clean",
            {"a.tcl": "ok\n", ".tclpkgignore": "build\n"},
        )
        assert integrity_of_tree(root) == integrity_of_tree(root2)

    def test_empty_tree(self, tmp_path) -> None:
        root = tmp_path / "empty"
        root.mkdir()
        h = integrity_of_tree(root)
        assert h.startswith("sha256-")

    def test_subdirectories_included(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"lib/foo.tcl": "proc foo {} {}\n", "main.tcl": "foo\n"})
        h = integrity_of_tree(root)
        assert h.startswith("sha256-")


class TestVerifyIntegrity:
    """Tests for the verification helper."""

    def test_valid(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        expected = integrity_of_tree(root)
        assert verify_integrity(root, expected) is True

    def test_invalid(self, tmp_path) -> None:
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        assert verify_integrity(root, "sha256-WRONG") is False


class TestContentAddressableStore:
    """Tests for the CAS store/materialise workflow."""

    def _cas(self, tmp_path: Path) -> ContentAddressableStore:
        return ContentAddressableStore(tmp_path / "cache")

    def test_store_and_has(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        integrity = cas.store(root, name="mypkg", version="1.0.0")
        assert integrity.startswith("sha256-")
        assert cas.has(integrity)

    def test_store_idempotent(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        h1 = cas.store(root, name="a", version="1.0")
        h2 = cas.store(root, name="a", version="1.0")
        assert h1 == h2

    def test_store_verifies_expected_integrity(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root = _make_tree(tmp_path, {"a.tcl": "set x 1\n"})
        with pytest.raises(IntegrityError, match="mismatch"):
            cas.store(root, name="a", version="1.0", expected_integrity="sha256-WRONG")

    def test_materialise_symlink(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root = _make_tree(tmp_path, {"a.tcl": "hello\n"})
        integrity = cas.store(root, name="pkg", version="1.0")
        dest = tmp_path / "lib" / "pkg-1.0"
        cas.materialise(integrity, dest, symlink=True)
        assert dest.is_symlink() or dest.is_dir()
        assert (dest / "a.tcl").read_text() == "hello\n"

    def test_materialise_copy(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root = _make_tree(tmp_path, {"a.tcl": "hello\n"})
        integrity = cas.store(root, name="pkg", version="1.0")
        dest = tmp_path / "lib" / "pkg-1.0"
        cas.materialise(integrity, dest, symlink=False)
        assert dest.is_dir()
        assert not dest.is_symlink()
        assert (dest / "a.tcl").read_text() == "hello\n"

    def test_tree_path_raises_for_missing(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        with pytest.raises(IntegrityError, match="missing"):
            cas.tree_path("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")

    def test_materialise_overwrites_existing_symlink(self, tmp_path) -> None:
        cas = self._cas(tmp_path)
        root1 = _make_tree(tmp_path / "v1", {"a.tcl": "v1\n"})
        root2 = _make_tree(tmp_path / "v2", {"a.tcl": "v2\n"})
        h1 = cas.store(root1, name="pkg", version="1.0")
        h2 = cas.store(root2, name="pkg", version="2.0")
        dest = tmp_path / "lib" / "pkg"
        cas.materialise(h1, dest)
        assert (dest / "a.tcl").read_text() == "v1\n"
        cas.materialise(h2, dest)
        assert (dest / "a.tcl").read_text() == "v2\n"
