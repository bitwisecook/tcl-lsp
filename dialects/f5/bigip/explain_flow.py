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

"""``f5 explain-flow`` core: trace each flow in a PCAP through the BIG-IP config.

For every unique L3/L4 flow observed in a capture, locate the virtual
server whose ``destination`` matches the flow's destination IP+port and
emit a per-flow plan:

* ordered profiles attached to the VS (TCP, client-ssl, http, …);
* attached LTM policies and APM access profiles (best-effort, via
  :class:`BigipGenericObject`);
* the iRule chain, with the *expected* event firing order computed
  from the attached profiles and the L7 features actually seen in the
  capture (TLS ClientHello, HTTP request, server connect, …);
* for each fired event, the verbatim ``when EVENT { … }`` block from
  the iRule — the static "path through the iRule" the operator should
  read to understand what executed;
* persistence, SNAT, default pool & members;
* GTM wide-IPs whose body references the matched virtual server.

True symbolic execution of an iRule against captured payload bytes
(branch-by-branch tracing) is out of scope for this module — we surface
the event blocks that would fire and let the operator read the Tcl.

Packet decoding is done in two layers:

1.  An always-available built-in walker (:mod:`dialects.f5.bigip.pcap_remap`'s
    libpcap + pcapng readers) extracts the IPv4 / IPv6 5-tuple, TCP
    flags, and the first L4 payload byte of each packet.
2.  Optional ``tshark`` post-processing (when the binary is available
    and the caller passes ``use_tshark=True``) enriches each flow with
    HTTP method / host / URI, TLS SNI / version, and any decoded
    fields ``tshark -T fields`` can emit.  Absence of tshark degrades
    gracefully: the static iRule event ordering still works for
    HTTP/TLS by inspecting the attached profiles and the observed
    well-known ports.

This module never re-parses the BIG-IP config; it only reads what
:func:`dialects.f5.bigip.parser.parse_bigip_conf` already produced.
"""

from __future__ import annotations

import ipaddress
import re
from collections.abc import Callable
from pathlib import Path

from .explain import compute_explain
from .flow._model import (
    Connection,
    ExplainFlowReport,
    Flow,
    Session,
    SessionExplain,
)
from .flow.packets import extract_flows
from .flow.sessions import pair_sessions
from .flow.tshark import enrich_with_tshark, extract_flows_via_tshark, tshark_available
from .model import BigipConfig, BigipVirtualServer, ProfileType
from .policy_eval import (
    PolicyDecision,
    evaluate_policy,
    request_state_from_session,
)

_DEST_RE = re.compile(
    r"^(?P<path>/[^/\s]+/)?(?P<addr>\[[^\]]+\]|[0-9a-fA-F\.:]+)(?:%\d+)?:(?P<port>\d+|any)$"
)


def _parse_destination(dest: str) -> tuple[str, int] | None:
    """Parse a VS destination string like ``/Common/10.0.0.1:443`` or ``/p/[::1]:80``.

    Returns the address in :mod:`ipaddress`-canonical form so it
    compares equal to flow IPs, which we always render through
    ``ipaddress.IPv4Address``/``IPv6Address`` (e.g. config dest
    ``"0:0:0:0:0:0:0:1"`` matches flow dst ``"::1"``).
    """
    m = _DEST_RE.match(dest.strip())
    if not m:
        return None
    addr = m.group("addr").strip("[]")
    port_raw = m.group("port")
    try:
        canonical = str(ipaddress.ip_address(addr))
    except ValueError:
        return None
    port = 0 if port_raw == "any" else int(port_raw)
    return canonical, port


def _match_virtual(cfg: BigipConfig, dst_ip: str, dst_port: int) -> str | None:
    # Canonicalise the flow IP too so v6 forms match regardless of
    # which side typed the address.  Invalid IPs fall through and
    # never match.
    try:
        flow_ip = str(ipaddress.ip_address(dst_ip))
    except ValueError:
        flow_ip = dst_ip
    for path, vs in cfg.virtual_servers.items():
        dest_text = str(vs.destination) if vs.destination is not None else ""
        parsed = _parse_destination(dest_text)
        if parsed is None:
            continue
        vs_addr, vs_port = parsed
        if vs_addr != flow_ip:
            continue
        if vs_port == 0 or vs_port == dst_port:
            return path
    return None


# ---------------------------------------------------------------------------
# iRule event ordering & block extraction.
# ---------------------------------------------------------------------------


# Canonical lifecycle order for L4/SSL/HTTP events as they fire on a single
# request/response cycle.  Used to sort the events that *could* fire for a
# given flow into the order an operator expects to see.
_EVENT_ORDER = (
    "RULE_INIT",
    "CLIENT_ACCEPTED",
    "CLIENTSSL_HANDSHAKE",
    "CLIENTSSL_CLIENTHELLO",
    "CLIENTSSL_SERVERHELLO_SEND",
    "CLIENTSSL_DATA",
    "HTTP_REQUEST",
    "HTTP_REQUEST_DATA",
    "HTTP_REQUEST_SEND",
    "LB_SELECTED",
    "LB_FAILED",
    "SERVER_CONNECTED",
    "SERVERSSL_HANDSHAKE",
    "SERVERSSL_DATA",
    "HTTP_RESPONSE",
    "HTTP_RESPONSE_CONTINUE",
    "HTTP_RESPONSE_DATA",
    "CLIENT_DATA",
    "SERVER_DATA",
    "HTTP_DISCONNECT",
    "CLIENT_CLOSED",
    "SERVER_CLOSED",
)
_EVENT_ORDER_INDEX = {name: i for i, name in enumerate(_EVENT_ORDER)}


def _extract_event_blocks(rule_source: str) -> dict[str, str]:
    """Return ``{event_name: body}`` for every ``when EVENT { … }`` block.

    Brace-aware extractor — handles nested braces inside the body.  Any
    event whose body is malformed (unbalanced braces, no opening
    brace) is skipped silently; the caller already has the full
    source for fallback display.
    """
    blocks: dict[str, str] = {}
    pattern = re.compile(r"\bwhen\s+([A-Z][A-Z0-9_]*)\s*\{", re.MULTILINE)
    for m in pattern.finditer(rule_source):
        event = m.group(1)
        start = m.end()  # index just after the opening "{"
        depth = 1
        i = start
        n = len(rule_source)
        while i < n and depth > 0:
            ch = rule_source[i]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            i += 1
        if depth == 0:
            blocks[event] = rule_source[start : i - 1]
    return blocks


