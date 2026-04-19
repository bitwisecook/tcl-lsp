"""Per-package registry for Expect command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401

_EXPECT_ONLY = frozenset({"expect"})

_REGISTRY, register = make_registry()
