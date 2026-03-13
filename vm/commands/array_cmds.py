"""Array commands: array get, set, names, size, exists, unset, etc."""

from __future__ import annotations

import fnmatch
from typing import TYPE_CHECKING

from ..machine import _list_escape, _split_list
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


_ARRAY_SUBCOMMANDS = (
    "anymore",
    "default",
    "donesearch",
    "exists",
    "for",
    "get",
    "names",
    "nextelement",
    "set",
    "size",
    "startsearch",
    "statistics",
    "unset",
)

_ARRAY_DISPATCH: dict[str, object] = {}  # populated in register()


def _resolve_subcommand(sub: str) -> str | None:
    """Resolve an abbreviated subcommand name to the full name."""
    if sub in _ARRAY_DISPATCH:
        return sub
    matches = [s for s in _ARRAY_SUBCOMMANDS if s.startswith(sub)]
    if len(matches) == 1 and matches[0] in _ARRAY_DISPATCH:
        return matches[0]
    return None


def _cmd_array(interp: TclInterp, args: list[str]) -> TclResult:
    """array subcommand arrayName ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "array subcommand ?arg ...?"')

    sub = args[0]
    rest = args[1:]

    resolved = _resolve_subcommand(sub)
    if resolved is not None:
        return _ARRAY_DISPATCH[resolved](interp, rest)

    subcmd_list = ", ".join(_ARRAY_SUBCOMMANDS[:-1]) + ", or " + _ARRAY_SUBCOMMANDS[-1]
    raise TclError(f'unknown or ambiguous subcommand "{sub}": must be {subcmd_list}')


def _array_get(interp: TclInterp, args: list[str]) -> TclResult:
    """array get arrayName ?pattern?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "array get arrayName ?pattern?"')
    name = args[0]
    pattern = args[1] if len(args) > 1 else None
    d = interp.current_frame.array_get(name)
    parts: list[str] = []
    for k, v in d.items():
        if pattern is not None and not fnmatch.fnmatch(k, pattern):
            continue
        parts.append(_list_escape(k))
        parts.append(_list_escape(v))
    return TclResult(value=" ".join(parts))


def _array_set(interp: TclInterp, args: list[str]) -> TclResult:
    """array set arrayName list"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "array set arrayName list"')
    name = args[0]
    elements = _split_list(args[1])
    if len(elements) % 2 != 0:
        raise TclError("list must have an even number of elements")
    mapping: dict[str, str] = {}
    for i in range(0, len(elements), 2):
        mapping[elements[i]] = elements[i + 1]
    interp.current_frame.array_set(name, mapping)
    return TclResult()


def _array_names(interp: TclInterp, args: list[str]) -> TclResult:
    """array names arrayName ?mode? ?pattern?"""
    if not args or len(args) > 3:
        raise TclError('wrong # args: should be "array names arrayName ?mode? ?pattern?"')
    name = args[0]
    mode = "glob"
    pattern = None
    if len(args) == 2:
        pattern = args[1]
    elif len(args) == 3:
        mode = args[1]
        pattern = args[2]

    names = interp.current_frame.array_names(name)

    if pattern is not None:
        match mode:
            case "-glob" | "glob":
                names = [n for n in names if fnmatch.fnmatch(n, pattern)]
            case "-exact" | "exact":
                names = [n for n in names if n == pattern]
            case "-regexp" | "regexp":
                import re

                names = [n for n in names if re.search(pattern, n)]
            case _:
                raise TclError(f'bad option "{mode}": must be -exact, -glob, or -regexp')

    return TclResult(value=" ".join(_list_escape(n) for n in names))


def _array_size(interp: TclInterp, args: list[str]) -> TclResult:
    """array size arrayName"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "array size arrayName"')
    return TclResult(value=str(interp.current_frame.array_size(args[0])))


def _array_exists(interp: TclInterp, args: list[str]) -> TclResult:
    """array exists arrayName"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "array exists arrayName"')
    return TclResult(value="1" if interp.current_frame.array_exists(args[0]) else "0")


def _array_unset(interp: TclInterp, args: list[str]) -> TclResult:
    """array unset arrayName ?pattern?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "array unset arrayName ?pattern?"')
    name = args[0]
    pattern = args[1] if len(args) > 1 else None

    if not interp.current_frame.array_exists(name):
        return TclResult()

    if pattern is None:
        # Unset the whole array
        frame, resolved = interp.current_frame._resolve(name)
        frame._arrays.pop(resolved, None)
    else:
        d = interp.current_frame.array_get(name)
        keys_to_remove = [k for k in d if fnmatch.fnmatch(k, pattern)]
        frame, resolved = interp.current_frame._resolve(name)
        arr = frame._arrays.get(resolved)
        if arr is not None:
            for k in keys_to_remove:
                arr.pop(k, None)
    return TclResult()


# Search operations (stateful iteration)

# Per-interpreter search state: maps search-id to (arrayName, keys, index).
# Active searches per array: maps arrayName to list of active search IDs
# (ordered as a stack — newest first, matching Tcl's linked list).
_search_state: dict[str, tuple[str, list[str], int]] = {}
_active_searches: dict[str, list[str]] = {}


