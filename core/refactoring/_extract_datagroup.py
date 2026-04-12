"""Extract to data-group — convert inline if/switch/matchclass patterns to class lookups.

Supports two modes:

1. **Static** (``extract_to_datagroup``): mechanically detects common patterns
   and produces a data-group definition + rewritten iRule code.  The
   data-group type is inferred from the values (IP/CIDR → ``ip``,
   integers → ``integer``, everything else → ``string``).

2. **AI-enhanced** (``suggest_datagroup_extraction``): returns structured
   context (patterns found, types detected, confidence) that an LLM can
   use to produce a richer extraction with naming, consolidation across
   events, and coverage notes.
"""

from __future__ import annotations

import ipaddress
import re

from ..parsing.command_segmenter import SegmentedCommand
from . import DataGroupDefinition, DataGroupExtractionResult, RefactoringEdit
from ._spans import command_replacement_range, find_command_at, walk_all_commands

# ── Value-type inference ──────────────────────────────────────────────

_CIDR_RE = re.compile(
    r"^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})"  # IPv4
    r"(/\d{1,2})?$"  # optional prefix length
)
_IPV6_CIDR_RE = re.compile(
    r"^([0-9a-fA-F:]+)"  # IPv6 address
    r"(/\d{1,3})?$"  # optional prefix length
)


def _is_ip_or_cidr(value: str) -> bool:
    """Return *True* if *value* looks like an IP address or CIDR range."""
    v = _strip_quotes(value)
    try:
        ipaddress.ip_network(v, strict=False)
        return True
    except (ValueError, TypeError):
        pass
    try:
        ipaddress.ip_address(v)
        return True
    except (ValueError, TypeError):
        pass
    return False


def _is_integer(value: str) -> bool:
    v = _strip_quotes(value)
    try:
        int(v)
        return True
    except (ValueError, TypeError):
        return False


def _infer_value_type(values: list[str]) -> str:
    """Infer the data-group value type from a list of keys/values.

    Returns ``"ip"`` if all values are IP addresses or CIDR ranges,
    ``"integer"`` if all are integers, otherwise ``"string"``.
    """
    if not values:
        return "string"

    stripped = [_strip_quotes(v) for v in values]

    if all(_is_ip_or_cidr(v) for v in stripped):
        return "ip"
    if all(_is_integer(v) for v in stripped):
        return "integer"
    return "string"


def _strip_quotes(s: str) -> str:
    s = s.strip()
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _normalise_dg_name(name: str) -> str:
    """Derive a clean data-group name from a descriptive string."""
    name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
    name = re.sub(r"_+", "_", name).strip("_").lower()
    return name or "extracted_dg"


# ── Pattern detection ─────────────────────────────────────────────────

# Pattern 1: if/elseif chain testing the same variable against literals.
#   if { $uri eq "/path1" } { ... } elseif { $uri eq "/path2" } { ... }
_EQ_COND_RE = re.compile(
    r"""
    ^\s*
    (?:\$\{?(\w+)\}?|"\$\{?(\w+)\}?")   # LHS variable
    \s+
    (eq|==|ne|!=)                          # operator
    \s+
    (.+?)                                  # RHS value
    \s*$
    """,
    re.VERBOSE,
)
_EQ_COND_REV_RE = re.compile(
    r"""
    ^\s*
    (.+?)                                  # LHS value
    \s+
    (eq|==|ne|!=)                          # operator
    \s+
    (?:\$\{?(\w+)\}?|"\$\{?(\w+)\}?")   # RHS variable
    \s*$
    """,
    re.VERBOSE,
)

# Pattern 2: switch with many literal arms.
# (Detected via the command segmenter.)

# Pattern 3: ``contains`` / ``starts_with`` membership tests.
#   if { $host eq "a.com" || $host eq "b.com" || ... }
_OR_SPLIT_RE = re.compile(r"\s*\|\|\s*")


