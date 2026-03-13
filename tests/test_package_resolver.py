"""Tests for the Tcl package resolver."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.packages import PackageResolver


class TestPackageResolver:
    def test_parse_standard_pkg_index(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create package structure
            pkg_dir = os.path.join(tmpdir, "http1.0")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "http.tcl")).write_text(
                "proc http::geturl {url} { return $url }"
            )
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded http 1.0 [list source [file join $dir http.tcl]]"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            resolver.scan_packages()

            assert "http" in resolver.all_package_names()

    def test_resolve_known_package(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "mylib")
            os.makedirs(pkg_dir)
            impl_file = os.path.join(pkg_dir, "mylib.tcl")
            Path(impl_file).write_text("proc mylib::init {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded mylib 2.1 [list source [file join $dir mylib.tcl]]"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("mylib")

            assert len(files) == 1
            assert files[0] == impl_file

    def test_resolve_unknown_package(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            assert resolver.resolve("nonexistent") == []

    def test_resolve_with_version(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "foo")
            os.makedirs(pkg_dir)
            impl_file = os.path.join(pkg_dir, "foo.tcl")
            Path(impl_file).write_text("proc foo {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded foo 3.2 [list source [file join $dir foo.tcl]]"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])

            # Exact version match
            files = resolver.resolve("foo", "3.2")
            assert len(files) == 1

            # Prefix match
            files = resolver.resolve("foo", "3")
            assert len(files) == 1

            # No match
            files = resolver.resolve("foo", "9.9")
            assert len(files) == 0

    def test_nested_package_dirs(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # Simulate typical Tcl library layout
            pkg_a = os.path.join(tmpdir, "lib", "alpha1.0")
            pkg_b = os.path.join(tmpdir, "lib", "beta2.0")
            os.makedirs(pkg_a)
            os.makedirs(pkg_b)
            Path(os.path.join(pkg_a, "alpha.tcl")).write_text("proc alpha {} {}")
            Path(os.path.join(pkg_a, "pkgIndex.tcl")).write_text(
                "package ifneeded alpha 1.0 [list source [file join $dir alpha.tcl]]"
            )
            Path(os.path.join(pkg_b, "beta.tcl")).write_text("proc beta {} {}")
            Path(os.path.join(pkg_b, "pkgIndex.tcl")).write_text(
                "package ifneeded beta 2.0 [list source [file join $dir beta.tcl]]"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            resolver.scan_packages()

            names = resolver.all_package_names()
            assert "alpha" in names
            assert "beta" in names

    def test_source_dir_slash_pattern(self):
        """Test the $dir/filename pattern."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "pkg")
            os.makedirs(pkg_dir)
            impl_file = os.path.join(pkg_dir, "impl.tcl")
            Path(impl_file).write_text("proc impl {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                'package ifneeded mypkg 1.0 "source $dir/impl.tcl"'
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("mypkg")

            assert len(files) == 1
            assert files[0] == impl_file

    def test_fallback_to_tcl_files_in_dir(self):
        """When pkgIndex has no parseable source lines, fall back to .tcl files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "quirky")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "quirky.tcl")).write_text("proc quirky {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded quirky 1.0 {load [file join $dir quirky.so]}"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            files = resolver.resolve("quirky")

            # Should fall back to quirky.tcl since no source pattern matched
            assert len(files) == 1
            assert files[0].endswith("quirky.tcl")

    def test_lazy_scan(self):
        """Scanning happens automatically on first resolve if not yet scanned."""
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "lazy")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "lazy.tcl")).write_text("proc lazy {} {}")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded lazy 1.0 [list source [file join $dir lazy.tcl]]"
            )

            resolver = PackageResolver()
            resolver.configure(search_paths=[tmpdir])
            # Don't call scan_packages explicitly
            files = resolver.resolve("lazy")
            assert len(files) == 1

    def test_nonexistent_search_path(self):
        resolver = PackageResolver()
        resolver.configure(search_paths=["/nonexistent/path/xyz"])
        resolver.scan_packages()
        assert resolver.all_package_names() == []