# ---------------------------------------------------------------------------
# HUD-command annotation: tie iRule commands to captured state.
# ---------------------------------------------------------------------------


# Patterns that recognise iRule commands which read captured connection
# state.  Each entry is a regex that matches the command (and optional
# argument) inside ``[...]`` brackets, plus a callable that turns a
# Session into the captured value the command would have returned.
def _hud_value_for(token: str, conn: Connection) -> str | None:
    """Resolve an iRule command token like ``HTTP::host`` against *conn*."""
    f = conn.client
    s = conn.server
    t = token
    # HTTP::* on the request side
    if t == "HTTP::host":
        return f.http_host or None
    if t == "HTTP::method":
        return f.http_method or None
    if t == "HTTP::uri":
        return f.http_uri or None
    if t == "HTTP::path":
        return f.http_path or None
    if t == "HTTP::query":
        return f.http_query or None
    if t == "HTTP::version":
        return f.http_request_version or None
    if t == "HTTP::status":
        return (s.http_response_code if s is not None else "") or None
    if t.startswith("HTTP::header "):
        name = t.split(None, 1)[1].strip().strip('"').strip("'").lower()
        if name in f.http_request_headers:
            return f.http_request_headers[name]
        if s is not None and name in s.http_response_headers:
            return s.http_response_headers[name]
        return None
    if t == "HTTP::cookie":
        return f.http_cookie or None
    if t == "HTTP::user_agent":
        return f.http_user_agent or None
    # SSL::*
    if t == "SSL::extensions":
        bits = []
        if f.tls_sni:
            bits.append(f"sni={f.tls_sni}")
        if f.tls_alpn:
            bits.append(f"alpn={f.tls_alpn}")
        return ", ".join(bits) if bits else None
    if t == "SSL::cipher":
        return f.tls_chosen_cipher or None
    if t == "SSL::cipher version":
        return f.tls_chosen_version or None
    if t == "SSL::cert subject":
        return f.tls_cert_subject or None
    if t == "SSL::cert":
        return f.tls_cert_subject or None
    # SNI::* style tokens that some iRules use.
    if t.upper() == "SNI" or t == "SSL::extensions servername":
        return f.tls_sni or None
    # IP / TCP
    if t == "IP::client_addr":
        return f.src_ip
    if t == "IP::local_addr":
        return f.dst_ip
    if t == "IP::remote_addr":
        return f.dst_ip
    if t == "TCP::client_port":
        return str(f.src_port) if f.src_port else None
    if t == "TCP::local_port":
        return str(f.dst_port) if f.dst_port else None
    if t == "TCP::remote_port":
        return str(f.dst_port) if f.dst_port else None
    # LB::server (only meaningful on a paired :np capture)
    if t == "LB::server" or t == "LB::server addr":
        return None  # filled by the caller from session.back if any
    return None


# Regex that finds iRule command invocations inside [ ... ] brackets.
# We match the leading namespace (HTTP::, SSL::, IP::, TCP::, LB::) plus
# the rest of the command up to ``]`` or whitespace.  Some commands take
# an argument (e.g. ``[HTTP::header User-Agent]``); we capture up to the
# closing bracket.
_HUD_TOKEN_RE = re.compile(r"\[\s*(HTTP::|SSL::|IP::|TCP::|LB::|SNI\b)([^\]\n]*?)\s*\]")


def _annotate_event_block(body: str, conn: Connection) -> list[tuple[str, str, str]]:
    """Return ``[(line_excerpt, command, captured_value), ...]`` for *body*.

    Walks *body* line-by-line, finds every ``[NAMESPACE::cmd ...]``
    token, looks up the captured value for that command on *conn*, and
    emits one tuple per line that referenced state we have.  Lines that
    reference commands but where we don't have captured state (e.g.
    ``[HTTP::host]`` on a TCP-only flow) are silently skipped — that
    distinction stays visible because the line still appears verbatim
    in the event body that's printed alongside the annotations.
    """
    out: list[tuple[str, str, str]] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        for m in _HUD_TOKEN_RE.finditer(line):
            ns = m.group(1)
            rest = m.group(2).strip()
            # The namespace match already includes "::" (e.g. "HTTP::"),
            # except for the special "SNI" token which has no namespace
            # prefix.  Reconstruct the full command name accordingly.
            full = "SNI" if ns == "SNI" else ns + rest
            value = _hud_value_for(full, conn)
            if value is None:
                continue
            out.append((stripped, full, value))
            # Only one annotation per line keeps the output tidy.
            break
    return out


def _expected_event_sequence(
    cfg: BigipConfig,
    vs: BigipVirtualServer,
    conn: Connection,
    rule_events: set[str],
) -> list[str]:
    """Pick the events from *rule_events* that would plausibly fire for *conn*.

    Selection is driven by:

    * the protocol of the connection (TCP / UDP);
    * the profiles attached to the VS (client-ssl ⇒ TLS events fire,
      http profile ⇒ HTTP events fire);
    * the L7 features observed on either side of the connection (TLS
      ClientHello → SSL handshake events; HTTP request line →
      HTTP_REQUEST; HTTP response → HTTP_RESPONSE; RST → CLIENT_CLOSED
      / SERVER_CLOSED).
    """
    profile_types = cfg.profile_types_for_virtual(vs.full_path)
    has_http = ProfileType.HTTP in profile_types
    has_client_ssl = ProfileType.CLIENT_SSL in profile_types
    has_server_ssl = ProfileType.SERVER_SSL in profile_types

    client = conn.client
    server = conn.server
    saw_tls = client.tls_clienthello or has_client_ssl
    saw_http_req = client.http_request_seen or (has_http and client.proto == 6)
    saw_http_resp = (
        server is not None and (server.http_response_seen or server.http_response_code)
    ) or client.http_response_seen
    saw_close = (
        client.tcp_fin
        or client.tcp_rst
        or (server is not None and (server.tcp_fin or server.tcp_rst))
    )

    candidate: list[str] = ["RULE_INIT", "CLIENT_ACCEPTED"]
    if saw_tls:
        candidate += ["CLIENTSSL_CLIENTHELLO", "CLIENTSSL_HANDSHAKE"]
    if saw_http_req:
        candidate += ["HTTP_REQUEST", "HTTP_REQUEST_DATA"]
    candidate += ["LB_SELECTED", "SERVER_CONNECTED"]
    if has_server_ssl:
        candidate.append("SERVERSSL_HANDSHAKE")
    if saw_http_req:
        candidate += ["HTTP_REQUEST_SEND"]
    if saw_http_resp:
        candidate += ["HTTP_RESPONSE", "HTTP_RESPONSE_DATA"]
    if saw_close:
        candidate += ["CLIENT_CLOSED", "SERVER_CLOSED"]

    selected = [ev for ev in candidate if ev in rule_events]
    extra = sorted(rule_events - set(selected) - set(_EVENT_ORDER))
    return selected + extra


