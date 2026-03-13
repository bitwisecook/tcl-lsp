"""Shared known-command cache used by parser recovery logic."""

from __future__ import annotations

from functools import lru_cache


@lru_cache(maxsize=1)
def known_command_names() -> frozenset[str]:
    """Return known command names from the command registry."""
    from ..commands.registry import REGISTRY

    return frozenset(REGISTRY.command_names())
