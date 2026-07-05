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

"""VM command registration — registers built-in Tcl command handlers on the global REGISTRY."""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING

from ..types import TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

CommandHandler = Callable[["TclInterp", list[str]], TclResult]


def register_builtins() -> None:
    """Register all built-in Tcl command handlers on the global REGISTRY."""
    from . import (
        array_cmds,
        binary_cmds,
        clock_cmds,
        control,
        core,
        dict_cmds,
        encoding_cmds,
        expr_cmd,
        file_cmds,
        format_cmds,
        info_cmds,
        interp_cmds,
        io,
        list_cmds,
        math_cmds,
        namespace_cmds,
        oo_cmds,
        package_cmds,
        proc_cmds,
        regexp_cmds,
        string_cmds,
        tm_cmds,
        trace_cmds,
    )

    core.register()
    io.register()
    expr_cmd.register()
    proc_cmds.register()
    control.register()
    info_cmds.register()
    string_cmds.register()
    list_cmds.register()
    math_cmds.register()
    regexp_cmds.register()
    format_cmds.register()
    dict_cmds.register()
    array_cmds.register()
    namespace_cmds.register()
    package_cmds.register()
    file_cmds.register()
    interp_cmds.register()
    encoding_cmds.register()
    binary_cmds.register()
    tm_cmds.register()
    trace_cmds.register()
    clock_cmds.register()
    oo_cmds.register()