def _ltm_policies_for(cfg: BigipConfig, vs: BigipVirtualServer) -> list[str]:
    """Return ``ltm policy`` paths attached to *vs* via its ``policies { … }`` block.

    Listed in attach order; resolved against the parsed generic_objects
    so the operator sees the canonical full path even when the VS
    referenced a short name.  Policies that don't appear in
    ``vs.policies`` are not returned — listing every policy in the
    config would mislead per-session troubleshooting.
    """
    if not vs.policies:
        return []
    out: list[str] = []
    for ref in vs.policies:
        resolved = cfg.resolve_generic_object(ref, module="ltm", object_types=("policy",))
        if resolved is not None:
            obj = cfg.generic_objects[resolved]
            out.append(obj.identifier or resolved)
        else:
            out.append(f"{ref} (unresolved)")
    return out


def _evaluate_attached_policies(
    cfg: BigipConfig, attached_paths: list[str], session: Session
) -> list[PolicyDecision]:
    """Evaluate every parsed LTM policy attached to the matched VS.

    Skips policies whose body wasn't parsed (e.g. attached by short
    name we couldn't resolve, or a policy stanza absent from the
    config slice we were handed).  The same evaluator runs under
    static analysis and ``simulate=true`` — LTM policies are
    declarative, so the captured state fully determines the outcome.
    """
    if not attached_paths:
        return []
    state = request_state_from_session(session)
    out: list[PolicyDecision] = []
    for path in attached_paths:
        if path.endswith("(unresolved)"):
            continue
        policy = cfg.policies.get(path)
        if policy is None:
            continue
        out.append(evaluate_policy(policy, state))
    return out


def _gtm_wide_ips_in_config(cfg: BigipConfig) -> list[str]:
    """Return every GTM wide-IP identifier in *cfg* (a global inventory).

    This is intentionally **not** filtered against the matched virtual
    server: :class:`BigipGenericObject` retains only the stanza
    *header* (module / object_type / identifier), not its body, so we
    can't currently tell from the parsed config which wide-IP resolves
    to which VS.  Callers should treat the returned list as "GTM
    wide-IPs that exist in this config; one of them might point at the
    matched VS — confirm by reading the source".  If we ever extend
    the parser to retain wide-IP bodies, this helper should grow a
    ``vs`` parameter and filter appropriately.
    """
    out: list[str] = []
    for key, obj in cfg.generic_objects.items():
        if obj.module != "gtm":
            continue
        if (
            obj.object_type.startswith("wideip")
            or obj.object_type == "wideip-a"
            or "wideip" in obj.object_type
        ):
            out.append(obj.identifier or key)
    return out


def _apm_profile_for(cfg: BigipConfig, vs: BigipVirtualServer) -> str:
    for pref in vs.profiles.paths:
        resolved = cfg.resolve_profile(pref) or pref
        if "/access" in resolved or resolved.endswith("access"):
            return resolved
        # generic apm profile
        for key, obj in cfg.generic_objects.items():
            if obj.module == "apm" and (
                resolved == obj.identifier or resolved.endswith(obj.identifier)
            ):
                return resolved
    return ""


# ---------------------------------------------------------------------------
# Top-level driver.
# ---------------------------------------------------------------------------


def _analyse_reset(session: Session) -> str:
    """Produce a human-readable narrative of why the session ended."""
    parts: list[str] = []
    front = session.front
    back = session.back
    causes = session.reset_causes()

    if front.client.tcp_rst:
        parts.append(
            f"client→VIP RST after {front.client.tcp_rst_after_bytes} bytes "
            f"({front.client.tcp_rst_count}x)"
        )
    if front.server is not None and front.server.tcp_rst:
        parts.append(
            f"VIP→client RST after {front.server.tcp_rst_after_bytes} bytes "
            f"({front.server.tcp_rst_count}x)"
        )
    if back is not None:
        if back.client.tcp_rst:
            parts.append(
                f"TMM→server RST after {back.client.tcp_rst_after_bytes} bytes "
                f"({back.client.tcp_rst_count}x)"
            )
        if back.server is not None and back.server.tcp_rst:
            parts.append(
                f"server→TMM RST after {back.server.tcp_rst_after_bytes} bytes "
                f"({back.server.tcp_rst_count}x)"
            )

    if not parts:
        # No RST seen — describe FIN-based teardown if any.
        if front.client.tcp_fin or (front.server is not None and front.server.tcp_fin):
            return "graceful FIN teardown (no RST)"
        return "no termination observed in capture"

    if causes:
        parts.append("F5 reset cause(s): " + " | ".join(causes))
    else:
        parts.append("no F5 reset cause string in trailer (LOW/MED TLV absent or opaque)")

    if front.client.tls_alert_seen:
        parts.append(f"client TLS alert: {front.client.tls_alert_desc}")
    if front.server is not None and front.server.tls_alert_seen:
        parts.append(f"server TLS alert: {front.server.tls_alert_desc}")

    return " ; ".join(parts)


# A simulator runs the matched VS's iRule against a captured session and
# returns the dynamic outcome dict (pool/node/response_committed/logs/
# decisions/error).  Injected by the CLI / MCP adapters so this module
# never imports the iRule-test framework directly — see
# tooling.f5.irule_simulation.simulate_irule_for_session.
IruleSimulator = Callable[["BigipConfig", "BigipVirtualServer", "Session"], dict]


