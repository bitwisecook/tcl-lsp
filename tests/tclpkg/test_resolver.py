"""Tests for tclpkg.resolver — MVS dependency resolution."""

from __future__ import annotations

import pytest

from tooling.tclpkg.errors import ResolutionError
from tooling.tclpkg.resolver import (
    ExcludeSpec,
    PackageRef,
    ReplaceSpec,
    resolve,
)
from tooling.tclpkg.version import Version


def _ref(name: str, ver: str) -> PackageRef:
    return PackageRef(name=name, version=Version.parse(ver))


def _make_provider(
    graph: dict[str, dict[str, list[tuple[str, str]]]],
):
    """Build a provider from a ``{name: {version: [(dep_name, dep_ver)]}}`` dict."""

    def _provider(name: str, version: Version) -> list[PackageRef]:
        versions = graph.get(name, {})
        deps = versions.get(str(version), [])
        return [_ref(n, v) for n, v in deps]

    return _provider


class TestSimpleResolution:
    """Tests for basic MVS resolution without replace/exclude."""

    def test_single_direct_dep(self) -> None:
        result = resolve([_ref("json", "1.0.0")])
        assert len(result) == 1
        assert result[0].ref.name == "json"
        assert str(result[0].ref.version) == "1.0.0"

    def test_two_independent_deps(self) -> None:
        result = resolve([_ref("json", "1.0.0"), _ref("http", "2.0.0")])
        assert len(result) == 2
        names = {r.ref.name for r in result}
        assert names == {"json", "http"}

    def test_transitive_dep(self) -> None:
        graph = {
            "json": {"1.0.0": [("http", "2.0.0")]},
            "http": {"2.0.0": []},
        }
        result = resolve([_ref("json", "1.0.0")], provider=_make_provider(graph))
        names = {r.ref.name for r in result}
        assert names == {"json", "http"}

    def test_mvs_picks_max_of_minimums(self) -> None:
        """When two packages require different minimums of the same dep,
        MVS picks the higher one."""
        graph = {
            "a": {"1.0.0": [("shared", "1.0.0")]},
            "b": {"1.0.0": [("shared", "2.0.0")]},
            "shared": {"1.0.0": [], "2.0.0": []},
        }
        result = resolve(
            [_ref("a", "1.0.0"), _ref("b", "1.0.0")],
            provider=_make_provider(graph),
        )
        shared = next(r for r in result if r.ref.name == "shared")
        assert str(shared.ref.version) == "2.0.0"

    def test_diamond_dependency(self) -> None:
        """A -> B, A -> C, B -> D@1.0, C -> D@2.0 => D@2.0."""
        graph = {
            "a": {"1.0.0": [("b", "1.0.0"), ("c", "1.0.0")]},
            "b": {"1.0.0": [("d", "1.0.0")]},
            "c": {"1.0.0": [("d", "2.0.0")]},
            "d": {"1.0.0": [], "2.0.0": []},
        }
        result = resolve([_ref("a", "1.0.0")], provider=_make_provider(graph))
        d = next(r for r in result if r.ref.name == "d")
        assert str(d.ref.version) == "2.0.0"

    def test_no_deps(self) -> None:
        result = resolve([])
        assert result == []


class TestDevDeps:
    """Tests for dev-require handling."""

    def test_dev_included_by_default(self) -> None:
        result = resolve(
            [_ref("json", "1.0.0")],
            dev_direct=[_ref("tcltest", "2.5.0")],
        )
        names = {r.ref.name for r in result}
        assert "tcltest" in names

    def test_dev_excluded_when_flag_false(self) -> None:
        result = resolve(
            [_ref("json", "1.0.0")],
            dev_direct=[_ref("tcltest", "2.5.0")],
            include_dev=False,
        )
        names = {r.ref.name for r in result}
        assert "tcltest" not in names

    def test_dev_marked_on_result(self) -> None:
        result = resolve(
            [_ref("json", "1.0.0")],
            dev_direct=[_ref("tcltest", "2.5.0")],
        )
        tcltest = next(r for r in result if r.ref.name == "tcltest")
        assert tcltest.dev is True
        json_pkg = next(r for r in result if r.ref.name == "json")
        assert json_pkg.dev is False


class TestReplace:
    """Tests for the replace directive."""

    def test_replace_overrides_version(self) -> None:
        result = resolve(
            [_ref("json", "1.0.0")],
            replaces=[ReplaceSpec(name="json", version=Version.parse("2.0.0"))],
        )
        json_pkg = next(r for r in result if r.ref.name == "json")
        assert str(json_pkg.ref.version) == "2.0.0"

    def test_replace_affects_transitive(self) -> None:
        graph = {
            "a": {"1.0.0": [("json", "1.0.0")]},
            "json": {"1.0.0": [], "2.0.0": []},
        }
        result = resolve(
            [_ref("a", "1.0.0")],
            replaces=[ReplaceSpec(name="json", version=Version.parse("2.0.0"))],
            provider=_make_provider(graph),
        )
        json_pkg = next(r for r in result if r.ref.name == "json")
        assert str(json_pkg.ref.version) == "2.0.0"


class TestExclude:
    """Tests for the exclude directive."""

    def test_exclude_raises_on_selected_version(self) -> None:
        with pytest.raises(ResolutionError, match="excluded"):
            resolve(
                [_ref("json", "1.0.0")],
                excludes=[ExcludeSpec(name="json", version=Version.parse("1.0.0"))],
            )

    def test_exclude_does_not_affect_other_versions(self) -> None:
        result = resolve(
            [_ref("json", "2.0.0")],
            excludes=[ExcludeSpec(name="json", version=Version.parse("1.0.0"))],
        )
        assert len(result) == 1
        assert str(result[0].ref.version) == "2.0.0"


class TestSorting:
    """Tests for the output ordering."""

    def test_result_sorted_by_name(self) -> None:
        result = resolve([_ref("zlib", "1.0.0"), _ref("a", "1.0.0"), _ref("mid", "1.0.0")])
        names = [r.ref.name for r in result]
        assert names == sorted(names)


class TestErrorHandling:
    """Tests for resolver error conditions."""

    def test_provider_error_wrapped(self) -> None:
        def bad_provider(name: str, version) -> list[PackageRef]:
            raise RuntimeError("network failure")

        with pytest.raises(ResolutionError, match="network failure"):
            resolve(
                [_ref("a", "1.0.0")],
                provider=bad_provider,
            )
