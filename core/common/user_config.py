"""User-level configuration stored in a platform-native config file.

The config file location follows platform conventions:

- **Linux / BSD / WSL2**: ``~/.config/tcl-lsp/config.ini``
  (or ``$XDG_CONFIG_HOME/tcl-lsp/config.ini``)
- **macOS**: ``~/Library/Application Support/tcl-lsp/config.ini``
- **Windows** (native): ``%APPDATA%\\tcl-lsp\\config.ini``
- **MSYS2 / Cygwin**: ``~/.config/tcl-lsp/config.ini`` (XDG convention)

Setting ``$XDG_CONFIG_HOME`` always takes precedence on every platform.

Uses Python's built-in :mod:`configparser` module (INI format).

Example ``config.ini``::

    [diagnostics]
    # Regex patterns matching generic static:: variable bare names.
    # One pattern per line; matched case-insensitively.
    generic_variable_patterns =
        ^debug(_level|_enabled)?$
        ^dbg$
        ^log_(level|server|enabled)$

    # Comma-separated diagnostic codes to disable.
    disabled = W111, T100

    [optimiser]
    enabled = true
    # Profile: off, readability, standard, full, aggressive.
    profile = readability
    # Comma-separated optimisation codes to disable (overrides profile).
    disabled = O109, O126

    [shimmer]
    enabled = true

    [xcDiagnostics]
    enabled = false

    [features]
    hover = true
    completion = true
    diagnostics = true
    formatting = true
    semanticTokens = true
    codeActions = true
    definition = true
    references = true
    documentSymbols = true
    folding = true
    rename = true
    signatureHelp = true
    workspaceSymbols = true
    inlayHints = false
    callHierarchy = true
    documentLinks = true
    selectionRange = true

    [formatting]
    indent_size = 4
    indent_style = spaces
    brace_style = k_and_r
    max_line_length = 120
    goal_line_length = 100

    [style]
    line_length = 120
"""

from __future__ import annotations

import configparser
import logging
import os
import sys
from pathlib import Path

log = logging.getLogger(__name__)


def _is_posix_compat_windows() -> bool:
    """Detect MSYS2, Cygwin, or similar POSIX compatibility layers on Windows."""
    if sys.platform in ("msys", "cygwin"):
        return True
    # Native Windows Python running inside an MSYS2 shell.
    if sys.platform == "win32" and os.environ.get("MSYSTEM"):
        return True
    return False


def _config_dir() -> Path:
    """Return the config directory using platform-native conventions.

    ``$XDG_CONFIG_HOME`` always takes precedence when set.  Otherwise:

    - **Linux / BSD / WSL2**: ``~/.config/tcl-lsp``
    - **macOS**: ``~/Library/Application Support/tcl-lsp``
    - **Windows** (native): ``%APPDATA%/tcl-lsp``
    - **MSYS2 / Cygwin**: ``~/.config/tcl-lsp``
    """
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if xdg:
        return Path(xdg) / "tcl-lsp"

    # Native Windows (not MSYS2/Cygwin) → %APPDATA%
    if sys.platform == "win32" and not _is_posix_compat_windows():
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "tcl-lsp"

    # macOS → ~/Library/Application Support
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "tcl-lsp"

    # Linux, BSD, WSL2, MSYS2, Cygwin — XDG default
    return Path.home() / ".config" / "tcl-lsp"


def _cache_dir() -> Path:
    """Return the cache directory using platform-native conventions.

    ``$XDG_CACHE_HOME`` always takes precedence when set.  Otherwise:

    - **Linux / BSD / WSL2**: ``~/.cache/tcl-lsp``
    - **macOS**: ``~/Library/Caches/tcl-lsp``
    - **Windows** (native): ``%LOCALAPPDATA%/tcl-lsp/Cache``
    - **MSYS2 / Cygwin**: ``~/.cache/tcl-lsp``

    The cache directory is used for large, regeneratable data such as the
    tclpkg content-addressable store and the cached package registry.  Do
    not store configuration or user preferences here.
    """
    xdg = os.environ.get("XDG_CACHE_HOME")
    if xdg:
        return Path(xdg) / "tcl-lsp"

    # Native Windows (not MSYS2/Cygwin) → %LOCALAPPDATA%\tcl-lsp\Cache
    if sys.platform == "win32" and not _is_posix_compat_windows():
        local = os.environ.get("LOCALAPPDATA")
        if local:
            return Path(local) / "tcl-lsp" / "Cache"

    # macOS → ~/Library/Caches
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Caches" / "tcl-lsp"

    # Linux, BSD, WSL2, MSYS2, Cygwin — XDG default
    return Path.home() / ".cache" / "tcl-lsp"


