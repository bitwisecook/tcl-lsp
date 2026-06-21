"""Tests for tclpkg.installer — source resolution, fetch, store, materialise.

Mirrors ``rust/tcl-pkg/src/installer.rs``'s unit tests so the Python and Rust
install paths stay in lockstep.
"""

from __future__ import annotations

from pathlib import Path

from tooling.tclpkg.cas import ContentAddressableStore
from tooling.tclpkg.installer import fetch_and_store, materialise, resolve_source


class TestResolveSource:
    """Source-resolution precedence: replace > -source > registry."""

    def test_replace_wins(self) -> None:
        spec = resolve_source(
            "a",
            "1.0",
            replaces={"a": "https://r/a.tar.gz"},
            manifest_sources={"a": "https://s/a.tar.gz"},
            registry=None,
        )
        assert spec is not None
        assert spec.url == "https://r/a.tar.gz"

    def test_manifest_source_used(self) -> None:
        spec = resolve_source(
            "b",
            "2.0",
            replaces={},
            manifest_sources={"b": "https://s/b.git"},
            registry=None,
        )
        assert spec is not None
        assert spec.type == "git"

    def test_local_dir_classified_as_path(self, tmp_path) -> None:
        spec = resolve_source(
            "c",
            "1.0",
            replaces={},
            manifest_sources={"c": str(tmp_path)},
            registry=None,
        )
        assert spec is not None
        assert spec.type == "path"

    def test_no_source_returns_none(self) -> None:
        assert resolve_source("x", "1.0", replaces={}, manifest_sources={}, registry=None) is None


class TestFetchStoreMaterialise:
    """End-to-end fetch → CAS store → materialise for a local package."""

    def _make_pkg(self, root: Path) -> Path:
        pkg = root / "pkgsrc"
        pkg.mkdir(parents=True)
        (pkg / "tclpkg.tcl").write_text(
            "package foo\nversion 1.0.0\nprovides foo::api\nlicense MIT\n",
            encoding="utf-8",
        )
        (pkg / "foo.tcl").write_text("proc foo::hello {} { return hi }\n", encoding="utf-8")
        return pkg

    def test_roundtrip(self, tmp_path) -> None:
        from tooling.tclpkg.lockfile import SourceSpec

        pkg = self._make_pkg(tmp_path)
        cas = ContentAddressableStore(tmp_path / "cache")
        source = SourceSpec(type="path", url=str(pkg))
        result = fetch_and_store(source, "foo", "1.0.0", cas)

        assert result.integrity.startswith("sha256-")
        assert result.size > 0
        assert result.provides == ["foo::api"]
        assert result.license == "MIT"

        lib = tmp_path / "lib"
        dest = materialise(cas, result.integrity, lib, "foo", "1.0.0", symlink=False)
        assert dest.is_dir()
        assert (dest / "foo.tcl").read_text() == "proc foo::hello {} { return hi }\n"
