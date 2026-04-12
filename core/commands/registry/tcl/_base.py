"""Per-package registry for Tcl command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401

_REGISTRY, register = make_registry()
