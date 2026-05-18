"""Generate ``tmsh delete`` commands for BIG-IP objects unreferenced by any virtual.

Walks the forward reference graph produced by
:mod:`core.bigip.link_extract`, treats every ``ltm virtual`` (and
``gtm wideip *``) as a graph root, and flags every ``ltm`` / ``gtm``
object **not** transitively reachable from a root as a cleanup
candidate.  Candidates are emitted in reverse-topological order so
that any object holding a reference to another candidate is deleted
first — matching ``tmsh`` deletion semantics (``tmsh`` rejects
deletes whose target is still referenced).

The same forward-edge engine that powers
:func:`core.bigip.link_extract.extract_linked_bigip_objects` is reused
here, including the iRule-body scan via
:func:`core.bigip.irules_refs.extract_irules_object_references`, so
references inside iRule Tcl source (``pool ...``, ``persist ...``,
``class match ... <data-group>``) are tracked the same way as
configuration-property references.
"""

from __future__ import annotations

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
from .registry.data import OBJECT_KIND_SPECS

# Kinds that are roots of the keep-graph (never deleted, always traversed from).
_ROOT_KINDS: frozenset[str] = frozenset({"ltm_virtual"})

# Kinds that are out of scope for cleanup regardless of reachability.
# ``ltm_virtual`` is a root.  ``ltm_virtual_address`` is an address record,
# which BIG-IP creates implicitly from virtual-server destinations; deleting
# it through this script would race the implicit lifecycle.
_NEVER_DELETE_KINDS: frozenset[str] = frozenset(
    {
        "ltm_virtual",
        "ltm_virtual_address",
    }
)


@dataclass(frozen=True, slots=True)
class CleanupCandidate:
    """One BIG-IP object proposed for deletion."""

    node_id: str
    uri: str
    module: str
    object_type: str
    full_path: str
    kind: str | None
    delete_command: str
    reason: str
    range: Range


@dataclass(frozen=True, slots=True)
class CleanupReport:
    """Result of :func:`compute_cleanup` — ordered candidates plus a tmsh script."""

    candidates: tuple[CleanupCandidate, ...] = ()
    kept: tuple[str, ...] = ()
    roots: tuple[str, ...] = ()
    summary: dict[str, int] = field(default_factory=dict)
    tmsh_script: str = ""


def _is_root_kind(kind: str | None) -> bool:
    if kind is None:
        return False
    if kind in _ROOT_KINDS:
        return True
    # GTM wide-IPs are top-level entry points just like virtuals.
    return kind.startswith("gtm_wideip")


def _is_in_module_scope(node: BigipObjectNode) -> bool:
    return node.module in {"ltm", "gtm"}


def _deletable_kinds() -> frozenset[str]:
    """Every ``ltm`` / ``gtm`` kind eligible for cleanup."""
    eligible: set[str] = set()
    for kind, spec in OBJECT_KIND_SPECS.items():
        if spec.module not in {"ltm", "gtm"}:
            continue
        if kind in _NEVER_DELETE_KINDS:
            continue
        if _is_root_kind(kind):
            continue
        eligible.add(kind)
    return frozenset(eligible)


def _matches_keep_filter(
    node: BigipObjectNode,
    *,
    keep_paths: frozenset[str],
    keep_partitions: frozenset[str],
) -> bool:
    if node.identifier in keep_paths:
        return True
    for prefix in keep_partitions:
        if node.identifier.startswith(prefix):
            return True
    return False


def _build_delete_command(node: BigipObjectNode) -> str:
    return f"delete {node.module} {node.object_type} {node.identifier}"


def _topological_order(
    candidates: dict[str, BigipObjectNode],
    edges: list[BigipObjectEdge],
) -> list[str]:
    """Topologically sort candidates so referrers come before referents.

    Returns a list of ``node_id`` such that whenever ``A -> B`` is an
    edge between two candidates, ``A`` precedes ``B``.  Cycles are
    broken with a deterministic ``(kind, identifier)`` tie-break so
    output is stable across runs.
    """
    in_degree: dict[str, int] = {nid: 0 for nid in candidates}
    out_adj: dict[str, list[str]] = defaultdict(list)
    seen: set[tuple[str, str]] = set()
    for edge in edges:
        if edge.source_id not in candidates or edge.target_id not in candidates:
            continue
        if edge.source_id == edge.target_id:
            continue  # self-edge has no effect on order
        key = (edge.source_id, edge.target_id)
        if key in seen:
            continue
        seen.add(key)
        out_adj[edge.source_id].append(edge.target_id)
        in_degree[edge.target_id] += 1

    def sort_key(nid: str) -> tuple[str, str]:
        node = candidates[nid]
        return (node.kind or "", node.identifier)

    ready: list[str] = sorted(
        (nid for nid, deg in in_degree.items() if deg == 0),
        key=sort_key,
    )
    queue: deque[str] = deque(ready)
    order: list[str] = []
    while queue:
        nid = queue.popleft()
        order.append(nid)
        # Drain neighbours in deterministic order.
        for neighbour in sorted(out_adj.get(nid, ()), key=sort_key):
            in_degree[neighbour] -= 1
            if in_degree[neighbour] == 0:
                queue.append(neighbour)

    if len(order) < len(candidates):
        # Cycle remnant — emit the leftovers in deterministic order so the
        # script is at least reviewable.  These candidates may need manual
        # detachment before tmsh accepts the deletes.
        order_set = set(order)
        leftover = sorted(
            (nid for nid in candidates if nid not in order_set),
            key=sort_key,
        )
        order.extend(leftover)
    return order


