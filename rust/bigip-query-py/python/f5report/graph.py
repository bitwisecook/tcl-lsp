"""Derive the object graph, listener-matching fields and iRule dynamic actions.

This is the analysis layer the interactive diagrams and the listener view run
on. It consumes the already-shaped per-device model from :mod:`f5report.report`
(which itself came from the query engine) and produces:

* a **graph** of typed nodes and edges — including pools referenced *inside*
  iRules (via the engine's ``.refs`` sub-object) and pool-member → node links —
  that the topology explorer renders with Mermaid;
* per-virtual **listener** fields (address / prefix / port / protocol / source /
  VLANs / route-domain) that the listener-matching table ranks by specificity;
* per-iRule **dynamic actions** — commands that change traffic processing at
  runtime (``SSL::disable``, ``HTTP::disable``, ``LB::detach``, ``pool``,
  ``persist`` …) — surfaced on the virtuals that attach the rule.
"""

from __future__ import annotations

import ipaddress
import json
import re
from typing import Any

from . import _engine

# Node type -> short prefix used in stable object ids ("vs:/Common/x").
NODE_TYPES = {
    "virtuals": "vs",
    "pools": "pool",
    "nodes": "node",
    "monitors": "mon",
    "rules": "rule",
    "profiles": "prof",
    "persistence": "persist",
    "policies": "policy",
    "snatpools": "snat",
    "dataGroups": "dg",
}
TYPE_LABEL = {
    "vs": "Virtual", "pool": "Pool", "node": "Node", "mon": "Monitor",
    "rule": "iRule", "prof": "Profile", "persist": "Persistence",
    "policy": "Policy", "snat": "SNAT Pool", "dg": "Data Group",
}


# --- route domain / listener parsing ----------------------------------------
def _split_rd(addr: str) -> tuple[str, int]:
    """Split ``10.0.0.1%2`` into ``('10.0.0.1', 2)``; default route-domain 0."""
    if "%" in addr:
        base, _, rd = addr.partition("%")
        rd = re.split(r"[:./]", rd)[0]
        try:
            return base, int(rd)
        except ValueError:
            return base, 0
    return addr, 0


def _mask_to_prefix(addr: str, mask: str) -> int | None:
    """Turn a netmask (dotted or prefix) into a prefix length for *addr*."""
    if not mask:
        return None
    mask = mask.strip()
    if mask.isdigit():
        return int(mask)
    try:
        if ":" in mask:  # IPv6 mask is unusual; fall back to counting bits
            return sum(bin(int(b, 16)).count("1") for b in mask.split(":") if b)
        return ipaddress.IPv4Network(f"0.0.0.0/{mask}").prefixlen
    except (ipaddress.AddressValueError, ValueError):
        return None


def _is_any(addr: str) -> bool:
    return addr in ("", "any", "0.0.0.0", "::", "0.0.0.0/0", "::/0")


def parse_listener(v: dict[str, Any]) -> dict[str, Any]:
    """Extract BIG-IP listener-matching fields from a shaped virtual."""
    dest_addr, rd = _split_rd(v.get("destAddr", ""))
    is_v6 = ":" in dest_addr
    any_addr = _is_any(dest_addr)
    full_bits = 128 if is_v6 else 32

    # destination prefix: from the mask, else a host route — unless the address
    # is a wildcard, which is a /0 forwarding/catch-all listener.
    mask = v.get("mask", "")
    if any_addr:
        prefix = 0
    else:
        prefix = _mask_to_prefix(dest_addr, mask)
        if prefix is None:
            prefix = full_bits

    port_raw = str(v.get("destPort", "") or "")
    port = 0 if port_raw in ("", "0", "any", "*") else _port_num(port_raw)

    src = v.get("source", "") or ("::/0" if is_v6 else "0.0.0.0/0")
    src_addr, src_rd = _split_rd(src.split("/")[0])
    try:
        src_net = ipaddress.ip_network(_normalise_cidr(src), strict=False)
        src_prefix = src_net.prefixlen
    except ValueError:
        src_prefix = 0

    return {
        "family": "IPv6" if is_v6 else "IPv4",
        "address": dest_addr,
        "anyAddr": any_addr,
        "prefix": prefix,
        "maxPrefix": full_bits,
        "port": port,
        "portRaw": "any" if port == 0 else str(port),
        "protocol": (v.get("ipProtocol") or "any"),
        "routeDomain": rd,
        "source": src,
        "sourcePrefix": src_prefix,
        "vlans": [x.split("/")[-1] for x in v.get("vlans", []) or []],
        "vlansEnabled": bool(v.get("vlansEnabled")),
        "vlansDisabled": bool(v.get("vlansDisabled")),
    }


def _port_num(p: str) -> int:
    if p.isdigit():
        return int(p)
    # named service ports the projection may leave as text
    import socket
    try:
        return socket.getservbyname(p)
    except OSError:
        return 0


