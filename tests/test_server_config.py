"""Tests for LSP server configuration parsing."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import lsp.server as server_module


def test_optimiser_filters_include_o103_o104():
    original = server_module.feature_config
    try:
        server_module.feature_config = server_module.FeatureConfig()
        changed = server_module._apply_feature_settings(
            {"optimiser": {"O103": False, "O104": False}}
        )
        assert changed
        assert "O103" in server_module.feature_config.disabled_optimisations
        assert "O104" in server_module.feature_config.disabled_optimisations
    finally:
        server_module.feature_config = original
