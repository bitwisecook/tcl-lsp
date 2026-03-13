"""Go-to-definition provider -- jump to proc/variable definitions."""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult
from core.bigip.model import BigipConfig
from core.bigip.object_registry import (
    candidate_kinds_for_key,
    candidate_kinds_for_section_item,
    kind_for_header,
    resolve_kind_in_configs,
)
from core.bigip.parser import parse_bigip_conf
from core.common.lsp import find_var_in_scopes, to_lsp_location
from core.common.position import position_in_range

from .symbol_resolution import find_scope_at_line, find_var_at_position, find_word_at_position


def get_definition(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.Location]:
    """Find definition locations for the symbol at the given position."""
    if analysis is None:
        analysis = analyse(source)

    # Check for variable definition ($var → set var)
    var_name = find_var_at_position(source, line, character)
    if var_name:
        scope = find_scope_at_line(analysis.global_scope, line)
        var_def = find_var_in_scopes(var_name, scope)
        if var_def:
            return [to_lsp_location(uri, var_def.definition_range)]
        return []

    # Check for proc definition
    word = find_word_at_position(source, line, character)
    if not word:
        return []

    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is not None:
        _qname, proc_def = proc_match
        return [to_lsp_location(uri, proc_def.name_range)]

    return []


def _resolve_pool_across_configs(
    pool_ref: str,
    configs: dict[str, BigipConfig],
    *,
    preferred_module: str | None = None,
) -> tuple[str, str] | None:
    """Resolve ``pool_ref`` across all BIG-IP configs as ``(uri, full_path)``."""
    # Exact match first
    for cfg_uri, cfg in configs.items():
        pool = cfg.pools.get(pool_ref)
        if pool is not None:
            if preferred_module is not None and pool.module != preferred_module:
                continue
            return (cfg_uri, pool_ref)

    # Then per-config resolver (short names, /Common prefix, suffix match)
    for cfg_uri, cfg in configs.items():
        resolved = cfg.resolve_pool(pool_ref)
        if resolved and resolved in cfg.pools:
            pool = cfg.pools[resolved]
            if preferred_module is not None and pool.module != preferred_module:
                continue
            return (cfg_uri, resolved)
    return None


_BIGIP_PATH_DELIMS = " \t\n\r;{}[]\"'"
_BIGIP_SECTION_OPEN_RE = re.compile(r"^\s*([A-Za-z0-9_-]+)\s*\{\s*(?:#.*)?$")
_BIGIP_HEADER_LINE_RE = re.compile(
    r"^\s*(ltm|gtm|net|auth|cm|sys|security)\s+(.+?)\s*\{\s*(?:#.*)?$"
)
_BIGIP_CLASS_OPERATORS = r"equals|starts_with|ends_with|contains|matches_glob|matches_regex"
_BIGIP_CLASS_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(
        rf"\bclass\s+(?:match|search)\s+(?:(?:-\w+\s+)*)"
        rf"(?:--\s+)?\S+\s+(?:{_BIGIP_CLASS_OPERATORS})\s+([^\s{{}}]+)"
    ),
    re.compile(r"\bclass\s+lookup\s+(?:--\s+)?\S+\s+([^\s{}]+)"),
    re.compile(r"\bclass\s+(?:exists|size|type|get|startsearch)\s+([^\s{}]+)"),
)
_BIGIP_RULE_BODY_PATTERNS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"\bpool\s+([^\s{}]+)"), "pool"),
    (re.compile(r"\bsnatpool\s+([^\s{}]+)"), "snat_pool"),
    (re.compile(r"\bpersist\s+([^\s{}]+)"), "persistence"),
    (re.compile(r"\bvirtual\s+([^\s{}]+)"), "virtual"),
    (re.compile(r"\bnode\s+([^\s{}]+)"), "node"),
    (re.compile(r"\bprofile\s+([^\s{}]+)"), "profile"),
    (re.compile(r"\bmonitor\s+([^\s{}]+)"), "monitor"),
    (re.compile(r"\broute\s+([^\s{}]+)"), "route"),
    (re.compile(r"\bvlan\s+([^\s{}]+)"), "vlan"),
)

_BIGIP_FALSEY_REF_TOKENS = frozenset(
    {
        "none",
        "add",
        "delete",
        "modify",
        "replace-all-with",
        "enabled",
        "disabled",
        "default",
        "all",
        "and",
        "or",
        "context",
        "clientside",
        "serverside",
        "true",
        "false",
    }
)