def _next_search_id(array_name: str) -> str:
    """Compute the next search ID for *array_name*.

    Tcl assigns ``head.id + 1`` (or 1 when no active searches exist).
    This matches the C implementation's stack-based ID allocation.
    """
    active = _active_searches.get(array_name, [])
    if not active:
        return f"s-1-{array_name}"
    # The head (first in list) has the highest ID
    import re

    m = re.match(r"^s-(\d+)-", active[0])
    head_id = int(m.group(1)) if m else 0
    return f"s-{head_id + 1}-{array_name}"


def _invalidate_searches(array_name: str) -> None:
    """Invalidate all active searches for *array_name*.

    In Tcl, adding or removing array elements invalidates all active
    searches on that array.
    """
    active = _active_searches.pop(array_name, [])
    for sid in active:
        _search_state.pop(sid, None)


def _array_startsearch(interp: TclInterp, args: list[str]) -> TclResult:
    """array startsearch arrayName"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "array startsearch arrayName"')
    name = args[0]
    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')
    search_id = _next_search_id(name)
    keys = interp.current_frame.array_names(name)
    _search_state[search_id] = (name, keys, 0)
    _active_searches.setdefault(name, []).insert(0, search_id)
    return TclResult(value=search_id)


def _validate_search_id(search_id: str, array_name: str) -> None:
    """Validate search identifier format and ownership.

    Tcl search IDs have the format ``s-<number>-<arrayName>``.
    """
    import re

    m = re.match(r"^s-(\d+)-(.+)$", search_id)
    if m is None:
        raise TclError(f'illegal search identifier "{search_id}"')
    id_array = m.group(2)
    if id_array != array_name:
        raise TclError(f'search identifier "{search_id}" isn\'t for variable "{array_name}"')


def _array_nextelement(interp: TclInterp, args: list[str]) -> TclResult:
    """array nextelement arrayName searchId"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "array nextelement arrayName searchId"')
    name = args[0]
    search_id = args[1]
    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')
    _validate_search_id(search_id, name)
    if search_id not in _search_state:
        raise TclError(f'couldn\'t find search "{search_id}"')
    arr_name, keys, idx = _search_state[search_id]
    if idx >= len(keys):
        return TclResult(value="")
    _search_state[search_id] = (arr_name, keys, idx + 1)
    return TclResult(value=keys[idx])


def _array_donesearch(interp: TclInterp, args: list[str]) -> TclResult:
    """array donesearch arrayName searchId"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "array donesearch arrayName searchId"')
    name = args[0]
    search_id = args[1]
    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')
    _validate_search_id(search_id, name)
    _search_state.pop(search_id, None)
    active = _active_searches.get(name)
    if active and search_id in active:
        active.remove(search_id)
        if not active:
            _active_searches.pop(name, None)
    return TclResult()


def _array_anymore(interp: TclInterp, args: list[str]) -> TclResult:
    """array anymore arrayName searchId"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "array anymore arrayName searchId"')
    name = args[0]
    search_id = args[1]
    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')
    _validate_search_id(search_id, name)
    if search_id not in _search_state:
        raise TclError(f'couldn\'t find search "{search_id}"')
    _, keys, idx = _search_state[search_id]
    return TclResult(value="1" if idx < len(keys) else "0")


def _array_statistics(interp: TclInterp, args: list[str]) -> TclResult:
    """array statistics arrayName"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "array statistics arrayName"')
    name = args[0]
    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')
    size = interp.current_frame.array_size(name)
    return TclResult(value=f"{size} entries in table, average chain length 1.0")


def _cmd_parray(interp: TclInterp, args: list[str]) -> TclResult:
    """parray arrayName ?pattern?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "parray arrayName ?pattern?"')
    name = args[0]
    pattern = args[1] if len(args) > 1 else None

    if not interp.current_frame.array_exists(name):
        raise TclError(f'"{name}" isn\'t an array')

    d = interp.current_frame.array_get(name)
    keys = sorted(d.keys())
    if pattern is not None:
        keys = [k for k in keys if fnmatch.fnmatch(k, pattern)]

    if keys:
        max_len = max(len(f"{name}({k})") for k in keys)
        channel = interp.channels.get("stdout")
        for k in keys:
            label = f"{name}({k})"
            line = f"{label:<{max_len}} = {d[k]}\n"
            if channel is not None:
                channel.write(line)
    return TclResult()


def register() -> None:
    """Register array commands."""
    from core.commands.registry import REGISTRY

    _ARRAY_DISPATCH.update(
        {
            "anymore": _array_anymore,
            "donesearch": _array_donesearch,
            "exists": _array_exists,
            "get": _array_get,
            "names": _array_names,
            "nextelement": _array_nextelement,
            "set": _array_set,
            "size": _array_size,
            "startsearch": _array_startsearch,
            "statistics": _array_statistics,
            "unset": _array_unset,
        }
    )
    REGISTRY.register_handler("array", _cmd_array)
    REGISTRY.register_handler("parray", _cmd_parray)
