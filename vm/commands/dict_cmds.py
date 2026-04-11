"""Dict commands: dict create, get, set, exists, keys, values, etc."""

from __future__ import annotations

import fnmatch
from typing import TYPE_CHECKING

from ..machine import _dict_from_list, _list_escape, _split_list
from ..types import TclBreak, TclContinue, TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


# Dict serialisation helpers


def _dict_to_list(d: dict[str, str]) -> str:
    """Serialise a Python dict to Tcl list format."""
    parts: list[str] = []
    for k, v in d.items():
        parts.append(_list_escape(k))
        parts.append(_list_escape(v))
    return " ".join(parts)


# Nested dict access helpers


def _nested_get(d: dict[str, str], keys: list[str]) -> str:
    """Walk a chain of keys through nested dicts, returning the final value."""
    current: str = _dict_to_list(d)
    for key in keys:
        inner = _dict_from_list(current)
        if key not in inner:
            raise TclError(f'key "{key}" not known in dictionary')
        current = inner[key]
    return current


def _nested_set(d: dict[str, str], keys: list[str], value: str) -> None:
    """Set a value in a nested dict structure."""
    if len(keys) == 1:
        d[keys[0]] = value
        return

    # Navigate to the parent dict and set the final key
    key = keys[0]
    if key in d:
        inner = _dict_from_list(d[key])
    else:
        inner: dict[str, str] = {}
    _nested_set(inner, keys[1:], value)
    d[key] = _dict_to_list(inner)


def _nested_exists(d: dict[str, str], keys: list[str]) -> bool:
    """Check whether a chain of keys exists in nested dicts."""
    current: str = _dict_to_list(d)
    for key in keys:
        try:
            inner = _dict_from_list(current)
        except TclError:
            return False
        if key not in inner:
            return False
        current = inner[key]
    return True


def _nested_unset(d: dict[str, str], keys: list[str]) -> None:
    """Remove a key from a nested dict structure."""
    if len(keys) == 1:
        d.pop(keys[0], None)
        return
    key = keys[0]
    if key not in d:
        raise TclError(f'key "{key}" not known in dictionary')
    inner = _dict_from_list(d[key])
    _nested_unset(inner, keys[1:])
    d[key] = _dict_to_list(inner)


# Dict subcommand handlers


def _cmd_dict(interp: TclInterp, args: list[str]) -> TclResult:
    """dict subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "dict subcommand ?arg ...?"')

    sub = args[0]
    rest = args[1:]

    match sub:
        case "create":
            return _dict_create(interp, rest)
        case "get":
            return _dict_get(interp, rest)
        case "set":
            return _dict_set(interp, rest)
        case "exists":
            return _dict_exists(interp, rest)
        case "keys":
            return _dict_keys(interp, rest)
        case "values":
            return _dict_values(interp, rest)
        case "size":
            return _dict_size(interp, rest)
        case "remove":
            return _dict_remove(interp, rest)
        case "replace":
            return _dict_replace(interp, rest)
        case "merge":
            return _dict_merge(interp, rest)
        case "for":
            return _dict_for(interp, rest)
        case "map":
            return _dict_map(interp, rest)
        case "filter":
            return _dict_filter(interp, rest)
        case "unset":
            return _dict_unset(interp, rest)
        case "append":
            return _dict_append(interp, rest)
        case "lappend":
            return _dict_lappend(interp, rest)
        case "incr":
            return _dict_incr(interp, rest)
        case "info":
            return _dict_info(interp, rest)
        case "update":
            return _dict_update(interp, rest)
        case "with":
            return _dict_with(interp, rest)
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                "append, create, exists, filter, for, get, incr, info, "
                "keys, lappend, map, merge, remove, replace, set, size, "
                "unset, update, values, or with"
            )


def _dict_create(interp: TclInterp, args: list[str]) -> TclResult:
    """dict create ?key value ...?"""
    if len(args) % 2 != 0:
        raise TclError('wrong # args: should be "dict create ?key value ...?"')
    parts: list[str] = []
    for i in range(0, len(args), 2):
        parts.append(_list_escape(args[i]))
        parts.append(_list_escape(args[i + 1]))
    return TclResult(value=" ".join(parts))


def _dict_get(interp: TclInterp, args: list[str]) -> TclResult:
    """dict get dictValue ?key ...?"""
    if not args:
        raise TclError('wrong # args: should be "dict get dictValue ?key ...?"')
    d = _dict_from_list(args[0])
    if len(args) == 1:
        return TclResult(value=args[0])
    return TclResult(value=_nested_get(d, args[1:]))


def _dict_set(interp: TclInterp, args: list[str]) -> TclResult:
    """dict set dictVariable key ?key ...? value"""
    if len(args) < 3:
        raise TclError('wrong # args: should be "dict set dictVariable key ?key ...? value"')
    var_name = args[0]
    keys = args[1:-1]
    value = args[-1]

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    _nested_set(d, keys, value)
    result = _dict_to_list(d)
    interp.current_frame.set_var(var_name, result)
    return TclResult(value=result)


def _dict_exists(interp: TclInterp, args: list[str]) -> TclResult:
    """dict exists dictValue key ?key ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict exists dictValue key ?key ...?"')
    try:
        d = _dict_from_list(args[0])
    except TclError:
        return TclResult(value="0")
    return TclResult(value="1" if _nested_exists(d, args[1:]) else "0")


