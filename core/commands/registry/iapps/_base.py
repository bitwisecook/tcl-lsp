"""Per-package registry for iApps command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401

_IAPPS_ONLY = frozenset({"f5-iapps"})

_REGISTRY, register = make_registry()