def _config_path() -> Path:
    """Return the path to the user config file."""
    return _config_dir() / "config.ini"


def load_user_config() -> configparser.ConfigParser:
    """Load the user configuration file.

    Returns a :class:`configparser.ConfigParser` instance.  If the file
    does not exist, returns an empty configuration.
    """
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment, invalid-assignment]  # preserve camelCase keys
    path = _config_path()
    if path.is_file():
        try:
            config.read(str(path), encoding="utf-8")
            log.info("Loaded user config from %s", path)
        except Exception:
            log.warning("Failed to parse %s, using defaults", path, exc_info=True)
    return config


def get_generic_variable_patterns(
    config: configparser.ConfigParser | None = None,
) -> list[str] | None:
    """Extract generic variable patterns from the user config.

    Returns ``None`` if no user override is configured (use defaults).
    Returns a list of regex pattern strings if the user has specified
    custom patterns.
    """
    if config is None:
        config = load_user_config()
    if not config.has_option("diagnostics", "generic_variable_patterns"):
        return None
    raw = config.get("diagnostics", "generic_variable_patterns", fallback="")
    patterns = [line.strip() for line in raw.splitlines() if line.strip()]
    return patterns if patterns else None


def _parse_comma_list(raw: str) -> list[str]:
    """Split a comma-or-whitespace-separated string into stripped tokens.

    Supports ``"O109, O126"``, ``"W111 T100"``, and multi-line lists.
    """
    # Normalise commas to spaces, then split on any whitespace.
    tokens = raw.replace(",", " ").split()
    return [token.strip() for token in tokens if token.strip()]


def _parse_bool(value: str) -> bool | None:
    """Parse a boolean string, returning ``None`` on unrecognised input."""
    lower = value.strip().lower()
    if lower in ("true", "yes", "1", "on"):
        return True
    if lower in ("false", "no", "0", "off"):
        return False
    return None


def get_all_settings(
    config: configparser.ConfigParser | None = None,
) -> dict:
    """Build a settings dict from all config file sections.

    The returned dict uses the same shape as ``_extract_tcl_lsp_settings``
    output in the LSP server, so it can be passed directly to
    ``_apply_feature_settings`` and ``FormatterConfig.from_dict``.

    Only keys that are explicitly set in the config file are included;
    missing sections or keys are omitted so that built-in defaults are
    preserved.
    """
    if config is None:
        config = load_user_config()

    result: dict[str, object] = {}

    # [diagnostics]
    if config.has_section("diagnostics"):
        diag: dict[str, object] = {}
        # ``disabled`` key: comma-separated codes → set each to False.
        if config.has_option("diagnostics", "disabled"):
            raw = config.get("diagnostics", "disabled", fallback="")
            for code in _parse_comma_list(raw):
                diag[code] = False
        # Delegate to ``get_generic_variable_patterns`` (single implementation).
        patterns = get_generic_variable_patterns(config)
        if patterns is not None:
            diag["genericVariablePatterns"] = patterns
        if diag:
            result["diagnostics"] = diag

    # [optimiser]
    if config.has_section("optimiser"):
        opt: dict[str, object] = {}
        if config.has_option("optimiser", "enabled"):
            val = _parse_bool(config.get("optimiser", "enabled"))
            if val is not None:
                opt["enabled"] = val
        if config.has_option("optimiser", "profile"):
            opt["profile"] = config.get("optimiser", "profile").strip()
        if config.has_option("optimiser", "disabled"):
            raw = config.get("optimiser", "disabled", fallback="")
            for code in _parse_comma_list(raw):
                opt[code] = False
        if opt:
            result["optimiser"] = opt

    # [shimmer]
    if config.has_section("shimmer"):
        shim: dict[str, object] = {}
        if config.has_option("shimmer", "enabled"):
            val = _parse_bool(config.get("shimmer", "enabled"))
            if val is not None:
                shim["enabled"] = val
        if shim:
            result["shimmer"] = shim

    # [xcDiagnostics]
    if config.has_section("xcDiagnostics"):
        xc: dict[str, object] = {}
        if config.has_option("xcDiagnostics", "enabled"):
            val = _parse_bool(config.get("xcDiagnostics", "enabled"))
            if val is not None:
                xc["enabled"] = val
        if xc:
            result["xcDiagnostics"] = xc

    # [features]
    if config.has_section("features"):
        feat: dict[str, object] = {}
        for key in config.options("features"):
            raw_val = config.get("features", key)
            val = _parse_bool(raw_val)
            if val is not None:
                feat[key] = val
        if feat:
            result["features"] = feat

    # [formatting]
    if config.has_section("formatting"):
        fmt: dict[str, object] = {}
        for key in config.options("formatting"):
            raw_val = config.get("formatting", key)
            # Try integer first, then boolean, then keep as string.
            try:
                fmt[key] = int(raw_val)
            except ValueError:
                bool_val = _parse_bool(raw_val)
                if bool_val is not None:
                    fmt[key] = bool_val
                else:
                    fmt[key] = raw_val.strip()
        if fmt:
            result["formatting"] = fmt

    # [style]
    if config.has_section("style"):
        sty: dict[str, object] = {}
        if config.has_option("style", "line_length"):
            try:
                sty["lineLength"] = int(config.get("style", "line_length"))
            except ValueError:
                pass
        if sty:
            result["style"] = sty

    return result


