"""``f5 explain`` core: trace what a virtual or pool actually does.

Walks the config from a seed object (virtual or pool) and assembles a
human-readable plan: profiles in attach order, iRule chain, persistence,
SNAT, monitor cascade, pool members.  Reuses :class:`BigipConfig`'s
resolve_* helpers and the reference graph from
:mod:`core.bigip.link_extract`.
"""

from __future__ import annotations

from dataclasses import dataclass

from .model import BigipConfig, BigipPool, BigipVirtualServer

_KIND_VIRTUAL = "virtual"
_KIND_POOL = "pool"


@dataclass(frozen=True, slots=True)
class ExplainReport:
    target: str
    kind: str  # "virtual" | "pool"
    found: bool
    sections: tuple[tuple[str, tuple[str, ...]], ...] = ()
    text_report: str = ""


def _resolve_target(
    cfg: BigipConfig, target: str, kind_hint: str | None
) -> tuple[str, BigipVirtualServer | BigipPool] | None:
    if kind_hint in (None, _KIND_VIRTUAL):
        path = cfg.resolve_name(target, cfg.virtual_servers)
        if path is not None:
            return _KIND_VIRTUAL, cfg.virtual_servers[path]
    if kind_hint in (None, _KIND_POOL):
        path = cfg.resolve_pool(target)
        if path is not None:
            return _KIND_POOL, cfg.pools[path]
    return None


def _explain_virtual(cfg: BigipConfig, vs: BigipVirtualServer) -> list[tuple[str, list[str]]]:
    sections: list[tuple[str, list[str]]] = []
    # ``vs.destination`` is a typed :class:`Destination` | None — render
    # to its canonical text spelling for the explainer output.
    dest_text = str(vs.destination) if vs.destination is not None else "(none)"
    sections.append(("destination", [dest_text]))

    profiles = []
    for pref in vs.profiles.paths:
        resolved = cfg.resolve_profile(pref) or pref
        profile = cfg.profiles.get(resolved)
        if profile is not None:
            profiles.append(f"{resolved} ({profile.profile_type.name.lower()})")
        else:
            profiles.append(f"{pref} (unresolved)")
    sections.append(("profiles", profiles or ["(none)"]))

    rules = []
    for rref in vs.rules:
        resolved = cfg.resolve_rule(rref) or rref
        rule = cfg.rules.get(resolved)
        if rule is not None:
            from .stats import _count_irule_lines, _extract_irule_events  # local helper reuse

            events = sorted(set(_extract_irule_events(rule.source)))
            loc = _count_irule_lines(rule.source)
            event_str = ", ".join(events) if events else "(no when blocks)"
            rules.append(f"{resolved} — {loc} loc; events: {event_str}")
        else:
            rules.append(f"{rref} (unresolved)")
    sections.append(("iRules", rules or ["(none)"]))

    persist = []
    for pref in vs.persist.paths:
        resolved = cfg.resolve_persistence(pref) or pref
        prof = cfg.persistence.get(resolved)
        if prof is not None:
            persist.append(f"{resolved} ({prof.persistence_type})")
        else:
            persist.append(f"{pref} (unresolved)")
    sections.append(("persistence", persist or ["(none)"]))

    snat: list[str] = []
    if vs.snatpool:
        resolved = cfg.resolve_snat_pool(vs.snatpool) or vs.snatpool
        sp = cfg.snat_pools.get(resolved)
        if sp is not None:
            snat.append(f"snatpool {resolved} ({len(sp.members)} member(s))")
        else:
            snat.append(f"snatpool {vs.snatpool} (unresolved)")
    if vs.source_address_translation:
        snat.append(f"translation: {vs.source_address_translation}")
    sections.append(("SNAT", snat or ["(none)"]))

    pool_lines: list[str] = []
    if vs.pool:
        resolved_pool = cfg.resolve_pool(vs.pool)
        if resolved_pool is not None:
            pool_lines = _explain_pool_lines(cfg, cfg.pools[resolved_pool])
        else:
            pool_lines = [f"{vs.pool} (unresolved)"]
    sections.append(("default pool", pool_lines or ["(none)"]))
    return sections


def _explain_pool(cfg: BigipConfig, pool: BigipPool) -> list[tuple[str, list[str]]]:
    return [("pool", _explain_pool_lines(cfg, pool))]


def _explain_pool_lines(cfg: BigipConfig, pool: BigipPool) -> list[str]:
    lines = [f"{pool.full_path} ({pool.load_balancing_mode or 'round-robin'})"]
    if pool.monitor:
        lines.append(f"  pool monitor: {pool.monitor}")
    for member in pool.members:
        line = f"  member {member.name}"
        if member.address:
            line += f" addr={member.address}"
        if member.port:
            line += f" port={member.port}"
        if member.monitor:
            line += f" monitor={member.monitor}"
        lines.append(line)
    return lines


def compute_explain(cfg: BigipConfig, target: str, *, kind: str | None = None) -> ExplainReport:
    resolved = _resolve_target(cfg, target, kind)
    if resolved is None:
        return ExplainReport(
            target=target,
            kind=kind or "?",
            found=False,
            text_report=f"no virtual or pool found for {target!r}",
        )

    actual_kind, obj = resolved
    if actual_kind == _KIND_VIRTUAL:
        sections = _explain_virtual(cfg, obj)  # type: ignore[arg-type]
    else:
        sections = _explain_pool(cfg, obj)  # type: ignore[arg-type]

    sections_t = tuple((heading, tuple(lines)) for heading, lines in sections)
    text = _format_text(target, actual_kind, obj.full_path, sections)

    return ExplainReport(
        target=target,
        kind=actual_kind,
        found=True,
        sections=sections_t,
        text_report=text,
    )


def _format_text(
    target: str, kind: str, full_path: str, sections: list[tuple[str, list[str]]]
) -> str:
    lines = [f"explain {kind}: {full_path}", ""]
    for heading, items in sections:
        lines.append(f"  {heading}:")
        for item in items:
            lines.append(f"    {item}")
        lines.append("")
    return "\n".join(lines).rstrip()


def report_to_dict(report: ExplainReport) -> dict:
    return {
        "target": report.target,
        "kind": report.kind,
        "found": report.found,
        "sections": [
            {"heading": heading, "lines": list(lines)} for heading, lines in report.sections
        ],
    }
