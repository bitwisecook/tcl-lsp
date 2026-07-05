# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Go-to-definition provider -- jump to proc/variable definitions."""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.proc_lookup import find_proc_by_reference
from analyser.semantic_model import AnalysisResult
from dialects.f5.bigip.model import BigipConfig
from dialects.f5.bigip.object_registry import (
    candidate_kinds_for_key,
    candidate_kinds_for_section_item,
    kind_for_header,
    resolve_kind_in_configs,
)
from dialects.f5.bigip.parser import parse_bigip_conf
from dialects.f5.bigip.parser._helpers import (
    _extract_blocks,
    _parse_generic_header,
    _parse_properties_with_spans,
)
from dialects.f5.bigip.registry import (
    candidate_registry_kinds_for_display,
    references_via_spec,
)
from dialects.f5.bigip.registry.pilot import pilot_property_spec_for
from server._lsp_conv import find_var_in_scopes, to_lsp_location
from shared.alias import lookup_alias_for_word
from shared.document_buffer import DocumentBuffer
from shared.position import position_in_range

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

    # Check for class definition
    for _qname, class_def in analysis.all_classes.items():
        if (
            class_def.name == word
            or class_def.qualified_name == word
            or class_def.qualified_name == f"::{word}"
        ):
            return [to_lsp_location(uri, class_def.name_range)]

    # Check for method definition — if inside a class body, resolve method names
    scope = find_scope_at_line(analysis.global_scope, line)
    if scope.kind == "method" and scope.parent:
        parent_scope = scope.parent
        for _qname, class_def in analysis.all_classes.items():
            if class_def.name == parent_scope.name or class_def.qualified_name == parent_scope.name:
                # Check if word is a method in this class (for `my method` calls)
                if word in class_def.methods:
                    return [to_lsp_location(uri, class_def.methods[word].name_range)]
                if word in class_def.class_methods:
                    return [to_lsp_location(uri, class_def.class_methods[word].name_range)]
                break

    # Check if word is a command alias — follow to target proc definition.
    if analysis.command_aliases:
        alias_info = lookup_alias_for_word(word, analysis.command_aliases)
        if alias_info is not None:
            target_cmd, _prepended = alias_info
            target_match = find_proc_by_reference(analysis, target_cmd)
            if target_match is not None:
                _qname, proc_def = target_match
                return [to_lsp_location(uri, proc_def.name_range)]

    return []


def _bigip_definition_via_registry(
    source: str,
    self_uri: str,
    line: int,
    character: int,
    configs: dict[str, BigipConfig],
) -> list[types.Location]:
    """Find a definition target by scanning every migrated property
    for a reference whose source span covers the cursor.

    Uses the value-spec dispatch (:func:`references_via_spec`) so
    any property the registry's pilot table covers — including
    nested references inside keyed-block lists (firewall rules'
    destination address-lists, profile attachments, cert-key-chain
    cert refs, ...) — gets exact-byte navigation without per-feature
    regex seeding.
    """
    import server.state as _state

    buffer = _state.document_buffer_for(self_uri, source)
    try:
        cursor_offset = _line_character_to_offset(buffer, line, character)
    except ValueError:
        return []
    for block in _extract_blocks(source):
        if not (block.start_offset <= cursor_offset <= block.end_offset):
            continue
        generic = _parse_generic_header(block.header)
        if generic is None:
            continue
        module, object_type, identifier = generic
        body_base = block.start_offset + 1
        prop_map = _parse_properties_with_spans(block.body)
        for key, prop in prop_map.items():
            if pilot_property_spec_for(module, object_type, key) is None:
                continue
            if prop.value_start is None:
                continue
            base = body_base + prop.value_start
            refs = references_via_spec(
                module=module,
                object_type=object_type,
                property_name=key,
                value=prop.value,
                owner_path=identifier,
                source_uri=self_uri,
                base_offset=base,
                source_text=source,
            )
            for ref in refs or ():
                if ref.range is None:
                    continue
                # ``ref.range`` is half-open ``[start, end)`` so the
                # cursor sitting exactly on ``end`` is *past* the
                # reference, not on it.  Treating it as inside would
                # fire go-to-definition for the character after the
                # token (e.g. the trailing brace).
                if not (ref.range.start <= cursor_offset < ref.range.end):
                    continue
                for kind in candidate_registry_kinds_for_display(ref.target_kind):
                    resolved = resolve_kind_in_configs(
                        kind, ref.target_path, configs, preferred_module=module
                    )
                    if resolved is not None:
                        target_uri, target_range = resolved
                        return [to_lsp_location(target_uri, target_range)]
    return []


