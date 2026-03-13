"""Convert an if/elseif chain to a switch statement."""

from __future__ import annotations

import re

from . import RefactoringEdit, RefactoringResult
from ._spans import command_replacement_range, find_command_at

# Patterns for detecting equality tests on the same variable.
# Matches: $var eq "value", $var == "value", "$var" eq "value"
_EQ_PATTERN = re.compile(
    r"""
    ^\s*
    (?:\$\{?(\w+)\}?|"\$\{?(\w+)\}?")  # LHS variable
    \s+
    (eq|==|ne|!=)                         # operator
    \s+
    (.+?)                                 # RHS value
    \s*$
    """,
    re.VERBOSE,
)

# Reverse: "value" eq $var
_EQ_REVERSE_PATTERN = re.compile(
    r"""
    ^\s*
    (.+?)                                 # LHS value
    \s+
    (eq|==|ne|!=)                         # operator
    \s+
    (?:\$\{?(\w+)\}?|"\$\{?(\w+)\}?")  # RHS variable
    \s*$
    """,
    re.VERBOSE,
)


def _parse_eq_test(condition: str) -> tuple[str, str, str, bool] | None:
    """Parse an equality condition.

    Returns ``(var_name, operator, value, negated)`` or *None*.
    """
    cond = condition.strip()

    # Strip outer braces if present: { $x eq "foo" }
    if cond.startswith("{") and cond.endswith("}"):
        cond = cond[1:-1].strip()

    # Check for negation: !($x eq "foo") or !{$x eq "foo"}
    negated = False
    if cond.startswith("!"):
        inner = cond[1:].strip()
        if inner.startswith("(") and inner.endswith(")"):
            cond = inner[1:-1].strip()
            negated = True
        elif inner.startswith("{") and inner.endswith("}"):
            cond = inner[1:-1].strip()
            negated = True

    m = _EQ_PATTERN.match(cond)
    if m:
        var = m.group(1) or m.group(2)
        op = m.group(3)
        value = m.group(4).strip()
        is_ne = op in ("ne", "!=")
        return (var, op, value, negated ^ is_ne)

    m = _EQ_REVERSE_PATTERN.match(cond)
    if m:
        value = m.group(1).strip()
        op = m.group(2)
        var = m.group(3) or m.group(4)
        is_ne = op in ("ne", "!=")
        return (var, op, value, negated ^ is_ne)

    return None


def _strip_quotes(s: str) -> str:
    """Remove surrounding double quotes if present."""
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _render_switch_pattern(value: str) -> str:
    """Render a switch pattern safely, preserving whitespace literals."""
    if not value:
        return "{}"
    if any(ch.isspace() for ch in value):
        return "{" + value.replace("}", r"\}") + "}"
    return value


def if_to_switch(
    source: str,
    line: int,
    character: int,
) -> RefactoringResult | None:
    """Convert an if/elseif chain at the cursor to a switch statement.

    Only converts when every branch tests the same variable with ``eq``
    or ``==`` against a literal value.

    Returns *None* when conversion is not applicable.
    """
    # Find the if command at the cursor (recursing into nested bodies).
    cmd = find_command_at(source, line, character, predicate="if")
    if cmd is None:
        return None

    texts = cmd.texts
    if len(texts) < 3:
        return None

    # Parse the if/elseif chain.
    # Structure: if cond body ?elseif cond body?... ?else body?
    branches: list[tuple[str, str, bool]] = []  # (value, body, negated)
    else_body: str | None = None
    target_var: str | None = None

    i = 1  # skip "if"
    while i < len(texts):
        word = texts[i]

        if word in ("elseif", "then"):
            i += 1
            continue

        if word == "else":
            if i + 1 < len(texts):
                else_body = texts[i + 1]
            break

        # This should be a condition.
        condition = word
        if i + 1 >= len(texts):
            return None
        body = texts[i + 1]
        i += 2

        # Skip optional "then" after condition.
        if i < len(texts) and texts[i] == "then":
            i += 1
            if i < len(texts):
                body = texts[i]
                i += 1

        parsed = _parse_eq_test(condition)
        if parsed is None:
            return None

        var, _op, value, negated = parsed

        if target_var is None:
            target_var = var
        elif var != target_var:
            return None  # different variables in the chain

        # Negated conditions (ne/!=) make switch conversion awkward.
        if negated:
            return None

        branches.append((_strip_quotes(value), body, negated))

    if target_var is None or len(branches) < 2:
        return None

    # Build the switch statement.
    lines_list = source.split("\n")
    if cmd.range.start.line < len(lines_list):
        indent = _get_indent(lines_list[cmd.range.start.line])
    else:
        indent = ""
    inner = indent + "    "

    parts = [f"switch -exact -- ${target_var} {{"]
    for value, body, _neg in branches:
        body_trimmed = _reindent_body(body, inner + "    ")
        parts.append(f"{inner}{_render_switch_pattern(value)} {{")
        parts.append(body_trimmed)
        parts.append(f"{inner}}}")
    if else_body is not None:
        body_trimmed = _reindent_body(else_body, inner + "    ")
        parts.append(f"{inner}default {{")
        parts.append(body_trimmed)
        parts.append(f"{inner}}}")
    parts.append(f"{indent}}}")

    replacement = "\n".join(parts)

    start_line, start_character, end_line, end_character = command_replacement_range(source, cmd)
    edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_character,
        end_line=end_line,
        end_character=end_character,
        new_text=replacement,
    )

    return RefactoringResult(
        title=f"Convert to switch on ${target_var}",
        edits=(edit,),
        kind="refactor.rewrite",
    )


def _get_indent(line: str) -> str:
    return line[: len(line) - len(line.lstrip())]


def _reindent_body(body: str, target_indent: str) -> str:
    """Re-indent a braced body to *target_indent*."""
    # Strip surrounding braces/whitespace.
    text = body.strip()
    if text.startswith("{") and text.endswith("}"):
        text = text[1:-1]

    body_lines = text.split("\n")
    non_empty = [ln for ln in body_lines if ln.strip()]
    if not non_empty:
        return f"{target_indent}# empty"
    min_indent = min(len(ln) - len(ln.lstrip()) for ln in non_empty)

    result_lines = []
    for ln in body_lines:
        stripped = ln.strip()
        if stripped:
            result_lines.append(f"{target_indent}{ln[min_indent:]}")
        elif result_lines:  # preserve internal blank lines
            result_lines.append("")
    return "\n".join(result_lines)
