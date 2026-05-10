"""Build a packaged "iRule + referenced-object" context bundle.

When an AI agent reasons about an iRule it almost always needs the
surrounding BIG-IP context: the pools the rule selects, the data
groups it matches against, the persistence profiles it pins on, the
SNAT pools it forwards through, etc.  Without that context the agent
either has to be force-fed the entire bigip.conf or guess.

This module assembles a focused bundle: the iRule body plus every
BIG-IP object the rule references (resolved against the surrounding
:class:`~core.bigip.model.BigipConfig`), with optional one-hop
transitive expansion (pool → members → nodes; pool → monitor;
persistence → profile chain).

The bundle is consumable as either:

- structured JSON (:func:`context_bundle_to_dict`) for MCP tools and
  programmatic consumers, or
- a Tcl-flavoured text block (:func:`context_bundle_to_text`) ready
  to drop into an LLM prompt.

Both renderings preserve the original config text when ``sources`` is
provided, so what the LLM sees matches the operator's actual
``bigip.conf``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from core.bigip.irules_refs import extract_irules_object_references
from core.bigip.model import (
    BigipConfig,
    BigipDataGroup,
    BigipMonitor,
    BigipNode,
    BigipPersistence,
    BigipPool,
    BigipProfile,
    BigipRule,
    BigipSnatPool,
)


@dataclass(frozen=True, slots=True)
class IruleContextBundle:
    """A resolved iRule plus every BIG-IP object it references.

    *unresolved* is keyed by reference kind (``"pool"``,
    ``"data-group"``, …) and contains the names that did not resolve
    against the surrounding config — surface these to the consumer.
    *source_slices* maps each referenced object's full path to the
    original config text that defined it, when the source map was
    supplied to :func:`build_irule_context`.
    """

    rule: BigipRule
    pools: dict[str, BigipPool] = field(default_factory=dict)
    data_groups: dict[str, BigipDataGroup] = field(default_factory=dict)
    persistence: dict[str, BigipPersistence] = field(default_factory=dict)
    snat_pools: dict[str, BigipSnatPool] = field(default_factory=dict)
    profiles: dict[str, BigipProfile] = field(default_factory=dict)
    monitors: dict[str, BigipMonitor] = field(default_factory=dict)
    nodes: dict[str, BigipNode] = field(default_factory=dict)
    rules: dict[str, BigipRule] = field(default_factory=dict)
    unresolved: dict[str, list[str]] = field(default_factory=dict)
    source_slices: dict[str, str] = field(default_factory=dict)


def _classify_kind(ref_kinds: tuple[str, ...]) -> str | None:
    if any(k == "ltm_pool" for k in ref_kinds):
        return "pool"
    if any(k.startswith("ltm_data_group") or k == "sys_file_data_group" for k in ref_kinds):
        return "data-group"
    if any(k.startswith("ltm_persistence") for k in ref_kinds):
        return "persistence"
    if any(k == "ltm_snatpool" for k in ref_kinds):
        return "snat-pool"
    if any(k.startswith("ltm_monitor") for k in ref_kinds):
        return "monitor"
    if any(k.startswith("ltm_profile") for k in ref_kinds):
        return "profile"
    if any(k == "ltm_node" for k in ref_kinds):
        return "node"
    if any(k == "ltm_rule" for k in ref_kinds):
        return "rule"
    return None


def _slice_for(
    obj: Any,
    *,
    source: str | None,
) -> str | None:
    """Slice the original config text for an object using its ``range``."""
    if source is None:
        return None
    rng = getattr(obj, "range", None)
    if rng is None:
        return None
    lines = source.splitlines(keepends=True)
    start_line = rng.start.line
    end_line = rng.end.line
    if start_line < 0 or end_line >= len(lines):
        return None
    chunk = "".join(lines[start_line : end_line + 1])
    return chunk.rstrip() + "\n"


def _origin_for_object(
    obj: Any,
    sources: dict[str, str] | None,
    config_origin: str | None,
) -> str | None:
    """Return the source text that contains *obj*, if available.

    Today every object carries a single :class:`Range` — we don't
    record which source URI it came from, so we fall back to
    *config_origin* (the URI of the merged/single config the bundle
    was built from).
    """
    if sources is None:
        return None
    if config_origin and config_origin in sources:
        return sources[config_origin]
    if len(sources) == 1:
        return next(iter(sources.values()))
    return None


def build_irule_context(
    rule: BigipRule,
    cfg: BigipConfig,
    *,
    transitive: bool = True,
    sources: dict[str, str] | None = None,
    config_origin: str | None = None,
    dialect: str = "f5-irules",
) -> IruleContextBundle:
    """Walk *rule* and return every BIG-IP object it references.

    *transitive* expands one level deeper: pool members are resolved
    to nodes, pool monitors to monitor objects.

    When *sources* is supplied, the bundle records the original
    config-text slice for every referenced object (under
    :attr:`IruleContextBundle.source_slices`) so renderers can echo
    the operator's exact text rather than synthesising a stanza.
    """
    from core.commands.registry.runtime import configure_signatures

    configure_signatures(dialect=dialect)

    pools: dict[str, BigipPool] = {}
    data_groups: dict[str, BigipDataGroup] = {}
    persistence: dict[str, BigipPersistence] = {}
    snat_pools: dict[str, BigipSnatPool] = {}
    profiles: dict[str, BigipProfile] = {}
    monitors: dict[str, BigipMonitor] = {}
    nodes: dict[str, BigipNode] = {}
    rules: dict[str, BigipRule] = {}
    unresolved: dict[str, list[str]] = {}
    source_slices: dict[str, str] = {}

    seen: set[tuple[str, str]] = set()
    src_text = _origin_for_object(rule, sources, config_origin)
    if src_text is not None:
        slice_text = _slice_for(rule, source=src_text)
        if slice_text:
            source_slices[rule.full_path] = slice_text

    def _record_slice(obj: Any) -> None:
        if src_text is None:
            return
        path = getattr(obj, "full_path", None)
        if not path:
            return
        chunk = _slice_for(obj, source=src_text)
        if chunk:
            source_slices[path] = chunk

    for ref in extract_irules_object_references(rule.source):
        kind = _classify_kind(ref.kinds)
        if kind is None:
            continue
        if (kind, ref.name) in seen:
            continue
        seen.add((kind, ref.name))

        if kind == "pool":
            resolved = cfg.resolve_pool(ref.name)
            if resolved and resolved in cfg.pools:
                obj = cfg.pools[resolved]
                pools[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "data-group":
            resolved = cfg.resolve_data_group(ref.name)
            if resolved and resolved in cfg.data_groups:
                obj = cfg.data_groups[resolved]
                data_groups[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "persistence":
            resolved = cfg.resolve_persistence(ref.name)
            if resolved and resolved in cfg.persistence:
                obj = cfg.persistence[resolved]
                persistence[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "snat-pool":
            resolved = cfg.resolve_snat_pool(ref.name)
            if resolved and resolved in cfg.snat_pools:
                obj = cfg.snat_pools[resolved]
                snat_pools[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "profile":
            resolved = cfg.resolve_profile(ref.name)
            if resolved and resolved in cfg.profiles:
                obj = cfg.profiles[resolved]
                profiles[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "monitor":
            resolved = cfg.resolve_name(ref.name, cfg.monitors)
            if resolved and resolved in cfg.monitors:
                obj = cfg.monitors[resolved]
                monitors[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "node":
            resolved = cfg.resolve_name(ref.name, cfg.nodes)
            if resolved and resolved in cfg.nodes:
                obj = cfg.nodes[resolved]
                nodes[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)
        elif kind == "rule":
            resolved = cfg.resolve_rule(ref.name)
            if resolved and resolved in cfg.rules and resolved != rule.full_path:
                obj = cfg.rules[resolved]
                rules[resolved] = obj
                _record_slice(obj)
            else:
                unresolved.setdefault(kind, []).append(ref.name)

    if transitive:
        # Pool members → nodes; pool monitor → monitor object.  Pool
        # member names take the form "/Common/<node>:<port>" — strip
        # the trailing ":port" before resolving against ``cfg.nodes``.
        for pool in list(pools.values()):
            for member in pool.members:
                node_name = member.name.rsplit(":", 1)[0] if ":" in member.name else member.name
                resolved_node = cfg.resolve_name(node_name, cfg.nodes)
                if resolved_node and resolved_node in cfg.nodes:
                    node = cfg.nodes[resolved_node]
                    nodes.setdefault(resolved_node, node)
                    _record_slice(node)
            if pool.monitor:
                first = pool.monitor.split(" ", 1)[0]
                resolved_mon = cfg.resolve_name(first, cfg.monitors)
                if resolved_mon and resolved_mon in cfg.monitors:
                    mon = cfg.monitors[resolved_mon]
                    monitors.setdefault(resolved_mon, mon)
                    _record_slice(mon)

    return IruleContextBundle(
        rule=rule,
        pools=pools,
        data_groups=data_groups,
        persistence=persistence,
        snat_pools=snat_pools,
        profiles=profiles,
        monitors=monitors,
        nodes=nodes,
        rules=rules,
        unresolved=unresolved,
        source_slices=source_slices,
    )


# ---------------------------------------------------------------------------
# Rendering helpers
# ---------------------------------------------------------------------------


def _summarise_pool(pool: BigipPool) -> dict[str, Any]:
    return {
        "fullPath": pool.full_path,
        "monitor": pool.monitor or None,
        "loadBalancingMode": pool.load_balancing_mode or None,
        "members": [
            {
                "name": m.name,
                "address": m.address or None,
                "port": m.port or None,
                "monitor": m.monitor or None,
            }
            for m in pool.members
        ],
    }


def _summarise_data_group(dg: BigipDataGroup) -> dict[str, Any]:
    return {
        "fullPath": dg.full_path,
        "kind": dg.kind.name.lower() if hasattr(dg.kind, "name") else str(dg.kind),
        "valueType": dg.value_type or None,
        "recordCount": len(dg.records),
        "records": list(dg.records),
    }


def _summarise_persistence(p: BigipPersistence) -> dict[str, Any]:
    return {"fullPath": p.full_path, "type": p.persistence_type or None}


def _summarise_snat_pool(s: BigipSnatPool) -> dict[str, Any]:
    return {"fullPath": s.full_path, "members": list(s.members)}


def _summarise_profile(p: BigipProfile) -> dict[str, Any]:
    profile_type = p.profile_type.name.lower() if hasattr(p.profile_type, "name") else str(p.profile_type)
    return {"fullPath": p.full_path, "type": profile_type}


def _summarise_monitor(m: BigipMonitor) -> dict[str, Any]:
    return {"fullPath": m.full_path, "type": m.monitor_type or None}


def _summarise_node(n: BigipNode) -> dict[str, Any]:
    return {"fullPath": n.full_path, "address": n.address or None}


def context_bundle_to_dict(bundle: IruleContextBundle) -> dict[str, Any]:
    """Serialise *bundle* to a JSON-friendly dict."""
    return {
        "rule": {
            "fullPath": bundle.rule.full_path,
            "name": bundle.rule.name,
            "source": bundle.rule.source,
        },
        "pools": [_summarise_pool(p) for p in bundle.pools.values()],
        "dataGroups": [_summarise_data_group(d) for d in bundle.data_groups.values()],
        "persistence": [_summarise_persistence(p) for p in bundle.persistence.values()],
        "snatPools": [_summarise_snat_pool(s) for s in bundle.snat_pools.values()],
        "profiles": [_summarise_profile(p) for p in bundle.profiles.values()],
        "monitors": [_summarise_monitor(m) for m in bundle.monitors.values()],
        "nodes": [_summarise_node(n) for n in bundle.nodes.values()],
        "rules": [
            {"fullPath": r.full_path, "name": r.name, "source": r.source}
            for r in bundle.rules.values()
        ],
        "unresolved": {kind: sorted(set(names)) for kind, names in bundle.unresolved.items()},
        "sourceSlices": dict(bundle.source_slices),
    }


def _render_pool_text(pool: BigipPool) -> str:
    lines = [f"ltm pool {pool.full_path} {{"]
    if pool.load_balancing_mode:
        lines.append(f"    load-balancing-mode {pool.load_balancing_mode}")
    if pool.members:
        lines.append("    members {")
        for m in pool.members:
            lines.append(f"        {m.name} {{")
            if m.address:
                lines.append(f"            address {m.address}")
            if m.monitor:
                lines.append(f"            monitor {m.monitor}")
            lines.append("        }")
        lines.append("    }")
    if pool.monitor:
        lines.append(f"    monitor {pool.monitor}")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _render_data_group_text(dg: BigipDataGroup) -> str:
    kind = dg.kind.name.lower() if hasattr(dg.kind, "name") else str(dg.kind)
    lines = [f"ltm data-group {kind} {dg.full_path} {{"]
    if dg.value_type:
        lines.append(f"    type {dg.value_type}")
    if dg.records:
        lines.append("    records {")
        for record in dg.records:
            lines.append(f"        {record} {{ }}")
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _render_persistence_text(p: BigipPersistence) -> str:
    return f"ltm persistence {p.persistence_type or '<unknown>'} {p.full_path} {{ }}\n"


def _render_snat_pool_text(s: BigipSnatPool) -> str:
    lines = [f"ltm snatpool {s.full_path} {{"]
    if s.members:
        lines.append("    members {")
        for member in s.members:
            lines.append(f"        {member}")
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _render_profile_text(p: BigipProfile) -> str:
    profile_type = p.profile_type.name.lower() if hasattr(p.profile_type, "name") else str(p.profile_type)
    return f"ltm profile {profile_type} {p.full_path} {{ }}\n"


def _render_monitor_text(m: BigipMonitor) -> str:
    return f"ltm monitor {m.monitor_type or '<unknown>'} {m.full_path} {{ }}\n"


def _render_node_text(n: BigipNode) -> str:
    addr = n.address or ""
    return f"ltm node {n.full_path} {{ address {addr} }}\n"


def _render_object_text(bundle: IruleContextBundle, obj: Any, fallback: str) -> str:
    """Prefer a real source slice; fall back to the synthetic stanza."""
    full_path = getattr(obj, "full_path", None)
    if full_path and full_path in bundle.source_slices:
        return bundle.source_slices[full_path]
    return fallback


def context_bundle_to_text(bundle: IruleContextBundle) -> str:
    """Render *bundle* as a single Tcl-flavoured text block.

    Layout::

        # ===== iRule /Common/foo =====
        ltm rule /Common/foo { … }

        # ===== referenced pool /Common/web_pool =====
        ltm pool /Common/web_pool { … }

        # ===== unresolved =====
        # data-group: my_dg
    """
    parts: list[str] = []

    parts.append(f"# ===== iRule {bundle.rule.full_path} =====")
    rule_text = bundle.source_slices.get(bundle.rule.full_path)
    if rule_text:
        parts.append(rule_text.rstrip())
    else:
        parts.append(f"ltm rule {bundle.rule.full_path} {{")
        parts.append(bundle.rule.source.rstrip())
        parts.append("}")

    sections: list[tuple[str, list[tuple[str, str]]]] = [
        (
            "pool",
            [(p.full_path, _render_object_text(bundle, p, _render_pool_text(p))) for p in bundle.pools.values()],
        ),
        (
            "data-group",
            [
                (d.full_path, _render_object_text(bundle, d, _render_data_group_text(d)))
                for d in bundle.data_groups.values()
            ],
        ),
        (
            "persistence",
            [
                (p.full_path, _render_object_text(bundle, p, _render_persistence_text(p)))
                for p in bundle.persistence.values()
            ],
        ),
        (
            "snat-pool",
            [
                (s.full_path, _render_object_text(bundle, s, _render_snat_pool_text(s)))
                for s in bundle.snat_pools.values()
            ],
        ),
        (
            "profile",
            [
                (p.full_path, _render_object_text(bundle, p, _render_profile_text(p)))
                for p in bundle.profiles.values()
            ],
        ),
        (
            "monitor",
            [
                (m.full_path, _render_object_text(bundle, m, _render_monitor_text(m)))
                for m in bundle.monitors.values()
            ],
        ),
        (
            "node",
            [
                (n.full_path, _render_object_text(bundle, n, _render_node_text(n)))
                for n in bundle.nodes.values()
            ],
        ),
        (
            "rule",
            [
                (
                    r.full_path,
                    _render_object_text(
                        bundle,
                        r,
                        f"ltm rule {r.full_path} {{\n{r.source.rstrip()}\n}}\n",
                    ),
                )
                for r in bundle.rules.values()
            ],
        ),
    ]
    for kind, entries in sections:
        for full_path, text in entries:
            parts.append(f"\n# ===== referenced {kind} {full_path} =====")
            parts.append(text.rstrip())

    if bundle.unresolved:
        parts.append("\n# ===== unresolved =====")
        for kind in sorted(bundle.unresolved):
            for name in sorted(set(bundle.unresolved[kind])):
                parts.append(f"# {kind}: {name}")

    return "\n".join(parts) + "\n"