def _line_character_to_offset(buffer: DocumentBuffer, line: int, character: int) -> int:
    """Map an LSP ``(line, character)`` to an absolute byte offset.

    Raises :class:`ValueError` when the position is out of range so
    callers fall back to legacy paths."""
    line_starts = buffer.line_starts
    if line < 0 or line >= len(line_starts):
        raise ValueError(f"line {line} out of range")
    return line_starts[line] + character


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
# NOTE: the legacy ``_BIGIP_CLASS_PATTERNS`` and
# ``_BIGIP_RULE_BODY_PATTERNS`` regex catalogues used to live here —
# nine hardcoded ``\b<keyword>\s+([^\s{}]+)`` patterns plus three
# ``class match`` shapes — and missed every iRule command that wasn't
# in that fixed list.  They've been replaced by
# :func:`_resolve_irule_body_definition`, which routes the cursor
# through :func:`extract_irules_object_references` (the same iRule
# scanner ``f5 grep`` / cleanup linter / document links use).  Adding
# a new command-arg → kind mapping to
# ``dialects/f5/bigip/data/irules_object_refs_graph.json`` now lights up
# go-to-definition automatically.

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


def _is_section_name(key: str) -> bool:
    """True when *key* names a nested section (a bare identifier).

    Mirrors the old ``_BIGIP_SECTION_OPEN_RE`` ``[A-Za-z0-9_-]+``
    character class: keyed-list entries whose names are object paths
    (``/Common/web1:80``) are transparent — the enclosing *section*
    is the nearest bare-identifier sub-block, exactly as the legacy
    line-walk excluded ``stripped.startswith("/")`` from the stack.
    """
    return bool(key) and all(c.isalnum() or c in "_-" for c in key)


def _structural_context_at_offset(
    block_body: str,
    body_base: int,
    cursor_offset: int,
) -> tuple[str | None, str | None]:
    """Walk a parsed block body and return ``(section, key)`` at the cursor.

    Both values are derived from the parsed structure
    (:func:`_parse_properties_with_spans`), not line regexes:

    * ``section`` is the innermost enclosing bare-identifier sub-block
      whose braced body span covers the cursor (the parser-driven
      replacement for ``_scan_section_stack``'s ``stack[-1]``).
    * ``key`` is the property under the cursor — the first token of
      that property's ``key value`` line (replacing the inline
      ``^\\s*([A-Za-z0-9_-]+)\\s+...`` regex).  It's reported whether
      the cursor sits on the key or anywhere in the value.

    The walk descends through nested sub-blocks so the section/key the
    cursor lands in reflects the full brace nesting.
    """
    section: str | None = None
    key: str | None = None

    props = _parse_properties_with_spans(block_body)
    for prop in props.values():
        if prop.value_start is None or prop.value_end is None:
            continue
        val_end = body_base + prop.value_end
        # Reconstruct the key's local start by stepping back from the
        # value over the inter-token whitespace and then the key token,
        # so a cursor on the key (not just the value) still attributes
        # to this property.
        sep = prop.value_start
        while sep > 0 and block_body[sep - 1] in " \t":
            sep -= 1
        key_local_start = sep - len(prop.key)
        key_start = body_base + key_local_start
        if not (key_start <= cursor_offset < val_end):
            continue

        # This property owns the cursor.
        if prop.value.startswith("{"):
            # Braced sub-block: this is a *section*, not a ``key value``
            # line.  Descend so the cursor's real section/key come from
            # the inner level.  ``value_start`` points at the opening
            # ``{``; the body runs to just before the matching ``}`` at
            # ``value_end - 1``.
            inner_open = body_base + prop.value_start
            if inner_open < cursor_offset:
                inner_body = block_body[prop.value_start + 1 : prop.value_end - 1]
                inner_base = inner_open + 1
                inner_section, inner_key = _structural_context_at_offset(
                    inner_body, inner_base, cursor_offset
                )
                # The enclosing section for the cursor is this sub-block
                # when it is a bare-identifier section; otherwise the
                # path-named entry is transparent and we keep whatever
                # the deeper level resolved.
                if _is_section_name(prop.key):
                    section = inner_section if inner_section is not None else prop.key
                else:
                    section = inner_section
                key = inner_key
        else:
            # Simple ``key value`` line — the cursor's line key.
            key = prop.key
        break

    return (section, key)


