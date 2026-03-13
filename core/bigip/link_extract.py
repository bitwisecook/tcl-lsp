"""Transitive BIG-IP object link extraction from configuration sources."""

from __future__ import annotations

import os
import re
from collections import defaultdict, deque
from dataclasses import dataclass

from core.analysis.semantic_model import Range

from .irules_refs import extract_irules_object_references
from .model import BigipConfig
from .object_registry import (
    candidate_kinds_for_key,
    candidate_kinds_for_section_item,
    kind_for_header,
    resolve_kind_in_configs,
)
from .parser import (
    _extract_blocks,
    _parse_generic_header,
    _parse_list_block,
    _parse_properties_with_spans,
)

# Mapping from config file basenames to source-origin tags.
_SOURCE_ORIGIN_MAP: dict[str, str] = {
    "bigip_base.conf": "base",
    "bigip.conf": "synced",
    "bigip_gtm.conf": "synced",
    "bigip_user.conf": "synced",
    "bigip_script.conf": "script",
}


def _source_origin_for_uri(uri: str) -> str | None:
    """Classify a source URI as ``base``, ``synced``, or ``script``."""
    # Strip query/fragment and extract the final path component.
    path = uri.split("?", 1)[0].split("#", 1)[0]
    basename = os.path.basename(path)
    return _SOURCE_ORIGIN_MAP.get(basename)


