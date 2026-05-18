"""Tests for user_config.py — platform-native config file parsing."""

from __future__ import annotations

import configparser
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.common.user_config import (
    _cache_dir,
    _config_dir,
    _is_posix_compat_windows,
    get_all_settings,
    get_generic_variable_patterns,
    load_user_config,
    save_settings_to_config,
)


def _config_from_string(text: str) -> configparser.ConfigParser:
    """Build a ConfigParser from an INI string (case-preserving, like load_user_config)."""
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment, invalid-assignment]  # preserve camelCase
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


class TestGlobalAndProjectSections:
    """``[global]`` in config.ini and ``[project]`` in .tcl-lsp.ini.

    Mirrors the location-based safeguard documented in
    ``docs/design/contracts/config-precedence.md`` — the section name
    must match the file's role, and a mismatched section is ignored.
    """

    def test_global_dialect(self):
        config = _config_from_string("[global]\ndialect = tcl9.0\n")
        result = get_all_settings(config, kind="global")
        assert result["dialect"] == "tcl9.0"

    def test_project_dialect(self):
        config = _config_from_string("[project]\ndialect = tcl8.4\n")
        result = get_all_settings(config, kind="project")
        assert result["dialect"] == "tcl8.4"

    def test_global_section_in_project_file_is_ignored(self):
        config = _config_from_string("[global]\ndialect = tcl9.0\n")
        result = get_all_settings(config, kind="project")
        assert "dialect" not in result

    def test_project_section_in_global_file_is_ignored(self):
        config = _config_from_string("[project]\ndialect = tcl9.0\n")
        result = get_all_settings(config, kind="global")
        assert "dialect" not in result

    def test_kind_none_skips_both_sections(self):
        """Backward-compat: callers without a known origin get neither section."""
        config = _config_from_string("[global]\ndialect = tcl9.0\n[project]\ndialect = tcl8.4\n")
        result = get_all_settings(config)
        assert "dialect" not in result

    def test_extra_commands_comma_separated(self):
        config = _config_from_string("[global]\nextraCommands = mylib::send, mylib::recv\n")
        result = get_all_settings(config, kind="global")
        assert result["extraCommands"] == ["mylib::send", "mylib::recv"]

    def test_extra_commands_multiline(self):
        config = _config_from_string(
            "[project]\nextraCommands =\n    mylib::send\n    mylib::recv\n"
        )
        result = get_all_settings(config, kind="project")
        assert result["extraCommands"] == ["mylib::send", "mylib::recv"]

    def test_library_paths_multiline(self):
        config = _config_from_string(
            "[global]\nlibraryPaths =\n    /opt/tcl/lib\n    /home/me/stubs\n"
        )
        result = get_all_settings(config, kind="global")
        assert result["libraryPaths"] == ["/opt/tcl/lib", "/home/me/stubs"]

    def test_library_paths_comma_one_line(self):
        config = _config_from_string("[project]\nlibraryPaths = /opt/tcl/lib, /home/me/stubs\n")
        result = get_all_settings(config, kind="project")
        assert result["libraryPaths"] == ["/opt/tcl/lib", "/home/me/stubs"]

    def test_empty_dialect_is_ignored(self):
        config = _config_from_string("[global]\ndialect =\n")
        result = get_all_settings(config, kind="global")
        assert "dialect" not in result

    def test_wrong_section_logged_at_warning_level(self, caplog):
        import logging

        caplog.set_level(logging.WARNING, logger="core.common.user_config")
        config = _config_from_string("[project]\ndialect = tcl9.0\n")
        get_all_settings(config, kind="global")
        assert any("Ignored [project] section" in rec.message for rec in caplog.records)

    def test_no_arg_path_treats_file_as_global(self, tmp_path, monkeypatch):
        """``get_all_settings()`` with no arguments parses the global file
        as ``kind="global"``; the convenience path must not silently
        drop ``[global]`` top-level keys."""
        cfg_dir = tmp_path / "tcl-lsp"
        cfg_dir.mkdir()
        (cfg_dir / "config.ini").write_text("[global]\ndialect = tcl9.0\n")
        monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
        result = get_all_settings()
        assert result["dialect"] == "tcl9.0"

    def test_combined_with_other_sections(self):
        """Top-level keys coexist with the regular nested sections."""
        config = _config_from_string("[global]\ndialect = tcl9.0\n[diagnostics]\ndisabled = W111\n")
        result = get_all_settings(config, kind="global")
        assert result["dialect"] == "tcl9.0"
        assert result["diagnostics"]["W111"] is False


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


