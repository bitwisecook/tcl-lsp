"""Serialise the BIG-IP reference graph to DOT, JSON, or Mermaid.

Powers ``f5 graph`` (alias ``deps``).  Reuses the graph already
computed by :func:`core.bigip.link_extract.build_bigip_object_graph`.
"""

from __future__ import annotations

from collections import defaultdict, deque
from dataclasses import dataclass

from .link_extract import (
    BigipObjectEdge,
    BigipObjectNode,
    build_bigip_object_graph,
)
from .model import BigipConfig

FORMATS = ("dot", "json", "mermaid")


@dataclass(frozen=True, slots=True)
class GraphExport:
    fmt: str
    text: str
    node_count: int
    edge_count: int


def _safe_id(node_id: str) -> str:
    return "n_" + "".join(c if c.isalnum() else "_" for c in node_id)


def _label(node: BigipObjectNode) -> str:
    return f"{node.kind or node.object_type}\\n{node.identifier}"


def _filter_to_subgraph(
    seed_paths: tuple[str, ...],
    nodes: dict[str, BigipObjectNode],
    edges: list[BigipObjectEdge],
    *,
    reverse: bool,
    max_depth: int | None,
) -> tuple[dict[str, BigipObjectNode], list[BigipObjectEdge]]:
    """BFS the graph from seeds and return only reached nodes/edges.

    *reverse* swaps direction: incoming references rather than outgoing.
    """
    if not seed_paths:
        return nodes, edges

    matched_seeds: set[str] = set()
    for path in seed_paths:
        for node_id, node in nodes.items():
            if node.identifier == path:
                matched_seeds.add(node_id)

    if not matched_seeds:
        return {}, []

    adjacency: dict[str, list[str]] = defaultdict(list)
    for edge in edges:
        if reverse:
            adjacency[edge.target_id].append(edge.source_id)
        else:
            adjacency[edge.source_id].append(edge.target_id)

    visited: dict[str, int] = {seed: 0 for seed in matched_seeds}
    queue: deque[str] = deque(matched_seeds)
    while queue:
        current = queue.popleft()
        depth = visited[current]
        if max_depth is not None and depth >= max_depth:
            continue
        for neighbour in adjacency.get(current, []):
            if neighbour in visited:
                continue
            visited[neighbour] = depth + 1
            queue.append(neighbour)

    keep_nodes = {nid: nodes[nid] for nid in visited if nid in nodes}
    keep_edges = [
        e for e in edges
        if e.source_id in keep_nodes and e.target_id in keep_nodes
    ]
    return keep_nodes, keep_edges


def export_graph(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    fmt: str = "dot",
    seeds: tuple[str, ...] = (),
    reverse: bool = False,
    max_depth: int | None = None,
) -> GraphExport:
    if fmt not in FORMATS:
        raise ValueError(f"unknown graph format {fmt!r} (expected one of {FORMATS})")

    nodes_by_uri, edges = build_bigip_object_graph(sources=sources, configs=configs)
    nodes: dict[str, BigipObjectNode] = {
        n.node_id: n for by_uri in nodes_by_uri.values() for n in by_uri.values()
    }
    nodes, edges = _filter_to_subgraph(seeds, nodes, edges, reverse=reverse, max_depth=max_depth)

    if fmt == "dot":
        text = _to_dot(nodes, edges)
    elif fmt == "json":
        text = _to_json(nodes, edges)
    else:
        text = _to_mermaid(nodes, edges)

    return GraphExport(fmt=fmt, text=text, node_count=len(nodes), edge_count=len(edges))


def _to_dot(
    nodes: dict[str, BigipObjectNode], edges: list[BigipObjectEdge]
) -> str:
    lines = ["digraph bigip {", "  rankdir=LR;", '  node [shape=box, fontname="monospace"];']
    for nid, node in nodes.items():
        lines.append(f'  {_safe_id(nid)} [label="{_label(node)}"];')
    for edge in edges:
        if edge.source_id not in nodes or edge.target_id not in nodes:
            continue
        lines.append(
            f'  {_safe_id(edge.source_id)} -> {_safe_id(edge.target_id)} '
            f'[label="{edge.via_kind}"];'
        )
    lines.append("}")
    return "\n".join(lines) + "\n"


def _to_json(
    nodes: dict[str, BigipObjectNode], edges: list[BigipObjectEdge]
) -> str:
    import json

    payload = {
        "nodes": [
            {
                "id": nid,
                "module": node.module,
                "objectType": node.object_type,
                "identifier": node.identifier,
                "kind": node.kind,
                "uri": node.uri,
            }
            for nid, node in nodes.items()
        ],
        "edges": [
            {
                "source": e.source_id,
                "target": e.target_id,
                "viaProperty": e.via_property,
                "viaKind": e.via_kind,
            }
            for e in edges
            if e.source_id in nodes and e.target_id in nodes
        ],
    }
    return json.dumps(payload, indent=2) + "\n"


def _to_mermaid(
    nodes: dict[str, BigipObjectNode], edges: list[BigipObjectEdge]
) -> str:
    lines = ["graph LR"]
    for nid, node in nodes.items():
        sid = _safe_id(nid)
        label = node.identifier.replace('"', "'")
        lines.append(f'  {sid}["{label}"]')
    for edge in edges:
        if edge.source_id not in nodes or edge.target_id not in nodes:
            continue
        lines.append(
            f"  {_safe_id(edge.source_id)} -->|{edge.via_kind}| "
            f"{_safe_id(edge.target_id)}"
        )
    return "\n".join(lines) + "\n"
