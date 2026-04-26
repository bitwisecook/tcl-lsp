"""Tests for the Tcl package resolver."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

import pytest

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

    def test_first_search_path_wins(self):
        # When the same package name is provided by two paths, the first
        # one scanned takes priority — matching Tcl's ``auto_path``
        # semantics where the first ``package ifneeded`` that matches
        # satisfies the require.
        with tempfile.TemporaryDirectory() as tmpdir:
            workspace = os.path.join(tmpdir, "ws")
            libdir = os.path.join(tmpdir, "lib")
            for base in (workspace, libdir):
                pkg_dir = os.path.join(base, "mylib")
                os.makedirs(pkg_dir)
                Path(os.path.join(pkg_dir, "mylib.tcl")).write_text(f"# from {base}")
                Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                    "package ifneeded mylib 1.0 [list source [file join $dir mylib.tcl]]"
                )
            resolver = PackageResolver()
            # Workspace first — its copy should win.
            resolver.configure(search_paths=[workspace, libdir])
            files = resolver.resolve("mylib")
            assert len(files) == 1
            assert files[0].startswith(workspace)

    def test_add_search_paths_preserves_existing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            first_dir = os.path.join(tmpdir, "first")
            second_dir = os.path.join(tmpdir, "second")
            for base, name in ((first_dir, "firstpkg"), (second_dir, "secondpkg")):
                pkg_dir = os.path.join(base, f"{name}1.0")
                os.makedirs(pkg_dir)
                Path(os.path.join(pkg_dir, f"{name}.tcl")).write_text("")
                Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                    f"package ifneeded {name} 1.0 [list source [file join $dir {name}.tcl]]"
                )
            resolver = PackageResolver()
            resolver.configure(search_paths=[first_dir])
            resolver.scan_packages()
            assert "firstpkg" in resolver.all_package_names()
            resolver.add_search_paths([second_dir])
            assert "firstpkg" in resolver.all_package_names()
            assert "secondpkg" in resolver.all_package_names()

    def test_add_search_paths_prepend_wins_over_existing_provider(self):
        # Regression for the Codex-flagged
        # ``add_search_paths(prepend=True)`` ordering bug:
        # ``resolve()`` returns the first ``ifneeded`` entry, but
        # the prior implementation only updated ``_search_paths``
        # without rotating ``_packages[name]``, so an old provider
        # already at the head of the list silently shadowed the
        # newly-prepended workspace path.
        with tempfile.TemporaryDirectory() as tmpdir:
            old_dir = os.path.join(tmpdir, "old")
            new_dir = os.path.join(tmpdir, "new")
            for base in (old_dir, new_dir):
                pkg_dir = os.path.join(base, "shared1.0")
                os.makedirs(pkg_dir)
                Path(os.path.join(pkg_dir, "shared.tcl")).write_text(f"# from {base}")
                Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                    "package ifneeded shared 1.0 [list source [file join $dir shared.tcl]]"
                )
            resolver = PackageResolver()
            resolver.configure(search_paths=[old_dir])
            resolver.scan_packages()
            old_files = resolver.resolve("shared")
            assert old_files and old_files[0].startswith(old_dir)
            # Prepend the new path — workspace-wins semantics: the
            # ``shared`` package must now resolve to the *new* dir.
            resolver.add_search_paths([new_dir], prepend=True)
            new_files = resolver.resolve("shared")
            assert new_files and new_files[0].startswith(new_dir), (
                f"prepend=True should make {new_dir} win, but resolve() returned {new_files}"
            )

    def test_add_search_paths_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg_dir = os.path.join(tmpdir, "pkg1.0")
            os.makedirs(pkg_dir)
            Path(os.path.join(pkg_dir, "pkg.tcl")).write_text("")
            Path(os.path.join(pkg_dir, "pkgIndex.tcl")).write_text(
                "package ifneeded pkg 1.0 [list source [file join $dir pkg.tcl]]"
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[])
            resolver.scan_packages()
            resolver.add_search_paths([tmpdir])
            first_count = len(resolver.resolve("pkg"))
            resolver.add_search_paths([tmpdir])
            second_count = len(resolver.resolve("pkg"))
            assert first_count == second_count == 1


def _abs(p: str) -> str:
    return os.path.abspath(os.path.expanduser(p))


class TestAddSearchPathsPrepend:
    """Order-preservation contract for ``add_search_paths(prepend=True)``.

    Each batch of paths must keep its own input order; later batches must
    sit in front of earlier batches. Naive ``insert(0, …)`` per element
    reverses the input order within a batch.
    """

    def test_single_batch_preserves_input_order(self):
        resolver = PackageResolver()
        resolver.add_search_paths(["/tmp/a", "/tmp/b", "/tmp/c"], prepend=True)
        assert resolver._search_paths == [_abs("/tmp/a"), _abs("/tmp/b"), _abs("/tmp/c")]

    def test_two_batches_preserve_per_batch_and_batch_order(self):
        resolver = PackageResolver()
        resolver.add_search_paths(["/tmp/a", "/tmp/b", "/tmp/c"], prepend=True)
        resolver.add_search_paths(["/tmp/d", "/tmp/e"], prepend=True)
        # Second batch in front, each batch keeps its own input order.
        assert resolver._search_paths == [
            _abs("/tmp/d"),
            _abs("/tmp/e"),
            _abs("/tmp/a"),
            _abs("/tmp/b"),
            _abs("/tmp/c"),
        ]

    @pytest.mark.parametrize(
        ("first", "second", "expected_relative"),
        [
            (["/tmp/a"], ["/tmp/b"], ["/tmp/b", "/tmp/a"]),
            (["/tmp/a", "/tmp/b"], ["/tmp/c"], ["/tmp/c", "/tmp/a", "/tmp/b"]),
            (["/tmp/a"], ["/tmp/b", "/tmp/c"], ["/tmp/b", "/tmp/c", "/tmp/a"]),
            (
                ["/tmp/a", "/tmp/b", "/tmp/c"],
                ["/tmp/d", "/tmp/e", "/tmp/f"],
                ["/tmp/d", "/tmp/e", "/tmp/f", "/tmp/a", "/tmp/b", "/tmp/c"],
            ),
        ],
    )
    def test_parametrised_two_batch_orderings(
        self, first: list[str], second: list[str], expected_relative: list[str]
    ) -> None:
        resolver = PackageResolver()
        resolver.add_search_paths(first, prepend=True)
        resolver.add_search_paths(second, prepend=True)
        assert resolver._search_paths == [_abs(p) for p in expected_relative]

    def test_prepend_with_existing_non_prepend_path(self):
        resolver = PackageResolver()
        resolver.add_search_paths(["/tmp/existing"], prepend=False)
        resolver.add_search_paths(["/tmp/new1", "/tmp/new2"], prepend=True)
        # Prepended batch keeps input order; original non-prepend path slides back.
        assert resolver._search_paths == [
            _abs("/tmp/new1"),
            _abs("/tmp/new2"),
            _abs("/tmp/existing"),
        ]
