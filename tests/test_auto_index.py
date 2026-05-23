"""Tests for tclIndex auto-loading index file parsing."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.packages.auto_index import parse_tcl_index
from analyser.packages.resolver import PackageResolver
from server.workspace.scanner import BackgroundScanner
from server.workspace.workspace_index import EntrySource, WorkspaceIndex

# Parser unit tests


class TestParseTclIndex:
    """parse_tcl_index() unit tests."""

    def _write_index(self, tmpdir: str, content: str, *, files: list[str] | None = None) -> str:
        """Write a tclIndex file and create referenced source files."""
        index_path = os.path.join(tmpdir, "tclIndex")
        Path(index_path).write_text(content, encoding="utf-8")
        for fname in files or []:
            Path(os.path.join(tmpdir, fname)).write_text(
                f"proc {fname.removesuffix('.tcl')} {{}} {{}}\n",
                encoding="utf-8",
            )
        return index_path

    def test_standard_line(self):
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(
                d,
                "set auto_index(myProc) [list source [file join $dir myfile.tcl]]\n",
                files=["myfile.tcl"],
            )
            entries = parse_tcl_index(path)
            assert len(entries) == 1
            assert entries[0].proc_name == "myProc"
            assert entries[0].source_file == os.path.join(d, "myfile.tcl")

    def test_namespaced_proc(self):
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(
                d,
                "set auto_index(::http::geturl) [list source [file join $dir http.tcl]]\n",
                files=["http.tcl"],
            )
            entries = parse_tcl_index(path)
            assert len(entries) == 1
            assert entries[0].proc_name == "::http::geturl"

    def test_pkg_source_variant(self):
        """Tcl 9.0 uses ::tcl::Pkg::source instead of plain source."""
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(
                d,
                "set auto_index(auto_reset) [list ::tcl::Pkg::source [file join $dir auto.tcl]]\n",
                files=["auto.tcl"],
            )
            entries = parse_tcl_index(path)
            assert len(entries) == 1
            assert entries[0].proc_name == "auto_reset"

    def test_encoding_variant(self):
        """Tcl 8.6 may include -encoding utf-8."""
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(
                d,
                "set auto_index(tcltest) [list source -encoding utf-8 [file join $dir tcltest.tcl]]\n",
                files=["tcltest.tcl"],
            )
            entries = parse_tcl_index(path)
            assert len(entries) == 1
            assert entries[0].proc_name == "tcltest"

    def test_missing_source_file_skipped(self):
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(
                d,
                "set auto_index(gone) [list source [file join $dir missing.tcl]]\n",
            )
            entries = parse_tcl_index(path)
            assert len(entries) == 0

    def test_empty_file(self):
        with tempfile.TemporaryDirectory() as d:
            path = self._write_index(d, "# empty\n")
            entries = parse_tcl_index(path)
            assert entries == []

    def test_nonexistent_file(self):
        entries = parse_tcl_index("/no/such/tclIndex")
        assert entries == []

    def test_mixed_valid_invalid(self):
        with tempfile.TemporaryDirectory() as d:
            content = (
                "# Tcl autoload index file, version 2.0\n"
                "set auto_index(good) [list source [file join $dir good.tcl]]\n"
                "this is not a valid line\n"
                "set auto_index(missing) [list source [file join $dir gone.tcl]]\n"
                "set auto_index(also_good) [list source [file join $dir also.tcl]]\n"
            )
            path = self._write_index(d, content, files=["good.tcl", "also.tcl"])
            entries = parse_tcl_index(path)
            assert len(entries) == 2
            names = {e.proc_name for e in entries}
            assert names == {"good", "also_good"}

    def test_multiple_procs_same_file(self):
        with tempfile.TemporaryDirectory() as d:
            content = (
                "set auto_index(foo) [list source [file join $dir utils.tcl]]\n"
                "set auto_index(bar) [list source [file join $dir utils.tcl]]\n"
            )
            path = self._write_index(d, content, files=["utils.tcl"])
            entries = parse_tcl_index(path)
            assert len(entries) == 2
            assert {e.proc_name for e in entries} == {"foo", "bar"}
            assert entries[0].source_file == entries[1].source_file

    def test_real_tcl_index(self):
        """Parse an actual tclIndex from a Tcl source tree if available."""
        real_index = (
            Path(__file__).resolve().parent.parent / "tmp" / "tcl9.0.3" / "library" / "tclIndex"
        )
        if not real_index.exists():
            return  # skip if Tcl source not fetched
        entries = parse_tcl_index(str(real_index))
        assert len(entries) > 0
        names = {e.proc_name for e in entries}
        assert "auto_reset" in names


# Scanner integration tests


class TestBackgroundScannerAutoIndex:
    """BackgroundScanner tclIndex integration."""

    def _setup_tree(self, tmpdir: str) -> None:
        """Create a directory tree with a tclIndex and source files."""
        Path(os.path.join(tmpdir, "lib")).mkdir()
        Path(os.path.join(tmpdir, "lib", "helpers.tcl")).write_text(
            "proc myHelper {x} { return $x }\n",
            encoding="utf-8",
        )
        Path(os.path.join(tmpdir, "lib", "tclIndex")).write_text(
            "set auto_index(myHelper) [list source [file join $dir helpers.tcl]]\n",
            encoding="utf-8",
        )

    def test_collect_files_includes_tclindex_refs(self):
        with tempfile.TemporaryDirectory() as d:
            self._setup_tree(d)
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[d])
            files = scanner.collect_files()
            paths = {fp for fp, _ in files}
            assert os.path.join(d, "lib", "helpers.tcl") in paths

    def test_tclindex_file_not_analysed(self):
        with tempfile.TemporaryDirectory() as d:
            self._setup_tree(d)
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[d])
            files = scanner.collect_files()
            paths = {fp for fp, _ in files}
            assert os.path.join(d, "lib", "tclIndex") not in paths

    def test_auto_index_entries_populated(self):
        with tempfile.TemporaryDirectory() as d:
            self._setup_tree(d)
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[d])
            scanner.collect_files()
            entries = scanner.auto_index_entries
            assert "myHelper" in entries
            assert entries["myHelper"] == os.path.join(d, "lib", "helpers.tcl")

    def test_non_standard_extension_via_tclindex(self):
        """Files referenced by tclIndex that don't match TCL_EXTENSIONS are included."""
        with tempfile.TemporaryDirectory() as d:
            Path(os.path.join(d, "custom.lib")).write_text(
                "proc customProc {} {}\n",
                encoding="utf-8",
            )
            Path(os.path.join(d, "tclIndex")).write_text(
                "set auto_index(customProc) [list source [file join $dir custom.lib]]\n",
                encoding="utf-8",
            )
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[d])
            files = scanner.collect_files()
            paths = {fp for fp, _ in files}
            assert os.path.join(d, "custom.lib") in paths

    def test_no_duplicate_files(self):
        """Files already discovered by extension should not appear twice."""
        with tempfile.TemporaryDirectory() as d:
            self._setup_tree(d)
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[d])
            files = scanner.collect_files()
            paths = [fp for fp, _ in files]
            helpers_path = os.path.join(d, "lib", "helpers.tcl")
            assert paths.count(helpers_path) == 1


