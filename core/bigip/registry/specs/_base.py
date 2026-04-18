"""Per-package registry for BIG-IP object definitions."""

from __future__ import annotations

from ..models import BigipObjectSpec

_REGISTRY: list[BigipObjectSpec] = []


def register(spec_factory):
    """Decorator that registers an object spec at import time."""
    _REGISTRY.append(spec_factory())
    return spec_factory
