"""Comprehensive tests for the package loading system.

Covers the full matrix of package sources (stdlib, tcllib, Tk) and the
iRules equivalent cross-file proc system.  Exercises split-file packages,
version matching, pkgIndex.tcl parsing edge cases, analyser extraction,
registry filtering, and workspace-index package integration.
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, PackageRequire, Range
from core.commands.registry import REGISTRY
from core.commands.registry.command_registry import CommandRegistry
from core.commands.registry.stdlib import stdlib_command_specs
from core.commands.registry.tcllib import tcllib_command_specs
from core.commands.registry.tk import tk_command_specs
from core.packages import PackageResolver
from core.tk.detection import has_tk_require
from lsp.features.package_suggestions import rank_package_suggestions
from lsp.workspace.workspace_index import EntrySource, WorkspaceIndex

# Helper utilities


def _make_analysis(package_names: list[str]) -> AnalysisResult:
    """Build a minimal AnalysisResult with the given package requires."""
    result = AnalysisResult()
    zero = Range.zero()
    result.package_requires = [
        PackageRequire(name=n, version=None, range=zero) for n in package_names
    ]
    return result


def _make_pkg_dir(
    tmpdir: str, name: str, version: str, filenames: list[str], *, ifneeded_style: str = "join"
) -> str:
    """Create a package directory with a pkgIndex.tcl and dummy source files.

    *ifneeded_style* controls how the source is referenced:
    - ``"join"``  — ``source [file join $dir <file>]``
    - ``"slash"`` — ``source $dir/<file>``
    - ``"load"``  — ``load [file join $dir <file>.so]`` (forces fallback)
    - ``"multi"`` — multiple source statements in a single ifneeded script
    """
    pkg_dir = os.path.join(tmpdir, f"{name}{version}")
    os.makedirs(pkg_dir, exist_ok=True)

    for fname in filenames:
        Path(os.path.join(pkg_dir, fname)).write_text(
            f"# Dummy source for {name}\nproc {name}::init {{}} {{}}\n"
        )

    if ifneeded_style == "join":
        if len(filenames) == 1:
            script = f"[list source [file join $dir {filenames[0]}]]"
        else:
            parts = "; ".join(f"source [file join $dir {f}]" for f in filenames)
            script = f'"{parts}"'
    elif ifneeded_style == "slash":
        if len(filenames) == 1:
            script = f'"source $dir/{filenames[0]}"'
        else:
            parts = "; ".join(f"source $dir/{f}" for f in filenames)
            script = f'"{parts}"'
    elif ifneeded_style == "load":
        script = f"{{load [file join $dir {name}.so]}}"
    elif ifneeded_style == "multi":
        parts = "; ".join(f"source [file join $dir {f}]" for f in filenames)
        script = f'"{parts}"'
    else:
        msg = f"Unknown ifneeded_style: {ifneeded_style}"
        raise ValueError(msg)

    Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
        f"package ifneeded {name} {version} {script}\n"
    )
    return pkg_dir


# Section 1: PackageResolver — pkgIndex.tcl parsing and file resolution


class TestPackageResolverStdlibPatterns:
    """Patterns found in the Tcl standard library's pkgIndex.tcl files."""

    def test_file_join_pattern(self):
        """Standard ``source [file join $dir <file>]`` pattern."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "http", "2.9", ["http.tcl"], ifneeded_style="join")
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("http")
            assert len(files) == 1
            assert files[0].endswith("http.tcl")

    def test_dollar_dir_slash_pattern(self):
        """Alternative ``source $dir/<file>`` pattern."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "msgcat", "1.6", ["msgcat.tcl"], ifneeded_style="slash")
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("msgcat")
            assert len(files) == 1
            assert files[0].endswith("msgcat.tcl")

    def test_load_falls_back_to_tcl_files(self):
        """When pkgIndex uses ``load``, fall back to .tcl files in the directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "tls", "1.7", ["tls.tcl"], ifneeded_style="load")
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("tls")
            assert len(files) == 1
            assert files[0].endswith("tls.tcl")

    def test_multiple_versions(self):
        """Multiple versions of the same package are tracked."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "http", "2.8", ["http.tcl"])
            _make_pkg_dir(tmpdir, "http", "2.9", ["http.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            resolver.scan_packages()

            assert "http" in resolver.all_package_names()
            # Without version constraint, return first found
            files = resolver.resolve("http")
            assert len(files) == 1

            # Exact version match
            files_28 = resolver.resolve("http", "2.8")
            assert len(files_28) == 1
            files_29 = resolver.resolve("http", "2.9")
            assert len(files_29) == 1
            # The file paths come from different directories
            assert files_28[0] != files_29[0]

    def test_prefix_version_match(self):
        """Version prefix matching (e.g. ``2`` matches ``2.9``)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "http", "2.9.4", ["http.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("http", "2")
            assert len(files) == 1
            files = resolver.resolve("http", "2.9")
            assert len(files) == 1

    def test_version_mismatch_returns_empty(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "http", "2.9", ["http.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            assert resolver.resolve("http", "3.0") == []

    def test_alpha_beta_version_tags(self):
        """Packages with alpha/beta version tags."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "expat1.0a2")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "expat.tcl")).write_text("proc expat {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded expat 1.0a2 [list source [file join $dir expat.tcl]]"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("expat", "1.0a2")
            assert len(files) == 1


class TestPackageResolverSplitFiles:
    """Packages whose implementation is split across multiple files."""

    def test_multi_source_join(self):
        """Package with multiple ``source [file join $dir ...]`` in one script."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(
                tmpdir,
                "http",
                "2.9",
                ["http.tcl", "cookiejar.tcl"],
                ifneeded_style="multi",
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("http")
            assert len(files) == 2
            basenames = {os.path.basename(f) for f in files}
            assert basenames == {"http.tcl", "cookiejar.tcl"}

    def test_multi_source_slash_partial_resolution(self):
        """The ``$dir/`` pattern with semicolons captures trailing ``;``
        in filenames, causing ``os.path.isfile`` to fail for all but the
        last entry.  Only the last file (no trailing ``;``) resolves;
        the others are silently ignored.  Because at least one file
        matches, the directory-listing fallback does *not* trigger.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "struct2.1")
            os.makedirs(pkg_dir)
            for name in ("list.tcl", "set.tcl", "tree.tcl"):
                Path(os.path.join(pkg_dir, name)).write_text(f"# {name}\n")
            # Semicolons cause the $dir/ regex to capture "list.tcl;" etc.
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded struct 2.1 "
                '"source $dir/list.tcl; source $dir/set.tcl; source $dir/tree.tcl"'
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("struct")
            # Only the last file (no trailing ;) passes os.path.isfile;
            # the others are ignored and fallback does not trigger.
            assert len(files) >= 1
            basenames = {os.path.basename(f) for f in files}
            assert "tree.tcl" in basenames

    def test_single_source_slash(self):
        """Single ``source $dir/<file>`` with no semicolons works cleanly."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "simple1.0")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "simple.tcl")).write_text("proc simple {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                'package ifneeded simple 1.0 "source $dir/simple.tcl"'
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("simple")
            assert len(files) == 1
            assert files[0].endswith("simple.tcl")

    def test_fallback_finds_all_tcl_files(self):
        """When no source pattern matches, all .tcl files in the dir are returned."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(
                tmpdir,
                "complexlib",
                "1.0",
                ["part_a.tcl", "part_b.tcl", "part_c.tcl"],
                ifneeded_style="load",
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("complexlib")
            # Fallback should find all 3 .tcl files
            assert len(files) == 3
            basenames = {os.path.basename(f) for f in files}
            assert "part_a.tcl" in basenames
            assert "part_b.tcl" in basenames
            assert "part_c.tcl" in basenames

    def test_pkgindex_excluded_from_fallback(self):
        """Fallback scan must NOT include pkgIndex.tcl itself."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "foo", "1.0", ["foo.tcl"], ifneeded_style="load")
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("foo")
            for f in files:
                assert not f.endswith("pkgIndex.tcl")


class TestPackageResolverEdgeCases:
    """Edge cases and error handling."""

    def test_nonexistent_search_path(self):
        resolver = PackageResolver()
        resolver.configure(search_paths=["/nonexistent/path/xyz"])
        resolver.scan_packages()
        assert resolver.all_package_names() == []

    def test_empty_search_paths(self):
        resolver = PackageResolver()
        resolver.configure(search_paths=[])
        resolver.scan_packages()
        assert resolver.all_package_names() == []

    def test_lazy_scan_triggers_on_resolve(self):
        """Scanning happens automatically on first resolve if not yet scanned."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "lazy", "1.0", ["lazy.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            # Don't call scan_packages explicitly
            files = resolver.resolve("lazy")
            assert len(files) == 1

    def test_lazy_scan_triggers_on_all_package_names(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(tmpdir, "lazy", "1.0", ["lazy.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            names = resolver.all_package_names()
            assert "lazy" in names

    def test_deeply_nested_packages(self):
        """Packages buried multiple levels deep are found."""
        with tempfile.TemporaryDirectory() as tmpdir:
            deep_dir = os.path.join(tmpdir, "lib", "vendor", "mypkg1.0")
            os.makedirs(deep_dir)
            Path(os.path.join(deep_dir, "mypkg.tcl")).write_text("proc mypkg {} {}")
            Path(os.path.join(deep_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("mypkg")
            assert len(files) == 1

    def test_multiple_search_paths(self):
        """Packages from multiple search paths are merged."""
        with tempfile.TemporaryDirectory() as d1, tempfile.TemporaryDirectory() as d2:
            _make_pkg_dir(d1, "alpha", "1.0", ["alpha.tcl"])
            _make_pkg_dir(d2, "beta", "2.0", ["beta.tcl"])
            resolver = PackageResolver()
            resolver.configure(search_paths=[d1, d2])
            resolver.scan_packages()
            names = resolver.all_package_names()
            assert "alpha" in names
            assert "beta" in names

    def test_reconfigure_clears_state(self):
        """Calling configure resets the scanned state."""
        with tempfile.TemporaryDirectory() as d1, tempfile.TemporaryDirectory() as d2:
            _make_pkg_dir(d1, "alpha", "1.0", ["alpha.tcl"])
            _make_pkg_dir(d2, "beta", "2.0", ["beta.tcl"])

            resolver = PackageResolver()
            resolver.configure(search_paths=[d1])
            resolver.scan_packages()
            assert "alpha" in resolver.all_package_names()

            resolver.configure(search_paths=[d2])
            resolver.scan_packages()
            names = resolver.all_package_names()
            assert "alpha" not in names
            assert "beta" in names

    def test_missing_source_file_not_returned(self):
        """If pkgIndex references a file that doesn't exist on disk, skip it."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "ghost1.0")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded ghost 1.0 [list source [file join $dir ghost.tcl]]"
            )
            # ghost.tcl does NOT exist
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("ghost")
            # No source files found, and no .tcl fallback either
            assert files == []

    def test_quoted_filename_in_source(self):
        """Filenames with quotes in the pkgIndex script are handled."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "quoted1.0")
            os.makedirs(pkg_dir)
            impl = os.path.join(pkg_dir, "impl.tcl")
            Path(impl).write_text("proc quoted {} {}")
            # Some pkgIndex.tcl files quote the filename
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                'package ifneeded quoted 1.0 [list source [file join $dir "impl.tcl"]]'
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("quoted")
            assert len(files) == 1

    def test_multiple_packages_same_dir(self):
        """Multiple package ifneeded lines in a single pkgIndex.tcl."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "multi")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "alpha.tcl")).write_text("proc alpha {} {}")
            Path(os.path.join(pkg_dir, "beta.tcl")).write_text("proc beta {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded alpha 1.0 [list source [file join $dir alpha.tcl]]\n"
                "package ifneeded beta 2.0 [list source [file join $dir beta.tcl]]\n"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            resolver.scan_packages()
            assert "alpha" in resolver.all_package_names()
            assert "beta" in resolver.all_package_names()
            assert resolver.resolve("alpha")[0].endswith("alpha.tcl")
            assert resolver.resolve("beta")[0].endswith("beta.tcl")


class TestPackageResolverTcllibPatterns:
    """Patterns common in tcllib pkgIndex.tcl files."""

    def test_namespaced_package_name(self):
        """Package names with ``::`` separators (e.g. ``struct::list``)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "struct_list2.1")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "list.tcl")).write_text("namespace eval struct::list {}\n")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded struct::list 2.1 [list source [file join $dir list.tcl]]\n"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("struct::list")
            assert len(files) == 1

    def test_multiple_related_packages_in_one_dir(self):
        """tcllib often has multiple related packages in the same directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "struct")
            os.makedirs(pkg_dir)
            for name in ("list", "set", "tree", "graph"):
                Path(os.path.join(pkg_dir, f"{name}.tcl")).write_text(
                    f"namespace eval struct::{name} {{}}\n"
                )
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded struct::list 2.1 [list source [file join $dir list.tcl]]\n"
                "package ifneeded struct::set 2.2 [list source [file join $dir set.tcl]]\n"
                "package ifneeded struct::tree 2.1 [list source [file join $dir tree.tcl]]\n"
                "package ifneeded struct::graph 2.0 [list source [file join $dir graph.tcl]]\n"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            resolver.scan_packages()
            names = resolver.all_package_names()
            assert "struct::list" in names
            assert "struct::set" in names
            assert "struct::tree" in names
            assert "struct::graph" in names

    def test_tilde_expansion_in_search_path(self):
        """Home-directory tilde in search path is expanded."""
        resolver = PackageResolver()
        # This shouldn't crash even if the expanded dir doesn't exist
        resolver.configure(search_paths=["~/nonexistent_tcllib_test_path"])
        resolver.scan_packages()
        assert resolver.all_package_names() == []


# Section 2: Analyser — package require statement extraction


class TestAnalyserPackageRequire:
    """Verify the analyser correctly extracts package require statements."""

    def test_simple_package_require(self):
        result = analyse("package require http")
        names = result.active_package_names()
        assert "http" in names

    def test_package_require_with_version(self):
        result = analyse("package require http 2.9")
        names = result.active_package_names()
        assert "http" in names
        assert result.package_requires[0].version == "2.9"

    def test_package_require_exact(self):
        result = analyse("package require -exact http 2.9")
        names = result.active_package_names()
        assert "http" in names
        assert result.package_requires[0].version == "2.9"

    def test_multiple_package_requires(self):
        source = "package require http\npackage require json\npackage require Tk\n"
        result = analyse(source)
        names = result.active_package_names()
        assert names == frozenset({"http", "json", "Tk"})

    def test_package_require_in_proc(self):
        """Package require inside a proc body is still tracked."""
        source = "proc init {} {\n    package require http\n}\n"
        result = analyse(source)
        names = result.active_package_names()
        assert "http" in names

    def test_package_require_in_if(self):
        source = "if {1} {\n    package require http\n}\n"
        result = analyse(source)
        names = result.active_package_names()
        assert "http" in names

    def test_no_package_require(self):
        result = analyse("set x 42\nputs $x")
        assert len(result.package_requires) == 0
        assert result.active_package_names() == frozenset()

    def test_package_provide_not_tracked_as_require(self):
        result = analyse("package provide mylib 1.0")
        assert result.active_package_names() == frozenset()

    def test_package_ifneeded_not_tracked_as_require(self):
        result = analyse("package ifneeded mylib 1.0 {set x 1}")
        assert result.active_package_names() == frozenset()

    def test_stdlib_packages_tracked(self):
        source = (
            "package require http\n"
            "package require msgcat\n"
            "package require tcltest\n"
            "package require safe\n"
            "package require platform\n"
        )
        result = analyse(source)
        names = result.active_package_names()
        assert names == frozenset({"http", "msgcat", "tcltest", "safe", "platform"})

    def test_tcllib_packages_tracked(self):
        source = (
            "package require json\n"
            "package require base64\n"
            "package require csv\n"
            "package require uri\n"
        )
        result = analyse(source)
        names = result.active_package_names()
        assert names == frozenset({"json", "base64", "csv", "uri"})

    def test_tk_package_tracked(self):
        result = analyse("package require Tk")
        names = result.active_package_names()
        assert "Tk" in names

    def test_tk_with_version_tracked(self):
        result = analyse("package require Tk 8.6")
        names = result.active_package_names()
        assert "Tk" in names
        assert result.package_requires[0].version == "8.6"


# Section 3: Stdlib package loading — registry and filtering


class TestStdlibPackageSpecs:
    """Verify stdlib command specs have correct package requirements."""

    def test_all_http_commands_require_http(self):
        specs = [
            s
            for s in stdlib_command_specs()
            if s.name.startswith("http::") and "cookiejar" not in s.name.lower()
        ]
        assert len(specs) > 0
        for spec in specs:
            assert spec.required_package == "http", f"{spec.name}: wrong package"

    def test_all_msgcat_commands_require_msgcat(self):
        specs = [s for s in stdlib_command_specs() if s.name.startswith("msgcat::")]
        assert len(specs) > 0
        for spec in specs:
            assert spec.required_package == "msgcat", f"{spec.name}: wrong package"

    def test_cookiejar_commands_require_cookiejar(self):
        specs = [s for s in stdlib_command_specs() if "cookiejar" in s.name.lower()]
        assert len(specs) > 0
        for spec in specs:
            assert spec.required_package == "cookiejar", f"{spec.name}: wrong package"

    def test_non_package_commands_have_no_requirement(self):
        """Commands like history, pkg_mkIndex don't require a package."""
        non_pkg = [s for s in stdlib_command_specs() if s.required_package is None]
        assert len(non_pkg) > 0
        names = {s.name for s in non_pkg}
        # These should be unconditionally available
        assert "history" in names or "pkg_mkIndex" in names

    def test_stdlib_packages_listed(self):
        """Verify that known stdlib packages are present in the registry."""
        all_specs = stdlib_command_specs()
        packages = {s.required_package for s in all_specs if s.required_package}
        # Common stdlib packages
        for pkg in ("http", "msgcat", "tcltest", "safe", "platform"):
            assert pkg in packages, f"Missing stdlib package: {pkg}"


class TestStdlibRegistryFiltering:
    """Registry filtering based on active stdlib packages."""

    def test_http_visible_with_package(self):
        spec = REGISTRY.get("http::geturl", active_packages=frozenset({"http"}))
        assert spec is not None

    def test_http_hidden_without_package(self):
        spec = REGISTRY.get("http::geturl", active_packages=frozenset())
        assert spec is None

    def test_msgcat_visible_with_package(self):
        spec = REGISTRY.get("msgcat::mc", active_packages=frozenset({"msgcat"}))
        assert spec is not None

    def test_msgcat_hidden_without_package(self):
        spec = REGISTRY.get("msgcat::mc", active_packages=frozenset())
        assert spec is None

    def test_multiple_stdlib_packages_together(self):
        """Multiple stdlib packages can be active simultaneously."""
        pkgs = frozenset({"http", "msgcat", "tcltest"})
        assert REGISTRY.get("http::geturl", active_packages=pkgs) is not None
        assert REGISTRY.get("msgcat::mc", active_packages=pkgs) is not None
        assert REGISTRY.get("tcltest::test", active_packages=pkgs) is not None

    def test_builtins_always_available(self):
        """Core commands are available regardless of active packages."""
        for cmd in ("set", "puts", "proc", "if", "foreach", "string"):
            spec = REGISTRY.get(cmd, active_packages=frozenset())
            assert spec is not None, f"{cmd} should be unconditionally available"

    def test_no_filter_shows_everything(self):
        """active_packages=None means no filtering — show all commands."""
        spec = REGISTRY.get("http::geturl", active_packages=None)
        assert spec is not None

    def test_commands_for_packages_with_stdlib(self):
        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(frozenset({"http"})))
        assert "http::geturl" in names
        assert "http::config" in names
        # Core builtins still present
        assert "set" in names
        # Other packages not included
        assert "msgcat::mc" not in names


# Section 4: Tcllib package loading — registry and filtering


class TestTcllibPackageSpecs:
    """Verify tcllib command specs are well-formed."""

    def test_all_tcllib_specs_have_package(self):
        for spec in tcllib_command_specs():
            assert spec.tcllib_package is not None, f"{spec.name} has no tcllib_package"

    def test_all_tcllib_specs_have_hover(self):
        for spec in tcllib_command_specs():
            assert spec.hover is not None, f"{spec.name} has no hover"
            assert spec.hover.summary, f"{spec.name} has empty hover summary"

    def test_known_tcllib_packages_not_empty(self):
        packages = REGISTRY.known_tcllib_packages()
        assert len(packages) > 10

    def test_all_tcllib_command_names_not_empty(self):
        names = REGISTRY.all_tcllib_command_names()
        assert len(names) > 30


class TestTcllibPackageFiltering:
    """Tcllib commands filtered by active packages."""

    def test_json_commands_with_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"json"}))
        assert "json::json2dict" in cmds
        assert "json::dict2json" in cmds

    def test_base64_commands_with_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"base64"}))
        assert "base64::encode" in cmds
        assert "base64::decode" in cmds

    def test_csv_commands_with_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"csv"}))
        assert "csv::split" in cmds
        assert "csv::join" in cmds

    def test_no_commands_without_package(self):
        cmds = REGISTRY.tcllib_command_names(frozenset())
        assert cmds == ()

    def test_unknown_package_returns_empty(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"nonexistent_pkg"}))
        assert cmds == ()

    def test_multiple_tcllib_packages(self):
        cmds = REGISTRY.tcllib_command_names(frozenset({"json", "base64", "csv"}))
        assert "json::json2dict" in cmds
        assert "base64::encode" in cmds
        assert "csv::split" in cmds

    def test_tcllib_package_for_lookup(self):
        assert REGISTRY.tcllib_package_for("json::json2dict") == "json"
        assert REGISTRY.tcllib_package_for("base64::encode") == "base64"
        assert REGISTRY.tcllib_package_for("set") is None

    def test_is_tcllib_command(self):
        assert REGISTRY.is_tcllib_command("json::json2dict")
        assert REGISTRY.is_tcllib_command("csv::split")
        assert not REGISTRY.is_tcllib_command("set")
        assert not REGISTRY.is_tcllib_command("http::geturl")  # stdlib, not tcllib

    def test_package_to_command_consistency(self):
        """Every command maps back to its declaring package."""
        for pkg in REGISTRY.known_tcllib_packages():
            cmds = REGISTRY.tcllib_command_names(frozenset({pkg}))
            for cmd in cmds:
                assert REGISTRY.tcllib_package_for(cmd) == pkg