# Resolver integration tests


class TestPackageResolverAutoIndex:
    """PackageResolver tclIndex integration."""

    def test_resolve_auto_proc(self):
        with tempfile.TemporaryDirectory() as d:
            Path(os.path.join(d, "utils.tcl")).write_text(
                "proc myUtil {} {}\n",
                encoding="utf-8",
            )
            Path(os.path.join(d, "tclIndex")).write_text(
                "set auto_index(myUtil) [list source [file join $dir utils.tcl]]\n",
                encoding="utf-8",
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[d])
            files = resolver.resolve_auto_proc("myUtil")
            assert len(files) == 1
            assert files[0] == os.path.join(d, "utils.tcl")

    def test_all_auto_proc_names(self):
        with tempfile.TemporaryDirectory() as d:
            Path(os.path.join(d, "a.tcl")).write_text("proc a {} {}\n", encoding="utf-8")
            Path(os.path.join(d, "b.tcl")).write_text("proc b {} {}\n", encoding="utf-8")
            Path(os.path.join(d, "tclIndex")).write_text(
                "set auto_index(a) [list source [file join $dir a.tcl]]\n"
                "set auto_index(b) [list source [file join $dir b.tcl]]\n",
                encoding="utf-8",
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[d])
            names = resolver.all_auto_proc_names()
            assert set(names) == {"a", "b"}

    def test_coexistence_with_pkg_index(self):
        """Both pkgIndex.tcl and tclIndex are parsed in the same directory."""
        with tempfile.TemporaryDirectory() as d:
            Path(os.path.join(d, "mypkg.tcl")).write_text(
                "package provide mypkg 1.0\nproc mypkg_init {} {}\n",
                encoding="utf-8",
            )
            Path(os.path.join(d, "autohelper.tcl")).write_text(
                "proc autohelper {} {}\n",
                encoding="utf-8",
            )
            Path(os.path.join(d, "pkgIndex.tcl")).write_text(
                "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
                encoding="utf-8",
            )
            Path(os.path.join(d, "tclIndex")).write_text(
                "set auto_index(autohelper) [list source [file join $dir autohelper.tcl]]\n",
                encoding="utf-8",
            )
            resolver = PackageResolver()
            resolver.configure(search_paths=[d])
            # Package resolution works
            pkg_files = resolver.resolve("mypkg")
            assert len(pkg_files) == 1
            # Auto-index resolution works
            auto_files = resolver.resolve_auto_proc("autohelper")
            assert len(auto_files) == 1

    def test_unknown_proc_returns_empty(self):
        resolver = PackageResolver()
        resolver.configure(search_paths=[])
        assert resolver.resolve_auto_proc("nonexistent") == []


# WorkspaceIndex tests


class TestWorkspaceIndexAutoIndex:
    """WorkspaceIndex AUTO_INDEX entry source handling."""

    def test_auto_index_is_background(self):
        idx = WorkspaceIndex()
        from analyser.semantic_model import AnalysisResult

        result = AnalysisResult()
        idx.update("file:///test.tcl", result, EntrySource.AUTO_INDEX)
        assert idx.is_background("file:///test.tcl")

    def test_remove_background_clears_auto_index(self):
        idx = WorkspaceIndex()
        from analyser.semantic_model import AnalysisResult

        result = AnalysisResult()
        idx.update("file:///test.tcl", result, EntrySource.AUTO_INDEX)
        idx.remove_background_entries()
        assert idx.get_analysis("file:///test.tcl") is None