class TestInvalidInputs:
    """Negative tests for robustness against bad config values."""

    def test_invalid_bool_ignored(self):
        config = _config_from_string("[shimmer]\nenabled = banana\n")
        result = get_all_settings(config)
        assert "shimmer" not in result  # invalid bool produces no section

    def test_unrecognised_diagnostic_codes_do_not_crash(self):
        config = _config_from_string("[diagnostics]\ndisabled = FAKE999, W111\n")
        result = get_all_settings(config)
        assert result["diagnostics"]["FAKE999"] is False  # stored; validated by server
        assert result["diagnostics"]["W111"] is False

    def test_empty_patterns(self):
        config = _config_from_string("[diagnostics]\ngeneric_variable_patterns =\n")
        result = get_all_settings(config)
        assert "diagnostics" not in result  # no patterns, no disabled = empty

    def test_non_integer_line_length_ignored(self):
        config = _config_from_string("[style]\nline_length = abc\n")
        result = get_all_settings(config)
        assert "style" not in result

    def test_invalid_optimiser_bool_ignored(self):
        config = _config_from_string("[optimiser]\nenabled = maybe\n")
        result = get_all_settings(config)
        assert "optimiser" not in result


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


class TestXDGConfigHome:
    """Tests for $XDG_CONFIG_HOME override."""

    def test_xdg_config_home_override(self, tmp_path, monkeypatch):
        """load_user_config() reads from $XDG_CONFIG_HOME/tcl-lsp/config.ini."""
        config_dir = tmp_path / "tcl-lsp"
        config_dir.mkdir()
        ini = config_dir / "config.ini"
        ini.write_text("[diagnostics]\ndisabled = W111\n", encoding="utf-8")

        monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
        config = load_user_config()
        result = get_all_settings(config)
        assert result["diagnostics"]["W111"] is False


class TestPlatformConfigDir:
    """Tests for platform-native _config_dir() behaviour."""

    def test_xdg_config_home_wins_on_any_platform(self, tmp_path, monkeypatch):
        """$XDG_CONFIG_HOME always takes precedence regardless of platform."""
        monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))
        assert _config_dir() == tmp_path / "tcl-lsp"

    def test_linux_default(self, monkeypatch):
        """Linux/BSD uses ~/.config/tcl-lsp when $XDG_CONFIG_HOME is unset."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "linux")
        result = _config_dir()
        assert result == Path.home() / ".config" / "tcl-lsp"

    def test_macos_default(self, monkeypatch):
        """macOS uses ~/Library/Application Support/tcl-lsp."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "darwin")
        result = _config_dir()
        assert result == Path.home() / "Library" / "Application Support" / "tcl-lsp"

    def test_windows_native_uses_appdata(self, tmp_path, monkeypatch):
        """Native Windows uses %APPDATA%/tcl-lsp."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.delenv("MSYSTEM", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.setenv("APPDATA", str(tmp_path))
        result = _config_dir()
        assert result == tmp_path / "tcl-lsp"

    def test_windows_msys2_uses_xdg(self, monkeypatch):
        """Windows with MSYSTEM set (MSYS2 shell) uses XDG default."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.setenv("MSYSTEM", "UCRT64")
        result = _config_dir()
        assert result == Path.home() / ".config" / "tcl-lsp"

    def test_cygwin_uses_xdg(self, monkeypatch):
        """Cygwin uses XDG default."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "cygwin")
        result = _config_dir()
        assert result == Path.home() / ".config" / "tcl-lsp"

    def test_msys_platform_uses_xdg(self, monkeypatch):
        """MSYS2-native Python (sys.platform == 'msys') uses XDG default."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "msys")
        result = _config_dir()
        assert result == Path.home() / ".config" / "tcl-lsp"

    def test_freebsd_uses_xdg(self, monkeypatch):
        """FreeBSD uses XDG default."""
        monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "freebsd13")
        result = _config_dir()
        assert result == Path.home() / ".config" / "tcl-lsp"


