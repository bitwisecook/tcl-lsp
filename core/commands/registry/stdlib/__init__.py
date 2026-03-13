"""Tcl stdlib command definitions -- activated by ``package require``.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    cookiejar,  # noqa: F401
    history_,  # noqa: F401
    http_,  # noqa: F401
    msgcat,  # noqa: F401
    opt,  # noqa: F401
    pkg_utils,  # noqa: F401
    platform_,  # noqa: F401
    safe_,  # noqa: F401
    tcltest,  # noqa: F401
    tm,  # noqa: F401
    utilities,  # noqa: F401
)
from ._base import _REGISTRY


def stdlib_command_specs() -> tuple[CommandSpec, ...]:
    """Return stdlib command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