def _normalise_cidr(s: str) -> str:
    s = s.split("%")[0]
    if "/" not in s:
        s += "/128" if ":" in s else "/32"
    return s


# --- iRule dynamic actions ---------------------------------------------------
# Commands that change traffic processing / profiles at runtime. Each entry:
# (regex, category, human effect). Matched case-insensitively on the rule body.
_DYNAMIC_CMDS: list[tuple[re.Pattern[str], str, str]] = [
    (re.compile(r"\bSSL::disable(?:\s+(clientside|serverside))?", re.I), "SSL", "SSL::disable"),
    (re.compile(r"\bSSL::enable(?:\s+(clientside|serverside))?", re.I), "SSL", "SSL::enable"),
    (re.compile(r"\bHTTP::disable\b", re.I), "HTTP", "HTTP::disable"),
    (re.compile(r"\bHTTP::enable\b", re.I), "HTTP", "HTTP::enable"),
    (re.compile(r"\bCOMPRESS::disable\b", re.I), "Compression", "COMPRESS::disable"),
    (re.compile(r"\bCOMPRESS::enable\b", re.I), "Compression", "COMPRESS::enable"),
    (re.compile(r"\bCACHE::disable\b", re.I), "Cache", "CACHE::disable"),
    (re.compile(r"\bCACHE::enable\b", re.I), "Cache", "CACHE::enable"),
    (re.compile(r"\bWAM::(?:enable|disable)\b", re.I), "WebAccel", "WAM"),
    (re.compile(r"\bLB::detach\b", re.I), "Pool", "LB::detach"),
    (re.compile(r"\bpersist\s+(none|uie|cookie|source_addr|dest_addr|hash|ssl|sip|carp|universal)", re.I), "Persistence", "persist"),
    (re.compile(r"\bsnatpool\s+(\S+)", re.I), "SNAT", "snatpool"),
    (re.compile(r"\bsnat\s+(automap|none|\S+)", re.I), "SNAT", "snat"),
    (re.compile(r"\bnode\s+(\d[\d.:a-fA-F]+)", re.I), "Node", "node override"),
    (re.compile(r"\bvirtual\s+(\S+)", re.I), "Virtual", "virtual target"),
]


# Attachable object types considered for dynamic (iRule) reachability: `pool`
# (LB target), `node` (direct node), `snatpool` (SNAT). Literal references
# (`pool /Common/x`) are already captured by the engine's `referenced_by`.
ATTACH_TYPES = ("pools", "nodes", "snatpools")


def _pattern_matches(pat: dict[str, Any], name: str) -> bool:
    """Does ``name`` fall within the set of names a reconstructed attach
    expression could build? Mirrors ``tcl_diagram::AttachPattern::matches``."""
    if pat.get("unconstrained"):
        return True
    if pat.get("exact"):
        return name == pat.get("prefix", "")
    hay = name
    prefix = pat.get("prefix", "")
    if prefix:
        if not hay.startswith(prefix):
            return False
        hay = hay[len(prefix):]
    suffix = pat.get("suffix", "")
    if suffix:
        if not hay.endswith(suffix):
            return False
        hay = hay[: len(hay) - len(suffix)]
    for frag in pat.get("contains", []):
        i = hay.find(frag)
        if i < 0:
            return False
        hay = hay[i + len(frag):]
    return True


