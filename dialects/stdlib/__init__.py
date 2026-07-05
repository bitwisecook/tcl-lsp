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

"""Tcl stdlib command definitions -- activated by ``package require``.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.registry.models import CommandSpec

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
    tcltest_cmds,  # noqa: F401
    tm,  # noqa: F401
    utilities,  # noqa: F401
)
from ._base import _REGISTRY


def stdlib_command_specs() -> tuple[CommandSpec, ...]:
    """Return stdlib command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
