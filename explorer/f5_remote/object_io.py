"""Single-object iControl REST round-trip helpers.

Used by ``f5 pull`` and ``f5 push`` to fetch / send one virtual, pool,
or iRule from / to a live device.

Endpoint shape (iControl REST):

  GET  /mgmt/tm/<module>/<type>/<encoded-name>      -> JSON describing the object
  PUT  /mgmt/tm/<module>/<type>/<encoded-name>      -> replace the object
  POST /mgmt/tm/<module>/<type>                     -> create a new object

The ``<encoded-name>`` for a full-path is `~Common~name` (slashes
become tildes).
"""

from __future__ import annotations

import json
from urllib.parse import quote

from .auth import Credentials
from .rest import RestError, _auth_header, _make_connection, _request


def _encode_path(full_path: str) -> str:
    """Convert ``/Common/foo`` -> ``~Common~foo`` (iControl REST format)."""
    if not full_path.startswith("/"):
        return quote(full_path, safe="")
    return full_path.replace("/", "~")


def _kind_to_endpoint(kind: str) -> tuple[str, str]:
    """Map our kind name to (module, type) URI segments."""
    table = {
        "virtual": ("ltm", "virtual"),
        "pool": ("ltm", "pool"),
        "node": ("ltm", "node"),
        "rule": ("ltm", "rule"),
        "monitor-http": ("ltm", "monitor/http"),
        "monitor-tcp": ("ltm", "monitor/tcp"),
        "data-group-internal": ("ltm", "data-group/internal"),
    }
    if kind not in table:
        raise ValueError(f"unsupported object kind {kind!r} (supported: {sorted(table)})")
    return table[kind]


def pull_object(
    credentials: Credentials,
    *,
    kind: str,
    full_path: str,
    insecure: bool = True,
    timeout: float = 60.0,
) -> dict:
    """GET one object via iControl REST and return its JSON dict."""
    module, otype = _kind_to_endpoint(kind)
    encoded = _encode_path(full_path)
    auth = _auth_header(credentials)
    conn = _make_connection(credentials, insecure=insecure, timeout=timeout)
    try:
        status, _hdrs, data = _request(
            conn, "GET", f"/mgmt/tm/{module}/{otype}/{encoded}", auth=auth
        )
        if status >= 400:
            raise RestError(f"GET {full_path}: HTTP {status}: {data[:200]!r}")
        return json.loads(data)
    finally:
        conn.close()


def push_object(
    credentials: Credentials,
    *,
    kind: str,
    payload: dict,
    create: bool = False,
    insecure: bool = True,
    timeout: float = 60.0,
) -> dict:
    """PUT (default) or POST (create=True) one object via iControl REST."""
    module, otype = _kind_to_endpoint(kind)
    auth = _auth_header(credentials)
    conn = _make_connection(credentials, insecure=insecure, timeout=timeout)
    try:
        if create:
            method = "POST"
            path = f"/mgmt/tm/{module}/{otype}"
        else:
            full_path = payload.get("fullPath") or payload.get("name")
            if not full_path:
                raise ValueError("payload must include fullPath or name")
            method = "PUT"
            path = f"/mgmt/tm/{module}/{otype}/{_encode_path(full_path)}"
        body = json.dumps(payload).encode("utf-8")
        status, _hdrs, data = _request(conn, method, path, auth=auth, body=body)
        if status >= 400:
            raise RestError(f"{method} {path}: HTTP {status}: {data[:200]!r}")
        return json.loads(data) if data else {}
    finally:
        conn.close()


def object_to_scf_stanza(kind: str, obj: dict) -> str:
    """Render an iControl REST object dict as an SCF stanza (best-effort).

    The full reverse mapping is enormous; this covers virtuals, pools,
    nodes, and rules — enough for ``pull`` to produce a useful
    diffable artefact.
    """
    name = obj.get("fullPath") or obj.get("name", "")
    if kind == "virtual":
        lines = [f"ltm virtual {name} {{"]
        if obj.get("destination"):
            lines.append(f"    destination {obj['destination']}")
        if obj.get("pool"):
            lines.append(f"    pool {obj['pool']}")
        rules = obj.get("rules") or []
        if rules:
            lines.append("    rules {")
            for r in rules:
                lines.append(f"        {r}")
            lines.append("    }")
        lines.append("}")
        return "\n".join(lines) + "\n"
    if kind == "pool":
        lines = [f"ltm pool {name} {{"]
        members = obj.get("membersReference", {}).get("items") or obj.get("members") or []
        if members:
            lines.append("    members {")
            for m in members:
                mname = m.get("fullPath") or m.get("name", "")
                lines.append(f"        {mname} {{")
                if m.get("address"):
                    lines.append(f"            address {m['address']}")
                lines.append("        }")
            lines.append("    }")
        if obj.get("monitor"):
            lines.append(f"    monitor {obj['monitor']}")
        if obj.get("loadBalancingMode"):
            lines.append(f"    load-balancing-mode {obj['loadBalancingMode']}")
        lines.append("}")
        return "\n".join(lines) + "\n"
    if kind == "node":
        lines = [f"ltm node {name} {{"]
        if obj.get("address"):
            lines.append(f"    address {obj['address']}")
        lines.append("}")
        return "\n".join(lines) + "\n"
    if kind == "rule":
        body = obj.get("apiAnonymous", "")
        return f"ltm rule {name} {{\n{body}\n}}\n"
    # Fallback: emit JSON as a comment so nothing's lost.
    return f"# {kind} {name}\n# {json.dumps(obj)}\n"
