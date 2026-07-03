"""Build a structured report model from BIG-IP configs using the query engine.

Every fact in the report is pulled from the native F5 BIG-IP *query engine*
(:mod:`f5report._engine`) — the same ``f5-query`` DSL that ships with tcl-lsp.
The engine does the parsing, object projection and (crucially) the reference-
graph walk (``referenced_by``) that powers the orphan / dependency analysis.
This module only shapes the engine's output into a template-friendly model.

Public API:

* :func:`collect_model` — run the queries and return the report model (a plain
  ``dict``; handy on its own for JSON export or testing).
* :func:`build_report` — :func:`collect_model` + HTML rendering.
"""

from __future__ import annotations

import os
import re
from typing import Any

from . import _engine
from . import graph as _graph
from .render import render_report

Sources = list[tuple[str, str]]

# --- object containers the report walks, in display order --------------------
# Each is an `f5-query` container path under a config root. `referenced_by`
# turns every leaf object into a node in a dependency graph, which is what lets
# us flag orphans (objects nothing points at) natively.
_CONTAINERS = {
    "virtuals": ".ltm.virtual",
    "pools": ".ltm.pool",
    "nodes": ".ltm.node",
    "monitors": ".ltm.monitor",
    "rules": ".ltm.rule",
    "dataGroups": '.ltm."data-group"',
    "profiles": ".ltm.profile",
    "snatpools": ".ltm.snatpool",
    "persistence": ".ltm.persistence",
    "policies": ".ltm.policy",
    "virtualAddresses": '.ltm."virtual-address"',
}

# Leaf object types that are *referenced* by something else; an empty referrer
# set means the object is orphaned. Virtuals and virtual-addresses are entry
# points, so they are never treated as orphans.
_REFERABLE = ["pools", "nodes", "monitors", "rules", "profiles", "dataGroups", "snatpools"]

_TMSH_RE = re.compile(r"#TMSH-VERSION:\s*(\S+)")
_HOSTNAME_RE = re.compile(r"hostname\s+(\S+)")


# --- small helpers -----------------------------------------------------------
def _fields(rows: list[Any]) -> list[dict[str, Any]]:
    """Unwrap engine ObjectRef dicts (`{kind, full-path, fields}`) to `fields`."""
    out = []
    for r in rows:
        if isinstance(r, dict) and "fields" in r:
            out.append(r["fields"])
    return out


def _clean_path(value: str) -> str:
    """Strip trailing ` { ... }` context the projection appends to some refs."""
    if not isinstance(value, str):
        return value
    return value.split(" {", 1)[0].strip()


def _split_dest(dest: str) -> tuple[str, str]:
    """Split ``/Common/192.168.1.21:443`` into ``(192.168.1.21, 443)``."""
    if not dest:
        return "", ""
    leaf = dest.rsplit("/", 1)[-1]
    if ":" in leaf:  # IPv4/name:port or IPv6.port
        addr, _, port = leaf.rpartition(":")
        return addr, port
    return leaf, ""


def _refmap(sources: Sources, container: str) -> dict[str, list[str]]:
    """Map every object's full-path to the full-paths that reference it.

    This is the engine's ``referenced_by`` graph builtin, surfaced verbatim —
    the single most useful thing the query DSL gives a report generator.
    """
    rows = _engine.query(
        f'{container}[] | {{p: ."full-path", by: referenced_by(.)}}', sources
    )
    out: dict[str, list[str]] = {}
    for r in rows:
        out[r["p"]] = list(r.get("by") or [])
    return out


