"""Tests for tclpkg.lockfile — canonical JSON serialisation and round-tripping."""

from __future__ import annotations

import json

import pytest

from tooling.tclpkg.errors import TclPkgError
from tooling.tclpkg.lockfile import (
    ExcludeEntry,
    LockedPackage,
    LockFile,
    ReplaceEntry,
    SourceSpec,
    deserialise,
    read_lockfile,
    serialise,
    write_lockfile,
)


def _sample_lockfile() -> LockFile:
    return LockFile(
        name="myapp",
        tcl=">=8.6",
        generated="2026-04-09T12:34:56Z",
        root_hash="sha256-abc",
        packages=[
            LockedPackage(
                name="json",
                version="1.4.0",
                source=SourceSpec(type="tarball", url="https://example.org/json.tar.gz"),
                integrity="sha256-def",
                size=48212,
                requires=["http@2.9.8"],
                provides=["json"],
                license="BSD-3-Clause",
                dev=False,
            ),
            LockedPackage(
                name="http",
                version="2.9.8",
                source=SourceSpec(type="tarball", url="https://example.org/http.tar.gz"),
                integrity="sha256-ghi",
                size=31204,
                requires=[],
                provides=["http"],
                license="TCL",
                dev=False,
            ),
        ],
        replaces=[ReplaceEntry(name="json", from_version="1.3.5", to_version="1.4.0")],
        excludes=[ExcludeEntry(name="http", version="2.9.7")],
    )


class TestSerialise:
    """Tests for canonical JSON output."""

    def test_output_is_valid_json(self) -> None:
        text = serialise(_sample_lockfile())
        data = json.loads(text)
        assert isinstance(data, dict)

    def test_packages_sorted_by_name(self) -> None:
        text = serialise(_sample_lockfile())
        data = json.loads(text)
        names = [p["name"] for p in data["packages"]]
        assert names == sorted(names)

    def test_keys_sorted(self) -> None:
        text = serialise(_sample_lockfile())
        data = json.loads(text)
        assert list(data.keys()) == sorted(data.keys())

    def test_trailing_newline(self) -> None:
        text = serialise(_sample_lockfile())
        assert text.endswith("\n")

    def test_two_space_indent(self) -> None:
        text = serialise(_sample_lockfile())
        # First nested line should be indented with 2 spaces.
        lines = text.split("\n")
        indented = [ln for ln in lines if ln.startswith("  ") and not ln.startswith("    ")]
        assert len(indented) > 0

    def test_deterministic(self) -> None:
        lf = _sample_lockfile()
        a = serialise(lf)
        b = serialise(lf)
        assert a == b


class TestDeserialise:
    """Tests for JSON parsing."""

    def test_round_trip(self) -> None:
        lf = _sample_lockfile()
        text = serialise(lf)
        restored = deserialise(text)
        assert restored.name == lf.name
        assert restored.tcl == lf.tcl
        assert len(restored.packages) == 2
        assert restored.packages[0].name in ("http", "json")

    def test_packages_restored(self) -> None:
        lf = _sample_lockfile()
        text = serialise(lf)
        restored = deserialise(text)
        json_pkg = restored.lookup("json")
        assert json_pkg is not None
        assert json_pkg.version == "1.4.0"
        assert json_pkg.integrity == "sha256-def"
        assert json_pkg.source.type == "tarball"
        assert json_pkg.requires == ["http@2.9.8"]

    def test_replaces_restored(self) -> None:
        lf = _sample_lockfile()
        text = serialise(lf)
        restored = deserialise(text)
        assert len(restored.replaces) == 1
        assert restored.replaces[0].name == "json"

    def test_excludes_restored(self) -> None:
        lf = _sample_lockfile()
        text = serialise(lf)
        restored = deserialise(text)
        assert len(restored.excludes) == 1
        assert restored.excludes[0].version == "2.9.7"

    def test_invalid_json_raises(self) -> None:
        with pytest.raises(TclPkgError, match="invalid lockfile JSON"):
            deserialise("{bad json")

    def test_non_object_raises(self) -> None:
        with pytest.raises(TclPkgError, match="must be a JSON object"):
            deserialise("[1,2,3]")

    def test_newer_schema_version_raises(self) -> None:
        text = json.dumps({"version": 999})
        with pytest.raises(TclPkgError, match="newer than supported"):
            deserialise(text)


class TestLookup:
    """Tests for the ``LockFile.lookup`` helper."""

    def test_finds_existing(self) -> None:
        lf = _sample_lockfile()
        assert lf.lookup("json") is not None

    def test_returns_none_for_missing(self) -> None:
        lf = _sample_lockfile()
        assert lf.lookup("nonexistent") is None


class TestStamp:
    """Tests for the ``LockFile.stamp`` helper."""

    def test_sets_generated(self) -> None:
        lf = LockFile()
        assert lf.generated == ""
        lf.stamp()
        assert lf.generated != ""
        assert "T" in lf.generated


class TestDiskIO:
    """Tests for ``write_lockfile`` and ``read_lockfile``."""

    def test_write_then_read(self, tmp_path) -> None:
        lf = _sample_lockfile()
        path = tmp_path / "tclpkg.lock"
        write_lockfile(lf, path)
        assert path.is_file()
        restored = read_lockfile(path)
        assert restored.name == "myapp"
        assert len(restored.packages) == 2

    def test_write_is_atomic(self, tmp_path) -> None:
        """No .tmp file left behind on success."""
        path = tmp_path / "tclpkg.lock"
        write_lockfile(_sample_lockfile(), path)
        assert not (tmp_path / "tclpkg.lock.tmp").exists()

    def test_read_missing_file_raises(self, tmp_path) -> None:
        with pytest.raises(Exception, match="not found"):
            read_lockfile(tmp_path / "no-such-file.lock")

    def test_bytes_identical_across_writes(self, tmp_path) -> None:
        lf = _sample_lockfile()
        a = tmp_path / "a.lock"
        b = tmp_path / "b.lock"
        write_lockfile(lf, a)
        write_lockfile(lf, b)
        assert a.read_bytes() == b.read_bytes()


class TestSourceSpec:
    """Tests for SourceSpec round-tripping."""

    def test_tarball(self) -> None:
        spec = SourceSpec(type="tarball", url="https://x.org/a.tar.gz")
        d = spec.to_dict()
        restored = SourceSpec.from_dict(d)
        assert restored == spec

    def test_git_with_rev(self) -> None:
        spec = SourceSpec(type="git", url="https://github.com/x/y.git", rev="abc123")
        d = spec.to_dict()
        restored = SourceSpec.from_dict(d)
        assert restored.rev == "abc123"

    def test_path(self) -> None:
        spec = SourceSpec(type="path", url="/local/dir")
        assert spec.to_dict()["type"] == "path"