class TestTcllibCrossConcerns:
    """Tcllib integration with analyser and registry across dialects."""

    def test_tcllib_commands_present_across_dialects(self):
        """Tcllib commands should be available in all Tcl dialects."""
        from core.commands.registry.runtime import SIGNATURES, configure_signatures

        for dialect in ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"):
            configure_signatures(dialect=dialect)
            assert "json::json2dict" in SIGNATURES, f"missing in {dialect}"

    def test_struct_family_packages(self):
        """The struct:: family (list, set, tree, etc.) are separate packages."""
        packages = REGISTRY.known_tcllib_packages()
        struct_pkgs = [p for p in packages if p.startswith("struct")]
        assert len(struct_pkgs) >= 1  # at least struct::list is expected

    def test_hash_packages(self):
        """Hashing packages like sha1, md5 are in tcllib."""
        for pkg in ("sha1", "md5"):
            if pkg in REGISTRY.known_tcllib_packages():
                cmds = REGISTRY.tcllib_command_names(frozenset({pkg}))
                assert len(cmds) >= 1, f"{pkg} has no commands"


# Section 5: Tk package loading — registry and filtering


class TestTkPackageSpecs:
    """Verify Tk command specs and package requirements."""

    def test_all_tk_specs_require_tk(self):
        for spec in tk_command_specs():
            assert spec.required_package == "Tk", f"{spec.name} missing required_package"

    def test_key_widget_commands_present(self):
        names = {s.name for s in tk_command_specs()}
        expected = {"button", "label", "entry", "text", "frame", "canvas", "listbox"}
        for cmd in expected:
            assert cmd in names, f"Missing Tk widget: {cmd}"

    def test_key_geometry_managers_present(self):
        names = {s.name for s in tk_command_specs()}
        for cmd in ("pack", "grid", "place"):
            assert cmd in names, f"Missing geometry manager: {cmd}"

    def test_ttk_commands_present(self):
        names = {s.name for s in tk_command_specs()}
        assert "ttk::button" in names
        assert "ttk::treeview" in names
        assert "ttk::style" in names

    def test_wm_winfo_present(self):
        names = {s.name for s in tk_command_specs()}
        assert "wm" in names
        assert "winfo" in names

    def test_no_duplicate_tk_names(self):
        names = [s.name for s in tk_command_specs()]
        assert len(names) == len(set(names))