def attach_index(rules: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    """Per attachable object type, the ``{rule, …pattern…}`` records an iRule
    could dynamically construct.

    Supersedes the all-or-nothing risk heuristic: each attach expression is
    reconstructed (by the real IR walk + ``set`` constant propagation in
    ``_engine.irule_attach_patterns``) into a prefix / contained / suffix name
    pattern, so orphan classification can filter candidate objects by name.
    """
    index: dict[str, list[dict[str, Any]]] = {}
    for r in rules:
        body = r.get("body", "") or ""
        if not body:
            continue
        rule_name = r.get("name") or r.get("fullPath", "")
        reach = json.loads(_engine.irule_attach_patterns(body))
        for ty in ATTACH_TYPES:
            for pat in reach.get(ty, []):
                index.setdefault(ty, []).append({"rule": rule_name, **pat})
    return index


def attach_matches(
    patterns: list[dict[str, Any]], names: list[str]
) -> list[dict[str, str]]:
    """The ``{rule, pattern}`` records whose name-pattern could attach an object
    with any of ``names`` (leaf name, full path, and — for nodes — address)."""
    out: list[dict[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for rp in patterns:
        if not any(_pattern_matches(rp, n) for n in names):
            continue
        key = (rp["rule"], rp["glob"])
        if key in seen:
            continue
        seen.add(key)
        out.append({"rule": rp["rule"], "pattern": rp["glob"]})
    return out


def irule_dynamic_actions(body: str) -> list[dict[str, str]]:
    """Return the profile/processing-changing commands a rule issues."""
    if not body:
        return []
    seen: set[tuple[str, str]] = set()
    out: list[dict[str, str]] = []
    for pat, category, effect in _DYNAMIC_CMDS:
        for m in pat.finditer(body):
            arg = (m.group(1) if m.groups() else "") or ""
            key = (effect, arg)
            if key in seen:
                continue
            seen.add(key)
            out.append({"category": category, "effect": effect, "arg": arg})
    return out


# --- object graph ------------------------------------------------------------
def _oid(kind: str, full_path: str) -> str:
    return f"{kind}:{full_path}"


def build_graph(device: dict[str, Any]) -> dict[str, Any]:
    """Build a typed node/edge graph for one device.

    Edges (directed, source → target):
      virtual → pool | rule | profile | persistence | policy | snatpool
      pool    → node (member) | monitor
      rule    → pool (referenced inside the iRule) | data-group
    """
    nodes: dict[str, dict[str, Any]] = {}
    edges: list[dict[str, str]] = []
    edge_seen: set[tuple[str, str, str]] = set()

    # index nodes by full-path and (for members) by address
    node_by_path: dict[str, dict[str, Any]] = {}
    node_by_addr: dict[str, dict[str, Any]] = {}

    def add_node(kind: str, full_path: str, name: str, extra: dict[str, Any] | None = None) -> str:
        oid = _oid(kind, full_path)
        if oid not in nodes:
            nodes[oid] = {
                "oid": oid, "type": kind, "name": name or full_path.split("/")[-1],
                "fullPath": full_path, "partition": full_path.split("/")[1] if full_path.startswith("/") else "",
                **(extra or {}),
            }
        return oid

    def add_edge(src: str, dst: str, kind: str, label: str = "") -> None:
        if not src or not dst or src not in nodes or dst not in nodes:
            return
        key = (src, dst, kind)
        if key in edge_seen:
            return
        edge_seen.add(key)
        edges.append({"from": src, "to": dst, "kind": kind, "label": label})

    # register every referable object first
    for key, prefix in NODE_TYPES.items():
        for o in device.get(key, []):
            fp = o.get("fullPath", "")
            if not fp:
                continue
            # Confirmed orphan only; a "possible" orphan (dynamic iRule
            # attachment) is not coloured as an orphan.
            if "orphanStatus" in o:
                is_orphan = o["orphanStatus"] == "orphan"
            else:
                is_orphan = bool(not o.get("usedBy")) if "usedBy" in o else False
            n = add_node(prefix, fp, o.get("name", ""), {"orphan": is_orphan})
            node_by_path[fp] = nodes[n]
            if key == "nodes" and o.get("address"):
                node_by_addr[o["address"]] = nodes[n]

    # virtual edges
    for v in device.get("virtuals", []):
        vid = _oid("vs", v["fullPath"])
        if v.get("pool"):
            add_edge(vid, _oid("pool", v["pool"]), "pool", "pool")
        for r in v.get("rules", []):
            add_edge(vid, _oid("rule", r), "rule", "irule")
        for p in v.get("profiles", []):
            add_edge(vid, _oid("prof", p), "profile", "profile")
        for p in v.get("persist", []):
            add_edge(vid, _oid("persist", p), "persist", "persist")
        for p in v.get("policies", []):
            add_edge(vid, _oid("policy", p), "policy", "policy")
        if v.get("snatpool"):
            add_edge(vid, _oid("snat", v["snatpool"]), "snat", "snat")

    # pool edges: members → nodes, pool → monitors
    for p in device.get("pools", []):
        pid = _oid("pool", p["fullPath"])
        for m in p.get("members", []):
            npath = m.get("name", "").rsplit(":", 1)[0]
            target = node_by_path.get(npath) or node_by_addr.get(m.get("address", ""))
            if target:
                add_edge(pid, target["oid"], "member", m.get("port", ""))
        for mon in _split_monitors(p.get("monitor", "")):
            if mon in node_by_path:
                add_edge(pid, _oid("mon", mon), "monitor", "monitor")

    # iRule edges: pools referenced *inside* the rule body (engine .refs)
    for r in device.get("rules", []):
        rid = _oid("rule", r["fullPath"])
        for pool in r.get("refPools", []):
            add_edge(rid, _oid("pool", pool), "pool-irule", "pool (iRule)")
        for dg in r.get("refDataGroups", []):
            add_edge(rid, _oid("dg", dg), "datagroup", "data-group")

    return {"nodes": list(nodes.values()), "edges": edges}


def _split_monitors(monitor: str) -> list[str]:
    """`/Common/http and /Common/tcp` -> ['/Common/http', '/Common/tcp']."""
    if not monitor:
        return []
    parts = re.split(r"\s+and\s+|\s+or\s+|\s*,\s*|\s+min\s+\d+\s+of\s*", monitor)
    return [p.strip() for p in parts if p.strip().startswith("/")]
