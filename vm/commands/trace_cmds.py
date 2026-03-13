"""The ``trace`` command — variable traces for tcltest support."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..machine import _list_escape, _split_list
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_trace(interp: TclInterp, args: list[str]) -> TclResult:
    """trace add|remove|info variable ..."""
    if len(args) < 2:
        raise TclError('wrong # args: should be "trace option ?arg ...?"')

    action = args[0]
    kind = args[1] if len(args) > 1 else ""

    # Accept abbreviations of "variable" (e.g. "var", "vari", etc.)
    is_variable = kind == "variable" or (len(kind) >= 1 and "variable".startswith(kind))

    match action:
        case "add":
            if not is_variable or len(args) < 5:
                raise TclError('wrong # args: should be "trace add variable name ops command"')
            var_name = args[2]
            ops = _split_list(args[3])
            script = args[4]
            traces = interp.variable_traces.setdefault(var_name, [])
            traces.append((ops, script))
            return TclResult()

        case "remove":
            if not is_variable or len(args) < 5:
                raise TclError('wrong # args: should be "trace remove variable name ops command"')
            var_name = args[2]
            ops = _split_list(args[3])
            script = args[4]
            traces = interp.variable_traces.get(var_name, [])
            for i, (t_ops, t_script) in enumerate(traces):
                if t_ops == ops and t_script == script:
                    traces.pop(i)
                    break
            return TclResult()

        case "info":
            if not is_variable or len(args) < 3:
                raise TclError('wrong # args: should be "trace info variable name"')
            var_name = args[2]
            traces = interp.variable_traces.get(var_name, [])
            items: list[str] = []
            for ops, script in traces:
                # Each trace spec is a two-element list: {opsList command}
                ops_str = " ".join(ops)
                spec = f"{ops_str} {script}"
                items.append(_list_escape(spec))
            return TclResult(value=" ".join(items))

        case _:
            raise TclError(f'bad option "{action}": must be add, info, or remove')


def fire_traces(
    interp: TclInterp,
    var_name: str,
    op: str,
    *,
    lookup_key: str | None = None,
) -> None:
    """Fire any registered traces for *var_name* and *op* ('read'|'write'|'unset').

    *var_name* may be a plain scalar (``x``), a qualified name
    (``::tcltest::testConstraints``), or an array element
    (``testConstraints(singleTestInterp)``).  Traces are registered on the
    base array name, so we split out name1/name2 and look up by name1.

    When *lookup_key* is provided, it is used to find the trace
    registrations instead of *var_name*.  This is needed when a variable
    is accessed through an upvar alias: the alias name is passed as
    *var_name* (used in the callback) while the resolved target name is
    passed as *lookup_key* (used to find traces).

    Traces on a given variable are disabled while their callback is
    executing (matching real Tcl behaviour) to prevent infinite recursion.
    """
    from ..scope import parse_array_ref

    name1, name2 = parse_array_ref(var_name)

    # Determine which key to use for trace lookup
    if lookup_key is not None:
        trace_base, _trace_elem = parse_array_ref(lookup_key)
    else:
        trace_base = name1

    traces = interp.variable_traces.get(trace_base)
    if traces is None:
        return

    # Guard against re-entrant trace firing (Tcl disables traces during
    # callback execution for the same variable).
    active: set[str] = getattr(interp, "_active_traces", None) or set()
    if not hasattr(interp, "_active_traces"):
        interp._active_traces = active  # type: ignore[attr-defined]
    guard_key = f"{trace_base}\0{op}"
    if guard_key in active:
        return
    active.add(guard_key)

    name2_str = _list_escape(name2) if name2 is not None else "{}"
    try:
        for ops, script in list(traces):
            if op in ops:
                # Tcl trace callbacks receive: name1 name2 op
                callback = f"{script} {_list_escape(name1)} {name2_str} {op}"
                try:
                    interp.eval(callback)
                except Exception as exc:
                    if op == "write":
                        # Write trace errors reject the set — propagate
                        # with the Tcl-standard "can't set" prefix.
                        raise TclError(f'can\'t set "{var_name}": {exc}') from None
                    # Read/unset trace errors are silently ignored
    finally:
        active.discard(guard_key)


def register() -> None:
    """Register trace command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("trace", _cmd_trace)
