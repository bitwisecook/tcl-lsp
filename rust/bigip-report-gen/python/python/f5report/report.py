# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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

import json
import os
import re
from typing import Any

from . import _engine
from . import certs as _certs
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
_REFERABLE = [
    "pools",
    "nodes",
    "monitors",
    "rules",
    "profiles",
    "dataGroups",
    "snatpools",
]

# Object-list keys that carry a displayed, partition-scoped object.
_DISPLAY_KEYS = (
    "virtuals",
    "pools",
    "nodes",
    "monitors",
    "rules",
    "dataGroups",
    "profiles",
    "policies",
    "snatpools",
    "persistence",
    "certificates",
)

# Config members that hold TMOS's built-in defaults (default profiles / monitors
# / `_sys_*` objects). Objects declared here are shown only where directly linked.
_DEFAULT_MEMBERS = frozenset(
    {"config/profile_base.conf", "config/low_profile_base.conf"}
)


def _default_object_paths(config_text: str) -> set[str]:
    """Full paths of built-in / system objects, from the ``# config/<member>``
    sections the UCS extractor writes. Mirrors the Rust ``default_object_paths``."""
    out: set[str] = set()
    in_default = False
    for line in config_text.splitlines():
        if line.startswith("# "):
            name = line[2:].strip()
            if name.startswith("config/"):
                in_default = name in _DEFAULT_MEMBERS
            continue
        if not in_default:
            continue
        if line[:1].islower() and "{" in line:
            toks = line.split("{", 1)[0].split()
            if toks:
                nm = toks[-1]
                out.add(nm if nm.startswith("/") else "/Common/" + nm)
    return out


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
    """Split a BIG-IP destination into ``(address, port)``.

    IPv4 / names use a ``:`` separator (``/Common/192.168.1.21:443``); IPv6 uses
    a ``.`` separator so the colons in the address aren't ambiguous
    (``/Common/2001:db8::10.443``).
    """
    if not dest:
        return "", ""
    leaf = dest.rsplit("/", 1)[-1]
    if leaf.count(":") >= 2:  # IPv6: address holds the colons, port after a dot
        if "." in leaf:
            addr, _, port = leaf.rpartition(".")
            return addr, port
        return leaf, ""
    if ":" in leaf:  # IPv4 / name : port
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
    # Canonical firing order (via the shared registry), not alphabetical — so
    # CLIENT_ACCEPTED precedes CLIENTSSL_HANDSHAKE, etc. Matches the Rust report.
    events = _engine.order_events(
        list(set(re.findall(r"\bwhen\s+([A-Z][A-Z0-9_]+)", body)))
    )
    fp = f.get("full-path", "")
    # `.refs` is the engine's synthesised iRule reference sub-object: pools /
    # persistences / data-groups the rule body actually uses.
    refs = (
        (f.get("refs") or {}).get("fields", {})
        if isinstance(f.get("refs"), dict)
        else {}
    )
    return {
        "name": f.get("name", ""),
        "fullPath": fp,
        "lineCount": body.count("\n") + 1 if body else 0,
        "events": events,
        "body": body,
        "bodyHtml": _engine.highlight_tcl(body),
        "flowchart": _engine.irule_flowchart(body),
        # The reconstructed dynamic-attach name patterns and resolved objects are
        # attached later as ``referencedObjects`` (once the device's object lists
        # and per-virtual partition contexts are known — see
        # ``graph.annotate_rule_reachability``).
        "usedBy": used_by.get(fp, []),
        "refPools": [_clean_path(p) for p in refs.get("pools", []) or []],
        "refDataGroups": [_clean_path(p) for p in refs.get("data-groups", []) or []],
        "dynamicActions": _graph.irule_dynamic_actions(body),
    }


