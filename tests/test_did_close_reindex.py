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

"""did_close re-indexes a closed document from disk instead of dropping it.

The initial folder scan only runs at ``initialized``, so a file opened
after that has no background-cache entry.  Closing it must re-read the
on-disk contents into the workspace index (keeping cross-document
definition / references / rename / call-hierarchy working), and only
remove the entry when the file is actually gone.
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import server.state as _state
from server.lifecycle import _reindex_closed_document_from_disk
from server.workspace.scanner import BackgroundScanner, path_to_uri
from server.workspace.workspace_index import WorkspaceIndex


def _fresh_state(monkeypatch) -> None:
    monkeypatch.setattr(_state, "background_scanner", BackgroundScanner())
    monkeypatch.setattr(_state, "workspace_index", WorkspaceIndex())


def test_reindex_keeps_on_disk_file_in_index(monkeypatch):
    _fresh_state(monkeypatch)
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "lib.tcl")
        Path(path).write_text("proc greet {name} { puts $name }\n", encoding="utf-8")
        uri = path_to_uri(path)

        assert _reindex_closed_document_from_disk(uri) is True
        entries = _state.workspace_index.find_proc("greet")
        assert [e.uri for e in entries] == [uri]


def test_reindex_reports_missing_file(monkeypatch):
    _fresh_state(monkeypatch)
    with tempfile.TemporaryDirectory() as d:
        uri = path_to_uri(os.path.join(d, "gone.tcl"))
        assert _reindex_closed_document_from_disk(uri) is False
        assert _state.workspace_index.find_proc("greet") == []


def test_reindex_non_file_uri_reports_false(monkeypatch):
    _fresh_state(monkeypatch)
    assert _reindex_closed_document_from_disk("untitled:Untitled-1") is False
