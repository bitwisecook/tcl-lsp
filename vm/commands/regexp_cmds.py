"""Regular expression commands: regexp, regsub."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _translate_pattern(pattern: str) -> str:
    """Translate Tcl ARE pattern extensions to Python regex.

    Handles ``***=`` (literal string match) and ``***:`` (director prefix).
    """
    if pattern.startswith("***="):
        # Literal string match — escape everything after the prefix
        return re.escape(pattern[4:])
    if pattern.startswith("***:"):
        # ARE director prefix — strip it, rest is normal ARE
        return pattern[4:]
    return pattern


def _tcl_list_element(s: str) -> str:
    """Format a string as a Tcl list element, quoting if needed."""
    if s == "":
        return "{}"
    # Characters that require bracing
    needs_quoting = False
    for ch in s:
        if ch in ' \t\n\\{}"$;':
            needs_quoting = True
            break
    if not needs_quoting:
        return s
    # If no unbalanced braces, brace it
    depth = 0
    has_bad_braces = False
    for ch in s:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                has_bad_braces = True
                break
    if depth != 0:
        has_bad_braces = True
    if not has_bad_braces:
        return "{" + s + "}"
    # Backslash-quote special chars
    result: list[str] = []
    for ch in s:
        if ch in ' \t\n\\{}"$;':
            result.append("\\")
        result.append(ch)
    return "".join(result)


def _cmd_regexp(interp: TclInterp, args: list[str]) -> TclResult:
    """regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?"""
    nocase = False
    all_flag = False
    inline = False
    indices_flag = False
    lineanchor = False
    linestop = False
    expanded = False
    start = 0
    remaining = list(args)

    while remaining and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        match opt:
            case "-nocase":
                nocase = True
            case "-all":
                all_flag = True
            case "-inline":
                inline = True
            case "-indices":
                indices_flag = True
            case "-line":
                lineanchor = True
                linestop = True
            case "-lineanchor":
                lineanchor = True
            case "-linestop":
                linestop = True
            case "-expanded":
                expanded = True
            case "-start":
                if not remaining:
                    raise TclError('missing argument to "-start" option')
                try:
                    start = int(remaining.pop(0))
                except ValueError:
                    raise TclError('expected integer but got "..."') from None
            case "--":
                break
            case _:
                raise TclError(
                    f'bad option "{opt}": must be -all, -about, -indices, '
                    f"-inline, -expanded, -line, -linestop, -lineanchor, "
                    f"-nocase, -start, or --"
                )

    if len(remaining) < 2:
        raise TclError(
            'wrong # args: should be "regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?"'
        )

    pattern = _translate_pattern(remaining[0])
    string = remaining[1]
    match_vars = remaining[2:]

    # In Tcl's ARE, `.` matches ALL chars including \n by default.
    # Python's default is the opposite, so start with DOTALL.
    flags = re.DOTALL
    if nocase:
        flags |= re.IGNORECASE
    if lineanchor:
        flags |= re.MULTILINE
    if linestop:
        # -linestop: `.` does NOT match \n — remove DOTALL
        flags &= ~re.DOTALL
    if expanded:
        flags |= re.VERBOSE

    # Clamp start
    if start < 0:
        start = 0
    if start > len(string):
        start = len(string)

    try:
        compiled = re.compile(pattern, flags)
    except re.error as e:
        raise TclError(f"cannot compile regular expression pattern: {e}") from e

    try:
        if all_flag:
            return _regexp_all(
                interp,
                compiled,
                string,
                start,
                match_vars,
                indices_flag,
                inline,
            )
        else:
            return _regexp_single(
                interp,
                compiled,
                string,
                start,
                match_vars,
                indices_flag,
                inline,
            )
    except re.error as e:
        raise TclError(f"cannot compile regular expression pattern: {e}") from e


def _set_match_vars(
    interp: TclInterp,
    m: re.Match[str] | None,
    match_vars: list[str],
    indices_flag: bool,
    offset: int,
) -> None:
    """Set match variables from a regex match result."""
    if not match_vars:
        return

    num_groups = m.lastindex if m is not None and m.lastindex else 0

    for i, v in enumerate(match_vars):
        if m is None:
            if indices_flag:
                interp.current_frame.set_var(v, "-1 -1")
            else:
                interp.current_frame.set_var(v, "")
        elif i == 0:
            if indices_flag:
                interp.current_frame.set_var(v, f"{m.start(0) + offset} {m.end(0) - 1 + offset}")
            else:
                interp.current_frame.set_var(v, m.group(0))
        elif i <= num_groups:
            grp = m.group(i)
            if grp is None:
                if indices_flag:
                    interp.current_frame.set_var(v, "-1 -1")
                else:
                    interp.current_frame.set_var(v, "")
            else:
                if indices_flag:
                    interp.current_frame.set_var(
                        v, f"{m.start(i) + offset} {m.end(i) - 1 + offset}"
                    )
                else:
                    interp.current_frame.set_var(v, grp)
        else:
            if indices_flag:
                interp.current_frame.set_var(v, "-1 -1")
            else:
                interp.current_frame.set_var(v, "")


def _regexp_single(
    interp: TclInterp,
    compiled: re.Pattern[str],
    string: str,
    start: int,
    match_vars: list[str],
    indices_flag: bool,
    inline: bool,
) -> TclResult:
    """Handle non-all regexp."""
    m = compiled.search(string, pos=start)
    if m is None:
        # In Tcl, match vars are NOT set on no-match
        if inline:
            return TclResult(value="")
        return TclResult(value="0")

    _set_match_vars(interp, m, match_vars, indices_flag, 0)

    if inline:
        if indices_flag:
            parts = ["{" + f"{m.start(0)} {m.end(0) - 1}" + "}"]
            num_groups = m.lastindex if m.lastindex else 0
            for i in range(1, num_groups + 1):
                grp = m.group(i)
                if grp is None:
                    parts.append("{-1 -1}")
                else:
                    parts.append("{" + f"{m.start(i)} {m.end(i) - 1}" + "}")
            return TclResult(value=" ".join(parts))
        else:
            parts = [m.group(0)]
            num_groups = m.lastindex if m.lastindex else 0
            for i in range(1, num_groups + 1):
                parts.append(m.group(i) or "")
            return TclResult(value=" ".join(_tcl_list_element(p) for p in parts))
    return TclResult(value="1")


def _regexp_all(
    interp: TclInterp,
    compiled: re.Pattern[str],
    string: str,
    start: int,
    match_vars: list[str],
    indices_flag: bool,
    inline: bool,
) -> TclResult:
    """Handle -all regexp."""
    matches: list[re.Match[str]] = []
    pos = start
    while pos <= len(string):
        m = compiled.search(string, pos=pos)
        if m is None:
            break
        matches.append(m)
        if m.start() == m.end():
            # Zero-length match — advance past it and stop if at end
            pos = m.end() + 1
        else:
            pos = m.end()

    if not matches:
        # In Tcl, match vars are NOT set on no-match
        if inline:
            return TclResult(value="")
        return TclResult(value="0")

    count = len(matches)
    # Set match vars from the LAST match (Tcl semantics)
    _set_match_vars(interp, matches[-1], match_vars, indices_flag, 0)

    if inline:
        result_parts: list[str] = []
        for m in matches:
            num_groups = m.lastindex if m.lastindex else 0
            if indices_flag:
                result_parts.append("{" + f"{m.start(0)} {m.end(0) - 1}" + "}")
                for i in range(1, num_groups + 1):
                    grp = m.group(i)
                    if grp is None:
                        result_parts.append("{-1 -1}")
                    else:
                        result_parts.append("{" + f"{m.start(i)} {m.end(i) - 1}" + "}")
            else:
                result_parts.append(_tcl_list_element(m.group(0)))
                for i in range(1, num_groups + 1):
                    result_parts.append(_tcl_list_element(m.group(i) or ""))
        return TclResult(value=" ".join(result_parts))
    return TclResult(value=str(count))


def _tcl_regsub_replace(m: re.Match[str], sub_spec: str) -> str:
    """Apply Tcl regsub substitution spec to a match.

    In Tcl's regsub, the substitution string is NOT evaluated as a
    Tcl script — ``[...]`` and ``$`` are treated literally.  Only
    ``&`` (or ``\\0``) refers to the full match, and ``\\1`` through
    ``\\9`` refer to sub-match groups.  A literal ``\\`` is ``\\\\``.
    """
    result: list[str] = []
    i = 0
    while i < len(sub_spec):
        ch = sub_spec[i]
        if ch == "&":
            result.append(m.group(0))
        elif ch == "\\" and i + 1 < len(sub_spec):
            next_ch = sub_spec[i + 1]
            if next_ch.isdigit():
                idx = int(next_ch)
                try:
                    grp = m.group(idx)
                    result.append(grp if grp is not None else "")
                except IndexError:
                    result.append("")
                i += 1
            elif next_ch == "\\":
                result.append("\\")
                i += 1
            elif next_ch == "&":
                result.append("&")
                i += 1
            else:
                result.append("\\")
                result.append(next_ch)
                i += 1
        else:
            result.append(ch)
        i += 1
    return "".join(result)


def _split_tcl_list(s: str) -> list[str]:
    """Minimal Tcl list split for command prefix parsing."""
    # Simple brace/quote-aware split
    result: list[str] = []
    i = 0
    while i < len(s):
        # Skip whitespace
        while i < len(s) and s[i] in " \t\n":
            i += 1
        if i >= len(s):
            break
        if s[i] == "{":
            depth = 1
            i += 1
            start = i
            while i < len(s) and depth > 0:
                if s[i] == "{":
                    depth += 1
                elif s[i] == "}":
                    depth -= 1
                i += 1
            if depth != 0:
                raise TclError("unmatched open brace in list")
            result.append(s[start : i - 1])
        elif s[i] == '"':
            i += 1
            start = i
            while i < len(s) and s[i] != '"':
                if s[i] == "\\":
                    i += 1
                i += 1
            result.append(s[start:i])
            if i < len(s):
                i += 1
        else:
            start = i
            while i < len(s) and s[i] not in " \t\n":
                i += 1
            result.append(s[start:i])
    return result


def _cmd_regsub(interp: TclInterp, args: list[str]) -> TclResult:
    """regsub ?-option ...? exp string subSpec ?varName?"""
    nocase = False
    all_flag = False
    command_flag = False
    lineanchor = False
    linestop = False
    expanded = False
    start = 0
    remaining = list(args)

    while remaining and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        match opt:
            case "-nocase":
                nocase = True
            case "-all":
                all_flag = True
            case "-command":
                command_flag = True
            case "-line":
                lineanchor = True
                linestop = True
            case "-lineanchor":
                lineanchor = True
            case "-linestop":
                linestop = True
            case "-expanded":
                expanded = True
            case "-start":
                if not remaining:
                    raise TclError('missing argument to "-start" option')
                try:
                    start = int(remaining.pop(0))
                except ValueError:
                    raise TclError('expected integer but got "..."') from None
            case "--":
                break
            case _:
                raise TclError(
                    f'bad option "{opt}": must be -all, -command, -expanded, '
                    f"-line, -lineanchor, -linestop, -nocase, -start, or --"
                )

    if len(remaining) < 3:
        raise TclError(
            'wrong # args: should be "regsub ?-option ...? exp string subSpec ?varName?"'
        )

    pattern_raw = remaining[0]
    string = remaining[1]
    sub_spec = remaining[2]
    var_name = remaining[3] if len(remaining) > 3 else None

    pattern = _translate_pattern(pattern_raw)

    # Tcl ARE default: . matches \n
    flags = re.DOTALL
    if nocase:
        flags |= re.IGNORECASE
    if lineanchor:
        flags |= re.MULTILINE
    if linestop:
        flags &= ~re.DOTALL
    if expanded:
        flags |= re.VERBOSE

    # Validate -command prefix
    if command_flag:
        cmd_prefix = _split_tcl_list(sub_spec)
        if not cmd_prefix:
            raise TclError("command prefix must be a list of at least one element")

    # Clamp start
    if start < 0:
        start = 0

    try:
        compiled = re.compile(pattern, flags)
    except re.error as e:
        raise TclError(f"cannot compile regular expression pattern: {e}") from e

    if start > 0:
        prefix = string[:start]
        search_part = string[start:]
    else:
        prefix = ""
        search_part = string

    if all_flag:
        # Replace all occurrences
        result_parts: list[str] = []
        count = 0
        last_end = 0
        for m in compiled.finditer(search_part):
            result_parts.append(search_part[last_end : m.start()])
            if command_flag:
                result_parts.append(_regsub_command_replace(interp, m, cmd_prefix))
            else:
                result_parts.append(_tcl_regsub_replace(m, sub_spec))
            last_end = m.end()
            count += 1
            # Avoid infinite loop on zero-length matches
            if m.start() == m.end():
                if last_end < len(search_part):
                    result_parts.append(search_part[last_end])
                    last_end += 1
        result_parts.append(search_part[last_end:])
        result = prefix + "".join(result_parts)
    else:
        m = compiled.search(search_part)
        if m is None:
            result = string
            count = 0
        else:
            if command_flag:
                replacement = _regsub_command_replace(interp, m, cmd_prefix)
            else:
                replacement = _tcl_regsub_replace(m, sub_spec)
            result = prefix + search_part[: m.start()] + replacement + search_part[m.end() :]
            count = 1

    if var_name:
        interp.current_frame.set_var(var_name, result)
        return TclResult(value=str(count))
    return TclResult(value=result)


def _regsub_command_replace(
    interp: TclInterp,
    m: re.Match[str],
    cmd_prefix: list[str],
) -> str:
    """Apply a -command regsub: call cmd_prefix with match groups."""
    cmd_args = list(cmd_prefix)
    cmd_args.append(m.group(0))
    num_groups = m.lastindex if m.lastindex else 0
    for i in range(1, num_groups + 1):
        grp = m.group(i)
        cmd_args.append(grp if grp is not None else "")
    # Build Tcl command string
    cmd_str = " ".join(_tcl_list_element(a) for a in cmd_args)
    result = interp.eval(cmd_str)
    return result.value


def register() -> None:
    """Register regexp commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("regexp", _cmd_regexp)
    REGISTRY.register_handler("regsub", _cmd_regsub)
