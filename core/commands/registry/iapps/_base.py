"""Per-package registry for iApps and tmsh command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401

_IAPPS_ONLY = frozenset({"f5-iapps"})
# tmsh commands are available in both iApps and standalone tmsh scripts.
_TMSH_DIALECTS = frozenset({"f5-iapps", "f5-tmsh"})

_REGISTRY, register = make_registry()