def compute_explain_flow(
    pcap_path: Path,
    configs: dict[str, BigipConfig],
    *,
    use_tshark: bool = False,
    keylog_path: str = "",
    tshark_filter: str = "",
    show_event_bodies: bool = True,
    max_event_body_lines: int = 40,
    simulator: IruleSimulator | None = None,
) -> ExplainFlowReport:
    """Build a per-session explanation for *pcap_path* against parsed *configs*.

    Flows are paired into bidirectional Connections, which are then
    paired into Sessions via the F5 ethernet trailer's peer-tuple
    (front-side client↔VIP + back-side TMM↔pool-member from the same
    `:np` capture).

    *configs* is the dict produced by
    :func:`tooling.f5.verbs._paths.load_paths`; the first config whose
    virtual-server set matches a session's destination wins.

    *keylog_path* — when set and tshark is available, passed through
    as ``-o tls.keylog_file:<path>`` so HTTPS payloads decrypt and the
    HTTP request/response fields populate for TLS-wrapped sessions.

    *tshark_filter* — when set, the entire flow-extraction pass runs
    through tshark with ``-Y <filter>`` instead of the built-in
    walker.  This means only packets matching the display filter
    contribute to the report.  Implies ``use_tshark=True``.  Note
    that the F5 ethernet trailer peer-tuple (used for ``:np`` capture
    pairing) is only populated by the built-in walker, so a filtered
    tshark-only run loses front/back session pairing.
    """
    used_tshark = False
    if tshark_filter:
        # Filter requested → tshark is the canonical flow source.
        flows = extract_flows_via_tshark(
            pcap_path, keylog_path=keylog_path, tshark_filter=tshark_filter
        )
        used_tshark = bool(flows) or tshark_available()
    else:
        flows = extract_flows(pcap_path)
        if use_tshark and tshark_available():
            used_tshark = enrich_with_tshark(flows, pcap_path, keylog_path=keylog_path)

    sessions = pair_sessions(flows)
    session_explains: list[SessionExplain] = []
    matched = 0

    # Sort sessions: paired (front+back) first, then by packets.
    sessions.sort(
        key=lambda s: (
            0 if s.back is not None else 1,
            -(s.front.client.packets + (s.front.server.packets if s.front.server else 0)),
        )
    )

    for session in sessions:
        front = session.front

        # The front-side client flow's destination is the VIP.
        vs_path: str | None = None
        cfg_hit: BigipConfig | None = None
        for cfg in configs.values():
            vs_path = _match_virtual(cfg, front.client.dst_ip, front.client.dst_port)
            if vs_path is None and front.server is not None:
                # Capture might only have the response side of the front.
                vs_path = _match_virtual(cfg, front.server.src_ip, front.server.src_port)
            if vs_path is not None:
                cfg_hit = cfg
                break

        if vs_path is None or cfg_hit is None:
            session_explains.append(
                SessionExplain(session=session, reset_analysis=_analyse_reset(session))
            )
            continue

        matched += 1
        vs = cfg_hit.virtual_servers[vs_path]
        partition = vs_path.split("/")[1] if vs_path.startswith("/") else ""

        profile_chain: list[str] = []
        for pref in vs.profiles.paths:
            resolved = cfg_hit.resolve_profile(pref) or pref
            prof = cfg_hit.profiles.get(resolved)
            if prof is not None:
                profile_chain.append(f"{resolved} ({prof.profile_type.name.lower()})")
            else:
                profile_chain.append(f"{pref} (unresolved)")

        event_blocks: list[tuple[str, str, str]] = []
        event_annotations: list[tuple[str, str, tuple[tuple[str, str, str], ...]]] = []
        sequence: list[str] = []
        for rref in vs.rules:
            resolved_rule = cfg_hit.resolve_rule(rref) or rref
            rule = cfg_hit.rules.get(resolved_rule)
            if rule is None:
                continue
            blocks = _extract_event_blocks(rule.source)
            ordered_events = _expected_event_sequence(cfg_hit, vs, front, set(blocks.keys()))
            for ev in ordered_events:
                sequence.append(f"{resolved_rule}::{ev}")
                full_body = blocks[ev]
                # Annotations always run on the full body so we don't
                # miss tokens past the truncation cutoff.
                ann = tuple(_annotate_event_block(full_body, front))
                # If LB::server is referenced and we have a paired back side,
                # synthesise the captured value.
                if session.back is not None:
                    bc = session.back.client
                    lb_value = f"{bc.dst_ip}:{bc.dst_port}"
                    extra: list[tuple[str, str, str]] = []
                    for line in full_body.splitlines():
                        if "LB::server" in line and not any(a[1].startswith("LB::") for a in ann):
                            extra.append((line.strip(), "LB::server", lb_value))
                            break
                    if extra:
                        ann = ann + tuple(extra)
                if ann:
                    event_annotations.append((resolved_rule, ev, ann))
                if show_event_bodies:
                    body = full_body
                    body_lines = body.splitlines()
                    if len(body_lines) > max_event_body_lines:
                        body = "\n".join(body_lines[:max_event_body_lines]) + "\n... (truncated)"
                    event_blocks.append((resolved_rule, ev, body))

        vs_report = compute_explain(cfg_hit, vs.full_path, kind="virtual")
        ltm_policies = _ltm_policies_for(cfg_hit, vs)
        policy_decisions = _evaluate_attached_policies(cfg_hit, ltm_policies, session)
        apm = _apm_profile_for(cfg_hit, vs)
        gtm = _gtm_wide_ips_in_config(cfg_hit)

        # Pool selection + SNAT inferred from the back-side flow if present.
        pool_selected = ""
        snat_observed = ""
        if session.back is not None:
            bc = session.back.client
            pool_selected = f"{bc.dst_ip}:{bc.dst_port}"
            if bc.src_ip != front.client.src_ip:
                snat_observed = f"{bc.src_ip}:{bc.src_port}"

        sim_pool = ""
        sim_node = ""
        sim_resp = False
        sim_logs: tuple[str, ...] = ()
        sim_decisions: tuple[tuple[str, str, str], ...] = ()
        sim_error = ""
        if simulator is not None:
            sim = simulator(cfg_hit, vs, session)
            sim_pool = sim["pool"]
            sim_node = sim["node"]
            sim_resp = sim["response_committed"]
            sim_logs = tuple(sim["logs"])
            sim_decisions = tuple(sim["decisions"])
            sim_error = sim["error"]

        session_explains.append(
            SessionExplain(
                session=session,
                matched_vs=vs.full_path,
                matched_partition=partition,
                profile_chain=tuple(profile_chain),
                pool_selected=pool_selected,
                snat_observed=snat_observed,
                event_sequence=tuple(sequence),
                event_blocks=tuple(event_blocks),
                event_annotations=tuple(event_annotations),
                ltm_policies=tuple(ltm_policies),
                policy_decisions=tuple(policy_decisions),
                apm_profile=apm,
                gtm_wide_ips=tuple(gtm),
                explain_text=vs_report.text_report,
                reset_analysis=_analyse_reset(session),
                simulated_pool=sim_pool,
                simulated_node=sim_node,
                simulated_response_committed=sim_resp,
                simulated_logs=sim_logs,
                simulated_decisions=sim_decisions,
                simulation_error=sim_error,
            )
        )

    text = _format_report(pcap_path, session_explains, used_tshark, keylog_path, tshark_filter)
    return ExplainFlowReport(
        pcap_path=str(pcap_path),
        flow_count=len(flows),
        session_count=len(sessions),
        matched_count=matched,
        sessions=tuple(session_explains),
        text_report=text,
        used_tshark=used_tshark,
        keylog_path=keylog_path,
        tshark_filter=tshark_filter,
    )