def _dict_keys(interp: TclInterp, args: list[str]) -> TclResult:
    """dict keys dictValue ?globPattern?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "dict keys dictValue ?globPattern?"')
    d = _dict_from_list(args[0])
    pattern = args[1] if len(args) > 1 else None
    keys = list(d.keys())
    if pattern is not None:
        keys = [k for k in keys if fnmatch.fnmatch(k, pattern)]
    return TclResult(value=" ".join(_list_escape(k) for k in keys))


def _dict_values(interp: TclInterp, args: list[str]) -> TclResult:
    """dict values dictValue ?globPattern?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "dict values dictValue ?globPattern?"')
    d = _dict_from_list(args[0])
    pattern = args[1] if len(args) > 1 else None
    if pattern is not None:
        values = [v for k, v in d.items() if fnmatch.fnmatch(v, pattern)]
    else:
        values = list(d.values())
    return TclResult(value=" ".join(_list_escape(v) for v in values))


def _dict_size(interp: TclInterp, args: list[str]) -> TclResult:
    """dict size dictValue"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "dict size dictValue"')
    d = _dict_from_list(args[0])
    return TclResult(value=str(len(d)))


def _dict_remove(interp: TclInterp, args: list[str]) -> TclResult:
    """dict remove dictValue ?key ...?"""
    if not args:
        raise TclError('wrong # args: should be "dict remove dictValue ?key ...?"')
    d = _dict_from_list(args[0])
    for key in args[1:]:
        d.pop(key, None)
    return TclResult(value=_dict_to_list(d))


def _dict_replace(interp: TclInterp, args: list[str]) -> TclResult:
    """dict replace dictValue ?key value ...?"""
    if not args:
        raise TclError('wrong # args: should be "dict replace dictValue ?key value ...?"')
    if (len(args) - 1) % 2 != 0:
        raise TclError('wrong # args: should be "dict replace dictValue ?key value ...?"')
    d = _dict_from_list(args[0])
    for i in range(1, len(args), 2):
        d[args[i]] = args[i + 1]
    return TclResult(value=_dict_to_list(d))


def _dict_merge(interp: TclInterp, args: list[str]) -> TclResult:
    """dict merge ?dictValue ...?"""
    result: dict[str, str] = {}
    for arg in args:
        d = _dict_from_list(arg)
        result.update(d)
    return TclResult(value=_dict_to_list(result))


def _dict_for(interp: TclInterp, args: list[str]) -> TclResult:
    """dict for {keyVar valueVar} dictValue body"""
    if len(args) != 3:
        raise TclError('wrong # args: should be "dict for {keyVar valueVar} dictValue body"')
    var_list = _split_list(args[0])
    if len(var_list) != 2:
        raise TclError("must have exactly two variable names")
    key_var, val_var = var_list
    d = _dict_from_list(args[1])
    body = args[2]
    last_result = ""

    for k, v in d.items():
        interp.current_frame.set_var(key_var, k)
        interp.current_frame.set_var(val_var, v)
        try:
            result = interp.eval(body)
            last_result = result.value
        except TclBreak:
            break
        except TclContinue:
            continue

    return TclResult(value=last_result)


def _dict_map(interp: TclInterp, args: list[str]) -> TclResult:
    """dict map {keyVar valueVar} dictValue body"""
    if len(args) != 3:
        raise TclError('wrong # args: should be "dict map {keyVar valueVar} dictValue body"')
    var_list = _split_list(args[0])
    if len(var_list) != 2:
        raise TclError("must have exactly two variable names")
    key_var, val_var = var_list
    d = _dict_from_list(args[1])
    body = args[2]
    result_pairs: list[str] = []

    for k, v in d.items():
        interp.current_frame.set_var(key_var, k)
        interp.current_frame.set_var(val_var, v)
        try:
            result = interp.eval(body)
            result_pairs.append(_list_escape(k))
            result_pairs.append(_list_escape(result.value))
        except TclBreak:
            break
        except TclContinue:
            continue

    return TclResult(value=" ".join(result_pairs))


def _dict_filter(interp: TclInterp, args: list[str]) -> TclResult:
    """dict filter dictValue filterType arg ..."""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict filter dictValue filterType ..."')
    d = _dict_from_list(args[0])
    filter_type = args[1]

    match filter_type:
        case "key":
            if len(args) < 3:
                raise TclError(
                    'wrong # args: should be "dict filter dictValue key ?globPattern ...?"'
                )
            patterns = args[2:]
            result: dict[str, str] = {}
            for k, v in d.items():
                if any(fnmatch.fnmatch(k, p) for p in patterns):
                    result[k] = v
            return TclResult(value=_dict_to_list(result))

        case "value":
            if len(args) < 3:
                raise TclError(
                    'wrong # args: should be "dict filter dictValue value ?globPattern ...?"'
                )
            patterns = args[2:]
            result = {}
            for k, v in d.items():
                if any(fnmatch.fnmatch(v, p) for p in patterns):
                    result[k] = v
            return TclResult(value=_dict_to_list(result))

        case "script":
            if len(args) != 4:
                raise TclError(
                    'wrong # args: should be "dict filter dictValue script {keyVar valueVar} script"'
                )
            var_list = _split_list(args[2])
            if len(var_list) != 2:
                raise TclError("must have exactly two variable names")
            key_var, val_var = var_list
            body = args[3]
            result = {}
            for k, v in d.items():
                interp.current_frame.set_var(key_var, k)
                interp.current_frame.set_var(val_var, v)
                r = interp.eval(body)
                if r.value not in ("0", "false", "no", "off", ""):
                    result[k] = v
            return TclResult(value=_dict_to_list(result))

        case _:
            raise TclError(f'bad filterType "{filter_type}": must be key, script, or value')


def _dict_unset(interp: TclInterp, args: list[str]) -> TclResult:
    """dict unset dictVariable key ?key ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict unset dictVariable key ?key ...?"')
    var_name = args[0]
    keys = args[1:]

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    _nested_unset(d, keys)
    result = _dict_to_list(d)
    interp.current_frame.set_var(var_name, result)
    return TclResult(value=result)