def _parse_eq(cond: str) -> tuple[str, str, bool] | None:
    """Parse a simple equality test.

    Returns ``(var_name, value, negated)`` or *None*.
    """
    cond = cond.strip()
    if cond.startswith("{") and cond.endswith("}"):
        cond = cond[1:-1].strip()

    negated = False
    if cond.startswith("!"):
        inner = cond[1:].strip()
        if inner.startswith("(") and inner.endswith(")"):
            cond = inner[1:-1].strip()
            negated = True

    m = _EQ_COND_RE.match(cond)
    if m:
        var = m.group(1) or m.group(2)
        is_ne = m.group(3) in ("ne", "!=")
        return (var, m.group(4).strip(), negated ^ is_ne)

    m = _EQ_COND_REV_RE.match(cond)
    if m:
        var = m.group(3) or m.group(4)
        is_ne = m.group(2) in ("ne", "!=")
        return (var, m.group(1).strip(), negated ^ is_ne)

    return None


# ── Static extraction ─────────────────────────────────────────────────


def extract_to_datagroup_from_if(
    source: str,
    line: int,
    character: int,
    *,
    dg_name: str = "",
) -> DataGroupExtractionResult | None:
    """Extract an if/elseif chain comparing one variable to literals.

    Converts to ``[class match $var equals <dg_name>]``.
    """
    cmd = find_command_at(source, line, character, predicate="if")
    if cmd is None:
        return None

    texts = cmd.texts

    # Parse the if/elseif chain — look for a simple membership pattern
    # where every branch tests $var eq "literal" with the same body shape.
    target_var: str | None = None
    values: list[str] = []
    bodies: list[str] = []
    has_else = False
    else_body: str | None = None

    # Also try detecting OR-chain in a single condition.
    if len(texts) >= 3:
        or_result = _try_or_chain(texts[1])
        if or_result is not None:
            target_var, values = or_result
            bodies.append(texts[2])

    if target_var is None:
        # Fall back to if/elseif chain parsing.
        i = 1
        while i < len(texts):
            word = texts[i]
            if word in ("elseif", "then"):
                i += 1
                continue
            if word == "else":
                has_else = True
                if i + 1 < len(texts):
                    else_body = texts[i + 1]
                break

            condition = word
            if i + 1 >= len(texts):
                return None
            body = texts[i + 1]
            i += 2

            parsed = _parse_eq(condition)
            if parsed is None:
                return None
            var, value, negated = parsed
            if negated:
                return None

            if target_var is None:
                target_var = var
            elif var != target_var:
                return None

            values.append(value)
            bodies.append(body)

    if target_var is None or len(values) < 2:
        return None

    stripped_values = [_strip_quotes(v) for v in values]
    value_type = _infer_value_type(stripped_values)

    if not dg_name:
        dg_name = _normalise_dg_name(f"{target_var}_whitelist")

    # Build the replacement code.
    src_lines = source.split("\n")
    indent = ""
    if cmd.range.start.line < len(src_lines):
        indent = _get_indent(src_lines[cmd.range.start.line])

    bodies_identical = len(set(b.strip() for b in bodies)) <= 1

    if bodies_identical and bodies:
        # All branches have the same body → membership test.
        records = tuple((v, "") for v in stripped_values)
        data_group = DataGroupDefinition(
            name=dg_name,
            value_type=value_type,
            records=records,
        )

        body_text = _reindent_body(bodies[0], indent + "    ")
        if has_else and else_body is not None:
            else_text = _reindent_body(else_body, indent + "    ")
            replacement = (
                f"if {{ [class match ${target_var} equals {dg_name}] }} {{\n"
                f"{body_text}\n"
                f"{indent}}} else {{\n"
                f"{else_text}\n"
                f"{indent}}}"
            )
        else:
            replacement = (
                f"if {{ [class match ${target_var} equals {dg_name}] }} {{\n{body_text}\n{indent}}}"
            )
    else:
        # Try to detect set/return mapping shape.
        pairs = list(zip(values, bodies))
        mapped = _extract_set_or_return_values(pairs)
        if mapped is None:
            return None

        set_var, use_return, value_entries = mapped
        records = tuple(
            (_strip_quotes(v), _strip_quotes(ve)) for v, ve in zip(stripped_values, value_entries)
        )
        data_group = DataGroupDefinition(
            name=dg_name,
            value_type="string",
            records=records,
        )

        if use_return:
            replacement = f"return [class lookup ${target_var} {dg_name}]"
        else:
            replacement = f"set {set_var} [class lookup ${target_var} {dg_name}]"

    start_line, start_character, end_line, end_character = command_replacement_range(source, cmd)
    edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_character,
        end_line=end_line,
        end_character=end_character,
        new_text=replacement,
    )

    return DataGroupExtractionResult(
        title=f"Extract to data-group '{dg_name}' ({value_type})",
        edits=(edit,),
        data_group=data_group,
    )