def _flow_to_dict(f: Flow) -> dict:
    return {
        "src_ip": f.src_ip,
        "src_port": f.src_port,
        "dst_ip": f.dst_ip,
        "dst_port": f.dst_port,
        "proto": f.proto_name,
        "packets": f.packets,
        "bytes": f.bytes_total,
        "tcp_syn": f.tcp_syn,
        "tcp_synack": f.tcp_synack,
        "tcp_fin": f.tcp_fin,
        "tcp_rst": f.tcp_rst,
        "tcp_rst_count": f.tcp_rst_count,
        "tcp_rst_after_bytes": f.tcp_rst_after_bytes,
        "tls_clienthello": f.tls_clienthello,
        "tls_sni": f.tls_sni,
        "tls_version": f.tls_version,
        "tls_chosen_version": f.tls_chosen_version,
        "tls_chosen_cipher": f.tls_chosen_cipher,
        "tls_alpn": f.tls_alpn,
        "tls_alert_seen": f.tls_alert_seen,
        "tls_alert_desc": f.tls_alert_desc,
        "http_method": f.http_method,
        "http_host": f.http_host,
        "http_uri": f.http_uri,
        "http_response_seen": f.http_response_seen,
        "http_path": f.http_path,
        "http_query": f.http_query,
        "http_request_version": f.http_request_version,
        "http_user_agent": f.http_user_agent,
        "http_cookie": f.http_cookie,
        "http_referer": f.http_referer,
        "http_request_content_type": f.http_request_content_type,
        "http_request_content_length": f.http_request_content_length,
        "http_request_headers": dict(f.http_request_headers),
        "http_response_code": f.http_response_code,
        "http_response_phrase": f.http_response_phrase,
        "http_response_content_type": f.http_response_content_type,
        "http_response_content_length": f.http_response_content_length,
        "http_response_headers": dict(f.http_response_headers),
        "tls_cert_subject": f.tls_cert_subject,
        "tls_cert_issuer": f.tls_cert_issuer,
        "tls_supported_groups": f.tls_supported_groups,
        "tls_signature_algos": f.tls_signature_algos,
        "f5_reset_causes": list(f.f5_reset_causes),
        "peer_remote_ip": f.peer_remote_ip,
        "peer_remote_port": f.peer_remote_port,
        "peer_local_ip": f.peer_local_ip,
        "peer_local_port": f.peer_local_port,
    }


def _conn_to_dict(c: Connection | None) -> dict | None:
    if c is None:
        return None
    return {
        "client": _flow_to_dict(c.client),
        "server": _flow_to_dict(c.server) if c.server is not None else None,
    }


