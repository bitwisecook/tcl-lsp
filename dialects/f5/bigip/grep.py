"""Find every BIG-IP object related to a given object name or regex.

Walks the forward-and-reverse reference graph produced by
:mod:`dialects.f5.bigip.link_extract` from a set of *seed* objects whose
identifiers match the user's pattern, and returns every object
reachable from the seeds through reference edges in either direction.

The same forward-edge engine that powers
:func:`dialects.f5.bigip.link_extract.extract_linked_bigip_objects` and
:func:`dialects.f5.bigip.cleanup.compute_cleanup` is reused here, so iRule
body references (``pool ...``, ``persist ...``, ``class match ...``)
count exactly as they do for the cleanup analysis.
"""

from __future__ import annotations

import ipaddress
import re
from collections import defaultdict, deque
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field

from compiler.registry.runtime import active_signature_profile, configure_signatures
from core.analysis.semantic_model import Range
from shared.ip_utils import IPV4_RE, IPV6_RE

from .link_extract import (
    BigipObjectEdge,
    BigipObjectNode,
    build_bigip_object_graph,
)
from .model import BigipConfig

DIRECTIONS: frozenset[str] = frozenset({"forward", "reverse", "both"})

_IPNetwork = ipaddress.IPv4Network | ipaddress.IPv6Network


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
    use_cidr: bool = False
    recurse: bool = True


def _parse_cidr_pattern(pattern: str) -> tuple[_IPNetwork, ...]:
    """Parse one or more whitespace/comma-separated IPs or CIDRs.

    Each token is fed through :func:`ipaddress.ip_network` with
    ``strict=False`` so host-address forms (``10.0.0.1`` →
    ``10.0.0.1/32``) and network forms (``10.0.0.0/8``) are both
    accepted.
    """
    tokens = [tok for tok in re.split(r"[,\s]+", pattern.strip()) if tok]
    if not tokens:
        raise ValueError("CIDR pattern must contain at least one IP address or network")
    networks: list[_IPNetwork] = []
    for token in tokens:
        try:
            networks.append(ipaddress.ip_network(token, strict=False))
        except (ValueError, ipaddress.AddressValueError) as exc:
            raise ValueError(f"invalid CIDR pattern {token!r}: {exc}") from exc
    return tuple(networks)


def _extract_ip_networks(text: str) -> Iterator[_IPNetwork]:
    """Yield every IP/CIDR token found in *text* as an :mod:`ipaddress` network.

    Scans for IPv4 and IPv6 literals (with an optional ``/prefix``) using
    the shared :mod:`shared.ip_utils` regexes and validates each
    candidate via :func:`ipaddress.ip_network` (``strict=False``).
    Tokens that fail validation are silently skipped — the regexes are
    deliberately permissive so we lean on the stdlib parser as the
    authority.
    """
    for match in IPV4_RE.finditer(text):
        token = match.group(0)
        try:
            yield ipaddress.ip_network(token, strict=False)
        except (ValueError, ipaddress.AddressValueError):
            continue
    for match in IPV6_RE.finditer(text):
        token = match.group(0)
        try:
            yield ipaddress.ip_network(token, strict=False)
        except (ValueError, ipaddress.AddressValueError):
            continue


def _node_matches_cidrs(
    node: BigipObjectNode,
    seed_networks: tuple[_IPNetwork, ...],
) -> bool:
    """Return True when any IP/CIDR mentioned by *node* overlaps a seed network.

    The node's full path, header, and body are searched together so
    that hits inside iRule scripts (``equals "10.0.0.0/8"``,
    ``IP::addr 10.0.0.1`` …) and inside configuration bodies
    (``destination /Common/10.0.0.10:80``) both qualify.
    """
    haystack = f"{node.identifier}\n{node.header}\n{node.body}"
    for found in _extract_ip_networks(haystack):
        for seed in seed_networks:
            if found.version != seed.version:
                continue
            if seed.overlaps(found):
                return True
    return False


