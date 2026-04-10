"""Tests for tclpkg.manifest — tclpkg.tcl safe evaluation and validation."""

from __future__ import annotations

import pytest

from tclpkg.errors import ManifestError
from tclpkg.manifest import ManifestAST, load_manifest, load_manifest_text
from tclpkg.version import Version


class TestMinimalManifest:
    """Tests for manifests with only the required fields."""

    def test_name_and_version(self) -> None:
        ast = load_manifest_text("package myapp\nversion 1.0.0\n")
        assert isinstance(ast, ManifestAST)
        assert ast.name == "myapp"
        assert ast.version == Version.parse("1.0.0")

    def test_default_tcl_constraint(self) -> None:
        ast = load_manifest_text("package myapp\nversion 1.0.0\n")
        assert ast.tcl_constraint == ">=8.6"

    def test_empty_fields_default(self) -> None:
        ast = load_manifest_text("package myapp\nversion 1.0.0\n")
        assert ast.description == ""
        assert ast.license == ""
        assert ast.homepage == ""
        assert ast.entry == ""
        assert ast.requires == []
        assert ast.dev_requires == []
        assert ast.replaces == []
        assert ast.excludes == []


class TestRequiredFields:
    """Tests for missing required fields raising ManifestError."""

    def test_missing_package_directive(self) -> None:
        with pytest.raises(ManifestError, match="required 'package'"):
            load_manifest_text("version 1.0.0\n")

    def test_missing_version_directive(self) -> None:
        with pytest.raises(ManifestError, match="required 'version'"):
            load_manifest_text("package myapp\n")

    def test_empty_manifest(self) -> None:
        with pytest.raises(ManifestError):
            load_manifest_text("")

    def test_duplicate_package_directive(self) -> None:
        with pytest.raises(ManifestError, match="already set"):
            load_manifest_text("package a\npackage b\nversion 1.0.0\n")


class TestRequireDirective:
    """Tests for the ``require`` directive."""

    def test_simple_require(self) -> None:
        ast = load_manifest_text("package myapp\nversion 1.0.0\nrequire json 1.3.5\n")
        assert len(ast.requires) == 1
        req = ast.requires[0]
        assert req.name == "json"
        assert req.minimum == Version.parse("1.3.5")
        assert req.source_url is None
        assert req.dev is False

    def test_require_with_source(self) -> None:
        ast = load_manifest_text(
            "package myapp\nversion 1.0.0\n"
            "require json 1.3.5 -source https://example.org/json.tar.gz\n"
        )
        req = ast.requires[0]
        assert req.source_url == "https://example.org/json.tar.gz"

    def test_dev_require(self) -> None:
        ast = load_manifest_text("package myapp\nversion 1.0.0\ndev-require tcltest 2.5\n")
        assert len(ast.requires) == 0
        assert len(ast.dev_requires) == 1
        assert ast.dev_requires[0].dev is True

    def test_require_invalid_version(self) -> None:
        with pytest.raises(ManifestError, match="invalid version"):
            load_manifest_text("package myapp\nversion 1.0.0\nrequire json garbage\n")

    def test_require_conflict_with_dev_require(self) -> None:
        with pytest.raises(ManifestError, match="both as require and dev-require"):
            load_manifest_text(
                "package myapp\nversion 1.0.0\nrequire json 1.0\ndev-require json 1.0\n"
            )


class TestReplaceExclude:
    """Tests for replace/exclude directives."""

    def test_replace(self) -> None:
        ast = load_manifest_text(
            "package a\nversion 1.0.0\nreplace json 1.4.0 https://example.org/json.tar.gz\n"
        )
        assert len(ast.replaces) == 1
        rep = ast.replaces[0]
        assert rep.name == "json"
        assert rep.version == Version.parse("1.4.0")
        assert rep.source_url == "https://example.org/json.tar.gz"

    def test_exclude(self) -> None:
        ast = load_manifest_text("package a\nversion 1.0.0\nexclude http 2.9.7\n")
        assert len(ast.excludes) == 1
        assert ast.excludes[0].name == "http"
        assert ast.excludes[0].version == Version.parse("2.9.7")


class TestTclConstraint:
    """Tests for the ``tcl`` directive."""

    def test_explicit_constraint(self) -> None:
        ast = load_manifest_text("package a\nversion 1.0.0\ntcl >=9.0\n")
        assert ast.tcl_constraint == ">=9.0"

    def test_shorthand_promoted_to_ge(self) -> None:
        ast = load_manifest_text("package a\nversion 1.0.0\ntcl 8.6\n")
        assert ast.tcl_constraint == ">=8.6"

    def test_invalid_constraint(self) -> None:
        with pytest.raises(ManifestError, match="invalid version constraint"):
            load_manifest_text("package a\nversion 1.0.0\ntcl @@@\n")


class TestMetadata:
    """Tests for optional metadata directives."""

    def test_description_license_homepage(self) -> None:
        ast = load_manifest_text(
            "package a\nversion 1.0.0\n"
            "description {Example}\nlicense MIT\nhomepage https://example.org\n"
        )
        assert ast.description == "Example"
        assert ast.license == "MIT"
        assert ast.homepage == "https://example.org"

    def test_multiple_authors(self) -> None:
        ast = load_manifest_text(
            "package a\nversion 1.0.0\nauthor {Alice <a@x>}\nauthor {Bob <b@x>}\n"
        )
        assert ast.authors == ["Alice <a@x>", "Bob <b@x>"]

    def test_provides_and_entry(self) -> None:
        ast = load_manifest_text(
            "package a\nversion 1.0.0\nprovides a::serve\nprovides a::client\nentry main.tcl\n"
        )
        assert ast.provides == ["a::serve", "a::client"]
        assert ast.entry == "main.tcl"

    def test_license_rejects_garbage(self) -> None:
        with pytest.raises(ManifestError, match="invalid SPDX"):
            load_manifest_text("package a\nversion 1.0.0\nlicense <bad>\n")


class TestSafeMode:
    """Tests that forbidden Tcl commands are refused inside the manifest."""

    @pytest.mark.parametrize(
        "forbidden",
        [
            "exec ls /",
            "open /etc/passwd",
            "source /tmp/x.tcl",
            "puts stderr hi",
            "file delete /tmp/x",
            "socket localhost 80",
        ],
    )
    def test_forbidden_command(self, forbidden: str) -> None:
        body = "package a\nversion 1.0.0\n" + forbidden + "\n"
        with pytest.raises(ManifestError, match="not permitted in safe mode"):
            load_manifest_text(body)


class TestLoadManifestFromDisk:
    """Tests for the ``load_manifest`` helper that reads a file."""

    def test_reads_file(self, tmp_path) -> None:
        manifest_path = tmp_path / "tclpkg.tcl"
        manifest_path.write_text(
            "package myapp\nversion 1.0.0\nrequire json 1.0\n", encoding="utf-8"
        )
        ast = load_manifest(manifest_path)
        assert ast.name == "myapp"
        assert len(ast.requires) == 1
        assert ast.path == str(manifest_path)

    def test_missing_file_raises_with_hint(self, tmp_path) -> None:
        with pytest.raises(ManifestError, match="not found") as exc_info:
            load_manifest(tmp_path / "no-such-file.tcl")
        assert "tcl pkg init" in str(exc_info.value)
