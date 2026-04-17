"""Tests for $/progress wiring in _run_background_scan."""

from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import MagicMock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import lsp.server as server_module


class TestRunBackgroundScanProgress:
    def test_progress_cb_forwarded_to_scanner(self, monkeypatch):
        """_run_background_scan should forward progress_cb to scan_all."""
        fake_scanner = MagicMock()
        fake_scanner.scan_all.return_value = {}
        fake_scanner.irules_procs = {}
        fake_scanner.irules_rule_init_vars = {}
        fake_scanner.auto_index_entries = {}

        monkeypatch.setattr(server_module, "background_scanner", fake_scanner)

        # Prevent dialect upgrade from touching real state.
        monkeypatch.setattr(server_module.feature_config, "dialect_explicitly_set", True)

        observed = []

        def cb(i, n, p):
            observed.append((i, n, p))

        server_module._run_background_scan(progress_cb=cb)

        fake_scanner.scan_all.assert_called_once()
        call_kwargs = fake_scanner.scan_all.call_args.kwargs
        assert call_kwargs.get("progress_cb") is cb

    def test_progress_cb_is_optional(self, monkeypatch):
        fake_scanner = MagicMock()
        fake_scanner.scan_all.return_value = {}
        fake_scanner.irules_procs = {}
        fake_scanner.irules_rule_init_vars = {}
        fake_scanner.auto_index_entries = {}
        monkeypatch.setattr(server_module, "background_scanner", fake_scanner)
        monkeypatch.setattr(server_module.feature_config, "dialect_explicitly_set", True)
        server_module._run_background_scan()
        fake_scanner.scan_all.assert_called_once()
        assert fake_scanner.scan_all.call_args.kwargs.get("progress_cb") is None


class TestScannerProgressCallback:
    """scan_all invokes progress_cb once per discovered file."""

    def test_scan_all_invokes_cb(self, tmp_path):
        (tmp_path / "a.tcl").write_text("proc a {} {}\n")
        (tmp_path / "b.tcl").write_text("proc b {} {}\n")
        from lsp.workspace.scanner import BackgroundScanner

        scanner = BackgroundScanner()
        scanner.configure(workspace_roots=[str(tmp_path)])
        calls: list[tuple[int, int, str]] = []
        scanner.scan_all(progress_cb=lambda i, n, p: calls.append((i, n, p)))
        # Should fire for every file encountered (count may include sourced deps).
        assert len(calls) >= 2
        assert all(c[1] >= 2 for c in calls)
