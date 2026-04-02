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
        # Use ``full`` profile as baseline so all codes start enabled,
        # then disable O103/O104 to verify the per-code override works.
        changed = server_module._apply_feature_settings(
            {"optimiser": {"profile": "full", "O103": False, "O104": False}}
        )
        assert changed
        assert "O103" in server_module.feature_config.disabled_optimisations
        assert "O104" in server_module.feature_config.disabled_optimisations
    finally:
        server_module.feature_config = original


# -- _extract_tcl_lsp_settings -----------------------------------------------


class TestExtractSettings:
    """Tests for the _extract_tcl_lsp_settings helper."""

    def test_nested_payload(self):
        """Standard nested shape: {\"tclLsp\": {...}}."""
        settings = {"tclLsp": {"dialect": "tcl8.6", "optimiser": {"O109": False}}}
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result["dialect"] == "tcl8.6"
        assert result["optimiser"]["O109"] is False

    def test_flat_keys(self):
        """Flat dotted keys like tclLsp.optimiser.O109."""
        settings = {
            "tclLsp.optimiser.O109": False,
            "tclLsp.optimiser.enabled": True,
            "tclLsp.dialect": "f5-irules",
        }
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result["dialect"] == "f5-irules"
        assert result["optimiser"]["O109"] is False
        assert result["optimiser"]["enabled"] is True

    def test_unwrapped_payload(self):
        """Unwrapped payload from JetBrains or pull-model responses."""
        settings = {
            "dialect": "tcl8.6",
            "optimiser": {"O109": False, "enabled": True},
            "diagnostics": {"W111": False},
        }
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result["dialect"] == "tcl8.6"
        assert result["optimiser"]["O109"] is False
        assert result["diagnostics"]["W111"] is False

    def test_unwrapped_not_triggered_for_random_keys(self):
        """Unrelated settings dicts should not trigger the unwrapped fallback."""
        settings = {"some_other_setting": 42, "another": "value"}
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result == {}

    def test_flat_keys_xcDiagnostics(self):
        """xcDiagnostics section is routed from flat keys."""
        settings = {"tclLsp.xcDiagnostics.enabled": True}
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result["xcDiagnostics"]["enabled"] is True

    def test_flat_keys_runtimeValidation(self):
        """runtimeValidation section is routed from flat keys."""
        settings = {"tclLsp.runtimeValidation.enabled": False}
        result = server_module._extract_tcl_lsp_settings(settings)
        assert result["runtimeValidation"]["enabled"] is False

    def test_empty_settings(self):
        """Empty settings dict returns empty extraction."""
        assert server_module._extract_tcl_lsp_settings({}) == {}


# -- _apply_feature_settings --------------------------------------------------


class TestApplyFeatureSettings:
    """Tests for _apply_feature_settings."""

    def _with_fresh_config(self, settings):
        """Apply settings on a fresh FeatureConfig, return (changed, config)."""
        original = server_module.feature_config
        try:
            server_module.feature_config = server_module.FeatureConfig()
            changed = server_module._apply_feature_settings(settings)
            return changed, server_module.feature_config
        finally:
            server_module.feature_config = original

    def test_disable_diagnostics(self):
        changed, cfg = self._with_fresh_config(
            {"diagnostics": {"W111": False, "T100": False, "IRULE1005": False}}
        )
        assert changed
        assert "W111" in cfg.disabled_diagnostics
        assert "T100" in cfg.disabled_diagnostics
        assert "IRULE1005" in cfg.disabled_diagnostics

    def test_disable_optimisations(self):
        # Use ``full`` profile so all codes start enabled, then verify
        # per-code overrides disable them.
        changed, cfg = self._with_fresh_config(
            {"optimiser": {"profile": "full", "O109": False, "O126": False}}
        )
        assert changed
        assert "O109" in cfg.disabled_optimisations
        assert "O126" in cfg.disabled_optimisations

    def test_master_optimiser_toggle(self):
        changed, cfg = self._with_fresh_config({"optimiser": {"enabled": False}})
        assert changed
        assert cfg.optimiser_enabled is False

    def test_shimmer_toggle(self):
        changed, cfg = self._with_fresh_config({"shimmer": {"enabled": False}})
        assert changed
        assert cfg.shimmer_enabled is False

    def test_xc_diagnostics_toggle(self):
        changed, cfg = self._with_fresh_config({"xcDiagnostics": {"enabled": True}})
        assert changed
        assert cfg.xc_diagnostics_enabled is True

    def test_feature_toggle(self):
        changed, cfg = self._with_fresh_config({"features": {"hover": False, "completion": False}})
        assert changed
        assert cfg.hover_enabled is False
        assert cfg.completion_enabled is False

    def test_style_line_length(self):
        changed, cfg = self._with_fresh_config({"style": {"lineLength": 80}})
        assert changed
        assert cfg.line_length == 80

    def test_no_change_returns_false(self):
        """When settings match defaults, changed should be False."""
        changed, _ = self._with_fresh_config({"optimiser": {"enabled": True}})
        assert not changed

    def test_wrong_type_bool_ignored(self):
        """String where bool expected should not crash."""
        changed, cfg = self._with_fresh_config({"shimmer": {"enabled": "yes"}})
        assert not changed
        assert cfg.shimmer_enabled is True  # default preserved

    def test_unrecognised_codes_ignored(self):
        """Codes not in the manifest are silently stored but don't crash."""
        changed, cfg = self._with_fresh_config({"diagnostics": {"FAKE999": False}})
        # FAKE999 is not in _ALL_DIAGNOSTIC_CODES, so it should not appear
        assert "FAKE999" not in cfg.disabled_diagnostics

    def test_wrong_type_line_length_ignored(self):
        """String where int expected should not crash."""
        changed, cfg = self._with_fresh_config({"style": {"lineLength": "eighty"}})
        assert not changed
        assert cfg.line_length == 120  # default
