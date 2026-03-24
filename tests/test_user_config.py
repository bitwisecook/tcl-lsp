"""Tests for user_config.py — XDG config file parsing."""

from __future__ import annotations

import configparser
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.common.user_config import (
    get_all_settings,
    get_generic_variable_patterns,
    save_settings_to_config,
)


def _config_from_string(text: str) -> configparser.ConfigParser:
    """Build a ConfigParser from an INI string (case-preserving, like load_user_config)."""
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment]  # preserve camelCase
    config.read_string(text)
    return config


class TestGetAllSettings:
    """Tests for get_all_settings()."""

    def test_empty_config(self):
        config = _config_from_string("")
        assert get_all_settings(config) == {}

    def test_diagnostics_disabled(self):
        config = _config_from_string("[diagnostics]\ndisabled = W111, T100, IRULE1005\n")
        result = get_all_settings(config)
        diag = result["diagnostics"]
        assert diag["W111"] is False
        assert diag["T100"] is False
        assert diag["IRULE1005"] is False

    def test_optimiser_disabled(self):
        config = _config_from_string("[optimiser]\nenabled = true\ndisabled = O109, O126\n")
        result = get_all_settings(config)
        opt = result["optimiser"]
        assert opt["enabled"] is True
        assert opt["O109"] is False
        assert opt["O126"] is False

    def test_optimiser_master_off(self):
        config = _config_from_string("[optimiser]\nenabled = false\n")
        result = get_all_settings(config)
        assert result["optimiser"]["enabled"] is False

    def test_shimmer_toggle(self):
        config = _config_from_string("[shimmer]\nenabled = false\n")
        result = get_all_settings(config)
        assert result["shimmer"]["enabled"] is False

    def test_xc_diagnostics(self):
        config = _config_from_string("[xcDiagnostics]\nenabled = true\n")
        result = get_all_settings(config)
        assert result["xcDiagnostics"]["enabled"] is True

    def test_features(self):
        config = _config_from_string("[features]\nhover = false\ncompletion = true\n")
        result = get_all_settings(config)
        assert result["features"]["hover"] is False
        assert result["features"]["completion"] is True

    def test_features_camelcase_preserved(self):
        """camelCase feature keys like semanticTokens must survive config parsing."""
        config = _config_from_string(
            "[features]\nsemanticTokens = false\ninlayHints = false\ncodeActions = true\n"
        )
        result = get_all_settings(config)
        assert result["features"]["semanticTokens"] is False
        assert result["features"]["inlayHints"] is False
        assert result["features"]["codeActions"] is True

    def test_formatting(self):
        config = _config_from_string("[formatting]\nindent_size = 2\nindent_style = tabs\n")
        result = get_all_settings(config)
        assert result["formatting"]["indent_size"] == 2
        assert result["formatting"]["indent_style"] == "tabs"

    def test_style_line_length(self):
        config = _config_from_string("[style]\nline_length = 80\n")
        result = get_all_settings(config)
        assert result["style"]["lineLength"] == 80

    def test_generic_variable_patterns(self):
        config = _config_from_string(
            "[diagnostics]\ngeneric_variable_patterns =\n    ^debug$\n    ^test$\n"
        )
        result = get_all_settings(config)
        assert result["diagnostics"]["genericVariablePatterns"] == ["^debug$", "^test$"]

    def test_multiline_disabled_list(self):
        """Disabled codes can span multiple lines."""
        config = _config_from_string("[diagnostics]\ndisabled =\n    W111\n    W100, T100\n")
        result = get_all_settings(config)
        diag = result["diagnostics"]
        assert diag["W111"] is False
        assert diag["W100"] is False
        assert diag["T100"] is False


class TestSaveSettings:
    """Tests for save_settings_to_config()."""

    def test_roundtrip_disabled_optimisations(self, tmp_path, monkeypatch):
        monkeypatch.setattr(
            "core.common.user_config._config_path",
            lambda: tmp_path / "config.ini",
        )

        settings = {
            "optimiser": {"enabled": True, "O109": False, "O126": False},
            "diagnostics": {"W111": False},
        }
        path = save_settings_to_config(settings, only_non_default=False)
        assert Path(path).exists()

        # Re-read and verify
        reloaded = configparser.ConfigParser()
        reloaded.read(path)
        assert reloaded.has_option("optimiser", "disabled")
        disabled_str = reloaded.get("optimiser", "disabled")
        assert "O109" in disabled_str
        assert "O126" in disabled_str
        assert reloaded.has_option("diagnostics", "disabled")
        assert "W111" in reloaded.get("diagnostics", "disabled")


class TestGetGenericVariablePatterns:
    """Tests for get_generic_variable_patterns()."""

    def test_no_section_returns_none(self):
        config = _config_from_string("")
        assert get_generic_variable_patterns(config) is None

    def test_empty_patterns_returns_none(self):
        config = _config_from_string("[diagnostics]\ngeneric_variable_patterns =\n")
        assert get_generic_variable_patterns(config) is None

    def test_patterns_returned(self):
        config = _config_from_string(
            "[diagnostics]\ngeneric_variable_patterns =\n    ^foo$\n    ^bar$\n"
        )
        result = get_generic_variable_patterns(config)
        assert result == ["^foo$", "^bar$"]
