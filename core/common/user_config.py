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
        ^logging$
        ^verbose$
        ^trace$
        ^(response_)?timeout$
        ^(max_)?retr(y|ies)$
        ^config$
        ^(enabled|disabled|active)$
        ^mode$
        ^(port|host|server|pool)$
        ^count(er)?$
        ^(limit|max_connections|threshold|rate|interval)$
        ^(flag|level|status|state|version|name|value|data|result|test|init|default)$
"""

from __future__ import annotations

import configparser
import logging
import os
from pathlib import Path

log = logging.getLogger(__name__)


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
