"""Dialect-specific command specs for F5 iApps utility package.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    iapp__apm_config,  # noqa: F401
    iapp__conf,  # noqa: F401
    iapp__debug,  # noqa: F401
    iapp__destination,  # noqa: F401
    iapp__downgrade,  # noqa: F401
    iapp__downgrade_template,  # noqa: F401
    iapp__get_items,  # noqa: F401
    iapp__is,  # noqa: F401
    iapp__make_safe_password,  # noqa: F401
    iapp__pool_members,  # noqa: F401
    iapp__substa,  # noqa: F401
    iapp__template,  # noqa: F401
    iapp__tmos_version,  # noqa: F401
    iapp__upgrade,  # noqa: F401
    iapp__upgrade_template,  # noqa: F401
)
from ._base import _REGISTRY


def iapps_command_specs() -> tuple[CommandSpec, ...]:
    """Return iApps-specific command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
