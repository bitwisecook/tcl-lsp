"""List commands: list, lindex, llength, lappend, lrange, lsort, etc."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..machine import _list_escape, _parse_index, _split_list
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_list(interp: TclInterp, args: list[str]) -> TclResult:
    """list ?arg ...?"""
    return TclResult(value=" ".join(_list_escape(a) for a in args))


def _cmd_lindex(interp: TclInterp, args: list[str]) -> TclResult:
    """lindex list ?index ...?"""
    if not args:
        raise TclError('wrong # args: should be "lindex list ?index ...?"')
    lst = _split_list(args[0])
    for idx_str in args[1:]:
        idx = _parse_index(idx_str, len(lst))
        if 0 <= idx < len(lst):
            lst = _split_list(lst[idx])
        else:
            return TclResult(value="")
    if len(args) == 1:
        return TclResult(value=args[0])
    return TclResult(value=lst[0] if len(lst) == 1 else " ".join(_list_escape(e) for e in lst))


def _cmd_llength(interp: TclInterp, args: list[str]) -> TclResult:
    """llength list"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "llength list"')
    return TclResult(value=str(len(_split_list(args[0]))))


def _cmd_lrange(interp: TclInterp, args: list[str]) -> TclResult:
    """lrange list first last"""
    if len(args) != 3:
        raise TclError('wrong # args: should be "lrange list first last"')
    lst = _split_list(args[0])
    first = _parse_index(args[1], len(lst))
    last = _parse_index(args[2], len(lst))
    first = max(0, first)
    last = min(len(lst) - 1, last)
    if first > last:
        return TclResult(value="")
    return TclResult(value=" ".join(_list_escape(e) for e in lst[first : last + 1]))


def _cmd_lsearch(interp: TclInterp, args: list[str]) -> TclResult:
    """lsearch ?options? list pattern"""
    import fnmatch

    all_results = False
    inline = False
    exact = False
    regexp_mode = False
    not_flag = False
    nocase = False

    remaining = list(args)
    while len(remaining) > 2 and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        match opt:
            case "-all":
                all_results = True
            case "-inline":
                inline = True
            case "-exact":
                exact = True
            case "-glob":
                exact = False
                regexp_mode = False
            case "-regexp":
                regexp_mode = True
                exact = False
            case "-sorted":
                pass  # TODO: sorted search optimisation
            case "-not":
                not_flag = True
            case "-nocase":
                nocase = True
            case "--":
                break

    if len(remaining) != 2:
        raise TclError('wrong # args: should be "lsearch ?options? list pattern"')

    lst = _split_list(remaining[0])
    pattern = remaining[1]

    matches: list[tuple[int, str]] = []
    for i, elem in enumerate(lst):
        matched = False
        a, b = elem, pattern
        if nocase:
            a, b = a.lower(), b.lower()
        if exact:
            matched = a == b
        elif regexp_mode:
            import re

            matched = re.search(b, a) is not None
        else:
            matched = fnmatch.fnmatch(a, b)

        if not_flag:
            matched = not matched

        if matched:
            matches.append((i, elem))
            if not all_results:
                break

    if not matches:
        return TclResult(value="-1" if not all_results else "")

    if all_results:
        if inline:
            return TclResult(value=" ".join(_list_escape(e) for _, e in matches))
        return TclResult(value=" ".join(str(i) for i, _ in matches))

    if inline:
        return TclResult(value=matches[0][1])
    return TclResult(value=str(matches[0][0]))