def _extract_token_at_cursor(
    line_text: str,
    character: int,
) -> tuple[str, int, int] | None:
    """Return ``(token, start, end_exclusive)`` under cursor."""
    if not line_text:
        return None
    col = min(max(character, 0), len(line_text))

    start = col
    while start > 0 and line_text[start - 1] not in _BIGIP_PATH_DELIMS:
        start -= 1
    end = col
    while end < len(line_text) and line_text[end] not in _BIGIP_PATH_DELIMS:
        end += 1
    if start == end:
        return None
    return (line_text[start:end], start, end)


def _cursor_in_span(character: int, start: int, end_exclusive: int) -> bool:
    return start <= character < end_exclusive


def _normalise_reference_for_kind(kind: str, token: str) -> str:
    """Normalise a source token for object-resolution lookup."""
    ref = token.strip("{}\"'[](),")
    if kind in {"node", "virtual_address", "ltm_node", "ltm_virtual_address"}:
        if ":" in ref and ref.count(":") == 1:
            left, right = ref.rsplit(":", 1)
            if right.isdigit():
                ref = left
    return ref


def _is_candidate_reference(token: str) -> bool:
    clean = token.strip("{}\"'[](),")
    if not clean:
        return False
    return clean.lower() not in _BIGIP_FALSEY_REF_TOKENS


def _parse_bigip_header_line(line_text: str) -> tuple[str, str, str, int, int] | None:
    """Parse one BIG-IP header line into module, type, identifier and span."""
    match = _BIGIP_HEADER_LINE_RE.match(line_text)
    if match is None:
        return None
    module = match.group(1)
    rest = match.group(2).strip()
    if not rest:
        return None
    parts = rest.split()
    object_type = parts[0]
    identifier = ""
    if len(parts) >= 2:
        object_type = " ".join(parts[:-1])
        identifier = parts[-1]
    ident_start = line_text.find(identifier) if identifier else -1
    ident_end = ident_start + len(identifier) if ident_start >= 0 else -1
    return (module, object_type, identifier, ident_start, ident_end)


def _containing_bigip_header(
    config: BigipConfig,
    line: int,
) -> tuple[str, str] | None:
    """Return ``(module, object_type)`` for the containing BIG-IP stanza."""
    for obj in config.generic_objects.values():
        rng = obj.range
        if rng is None:
            continue
        if rng.start.line <= line <= rng.end.line:
            return (obj.module, obj.object_type)
    return None


def _scan_section_stack(lines: list[str], line: int) -> list[str]:
    """Best-effort nested section stack at *line* (inside a BIG-IP object)."""
    stack: list[str] = []
    for idx in range(0, min(line, len(lines))):
        text = lines[idx]
        stripped = text.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if _BIGIP_HEADER_LINE_RE.match(text):
            stack.clear()
            continue
        m = _BIGIP_SECTION_OPEN_RE.match(text)
        if m and not stripped.startswith("/"):
            opens = text.count("{")
            closes = text.count("}")
            if opens > closes:
                stack.append(m.group(1))
            continue
        closes = text.count("}")
        while closes > 0 and stack:
            stack.pop()
            closes -= 1
    return stack


