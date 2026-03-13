"""Tcllib command definitions -- one class per command or namespace.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    base64_,  # noqa: F401
    cmdline,  # noqa: F401
    csv_,  # noqa: F401
    dns_,  # noqa: F401
    fileutil,  # noqa: F401
    html_,  # noqa: F401
    ip,  # noqa: F401
    json_,  # noqa: F401
    logger,  # noqa: F401
    math_statistics,  # noqa: F401
    md5_,  # noqa: F401
    mime_,  # noqa: F401
    sha,  # noqa: F401
    smtp_,  # noqa: F401
    snit,  # noqa: F401
    struct_list,  # noqa: F401
    struct_queue,  # noqa: F401
    struct_set,  # noqa: F401
    struct_stack,  # noqa: F401
    textutil,  # noqa: F401
    uri,  # noqa: F401
    uuid_,  # noqa: F401
    yaml_,  # noqa: F401
)
from ._base import _REGISTRY


def tcllib_command_specs() -> tuple[CommandSpec, ...]:
    """Return tcllib command specs from all registered classes.

    Each spec's ``required_package`` is set from ``tcllib_package`` so the
    upstream ``supports_packages()`` filtering gates these commands on the
    corresponding ``package require`` statement.
    """
    from dataclasses import replace

    specs: list[CommandSpec] = []
    for cls in _REGISTRY:
        spec = cls.spec()
        if spec.tcllib_package and not spec.required_package:
            spec = replace(spec, required_package=spec.tcllib_package)
        specs.append(spec)
    return tuple(specs)