def extract_to_datagroup_from_switch(
    source: str,
    line: int,
    character: int,
    *,
    dg_name: str = "",
) -> DataGroupExtractionResult | None:
    """Extract a switch statement with many literal arms to a data-group.

    Handles two shapes:
    1. Every arm has the same body → ``class match`` membership test.
    2. Each arm maps to a different constant → ``class lookup`` with values.
    """
    cmd = find_command_at(source, line, character, predicate="switch")
    if cmd is None:
        return None

    texts = cmd.texts

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

    if mode != "exact":
        return None

    if i >= len(texts):
        return None
    subject = texts[i]
    i += 1

    # Extract the variable name from the subject.
    subject_var = _extract_var_name(subject)
    if subject_var is None:
        return None

    # Parse pattern-body pairs.
    pairs: list[tuple[str, str]] = []
    default_body: str | None = None
    if i + 1 == len(texts):
        pairs = _parse_braced_pairs(texts[i])
    else:
        while i + 1 < len(texts):
            pairs.append((texts[i], texts[i + 1]))
            i += 2

    if len(pairs) < 3:
        return None

    # Separate default from regular arms.
    regular_pairs: list[tuple[str, str]] = []
    for pattern, body in pairs:
        if pattern == "default":
            default_body = body
        elif body.strip() == "-":
            # Fallthrough — skip (can't map to data-group).
            return None
        else:
            regular_pairs.append((pattern, body))

    if len(regular_pairs) < 3:
        return None

    keys = [_strip_quotes(p) for p, _ in regular_pairs]
    value_type = _infer_value_type(keys)

    if not dg_name:
        dg_name = _normalise_dg_name(f"{subject_var}_map")

    # Check if all bodies are identical (membership test).
    bodies = [b.strip() for _, b in regular_pairs]
    all_same = len(set(bodies)) == 1

    src_lines = source.split("\n")
    indent = ""
    if cmd.range.start.line < len(src_lines):
        indent = _get_indent(src_lines[cmd.range.start.line])

    if all_same:
        # Pure membership test.
        records = tuple((_strip_quotes(p), "") for p, _ in regular_pairs)
        data_group = DataGroupDefinition(
            name=dg_name,
            value_type=value_type,
            records=records,
        )

        body_text = _reindent_body(bodies[0], indent + "    ")
        if default_body is not None:
            else_text = _reindent_body(default_body, indent + "    ")
            replacement = (
                f"if {{ [class match ${subject_var} equals {dg_name}] }} {{\n"
                f"{body_text}\n"
                f"{indent}}} else {{\n"
                f"{else_text}\n"
                f"{indent}}}"
            )
        else:
            replacement = (
                f"if {{ [class match ${subject_var} equals {dg_name}] }} {{\n"
                f"{body_text}\n"
                f"{indent}}}"
            )
    else:
        # Value-mapping: each arm maps key → value.
        # Try to detect ``set result VALUE`` or ``return VALUE`` shape.
        mapped = _extract_set_or_return_values(regular_pairs)
        if mapped is not None:
            target_var, use_return, value_entries = mapped
            records = tuple(
                (_strip_quotes(p), _strip_quotes(v)) for p, v in zip(keys, value_entries)
            )
            data_group = DataGroupDefinition(
                name=dg_name,
                value_type="string",
                records=records,
            )

            if default_body is not None:
                default_val = _extract_single_value(default_body, use_return, target_var)
                if use_return:
                    replacement = (
                        f"if {{ [class match ${subject_var} equals {dg_name}] }} {{\n"
                        f"{indent}    return [class lookup ${subject_var} {dg_name}]\n"
                        f"{indent}}} else {{\n"
                        f"{indent}    return {default_val or _strip_quotes(default_body)}\n"
                        f"{indent}}}"
                    )
                else:
                    replacement = (
                        f"if {{ [class match ${subject_var} equals {dg_name}] }} {{\n"
                        f"{indent}    set {target_var} [class lookup ${subject_var} {dg_name}]\n"
                        f"{indent}}} else {{\n"
                        f"{indent}    set {target_var} {default_val or _strip_quotes(default_body)}\n"
                        f"{indent}}}"
                    )
            else:
                if use_return:
                    replacement = f"return [class lookup ${subject_var} {dg_name}]"
                else:
                    replacement = f"set {target_var} [class lookup ${subject_var} {dg_name}]"
        else:
            # Bodies differ in complex ways — equivalence cannot be
            # guaranteed, so refuse the refactoring.
            return None

    start_line, start_character, end_line, end_character = command_replacement_range(source, cmd)
    edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_character,
        end_line=end_line,
        end_character=end_character,
        new_text=replacement,
    )

    return DataGroupExtractionResult(
        title=f"Extract to data-group '{dg_name}' ({value_type})",
        edits=(edit,),
        data_group=data_group,
    )