# --- per-type shaping --------------------------------------------------------
def _shape_virtual(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    addr, port = _split_dest(f.get("destination", ""))
    fp = f.get("full-path", "")
    v = {
        "name": f.get("name", ""),
        "fullPath": fp,
        "partition": fp.split("/")[1] if fp.startswith("/") else "",
        "destination": f.get("destination", ""),
        "destAddr": addr,
        "destPort": port,
        "mask": f.get("mask", ""),
        "pool": _clean_path(f.get("pool", "")),
        "profiles": [_clean_path(p) for p in f.get("profiles", []) or []],
        "rules": [_clean_path(r) for r in f.get("rules", []) or []],
        "persist": [_clean_path(p) for p in f.get("persist", []) or []],
        "policies": [_clean_path(p) for p in f.get("policies", []) or []],
        "snatpool": _clean_path(f.get("snatpool", "")),
        "sourceXlate": f.get("source-address-translation", ""),
        "ipProtocol": f.get("ip-protocol", ""),
        "source": f.get("source", ""),
        "vlans": [_clean_path(x) for x in f.get("vlans", []) or []],
        "vlansEnabled": bool(f.get("vlans-enabled")),
        "vlansDisabled": bool(f.get("vlans-disabled")),
        "description": f.get("description", ""),
        "disabled": bool(f.get("disabled")) or f.get("state") == "disabled",
    }
    v["listener"] = _graph.parse_listener(v)
    return v


def _shape_pool(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    members = []
    for m in f.get("members", []) or []:
        mf = m.get("fields", {}) if isinstance(m, dict) else {}
        name = mf.get("name", "")
        port = mf.get("port") or (name.rsplit(":", 1)[-1] if ":" in name else "")
        members.append(
            {
                "name": name,
                "address": mf.get("address", ""),
                "port": str(port),
                "monitor": _clean_path(mf.get("monitor", "")),
                "ratio": mf.get("ratio", ""),
                "priorityGroup": mf.get("priority-group", ""),
                "connectionLimit": mf.get("connection-limit", ""),
                "state": mf.get("state", ""),
                "description": mf.get("description", ""),
            }
        )
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "monitor": _clean_path(f.get("monitor", "")),
        "lbMode": f.get("load-balancing-mode", ""),
        "members": members,
        "memberCount": len(members),
        "usedBy": used_by.get(fp, []),
    }


def _shape_node(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "address": f.get("address", ""),
        "monitor": _clean_path(f.get("monitor", "")),
        "usedBy": used_by.get(fp, []),
    }


def _shape_monitor(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "type": f.get("type", ""),
        "interval": f.get("interval", ""),
        "timeout": f.get("timeout", ""),
        "send": f.get("send", ""),
        "recv": f.get("recv", ""),
        "usedBy": used_by.get(fp, []),
    }


def _shape_rule(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    body = f.get("body", "") or ""
    events = sorted(set(re.findall(r"\bwhen\s+([A-Z][A-Z0-9_]+)", body)))
    fp = f.get("full-path", "")
    # `.refs` is the engine's synthesised iRule reference sub-object: pools /
    # persistences / data-groups the rule body actually uses.
    refs = (f.get("refs") or {}).get("fields", {}) if isinstance(f.get("refs"), dict) else {}
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "lineCount": body.count("\n") + 1 if body else 0,
        "events": events,
        "body": body,
        "usedBy": used_by.get(fp, []),
        "refPools": [_clean_path(p) for p in refs.get("pools", []) or []],
        "refDataGroups": [_clean_path(p) for p in refs.get("data-groups", []) or []],
        "dynamicActions": _graph.irule_dynamic_actions(body),
    }


def _shape_data_group(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    records = f.get("records", []) or []
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "type": f.get("type", ""),
        "recordCount": len(records),
        "records": [str(r) for r in records[:200]],
        "usedBy": used_by.get(fp, []),
    }


def _shape_profile(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    ptype = str(f.get("type", "")).replace("ProfileType.", "")
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "type": ptype,
        "parent": _clean_path(f.get("defaults-from", "")),
        "ciphers": f.get("ciphers", "") or f.get("cipher-group", ""),
        "cert": _clean_path(f.get("cert", "")),
        "key": _clean_path(f.get("key", "")),
        "chain": _clean_path(f.get("chain", "")),
        "usedBy": used_by.get(fp, []),
    }


def _shape_policy(f: dict[str, Any], used_by: dict[str, list[str]]) -> dict[str, Any]:
    """Shape an LTM policy into rules → conditions / actions the simulator runs."""
    def _sub(x: Any) -> dict[str, Any]:
        return x.get("fields", x) if isinstance(x, dict) else {}

    rules = []
    for r in f.get("rules", []) or []:
        rf = _sub(r)
        conds = []
        for c in rf.get("conditions", []) or []:
            cf = _sub(c)
            conds.append({
                "operand": cf.get("operand", ""),
                "selector": cf.get("selector", ""),
                "operator": cf.get("operator", ""),
                "values": cf.get("values", []) or [],
                "negate": bool(cf.get("negate")),
                "caseInsensitive": bool(cf.get("case-insensitive")),
            })
        acts = []
        for a in rf.get("actions", []) or []:
            af = _sub(a)
            acts.append({
                "target": af.get("target", ""),
                "verb": af.get("verb", ""),
                "pool": _clean_path(af.get("pool", "")),
                "location": af.get("location", ""),
                "host": af.get("host", ""),
                "path": af.get("path", ""),
                "value": af.get("value", ""),
                "name": af.get("name", ""),
            })
        rules.append({
            "name": rf.get("name", ""),
            "ordinal": rf.get("ordinal", 0),
            "conditions": conds,
            "actions": acts,
        })
    fp = f.get("full-path", "")
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "strategy": f.get("strategy", ""),
        "rules": rules,
    }


_SHAPERS = {
    "virtuals": _shape_virtual,
    "pools": _shape_pool,
    "nodes": _shape_node,
    "monitors": _shape_monitor,
    "rules": _shape_rule,
    "dataGroups": _shape_data_group,
    "profiles": _shape_profile,
    "policies": _shape_policy,
}


# --- model assembly ----------------------------------------------------------
def _device_name(uri: str, source: str) -> str:
    m = _HOSTNAME_RE.search(source)
    if m:
        return m.group(1)
    base = os.path.basename(uri)
    return base.rsplit(".", 1)[0] if "." in base else base


