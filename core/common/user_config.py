"""User-level configuration from ``~/.config/tcl-lsp/config.ini``.

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
    disabled = W111, T100, IRULE1005

    [optimiser]
    enabled = true
    # Comma-separated optimisation codes to disable.
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
    inlayHints = true
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
import json
import logging
import os
from pathlib import Path

log = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Diagnostic manifest — single source of truth for all user-configurable
# diagnostic and optimisation codes.  Loaded once at import time.
# ---------------------------------------------------------------------------

_MANIFEST_PATH = Path(__file__).resolve().parent / "diagnostic_manifest.json"
_MANIFEST: dict | None = None


def _load_manifest() -> dict:
    global _MANIFEST
    if _MANIFEST is None:
        _MANIFEST = json.loads(_MANIFEST_PATH.read_text(encoding="utf-8"))
    return _MANIFEST


def manifest_diagnostic_codes() -> frozenset[str]:
    """Return all user-configurable diagnostic codes from the manifest."""
    return frozenset(d["code"] for d in _load_manifest()["diagnostics"])


def manifest_optimisation_codes() -> frozenset[str]:
    """Return all user-configurable optimisation codes from the manifest."""
    return frozenset(o["code"] for o in _load_manifest()["optimisations"])


def _config_dir() -> Path:
    """Return the config directory, respecting ``$XDG_CONFIG_HOME``."""
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if xdg:
        return Path(xdg) / "tcl-lsp"
    return Path.home() / ".config" / "tcl-lsp"


def _config_path() -> Path:
    """Return the path to the user config file."""
    return _config_dir() / "config.ini"


def load_user_config() -> configparser.ConfigParser:
    """Load the user configuration file.

    Returns a :class:`configparser.ConfigParser` instance.  If the file
    does not exist, returns an empty configuration.
    """
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment]  # preserve camelCase keys
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
    """Build a settings dict from all XDG config sections.

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

    # -- [diagnostics] -------------------------------------------------------
    if config.has_section("diagnostics"):
        diag: dict[str, object] = {}
        # ``disabled`` key: comma-separated codes → set each to False.
        if config.has_option("diagnostics", "disabled"):
            raw = config.get("diagnostics", "disabled", fallback="")
            for code in _parse_comma_list(raw):
                diag[code] = False
        # ``generic_variable_patterns`` is handled separately by the
        # existing ``get_generic_variable_patterns`` helper — we include
        # it here too so ``_apply_feature_settings`` can pick it up.
        if config.has_option("diagnostics", "generic_variable_patterns"):
            raw = config.get("diagnostics", "generic_variable_patterns", fallback="")
            patterns = [line.strip() for line in raw.splitlines() if line.strip()]
            if patterns:
                diag["genericVariablePatterns"] = patterns
        if diag:
            result["diagnostics"] = diag

    # -- [optimiser] ---------------------------------------------------------
    if config.has_section("optimiser"):
        opt: dict[str, object] = {}
        if config.has_option("optimiser", "enabled"):
            val = _parse_bool(config.get("optimiser", "enabled"))
            if val is not None:
                opt["enabled"] = val
        if config.has_option("optimiser", "disabled"):
            raw = config.get("optimiser", "disabled", fallback="")
            for code in _parse_comma_list(raw):
                opt[code] = False
        if opt:
            result["optimiser"] = opt

    # -- [shimmer] -----------------------------------------------------------
    if config.has_section("shimmer"):
        shim: dict[str, object] = {}
        if config.has_option("shimmer", "enabled"):
            val = _parse_bool(config.get("shimmer", "enabled"))
            if val is not None:
                shim["enabled"] = val
        if shim:
            result["shimmer"] = shim

    # -- [xcDiagnostics] -----------------------------------------------------
    if config.has_section("xcDiagnostics"):
        xc: dict[str, object] = {}
        if config.has_option("xcDiagnostics", "enabled"):
            val = _parse_bool(config.get("xcDiagnostics", "enabled"))
            if val is not None:
                xc["enabled"] = val
        if xc:
            result["xcDiagnostics"] = xc

    # -- [features] ----------------------------------------------------------
    if config.has_section("features"):
        feat: dict[str, object] = {}
        for key in config.options("features"):
            raw_val = config.get("features", key)
            val = _parse_bool(raw_val)
            if val is not None:
                feat[key] = val
        if feat:
            result["features"] = feat

    # -- [formatting] --------------------------------------------------------
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

    # -- [style] -------------------------------------------------------------
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
    """Write a settings dict to ``~/.config/tcl-lsp/config.ini``.

    When *only_non_default* is ``True`` (the default), only settings that
    differ from *defaults* are written.  This keeps the file minimal.

    Returns the path that was written.
    """
    config = configparser.ConfigParser()
    config.optionxform = str  # type: ignore[assignment]  # preserve case

    if defaults is None:
        defaults = {}

    def _differs(section: str, key: str, value: object) -> bool:
        if not only_non_default:
            return True
        default_section = defaults.get(section)
        if not isinstance(default_section, dict):
            return True
        return default_section.get(key) != value

    # -- diagnostics ---------------------------------------------------------
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

    # -- optimiser -----------------------------------------------------------
    opt = settings.get("optimiser")
    if isinstance(opt, dict):
        items: list[tuple[str, str]] = []
        enabled = opt.get("enabled")
        if isinstance(enabled, bool) and (not only_non_default or enabled is not True):
            items.append(("enabled", str(enabled).lower()))
        disabled_opts = [k for k, v in opt.items() if k != "enabled" and v is False]
        if disabled_opts:
            items.append(("disabled", ", ".join(sorted(disabled_opts))))
        if items:
            config.add_section("optimiser")
            for k, v in items:
                config.set("optimiser", k, v)

    # -- shimmer -------------------------------------------------------------
    shim = settings.get("shimmer")
    if isinstance(shim, dict):
        enabled = shim.get("enabled")
        if isinstance(enabled, bool) and (not only_non_default or enabled is not True):
            config.add_section("shimmer")
            config.set("shimmer", "enabled", str(enabled).lower())

    # -- xcDiagnostics -------------------------------------------------------
    xc = settings.get("xcDiagnostics")
    if isinstance(xc, dict):
        enabled = xc.get("enabled")
        if isinstance(enabled, bool) and (not only_non_default or enabled is not False):
            config.add_section("xcDiagnostics")
            config.set("xcDiagnostics", "enabled", str(enabled).lower())

    # -- features ------------------------------------------------------------
    feat = settings.get("features")
    if isinstance(feat, dict):
        feat_items = [
            (k, v) for k, v in feat.items() if isinstance(v, bool) and _differs("features", k, v)
        ]
        if feat_items:
            config.add_section("features")
            for k, v in sorted(feat_items):
                config.set("features", k, str(v).lower())

    # -- formatting ----------------------------------------------------------
    fmt = settings.get("formatting")
    if isinstance(fmt, dict):
        fmt_items = [(k, v) for k, v in fmt.items() if _differs("formatting", k, v)]
        if fmt_items:
            config.add_section("formatting")
            for k, v in sorted(fmt_items):
                config.set("formatting", k, str(v) if not isinstance(v, bool) else str(v).lower())

    # -- style ---------------------------------------------------------------
    sty = settings.get("style")
    if isinstance(sty, dict):
        ll = sty.get("lineLength")
        if isinstance(ll, int) and (not only_non_default or ll != 120):
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
