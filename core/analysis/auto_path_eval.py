"""Static mini-evaluator for ``lappend auto_path`` arguments.

A common Tcl idiom for adding a sibling/parent directory to the package
search path reuses only a handful of built-in commands that have
well-defined, side-effect-free behaviour:

    lappend auto_path [file join [file dirname [info script]] ..]
    lappend auto_path [file dirname [file dirname [info script]]]
    set auto_path [linsert $auto_path 0 /abs/path]

This module understands those forms well enough to produce a concrete
absolute directory.  Unknown forms return ``None`` and the caller simply
skips the entry — we never fall back to guessing.
"""

from __future__ import annotations

import os


def evaluate_auto_path_expr(raw: str, info_script: str | None) -> str | None:
    """Resolve *raw* (a ``lappend auto_path`` argument) to an absolute path.

    *info_script* should be the filesystem path of the file being
    analysed.  It substitutes for ``[info script]`` in evaluation — the
    value Tcl itself would report while running the file.

    Returns an absolute path on success, or ``None`` when any part of
    the expression is outside the supported subset.
    """
    raw = raw.strip()
    if not raw:
        return None
    tokens = _tokenise(raw)
    if tokens is None:
        return None
    parsed, rest = _parse(tokens, 0)
    if parsed is None or rest != len(tokens):
        return None
    result = _eval(parsed, info_script)
    if result is None:
        return None
    return os.path.abspath(result)


def _tokenise(text: str) -> list[str] | None:
    """Split *text* into ``[`` / ``]`` / word tokens.

    Brace groups are kept as a single word (brackets inside braces are
    literal).  Anything we don't recognise aborts by returning ``None``.
    """
    tokens: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if ch.isspace():
            i += 1
            continue
        if ch in "[]":
            tokens.append(ch)
            i += 1
            continue
        if ch == "{":
            depth = 1
            j = i + 1
            while j < n and depth > 0:
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                j += 1
            if depth != 0:
                return None
            tokens.append(text[i + 1 : j - 1])
            i = j
            continue
        if ch == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                j += 1
            if j >= n:
                return None
            tokens.append(text[i + 1 : j])
            i = j + 1
            continue
        j = i
        while j < n and not text[j].isspace() and text[j] not in "[]":
            j += 1
        tokens.append(text[i:j])
        i = j
    return tokens


# Parse tree node forms:
#   ("lit", str)             — a literal word
#   ("cmd", name, [args...]) — a command substitution [cmd arg ...]

_Node = tuple


def _parse(tokens: list[str], pos: int) -> tuple[_Node | None, int]:
    """Parse a single word starting at *pos*.  Returns (node, next_pos)."""
    if pos >= len(tokens):
        return (None, pos)
    tok = tokens[pos]
    if tok == "[":
        pos += 1
        if pos >= len(tokens):
            return (None, pos)
        name = tokens[pos]
        pos += 1
        args: list[_Node] = []
        while pos < len(tokens) and tokens[pos] != "]":
            node, pos = _parse(tokens, pos)
            if node is None:
                return (None, pos)
            args.append(node)
        if pos >= len(tokens):
            return (None, pos)
        pos += 1  # consume ]
        return (("cmd", name, args), pos)
    if tok == "]":
        return (None, pos)
    return (("lit", tok), pos + 1)


def _eval(node: _Node, info_script: str | None) -> str | None:
    kind = node[0]
    if kind == "lit":
        value = node[1]
        # Reject any word that still contains variable substitutions or
        # nested brackets we did not tokenise — we refuse to guess.
        if "$" in value:
            return None
        return value
    if kind == "cmd":
        name = node[1]
        args = node[2]
        if name == "info" and len(args) == 1:
            sub = _eval(args[0], info_script)
            if sub == "script":
                return info_script
            return None
        if name == "file" and len(args) >= 2:
            sub = _eval(args[0], info_script)
            if sub == "dirname" and len(args) == 2:
                inner = _eval(args[1], info_script)
                if inner is None:
                    return None
                return os.path.dirname(inner)
            if sub == "join" and len(args) >= 2:
                parts: list[str] = []
                for a in args[1:]:
                    p = _eval(a, info_script)
                    if p is None:
                        return None
                    parts.append(p)
                if not parts:
                    return None
                return os.path.join(*parts)
            return None
    return None