def _build_matcher(
    pattern: str,
    *,
    use_regex: bool,
    use_cidr: bool = False,
    use_exact: bool = False,
) -> Callable[[BigipObjectNode], bool]:
    """Return a ``(node) -> bool`` matcher for the given pattern mode.

    Modes are mutually exclusive:

    - **Substring** (default): the pattern is a substring of the
      node's identifier — preserves the historical seed semantics
      for the ``f5 grep`` CLI where partial paths are useful.
    - **Regex** (*use_regex*): ``re.search`` against the identifier.
    - **CIDR** (*use_cidr*): inspects the node's path, header, and
      body so IP / CIDR references buried inside iRule script
      bodies are picked up too.
    - **Exact** (*use_exact*): the pattern must equal the node's
      identifier exactly.  Used by ``references_to()`` / rename
      preflights where ``/Common/p`` must not also match
      ``/Common/p2``.

    Raises :class:`ValueError` on a bad regex or unparseable CIDR.
    """
    selected = [
        name
        for name, flag in (
            ("regex", use_regex),
            ("cidr", use_cidr),
            ("exact", use_exact),
        )
        if flag
    ]
    if len(selected) > 1:
        raise ValueError(f"match modes are mutually exclusive; got {selected}")
    if use_cidr:
        seed_networks = _parse_cidr_pattern(pattern)
        return lambda node: _node_matches_cidrs(node, seed_networks)
    if use_regex:
        try:
            compiled = re.compile(pattern)
        except re.error as exc:
            raise ValueError(f"invalid regex pattern {pattern!r}: {exc}") from exc
        return lambda node: compiled.search(node.identifier) is not None
    if use_exact:
        return lambda node: node.identifier == pattern
    return lambda node: pattern in node.identifier


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
    if report_meta.get("use_cidr"):
        mode_suffix = " (cidr)"
    elif report_meta["use_regex"]:
        mode_suffix = " (regex)"
    else:
        mode_suffix = ""
    related_line = (
        f"# Related: {len(related)} object(s)"
        if report_meta.get("recurse", True)
        else "# Related: 0 object(s) (skipped — --no-recurse)"
    )
    lines: list[str] = [
        "# tcl-lsp BIG-IP grep",
        f"# Pattern: {report_meta['pattern']}" + mode_suffix,
        f"# Direction: {report_meta['direction']}",
        f"# Sources: {', '.join(report_meta['source_uris']) or '(none)'}",
        f"# Searches: {len(seeds)} matched object(s)",
        related_line,
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

    lines.append("# Searches (matched by pattern):")
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
    use_cidr: bool = False,
    use_exact: bool = False,
    direction: str = "both",
    max_depth: int | None = None,
    max_nodes: int = 1000,
    include_body: bool = False,
    recurse: bool = True,
) -> GrepReport:
    """Find every BIG-IP object related to seeds whose identifiers match *pattern*.

    A seed is any object that matches *pattern*.  Three match modes are
    supported and they are mutually exclusive:

    - **Substring** (default): the pattern must appear as a substring of
      the object's ``identifier`` (full path).
    - **Regex** (*use_regex*): the pattern is compiled with :mod:`re` and
      tested against the object's identifier with :func:`re.search`.
    - **CIDR** (*use_cidr*): the pattern is parsed as one or more
      whitespace/comma-separated IP addresses or CIDRs, and an object is
      a seed when any IP literal or CIDR mentioned in its path, header,
      or body overlaps any of the requested networks.  This includes
      addresses buried deep inside iRule script bodies.

    Starting from the seeds the function walks the reference graph in
    the requested *direction* (``forward``: outgoing edges only,
    ``reverse``: incoming edges only, ``both``: union) until either every
    reachable object has been visited, *max_depth* hops are exhausted (when
    set), or *max_nodes* total objects have been collected.

    *recurse* is ``True`` by default — the related-object BFS runs and
    the report includes everything reachable from the seeds.  Pass
    ``recurse=False`` to skip the BFS entirely and return only the
    objects that directly match *pattern* — *direction* and *max_depth*
    are then ignored, and the ``related`` tuple in the report is empty.
    This is the right mode when the question is "which objects mention
    X?" rather than "what is the neighbourhood around X?".

    The iRule body scan inside :mod:`dialects.f5.bigip.link_extract` relies on
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

    # Validate the pattern before walking the graph so a bad regex or
    # unparseable CIDR surfaces as a user error rather than a silent
    # zero-match result.
    matches = _build_matcher(pattern, use_regex=use_regex, use_cidr=use_cidr, use_exact=use_exact)

    saved = active_signature_profile()
    configure_signatures(dialect="f5-irules")
    try:
        nodes_by_uri, edges = build_bigip_object_graph(sources=sources, configs=configs)
    finally:
        configure_signatures(
            dialect=saved["dialect"],
            extra_commands=saved["extra_commands"],
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
        (nid for nid, node in all_nodes.items() if matches(node)),
        key=_sort_key,
    )

    # Honour max_nodes as a strict cap on total collected objects: when more
    # seeds match the pattern than the cap allows, truncate before any BFS
    # expansion so the cap is never silently exceeded.
    if len(seed_ids) > max_nodes:
        seed_ids = seed_ids[:max_nodes]

    depths: dict[str, int] = {sid: 0 for sid in seed_ids}
    if recurse:
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
            "use_cidr": use_cidr,
            "direction": direction,
            "recurse": recurse,
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
        use_cidr=use_cidr,
        direction=direction,
        seeds=seeds_tuple,
        related=related_tuple,
        edges=tuple(edge_items),
        summary=summary_dict,
        text_report=text_report,
        recurse=recurse,
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
            "isSearch": obj.is_seed,
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
        "useCidr": report.use_cidr,
        "recurse": report.recurse,
        "direction": report.direction,
        "searches": [_obj_to_dict(o) for o in report.seeds],
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