def _bigip_structure_at_cursor(
    source: str,
    cursor_offset: int,
) -> tuple[str, str, str, int, int, str | None, str | None] | None:
    """Parser-driven structural context for the cursor's BIG-IP stanza.

    Returns ``(module, object_type, identifier, ident_start, ident_end,
    section, key)`` for the top-level block whose ``{ ... }`` span (or
    header) covers *cursor_offset*, all derived from
    :func:`_extract_blocks` / :func:`_parse_generic_header` /
    :func:`_parse_properties_with_spans` rather than line regexes.

    ``ident_start`` / ``ident_end`` give the absolute half-open span of
    the stanza identifier inside the header (``-1`` when there is no
    identifier).  ``section`` / ``key`` are the nested section name and
    the property key under the cursor (``None`` when the cursor is on
    the header rather than inside the body).
    """
    for block in _extract_blocks(source):
        if not (block.start_offset <= cursor_offset <= block.end_offset):
            continue
        generic = _parse_generic_header(block.header)
        if generic is None:
            return None
        module, object_type, identifier = generic

        # Header span: from the block start back to where the header
        # begins.  ``start_offset`` is the opening brace; the header
        # text precedes it.  Locate the identifier within that slice.
        header_text_start = block.start_offset - len(block.header)
        # The header may have trailing whitespace before the brace; find
        # the real header start by matching the parsed header string.
        actual_start = source.rfind(block.header, 0, block.start_offset)
        if actual_start != -1:
            header_text_start = actual_start
        ident_start = -1
        ident_end = -1
        if identifier:
            # Search for the identifier within the header slice only, so a
            # value with the same text deeper in the body can't match.
            rel = block.header.rfind(identifier)
            if rel != -1:
                ident_start = header_text_start + rel
                ident_end = ident_start + len(identifier)

        # When the cursor sits inside the body, derive section + key.
        section: str | None = None
        key: str | None = None
        body_base = block.start_offset + 1
        if body_base <= cursor_offset < block.end_offset:
            section, key = _structural_context_at_offset(block.body, body_base, cursor_offset)

        return (module, object_type, identifier, ident_start, ident_end, section, key)
    return None


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

    # Registry-first dispatch: ask the value-spec layer if any
    # migrated property's reference covers the cursor.  Falls through
    # to the parser-driven candidate_kinds_for_key / section-item path
    # when no registered spec owns the cursor position (so unmigrated
    # properties keep working unchanged).
    registry_hit = _bigip_definition_via_registry(source, uri, line, character, configs)
    if registry_hit:
        return registry_hit

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

    if not _cursor_in_span(character, token_start, token_end):
        return []

    # Map the cursor to an absolute offset so the structural context can
    # be read straight from the parsed block model.
    import server.state as _state

    buffer = _state.document_buffer_for(uri, source)
    try:
        cursor_offset = _line_character_to_offset(buffer, line, character)
    except ValueError:
        return []

    containing_header = _containing_bigip_header(current_config, line)
    container_module = containing_header[0] if containing_header else None
    container_object_type = containing_header[1] if containing_header else None

    # Structural context (container header, nested section, key under the
    # cursor) is read from the parsed block model — ``_extract_blocks`` /
    # ``_parse_generic_header`` / ``_parse_properties_with_spans`` — not
    # from line regexes.
    structure = _bigip_structure_at_cursor(source, cursor_offset)
    current_section = structure[5] if structure else None
    line_key = structure[6] if structure else None

    # Key/value lines: e.g. "pool /Common/p", "monitor /Common/m".
    if line_key is not None:
        for kind in candidate_kinds_for_key(
            line_key,
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
    if structure is not None:
        module, object_type, ident, ident_start, ident_end, _section, _key = structure
        if ident and ident_start >= 0 and ident_start <= cursor_offset < ident_end:
            kind = kind_for_header(module, object_type)
            if kind is not None:
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

    # iRule source refs in embedded "ltm rule" bodies.  Driven by the
    # full iRule command registry (every command argument that can name
    # a BIG-IP object is covered) instead of nine hand-rolled regexes.
    if container_module in {"ltm", "gtm"} and container_object_type == "rule":
        target = _resolve_irule_body_definition(
            current_config,
            line,
            character,
            container_module,
            configs,
        )
        if target is not None:
            return [target]

    return []


def _resolve_irule_body_definition(
    current_config: BigipConfig,
    line: int,
    character: int,
    container_module: str,
    configs: dict[str, BigipConfig],
) -> types.Location | None:
    """Locate the iRule body reference at (*line*, *character*) and resolve it.

    Replaces the legacy 9-regex catalogue with a parser-driven walk:
    :func:`extract_irules_object_references` runs the same iRule
    scanner ``f5 grep`` / cleanup linter / document links use, so
    every command-argument → kind mapping in the registry lights up
    here automatically.  Resolution falls through to the same
    :func:`resolve_kind_in_configs` the legacy path used.
    """
    from dialects.f5.bigip.irules_refs import extract_irules_object_references

    # Find the rule whose body contains this cursor position.
    rule = None
    rule_body_start_line = 0
    for candidate in current_config.rules.values():
        rng = candidate.range
        if rng is None:
            continue
        if rng.start.line <= line <= rng.end.line:
            rule = candidate
            rule_body_start_line = rng.start.line + 1
            break
    if rule is None:
        return None

    # The references' ranges are body-relative; the cursor we got is
    # document-absolute, so shift back into body coordinates before
    # comparing.
    body_line = line - rule_body_start_line
    if body_line < 0:
        return None

    for ref in extract_irules_object_references(rule.source):
        ref_start = ref.range.start
        ref_end = ref.range.end
        if not (ref_start.line <= body_line <= ref_end.line):
            continue
        if body_line == ref_start.line and character < ref_start.character:
            continue
        if body_line == ref_end.line and character > ref_end.character:
            continue
        # Try each candidate kind in the registry order.  The renamer
        # uses the same approach (first kind that resolves wins).
        for kind in ref.kinds:
            ref_text = _normalise_reference_for_kind(kind, ref.name)
            resolved = resolve_kind_in_configs(
                kind,
                ref_text,
                configs,
                preferred_module=container_module,
            )
            if resolved is not None:
                target_uri, target_range = resolved
                return to_lsp_location(target_uri, target_range)
    return None
