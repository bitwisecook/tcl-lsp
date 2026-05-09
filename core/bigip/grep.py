"""Find every BIG-IP object related to a given object name or regex.

Walks the forward-and-reverse reference graph produced by
:mod:`core.bigip.link_extract` from a set of *seed* objects whose
identifiers match the user's pattern, and returns every object
reachable from the seeds through reference edges in either direction.

The same forward-edge engine that powers
:func:`core.bigip.link_extract.extract_linked_bigip_objects` and
:func:`core.bigip.cleanup.compute_cleanup` is reused here, so iRule
body references (``pool ...``, ``persist ...``, ``class match ...``)
count exactly as they do for the cleanup analysis.
"""

from __future__ import annotations

import re
from collections import defaultdict, deque
from dataclasses import dataclass, field

from ..analysis.semantic_model import Range
from ..commands.registry.runtime import active_signature_profile, configure_signatures
from .link_extract import (
    BigipObjectEdge,
    BigipObjectNode,
    build_bigip_object_graph,
)
from .model import BigipConfig

DIRECTIONS: frozenset[str] = frozenset({"forward", "reverse", "both"})


@dataclass(frozen=True, slots=True)
class GrepObject:
    """One object in the grep result — a seed or a related object."""

    node_id: str
    uri: str
    module: str
    object_type: str
    full_path: str
    kind: str | None
    header: str
    body: str
    depth: int
    is_seed: bool
    range: Range


@dataclass(frozen=True, slots=True)
class GrepEdge:
    """One reference edge between two objects in the grep result."""

    source_id: str
    target_id: str
    via_property: str
    via_kind: str


@dataclass(frozen=True, slots=True)
class GrepReport:
    """Result of :func:`compute_grep` — seeds, related objects, and edges."""

    pattern: str
    use_regex: bool
    direction: str
    seeds: tuple[GrepObject, ...] = ()
    related: tuple[GrepObject, ...] = ()
    edges: tuple[GrepEdge, ...] = ()
    summary: dict[str, int] = field(default_factory=dict)
    text_report: str = ""


def _build_matcher(pattern: str, *, use_regex: bool):
    """Return a ``(identifier) -> bool`` matcher; raises ValueError on bad regex."""
    if use_regex:
        try:
            compiled = re.compile(pattern)
        except re.error as exc:
            raise ValueError(f"invalid regex pattern {pattern!r}: {exc}") from exc
        return lambda identifier: compiled.search(identifier) is not None
    return lambda identifier: pattern in identifier


def _to_grep_object(node: BigipObjectNode, *, depth: int, is_seed: bool) -> GrepObject:
    return GrepObject(
        node_id=node.node_id,
        uri=node.uri,
        module=node.module,
        object_type=node.object_type,
        full_path=node.identifier,
        kind=node.kind,
        header=node.header,
        body=node.body,
        depth=depth,
        is_seed=is_seed,
        range=node.range,
    )


def _format_text_report(
    report_meta: dict,
    seeds: tuple[GrepObject, ...],
    related: tuple[GrepObject, ...],
    summary: dict[str, int],
    *,
    include_body: bool,
) -> str:
    lines: list[str] = [
        "# tcl-lsp BIG-IP grep",
        f"# Pattern: {report_meta['pattern']}" + (" (regex)" if report_meta["use_regex"] else ""),
        f"# Direction: {report_meta['direction']}",
        f"# Sources: {', '.join(report_meta['source_uris']) or '(none)'}",
        f"# Seeds: {len(seeds)} matched object(s)",
        f"# Related: {len(related)} object(s)",
    ]
    if summary:
        for kind in sorted(summary):
            lines.append(f"#   {kind}: {summary[kind]}")
    lines.append("")

    if not seeds:
        lines.append(f"# No objects match pattern: {report_meta['pattern']}")
        lines.append("")
        return "\n".join(lines)

    def _emit(obj: GrepObject) -> None:
        marker = "*" if obj.is_seed else " "
        lines.append(f"{marker} [{obj.kind or '?'}] {obj.full_path}  (depth {obj.depth})")
        if include_body:
            header_line = obj.header.rstrip()
            lines.append(f"    {header_line} {{")
            for body_line in obj.body.splitlines():
                lines.append(f"    {body_line}".rstrip())
            lines.append("    }")
            lines.append("")

    lines.append("# Seeds (matched by pattern):")
    for obj in seeds:
        _emit(obj)
    lines.append("")

    if related:
        lines.append("# Related objects (reachable through reference edges):")
        for obj in related:
            _emit(obj)
        lines.append("")

    return "\n".join(lines)