def _dict_append(interp: TclInterp, args: list[str]) -> TclResult:
    """dict append dictVariable key ?string ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict append dictVariable key ?string ...?"')
    var_name = args[0]
    key = args[1]
    strings = args[2:]

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    current = d.get(key, "")
    d[key] = current + "".join(strings)
    result = _dict_to_list(d)
    interp.current_frame.set_var(var_name, result)
    return TclResult(value=result)


def _dict_lappend(interp: TclInterp, args: list[str]) -> TclResult:
    """dict lappend dictVariable key ?value ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict lappend dictVariable key ?value ...?"')
    var_name = args[0]
    key = args[1]
    values = args[2:]

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    current = d.get(key, "")
    parts = _split_list(current) if current else []
    parts.extend(values)
    d[key] = " ".join(_list_escape(p) for p in parts)
    result = _dict_to_list(d)
    interp.current_frame.set_var(var_name, result)
    return TclResult(value=result)


def _dict_incr(interp: TclInterp, args: list[str]) -> TclResult:
    """dict incr dictVariable key ?increment?"""
    if len(args) < 2 or len(args) > 3:
        raise TclError('wrong # args: should be "dict incr dictVariable key ?increment?"')
    var_name = args[0]
    key = args[1]
    increment = int(args[2]) if len(args) > 2 else 1

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    current = int(d.get(key, "0"))
    d[key] = str(current + increment)
    result = _dict_to_list(d)
    interp.current_frame.set_var(var_name, result)
    return TclResult(value=result)