def _format_report(
    pcap_path: Path,
    sessions: list[SessionExplain],
    used_tshark: bool,
    keylog_path: str,
    tshark_filter: str = "",
) -> str:
    lines: list[str] = []
    lines.append(f"explain-flow: {pcap_path}")
    lines.append(
        f"  sessions: {len(sessions)} | "
        f"matched: {sum(1 for s in sessions if s.matched_vs)} | "
        f"tshark: {'yes' if used_tshark else 'no'}"
        f"{f' | keylog: {keylog_path}' if keylog_path else ''}"
        f"{f' | filter: {tshark_filter!r}' if tshark_filter else ''}"
    )
    lines.append("")
    for i, se in enumerate(sessions, 1):
        s = se.session
        lines.append(f"[session {i}] {s.proto_name}")
        lines.append(f"  front: {s.front.summary()}")
        if s.back is not None:
            lines.append(f"  back:  {s.back.summary()}")
        if not se.matched_vs:
            lines.append("  (no virtual server matched this destination)")
            if se.reset_analysis:
                lines.append(f"  termination: {se.reset_analysis}")
            lines.append("")
            continue
        lines.append(f"  matched virtual: {se.matched_vs}")
        if se.pool_selected:
            lines.append(f"  pool member chosen (observed): {se.pool_selected}")
        if se.snat_observed:
            lines.append(f"  SNAT applied (observed): {se.snat_observed}")
        if se.profile_chain:
            lines.append("  profiles (in attach order):")
            for p in se.profile_chain:
                lines.append(f"    - {p}")
        if se.ltm_policies:
            lines.append("  ltm policies:")
            for p in se.ltm_policies:
                lines.append(f"    - {p}")
        if se.policy_decisions:
            lines.append("  ltm policy decisions:")
            for pd in se.policy_decisions:
                lines.append(f"    --- {pd.policy} (strategy: {pd.strategy}) ---")
                for rd in pd.rules:
                    state = "FIRED" if rd.fired else ("matched" if rd.matched else "no-match")
                    lines.append(f"      rule {rd.rule!r} (ord {rd.ordinal}): {state}")
                    for ct in rd.conditions:
                        target = ct.operand
                        if ct.name:
                            target += f"[{ct.name}]"
                        elif ct.selector:
                            target += f".{ct.selector}"
                        prefix = "      " + ("- " if ct.matched else "  ")
                        if ct.note:
                            lines.append(
                                f"{prefix}{target} {ct.operator} {list(ct.expected)}"
                                f" (skipped: {ct.note})"
                            )
                        else:
                            lines.append(
                                f"{prefix}{target} {ct.operator} {list(ct.expected)}"
                                f" -> actual={ct.actual!r} match={ct.matched}"
                            )
                    for at in rd.actions:
                        lines.append(f"        action: {at.target} {at.verb} {at.value}".rstrip())
        if se.apm_profile:
            lines.append(f"  apm: {se.apm_profile}")
        if se.gtm_wide_ips:
            lines.append(
                "  gtm wide-ips in config (global inventory; may or may not point at this VS):"
            )
            for w in se.gtm_wide_ips:
                lines.append(f"    - {w}")
        if se.event_sequence:
            lines.append("  expected iRule event firing order:")
            for ev in se.event_sequence:
                lines.append(f"    -> {ev}")
        if se.event_blocks:
            lines.append("  iRule event bodies (path through):")
            for rule, ev, body in se.event_blocks:
                lines.append(f"    --- {rule} :: when {ev} ---")
                for body_line in body.splitlines():
                    lines.append(f"      {body_line}")
        if se.event_annotations:
            lines.append("  HUD-state captured for iRule commands:")
            for rule, ev, anns in se.event_annotations:
                lines.append(f"    --- {rule} :: when {ev} ---")
                for line_excerpt, cmd, value in anns:
                    lines.append(f"      [{cmd}] = {value!r}")
                    lines.append(f"          ({line_excerpt})")
        # Captured HTTP exchange details (front side of the session).
        fc = se.session.front.client
        if fc.http_request_seen or fc.tls_clienthello:
            lines.append("  captured request:")
            if fc.http_method or fc.http_uri:
                req_line = f"{fc.http_method or '?'} {fc.http_uri or ''} {fc.http_request_version or ''}".strip()
                lines.append(f"    request line: {req_line}")
            if fc.http_host:
                lines.append(f"    host: {fc.http_host}")
            if fc.http_user_agent:
                lines.append(f"    user-agent: {fc.http_user_agent}")
            if fc.http_referer:
                lines.append(f"    referer: {fc.http_referer}")
            if fc.http_cookie:
                lines.append(f"    cookie: {fc.http_cookie}")
            if fc.tls_sni:
                tls_line = f"    tls: SNI={fc.tls_sni}"
                if fc.tls_chosen_version or fc.tls_version:
                    tls_line += f" version={fc.tls_chosen_version or fc.tls_version}"
                if fc.tls_chosen_cipher:
                    tls_line += f" cipher={fc.tls_chosen_cipher}"
                if fc.tls_alpn:
                    tls_line += f" alpn={fc.tls_alpn}"
                lines.append(tls_line)
            if fc.tls_cert_subject:
                lines.append(f"    server cert subject: {fc.tls_cert_subject}")
        # Captured response (server-side flow).
        sc = se.session.front.server
        if sc is not None and (sc.http_response_seen or sc.http_response_code):
            lines.append("  captured response:")
            status = f"{sc.http_response_code} {sc.http_response_phrase}".strip() or "(none)"
            lines.append(f"    status: {status}")
            if sc.http_response_content_type:
                lines.append(f"    content-type: {sc.http_response_content_type}")
            if sc.http_response_content_length:
                lines.append(f"    content-length: {sc.http_response_content_length}")
        if se.simulation_error:
            lines.append(f"  simulation skipped: {se.simulation_error}")
        elif (
            se.simulated_pool
            or se.simulated_decisions
            or se.simulated_logs
            or se.simulated_response_committed
        ):
            lines.append("  iRule simulation outcome (run under c-tcl orchestrator):")
            if se.simulated_pool:
                lines.append(f"    pool selected: {se.simulated_pool}")
            if se.simulated_node:
                lines.append(f"    node selected: {se.simulated_node}")
            if se.simulated_response_committed:
                lines.append("    HTTP response committed by iRule (HTTP::respond / drop)")
            if se.simulated_decisions:
                lines.append("    decisions:")
                for cat, act, val in se.simulated_decisions:
                    lines.append(f"      - {cat} {act} {val}".rstrip())
            if se.simulated_logs:
                lines.append("    log lines:")
                for entry in se.simulated_logs:
                    lines.append(f"      {entry}")
        lines.append(f"  termination: {se.reset_analysis}")
        lines.append("  resolved plan:")
        for explain_line in se.explain_text.splitlines():
            lines.append(f"    {explain_line}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def report_to_dict(report: ExplainFlowReport) -> dict:
    return {
        "pcap_path": report.pcap_path,
        "flow_count": report.flow_count,
        "session_count": report.session_count,
        "matched_count": report.matched_count,
        "used_tshark": report.used_tshark,
        "keylog_path": report.keylog_path,
        "tshark_filter": report.tshark_filter,
        "sessions": [
            {
                "session": {
                    "front": _conn_to_dict(se.session.front),
                    "back": _conn_to_dict(se.session.back),
                },
                "matched_vs": se.matched_vs,
                "partition": se.matched_partition,
                "profile_chain": list(se.profile_chain),
                "pool_selected": se.pool_selected,
                "snat_observed": se.snat_observed,
                "event_sequence": list(se.event_sequence),
                "event_blocks": [{"rule": r, "event": e, "body": b} for r, e, b in se.event_blocks],
                "event_annotations": [
                    {
                        "rule": r,
                        "event": e,
                        "annotations": [
                            {"line": ln, "command": cmd, "value": val} for ln, cmd, val in anns
                        ],
                    }
                    for r, e, anns in se.event_annotations
                ],
                "ltm_policies": list(se.ltm_policies),
                "policy_decisions": [_policy_decision_to_dict(pd) for pd in se.policy_decisions],
                "apm_profile": se.apm_profile,
                "gtm_wide_ips": list(se.gtm_wide_ips),
                "explain_text": se.explain_text,
                "reset_analysis": se.reset_analysis,
                "simulated_pool": se.simulated_pool,
                "simulated_node": se.simulated_node,
                "simulated_response_committed": se.simulated_response_committed,
                "simulated_logs": list(se.simulated_logs),
                "simulated_decisions": [
                    {"category": c, "action": a, "value": v} for c, a, v in se.simulated_decisions
                ],
                "simulation_error": se.simulation_error,
            }
            for se in report.sessions
        ],
    }


def _policy_decision_to_dict(pd: PolicyDecision) -> dict:
    """Verbose dict shape for ``report_to_dict`` — every condition / action."""
    return {
        "policy": pd.policy,
        "strategy": pd.strategy,
        "rules": [
            {
                "rule": rd.rule,
                "ordinal": rd.ordinal,
                "matched": rd.matched,
                "fired": rd.fired,
                "conditions": [
                    {
                        "operand": ct.operand,
                        "selector": ct.selector,
                        "operator": ct.operator,
                        "expected": list(ct.expected),
                        "actual": ct.actual,
                        "matched": ct.matched,
                        "name": ct.name,
                        "negate": ct.negate,
                        "event": ct.event,
                        "note": ct.note,
                    }
                    for ct in rd.conditions
                ],
                "actions": [
                    {"target": at.target, "verb": at.verb, "value": at.value} for at in rd.actions
                ],
            }
            for rd in pd.rules
        ],
    }


def _compact_policy_decision(pd: PolicyDecision) -> dict:
    """Compact MCP shape — only fired rules with their action summaries.

    The full per-rule trace is reachable via the verbose ``report_to_dict``
    when needed; for LLM context we surface only the rule that won and
    the action it produced — that's the answer to "what did the policy
    do?".  If no rule fired, we still include the policy with an empty
    ``fired`` list so the caller knows it was evaluated.
    """
    fired_entries: list[dict] = []
    for rd in pd.rules:
        if not rd.fired:
            continue
        entry: dict = {"rule": rd.rule, "ordinal": rd.ordinal}
        # Compact condition trace: only conditions that actually
        # matched.  For an unconditional default rule (zero
        # conditions) ``matched_on`` is omitted entirely — there's
        # nothing to attribute the firing to.  Unevaluable conditions
        # (note set, matched=False) and non-matching conditions are
        # also omitted; the verbose ``report_to_dict`` shape carries
        # the full per-condition trace if a consumer needs it.
        cond_trace: list[dict] = []
        for ct in rd.conditions:
            if not ct.matched:
                continue
            target = ct.operand
            if ct.name:
                target += f"[{ct.name}]"
            elif ct.selector:
                target += f".{ct.selector}"
            cond_trace.append(
                {
                    "field": target,
                    "operator": ct.operator,
                    "expected": list(ct.expected),
                    "actual": ct.actual,
                }
            )
        if cond_trace:
            entry["matched_on"] = cond_trace
        if rd.actions:
            entry["actions"] = [
                {"target": at.target, "verb": at.verb, "value": at.value} for at in rd.actions
            ]
        fired_entries.append(entry)
    out: dict = {"policy": pd.policy, "strategy": pd.strategy}
    if fired_entries:
        out["fired"] = fired_entries
    else:
        # Surface the no-match case so the caller sees we did evaluate.
        out["fired"] = []
    return out


def _profile_categories(profile_chain: tuple[str, ...]) -> list[str]:
    """Return just the profile *types* (TCP, HTTP, …) in attach order.

    Drops the per-profile path so an LLM sees the categories that
    actually matter for behaviour explanation rather than the full
    inventory of named profile objects.  Each entry looks like
    ``"http (lab_http)"`` — type first, name in parens.
    """
    out: list[str] = []
    for entry in profile_chain:
        # entry is like "/Common/lab_http (http)" — flip to "http (lab_http)".
        if "(" in entry and entry.endswith(")"):
            head, _, tail = entry.partition(" (")
            kind = tail.rstrip(")")
            name = head.rsplit("/", 1)[-1]
            out.append(f"{kind} ({name})")
        else:
            out.append(entry)
    return out


def _compact_session_summary(se: SessionExplain) -> str:
    """Build a one-line operator-style narrative of the matched session.

    Aimed at an LLM that needs the gist before it dives into details:
    who → who, via which VS, what the iRule did, what reset (if any).
    """
    s = se.session
    fc = s.front.client
    parts = [f"{fc.src_ip}:{fc.src_port} → {se.matched_vs or '(no VS)'}"]
    if fc.tls_sni:
        parts.append(f"SNI={fc.tls_sni}")
    if fc.http_method or fc.http_uri:
        parts.append(
            f"{fc.http_method or '?'} "
            f"{fc.http_host + fc.http_uri if fc.http_host and fc.http_uri.startswith('/') else fc.http_uri}"
        )
    fs = s.front.server
    if fs is not None and fs.http_response_code:
        parts.append(f"→ {fs.http_response_code}")
    if se.pool_selected:
        parts.append(f"pool→ {se.pool_selected}")
    if se.snat_observed:
        parts.append(f"snat→ {se.snat_observed}")
    if se.simulated_response_committed:
        parts.append("iRule HTTP::respond")
    if se.simulated_pool and se.simulated_pool != se.pool_selected:
        parts.append(f"iRule pool→ {se.simulated_pool}")
    if s.reset_side():
        parts.append(f"RST({s.reset_side()})")
    return " | ".join(parts)


def _compact_session(se: SessionExplain, *, max_event_body_lines: int = 8) -> dict:
    """Compact, LLM-friendly payload for one session.

    Drops:
    * full per-flow dicts (preserved fields are the high-signal ones —
      5-tuple, observed L7 features, RST counts, F5 reset cause);
    * profile chain in full path form (kept as ``type (name)`` only);
    * GTM wide-IP inventory (it's a global config view, not per-session
      — emit at the report level only);
    * empty/blank fields are omitted entirely so the LLM doesn't waste
      tokens on noise.
    """
    s = se.session
    fc = s.front.client
    fs = s.front.server
    bc = s.back.client if s.back else None

    captured_request: dict = {}
    if fc.http_method:
        captured_request["method"] = fc.http_method
    if fc.http_host:
        captured_request["host"] = fc.http_host
    if fc.http_uri:
        captured_request["uri"] = fc.http_uri
    if fc.http_user_agent:
        captured_request["user_agent"] = fc.http_user_agent
    if fc.tls_sni:
        captured_request["tls_sni"] = fc.tls_sni
    if fc.tls_chosen_version or fc.tls_version:
        captured_request["tls_version"] = fc.tls_chosen_version or fc.tls_version
    if fc.tls_chosen_cipher:
        captured_request["tls_cipher"] = fc.tls_chosen_cipher
    if fc.tls_alpn:
        captured_request["tls_alpn"] = fc.tls_alpn

    captured_response: dict = {}
    if fs is not None:
        if fs.http_response_code:
            captured_response["status"] = fs.http_response_code
        if fs.http_response_phrase:
            captured_response["phrase"] = fs.http_response_phrase
        if fs.tls_alert_desc:
            captured_response["tls_alert"] = fs.tls_alert_desc

    flow_5tuple: dict = {
        "client": f"{fc.src_ip}:{fc.src_port}",
        "vip": f"{fc.dst_ip}:{fc.dst_port}",
        "proto": fc.proto_name,
    }
    if bc is not None:
        flow_5tuple["pool_member"] = f"{bc.dst_ip}:{bc.dst_port}"
        if bc.src_ip != fc.src_ip:
            flow_5tuple["snat"] = f"{bc.src_ip}:{bc.src_port}"

    # iRule decisions: just (event, command, value), de-duplicated and
    # truncated.  The full source line is dropped — the LLM has the
    # event body (truncated below) for context already.
    decisions: list[dict] = []
    seen: set[tuple[str, str, str]] = set()
    for _rule, event, anns in se.event_annotations:
        for _line, cmd, value in anns:
            sig = (event, cmd, value)
            if sig in seen:
                continue
            seen.add(sig)
            decisions.append({"event": event, "command": cmd, "value": value})

    # iRule event bodies trimmed hard for LLM context — the full
    # bodies are available via the verbose report_to_dict if the
    # caller really needs them.
    bodies: list[dict] = []
    for rule, event, body in se.event_blocks:
        body_lines = body.splitlines()
        if len(body_lines) > max_event_body_lines:
            body = "\n".join(body_lines[:max_event_body_lines]) + "\n... (truncated)"
        bodies.append(
            {
                "rule": rule.rsplit("/", 1)[-1],  # short name
                "event": event,
                "body": body,
            }
        )

    out: dict = {
        "summary": _compact_session_summary(se),
        "matched_vs": se.matched_vs or None,
        "flow": flow_5tuple,
    }
    if captured_request:
        out["captured_request"] = captured_request
    if captured_response:
        out["captured_response"] = captured_response
    profiles = _profile_categories(se.profile_chain)
    if profiles:
        out["profiles"] = profiles
    if se.ltm_policies:
        out["ltm_policies"] = list(se.ltm_policies)
    if se.policy_decisions:
        out["policy_decisions"] = [_compact_policy_decision(pd) for pd in se.policy_decisions]
    if se.apm_profile:
        out["apm_profile"] = se.apm_profile
    if se.event_sequence:
        # Strip the rule path prefix so the LLM sees just event names
        # in firing order; rule attribution is still in `decisions`.
        out["events_fired"] = [s.split("::")[-1] for s in se.event_sequence]
    if decisions:
        out["irule_decisions"] = decisions
    if bodies:
        out["irule_bodies"] = bodies
    if se.reset_analysis and se.reset_analysis != "no termination observed in capture":
        out["termination"] = se.reset_analysis
    if se.simulated_pool or se.simulated_response_committed or se.simulation_error:
        sim: dict = {}
        if se.simulated_pool:
            sim["pool"] = se.simulated_pool
        if se.simulated_node:
            sim["node"] = se.simulated_node
        if se.simulated_response_committed:
            sim["response_committed"] = True
        if se.simulated_decisions:
            sim["decisions"] = [
                {"category": c, "action": a, "value": v} for c, a, v in se.simulated_decisions
            ]
        if se.simulation_error:
            sim["error"] = se.simulation_error
        out["simulated"] = sim
    return out


def report_to_mcp_dict(report: ExplainFlowReport, *, max_event_body_lines: int = 8) -> dict:
    """Compact, LLM-friendly version of :func:`report_to_dict`.

    Aggressively prunes context-bloat so an LLM consuming the
    ``explain_flow`` MCP tool sees high-signal fields only:

    * one-line ``summary`` per session;
    * 5-tuple as a ``"ip:port"`` string instead of a 30-key Flow dict;
    * profiles listed as ``type (short_name)`` rather than full paths;
    * ``irule_decisions`` is the de-duped list of every captured
      ``[CMD] = value`` annotation across events — the actual
      branching state the iRule looked at;
    * iRule event bodies truncated to *max_event_body_lines* (default 8);
    * empty / blank fields omitted entirely;
    * GTM wide-IP inventory and the static ``explain_text`` block
      from `f5 explain virtual` are emitted *once* at the top level
      rather than per session;
    * full pcap + per-flow data is still reachable via
      :func:`report_to_dict` for callers who want everything.

    The verbose ``report_to_dict`` shape stays available unchanged
    for non-LLM consumers (CLI ``--json``, programmatic tests).
    """
    sessions = [
        _compact_session(se, max_event_body_lines=max_event_body_lines) for se in report.sessions
    ]

    # GTM wide-IP inventory: dedupe across sessions and emit once.
    gtm: list[str] = []
    seen_gtm: set[str] = set()
    for se in report.sessions:
        for w in se.gtm_wide_ips:
            if w not in seen_gtm:
                seen_gtm.add(w)
                gtm.append(w)

    out: dict = {
        "pcap": report.pcap_path,
        "matched": report.matched_count,
        "sessions_total": report.session_count,
        "tshark": report.used_tshark,
        "sessions": sessions,
    }
    if report.keylog_path:
        out["keylog"] = report.keylog_path
    if report.tshark_filter:
        out["tshark_filter"] = report.tshark_filter
    if gtm:
        out["gtm_wide_ips_in_config"] = gtm
    return out
