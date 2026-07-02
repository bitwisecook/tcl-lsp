"""Wire the F5 explain-flow simulation to the Rust bytecode VM.

`explain_flow(--simulate)` runs the matched virtual server's iRule(s) against
the captured session. This adapter drives that simulation through the Rust
`tcl_lsp_py.simulate_irule` facade (which runs the embedded orchestrator on
`tcl-vm`) instead of the retiring Python c-tcl orchestrator.

It is **runtime-ready**: the VM lights this up as it grows the orchestrator
command surface. Until then every VM/orchestrator failure lands in the returned
``error`` (best-effort) so the static explain-flow report still renders. This
wires the simulation to the VM/runtime — it does not modify those components.

The extraction of iRule sources / profiles / pools / request from the F5 model
(`cfg` / `vs` / `session`) stays Python (it operates on the `dialects.f5` model
objects); only the *execution* moves to the Rust VM.
"""

from __future__ import annotations

from typing import Any


def _profiles_for(cfg: Any, vs: Any) -> list[str]:
    """The orchestrator profile labels (TCP / CLIENTSSL / HTTP / …) for *vs*."""
    from dialects.f5.bigip.model import ProfileType

    types = cfg.profile_types_for_virtual(vs.full_path)
    out: list[str] = []
    if ProfileType.TCP in types or vs.profiles:
        out.append("TCP")
    if ProfileType.UDP in types:
        out.append("UDP")
    if ProfileType.CLIENT_SSL in types:
        out.append("CLIENTSSL")
    if ProfileType.SERVER_SSL in types:
        out.append("SERVERSSL")
    if ProfileType.HTTP in types:
        out.append("HTTP")
    return out or ["TCP"]


def rust_irule_simulator(cfg: Any, vs: Any, session: Any) -> dict:
    """Simulate *vs*'s iRule(s) for *session* on the Rust VM.

    Returns ``{pool, node, response_committed, logs, decisions, error}`` — the
    same shape the explain-flow report consumes (``decisions`` as
    ``(category, action, value)`` tuples). Best-effort: a non-empty ``error``
    means the run could not complete and the other fields are defaulted.
    """
    from ai.shared.rust_bridge import require_rust

    out = {
        "pool": "",
        "node": "",
        "response_committed": False,
        "logs": [],
        "decisions": [],
        "error": "",
    }

    # Collect every attached iRule's source, resolving short refs.
    rules: list[str] = []
    for rref in vs.rules:
        resolved = cfg.resolve_rule(rref) or rref
        rule = cfg.rules.get(resolved)
        if rule is not None:
            rules.append(rule.source)
    if not rules:
        return {**out, "error": "no iRules attached to VS — nothing to simulate"}

    front = session.front.client
    profiles = _profiles_for(cfg, vs)

    # Every config pool as (short-name, [member node names]) so a `pool foo`
    # call inside an iRule resolves.
    pools: list[tuple[str, list[str]]] = []
    for pname, pool_obj in cfg.pools.items():
        members = [m.name.rsplit("/", 1)[-1] for m in pool_obj.members if m.name]
        if members:
            pools.append((pname.rsplit("/", 1)[-1], members))

    result = require_rust().simulate_irule(
        "\n".join(rules),
        method=front.http_method or "GET",
        uri=front.http_uri or "/",
        host=front.http_host or "",
        sni=front.tls_sni or "",
        profiles=profiles,
        pools=pools,
    )
    return {
        "pool": result["pool"],
        "node": result["node"],
        "response_committed": result["response_committed"],
        "logs": result["logs"],
        "decisions": [(d["category"], d["action"], d["value"]) for d in result["decisions"]],
        "error": result["error"],
    }