def _format_tmsh_script(
    candidates: tuple[CleanupCandidate, ...],
    *,
    summary: dict[str, int],
    root_count: int,
    source_uris: tuple[str, ...],
) -> str:
    if not candidates:
        return (
            "# tcl-lsp BIG-IP cleanup\n"
            f"# Sources: {', '.join(source_uris) or '(none)'}\n"
            f"# Roots (kept): {root_count} virtual server(s) / wide-IP(s)\n"
            "# Candidates: none — every object is referenced.\n"
        )

    lines: list[str] = [
        "# tcl-lsp BIG-IP cleanup",
        f"# Sources: {', '.join(source_uris) or '(none)'}",
        f"# Roots (kept): {root_count} virtual server(s) / wide-IP(s)",
        f"# Candidates: {len(candidates)} unreferenced object(s)",
    ]
    for kind in sorted(summary):
        lines.append(f"#   {kind}: {summary[kind]}")
    lines.append("#")
    lines.append("# Review each delete before running.  Order is reverse-")
    lines.append("# topological: referencing objects are deleted before the")
    lines.append("# objects they reference, so tmsh accepts each command in turn.")
    lines.append("")
    for candidate in candidates:
        lines.append(candidate.delete_command)
    lines.append("")
    return "\n".join(lines)


def compute_cleanup(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    keep_paths: frozenset[str] = frozenset(),
    keep_partitions: frozenset[str] = frozenset({"/Common/"}),
) -> CleanupReport:
    """Build a cleanup report for the BIG-IP configuration in *configs*.

    Every ``ltm virtual`` and ``gtm wideip *`` is treated as a graph root
    and the forward reference graph from :mod:`core.bigip.link_extract`
    is walked from those roots; anything unreached is a candidate.  The
    candidate set is filtered against *keep_paths* and *keep_partitions*
    (default: keep ``/Common/``-prefixed paths to avoid deleting the
    factory-shipped objects), then emitted in reverse-topological order
    so each ``tmsh delete`` runs after every still-extant object that
    references its target has already been deleted in the same script.

    The iRule body scan inside :mod:`core.bigip.link_extract` relies on
    the ``f5-irules`` command registry being active for body / expr role
    resolution (the ``when`` event handler must be recognised as
    ``ArgRole.BODY``).  This function configures the ``f5-irules``
    profile for the duration of the call and restores the previous
    active profile on exit, so callers do not need to manage it.
    """
    # Save/restore the active signature profile so the analysis sees iRule
    # event-handler bodies as ArgRole.BODY without leaking that choice to
    # the surrounding session (e.g. an LSP that has the user on tcl8.6).
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
    for edge in edges:
        outgoing[edge.source_id].append(edge)

    root_ids: list[str] = [nid for nid, node in all_nodes.items() if _is_root_kind(node.kind)]

    # BFS reachability from every root over outgoing edges.
    kept: set[str] = set(root_ids)
    queue: deque[str] = deque(root_ids)
    while queue:
        nid = queue.popleft()
        for edge in outgoing.get(nid, ()):
            if edge.target_id in kept:
                continue
            kept.add(edge.target_id)
            queue.append(edge.target_id)

    deletable_kinds = _deletable_kinds()

    candidate_nodes: dict[str, BigipObjectNode] = {}
    for nid, node in all_nodes.items():
        if nid in kept:
            continue
        if not _is_in_module_scope(node):
            continue
        if node.kind is None or node.kind not in deletable_kinds:
            continue
        if _matches_keep_filter(node, keep_paths=keep_paths, keep_partitions=keep_partitions):
            continue
        candidate_nodes[nid] = node

    order = _topological_order(candidate_nodes, edges)

    candidates_list: list[CleanupCandidate] = []
    summary: dict[str, int] = defaultdict(int)
    for nid in order:
        node = candidate_nodes[nid]
        candidate = CleanupCandidate(
            node_id=node.node_id,
            uri=node.uri,
            module=node.module,
            object_type=node.object_type,
            full_path=node.identifier,
            kind=node.kind,
            delete_command=_build_delete_command(node),
            reason="not reachable from any virtual server / wide-IP",
            range=node.range,
        )
        candidates_list.append(candidate)
        summary[node.kind or "<unknown>"] += 1

    candidates_tuple = tuple(candidates_list)
    source_uris = tuple(sorted(sources))
    tmsh_script = _format_tmsh_script(
        candidates_tuple,
        summary=dict(summary),
        root_count=len(root_ids),
        source_uris=source_uris,
    )

    return CleanupReport(
        candidates=candidates_tuple,
        kept=tuple(sorted(kept)),
        roots=tuple(sorted(root_ids)),
        summary=dict(summary),
        tmsh_script=tmsh_script,
    )


def report_to_dict(report: CleanupReport) -> dict:
    """Render *report* as a JSON-serialisable dict (LSP / CLI / AI consumers)."""
    return {
        "candidates": [
            {
                "nodeId": c.node_id,
                "uri": c.uri,
                "module": c.module,
                "objectType": c.object_type,
                "fullPath": c.full_path,
                "kind": c.kind,
                "deleteCommand": c.delete_command,
                "reason": c.reason,
                "range": {
                    "start": {
                        "line": c.range.start.line,
                        "character": c.range.start.character,
                    },
                    "end": {
                        "line": c.range.end.line,
                        "character": c.range.end.character,
                    },
                },
            }
            for c in report.candidates
        ],
        "kept": list(report.kept),
        "roots": list(report.roots),
        "summary": dict(report.summary),
        "tmshScript": report.tmsh_script,
    }


__all__ = [
    "CleanupCandidate",
    "CleanupReport",
    "compute_cleanup",
    "report_to_dict",
]
