"""Tcl command definitions -- one class per command.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

import builtins
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec
    from ..taint_hints import TaintHint

# Import command modules to trigger @register decorators
from . import (
    after,  # noqa: F401
    append,  # noqa: F401
    apply,  # noqa: F401
    array,  # noqa: F401
    binary,  # noqa: F401
    break_,  # noqa: F401
    catch,  # noqa: F401
    cd,  # noqa: F401
    chan,  # noqa: F401
    clock,  # noqa: F401
    close_,  # noqa: F401
    concat,  # noqa: F401
    continue_,  # noqa: F401
    coroutine,  # noqa: F401
    dict,  # noqa: F401
    disabled_in_irules,  # noqa: F401
    encoding_,  # noqa: F401
    eof,  # noqa: F401
    error,  # noqa: F401
    eval,  # noqa: F401
    exec_,  # noqa: F401
    exit,  # noqa: F401
    expr_,  # noqa: F401
    fblocked,  # noqa: F401
    fconfigure_,  # noqa: F401
    fcopy,  # noqa: F401
    file,  # noqa: F401
    fileevent,  # noqa: F401
    flush,  # noqa: F401
    for_,  # noqa: F401
    foreach_,  # noqa: F401
    format,  # noqa: F401
    gets,  # noqa: F401
    glob_,  # noqa: F401
    global_,  # noqa: F401
    if_,  # noqa: F401
    incr,  # noqa: F401
    info,  # noqa: F401
    interp,  # noqa: F401
    join,  # noqa: F401
    lappend,  # noqa: F401
    lassign,  # noqa: F401
    lindex,  # noqa: F401
    linsert,  # noqa: F401
    list,  # noqa: F401
    llength,  # noqa: F401
    lmap,  # noqa: F401
    load,  # noqa: F401
    lrange,  # noqa: F401
    lrepeat,  # noqa: F401
    lreplace,  # noqa: F401
    lreverse,  # noqa: F401
    lsearch_,  # noqa: F401
    lset,  # noqa: F401
    lsort_,  # noqa: F401
    namespace,  # noqa: F401
    oo_class,  # noqa: F401
    oo_copy,  # noqa: F401
    oo_define,  # noqa: F401
    oo_objdefine,  # noqa: F401
    oo_object,  # noqa: F401
    open_,  # noqa: F401
    package,  # noqa: F401
    parray,  # noqa: F401
    pid,  # noqa: F401
    proc_,  # noqa: F401
    puts_,  # noqa: F401
    read,  # noqa: F401
    regexp_,  # noqa: F401
    regsub_,  # noqa: F401
    rename,  # noqa: F401
    return_,  # noqa: F401
    scan,  # noqa: F401
    seek,  # noqa: F401
    set_,  # noqa: F401
    socket_,  # noqa: F401
    source_,  # noqa: F401
    split,  # noqa: F401
    string,  # noqa: F401
    subst_,  # noqa: F401
    switch_,  # noqa: F401
    tailcall,  # noqa: F401
    tell,  # noqa: F401
    throw,  # noqa: F401
    time,  # noqa: F401
    trace,  # noqa: F401
    try_,  # noqa: F401
    unknown,  # noqa: F401
    unload,  # noqa: F401
    unset,  # noqa: F401
    update,  # noqa: F401
    uplevel,  # noqa: F401
    upvar,  # noqa: F401
    variable,  # noqa: F401
    vwait,  # noqa: F401
    while_,  # noqa: F401
    yield_,  # noqa: F401
    yieldto,  # noqa: F401
)
from ._base import _REGISTRY


def tcl_command_specs() -> tuple[CommandSpec, ...]:
    """Return Tcl command specs from all registered classes."""
    return tuple(cls.spec() for cls in _REGISTRY)


def tcl_taint_hints() -> builtins.dict[str, TaintHint]:
    """Return taint hints from all registered classes."""
    return {cls.name: h for cls in _REGISTRY if (h := cls.taint_hints()) is not None}