def _shape_data_group(
    f: dict[str, Any], used_by: dict[str, list[str]]
) -> dict[str, Any]:
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
            conds.append(
                {
                    "operand": cf.get("operand", ""),
                    "selector": cf.get("selector", ""),
                    "operator": cf.get("operator", ""),
                    "values": cf.get("values", []) or [],
                    "negate": bool(cf.get("negate")),
                    "caseInsensitive": bool(cf.get("case-insensitive")),
                }
            )
        acts = []
        for a in rf.get("actions", []) or []:
            af = _sub(a)
            acts.append(
                {
                    "target": af.get("target", ""),
                    "verb": af.get("verb", ""),
                    "pool": _clean_path(af.get("pool", "")),
                    "location": af.get("location", ""),
                    "host": af.get("host", ""),
                    "path": af.get("path", ""),
                    "value": af.get("value", ""),
                    "name": af.get("name", ""),
                }
            )
        rules.append(
            {
                "name": rf.get("name", ""),
                "ordinal": rf.get("ordinal", 0),
                "conditions": conds,
                "actions": acts,
            }
        )
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
    lifecycle = device.get("releaseLifecycle") or {}
    if lifecycle.get("level") in {"warn", "danger"}:
        out.append({"level": lifecycle["level"], "text": lifecycle.get("text", "")})
    orph = device["orphans"]
    for kind in ("pools", "nodes", "rules", "monitors", "profiles"):
        n = len(orph.get(kind, []))
        if n:
            out.append(
                {
                    "level": "warn",
                    "text": f"{n} orphaned {kind} (defined, referenced by nothing "
                    "— no iRule can attach them either)",
                }
            )
    # Possible orphans: no static reference, but an iRule selects that type
    # dynamically, so they cannot be *proven* unused.
    poss = device.get("possibleOrphans", {})
    for kind in ("pools", "nodes", "snatpools"):
        n = len(poss.get(kind, []))
        if n:
            out.append(
                {
                    "level": "info",
                    "text": f"{n} {kind} have no static reference but an iRule could "
                    "build a matching name dynamically — can't be proven unused",
                }
            )
    empty_pools = [p["name"] for p in device["pools"] if p["memberCount"] == 0]
    if empty_pools:
        preview = ", ".join(empty_pools[:6]) + ("…" if len(empty_pools) > 6 else "")
        out.append(
            {
                "level": "warn",
                "text": f"{len(empty_pools)} pool(s) with no members: {preview}",
            }
        )
    no_pool_vs = [
        v["name"] for v in device["virtuals"] if not v["pool"] and not v["policies"]
    ]
    if no_pool_vs:
        out.append(
            {
                "level": "info",
                "text": f"{len(no_pool_vs)} virtual server(s) with no default pool "
                "(forwarding / policy-driven)",
            }
        )
    disabled_vs = [v["name"] for v in device["virtuals"] if v["disabled"]]
    if disabled_vs:
        out.append(
            {"level": "info", "text": f"{len(disabled_vs)} disabled virtual server(s)"}
        )
    ssl = [p for p in device["profiles"] if "SSL" in p["type"]]
    if ssl:
        out.append({"level": "info", "text": f"{len(ssl)} SSL profile(s) in use"})
    if not out:
        out.append(
            {"level": "ok", "text": "No orphaned objects or empty pools detected"}
        )
    return out


_SINGULAR_KIND = {
    "virtuals": "virtual",
    "pools": "pool",
    "nodes": "node",
    "monitors": "monitor",
    "rules": "rule",
    "dataGroups": "data-group",
    "profiles": "profile",
    "policies": "policy",
    "snatpools": "snatpool",
    "persistence": "persistence",
    "certificates": "certificate",
}


def _split_full_path(full_path: str) -> tuple[str, list[str], str] | None:
    """`/partition/seg1/…/name` -> (partition, [folder segments], name)."""
    if not full_path.startswith("/"):
        return None
    parts = [p for p in full_path[1:].split("/") if p]
    if len(parts) < 2:
        return None
    return parts[0], parts[1:-1], parts[-1]


def _folder_of(full_path: str) -> str:
    """Folder path (`/Common/App_X`), or `""` for a partition-root object."""
    split = _split_full_path(full_path)
    if not split or not split[1]:
        return ""
    partition, segments, _ = split
    return "/" + "/".join([partition, *segments])


def _app_folder_of(full_path: str) -> tuple[str, str] | None:
    """(partition, first sub-folder segment), or None for partition-root."""
    split = _split_full_path(full_path)
    if not split or not split[1]:
        return None
    return split[0], split[1][0]


def _build_apps(device: dict[str, Any]) -> list[dict[str, Any]]:
    """Group objects into applications by their application folder (partition +
    first sub-folder). Partition-root objects are left ungrouped; iApps are
    captured via their ``<name>.app`` folders. Mirrors the Rust ``build_apps``.
    """
    acc: dict[tuple[str, str], dict[str, Any]] = {}
    for key in _DISPLAY_KEYS:
        kind = _SINGULAR_KIND.get(key, "object")
        for o in device.get(key, []):
            if not isinstance(o, dict):
                continue
            fp = o.get("fullPath", "")
            app_folder = _app_folder_of(fp)
            if app_folder is None:
                continue
            part, seg = app_folder
            bucket = acc.setdefault((part, seg), {"members": [], "entryPoints": []})
            bucket["members"].append(
                {
                    "kind": kind,
                    "name": o.get("name", ""),
                    "fullPath": fp,
                    "partition": part,
                }
            )
            if key == "virtuals":
                bucket["entryPoints"].append(fp)

    apps: list[dict[str, Any]] = []
    for (part, seg), bucket in sorted(acc.items()):
        is_iapp = seg.endswith(".app")
        apps.append(
            {
                "name": seg[: -len(".app")] if is_iapp else seg,
                "partition": part,
                "folder": f"/{part}/{seg}",
                "source": "iapp" if is_iapp else "folder",
                "memberCount": len(bucket["members"]),
                "entryPoints": bucket["entryPoints"],
                "members": bucket["members"],
            }
        )
    return apps


