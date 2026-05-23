"""Dialect-specific command specs for F5 iApps utility package.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.registry.models import CommandSpec

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
    script__help,  # noqa: F401
    script__init,  # noqa: F401
    script__run,  # noqa: F401
    script__tabc,  # noqa: F401
    tmsh__add_help,  # noqa: F401
    tmsh__add_tabc,  # noqa: F401
    tmsh__begin_transaction,  # noqa: F401
    tmsh__builtin_help,  # noqa: F401
    tmsh__builtin_tabc,  # noqa: F401
    tmsh__cancel_transaction,  # noqa: F401
    tmsh__cd,  # noqa: F401
    tmsh__clear_screen,  # noqa: F401
    tmsh__commit_transaction,  # noqa: F401
    tmsh__create,  # noqa: F401
    tmsh__delete,  # noqa: F401
    tmsh__display,  # noqa: F401
    tmsh__display_threshold,  # noqa: F401
    tmsh__get_config,  # noqa: F401
    tmsh__get_field_names,  # noqa: F401
    tmsh__get_field_value,  # noqa: F401
    tmsh__get_name,  # noqa: F401
    tmsh__get_status,  # noqa: F401
    tmsh__get_type,  # noqa: F401
    tmsh__include,  # noqa: F401
    tmsh__list,  # noqa: F401
    tmsh__log,  # noqa: F401
    tmsh__log_dest,  # noqa: F401
    tmsh__log_level,  # noqa: F401
    tmsh__modify,  # noqa: F401
    tmsh__pwd,  # noqa: F401
    tmsh__reset_stats,  # noqa: F401
    tmsh__show,  # noqa: F401
    tmsh__stateless,  # noqa: F401
    tmsh__version,  # noqa: F401
)
from ._base import _REGISTRY


def iapps_command_specs() -> tuple[CommandSpec, ...]:
    """Return iApps-specific command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
