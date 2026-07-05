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

"""Tests for the willRenameFiles/didRenameFiles rename-edit computation."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult
from server.features.workspace_file_ops import (
    compute_batch_rename_edits,
    compute_rename_edits,
)
from server.workspace.scanner import path_to_uri


class _FakeIndex:
    def __init__(self, per_uri: dict[str, AnalysisResult]) -> None:
        self._per_uri = per_uri

    def reverse_dependents(self, uri: str) -> frozenset[str]:
        return frozenset(self._per_uri.keys())

    def get_analysis(self, uri: str) -> AnalysisResult | None:
        return self._per_uri.get(uri)


def _path_to_uri(p: str) -> str:
    """Convert an absolute filesystem path to a ``file://`` URI.

    Delegates to the shared scanner helper so Windows drives, spaces, and
    non-ASCII characters encode correctly on every platform.
    """
    return path_to_uri(p)


class TestComputeRenameEdits:
    def test_rewrites_relative_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            b = os.path.join(tmp, "b.tcl")
            a = os.path.join(tmp, "a.tcl")
            Path(b).write_text("# b\n")
            Path(a).write_text('source "b.tcl"\n')
            analysis = analyse('source "b.tcl"\n')
            idx = _FakeIndex({_path_to_uri(a): analysis})
            result = compute_rename_edits(
                old_uri=_path_to_uri(b),
                new_uri=_path_to_uri(os.path.join(tmp, "c.tcl")),
                index=idx,
                workspace_roots=[tmp],
            )
            assert result is not None
            assert result.document_changes is not None
            assert len(result.document_changes) == 1
            doc_edit = result.document_changes[0]
            assert isinstance(doc_edit, types.TextDocumentEdit)
            assert doc_edit.text_document.uri == _path_to_uri(a)
            assert len(doc_edit.edits) == 1
            edit0 = doc_edit.edits[0]
            assert isinstance(edit0, types.TextEdit)
            assert edit0.new_text == "c.tcl"

    def test_leaves_substitution_paths_alone(self):
        with tempfile.TemporaryDirectory() as tmp:
            b = os.path.join(tmp, "b.tcl")
            a = os.path.join(tmp, "a.tcl")
            Path(b).write_text("# b\n")
            Path(a).write_text("source $::env(B)\n")
            analysis = analyse("source $::env(B)\n")
            idx = _FakeIndex({_path_to_uri(a): analysis})
            result = compute_rename_edits(
                old_uri=_path_to_uri(b),
                new_uri=_path_to_uri(os.path.join(tmp, "c.tcl")),
                index=idx,
                workspace_roots=[tmp],
            )
            assert result is None

    def test_no_dependents(self):
        with tempfile.TemporaryDirectory() as tmp:
            idx = _FakeIndex({})
            result = compute_rename_edits(
                old_uri=_path_to_uri(os.path.join(tmp, "x.tcl")),
                new_uri=_path_to_uri(os.path.join(tmp, "y.tcl")),
                index=idx,
                workspace_roots=[tmp],
            )
            assert result is None


class TestComputeBatchRenameEdits:
    def test_batch_rewrites_multiple_renames(self):
        with tempfile.TemporaryDirectory() as tmp:
            b = os.path.join(tmp, "b.tcl")
            c = os.path.join(tmp, "c.tcl")
            dep = os.path.join(tmp, "dep.tcl")
            Path(b).write_text("# b\n")
            Path(c).write_text("# c\n")
            dep_src = 'source "b.tcl"\nsource "c.tcl"\n'
            Path(dep).write_text(dep_src)
            analysis = analyse(dep_src)
            idx = _FakeIndex({_path_to_uri(dep): analysis})
            result = compute_batch_rename_edits(
                files=[
                    types.FileRename(
                        old_uri=_path_to_uri(b),
                        new_uri=_path_to_uri(os.path.join(tmp, "bb.tcl")),
                    ),
                    types.FileRename(
                        old_uri=_path_to_uri(c),
                        new_uri=_path_to_uri(os.path.join(tmp, "cc.tcl")),
                    ),
                ],
                index=idx,
                workspace_roots=[tmp],
            )
            assert result is not None
            assert result.document_changes is not None
            assert len(result.document_changes) == 1
            doc_edit = result.document_changes[0]
            assert isinstance(doc_edit, types.TextDocumentEdit)
            assert len(doc_edit.edits) == 2
            plain_edits = [e for e in doc_edit.edits if isinstance(e, types.TextEdit)]
            assert len(plain_edits) == 2
            new_texts = sorted(e.new_text for e in plain_edits)
            assert new_texts == ["bb.tcl", "cc.tcl"]
