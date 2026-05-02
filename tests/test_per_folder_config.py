"""Per-workspace-folder configuration resolution (issue #230).

Covers:

- ``config_for_uri`` / ``formatter_config_for_uri`` longest-prefix
  matching and fallback when a document is outside every folder.
- ``_apply_settings_to_target`` writes feature toggles, formatter
  fields, line length, and disabled diagnostics to the right folder.
- The merged settings layers (``editor``, ``project``) honour the
  folder URI when resolving merged settings.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

import lsp.settings as _lsp_settings
import lsp.state as _lsp_state
from core.formatting import FormatterConfig
from lsp.feature_config import FeatureConfig


@pytest.fixture
def reset_per_folder_state():
    """Snapshot/restore the module-level per-folder maps and fallback configs."""
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


class TestConfigResolution:
    """``config_for_uri`` / ``formatter_config_for_uri`` resolver behaviour."""

    def test_falls_back_when_no_per_folder_configs(self, reset_per_folder_state):
        cfg = _lsp_state.config_for_uri("file:///tmp/foo.tcl")
        assert cfg is _lsp_state.feature_config

    def test_falls_back_when_uri_outside_every_folder(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-a")
        cfg = _lsp_state.config_for_uri("file:///elsewhere/foo.tcl")
        assert cfg is _lsp_state.feature_config

    def test_picks_matching_folder(self, reset_per_folder_state):
        a = _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-a")
        b = _lsp_state.get_or_init_folder_feature_config("file:///home/user/proj-b")
        a.line_length = 160
        b.line_length = 80

        assert _lsp_state.config_for_uri("file:///home/user/proj-a/foo.tcl").line_length == 160
        assert _lsp_state.config_for_uri("file:///home/user/proj-b/foo.tcl").line_length == 80

    def test_longest_prefix_wins_for_nested_folders(self, reset_per_folder_state):
        outer = _lsp_state.get_or_init_folder_feature_config("file:///home/user/repo")
        inner = _lsp_state.get_or_init_folder_feature_config("file:///home/user/repo/sub")
        outer.line_length = 100
        inner.line_length = 200

        outer_uri = "file:///home/user/repo/foo.tcl"
        inner_uri = "file:///home/user/repo/sub/bar.tcl"
        assert _lsp_state.config_for_uri(outer_uri).line_length == 100
        assert _lsp_state.config_for_uri(inner_uri).line_length == 200

    def test_does_not_match_sibling_path_with_shared_prefix(self, reset_per_folder_state):
        """``/home/foo`` should not match a doc under ``/home/foobar/...``."""
        _lsp_state.get_or_init_folder_feature_config("file:///home/foo").line_length = 99
        cfg = _lsp_state.config_for_uri("file:///home/foobar/x.tcl")
        assert cfg.line_length != 99
        assert cfg is _lsp_state.feature_config

    def test_uri_equal_to_folder_resolves_to_that_folder(self, reset_per_folder_state):
        cfg = _lsp_state.get_or_init_folder_feature_config("file:///home/foo")
        cfg.line_length = 77
        assert _lsp_state.config_for_uri("file:///home/foo").line_length == 77

    def test_none_uri_falls_back(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_feature_config("file:///home/foo")
        assert _lsp_state.config_for_uri(None) is _lsp_state.feature_config

    def test_formatter_config_for_uri_picks_matching_folder(self, reset_per_folder_state):
        a = _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-a")
        b = _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-b")
        a.max_line_length = 160
        b.max_line_length = 80

        a_uri = "file:///home/user/proj-a/foo.tcl"
        b_uri = "file:///home/user/proj-b/foo.tcl"
        assert _lsp_state.formatter_config_for_uri(a_uri).max_line_length == 160
        assert _lsp_state.formatter_config_for_uri(b_uri).max_line_length == 80

    def test_formatter_config_falls_back_outside_folders(self, reset_per_folder_state):
        _lsp_state.get_or_init_folder_formatter_config("file:///home/user/proj-a")
        cfg = _lsp_state.formatter_config_for_uri("file:///tmp/x.tcl")
        assert cfg is _lsp_state.formatter_config


class TestApplyToTarget:
    """``_apply_settings_to_target`` writes to the right config object."""

    def test_fallback_target_mutates_global_feature_config(self, reset_per_folder_state):
        _lsp_settings._apply_settings_to_target(
            "", {"style": {"lineLength": 90}, "shimmer": {"enabled": False}}
        )
        assert _lsp_state.feature_config.line_length == 90
        assert _lsp_state.feature_config.shimmer_enabled is False

    def test_folder_target_creates_isolated_config(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"style": {"lineLength": 200}})

        # Folder config sees the new value; fallback is unchanged.
        assert _lsp_state.config_for_uri(f"{folder_uri}/foo.tcl").line_length == 200
        assert _lsp_state.feature_config.line_length == 120

    def test_two_folders_independent(self, reset_per_folder_state):
        a_uri = "file:///home/user/proj-a"
        b_uri = "file:///home/user/proj-b"
        _lsp_settings._apply_settings_to_target(a_uri, {"style": {"lineLength": 200}})
        _lsp_settings._apply_settings_to_target(b_uri, {"style": {"lineLength": 60}})

        assert _lsp_state.config_for_uri(f"{a_uri}/x.tcl").line_length == 200
        assert _lsp_state.config_for_uri(f"{b_uri}/x.tcl").line_length == 60
        # Fallback untouched.
        assert _lsp_state.feature_config.line_length == 120

    def test_formatting_settings_applied_per_folder(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"formatting": {"maxLineLength": 160}})

        fmt_cfg = _lsp_state.formatter_config_for_uri(f"{folder_uri}/foo.tcl")
        assert fmt_cfg.max_line_length == 160
        # Fallback formatter unchanged.
        assert _lsp_state.formatter_config.max_line_length == 120

    def test_disabled_diagnostics_per_folder(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_settings._apply_settings_to_target(folder_uri, {"diagnostics": {"W111": False}})

        a_cfg = _lsp_state.config_for_uri(f"{folder_uri}/foo.tcl")
        assert "W111" in a_cfg.disabled_diagnostics
        # Fallback should not have W111 disabled (default keeps it enabled).
        assert "W111" not in _lsp_state.feature_config.disabled_diagnostics


class TestMergedSettingsPerFolder:
    """``_merged_settings`` resolves per-folder editor/project layers."""

    def test_per_folder_layers_isolated_from_fallback(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_state.editor_config_settings_per_folder[folder_uri] = {"style": {"lineLength": 99}}
        _lsp_state.editor_config_settings["style"] = {"lineLength": 50}

        merged_folder = _lsp_settings._merged_settings(folder_uri)
        merged_fallback = _lsp_settings._merged_settings("")

        assert merged_folder["style"]["lineLength"] == 99
        assert merged_fallback["style"]["lineLength"] == 50

    def test_folder_layer_falls_back_when_unset(self, reset_per_folder_state):
        """An unknown folder URI falls through to workspace-level editor/project."""
        _lsp_state.editor_config_settings["style"] = {"lineLength": 50}
        merged = _lsp_settings._merged_settings("file:///home/user/unknown")
        assert merged["style"]["lineLength"] == 50


class TestDropFolderConfigs:
    def test_drop_clears_all_layers(self, reset_per_folder_state):
        folder_uri = "file:///home/user/proj-a"
        _lsp_state.get_or_init_folder_feature_config(folder_uri)
        _lsp_state.get_or_init_folder_formatter_config(folder_uri)
        _lsp_state.editor_config_settings_per_folder[folder_uri] = {"x": 1}
        _lsp_state.project_config_settings_per_folder[folder_uri] = {"y": 2}

        _lsp_state.drop_folder_configs(folder_uri)

        assert folder_uri not in _lsp_state._per_folder_feature_configs
        assert folder_uri not in _lsp_state._per_folder_formatter_configs
        assert folder_uri not in _lsp_state.editor_config_settings_per_folder
        assert folder_uri not in _lsp_state.project_config_settings_per_folder
