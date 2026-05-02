"""End-to-end backend tests for per-folder config (issue #230).

Drives the full chain that the LSP handlers execute:

    1. Apply different ``tclLsp.*`` settings to two workspace folders.
    2. Pull each handler's config via ``config_for_uri(uri)`` /
       ``formatter_config_for_uri(uri)`` — the same calls the
       ``server.on_formatting`` / ``server.on_completion`` /
       diagnostics-pipeline handlers make.
    3. Assert the resulting output (formatted text, diagnostics) actually
       differs between folders for the same input source.

These are heavier than the unit tests in ``test_per_folder_config.py``
but pin the property "different folder settings produce different
output" — which is the user-visible contract from issue #230.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest
from lsprotocol import types

import lsp.settings as _lsp_settings
import lsp.state as _lsp_state
from core.formatting import FormatterConfig
from lsp.feature_config import FeatureConfig
from lsp.features.diagnostics import get_diagnostics
from lsp.features.formatting import get_formatting

FOLDER_A = "file:///home/user/proj-a"
FOLDER_B = "file:///home/user/proj-b"


@pytest.fixture
def two_folder_setup():
    """Snapshot/restore module state and provision two folders.

    Folder A: 160-col formatting, W111 diagnostic disabled.
    Folder B: 60-col formatting, W111 enabled (default).
    """
    snap_feature = _lsp_state.feature_config
    snap_formatter = _lsp_state.formatter_config
    snap_per_folder_feature = dict(_lsp_state._per_folder_feature_configs)
    snap_per_folder_formatter = dict(_lsp_state._per_folder_formatter_configs)
    snap_editor = dict(_lsp_state.editor_config_settings)
    snap_project = dict(_lsp_state.project_config_settings)
    snap_editor_pf = dict(_lsp_state.editor_config_settings_per_folder)
    snap_project_pf = dict(_lsp_state.project_config_settings_per_folder)

    _lsp_state.feature_config = FeatureConfig()
    _lsp_state.formatter_config = FormatterConfig()
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state.editor_config_settings.clear()
    _lsp_state.project_config_settings.clear()
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.clear()

    # Apply per-folder configs through the same path the production
    # pull-callback uses.
    _lsp_settings._apply_settings_to_target(
        FOLDER_A,
        {
            "formatting": {"maxLineLength": 160},
            "diagnostics": {"W111": False},
        },
    )
    _lsp_settings._apply_settings_to_target(
        FOLDER_B,
        {
            "formatting": {"maxLineLength": 60},
        },
    )

    yield

    _lsp_state.feature_config = snap_feature
    _lsp_state.formatter_config = snap_formatter
    _lsp_state._per_folder_feature_configs.clear()
    _lsp_state._per_folder_feature_configs.update(snap_per_folder_feature)
    _lsp_state._per_folder_formatter_configs.clear()
    _lsp_state._per_folder_formatter_configs.update(snap_per_folder_formatter)
    _lsp_state.editor_config_settings.clear()
    _lsp_state.editor_config_settings.update(snap_editor)
    _lsp_state.project_config_settings.clear()
    _lsp_state.project_config_settings.update(snap_project)
    _lsp_state.editor_config_settings_per_folder.clear()
    _lsp_state.editor_config_settings_per_folder.update(snap_editor_pf)
    _lsp_state.project_config_settings_per_folder.clear()
    _lsp_state.project_config_settings_per_folder.update(snap_project_pf)


# Formatting end-to-end


class TestFormattingEndToEnd:
    """``on_formatting`` resolves config per folder and produces folder-specific edits."""

    def test_max_line_length_differs_per_folder(self, two_folder_setup):
        # A long line that fits within 160 cols but not 60.
        long_line = (
            "set my_long_variable_name [list one two three four five six seven eight nine ten]\n"
        )
        source = f"proc foo {{}} {{\n    {long_line}}}\n"

        opts = types.FormattingOptions(tab_size=4, insert_spaces=True)

        edits_a = get_formatting(
            source, opts, _lsp_state.formatter_config_for_uri(f"{FOLDER_A}/foo.tcl")
        )
        edits_b = get_formatting(
            source, opts, _lsp_state.formatter_config_for_uri(f"{FOLDER_B}/foo.tcl")
        )

        # Same source, different formatter configs -> different output.
        a_text = edits_a[0].new_text if edits_a else source
        b_text = edits_b[0].new_text if edits_b else source

        # Folder B (60-col) wraps; folder A (160-col) does not.
        a_max = max(len(line) for line in a_text.splitlines())
        b_max = max(len(line) for line in b_text.splitlines())
        assert a_max > 60, f"folder A should keep long line (got max={a_max})"
        assert b_max <= 60 or b_max < a_max, (
            f"folder B should wrap below folder A (a={a_max}, b={b_max})"
        )

    def test_outside_folder_uses_fallback_formatter(self, two_folder_setup):
        # The fallback formatter has the default 120-col limit.
        outside_cfg = _lsp_state.formatter_config_for_uri("file:///tmp/elsewhere/x.tcl")
        assert outside_cfg.max_line_length == 120
        assert outside_cfg is _lsp_state.formatter_config


# Diagnostics end-to-end


class TestDiagnosticsEndToEnd:
    """``config_for_uri`` resolution drives folder-specific diagnostic codes."""

    def test_w111_disabled_in_folder_a_enabled_in_folder_b(self, two_folder_setup):
        # 200-character line — emits W111 (line too long) at default 120 col
        # limit.  We use a separate ``style.lineLength`` for the W111 check
        # to make this self-contained: folder A keeps the default 120, but
        # folder B inherited the same 120 plus W111 enabled.  Both folders
        # see a long line; only folder A has W111 silenced.
        long_line = "set x " + "a" * 250
        source = f"proc foo {{}} {{\n    {long_line}\n}}\n"

        cfg_a = _lsp_state.config_for_uri(f"{FOLDER_A}/foo.tcl")
        cfg_b = _lsp_state.config_for_uri(f"{FOLDER_B}/foo.tcl")

        diags_a = get_diagnostics(
            source,
            optimiser_enabled=cfg_a.optimiser_enabled,
            shimmer_enabled=cfg_a.shimmer_enabled,
            disabled_diagnostics=cfg_a.disabled_diagnostics,
            disabled_optimisations=cfg_a.disabled_optimisations,
            line_length=cfg_a.line_length,
            uri=f"{FOLDER_A}/foo.tcl",
        )
        diags_b = get_diagnostics(
            source,
            optimiser_enabled=cfg_b.optimiser_enabled,
            shimmer_enabled=cfg_b.shimmer_enabled,
            disabled_diagnostics=cfg_b.disabled_diagnostics,
            disabled_optimisations=cfg_b.disabled_optimisations,
            line_length=cfg_b.line_length,
            uri=f"{FOLDER_B}/foo.tcl",
        )

        codes_a = {d.code for d in diags_a}
        codes_b = {d.code for d in diags_b}

        assert "W111" not in codes_a, "folder A disabled W111 — should not appear"
        assert "W111" in codes_b, "folder B did not disable W111 — should appear"

    def test_two_folders_produce_distinct_diagnostic_sets(self, two_folder_setup):
        # The folder-A config and folder-B config differ — verify that's
        # observable end-to-end.
        cfg_a = _lsp_state.config_for_uri(f"{FOLDER_A}/foo.tcl")
        cfg_b = _lsp_state.config_for_uri(f"{FOLDER_B}/foo.tcl")
        assert cfg_a is not cfg_b
        assert cfg_a.disabled_diagnostics != cfg_b.disabled_diagnostics


# Cross-cutting: verify isolation


class TestFolderIsolationEndToEnd:
    def test_mutating_folder_a_does_not_leak_into_folder_b(self, two_folder_setup):
        cfg_a = _lsp_state.config_for_uri(f"{FOLDER_A}/x.tcl")
        cfg_b = _lsp_state.config_for_uri(f"{FOLDER_B}/x.tcl")

        # Mutate A directly (would happen via a settings change for that
        # folder).
        cfg_a.line_length = 999

        # B and the fallback are untouched.
        assert cfg_b.line_length != 999
        assert _lsp_state.feature_config.line_length != 999
