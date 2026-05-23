"""iRule-simulation bridge for ``f5 explain-flow``.

This is the *adapter* that drives the iRule-test orchestrator
(:class:`tooling.irule_test.IruleTestSession`) for a matched virtual
server and captured session, returning the dynamic outcome (selected
pool/node, logs, decisions).

It lives in ``tooling/`` — not ``dialects/`` — so the pure flow
analyser (``dialects.f5.bigip.explain_flow``) stays free of any
dependency on the developer test framework.  The analyser exposes an
``IruleSimulator`` injection point; the CLI / MCP adapters pass
:func:`simulate_irule_for_session` only when dynamic simulation is
requested.
"""

from __future__ import annotations

from dialects.f5.bigip.explain_flow import Session
from dialects.f5.bigip.model import BigipConfig, BigipVirtualServer, ProfileType


def profiles_for_orchestrator(cfg: BigipConfig, vs: BigipVirtualServer) -> list[str]:
    """Return the orchestrator profile labels (TCP / CLIENTSSL / HTTP / …) for *vs*."""
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


def simulate_irule_for_session(
    cfg: BigipConfig,
    vs: BigipVirtualServer,
    session: Session,
) -> dict:
    """Run the matched VS's iRule under the C-tcl orchestrator with captured state.

    Returns a dict with keys: ``pool``, ``node``, ``response_committed``,
    ``logs`` (list of preformatted strings), ``decisions`` (list of
    ``(category, action, value)``), ``error`` (str, empty on success).

    Best-effort — any exception starting the orchestrator or running
    the request is captured into ``error`` and returned, so the caller
    can still emit static analysis even when the simulation backend is
    unavailable (e.g. no `tclsh` on PATH).
    """
    import asyncio

    out: dict = {
        "pool": "",
        "node": "",
        "response_committed": False,
        "logs": [],
        "decisions": [],
        "error": "",
    }

    rules = []
    for rref in vs.rules:
        resolved = cfg.resolve_rule(rref) or rref
        rule = cfg.rules.get(resolved)
        if rule is not None:
            rules.append(rule.source)
    if not rules:
        out["error"] = "no iRules attached to VS — nothing to simulate"
        return out

    front = session.front.client
    headers: dict[str, str] = dict(front.http_request_headers)
    method = front.http_method or "GET"
    uri = front.http_uri or "/"
    host = front.http_host or ""
    sni = front.tls_sni or ""

    profiles = profiles_for_orchestrator(cfg, vs)

    async def _run() -> dict:
        try:
            from tooling.irule_test import IruleTestSession
        except ImportError as exc:
            return {**out, "error": f"irule_test framework unavailable: {exc}"}

        try:
            sess = IruleTestSession(profiles=profiles)
        except Exception as exc:  # pragma: no cover - defensive
            return {**out, "error": f"cannot construct IruleTestSession: {exc}"}

        try:
            async with sess:
                for source in rules:
                    sess.load_irule(source)
                # Add every pool referenced by the VS (default + any
                # ``pool foo`` calls inside iRules will resolve through
                # this set).
                for pname, pool_obj in cfg.pools.items():
                    members = [m.name.rsplit("/", 1)[-1] for m in pool_obj.members if m.name]
                    if members:
                        await sess.add_pool(pname.rsplit("/", 1)[-1], members)
                if vs.pool:
                    resolved_pool = cfg.resolve_pool(vs.pool)
                    if resolved_pool is not None:
                        members = [
                            m.name.rsplit("/", 1)[-1]
                            for m in cfg.pools[resolved_pool].members
                            if m.name
                        ]
                        if members:
                            await sess.add_pool(resolved_pool.rsplit("/", 1)[-1], members)
                if "HTTP" in profiles:
                    result = await sess.run_http_request(
                        method=method, uri=uri, host=host, headers=headers or None, sni=sni
                    )
                else:
                    # Non-HTTP: just fire CLIENT_ACCEPTED to exercise
                    # any L4 logic the iRule has.
                    await sess.fire_event("CLIENT_ACCEPTED")
                    result = None
        except Exception as exc:  # pragma: no cover - subprocess failure path
            return {**out, "error": f"orchestrator failure: {exc}"}

        if result is None:
            return out

        decisions: list[tuple[str, str, str]] = []
        for d in result.decisions or []:
            if isinstance(d, (list, tuple)) and len(d) >= 2:
                cat = str(d[0])
                act = str(d[1])
                val = ""
                if len(d) > 2:
                    val = " ".join(
                        str(x) for x in (d[2] if isinstance(d[2], (list, tuple)) else [d[2]])
                    )
                decisions.append((cat, act, val))
        log_lines: list[str] = []
        for entry in result.logs or []:
            if isinstance(entry, (list, tuple)):
                log_lines.append(" | ".join(str(x) for x in entry))
            else:
                log_lines.append(str(entry))

        return {
            "pool": result.pool_selected or "",
            "node": result.node_selected or "",
            "response_committed": bool(result.http_response_committed),
            "logs": log_lines,
            "decisions": decisions,
            "error": "",
        }

    try:
        return asyncio.run(_run())
    except RuntimeError as exc:
        # Already inside an event loop (rare from CLI but possible from MCP).
        return {**out, "error": f"asyncio: {exc}"}
