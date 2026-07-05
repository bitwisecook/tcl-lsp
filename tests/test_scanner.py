# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the background workspace scanner."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.workspace.scanner import (
    BackgroundScanner,
    _dialect_from_ext,
    is_editor_lock_symlink,
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

    def test_scan_ignores_emacs_lock_symlink(self):
        """A dangling Emacs lock symlink (``.#name.tcl``) is not analysed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            Path(os.path.join(tmpdir, "lib.tcl")).write_text("proc foo {} {}")
            # Emacs lock file: a symlink to a target that does not exist.
            lock = os.path.join(tmpdir, ".#lib.tcl")
            os.symlink("user@host.12345:1700000000", lock)

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            results = scanner.scan_all()

            # Only the real file is indexed; the lock symlink is skipped
            # without raising on the unreadable target.
            assert len(results) == 1
            assert path_to_uri(os.path.join(tmpdir, "lib.tcl")) in results
            assert path_to_uri(lock) not in results

    def test_rescan_skips_emacs_lock_symlink_without_reading(self, monkeypatch):
        """``rescan_file`` short-circuits a lock symlink before any read.

        The fix's value is avoiding the read entirely (and the log churn it
        produced); the old code would attempt the read, raise
        ``FileNotFoundError`` on the dangling target, and swallow it.  Guard
        against regressing back to "read then swallow" by failing if the file
        is read at all.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            lock = os.path.join(tmpdir, ".#rename-only.tcl")
            os.symlink("user@host.12345:1700000000", lock)

            def _no_read(self, *args, **kwargs):
                raise AssertionError(f"lock symlink should not be read: {self}")

            monkeypatch.setattr(Path, "read_text", _no_read)

            scanner = BackgroundScanner()
            scanner.configure(workspace_roots=[tmpdir])
            assert scanner.rescan_file(lock) is None

    def test_run_analysis_swallows_missing_file_quietly(self, caplog):
        """A genuinely missing file logs at debug with no traceback."""
        import logging

        scanner = BackgroundScanner()
        with caplog.at_level(logging.DEBUG, logger="server.workspace.scanner"):
            assert scanner.rescan_file("/nonexistent/path/gone.tcl") is None
        # No exception escaped, and nothing was logged with traceback info.
        assert all(rec.exc_info is None for rec in caplog.records)


class TestEditorLockSymlink:
    def test_dangling_lock_symlink_detected(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            lock = os.path.join(tmpdir, ".#doc.tcl")
            os.symlink("user@host.999:1", lock)
            assert is_editor_lock_symlink(lock) is True

    def test_regular_file_starting_with_hash_not_skipped(self):
        """A real (non-symlink) file named ``.#...`` is still analysed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            real = os.path.join(tmpdir, ".#real.tcl")
            Path(real).write_text("proc foo {} {}")
            assert is_editor_lock_symlink(real) is False

    def test_ordinary_source_not_skipped(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            ordinary = os.path.join(tmpdir, "lib.tcl")
            Path(ordinary).write_text("proc foo {} {}")
            assert is_editor_lock_symlink(ordinary) is False

    def test_symlink_without_lock_prefix_not_skipped(self):
        """A symlink that is not a ``.#`` lock file is not treated as one."""
        with tempfile.TemporaryDirectory() as tmpdir:
            target = os.path.join(tmpdir, "lib.tcl")
            Path(target).write_text("proc foo {} {}")
            link = os.path.join(tmpdir, "alias.tcl")
            os.symlink(target, link)
            assert is_editor_lock_symlink(link) is False