def _dict_info(interp: TclInterp, args: list[str]) -> TclResult:
    """dict info dictValue"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "dict info dictValue"')
    d = _dict_from_list(args[0])
    return TclResult(value=f"{len(d)} entries in table")


def _dict_update(interp: TclInterp, args: list[str]) -> TclResult:
    """dict update dictVariable key varName ?key varName ...? body"""
    if len(args) < 4 or (len(args) - 2) % 2 != 0:
        raise TclError(
            'wrong # args: should be "dict update dictVariable key varName ?key varName ...? body"'
        )
    var_name = args[0]
    body = args[-1]
    mappings: list[tuple[str, str]] = []
    for i in range(1, len(args) - 1, 2):
        mappings.append((args[i], args[i + 1]))

    try:
        d = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        d = {}

    # Set local variables from dict values
    for key, local_var in mappings:
        if key in d:
            interp.current_frame.set_var(local_var, d[key])

    # Execute body
    result = interp.eval(body)

    # Write local variables back to dict
    for key, local_var in mappings:
        try:
            val = interp.current_frame.get_var(local_var)
            d[key] = val
        except TclError:
            d.pop(key, None)

    interp.current_frame.set_var(var_name, _dict_to_list(d))
    return result


def _dict_with(interp: TclInterp, args: list[str]) -> TclResult:
    """dict with dictVariable ?key ...? body"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "dict with dictVariable ?key ...? body"')
    var_name = args[0]
    keys = args[1:-1]
    body = args[-1]

    try:
        top_dict = _dict_from_list(interp.current_frame.get_var(var_name))
    except TclError:
        top_dict = {}

    # Navigate to the target sub-dict
    if keys:
        target_text = _nested_get(top_dict, keys)
        target = _dict_from_list(target_text)
    else:
        target = top_dict

    # Set local variables from dict entries
    for k, v in target.items():
        interp.current_frame.set_var(k, v)

    # Execute body
    result = interp.eval(body)

    # Write local variables back to the target dict
    for k in list(target.keys()):
        try:
            target[k] = interp.current_frame.get_var(k)
        except TclError:
            del target[k]

    # Store back into the top-level dict
    if keys:
        _nested_set(top_dict, keys, _dict_to_list(target))
        interp.current_frame.set_var(var_name, _dict_to_list(top_dict))
    else:
        interp.current_frame.set_var(var_name, _dict_to_list(target))

    return result


def register() -> None:
    """Register dict commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("dict", _cmd_dict)
