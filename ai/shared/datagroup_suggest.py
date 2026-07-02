"""AI-enhanced data-group extraction scan.

This is an **AI-only heuristic**: it scans iRules source for `if`/`switch`
patterns that could become data-groups and returns structured context
(pattern type, inferred value type, CIDR detection, body-shape analysis,
confidence) for an LLM to refine. Per the "AI-only stays Python" rule it lives
here rather than in the Rust facade surface, but it is **decoupled from the
retiring `tooling`/`compiler` packages**: segmentation comes from the
`tcl_lsp_py.walk_commands` / `parse_tcl` facades and the static extractability
check from `tcl_lsp_py.refactor_extract_datagroup`.

Ported from the former `tooling.refactoring._extract_datagroup`.
"""

from __future__ import annotations

import ipaddress
import re
from typing import Any

_DIALECT = "f5-irules"


def _rust() -> Any:
    from ai.shared.rust_bridge import require_rust

    return require_rust()


# ── Value-type inference ──────────────────────────────────────────────


def _strip_quotes(s: str) -> str:
    s = s.strip()
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _is_ip_or_cidr(value: str) -> bool:
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
    if not values:
        return "string"
    stripped = [_strip_quotes(v) for v in values]
    if all(_is_ip_or_cidr(v) for v in stripped):
        return "ip"
    if all(_is_integer(v) for v in stripped):
        return "integer"
    return "string"


def _normalise_dg_name(name: str) -> str:
    name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
    name = re.sub(r"_+", "_", name).strip("_").lower()
    return name or "extracted_dg"


# ── Condition parsing ─────────────────────────────────────────────────

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
_OR_SPLIT_RE = re.compile(r"\s*\|\|\s*")


def _parse_eq(cond: str) -> tuple[str, str, bool] | None:
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


def _parse_set_or_return(text: str) -> tuple[str, str, str] | None:
    """Parse a single-command arm body as ``set var val`` / ``return val``.

    Only the command kind and (for ``set``) the variable name are consumed by
    the body-shape classifier, so the value word is taken from the top-level
    segmentation (`parse_tcl`) — no raw source span is needed here.
    """
    result = _rust().parse_tcl(text, dialect=_DIALECT)
    if len(result.commands) != 1:
        return None
    cmd = result.commands[0]
    texts = [cmd.name, *cmd.args]
    if texts[0] == "set" and len(texts) == 3:
        return ("set", texts[1], texts[2])
    if texts[0] == "return" and len(texts) == 2:
        return ("return", "", texts[1])
    return None


# ── Body-shape / helper analysis ──────────────────────────────────────


def _try_or_chain(condition: str) -> tuple[str, list[str]] | None:
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
    s = subject.strip()
    if s.startswith("${") and s.endswith("}"):
        return s[2:-1]
    if s.startswith("$"):
        name = s[1:]
        if re.match(r"^[A-Za-z_]\w*$", name):
            return name
    return None


def _classify_body_shape(bodies: list[str]) -> str:
    target_var = None
    use_return = None

    for body in bodies:
        text = body.strip()
        if text.startswith("{") and text.endswith("}"):
            text = text[1:-1].strip()

        parsed = _parse_set_or_return(text)
        if parsed is not None and parsed[0] == "set":
            var = parsed[1]
            if target_var is None:
                target_var = var
                use_return = False
            elif var != target_var or use_return:
                return "complex"
            continue

        if parsed is not None and parsed[0] == "return":
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


def _tokenise_switch_body(text: str) -> list[str]:
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


def _parse_braced_pairs(text: str) -> list[tuple[str, str]]:
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


# ── Pattern analysis ──────────────────────────────────────────────────


def _analyse_if_chain(texts: list[str], line: int) -> dict | None:
    if len(texts) < 3:
        return None

    target_var: str | None = None
    values: list[str] = []
    bodies: list[str] = []

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

    body_set = {b.strip() for b in bodies}
    body_shape = "identical" if len(body_set) == 1 else _classify_body_shape(bodies)

    confidence = (
        "high" if body_shape in ("identical", "set_mapping", "return_mapping") else "medium"
    )

    return {
        "line": line,
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


def _analyse_switch(texts: list[str], line: int) -> dict | None:
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
    body_shape = "identical" if len(set(bodies)) == 1 else _classify_body_shape(bodies)

    confidence = (
        "high" if body_shape in ("identical", "set_mapping", "return_mapping") else "medium"
    )

    return {
        "line": line,
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


def suggest_datagroup_extraction(source: str) -> list[dict]:
    """Scan *source* for `if`/`switch` patterns extractable to data-groups.

    Each candidate carries structured context for an LLM plus ``static_result``
    — the `tcl_lsp_py.refactor_extract_datagroup` result when the static
    extractor can handle it, else ``None``.
    """
    rust = _rust()
    candidates: list[dict] = []

    for cmd in rust.walk_commands(source, dialect=_DIALECT):
        texts = cmd["texts"]
        if not texts:
            continue
        line = cmd["line"]

        cand: dict | None = None
        if texts[0] == "if":
            cand = _analyse_if_chain(texts, line)
        elif texts[0] == "switch":
            cand = _analyse_switch(texts, line)

        if cand is not None:
            cand["static_result"] = rust.refactor_extract_datagroup(
                source, line, cmd.get("character", 0), dialect=_DIALECT
            )
            candidates.append(cand)

    return candidates
