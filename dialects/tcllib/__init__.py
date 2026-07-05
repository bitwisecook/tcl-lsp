# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tcllib command definitions -- one class per command or namespace.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.registry.models import CommandSpec

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