_FALSEY_REF_TOKENS = frozenset(
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

_TOKEN_RE = re.compile(r"[^\s{}]+")


@dataclass(slots=True)
class _BlockObject:
    node_id: str
    uri: str
    module: str
    object_type: str
    identifier: str
    kind: str | None
    header: str
    body: str
    header_start_offset: int
    start_offset: int
    end_offset: int
    range: Range


@dataclass(slots=True)
class _Edge:
    source_id: str
    target_id: str
    via_property: str
    via_kind: str


def _range_key(rng: Range) -> tuple[int, int, int, int]:
    return (rng.start.line, rng.start.character, rng.end.line, rng.end.character)


def _contains_offset(obj: _BlockObject, offset: int) -> bool:
    return obj.header_start_offset <= offset <= obj.end_offset


def _normalise_reference_for_kind(kind: str, token: str) -> str:
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
    return clean.lower() not in _FALSEY_REF_TOKENS


def _extract_value_tokens(value: str) -> list[str]:
    stripped = value.strip()
    if not stripped:
        return []
    if stripped.startswith("{") and stripped.endswith("}"):
        # List-style sections (rules/profiles/members/etc.)
        return _parse_list_block(stripped)
    return [match.group(0) for match in _TOKEN_RE.finditer(stripped)]


def _build_objects_for_source(uri: str, source: str) -> list[_BlockObject]:
    from core.common.source_map import SourceMap

    result: list[_BlockObject] = []
    source_map = SourceMap(source)
    for block in _extract_blocks(source):
        generic = _parse_generic_header(block.header)
        if generic is None:
            continue
        module, object_type, identifier = generic
        header_start = block.start_offset
        while header_start > 0 and source[header_start - 1] != "\n":
            header_start -= 1
        obj_range = source_map.range_from_offsets(block.start_offset, block.end_offset)
        node_id = (
            f"{uri}:{obj_range.start.line}:{obj_range.start.character}:"
            f"{module}:{object_type}:{identifier}"
        )
        result.append(
            _BlockObject(
                node_id=node_id,
                uri=uri,
                module=module,
                object_type=object_type,
                identifier=identifier,
                kind=kind_for_header(module, object_type),
                header=block.header,
                body=block.body,
                header_start_offset=header_start,
                start_offset=block.start_offset,
                end_offset=block.end_offset,
                range=obj_range,
            )
        )
    return result


def _resolve_target_node_id(
    kind: str,
    ref: str,
    source_module: str,
    configs: dict[str, BigipConfig],
    nodes_by_range: dict[str, dict[tuple[int, int, int, int], str]],
    nodes_by_uri: dict[str, dict[str, _BlockObject]],
) -> str | None:
    resolved = resolve_kind_in_configs(
        kind,
        ref,
        configs,
        preferred_module=source_module,
    )
    if resolved is None:
        return None
    target_uri, target_range = resolved
    direct = nodes_by_range.get(target_uri, {}).get(_range_key(target_range))
    if direct is not None:
        return direct

    for node in nodes_by_uri.get(target_uri, {}).values():
        rng = node.range
        if (
            rng.start.line <= target_range.start.line <= rng.end.line
            and rng.start.line <= target_range.end.line <= rng.end.line
        ):
            return node.node_id
    return None


def _build_forward_edges(
    nodes_by_uri: dict[str, dict[str, _BlockObject]],
    configs: dict[str, BigipConfig],
) -> list[_Edge]:
    edges: list[_Edge] = []
    seen: set[tuple[str, str, str, str]] = set()

    nodes_by_range: dict[str, dict[tuple[int, int, int, int], str]] = {}
    for uri, by_id in nodes_by_uri.items():
        index: dict[tuple[int, int, int, int], str] = {}
        for node in by_id.values():
            index[_range_key(node.range)] = node.node_id
        nodes_by_range[uri] = index

    for by_id in nodes_by_uri.values():
        for node in by_id.values():
            prop_map = _parse_properties_with_spans(node.body)
            for key, prop in prop_map.items():
                key_kinds = candidate_kinds_for_key(
                    key,
                    section=None,
                    container_module=node.module,
                    container_object_type=node.object_type,
                )
                if key_kinds:
                    for token in _extract_value_tokens(prop.value):
                        if not _is_candidate_reference(token):
                            continue
                        for kind in key_kinds:
                            ref = _normalise_reference_for_kind(kind, token)
                            target_id = _resolve_target_node_id(
                                kind,
                                ref,
                                node.module,
                                configs,
                                nodes_by_range,
                                nodes_by_uri,
                            )
                            if target_id is None:
                                continue
                            edge_key = (node.node_id, target_id, key, kind)
                            if edge_key in seen:
                                continue
                            seen.add(edge_key)
                            edges.append(
                                _Edge(
                                    source_id=node.node_id,
                                    target_id=target_id,
                                    via_property=key,
                                    via_kind=kind,
                                )
                            )

                section_kinds = candidate_kinds_for_section_item(
                    key,
                    container_module=node.module,
                    container_object_type=node.object_type,
                )
                if not section_kinds:
                    continue
                for token in _parse_list_block(prop.value):
                    if not _is_candidate_reference(token):
                        continue
                    for kind in section_kinds:
                        ref = _normalise_reference_for_kind(kind, token)
                        target_id = _resolve_target_node_id(
                            kind,
                            ref,
                            node.module,
                            configs,
                            nodes_by_range,
                            nodes_by_uri,
                        )
                        if target_id is None:
                            continue
                        edge_key = (node.node_id, target_id, f"{key}[]", kind)
                        if edge_key in seen:
                            continue
                        seen.add(edge_key)
                        edges.append(
                            _Edge(
                                source_id=node.node_id,
                                target_id=target_id,
                                via_property=f"{key}[]",
                                via_kind=kind,
                            )
                        )

            if node.module in {"ltm", "gtm"} and node.object_type == "rule":
                for ref in extract_irules_object_references(node.body, rule_module=node.module):
                    for kind in ref.kinds:
                        target_id = _resolve_target_node_id(
                            kind,
                            ref.name,
                            node.module,
                            configs,
                            nodes_by_range,
                            nodes_by_uri,
                        )
                        if target_id is None:
                            continue
                        edge_key = (
                            node.node_id,
                            target_id,
                            f"irule:{ref.command}",
                            kind,
                        )
                        if edge_key in seen:
                            continue
                        seen.add(edge_key)
                        edges.append(
                            _Edge(
                                source_id=node.node_id,
                                target_id=target_id,
                                via_property=f"irule:{ref.command}",
                                via_kind=kind,
                            )
                        )

    return edges


def _find_root_at_offset(
    uri: str,
    offset: int,
    nodes_by_uri: dict[str, dict[str, _BlockObject]],
) -> _BlockObject | None:
    """Return the smallest object containing *offset* in *uri*, or ``None``."""
    current_nodes = list(nodes_by_uri.get(uri, {}).values())
    if not current_nodes:
        return None
    containing = [obj for obj in current_nodes if _contains_offset(obj, offset)]
    if not containing:
        return None
    return min(containing, key=lambda obj: obj.end_offset - obj.start_offset)


def extract_linked_bigip_objects(
    *,
    uri: str | None = None,
    offset: int | None = None,
    offsets: list[tuple[str, int]] | None = None,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    max_depth: int = 4,
    max_nodes: int = 250,
) -> dict | None:
    """Extract transitive linked BIG-IP objects around one or more cursor positions.

    Callers may pass *offsets* (a list of ``(uri, offset)`` tuples) to seed the
    graph from multiple cursor positions or selections.  The legacy *uri* /
    *offset* single-cursor interface is still supported for backward
    compatibility.
    """
    # Normalise to a list of (uri, offset) seeds.
    if offsets is not None:
        seed_positions = offsets
    elif uri is not None and offset is not None:
        seed_positions = [(uri, offset)]
    else:
        return None

    if not seed_positions:
        return None

    # Validate that every seed URI is available.
    for seed_uri, _ in seed_positions:
        if seed_uri not in sources or seed_uri not in configs:
            return None

    nodes_by_uri: dict[str, dict[str, _BlockObject]] = {}
    for src_uri, src in sources.items():
        if src_uri not in configs:
            continue
        objects = _build_objects_for_source(src_uri, src)
        nodes_by_uri[src_uri] = {obj.node_id: obj for obj in objects}

    # Resolve roots – one per cursor position, deduplicated.
    roots: list[_BlockObject] = []
    seen_root_ids: set[str] = set()
    for seed_uri, seed_offset in seed_positions:
        root = _find_root_at_offset(seed_uri, seed_offset, nodes_by_uri)
        if root is not None and root.node_id not in seen_root_ids:
            roots.append(root)
            seen_root_ids.add(root.node_id)

    if not roots:
        return None

    edges = _build_forward_edges(nodes_by_uri, configs)
    all_nodes: dict[str, _BlockObject] = {
        node.node_id: node for by_id in nodes_by_uri.values() for node in by_id.values()
    }

    outgoing: dict[str, list[_Edge]] = defaultdict(list)
    incoming: dict[str, list[_Edge]] = defaultdict(list)
    for edge in edges:
        outgoing[edge.source_id].append(edge)
        incoming[edge.target_id].append(edge)

    # BFS from all roots simultaneously (all start at depth 0).
    depths: dict[str, int] = {root.node_id: 0 for root in roots}
    queue: deque[str] = deque(root.node_id for root in roots)

    while queue and len(depths) < max_nodes:
        node_id = queue.popleft()
        depth = depths[node_id]
        if depth >= max_depth:
            continue

        neighbours: list[str] = []
        for edge in outgoing.get(node_id, []):
            neighbours.append(edge.target_id)
        for edge in incoming.get(node_id, []):
            neighbours.append(edge.source_id)

        for neighbour in neighbours:
            if neighbour in depths:
                continue
            depths[neighbour] = depth + 1
            queue.append(neighbour)
            if len(depths) >= max_nodes:
                break

    visited = set(depths)

    node_items = []
    for node_id, depth in sorted(depths.items(), key=lambda item: (item[1], item[0])):
        node = all_nodes[node_id]
        node_items.append(
            {
                "id": node.node_id,
                "uri": node.uri,
                "module": node.module,
                "objectType": node.object_type,
                "identifier": node.identifier,
                "kind": node.kind,
                "header": node.header,
                "depth": depth,
                "sourceOrigin": _source_origin_for_uri(node.uri),
                "range": {
                    "start": {
                        "line": node.range.start.line,
                        "character": node.range.start.character,
                    },
                    "end": {
                        "line": node.range.end.line,
                        "character": node.range.end.character,
                    },
                },
            }
        )

    edge_items = []
    for edge in edges:
        if edge.source_id not in visited or edge.target_id not in visited:
            continue
        edge_items.append(
            {
                "source": edge.source_id,
                "target": edge.target_id,
                "viaProperty": edge.via_property,
                "viaKind": edge.via_kind,
            }
        )

    # Primary root is the first seed root (backward compat).
    primary = roots[0]
    return {
        "root": primary.node_id,
        "rootUri": primary.uri,
        "rootHeader": primary.header,
        "roots": [
            {
                "id": r.node_id,
                "uri": r.uri,
                "header": r.header,
            }
            for r in roots
        ],
        "maxDepth": max_depth,
        "maxNodes": max_nodes,
        "nodes": node_items,
        "edges": edge_items,
    }