def _collect_device(uri: str, source: str, sources: Sources) -> dict[str, Any]:
    source_only: Sources = [(uri, source)]

    # One reference-graph walk per referable container, up front.
    refmaps = {name: _refmap(source_only, _CONTAINERS[name]) for name in _REFERABLE}

    tmsh = _TMSH_RE.search(source)
    tmsh_version = tmsh.group(1) if tmsh else ""
    device: dict[str, Any] = {
        "uri": uri,
        "name": _device_name(uri, source),
        "tmshVersion": tmsh_version,
        "releaseLifecycle": json.loads(_engine.bigip_release_lifecycle(tmsh_version)),
    }

    for key, container in _CONTAINERS.items():
        rows = _fields(_engine.query(f"{container}[]", source_only))
        shaper = _SHAPERS.get(key)
        used_by = refmaps.get(key, {})
        if shaper:
            device[key] = [shaper(f, used_by) for f in rows]
        else:
            # snatpools / persistence / virtual-addresses: keep the projected
            # fields, tidy up the name/full-path for display. `usedBy` is carried
            # through so referable kinds (snatpools) are not miscounted as
            # orphans by the orphan pass below.
            device[key] = [
                {
                    "name": f.get("name", ""),
                    "fullPath": f.get("full-path", ""),
                    "usedBy": used_by.get(f.get("full-path", ""), []),
                    "fields": f,
                }
                for f in rows
            ]

    # Order each virtual's attached profiles into traffic (protocol-stack)
    # order via the shared engine — transport → TLS → application — so a
    # listener reads TCP → … → HTTP, not the raw config order. The config's
    # typed profile inventory is authoritative; the engine falls back to
    # well-known default-profile names for base profiles it doesn't define.
    type_of = {
        p["fullPath"]: p.get("type", "")
        for p in device.get("profiles", [])
        if isinstance(p, dict) and p.get("fullPath")
    }
    for v in device.get("virtuals", []):
        if isinstance(v, dict) and isinstance(v.get("profiles"), list):
            v["profiles"] = _engine.order_profiles(v["profiles"], type_of)

    # Tag built-in / system objects (default profiles, monitors, `_sys_*` iRules
    # from profile_base.conf / low_profile_base.conf) so they can be hidden by
    # default and kept out of orphan analysis, counts and diagrams.
    defaults = _default_object_paths(source)
    for key in _DISPLAY_KEYS:
        for o in device.get(key, []):
            if isinstance(o, dict):
                fp = o.get("fullPath", "")
                o["isDefault"] = fp in defaults or fp.rsplit("/", 1)[-1].startswith(
                    "_sys_"
                )

    # Link cross-iRule `call <rule>::<proc>` references before orphan analysis so
    # a proc-library iRule is counted as used by its callers.
    _graph.link_proc_calls(device)

    # Orphans: a referable leaf object is *confirmed* orphaned only when it has
    # an empty referrer set AND no iRule could dynamically attach an object of
    # its *name*, resolved per partition. Each attach expression is reconstructed
    # into a prefix/contained/suffix name pattern (``pool "web_[HTTP::host]"`` →
    # ``web_*``) and matched in the partition context of the attaching virtuals;
    # an object is a *possible* orphan only when some rule's pattern could build
    # its name in a partition where the rule actually runs.
    rule_ctx, rule_vs = _graph.build_rule_contexts(device)
    attach_idx = _graph.attach_index(device.get("rules", []), rule_ctx)
    device["orphanRisk"] = sorted(attach_idx.keys())
    device["attachPatterns"] = {
        ty: [{k: v for k, v in p.items() if k != "ctx"} for p in pats]
        for ty, pats in attach_idx.items()
    }
    device["orphans"] = {}
    device["possibleOrphans"] = {}
    for name in _REFERABLE:
        if name not in device:
            continue
        attaches = attach_idx.get(name, [])
        confirmed: list[str] = []
        possible: list[str] = []
        for o in device.get(name, []):
            if not isinstance(o, dict):
                continue
            if o.get("isDefault"):
                o["orphanStatus"] = ""  # built-in/system objects are never orphans
            elif o.get("usedBy"):
                o["orphanStatus"] = ""
            elif not attaches:
                o["orphanStatus"] = "orphan"
                confirmed.append(o["name"])
            else:
                fp = o.get("fullPath", "")
                matches = _graph.attach_matches(
                    attaches,
                    o.get("name", ""),
                    fp,
                    _graph._partition_of(fp),
                    o.get("address", ""),
                )
                if matches:
                    o["orphanStatus"] = "possible"
                    o["orphanMatches"] = matches
                    possible.append(o["name"])
                else:
                    o["orphanStatus"] = "orphan"
                    confirmed.append(o["name"])
        device["orphans"][name] = confirmed
        device["possibleOrphans"][name] = possible

    # Per-iRule reachability table: static references + the objects each dynamic
    # filter could select, resolved per attaching-virtual partition (VS-specific).
    _graph.annotate_rule_reachability(device, rule_vs)

    # Propagate each iRule's dynamic (runtime) actions onto the virtuals that
    # attach it, so a VS shows profiles/pools it changes at runtime, not just
    # its statically-attached ones.
    rule_actions = {r["fullPath"]: r["dynamicActions"] for r in device["rules"]}
    rule_by_name = {
        r["fullPath"].split("/")[-1]: r["fullPath"] for r in device["rules"]
    }
    for v in device["virtuals"]:
        acts: list[dict[str, str]] = []
        for rule_ref in v["rules"]:
            fp = (
                rule_ref
                if rule_ref in rule_actions
                else rule_by_name.get(rule_ref.split("/")[-1], "")
            )
            for a in rule_actions.get(fp, []):
                acts.append({**a, "rule": rule_ref.split("/")[-1]})
        v["dynamicProfiles"] = acts

    device["graph"] = _graph.build_graph(device)
    # The raw SCF/config text, embedded so the in-browser wasm query console can
    # run live queries against this exact device.
    device["configText"] = source

    # SSL certificate inventory & expiry (read from the parsed `sys file
    # ssl-cert` stanzas, cross-referenced against the SSL profiles / virtuals).
    device["certificates"] = _certs.collect_certs(source_only, device)

    # Secret inventory (Secrets tab). Values are clear text only when the config
    # was decrypted with the f5mku master key upstream in collect_model.
    device["secrets"] = json.loads(_engine.list_secrets(source))

    # Model-level BIG-IP and iApp diagnostics. The Rust validator is the
    # single source of truth; the complete source set supplies cross-file iApp
    # evidence, and each finding is routed to the same tab as native/wasm.
    device["configDiagnostics"] = json.loads(
        _engine.report_diagnostics(uri, source, sources)
    )
    routed = {
        "virtuals": "virtualDiagnostics",
        "rules": "ruleDiagnostics",
        "pools": "poolDiagnostics",
        "dataGroups": "dataGroupDiagnostics",
        "apps": "appDiagnostics",
        "objectIndex": "objectDiagnostics",
    }
    for tab, key in routed.items():
        device[key] = [
            diagnostic
            for diagnostic in device["configDiagnostics"]
            if diagnostic.get("tab") == tab
        ]
    device["iappDiagnosticEvidence"] = json.loads(
        _engine.iapp_diagnostic_evidence(uri, sources)
    )

    # Tag every displayed object with its partition (from the full path) and
    # collect the device's partition set, so the report can filter to a
    # partition while always keeping shared /Common objects visible.
    partitions: set[str] = set()
    for key in _DISPLAY_KEYS:
        for o in device.get(key, []):
            if not isinstance(o, dict):
                continue
            part = o.get("partition") or _graph._partition_of(o.get("fullPath", ""))
            if part:
                partitions.add(part)
            o["partition"] = part
            o["folder"] = _folder_of(o.get("fullPath", ""))
    device["partitions"] = sorted(partitions)

    # Auto-detected applications, grouped by folder (iApps included via their
    # `.app` folders). Objects in the partition root are left ungrouped, so a
    # partition with no sub-folders yields no apps.
    device["apps"] = _build_apps(device)

    # Counts exclude built-in/system defaults — the chips count the estate's own
    # objects, not the ~260 TMOS defaults.
    device["counts"] = {
        key: sum(
            1
            for o in device.get(key, [])
            if not (isinstance(o, dict) and o.get("isDefault"))
        )
        for key in _CONTAINERS
    }
    device["counts"]["poolMembers"] = sum(p["memberCount"] for p in device["pools"])
    device["counts"]["orphans"] = sum(len(v) for v in device["orphans"].values())
    device["counts"]["apps"] = len(device["apps"])
    device["counts"]["certificates"] = len(device["certificates"])
    device["counts"]["secrets"] = len(device["secrets"])
    device["counts"]["configDiagnostics"] = len(device["configDiagnostics"])
    # The Forensics tab is driven by the UCS file inventory, which is extracted
    # at the Rust/WASM entry point; the Python library path does not pull UCS
    # members, so it reports zero forensic files (the tab stays hidden).
    device["forensics"] = {"files": [], "checklist": []}
    device["counts"]["files"] = 0
    device["insights"] = _insights(device)
    return device