def extract_to_datagroup(
    source: str,
    line: int,
    character: int,
    *,
    dg_name: str = "",
) -> DataGroupExtractionResult | None:
    """Try all static extraction patterns at the cursor position.

    Tries if/elseif chains first, then switch statements.
    """
    result = extract_to_datagroup_from_if(
        source,
        line,
        character,
        dg_name=dg_name,
    )
    if result is not None:
        return result

    return extract_to_datagroup_from_switch(
        source,
        line,
        character,
        dg_name=dg_name,
    )


# ── AI-enhanced extraction context ────────────────────────────────────


def suggest_datagroup_extraction(source: str) -> list[dict]:
    """Scan source for patterns that could be extracted to data-groups.

    Returns a list of candidate extractions with structured context for
    an LLM to refine.  Each entry contains:
    - ``line``: start line of the pattern
    - ``pattern_type``: ``"if_chain"``, ``"switch"``, or ``"or_chain"``
    - ``variable``: the tested variable name
    - ``values``: list of literal values found
    - ``inferred_type``: ``"ip"``, ``"integer"``, or ``"string"``
    - ``has_cidr``: whether any values contain CIDR notation
    - ``body_shape``: ``"identical"``, ``"set_mapping"``, ``"return_mapping"``, or ``"complex"``
    - ``suggested_name``: auto-generated data-group name
    - ``confidence``: ``"high"`` (mechanical), ``"medium"`` (needs review),
      or ``"low"`` (LLM should decide)
    - ``static_result``: the ``DataGroupExtractionResult`` if the static
      extractor can handle it, else *None*
    """
    candidates: list[dict] = []

    for seg in walk_all_commands(source):
        if not seg.texts:
            continue

        cmd_name = seg.texts[0]
        line = seg.range.start.line

        if cmd_name == "if":
            cand = _analyse_if_chain(source, seg)
            if cand is not None:
                cand["static_result"] = extract_to_datagroup_from_if(
                    source,
                    line,
                    0,
                )
                candidates.append(cand)

        elif cmd_name == "switch":
            cand = _analyse_switch(source, seg)
            if cand is not None:
                cand["static_result"] = extract_to_datagroup_from_switch(
                    source,
                    line,
                    0,
                )
                candidates.append(cand)

    return candidates


