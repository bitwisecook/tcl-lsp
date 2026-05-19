"""F5 BIG-IP configuration file parsing and validation.

Parses ``bigip.conf`` / SCF files and cross-references iRules against
the configuration objects they use (data-groups, pools, profiles, etc.).

Public API
----------
parse_bigip_conf(source)
    Parse a configuration file and return a :class:`BigipConfig`.

validate_bigip_config(config)
    Run all cross-reference validations.

get_bigip_diagnostics(config)
    Validate and return LSP-ready diagnostics.
"""

from __future__ import annotations

from .model import BigipConfig
from .object_registry import (
    OBJECT_KIND_SPECS,
    build_bigip_object_registry,
    get_default_bigip_object_registry,
)
from .parser import parse_bigip_conf
from .validator import validate_bigip_config

__all__ = [
    "BigipConfig",
    "OBJECT_KIND_SPECS",
    "build_bigip_object_registry",
    "get_default_bigip_object_registry",
    "parse_bigip_conf",
    "validate_bigip_config",
]
