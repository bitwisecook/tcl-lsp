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
        package_cmds,
        proc_cmds,
        regexp_cmds,
        string_cmds,
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
    trace_cmds.register()
    clock_cmds.register()