def _insights(device: dict[str, Any]) -> list[dict[str, str]]:
    """Actionable findings — the kind of thing an operator scans a report for."""
    out: list[dict[str, str]] = []
    orph = device["orphans"]
    for kind in ("pools", "nodes", "rules", "monitors", "profiles"):
        n = len(orph.get(kind, []))
        if n:
            out.append(
                {"level": "warn", "text": f"{n} orphaned {kind} (defined, referenced by nothing)"}
            )
    empty_pools = [p["name"] for p in device["pools"] if p["memberCount"] == 0]
    if empty_pools:
        preview = ", ".join(empty_pools[:6]) + ("…" if len(empty_pools) > 6 else "")
        out.append({"level": "warn", "text": f"{len(empty_pools)} pool(s) with no members: {preview}"})
    no_pool_vs = [v["name"] for v in device["virtuals"] if not v["pool"] and not v["policies"]]
    if no_pool_vs:
        out.append(
            {"level": "info", "text": f"{len(no_pool_vs)} virtual server(s) with no default pool "
             "(forwarding / policy-driven)"}
        )
    disabled_vs = [v["name"] for v in device["virtuals"] if v["disabled"]]
    if disabled_vs:
        out.append({"level": "info", "text": f"{len(disabled_vs)} disabled virtual server(s)"})
    ssl = [p for p in device["profiles"] if "SSL" in p["type"]]
    if ssl:
        out.append({"level": "info", "text": f"{len(ssl)} SSL profile(s) in use"})
    if not out:
        out.append({"level": "ok", "text": "No orphaned objects or empty pools detected"})
    return out


def _collect_device(uri: str, source: str) -> dict[str, Any]:
    sources: Sources = [(uri, source)]

    # One reference-graph walk per referable container, up front.
    refmaps = {name: _refmap(sources, _CONTAINERS[name]) for name in _REFERABLE}

    tmsh = _TMSH_RE.search(source)
    device: dict[str, Any] = {
        "uri": uri,
        "name": _device_name(uri, source),
        "tmshVersion": tmsh.group(1) if tmsh else "",
    }

    for key, container in _CONTAINERS.items():
        rows = _fields(_engine.query(f"{container}[]", sources))
        shaper = _SHAPERS.get(key)
        used_by = refmaps.get(key, {})
        if shaper:
            device[key] = [shaper(f, used_by) for f in rows]
        else:
            # snatpools / persistence / policies / virtual-addresses: keep the
            # projected fields, tidy up the name/full-path for display.
            device[key] = [
                {"name": f.get("name", ""), "fullPath": f.get("full-path", ""), "fields": f}
                for f in rows
            ]

    # Orphans: referable leaf objects with an empty referrer set.
    device["orphans"] = {
        name: [
            o["name"]
            for o in device.get(name, [])
            if isinstance(o, dict) and not o.get("usedBy")
        ]
        for name in _REFERABLE
        if name in device
    }

    # Propagate each iRule's dynamic (runtime) actions onto the virtuals that
    # attach it, so a VS shows profiles/pools it changes at runtime, not just
    # its statically-attached ones.
    rule_actions = {r["fullPath"]: r["dynamicActions"] for r in device["rules"]}
    rule_by_name = {r["fullPath"].split("/")[-1]: r["fullPath"] for r in device["rules"]}
    for v in device["virtuals"]:
        acts: list[dict[str, str]] = []
        for rule_ref in v["rules"]:
            fp = rule_ref if rule_ref in rule_actions else rule_by_name.get(rule_ref.split("/")[-1], "")
            for a in rule_actions.get(fp, []):
                acts.append({**a, "rule": rule_ref.split("/")[-1]})
        v["dynamicProfiles"] = acts

    device["graph"] = _graph.build_graph(device)
    device["counts"] = {key: len(device.get(key, [])) for key in _CONTAINERS}
    device["counts"]["poolMembers"] = sum(p["memberCount"] for p in device["pools"])
    device["counts"]["orphans"] = sum(len(v) for v in device["orphans"].values())
    device["insights"] = _insights(device)
    return device


def collect_model(
    sources: Sources,
    *,
    title: str = "F5 BIG-IP Configuration Report",
) -> dict[str, Any]:
    """Build the full report model from loaded ``(uri, text)`` sources.

    ``sources`` is what :func:`f5report.load_paths` returns. The result is a
    plain ``dict`` — render it with :func:`f5report.render.render_report`, dump
    it to JSON, or assert against it in tests.
    """
    devices = [_collect_device(uri, src) for uri, src in sources]

    totals: dict[str, int] = {}
    for d in devices:
        for k, v in d["counts"].items():
            totals[k] = totals.get(k, 0) + v

    return {
        "title": title,
        "engine_version": _engine.__version__,
        "devices": devices,
        "totals": totals,
        "container_order": list(_CONTAINERS),
    }


def build_report(
    sources: Sources,
    *,
    title: str = "F5 BIG-IP Configuration Report",
) -> str:
    """Collect the model and render it to a standalone HTML document."""
    return render_report(collect_model(sources, title=title))