def save_settings_to_config(
    settings: dict,
    *,
    only_non_default: bool = True,
    defaults: dict | None = None,
) -> str:
    """Write a settings dict to the platform-native config file.

    When *only_non_default* is ``True`` (the default), only settings that
    differ from *defaults* are written.  This keeps the file minimal.

    Returns the path that was written.
    """
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment, invalid-assignment]  # preserve case

    if defaults is None:
        defaults = {}

    def _differs(section: str, key: str, value: object) -> bool:
        if not only_non_default:
            return True
        default_section = defaults.get(section)
        if not isinstance(default_section, dict):
            return True
        return default_section.get(key) != value

    # diagnostics
    diag = settings.get("diagnostics")
    if isinstance(diag, dict):
        disabled_codes = [k for k, v in diag.items() if v is False]
        patterns = diag.get("genericVariablePatterns")
        if disabled_codes or patterns:
            config.add_section("diagnostics")
        if disabled_codes:
            config.set("diagnostics", "disabled", ", ".join(sorted(disabled_codes)))
        if isinstance(patterns, list) and patterns:
            config.set(
                "diagnostics",
                "generic_variable_patterns",
                "\n    " + "\n    ".join(patterns),
            )

    # optimiser
    opt = settings.get("optimiser")
    if isinstance(opt, dict):
        items: list[tuple[str, str]] = []
        enabled = opt.get("enabled")
        if isinstance(enabled, bool) and _differs("optimiser", "enabled", enabled):
            items.append(("enabled", str(enabled).lower()))
        profile = opt.get("profile")
        if isinstance(profile, str) and _differs("optimiser", "profile", profile):
            items.append(("profile", profile))
        # Only write per-code disables that differ from the profile baseline
        # so the exported config stays minimal.
        _NON_CODE_KEYS = {"enabled", "profile"}
        disabled_opts = [k for k, v in opt.items() if k not in _NON_CODE_KEYS and v is False]
        if disabled_opts:
            items.append(("disabled", ", ".join(sorted(disabled_opts))))
        if items:
            config.add_section("optimiser")
            for k, v in items:
                config.set("optimiser", k, v)

    # shimmer
    shim = settings.get("shimmer")
    if isinstance(shim, dict):
        enabled = shim.get("enabled")
        if isinstance(enabled, bool) and _differs("shimmer", "enabled", enabled):
            config.add_section("shimmer")
            config.set("shimmer", "enabled", str(enabled).lower())

    # xcDiagnostics
    xc = settings.get("xcDiagnostics")
    if isinstance(xc, dict):
        enabled = xc.get("enabled")
        if isinstance(enabled, bool) and _differs("xcDiagnostics", "enabled", enabled):
            config.add_section("xcDiagnostics")
            config.set("xcDiagnostics", "enabled", str(enabled).lower())

    # features
    feat = settings.get("features")
    if isinstance(feat, dict):
        feat_items = [
            (k, v) for k, v in feat.items() if isinstance(v, bool) and _differs("features", k, v)
        ]
        if feat_items:
            config.add_section("features")
            for k, v in sorted(feat_items):
                config.set("features", k, str(v).lower())

    # formatting
    fmt = settings.get("formatting")
    if isinstance(fmt, dict):
        fmt_items = [(k, v) for k, v in fmt.items() if _differs("formatting", k, v)]
        if fmt_items:
            config.add_section("formatting")
            for k, v in sorted(fmt_items):
                config.set("formatting", k, str(v) if not isinstance(v, bool) else str(v).lower())

    # style
    sty = settings.get("style")
    if isinstance(sty, dict):
        ll = sty.get("lineLength")
        if isinstance(ll, int) and _differs("style", "lineLength", ll):
            config.add_section("style")
            config.set("style", "line_length", str(ll))

    # Write to file
    path = _config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write("# tcl-lsp configuration — generated by tcl-lsp.exportConfig\n")
        f.write("# See docs/kcs/kcs-xdg-config.md for reference.\n\n")
        config.write(f)

    log.info("Wrote settings to %s", path)
    return str(path)
