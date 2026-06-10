"""Regression test for issue #133 — format-on-save must be off by default.

Issue #133 ("Formatting VSCode settings are ignored when Formatting Feature
is enabled") reported that the server reformatted whole files on save even
when the user's editor had format-on-save disabled and a different formatter
selected.  The fix is that the server's ``willSaveWaitUntil`` format-on-save
fallback is **opt-in**: the ``will_save_wait_until_enabled`` toggle defaults
to ``False`` and ``on_will_save_wait_until`` returns ``None`` (no edits) when
it is off.  Editors with a native format-on-save mechanism (VS Code's
``editor.formatOnSave``) route through ``textDocument/formatting`` instead.

``tests/test_will_save.py`` covers the willSaveWaitUntil -> formatter
delegation, but nothing there asserts the actual fix.  These tests pin the
default-off behaviour so that re-enabling format-on-save by default would
fail loudly.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest
from lsprotocol import types

import server.state as _lsp_state
from server.feature_config import FeatureConfig
from server.server import on_will_save_wait_until

# A deliberately mis-formatted proc so a real format-on-save pass produces
# edits (used by the positive control to keep the negative tests honest).
_MESSY_SOURCE = "proc f {a  b} {return [expr {$a+$b}]}\n"
_DOC_URI = "file:///tmp/issue_133_format_on_save.tcl"


@pytest.fixture
def reset_feature_config():
    """Snapshot/restore the process-global feature config and document state."""
    snap_feature = _lsp_state.feature_config
    snap_per_folder = dict(_lsp_state._per_folder_feature_configs)
    try:
        _lsp_state.feature_config = FeatureConfig()
        _lsp_state._per_folder_feature_configs.clear()
        yield
    finally:
        _lsp_state.feature_config = snap_feature
        _lsp_state._per_folder_feature_configs.clear()
        _lsp_state._per_folder_feature_configs.update(snap_per_folder)
        _lsp_state.workspace_state.close(_DOC_URI)


def _will_save_params() -> types.WillSaveTextDocumentParams:
    return types.WillSaveTextDocumentParams(
        text_document=types.TextDocumentIdentifier(uri=_DOC_URI),
        reason=types.TextDocumentSaveReason.Manual,
    )


class TestFormatOnSaveDefaultOff:
    """Issue #133: format-on-save must never be the out-of-the-box default."""

    def test_default_config_does_not_enable_will_save_formatting(self):
        """A freshly constructed FeatureConfig has the toggle off."""
        assert FeatureConfig().will_save_wait_until_enabled is False

    def test_handler_returns_none_when_disabled(self, reset_feature_config):
        """With the default config, the handler emits no edits on save."""
        # Register a mis-formatted document; if the handler tried to format
        # it would return edits.  It must not, because the toggle is off.
        _lsp_state.workspace_state.open(_DOC_URI, _MESSY_SOURCE, analyse=False)
        assert _lsp_state.feature_config.will_save_wait_until_enabled is False

        result = on_will_save_wait_until(_will_save_params())

        assert result is None

    def test_handler_formats_when_explicitly_enabled(self, reset_feature_config):
        """Positive control: when opted in, the handler DOES format on save.

        Without this, the disabled-case assertions could pass vacuously (e.g.
        if the handler always returned None or the document never formats).
        """
        _lsp_state.workspace_state.open(_DOC_URI, _MESSY_SOURCE, analyse=False)
        _lsp_state.feature_config = FeatureConfig(will_save_wait_until_enabled=True)

        result = on_will_save_wait_until(_will_save_params())

        assert result is not None
        assert len(result) >= 1
        # The edit must actually change the mis-formatted source.
        assert any(edit.new_text != _MESSY_SOURCE for edit in result)