def compute_grep(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    pattern: str,
    use_regex: bool = False,
    direction: str = "both",
    max_depth: int | None = None,
    max_nodes: int = 1000,
    include_body: bool = False,
) -> GrepReport:
    """Find every BIG-IP object related to seeds whose identifiers match *pattern*.

    A seed is any object whose ``identifier`` (full path) matches *pattern* —
    by substring when *use_regex* is ``False``, by :func:`re.search` when
    ``True``.  Starting from the seeds the function walks the reference
    graph in the requested *direction* (``forward``: outgoing edges only,
    ``reverse``: incoming edges only, ``both``: union) until either every
    reachable object has been visited, *max_depth* hops are exhausted (when
    set), or *max_nodes* total objects have been collected.

    The iRule body scan inside :mod:`core.bigip.link_extract` relies on
    the ``f5-irules`` command registry being active; this function
    configures the ``f5-irules`` profile for the duration of the call and
    restores the previous active profile on exit, so callers do not need
    to manage it.
    """
    if direction not in DIRECTIONS:
        raise ValueError(f"direction must be one of {sorted(DIRECTIONS)}, got {direction!r}")
    if max_nodes < 1:
        raise ValueError(f"max_nodes must be >= 1, got {max_nodes}")
    if max_depth is not None and max_depth < 0:
        raise ValueError(f"max_depth must be >= 0 or None, got {max_depth}")

    # Validate the pattern before walking the graph so a bad regex surfaces
    # as a user error rather than a silent zero-match result.
    matches = _build_matcher(pattern, use_regex=use_regex)

    saved = active_signature_profile()
    configure_signatures(dialect="f5-irules")
    try:
        nodes_by_uri, edges = build_bigip_object_graph(sources=sources, configs=configs)
    finally:
        configure_signatures(
            dialect=saved["dialect"],  # type: ignore[arg-type]
            extra_commands=saved["extra_commands"],  # type: ignore[arg-type]
        )

    all_nodes: dict[str, BigipObjectNode] = {}
    for by_id in nodes_by_uri.values():
        all_nodes.update(by_id)

    outgoing: dict[str, list[BigipObjectEdge]] = defaultdict(list)
    incoming: dict[str, list[BigipObjectEdge]] = defaultdict(list)
    for edge in edges:
        outgoing[edge.source_id].append(edge)
        incoming[edge.target_id].append(edge)

    def _sort_key(nid: str) -> tuple[str, str]:
        node = all_nodes[nid]
        return (node.kind or "", node.identifier)

    seed_ids: list[str] = sorted(
        (nid for nid, node in all_nodes.items() if matches(node.identifier)),
        key=_sort_key,
    )

    # Honour max_nodes as a strict cap on total collected objects: when more
    # seeds match the pattern than the cap allows, truncate before any BFS
    # expansion so the cap is never silently exceeded.
    if len(seed_ids) > max_nodes:
        seed_ids = seed_ids[:max_nodes]

    depths: dict[str, int] = {sid: 0 for sid in seed_ids}
    queue: deque[str] = deque(seed_ids)
    while queue and len(depths) < max_nodes:
        nid = queue.popleft()
        depth = depths[nid]
        if max_depth is not None and depth >= max_depth:
            continue

        neighbours: list[str] = []
        if direction in {"forward", "both"}:
            for edge in outgoing.get(nid, ()):
                neighbours.append(edge.target_id)
        if direction in {"reverse", "both"}:
            for edge in incoming.get(nid, ()):
                neighbours.append(edge.source_id)

        for neighbour in neighbours:
            if neighbour in depths:
                continue
            depths[neighbour] = depth + 1
            queue.append(neighbour)
            if len(depths) >= max_nodes:
                break

    seed_set = set(seed_ids)
    visited = set(depths)

    seeds_list = [
        _to_grep_object(all_nodes[nid], depth=0, is_seed=True)
        for nid in sorted(seed_ids, key=_sort_key)
    ]

    related_ids = sorted(
        (nid for nid in visited if nid not in seed_set),
        key=lambda nid: (depths[nid], _sort_key(nid)),
    )
    related_list = [
        _to_grep_object(all_nodes[nid], depth=depths[nid], is_seed=False) for nid in related_ids
    ]

    summary: dict[str, int] = defaultdict(int)
    for obj in (*seeds_list, *related_list):
        summary[obj.kind or "<unknown>"] += 1

    edge_items: list[GrepEdge] = []
    seen_edges: set[tuple[str, str, str, str]] = set()
    for edge in edges:
        if edge.source_id not in visited or edge.target_id not in visited:
            continue
        key = (edge.source_id, edge.target_id, edge.via_property, edge.via_kind)
        if key in seen_edges:
            continue
        seen_edges.add(key)
        edge_items.append(
            GrepEdge(
                source_id=edge.source_id,
                target_id=edge.target_id,
                via_property=edge.via_property,
                via_kind=edge.via_kind,
            )
        )

    seeds_tuple = tuple(seeds_list)
    related_tuple = tuple(related_list)
    summary_dict = dict(summary)
    text_report = _format_text_report(
        {
            "pattern": pattern,
            "use_regex": use_regex,
            "direction": direction,
            "source_uris": tuple(sorted(sources)),
        },
        seeds_tuple,
        related_tuple,
        summary_dict,
        include_body=include_body,
    )

    return GrepReport(
        pattern=pattern,
        use_regex=use_regex,
        direction=direction,
        seeds=seeds_tuple,
        related=related_tuple,
        edges=tuple(edge_items),
        summary=summary_dict,
        text_report=text_report,
    )