def _cmd_lsort(interp: TclInterp, args: list[str]) -> TclResult:
    """lsort ?options? list"""
    ascending = True
    sort_type = "ascii"
    unique = False
    nocase = False

    remaining = list(args)
    while len(remaining) > 1 and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        match opt:
            case "-ascii":
                sort_type = "ascii"
            case "-dictionary":
                sort_type = "dictionary"
            case "-integer":
                sort_type = "integer"
            case "-real":
                sort_type = "real"
            case "-increasing":
                ascending = True
            case "-decreasing":
                ascending = False
            case "-unique":
                unique = True
            case "-nocase":
                nocase = True
            case "-command":
                remaining.pop(0)  # consume the command arg
            case "--":
                break

    if len(remaining) != 1:
        raise TclError('wrong # args: should be "lsort ?options? list"')

    lst = _split_list(remaining[0])

    def dedupe_key(s: str) -> str:
        if sort_type == "integer":
            return str(int(s, 0))
        if sort_type == "real":
            return str(float(s))
        return s.lower() if nocase else s

    result: list[str]
    if sort_type == "integer":
        for item in lst:
            try:
                int(item, 0)
            except ValueError:
                raise TclError(f'expected integer but got "{item}"') from None
        result = sorted(lst, key=lambda s: int(s, 0), reverse=not ascending)
    elif sort_type == "real":
        for item in lst:
            try:
                float(item)
            except ValueError:
                raise TclError(f'expected floating-point number but got "{item}"') from None
        result = sorted(lst, key=lambda s: float(s), reverse=not ascending)
    elif nocase:
        result = sorted(lst, key=str.lower, reverse=not ascending)
    else:
        result = sorted(lst, reverse=not ascending)

    if unique:
        seen: set[str] = set()
        deduped: list[str] = []
        for item in result:
            key_str = dedupe_key(item)
            if key_str not in seen:
                seen.add(key_str)
                deduped.append(item)
        result = deduped

    return TclResult(value=" ".join(_list_escape(e) for e in result))


def _cmd_lreplace(interp: TclInterp, args: list[str]) -> TclResult:
    """lreplace list first last ?element ...?"""
    if len(args) < 3:
        raise TclError('wrong # args: should be "lreplace list first last ?element ...?"')
    lst = _split_list(args[0])
    first = _parse_index(args[1], len(lst))
    last = _parse_index(args[2], len(lst))
    new_elems = args[3:]
    first = max(0, first)
    if last < first:
        # last < first → insert before first, no deletion
        result = lst[:first] + new_elems + lst[first:]
    else:
        last = min(len(lst) - 1, last)
        result = lst[:first] + new_elems + lst[last + 1 :]
    return TclResult(value=" ".join(_list_escape(e) for e in result))


def _cmd_linsert(interp: TclInterp, args: list[str]) -> TclResult:
    """linsert list index ?element ...?"""
    if len(args) < 3:
        raise TclError('wrong # args: should be "linsert list index ?element ...?"')
    lst = _split_list(args[0])
    idx = _parse_index(args[1], len(lst))
    new_elems = args[2:]
    idx = max(0, min(idx, len(lst)))
    result = lst[:idx] + new_elems + lst[idx:]
    return TclResult(value=" ".join(_list_escape(e) for e in result))


def _cmd_lrepeat(interp: TclInterp, args: list[str]) -> TclResult:
    """lrepeat count ?value ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "lrepeat count ?value ...?"')
    count = int(args[0])
    values = args[1:]
    result = values * count
    return TclResult(value=" ".join(_list_escape(e) for e in result))


def _cmd_lreverse(interp: TclInterp, args: list[str]) -> TclResult:
    """lreverse list"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "lreverse list"')
    lst = _split_list(args[0])
    lst.reverse()
    return TclResult(value=" ".join(_list_escape(e) for e in lst))


def _cmd_lassign(interp: TclInterp, args: list[str]) -> TclResult:
    """lassign list ?varName ...?"""
    if not args:
        raise TclError('wrong # args: should be "lassign list ?varName ...?"')
    lst = _split_list(args[0])
    for i, var_name in enumerate(args[1:]):
        if i < len(lst):
            interp.current_frame.set_var(var_name, lst[i])
        else:
            interp.current_frame.set_var(var_name, "")
    remaining = lst[len(args) - 1 :]
    return TclResult(value=" ".join(_list_escape(e) for e in remaining))


