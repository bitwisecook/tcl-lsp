"""File-operation helpers for workspace/willRename + didRename.

When a Tcl file is renamed we rewrite ``source`` statements in every
dependent file so the workspace still loads.  The core helper
``compute_rename_edits`` is pure and unit-testable; the pygls handlers in
``server.py`` translate the results into LSP types.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Protocol

from lsprotocol import types

from core.analysis.semantic_model import AnalysisResult, SourceTarget
from core.analysis.source_resolver import resolve_source_target
from shared.lsp import to_lsp_range

from ..workspace.scanner import path_to_uri, uri_to_path  # noqa: F401  (path_to_uri re-exported)


class _WorkspaceLike(Protocol):
    def reverse_dependents(self, uri: str) -> frozenset[str]: ...
    def get_analysis(self, uri: str) -> AnalysisResult | None: ...


def _compute_new_literal(
    old_literal: str,
    script_path: str,
    new_abs_path: str,
) -> str:
    """Return the new source-target literal preserving the existing style.

    - Absolute literal paths stay absolute.
    - Relative literal paths stay relative to the dependent script's directory.
    """
    if os.path.isabs(old_literal):
        return new_abs_path
    script_dir = os.path.dirname(script_path)
    try:
        rel = os.path.relpath(new_abs_path, start=script_dir)
    except ValueError:
        # On Windows this raises if paths live on different drives.  Fall
        # back to the absolute path.
        return new_abs_path
    return rel


@dataclass(slots=True, frozen=True)
class _RenameEdit:
    dep_uri: str
    target: SourceTarget
    new_text: str


def _collect_edits_for_rename(
    old_uri: str,
    new_uri: str,
    index: _WorkspaceLike,
    workspace_roots: list[str] | None,
) -> list[_RenameEdit]:
    new_path_raw = uri_to_path(new_uri)
    old_path_raw = uri_to_path(old_uri)
    if not new_path_raw or not old_path_raw:
        return []  # non-file URIs (untitled://, vscode-vfs://) are not renameable
    new_path = os.path.normpath(new_path_raw)
    old_path = os.path.normpath(old_path_raw)
    dependents = index.reverse_dependents(old_uri)
    edits: list[_RenameEdit] = []
    for dep_uri in dependents:
        analysis = index.get_analysis(dep_uri)
        if analysis is None:
            continue
        dep_path = uri_to_path(dep_uri)
        if not dep_path:
            continue
        for target in analysis.source_targets:
            if not target.is_literal:
                continue  # conservative: leave substitution paths alone
            resolved = resolve_source_target(
                target.raw_path,
                target.is_literal,
                dep_path,
                workspace_roots=workspace_roots,
            )
            if resolved is None:
                continue
            if os.path.normpath(resolved) != old_path:
                continue
            new_text = _compute_new_literal(target.raw_path, dep_path, new_path)
            edits.append(_RenameEdit(dep_uri=dep_uri, target=target, new_text=new_text))
    return edits


def compute_rename_edits(
    old_uri: str,
    new_uri: str,
    index: _WorkspaceLike,
    workspace_roots: list[str] | None = None,
) -> types.WorkspaceEdit | None:
    """Compute a WorkspaceEdit rewriting ``source`` references to ``old_uri``.

    Returns ``None`` when there are no edits to apply.
    """
    collected = _collect_edits_for_rename(old_uri, new_uri, index, workspace_roots)
    if not collected:
        return None
    by_dep: dict[str, list[types.TextEdit]] = {}
    for edit in collected:
        by_dep.setdefault(edit.dep_uri, []).append(
            types.TextEdit(
                range=to_lsp_range(edit.target.range),
                new_text=edit.new_text,
            )
        )
    document_changes = [
        types.TextDocumentEdit(
            text_document=types.OptionalVersionedTextDocumentIdentifier(
                uri=dep_uri,
                version=None,
            ),
            edits=edits,
        )
        for dep_uri, edits in by_dep.items()
    ]
    return types.WorkspaceEdit(document_changes=document_changes)


def compute_batch_rename_edits(
    files: list[types.FileRename],
    index: _WorkspaceLike,
    workspace_roots: list[str] | None = None,
) -> types.WorkspaceEdit | None:
    """Compute a single WorkspaceEdit covering multiple renames."""
    all_edits: list[_RenameEdit] = []
    for f in files:
        all_edits.extend(_collect_edits_for_rename(f.old_uri, f.new_uri, index, workspace_roots))
    if not all_edits:
        return None
    by_dep: dict[str, list[types.TextEdit]] = {}
    for edit in all_edits:
        by_dep.setdefault(edit.dep_uri, []).append(
            types.TextEdit(
                range=to_lsp_range(edit.target.range),
                new_text=edit.new_text,
            )
        )
    document_changes = [
        types.TextDocumentEdit(
            text_document=types.OptionalVersionedTextDocumentIdentifier(
                uri=dep_uri,
                version=None,
            ),
            edits=edits,
        )
        for dep_uri, edits in by_dep.items()
    ]
    return types.WorkspaceEdit(document_changes=document_changes)