def collect_model(
    sources: Sources,
    *,
    title: str = "F5 BIG-IP Configuration Report",
    master_key: str | None = None,
    manifest: str | None = None,
    copyright: str = "",
    front_matter: str = "",
    logo: str = "",
) -> dict[str, Any]:
    """Build the full report model from loaded ``(uri, text)`` sources.

    ``sources`` is what :func:`f5report.load_paths` returns. The result is a
    plain ``dict`` — render it with :func:`f5report.render.render_report`, dump
    it to JSON, or assert against it in tests.

    ``master_key`` is the base64 unit master key (``f5mku -K``); when given,
    every ``$M$…`` secret in each config (SSL key passphrases, monitor / RADIUS /
    SNMP secrets, …) is decrypted before the model is built.

    ``manifest`` is the optional architecture/topology DSL (roles, tiers,
    explicit links, zones, …) from the builder page; when given it overrides the
    engine's auto-detection, matching the WASM backend.
    """
    if master_key:
        sources = [
            (uri, _engine.decrypt_secrets(src, master_key)) for uri, src in sources
        ]
    devices = [_collect_device(uri, src, sources) for uri, src in sources]

    totals: dict[str, int] = {}
    for d in devices:
        for k, v in d["counts"].items():
            totals[k] = totals.get(k, 0) + v

    # Cross-device architecture (roles, tiers, links, topology diagram). Reuses
    # the Rust generator's detection via the native engine so the Python report
    # exposes the identical tier model — which the global search reads to scope
    # `t<n>:` queries and to label results by tier. A builder manifest, when
    # supplied, overrides auto-detection.
    architecture = json.loads(
        _engine.build_architecture(json.dumps(devices), manifest or None)
    )
    # Address/port enrichment (zone + CIDR-name + service labels) — same Rust
    # resolver the native generator uses, for parity.
    enrichment = json.loads(
        _engine.build_enrichment(json.dumps(devices), json.dumps(architecture))
    )

    return {
        "title": title,
        "copyright": copyright,
        # User-supplied Markdown front-matter, rendered to HTML by the same
        # shared renderer the Rust/wasm backends use (raw HTML stripped). Empty
        # keeps the template's front-matter tab hidden.
        "front_matter_html": (
            _engine.render_markdown(front_matter) if front_matter.strip() else ""
        ),
        # Report logo as an inlined data: URI (single-file); shown in the header.
        "logo": logo,
        "engine_version": _engine.__version__,
        "git_hash": getattr(_engine, "__git_hash__", "unknown"),
        # Single `git describe --tags` version (v-tag + commits + hash) for the footer.
        "version": getattr(_engine, "__git_describe__", "unknown"),
        # Backend badge shown in the footer; the Rust generator's model sets "rust".
        # Drives the `{{ backend }}` template variable.
        "backend": "py",
        "devices": devices,
        "architecture": architecture,
        "enrichment": enrichment,
        "totals": totals,
        "container_order": list(_CONTAINERS),
    }


def build_report(
    sources: Sources,
    *,
    title: str = "F5 BIG-IP Configuration Report",
    embed_console: bool | None = None,
    master_key: str | None = None,
    report_id: str = "",
    manifest: str | None = None,
    copyright: str = "",
    front_matter: str = "",
    logo: str = "",
) -> str:
    """Collect the model and render it to a standalone HTML document.

    ``embed_console=False`` omits the in-browser WASM query console (smaller
    page; suitable for hosting behind a strict CSP). ``master_key`` (the base64
    ``f5mku -K`` value) decrypts the config's ``$M$…`` secrets first. ``manifest``
    is the optional builder DSL that overrides architecture auto-detection.
    """
    return render_report(
        collect_model(
            sources,
            title=title,
            master_key=master_key,
            manifest=manifest,
            copyright=copyright,
            front_matter=front_matter,
            logo=logo,
        ),
        embed_console=embed_console,
        report_id=report_id,
    )