class TestTkRegistryFiltering:
    """Tk commands filtered by active packages."""

    def test_tk_visible_with_package(self):
        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(frozenset({"Tk"})))
        assert "button" in names
        assert "ttk::button" in names
        assert "pack" in names
        assert "wm" in names

    def test_tk_hidden_without_package(self):
        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(frozenset()))
        assert "button" not in names
        assert "ttk::button" not in names

    def test_tk_hidden_with_wrong_package(self):
        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(frozenset({"http"})))
        assert "button" not in names

    def test_tk_with_multiple_packages(self):
        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(frozenset({"Tk", "http"})))
        assert "button" in names
        assert "http::geturl" in names
        assert "set" in names


class TestTkDetection:
    """Tk detection from analysis results."""

    def test_has_tk_require_with_tk(self):
        assert has_tk_require(_make_analysis(["Tk"]))

    def test_has_tk_require_without_tk(self):
        assert not has_tk_require(_make_analysis(["http"]))

    def test_has_tk_require_empty(self):
        assert not has_tk_require(_make_analysis([]))

    def test_has_tk_require_case_sensitive(self):
        assert not has_tk_require(_make_analysis(["tk"]))

    def test_has_tk_require_among_others(self):
        assert has_tk_require(_make_analysis(["http", "Tk", "json"]))

    def test_analyser_detects_tk_require(self):
        """Full pipeline: analyser -> has_tk_require."""
        result = analyse("package require Tk\nbutton .b -text hello")
        assert has_tk_require(result)

    def test_analyser_no_tk_require(self):
        result = analyse("package require http\nhttp::geturl http://x")
        assert not has_tk_require(result)