def report_to_dict(report: GrepReport, *, include_body: bool = False) -> dict:
    """Render *report* as a JSON-serialisable dict (LSP / CLI / AI consumers).

    Pass ``include_body=True`` to embed each object's full body — this mirrors
    the CLI's ``--full`` flag for callers that want JSON instead of the text
    report.  Bodies are otherwise omitted to keep the JSON payload compact.
    """

    def _obj_to_dict(obj: GrepObject) -> dict:
        d: dict = {
            "nodeId": obj.node_id,
            "uri": obj.uri,
            "module": obj.module,
            "objectType": obj.object_type,
            "fullPath": obj.full_path,
            "kind": obj.kind,
            "header": obj.header,
            "depth": obj.depth,
            "isSeed": obj.is_seed,
            "range": {
                "start": {
                    "line": obj.range.start.line,
                    "character": obj.range.start.character,
                },
                "end": {
                    "line": obj.range.end.line,
                    "character": obj.range.end.character,
                },
            },
        }
        if include_body:
            d["body"] = obj.body
        return d

    return {
        "pattern": report.pattern,
        "useRegex": report.use_regex,
        "direction": report.direction,
        "seeds": [_obj_to_dict(o) for o in report.seeds],
        "related": [_obj_to_dict(o) for o in report.related],
        "edges": [
            {
                "source": e.source_id,
                "target": e.target_id,
                "viaProperty": e.via_property,
                "viaKind": e.via_kind,
            }
            for e in report.edges
        ],
        "summary": dict(report.summary),
        "textReport": report.text_report,
    }


__all__ = [
    "DIRECTIONS",
    "GrepEdge",
    "GrepObject",
    "GrepReport",
    "compute_grep",
    "report_to_dict",
]