def get_bigip_definition(
    source: str,
    uri: str,
    line: int,
    character: int,
    *,
    current_config: BigipConfig | None = None,
    workspace_configs: dict[str, BigipConfig] | None = None,
    lines: list[str] | None = None,
) -> list[types.Location]:
    """Go-to-definition for BIG-IP config references."""
    if current_config is None:
        current_config = parse_bigip_conf(source)

    configs = dict(workspace_configs or {})
    configs.setdefault(uri, current_config)

    if lines is None:
        lines = source.split("\n")
    if line < 0 or line >= len(lines):
        return []
    line_text = lines[line]
    token_info = _extract_token_at_cursor(line_text, character)

    # Fast-path for virtual default-pool using parser-provided token span.
    for vs in current_config.virtual_servers.values():
        if not vs.pool or vs.pool_range is None:
            continue
        # Accept cursor inside the pool_range (inclusive) or immediately
        # after the last character, matching end-exclusive LSP cursor
        # semantics used by _cursor_in_span() downstream.
        if not position_in_range(line, character, vs.pool_range) and not (
            line == vs.pool_range.end.line and character == vs.pool_range.end.character + 1
        ):
            continue

        resolved = _resolve_pool_across_configs(vs.pool, configs, preferred_module="ltm")
        if resolved is None:
            return []
        pool_uri, pool_path = resolved
        pool_obj = configs[pool_uri].pools.get(pool_path)
        if pool_obj and pool_obj.range is not None:
            return [to_lsp_location(pool_uri, pool_obj.range)]
        return []

    if token_info is None:
        return []
    token, token_start, token_end = token_info

    containing_header = _containing_bigip_header(current_config, line)
    container_module = containing_header[0] if containing_header else None
    container_object_type = containing_header[1] if containing_header else None
    section_stack = _scan_section_stack(lines, line)
    current_section = section_stack[-1] if section_stack else None

    # Key/value lines: e.g. "pool /Common/p", "monitor /Common/m".
    key_match = re.match(r"^\s*([A-Za-z0-9_-]+)\s+(.+?)\s*(?:#.*)?$", line_text)
    if key_match and _cursor_in_span(character, token_start, token_end):
        key = key_match.group(1)
        for kind in candidate_kinds_for_key(
            key,
            section=current_section,
            container_module=container_module,
            container_object_type=container_object_type,
        ):
            if not _is_candidate_reference(token):
                continue
            ref = _normalise_reference_for_kind(kind, token)
            resolved = resolve_kind_in_configs(
                kind,
                ref,
                configs,
                preferred_module=container_module,
            )
            if resolved is not None:
                target_uri, target_range = resolved
                return [to_lsp_location(target_uri, target_range)]

    # Section list entries (profiles/rules/persist/members/policies/vlans/etc).
    if _cursor_in_span(character, token_start, token_end):
        section = current_section or ""
        for kind in candidate_kinds_for_section_item(
            section,
            container_module=container_module,
            container_object_type=container_object_type,
        ):
            if not _is_candidate_reference(token):
                continue
            ref = _normalise_reference_for_kind(kind, token)
            resolved = resolve_kind_in_configs(
                kind,
                ref,
                configs,
                preferred_module=container_module,
            )
            if resolved is not None:
                target_uri, target_range = resolved
                return [to_lsp_location(target_uri, target_range)]

    # Top-level headers that reference named objects (e.g. auth user admin).
    parsed_header = _parse_bigip_header_line(line_text)
    if parsed_header and _cursor_in_span(character, token_start, token_end):
        module, object_type, ident, ident_start, ident_end = parsed_header
        if ident and ident_start >= 0 and _cursor_in_span(character, ident_start, ident_end):
            kind = kind_for_header(module, object_type)
        else:
            kind = None
        if kind is not None and ident:
            ref = _normalise_reference_for_kind(kind, ident)
            resolved = resolve_kind_in_configs(
                kind,
                ref,
                configs,
                preferred_module=module,
            )
            if resolved is not None:
                target_uri, target_range = resolved
                return [to_lsp_location(target_uri, target_range)]

    # iRule source refs in embedded "ltm rule" bodies.
    if container_module in {"ltm", "gtm"} and container_object_type == "rule":
        for pattern, kind in _BIGIP_RULE_BODY_PATTERNS:
            for match in pattern.finditer(line_text):
                start, end = match.span(1)
                if not _cursor_in_span(character, start, end):
                    continue
                resolved_kind = kind
                if kind == "pool" and container_module in {"ltm", "gtm"}:
                    resolved_kind = f"{container_module}_pool"
                ref = _normalise_reference_for_kind(resolved_kind, match.group(1))
                resolved = resolve_kind_in_configs(
                    resolved_kind,
                    ref,
                    configs,
                    preferred_module=container_module,
                )
                if resolved is not None:
                    target_uri, target_range = resolved
                    return [to_lsp_location(target_uri, target_range)]

        for pattern in _BIGIP_CLASS_PATTERNS:
            for match in pattern.finditer(line_text):
                start, end = match.span(1)
                if not _cursor_in_span(character, start, end):
                    continue
                for kind in ("ltm_data_group_internal", "ltm_data_group_external"):
                    ref = _normalise_reference_for_kind(kind, match.group(1))
                    resolved = resolve_kind_in_configs(
                        kind,
                        ref,
                        configs,
                        preferred_module=container_module,
                    )
                    if resolved is not None:
                        target_uri, target_range = resolved
                        return [to_lsp_location(target_uri, target_range)]

    return []