def _cmd_lpop(interp: TclInterp, args: list[str]) -> TclResult:
    """lpop varName ?index ...?

    Removes and returns the element at the given index from the list
    stored in the variable.  Default index is ``end``.
    """
    if not args:
        raise TclError('wrong # args: should be "lpop varName ?index ...?"')
    var_name = args[0]
    lst_str = interp.current_frame.get_var(var_name)
    lst = _split_list(lst_str)
    if not lst:
        raise TclError(
            "list doesn't contain element 0",
            error_code="TCL OPERATION LPOP BADINDEX",
        )
    # Navigate to nested element for multiple indices
    if len(args) <= 1:
        # Default: remove last element
        popped = lst.pop()
        interp.current_frame.set_var(
            var_name, " ".join(_list_escape(e) for e in lst) if lst else ""
        )
        return TclResult(value=popped)
    if len(args) == 2:
        idx = _parse_index(args[1], len(lst))
        if idx < 0 or idx >= len(lst):
            raise TclError(
                f"list doesn't contain element {args[1]}",
                error_code="TCL OPERATION LPOP BADINDEX",
            )
        popped = lst.pop(idx)
        interp.current_frame.set_var(
            var_name, " ".join(_list_escape(e) for e in lst) if lst else ""
        )
        return TclResult(value=popped)
    # Multiple indices: navigate nested lists, pop from the innermost
    indices = args[1:]
    # Build a path of (parent_list, index) pairs for reconstruction
    path: list[tuple[list[str], int]] = []
    current = lst
    for i, idx_str in enumerate(indices[:-1]):
        idx = _parse_index(idx_str, len(current))
        if idx < 0 or idx >= len(current):
            raise TclError(
                f"element {idx_str} missing from sublist at index {idx_str}",
                error_code="TCL OPERATION LPOP BADINDEX",
            )
        path.append((current, idx))
        current = _split_list(current[idx])
    # Pop from the innermost list
    last_idx = _parse_index(indices[-1], len(current))
    if last_idx < 0 or last_idx >= len(current):
        raise TclError(
            f"list doesn't contain element {indices[-1]}",
            error_code="TCL OPERATION LPOP BADINDEX",
        )
    popped = current.pop(last_idx)
    # Reconstruct outward: replace each parent's element with the modified child
    child_str = " ".join(_list_escape(e) for e in current) if current else ""
    for parent, pidx in reversed(path):
        parent[pidx] = child_str
        child_str = " ".join(_list_escape(e) for e in parent) if parent else ""
    interp.current_frame.set_var(var_name, child_str)
    return TclResult(value=popped)


def _cmd_join(interp: TclInterp, args: list[str]) -> TclResult:
    """join list ?joinString?"""
    if not args:
        raise TclError('wrong # args: should be "join list ?joinString?"')
    lst = _split_list(args[0])
    sep = args[1] if len(args) > 1 else " "
    return TclResult(value=sep.join(lst))


def _cmd_split(interp: TclInterp, args: list[str]) -> TclResult:
    """split string ?splitChars?"""
    if not args:
        raise TclError('wrong # args: should be "split string ?splitChars?"')
    s = args[0]
    split_chars = args[1] if len(args) > 1 else " \t\n"

    if not split_chars:
        # Split into individual characters
        result = list(s)
    elif len(split_chars) == 1:
        result = s.split(split_chars)
    else:
        # Split on any character in splitChars
        result = []
        current: list[str] = []
        for c in s:
            if c in split_chars:
                result.append("".join(current))
                current = []
            else:
                current.append(c)
        result.append("".join(current))

    return TclResult(value=" ".join(_list_escape(e) for e in result))


def register() -> None:
    """Register list commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("list", _cmd_list)
    REGISTRY.register_handler("lindex", _cmd_lindex)
    REGISTRY.register_handler("llength", _cmd_llength)
    REGISTRY.register_handler("lrange", _cmd_lrange)
    REGISTRY.register_handler("lsearch", _cmd_lsearch)
    REGISTRY.register_handler("lsort", _cmd_lsort)
    REGISTRY.register_handler("lreplace", _cmd_lreplace)
    REGISTRY.register_handler("linsert", _cmd_linsert)
    REGISTRY.register_handler("lrepeat", _cmd_lrepeat)
    REGISTRY.register_handler("lreverse", _cmd_lreverse)
    REGISTRY.register_handler("lassign", _cmd_lassign)
    REGISTRY.register_handler("lpop", _cmd_lpop)
    REGISTRY.register_handler("join", _cmd_join)
    REGISTRY.register_handler("split", _cmd_split)
