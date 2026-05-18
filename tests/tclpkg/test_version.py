"""Tests for tclpkg.version — semver ordering with Tcl-friendly parsing."""

from __future__ import annotations

import pytest

from tclpkg.version import Version, VersionError, max_version, parse_version


class TestParse:
    """Tests for ``Version.parse`` and ``parse_version``."""

    def test_basic_three_part(self) -> None:
        v = Version.parse("1.2.3")
        assert (v.major, v.minor, v.patch) == (1, 2, 3)
        assert v.prerelease_raw == ""
        assert v.build == ""

    def test_short_two_part_defaults_patch(self) -> None:
        v = Version.parse("1.2")
        assert (v.major, v.minor, v.patch) == (1, 2, 0)

    def test_single_major_defaults_rest(self) -> None:
        v = Version.parse("7")
        assert (v.major, v.minor, v.patch) == (7, 0, 0)

    def test_v_prefix_allowed(self) -> None:
        assert Version.parse("v1.2.3") == Version.parse("1.2.3")

    def test_tcl_style_alpha(self) -> None:
        v = Version.parse("8.6.3a1")
        assert (v.major, v.minor, v.patch) == (8, 6, 3)
        assert v.prerelease_raw == "-alpha.1"

    def test_tcl_style_beta(self) -> None:
        v = Version.parse("1.0b2")
        assert v.prerelease_raw == "-beta.2"

    def test_tcl_style_rc(self) -> None:
        v = Version.parse("2.0rc1")
        assert v.prerelease_raw == "-rc.1"

    def test_semver_prerelease_passthrough(self) -> None:
        v = Version.parse("1.2.3-rc.4")
        assert v.prerelease_raw == "-rc.4"

    def test_build_metadata(self) -> None:
        v = Version.parse("1.2.3+build.42")
        assert v.build == "build.42"

    def test_parse_empty_string_raises(self) -> None:
        with pytest.raises(VersionError):
            Version.parse("")

    def test_parse_non_string_raises(self) -> None:
        # Intentionally passing a non-str to assert ``parse`` rejects it.
        with pytest.raises(VersionError):
            Version.parse(1.2)  # type: ignore[arg-type]  # ty: ignore[invalid-argument-type]

    def test_parse_garbage_raises(self) -> None:
        with pytest.raises(VersionError):
            Version.parse("not a version")

    def test_parse_version_alias(self) -> None:
        assert parse_version("1.2.3") == Version.parse("1.2.3")


class TestCanonical:
    """Tests for the canonical string form."""

    def test_three_part(self) -> None:
        assert Version.parse("1.2.3").canonical() == "1.2.3"

    def test_two_part_expands(self) -> None:
        assert Version.parse("1.2").canonical() == "1.2.0"

    def test_v_prefix_stripped(self) -> None:
        assert Version.parse("v1.2.3").canonical() == "1.2.3"

    def test_tcl_alpha_canonicalised(self) -> None:
        assert Version.parse("8.6.3a1").canonical() == "8.6.3-alpha.1"

    def test_build_preserved(self) -> None:
        assert Version.parse("1.2.3+foo").canonical() == "1.2.3+foo"

    def test_str_uses_canonical(self) -> None:
        assert str(Version.parse("1.2")) == "1.2.0"


class TestOrdering:
    """Tests for version comparison."""

    def test_major_precedence(self) -> None:
        assert Version.parse("2.0.0") > Version.parse("1.99.99")

    def test_minor_precedence(self) -> None:
        assert Version.parse("1.3.0") > Version.parse("1.2.99")

    def test_patch_precedence(self) -> None:
        assert Version.parse("1.2.3") > Version.parse("1.2.2")

    def test_prerelease_sorts_before_release(self) -> None:
        assert Version.parse("1.2.3-rc.1") < Version.parse("1.2.3")

    def test_tcl_alpha_sorts_before_beta(self) -> None:
        assert Version.parse("8.6.3a1") < Version.parse("8.6.3b1")

    def test_tcl_beta_sorts_before_rc(self) -> None:
        assert Version.parse("8.6.3b2") < Version.parse("8.6.3rc1")

    def test_tcl_rc_sorts_before_release(self) -> None:
        assert Version.parse("8.6.3rc1") < Version.parse("8.6.3")

    def test_prerelease_numeric_order(self) -> None:
        assert Version.parse("1.0.0-rc.2") > Version.parse("1.0.0-rc.1")

    def test_build_metadata_not_compared(self) -> None:
        # Semver says build metadata does not affect ordering.
        assert Version.parse("1.2.3+a") == Version.parse("1.2.3+b")

    def test_sorting_mixed_forms(self) -> None:
        versions = [
            Version.parse("1.0.0"),
            Version.parse("1.2"),
            Version.parse("1.2.3a1"),
            Version.parse("v1.2.3"),
            Version.parse("0.9.9"),
        ]
        ordered = sorted(versions)
        assert [str(v) for v in ordered] == [
            "0.9.9",
            "1.0.0",
            "1.2.0",
            "1.2.3-alpha.1",
            "1.2.3",
        ]


class TestHashable:
    """Versions are hashable and usable in sets/dicts."""

    def test_same_parse_same_hash(self) -> None:
        assert hash(Version.parse("1.2.3")) == hash(Version.parse("1.2.3"))

    def test_canonical_equals_raw(self) -> None:
        # v-prefix and short form canonicalise to the same key.
        assert Version.parse("v1.2") == Version.parse("1.2.0")

    def test_used_in_set(self) -> None:
        s = {Version.parse("1.0.0"), Version.parse("v1.0.0"), Version.parse("1.0")}
        assert len(s) == 1


class TestMaxVersion:
    """Tests for ``max_version`` helper."""

    def test_picks_maximum(self) -> None:
        chosen = max_version(
            [Version.parse("1.0.0"), Version.parse("1.2.3"), Version.parse("1.1.9")]
        )
        assert str(chosen) == "1.2.3"

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError):
            max_version([])
