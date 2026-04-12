"""Tests for the background workspace scanner."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.workspace.scanner import (
    BackgroundScanner,
    _dialect_from_ext,
    path_to_uri,
    uri_to_path,
)


class TestPathToUri:
    def test_simple_path(self):
        uri = path_to_uri("/tmp/foo.tcl")
        assert uri == "file:///tmp/foo.tcl"

    def test_path_with_spaces(self):
        uri = path_to_uri("/tmp/my file.tcl")
        assert "my%20file.tcl" in uri
        assert uri.startswith("file:///")


class TestUriToPath:
    def test_file_uri(self):
        assert uri_to_path("file:///tmp/foo.tcl") == "/tmp/foo.tcl"

    def test_non_file_uri(self):
        assert uri_to_path("untitled:Untitled-1") is None

    def test_encoded_spaces(self):
        result = uri_to_path("file:///tmp/my%20file.tcl")
        assert result == "/tmp/my file.tcl"


class TestDialectFromExt:
    def test_irules(self):
        assert _dialect_from_ext(".irul") == "f5-irules"
        assert _dialect_from_ext(".irule") == "f5-irules"

    def test_iapps(self):
        assert _dialect_from_ext(".iapp") == "f5-iapps"
        assert _dialect_from_ext(".iappimpl") == "f5-iapps"
        assert _dialect_from_ext(".impl") == "f5-iapps"

    def test_tcl(self):
        assert _dialect_from_ext(".tcl") is None
        assert _dialect_from_ext(".tk") is None
        assert _dialect_from_ext(".tm") is None


class TestBackgroundScanner:
    def test_scan_empty_directory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()
            assert results == {}

    def test_scan_finds_tcl_files(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a .tcl file with a proc
            tcl_file = os.path.join(tmpdir, "lib.tcl")
            Path(tcl_file).write_text("proc greet {name} { puts $name }")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()

            assert len(results) == 1
            uri = path_to_uri(tcl_file)
            assert uri in results
            assert "::greet" in results[uri].analysis.all_procs

    def test_scan_recursive(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            subdir = os.path.join(tmpdir, "sub")
            os.makedirs(subdir)
            Path(os.path.join(subdir, "deep.tcl")).write_text("proc deep {} {}")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()

            assert len(results) == 1
            sr = list(results.values())[0]
            assert "::deep" in sr.analysis.all_procs

    def test_scan_ignores_non_tcl(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "script.py")).write_text("def foo(): pass")
            Path(os.path.join(tmpdir, "lib.tcl")).write_text("proc foo {} {}")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()

            assert len(results) == 1

    def test_scan_library_paths(self):
        with tempfile.TemporaryDirectory() as wsdir, tempfile.TemporaryDirectory() as libdir:
            Path(os.path.join(wsdir, "main.tcl")).write_text("proc main {} {}")
            Path(os.path.join(libdir, "util.tcl")).write_text("proc util {} {}")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[wsdir], library_paths=[libdir])
            results = scanner.scan_all()

            assert len(results) == 2

    def test_dialect_hint_irules(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "test.irul")).write_text(
                "when HTTP_REQUEST { proc handler {} {} }"
            )

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()

            sr = list(results.values())[0]
            assert sr.dialect_hint == "f5-irules"

    def test_irules_procs_property(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "rule.irul")).write_text(
                "proc helper {} {}\nproc handler {} {}"
            )
            Path(os.path.join(tmpdir, "lib.tcl")).write_text("proc tcl_util {} {}")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            irules_procs = scanner.irules_procs
            assert len(irules_procs) == 1
            uri = list(irules_procs.keys())[0]
            assert "::helper" in irules_procs[uri]
            assert "::handler" in irules_procs[uri]

    def test_rescan_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tcl_file = os.path.join(tmpdir, "lib.tcl")
            Path(tcl_file).write_text("proc foo {} {}")

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            # Modify the file
            Path(tcl_file).write_text("proc bar {} {}")
            result = scanner.rescan_file(tcl_file)

            assert result is not None
            assert "::bar" in result.analysis.all_procs
            assert "::foo" not in result.analysis.all_procs

    def test_has_cached_and_get_cached(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tcl_file = os.path.join(tmpdir, "lib.tcl")
            Path(tcl_file).write_text("proc foo {} {}")
            uri = path_to_uri(tcl_file)

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            assert scanner.has_cached(uri)
            assert scanner.get_cached(uri) is not None
            assert not scanner.has_cached("file:///nonexistent.tcl")
            assert scanner.get_cached("file:///nonexistent.tcl") is None

    def test_remove_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tcl_file = os.path.join(tmpdir, "lib.tcl")
            Path(tcl_file).write_text("proc foo {} {}")
            uri = path_to_uri(tcl_file)

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            scanner.scan_all()

            assert scanner.has_cached(uri)
            scanner.remove_file(uri)
            assert not scanner.has_cached(uri)

    def test_nonexistent_directory(self):
        scanner = BackgroundScanner()
        scanner.configure(workspace_roots=["/nonexistent/path/xyz"])
        results = scanner.scan_all()
        assert results == {}
