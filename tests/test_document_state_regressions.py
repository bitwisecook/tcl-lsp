"""Regression tests for document-state concurrency and dialect handling."""

from __future__ import annotations

from compiler.registry.runtime import active_signature_profile, configure_signatures
from server.workspace.document_state import DocumentState


def test_tokens_property_uses_snapshot_cache() -> None:
    state = DocumentState(uri="file:///test.tcl")
    state.source = "set value 1\n"

    tokens = state.tokens

    assert tokens
    assert state.tokens is tokens


def test_irules_document_update_restores_workspace_dialect() -> None:
    configure_signatures(dialect="tcl8.6")
    state = DocumentState(uri="file:///rule.irul")

    state.update("when HTTP_REQUEST { pool /Common/app_pool }\n")

    assert active_signature_profile()["dialect"] == "tcl8.6"
