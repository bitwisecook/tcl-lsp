"""Tests for the willSaveWaitUntil format-on-save handler."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from server.features.formatting import get_formatting
from tooling.formatter import FormatterConfig
from tooling.formatter.config import IndentStyle


def _lsp_options_for(config: FormatterConfig) -> types.FormattingOptions:
    return types.FormattingOptions(
        tab_size=config.indent_size,
        insert_spaces=config.indent_style == IndentStyle.SPACES,
    )


class TestWillSaveWaitUntilDelegation:
    """The server's on_will_save_wait_until delegates to get_formatting.

    This test exercises the pure helper call chain the handler uses so it
    catches regressions in either the FormattingOptions construction or the
    underlying formatter integration.
    """

    def test_reformats_messy_proc(self):
        source = "proc f {a  b} {return [expr {$a+$b}]}\n"
        config = FormatterConfig()
        edits = get_formatting(source, _lsp_options_for(config), config)
        assert edits is not None
        assert len(edits) >= 1
        assert edits[0].new_text != source

    def test_already_formatted_returns_empty(self):
        source = "proc f {a b} {\n    return [expr {$a + $b}]\n}\n"
        config = FormatterConfig()
        edits = get_formatting(source, _lsp_options_for(config), config)
        # Idempotent — already-formatted source yields no edits.
        assert edits == [] or all(e.new_text == source for e in edits)

    def test_handler_respects_indent_style(self):
        """insert_spaces flag flows from FormatterConfig.indent_style."""
        config_spaces = FormatterConfig(indent_size=2, indent_style=IndentStyle.SPACES)
        opts_spaces = _lsp_options_for(config_spaces)
        assert opts_spaces.insert_spaces is True
        assert opts_spaces.tab_size == 2

        config_tabs = FormatterConfig(indent_style=IndentStyle.TABS)
        opts_tabs = _lsp_options_for(config_tabs)
        assert opts_tabs.insert_spaces is False