def _analyse_if_chain(source: str, cmd: SegmentedCommand) -> dict | None:
    """Analyse an if/elseif chain for data-group extraction potential."""
    texts = cmd.texts
    if len(texts) < 3:
        return None

    target_var: str | None = None
    values: list[str] = []
    bodies: list[str] = []

    # First try OR-chain in a single condition.
    or_result = _try_or_chain(texts[1])
    if or_result is not None:
        target_var, values = or_result
        if len(texts) >= 3:
            bodies.append(texts[2])
    else:
        i = 1
        while i < len(texts):
            word = texts[i]
            if word in ("elseif", "then"):
                i += 1
                continue
            if word == "else":
                break

            condition = word
            if i + 1 >= len(texts):
                break
            body = texts[i + 1]
            i += 2

            parsed = _parse_eq(condition)
            if parsed is None:
                return None
            var, value, negated = parsed
            if negated:
                return None
            if target_var is None:
                target_var = var
            elif var != target_var:
                return None

            values.append(value)
            bodies.append(body)

    if target_var is None or len(values) < 2:
        return None

    stripped = [_strip_quotes(v) for v in values]
    value_type = _infer_value_type(stripped)
    has_cidr = any("/" in v for v in stripped if _is_ip_or_cidr(v))

    body_set = set(b.strip() for b in bodies)
    if len(body_set) == 1:
        body_shape = "identical"
    else:
        body_shape = _classify_body_shape(bodies)

    confidence = (
        "high" if body_shape in ("identical", "set_mapping", "return_mapping") else "medium"
    )

    return {
        "line": cmd.range.start.line,
        "pattern_type": "if_chain",
        "variable": target_var,
        "values": stripped,
        "inferred_type": value_type,
        "has_cidr": has_cidr,
        "body_shape": body_shape,
        "suggested_name": _normalise_dg_name(f"{target_var}_whitelist"),
        "confidence": confidence,
        "value_count": len(values),
    }


def _analyse_switch(source: str, cmd: SegmentedCommand) -> dict | None:
    """Analyse a switch statement for data-group extraction potential."""
    texts = cmd.texts

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

    if mode != "exact" or i >= len(texts):
        return None

    subject = texts[i]
    subject_var = _extract_var_name(subject)
    if subject_var is None:
        return None
    i += 1

    pairs: list[tuple[str, str]] = []
    if i + 1 == len(texts):
        pairs = _parse_braced_pairs(texts[i])
    else:
        while i + 1 < len(texts):
            pairs.append((texts[i], texts[i + 1]))
            i += 2

    regular = [(p, b) for p, b in pairs if p != "default" and b.strip() != "-"]
    if len(regular) < 3:
        return None

    keys = [_strip_quotes(p) for p, _ in regular]
    value_type = _infer_value_type(keys)
    has_cidr = any("/" in v for v in keys if _is_ip_or_cidr(v))

    bodies = [b.strip() for _, b in regular]
    if len(set(bodies)) == 1:
        body_shape = "identical"
    else:
        body_shape = _classify_body_shape(bodies)

    confidence = (
        "high" if body_shape in ("identical", "set_mapping", "return_mapping") else "medium"
    )
    if mode != "exact":
        confidence = "low"

    return {
        "line": cmd.range.start.line,
        "pattern_type": "switch",
        "variable": subject_var,
        "values": keys,
        "inferred_type": value_type,
        "has_cidr": has_cidr,
        "body_shape": body_shape,
        "suggested_name": _normalise_dg_name(f"{subject_var}_map"),
        "confidence": confidence,
        "value_count": len(regular),
    }


# ── Helpers ───────────────────────────────────────────────────────────

_SET_VAL_RE = re.compile(r"^\s*set\s+(\S+)\s+(.+?)\s*$", re.DOTALL)
_RETURN_VAL_RE = re.compile(r"^\s*return\s+(.+?)\s*$", re.DOTALL)


def _try_or_chain(condition: str) -> tuple[str, list[str]] | None:
    """Detect ``$var eq "a" || $var eq "b" || ...`` chains."""
    cond = condition.strip()
    if cond.startswith("{") and cond.endswith("}"):
        cond = cond[1:-1].strip()

    parts = _OR_SPLIT_RE.split(cond)
    if len(parts) < 2:
        return None

    target_var: str | None = None
    values: list[str] = []
    for part in parts:
        parsed = _parse_eq(part.strip())
        if parsed is None:
            return None
        var, value, negated = parsed
        if negated:
            return None
        if target_var is None:
            target_var = var
        elif var != target_var:
            return None
        values.append(value)

    if target_var is None or len(values) < 2:
        return None
    return (target_var, values)


