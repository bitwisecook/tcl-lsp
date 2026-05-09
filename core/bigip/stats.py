"""Aggregate statistics for one or more BIG-IP configs.

Powers ``f5 stats`` (alias ``summary``).  The numbers come straight off
the parsed :class:`BigipConfig` and the reference graph from
:mod:`core.bigip.link_extract`; nothing here re-parses or re-walks the
source.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field

from .link_extract import BigipObjectEdge, BigipObjectNode, build_bigip_object_graph
from .model import BigipConfig


@dataclass(frozen=True, slots=True)
class StatsReport:
    object_counts: dict[str, int]
    partition_counts: dict[str, int]
    irule_loc: dict[str, int]
    irule_event_histogram: dict[str, int]
    top_referenced: tuple[tuple[str, int], ...]
    orphan_count: int
    total_objects: int
    text_report: str = ""


def _count_irule_lines(body: str) -> int:
    count = 0
    for raw in body.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        count += 1
    return count


def _extract_irule_events(body: str) -> list[str]:
    """Cheap extractor: find ``when EVENT`` tokens in an iRule body."""
    import re

    return re.findall(r"\bwhen\s+([A-Z][A-Z0-9_]*)\b", body)


def _partition_of(full_path: str) -> str:
    if not full_path.startswith("/"):
        return "(no partition)"
    parts = full_path.split("/", 2)
    if len(parts) >= 2:
        return f"/{parts[1]}/"
    return "(no partition)"


def compute_stats(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    top_n: int = 10,
) -> StatsReport:
    object_counts: Counter[str] = Counter()
    partition_counts: Counter[str] = Counter()
    irule_loc: dict[str, int] = {}
    irule_events: Counter[str] = Counter()

    for cfg in configs.values():
        object_counts["pool"] += len(cfg.pools)
        object_counts["virtual"] += len(cfg.virtual_servers)
        object_counts["node"] += len(cfg.nodes)
        object_counts["profile"] += len(cfg.profiles)
        object_counts["monitor"] += len(cfg.monitors)
        object_counts["data-group"] += len(cfg.data_groups)
        object_counts["snatpool"] += len(cfg.snat_pools)
        object_counts["persistence"] += len(cfg.persistence)
        object_counts["rule"] += len(cfg.rules)
        object_counts["generic"] += len(cfg.generic_objects)

        for path in cfg.pools:
            partition_counts[_partition_of(path)] += 1
        for path in cfg.virtual_servers:
            partition_counts[_partition_of(path)] += 1
        for path in cfg.nodes:
            partition_counts[_partition_of(path)] += 1
        for path, rule in cfg.rules.items():
            irule_loc[path] = _count_irule_lines(rule.source)
            for event in _extract_irule_events(rule.source):
                irule_events[event] += 1

    nodes_by_uri, edges = build_bigip_object_graph(sources=sources, configs=configs)

    incoming: Counter[str] = Counter()
    for edge in edges:
        incoming[edge.target_id] += 1

    all_nodes: dict[str, BigipObjectNode] = {
        n.node_id: n for by_uri in nodes_by_uri.values() for n in by_uri.values()
    }

    # Top-N most-referenced objects (by inbound edge count).
    top: list[tuple[str, int]] = []
    for node_id, count in incoming.most_common():
        node = all_nodes.get(node_id)
        if node is None:
            continue
        top.append((node.identifier, count))
        if len(top) >= top_n:
            break

    # Orphan = a non-root, deletable-eligible node with zero incoming refs.
    from .cleanup import _deletable_kinds, _is_root_kind  # type: ignore[attr-defined]

    deletable = _deletable_kinds()
    orphans = 0
    for node_id, node in all_nodes.items():
        if _is_root_kind(node.kind):
            continue
        if node.kind not in deletable:
            continue
        if incoming.get(node_id, 0) == 0:
            orphans += 1

    total = sum(object_counts.values())

    text = _format_text(
        object_counts=object_counts,
        partition_counts=partition_counts,
        irule_loc=irule_loc,
        irule_events=irule_events,
        top=top,
        orphans=orphans,
        total=total,
    )

    return StatsReport(
        object_counts=dict(object_counts),
        partition_counts=dict(partition_counts),
        irule_loc=irule_loc,
        irule_event_histogram=dict(irule_events),
        top_referenced=tuple(top),
        orphan_count=orphans,
        total_objects=total,
        text_report=text,
    )


def _format_text(
    *,
    object_counts: Counter[str],
    partition_counts: Counter[str],
    irule_loc: dict[str, int],
    irule_events: Counter[str],
    top: list[tuple[str, int]],
    orphans: int,
    total: int,
) -> str:
    lines = [f"objects: {total} total"]
    for kind, count in sorted(object_counts.items()):
        lines.append(f"  {kind:<14s} {count}")
    lines.append("")
    lines.append("partitions:")
    for partition, count in sorted(partition_counts.items()):
        lines.append(f"  {partition:<20s} {count}")
    lines.append("")
    lines.append(f"iRules: {len(irule_loc)} ({sum(irule_loc.values())} non-blank lines)")
    if irule_loc:
        for path, loc in sorted(irule_loc.items(), key=lambda kv: -kv[1])[:10]:
            lines.append(f"  {path:<40s} {loc} loc")
    if irule_events:
        lines.append("")
        lines.append("iRule events:")
        for event, count in irule_events.most_common(15):
            lines.append(f"  {event:<32s} {count}")
    lines.append("")
    lines.append("most referenced:")
    for ident, count in top:
        lines.append(f"  {ident:<40s} {count}")
    lines.append("")
    lines.append(f"orphans (no inbound references): {orphans}")
    return "\n".join(lines)


def report_to_dict(report: StatsReport) -> dict:
    return {
        "totalObjects": report.total_objects,
        "objectCounts": report.object_counts,
        "partitionCounts": report.partition_counts,
        "iruleLoc": report.irule_loc,
        "iruleEventHistogram": report.irule_event_histogram,
        "topReferenced": [{"identifier": k, "incomingEdges": v} for k, v in report.top_referenced],
        "orphanCount": report.orphan_count,
    }
