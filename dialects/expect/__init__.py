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

"""Dialect-specific command specs for Expect (interactive process automation).

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.registry.models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    close_,  # noqa: F401
    debug_,  # noqa: F401
    disconnect,  # noqa: F401
    exit_,  # noqa: F401
    exp_continue,  # noqa: F401
    exp_internal,  # noqa: F401
    exp_pid,  # noqa: F401
    exp_version,  # noqa: F401
    expect_,  # noqa: F401
    expect_after,  # noqa: F401
    expect_background,  # noqa: F401
    expect_before,  # noqa: F401
    expect_tty,  # noqa: F401
    expect_user,  # noqa: F401
    fork_,  # noqa: F401
    interact,  # noqa: F401
    log_file,  # noqa: F401
    log_user,  # noqa: F401
    match_max,  # noqa: F401
    overlay,  # noqa: F401
    parity_,  # noqa: F401
    remove_nulls,  # noqa: F401
    send_,  # noqa: F401
    send_error,  # noqa: F401
    send_log,  # noqa: F401
    send_tty,  # noqa: F401
    send_user,  # noqa: F401
    sleep_,  # noqa: F401
    spawn_,  # noqa: F401
    strace,  # noqa: F401
    stty,  # noqa: F401
    system,  # noqa: F401
    timestamp_,  # noqa: F401
    trap_,  # noqa: F401
    wait_,  # noqa: F401
)
from ._base import _REGISTRY


def expect_command_specs() -> tuple[CommandSpec, ...]:
    """Return Expect-specific command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)