def _extract_var_name(subject: str) -> str | None:
    """Extract a variable name from ``$var`` or ``${var}``."""
    s = subject.strip()
    if s.startswith("${") and s.endswith("}"):
        return s[2:-1]
    if s.startswith("$"):
        name = s[1:]
        if re.match(r"^[A-Za-z_]\w*$", name):
            return name
    return None


def _classify_body_shape(bodies: list[str]) -> str:
    """Classify the shape of switch/if bodies."""
    target_var = None
    use_return = None

    for body in bodies:
        text = body.strip()
        if text.startswith("{") and text.endswith("}"):
            text = text[1:-1].strip()

        m = _SET_VAL_RE.match(text)
        if m:
            var = m.group(1)
            if target_var is None:
                target_var = var
                use_return = False
            elif var != target_var or use_return:
                return "complex"
            continue

        m = _RETURN_VAL_RE.match(text)
        if m:
            if use_return is None:
                use_return = True
            elif not use_return:
                return "complex"
            continue

        return "complex"

    if use_return:
        return "return_mapping"
    if target_var is not None:
        return "set_mapping"
    return "complex"


def _extract_set_or_return_values(
    pairs: list[tuple[str, str]],
) -> tuple[str, bool, list[str]] | None:
    """Extract (target_var, use_return, values) from switch arms."""
    target_var = None
    use_return = None
    values: list[str] = []

    for _pattern, body in pairs:
        text = body.strip()
        if text.startswith("{") and text.endswith("}"):
            text = text[1:-1].strip()

        m = _SET_VAL_RE.match(text)
        if m:
            var = m.group(1)
            val = m.group(2).strip()
            if target_var is None:
                target_var = var
                use_return = False
            elif var != target_var or use_return:
                return None
            values.append(val)
            continue

        m = _RETURN_VAL_RE.match(text)
        if m:
            val = m.group(1).strip()
            if use_return is None:
                use_return = True
                target_var = "__return__"
            elif not use_return:
                return None
            values.append(val)
            continue

        return None

    if target_var is None or not values:
        return None
    return (target_var, bool(use_return), values)


def _extract_single_value(body: str, use_return: bool, target_var: str) -> str | None:
    """Extract the value from a single arm body."""
    text = body.strip()
    if text.startswith("{") and text.endswith("}"):
        text = text[1:-1].strip()

    if use_return:
        m = _RETURN_VAL_RE.match(text)
        return m.group(1).strip() if m else None

    m = _SET_VAL_RE.match(text)
    if m and m.group(1) == target_var:
        return m.group(2).strip()
    return None


def _parse_value_body_map(
    cmd: SegmentedCommand,
) -> list[tuple[str, str]] | None:
    """Parse an if/elseif chain into (value, body) pairs."""
    texts = cmd.texts
    mapping: list[tuple[str, str]] = []

    i = 1
    while i < len(texts):
        word = texts[i]
        if word in ("elseif", "then"):
            i += 1
            continue
        if word == "else":
            break

        condition = word
        if i + 1 >= len(texts):
            return None
        body = texts[i + 1]
        i += 2

        parsed = _parse_eq(condition)
        if parsed is None:
            return None
        _var, value, negated = parsed
        if negated:
            return None
        mapping.append((value, body))

    return mapping if len(mapping) >= 2 else None


def _parse_braced_pairs(text: str) -> list[tuple[str, str]]:
    """Parse ``{pat1 {body1} pat2 {body2}}`` into pairs."""
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
    """Simple brace-aware tokeniser."""
    tokens: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        while i < n and text[i] in " \t\n\r":
            i += 1
        if i >= n:
            break
        if text[i] == "{":
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
            start = i
            while i < n and text[i] not in " \t\n\r{}":
                i += 1
            tokens.append(text[start:i])
    return tokens


def _escape_dg_value(value: str) -> str:
    """Escape a data-group value for tmsh syntax."""
    if " " in value or '"' in value or "{" in value:
        return f'"{value}"'
    return value


def _get_indent(line: str) -> str:
    return line[: len(line) - len(line.lstrip())]


def _reindent_body(body: str, target_indent: str) -> str:
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
        if ln.strip():
            result_lines.append(f"{target_indent}{ln[min_indent:]}")
        elif result_lines:
            result_lines.append("")
    return "\n".join(result_lines)