# Section 6: iRules cross-file "package" equivalent


class TestIrulesGlobalProcs:
    """iRules share procs globally across files instead of using packages."""

    def test_irules_procs_globally_visible(self):
        idx = WorkspaceIndex()
        result = analyse("proc helper {x} { return $x }")
        idx.update_irules_globals("file:///rule1.irul", result.all_procs)
        entries = idx.find_proc("helper")
        assert len(entries) == 1
        assert entries[0].uri == "file:///rule1.irul"

    def test_irules_procs_from_multiple_files(self):
        idx = WorkspaceIndex()
        r1 = analyse("proc helper_a {} { return a }")
        r2 = analyse("proc helper_b {} { return b }")
        idx.update_irules_globals("file:///rule1.irul", r1.all_procs)
        idx.update_irules_globals("file:///rule2.irul", r2.all_procs)
        assert len(idx.find_proc("helper_a")) == 1
        assert len(idx.find_proc("helper_b")) == 1

    def test_irules_procs_in_all_proc_names(self):
        idx = WorkspaceIndex()
        result = analyse("proc my_utility {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        names = idx.all_proc_names()
        assert "::my_utility" in names

    def test_irules_update_replaces_old_procs(self):
        idx = WorkspaceIndex()
        r1 = analyse("proc old_proc {} {}")
        idx.update_irules_globals("file:///rule.irul", r1.all_procs)
        assert len(idx.find_proc("old_proc")) == 1

        r2 = analyse("proc new_proc {} {}")
        idx.update_irules_globals("file:///rule.irul", r2.all_procs)
        assert len(idx.find_proc("old_proc")) == 0
        assert len(idx.find_proc("new_proc")) == 1

    def test_irules_cleared_on_remove_background(self):
        idx = WorkspaceIndex()
        result = analyse("proc helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        assert len(idx.find_proc("helper")) == 1
        idx.remove_background_entries()
        assert len(idx.find_proc("helper")) == 0

    def test_irules_and_regular_index_coexist(self):
        """iRules globals and regular index entries both appear."""
        idx = WorkspaceIndex()
        # Regular Tcl file
        idx.update("file:///lib.tcl", analyse("proc lib_func {} {}"), EntrySource.OPEN)
        # iRules file
        result = analyse("proc irule_func {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        assert len(idx.find_proc("lib_func")) == 1
        assert len(idx.find_proc("irule_func")) == 1


class TestIrulesRuleInitVars:
    """iRules RULE_INIT block exports cross-file variables."""

    def test_rule_init_vars_api_exists(self):
        """Verify the workspace index RULE_INIT variable API works."""
        idx = WorkspaceIndex()
        # Just verify the API contract: empty index returns empty set
        names = idx.all_rule_init_var_names()
        assert len(names) == 0


# Section 7: Workspace index — package source tracking


class TestWorkspaceIndexPackageSource:
    """Track how files enter the index (open, background, package)."""

    def test_package_source_kind(self):
        idx = WorkspaceIndex()
        result = analyse("proc pkg_func {} {}")
        idx.update("file:///pkg.tcl", result, EntrySource.PACKAGE)
        assert idx.is_background("file:///pkg.tcl")

    def test_open_overrides_package(self):
        idx = WorkspaceIndex()
        result = analyse("proc pkg_func {} {}")
        idx.update("file:///pkg.tcl", result, EntrySource.PACKAGE)
        assert idx.is_background("file:///pkg.tcl")
        idx.update("file:///pkg.tcl", result, EntrySource.OPEN)
        assert not idx.is_background("file:///pkg.tcl")

    def test_remove_background_clears_package(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc a {} {}"), EntrySource.OPEN)
        idx.update("file:///b.tcl", analyse("proc b {} {}"), EntrySource.PACKAGE)
        idx.remove_background_entries()
        assert len(idx.find_proc("a")) == 1  # OPEN survives
        assert len(idx.find_proc("b")) == 0  # PACKAGE removed

    def test_package_procs_findable(self):
        """Procs from package-sourced files are findable in the index."""
        idx = WorkspaceIndex()
        result = analyse("namespace eval http {\n    proc geturl {url} { return $url }\n}\n")
        idx.update("file:///http.tcl", result, EntrySource.PACKAGE)
        entries = idx.find_proc("http::geturl")
        assert len(entries) == 1
        assert entries[0].source_kind == EntrySource.PACKAGE

    def test_multi_file_package_index(self):
        """Multiple files from the same package are all indexed."""
        idx = WorkspaceIndex()
        r1 = analyse("proc http::geturl {url} { return $url }")
        r2 = analyse("proc http::config {args} { return $args }")
        idx.update("file:///http_geturl.tcl", r1, EntrySource.PACKAGE)
        idx.update("file:///http_config.tcl", r2, EntrySource.PACKAGE)
        assert len(idx.find_proc("http::geturl")) == 1
        assert len(idx.find_proc("http::config")) == 1
        names = idx.all_proc_names()
        assert "::http::geturl" in names
        assert "::http::config" in names


# Section 8: Package suggestions


class TestPackageSuggestions:
    """Package suggestion ranking for unresolved symbols."""

    def test_exact_match_ranked_first(self):
        packages = ["json", "json-tools", "ajson", "xjsonx"]
        result = rank_package_suggestions("json::parse", packages, 10)
        assert result[0] == "json"

    def test_prefix_match_ranked_second(self):
        packages = ["ajson", "json-tools", "json"]
        result = rank_package_suggestions("json::parse", packages, 10)
        assert result[0] == "json"
        assert "json-tools" in result

    def test_limit_respected(self):
        packages = ["json", "json-tools", "ajson", "xjsonx"]
        result = rank_package_suggestions("json::parse", packages, 2)
        assert len(result) <= 2

    def test_short_prefix_returns_empty(self):
        packages = ["json", "j-lib"]
        result = rank_package_suggestions("j::x", packages, 10)
        assert result == []

    def test_empty_symbol_returns_empty(self):
        result = rank_package_suggestions("", ["json"], 10)
        assert result == []

    def test_no_match_returns_empty(self):
        result = rank_package_suggestions("zzzz::func", ["json", "http"], 10)
        assert result == []


# Section 9: Cross-cutting integration — full pipeline tests


class TestFullPipelineStdlib:
    """End-to-end: source → analyser → active packages → registry filtering."""

    def test_http_pipeline(self):
        source = "package require http\nhttp::geturl http://example.com"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "http" in pkgs
        spec = REGISTRY.get("http::geturl", active_packages=pkgs)
        assert spec is not None

    def test_http_pipeline_without_require(self):
        source = "http::geturl http://example.com"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "http" not in pkgs
        spec = REGISTRY.get("http::geturl", active_packages=pkgs)
        assert spec is None

    def test_msgcat_pipeline(self):
        source = 'package require msgcat\nmsgcat::mc "hello"'
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "msgcat" in pkgs
        spec = REGISTRY.get("msgcat::mc", active_packages=pkgs)
        assert spec is not None


class TestFullPipelineTcllib:
    """End-to-end for tcllib packages."""

    def test_json_pipeline(self):
        source = "package require json\njson::json2dict $data"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "json" in pkgs
        # tcllib commands use required_package derived from tcllib_package
        spec = REGISTRY.get("json::json2dict", active_packages=pkgs)
        assert spec is not None

    def test_json_without_require(self):
        source = "json::json2dict $data"
        result = analyse(source)
        pkgs = result.active_package_names()
        spec = REGISTRY.get("json::json2dict", active_packages=pkgs)
        assert spec is None

    def test_base64_pipeline(self):
        source = "package require base64\nbase64::encode $data"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "base64" in pkgs
        spec = REGISTRY.get("base64::encode", active_packages=pkgs)
        assert spec is not None

    def test_mixed_stdlib_and_tcllib(self):
        """Both stdlib and tcllib packages active together."""
        source = (
            "package require http\n"
            "package require json\n"
            "http::geturl http://api.example.com\n"
            "json::json2dict $response\n"
        )
        result = analyse(source)
        pkgs = result.active_package_names()
        assert REGISTRY.get("http::geturl", active_packages=pkgs) is not None
        assert REGISTRY.get("json::json2dict", active_packages=pkgs) is not None


class TestFullPipelineTk:
    """End-to-end for Tk packages."""

    def test_tk_pipeline(self):
        source = "package require Tk\nbutton .b -text hello\npack .b"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "Tk" in pkgs
        assert has_tk_require(result)

        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(pkgs))
        assert "button" in names
        assert "pack" in names

    def test_tk_without_require(self):
        source = "button .b -text hello"
        result = analyse(source)
        pkgs = result.active_package_names()
        assert "Tk" not in pkgs
        assert not has_tk_require(result)

        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(pkgs))
        assert "button" not in names

    def test_tk_with_stdlib_packages(self):
        """Tk and stdlib packages active together."""
        source = (
            "package require Tk\n"
            "package require http\n"
            "button .b -text hello\n"
            "http::geturl http://example.com\n"
        )
        result = analyse(source)
        pkgs = result.active_package_names()
        assert has_tk_require(result)
        assert REGISTRY.get("http::geturl", active_packages=pkgs) is not None

        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(pkgs))
        assert "button" in names
        assert "http::geturl" in names


class TestFullPipelineAllThree:
    """All three package sources active simultaneously."""

    def test_stdlib_tcllib_tk_together(self):
        source = (
            "package require Tk\n"
            "package require http\n"
            "package require json\n"
            "package require base64\n"
            "button .b -text hello\n"
            "pack .b\n"
            "http::geturl http://api.example.com\n"
            "json::json2dict $response\n"
            "base64::encode $data\n"
        )
        result = analyse(source)
        pkgs = result.active_package_names()
        assert pkgs == frozenset({"Tk", "http", "json", "base64"})
        assert has_tk_require(result)

        registry = CommandRegistry.build_default()
        names = set(registry.commands_for_packages(pkgs))

        # Tk commands
        assert "button" in names
        assert "pack" in names
        # Stdlib commands
        assert "http::geturl" in names
        # Core builtins always present
        assert "set" in names
        assert "puts" in names

    def test_required_package_for_all_types(self):
        """required_package_for works for stdlib, tcllib, and Tk commands."""
        assert REGISTRY.required_package_for("http::geturl") == "http"
        assert REGISTRY.required_package_for("button") == "Tk"
        assert REGISTRY.required_package_for("set") is None
        # tcllib commands also have required_package set
        assert REGISTRY.has_required_package("json::json2dict")


# Section 10: Package resolver + workspace index integration


class TestResolverToIndexPipeline:
    """Simulate the server-side flow: resolver finds files → index them."""

    def test_resolve_and_index_package(self):
        """Resolve a package, analyse its source, and index the procs."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "mylib1.0")
            os.makedirs(pkg_dir)
            impl = os.path.join(pkg_dir, "mylib.tcl")
            Path(impl).write_text(
                "namespace eval mylib {\n"
                "    proc init {} { return ok }\n"
                "    proc cleanup {} { return done }\n"
                "}\n"
            )
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded mylib 1.0 [list source [file join $dir mylib.tcl]]"
            )

            # Step 1: Resolve package to source files
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("mylib")
            assert len(files) == 1

            # Step 2: Analyse the resolved source file
            source = Path(files[0]).read_text()
            result = analyse(source)

            # Step 3: Index with PACKAGE source kind
            idx = WorkspaceIndex()
            uri = f"file://{files[0]}"
            idx.update(uri, result, EntrySource.PACKAGE)

            # Step 4: Verify procs are findable
            entries = idx.find_proc("mylib::init")
            assert len(entries) == 1
            assert entries[0].source_kind == EntrySource.PACKAGE

            entries = idx.find_proc("mylib::cleanup")
            assert len(entries) == 1

    def test_resolve_split_package_and_index(self):
        """Split package: multiple files → all procs indexed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            _make_pkg_dir(
                tmpdir, "splitlib", "1.0", ["core.tcl", "utils.tcl"], ifneeded_style="multi"
            )
            # Write meaningful content
            pkg_dir = os.path.join(tmpdir, "splitlib1.0")
            Path(os.path.join(pkg_dir, "core.tcl")).write_text(
                "namespace eval splitlib {\n    proc core_func {} { return core }\n}\n"
            )
            Path(os.path.join(pkg_dir, "utils.tcl")).write_text(
                "namespace eval splitlib {\n    proc util_func {} { return util }\n}\n"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("splitlib")
            assert len(files) == 2

            idx = WorkspaceIndex()
            for f in files:
                source = Path(f).read_text()
                result = analyse(source)
                idx.update(f"file://{f}", result, EntrySource.PACKAGE)

            assert len(idx.find_proc("splitlib::core_func")) == 1
            assert len(idx.find_proc("splitlib::util_func")) == 1
