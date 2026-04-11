"""Convert a switch statement to a dict lookup."""

from __future__ import annotations

import re

from . import RefactoringEdit, RefactoringResult
from ._spans import command_replacement_range, find_command_at

_SET_RESULT_RE = re.compile(r"^\s*set\s+(\S+)\s+(.+?)\s*$", re.DOTALL)
_RETURN_RESULT_RE = re.compile(r"^\s*return\s+(.+?)\s*$", re.DOTALL)


def switch_to_dict(
    source: str,
    line: int,
    character: int,
) -> RefactoringResult | None:
    """Convert a switch where every arm sets the same variable to a constant.

    Patterns detected:
    - ``switch ... { pat1 { set result val1 } pat2 { set result val2 } }``
    - ``switch ... { pat1 { return val1 } pat2 { return val2 } }``

    Returns *None* when the switch doesn't match this shape.
    """
    cmd = find_command_at(source, line, character, predicate="switch")
    if cmd is None:
        return None

    texts = cmd.texts
    if len(texts) < 3:
        return None

    # Parse switch flags.
    i = 1
    mode = "exact"
    while i < len(texts) and texts[i].startswith("-"):
        flag = texts[i]
        if flag in ("-exact", "-glob", "-regexp"):
            mode = flag[1:]
        elif flag == "--":
            i += 1
            break
        i += 1

    # Only handle -exact mode.
    if mode != "exact":
        return None

    if i >= len(texts):
        return None
    subject = texts[i]
    i += 1

    # Remaining args are the pattern-body pairs.
    # Could be a single braced list or separate args.
    if i + 1 == len(texts):
        # Single braced argument — parse the pairs from it.
        pairs = _parse_braced_pairs(texts[i])
    else:
        # Separate arguments.
        pairs = []
        while i + 1 < len(texts):
            pairs.append((texts[i], texts[i + 1]))
            i += 2

    if not pairs:
        return None

    # Check every arm has the same shape: set VAR VALUE or return VALUE.
    target_var: str | None = None
    use_return = False
    dict_entries: list[tuple[str, str]] = []
    default_value: str | None = None

    for pattern, body in pairs:
        body_text = body.strip()
        if body_text.startswith("{") and body_text.endswith("}"):
            body_text = body_text[1:-1].strip()

        # Skip fallthrough markers.
        if body_text == "-":
            return None

        # Try ``set var value``.
        m = _SET_RESULT_RE.match(body_text)
        if m:
            var = m.group(1)
            value = m.group(2).strip()
            if target_var is None:
                target_var = var
                use_return = False
            elif var != target_var:
                return None
            if pattern == "default":
                default_value = value
            else:
                dict_entries.append((pattern, value))
            continue

        # Try ``return value``.
        m = _RETURN_RESULT_RE.match(body_text)
        if m:
            value = m.group(1).strip()
            if target_var is None:
                target_var = "__return__"
                use_return = True
            elif not use_return:
                return None
            if pattern == "default":
                default_value = value
            else:
                dict_entries.append((pattern, value))
            continue

        # Body doesn't match either pattern.
        return None

    if len(dict_entries) < 2:
        return None

    # Build the replacement.
    src_lines = source.split("\n")
    if cmd.range.start.line < len(src_lines):
        indent = _get_indent(src_lines[cmd.range.start.line])
    else:
        indent = ""

    # Build dict create line.
    dict_items = " ".join(f"{k} {v}" for k, v in dict_entries)
    dict_name = f"{target_var}_map" if not use_return else "result_map"

    parts: list[str] = [f"set {dict_name} [dict create {dict_items}]"]

    if default_value is not None:
        if use_return:
            parts.append(f"{indent}if {{[dict exists ${dict_name} {subject}]}} {{")
            parts.append(f"{indent}    return [dict get ${dict_name} {subject}]")
            parts.append(f"{indent}}} else {{")
            parts.append(f"{indent}    return {default_value}")
            parts.append(f"{indent}}}")
        else:
            parts.append(f"{indent}if {{[dict exists ${dict_name} {subject}]}} {{")
            parts.append(f"{indent}    set {target_var} [dict get ${dict_name} {subject}]")
            parts.append(f"{indent}}} else {{")
            parts.append(f"{indent}    set {target_var} {default_value}")
            parts.append(f"{indent}}}")
    else:
        if use_return:
            parts.append(f"{indent}return [dict get ${dict_name} {subject}]")
        else:
            parts.append(f"{indent}set {target_var} [dict get ${dict_name} {subject}]")

    replacement = "\n".join(parts)

    start_line, start_character, end_line, end_character = command_replacement_range(source, cmd)
    edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_character,
        end_line=end_line,
        end_character=end_character,
        new_text=replacement,
    )

    title = "Convert to dict lookup" if use_return else f"Convert to dict lookup on '{target_var}'"
    return RefactoringResult(title=title, edits=(edit,), kind="refactor.rewrite")


def _parse_braced_pairs(text: str) -> list[tuple[str, str]]:
    """Parse ``{pat1 {body1} pat2 {body2} ...}`` into pairs."""
    text = text.strip()
    if text.startswith("{") and text.endswith("}"):
        text = text[1:-1].strip()

    pairs: list[tuple[str, str]] = []
    tokens = _tokenise_switch_body(text)
    i = 0
    while i + 1 < len(tokens):
        pairs.append((tokens[i], tokens[i + 1]))
        i += 2
    return pairs


def _tokenise_switch_body(text: str) -> list[str]:
    """Simple brace-aware tokeniser for switch body contents."""
    tokens: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        # Skip whitespace.
        while i < n and text[i] in " \t\n\r":
            i += 1
        if i >= n:
            break

        if text[i] == "{":
            # Find matching close brace.
            depth = 1
            start = i
            i += 1
            while i < n and depth > 0:
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                i += 1
            tokens.append(text[start:i])
        elif text[i] == '"':
            # Quoted string.
            start = i
            i += 1
            while i < n and text[i] != '"':
                if text[i] == "\\":
                    i += 1
                i += 1
            if i < n:
                i += 1
            tokens.append(text[start:i])
        else:
            # Bare word.
            start = i
            while i < n and text[i] not in " \t\n\r{}":
                i += 1
            tokens.append(text[start:i])

    return tokens


def _get_indent(line: str) -> str:
    return line[: len(line) - len(line.lstrip())]