class TestPlatformCacheDir:
    """Tests for platform-native _cache_dir() behaviour."""

    def test_xdg_cache_home_wins_on_any_platform(self, tmp_path, monkeypatch):
        """$XDG_CACHE_HOME always takes precedence regardless of platform."""
        monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path))
        assert _cache_dir() == tmp_path / "tcl-lsp"

    def test_linux_default(self, monkeypatch):
        """Linux/BSD uses ~/.cache/tcl-lsp when $XDG_CACHE_HOME is unset."""
        monkeypatch.delenv("XDG_CACHE_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "linux")
        result = _cache_dir()
        assert result == Path.home() / ".cache" / "tcl-lsp"

    def test_macos_default(self, monkeypatch):
        """macOS uses ~/Library/Caches/tcl-lsp."""
        monkeypatch.delenv("XDG_CACHE_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "darwin")
        result = _cache_dir()
        assert result == Path.home() / "Library" / "Caches" / "tcl-lsp"

    def test_windows_native_uses_localappdata(self, tmp_path, monkeypatch):
        """Native Windows uses %LOCALAPPDATA%/tcl-lsp/Cache."""
        monkeypatch.delenv("XDG_CACHE_HOME", raising=False)
        monkeypatch.delenv("MSYSTEM", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.setenv("LOCALAPPDATA", str(tmp_path))
        result = _cache_dir()
        assert result == tmp_path / "tcl-lsp" / "Cache"

    def test_windows_msys2_uses_xdg(self, monkeypatch):
        """Windows with MSYSTEM set (MSYS2 shell) uses XDG default."""
        monkeypatch.delenv("XDG_CACHE_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.setenv("MSYSTEM", "UCRT64")
        result = _cache_dir()
        assert result == Path.home() / ".cache" / "tcl-lsp"

    def test_cache_and_config_are_independent(self, tmp_path, monkeypatch):
        """Setting XDG_CONFIG_HOME does not affect _cache_dir()."""
        monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "cfg"))
        monkeypatch.delenv("XDG_CACHE_HOME", raising=False)
        monkeypatch.setattr("core.common.user_config.sys.platform", "linux")
        assert _cache_dir() == Path.home() / ".cache" / "tcl-lsp"
        assert _config_dir() == tmp_path / "cfg" / "tcl-lsp"


class TestIsPosixCompatWindows:
    """Tests for MSYS2/Cygwin detection."""

    def test_msys_platform(self, monkeypatch):
        monkeypatch.setattr("core.common.user_config.sys.platform", "msys")
        assert _is_posix_compat_windows() is True

    def test_cygwin_platform(self, monkeypatch):
        monkeypatch.setattr("core.common.user_config.sys.platform", "cygwin")
        assert _is_posix_compat_windows() is True

    def test_win32_with_msystem(self, monkeypatch):
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.setenv("MSYSTEM", "MINGW64")
        assert _is_posix_compat_windows() is True

    def test_win32_without_msystem(self, monkeypatch):
        monkeypatch.setattr("core.common.user_config.sys.platform", "win32")
        monkeypatch.delenv("MSYSTEM", raising=False)
        assert _is_posix_compat_windows() is False

    def test_linux_not_posix_compat_windows(self, monkeypatch):
        monkeypatch.setattr("core.common.user_config.sys.platform", "linux")
        assert _is_posix_compat_windows() is False


class TestSaveSettingsRoundtrip:
    """Additional roundtrip tests for save_settings_to_config()."""

    def test_roundtrip_disabled_diagnostics(self, tmp_path, monkeypatch):
        """Disabled diagnostics survive a save/load roundtrip."""
        monkeypatch.setattr(
            "core.common.user_config._config_path",
            lambda: tmp_path / "config.ini",
        )

        settings = {"diagnostics": {"W100": False, "E002": False}}
        save_settings_to_config(settings, only_non_default=False)

        config = load_user_config()
        result = get_all_settings(config)
        assert result["diagnostics"]["W100"] is False
        assert result["diagnostics"]["E002"] is False

    def test_save_only_non_default_with_defaults_dict(self, tmp_path, monkeypatch):
        """When defaults are provided, only differing values are written."""
        monkeypatch.setattr(
            "core.common.user_config._config_path",
            lambda: tmp_path / "config.ini",
        )

        defaults = {"features": {"hover": True, "completion": True}}
        settings = {"features": {"hover": True, "completion": False}}
        save_settings_to_config(settings, only_non_default=True, defaults=defaults)

        config = load_user_config()
        result = get_all_settings(config)
        # hover matches default, should not be written
        assert "hover" not in result.get("features", {})
        # completion differs, should be written
        assert result["features"]["completion"] is False
