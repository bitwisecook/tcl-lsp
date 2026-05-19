"""Tests for ``f5 explain-flow`` and the underlying flow tracer."""

from __future__ import annotations

import io
import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.explain_flow import (
    _extract_event_blocks,
    _extract_peer_tuple_from_trailer,
    _parse_destination,
    compute_explain_flow,
    extract_flows,
    pair_sessions,
    report_to_mcp_dict,
)
from dialects.f5.bigip.parser import parse_bigip_conf
from tooling.f5.main import main

# ---------------------------------------------------------------------------
# Tiny libpcap builder (mirrors test_f5_pcap_remap helpers).
# ---------------------------------------------------------------------------


def _ipv4_checksum(header: bytes) -> int:
    total = 0
    for i in range(0, len(header), 2):
        total += (header[i] << 8) | header[i + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return (~total) & 0xFFFF


def _build_packet(
    src: bytes,
    dst: bytes,
    sp: int,
    dp: int,
    *,
    syn: bool = True,
    rst: bool = False,
    payload: bytes = b"",
    trailer: bytes = b"",
) -> bytes:
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    if rst:
        flags = 0x04  # RST
    elif syn:
        flags = 0x02  # SYN
    else:
        flags = 0x18  # PSH+ACK
    tcp_hdr = (
        struct.pack(">HH", sp, dp)
        + struct.pack(">II", 1, 0)
        + bytes([0x50, flags])
        + struct.pack(">H", 0xFFFF)
        + struct.pack(">HH", 0, 0)
    )
    tcp = tcp_hdr + payload
    ip_total = 20 + len(tcp)
    ip = bytearray(
        bytes([0x45, 0x00])
        + struct.pack(">H", ip_total)
        + bytes([0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00])
        + src
        + dst
    )
    cksum = _ipv4_checksum(bytes(ip))
    ip[10] = (cksum >> 8) & 0xFF
    ip[11] = cksum & 0xFF
    return eth + bytes(ip) + tcp + trailer


def _legacy_high_trailer(
    remote_v4: bytes, local_v4: bytes, remote_port: int, local_port: int
) -> bytes:
    """Build a legacy F5 HIGH v0 trailer carrying a peer 5-tuple."""
    ipv4_mapped_remote = bytes(10) + b"\xff\xff" + remote_v4
    ipv4_mapped_local = bytes(10) + b"\xff\xff" + local_v4
    body = (
        bytes([0x06])  # ipproto = TCP
        + struct.pack(">H", 0)  # vlan
        + ipv4_mapped_remote
        + ipv4_mapped_local
        + struct.pack(">H", remote_port)
        + struct.pack(">H", local_port)
    )
    return bytes([3, 40, 0]) + body  # type=HIGH, wire-len=40 (total=42), ver=0


def _build_pcap(packets: list[bytes]) -> bytes:
    out = io.BytesIO()
    out.write(struct.pack("<IHHIIII", 0xA1B2C3D4, 2, 4, 0, 0, 65535, 1))
    for packet in packets:
        out.write(struct.pack("<IIII", 1000000, 0, len(packet), len(packet)))
        out.write(packet)
    return out.getvalue()


# ---------------------------------------------------------------------------
# Unit tests for the helpers.
# ---------------------------------------------------------------------------


def test_parse_destination_ipv4():
    assert _parse_destination("/Common/10.0.0.1:443") == ("10.0.0.1", 443)
    assert _parse_destination("/p/10.0.0.1%0:any") == ("10.0.0.1", 0)


def test_parse_destination_ipv6():
    assert _parse_destination("/Common/[::1]:80") == ("::1", 80)


def test_extract_event_blocks_balanced_braces():
    src = """
when CLIENT_ACCEPTED {
    if { [TCP::local_port] == 443 } {
        log "443"
    }
}
when HTTP_REQUEST {
    HTTP::header insert X-Foo bar
}
"""
    blocks = _extract_event_blocks(src)
    assert "CLIENT_ACCEPTED" in blocks
    assert "HTTP_REQUEST" in blocks
    assert "TCP::local_port" in blocks["CLIENT_ACCEPTED"]
    assert "HTTP::header" in blocks["HTTP_REQUEST"]


def test_extract_flows_picks_up_syn_packet(tmp_path):
    pcap = _build_pcap(
        [
            _build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True),
            _build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=False),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    flows = extract_flows(p)
    assert len(flows) == 1
    flow = next(iter(flows.values()))
    assert flow.dst_ip == "5.6.7.8"
    assert flow.dst_port == 443
    assert flow.tcp_syn is True
    assert flow.packets == 2


# ---------------------------------------------------------------------------
# End-to-end against a synthetic config + pcap.
# ---------------------------------------------------------------------------


_CONF = """\
ltm virtual /Common/vs_app {
    destination /Common/5.6.7.8:443
    pool /Common/pool_app
    profiles {
        /Common/tcp { }
        /Common/clientssl { }
        /Common/http { }
    }
    rules {
        /Common/rule_app
    }
}
ltm pool /Common/pool_app {
    members {
        /Common/10.0.0.1:8080 { }
    }
}
ltm profile tcp /Common/tcp { }
ltm profile client-ssl /Common/clientssl { }
ltm profile http /Common/http { }
ltm rule /Common/rule_app {
when CLIENT_ACCEPTED {
    log local0. "client connected"
}
when HTTP_REQUEST {
    HTTP::header insert X-Forwarded-For [IP::client_addr]
}
when SERVER_CONNECTED {
    log local0. "server connected"
}
}
"""


def test_compute_explain_flow_matches_vs_and_orders_events(tmp_path):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    cfg = parse_bigip_conf(_CONF)

    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_flow(p, {"u://x": cfg})
    assert report.flow_count == 1
    assert report.matched_count == 1
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_app"
    assert any("clientssl" in line for line in se.profile_chain)
    seq_events = [s.split("::")[-1] for s in se.event_sequence]
    assert "CLIENT_ACCEPTED" in seq_events
    assert "HTTP_REQUEST" in seq_events
    assert "SERVER_CONNECTED" in seq_events
    assert seq_events.index("CLIENT_ACCEPTED") < seq_events.index("HTTP_REQUEST")
    assert seq_events.index("HTTP_REQUEST") < seq_events.index("SERVER_CONNECTED")
    assert any("X-Forwarded-For" in body for _, _, body in se.event_blocks)


def test_explain_flow_cli_text(tmp_path, capsys):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-flow", str(pcap_path), str(conf_path)])
    out = capsys.readouterr().out
    assert code == 0
    assert "/Common/vs_app" in out
    assert "CLIENT_ACCEPTED" in out
    assert "HTTP_REQUEST" in out


def test_explain_flow_cli_no_match_returns_1(tmp_path, capsys):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    # Destination 9.9.9.9 has no matching VS in the config.
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x09\x09\x09\x09", 12345, 80, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-flow", str(pcap_path), str(conf_path)])
    assert code == 1
    out = capsys.readouterr().out
    assert "no virtual server matched" in out


def test_explain_flow_cli_json(tmp_path, capsys):
    import json as _json

    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-flow", "--json", str(pcap_path), str(conf_path)])
    out = capsys.readouterr().out
    assert code == 0
    data = _json.loads(out)
    assert data["matched_count"] == 1
    assert data["sessions"][0]["matched_vs"] == "/Common/vs_app"


_CONF_WITH_POLICY = """\
ltm virtual /Common/vs_policy {
    destination /Common/5.6.7.8:80
    pool /Common/pool_default
    profiles {
        /Common/tcp { }
        /Common/http { }
    }
    policies {
        /Common/policy_api_routing
    }
}
ltm pool /Common/pool_default {
    members {
        /Common/10.0.0.1:80 { }
    }
}
ltm pool /Common/pool_api {
    members {
        /Common/10.0.0.20:8080 { }
    }
}
ltm profile tcp /Common/tcp { }
ltm profile http /Common/http { }
ltm policy /Common/policy_api_routing {
    requires { http }
    controls { forwarding }
    rules {
        api_route {
            ordinal 1
            conditions {
                0 {
                    http-host
                    host
                    equals
                    values { api.example.com }
                    request
                }
                1 {
                    http-uri
                    path
                    starts-with
                    values { /v1/ }
                    request
                }
            }
            actions {
                0 {
                    forward
                    select
                    pool /Common/pool_api
                }
            }
        }
        default_redirect {
            ordinal 99
            actions {
                0 {
                    http-reply
                    redirect
                    location "https://www.example.com/"
                }
            }
        }
    }
    strategy first-match
}
"""


def _http_get_packet(
    src: bytes,
    dst: bytes,
    sp: int,
    dp: int,
    *,
    host: str,
    path: str,
) -> bytes:
    payload = f"GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: tester\r\n\r\n".encode()
    return _build_packet(src, dst, sp, dp, syn=False, payload=payload)


def test_explain_flow_evaluates_policy_when_request_matches(tmp_path):
    """Policy whose conditions match the captured request should fire its action."""
    cfg = parse_bigip_conf(_CONF_WITH_POLICY)
    pcap = _build_pcap(
        [
            _build_packet(b"\xcb\x00\x71\x07", b"\x05\x06\x07\x08", 51422, 80, syn=True),
            _http_get_packet(
                b"\xcb\x00\x71\x07",
                b"\x05\x06\x07\x08",
                51422,
                80,
                host="api.example.com",
                path="/v1/health",
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_flow(p, {"u://x": cfg})
    assert report.matched_count == 1
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_policy"
    assert se.ltm_policies == ("/Common/policy_api_routing",)
    assert len(se.policy_decisions) == 1
    pd = se.policy_decisions[0]
    assert pd.policy == "/Common/policy_api_routing"
    fired = [r for r in pd.rules if r.fired]
    assert [r.rule for r in fired] == ["api_route"]
    assert fired[0].actions[0].target == "forward"
    assert fired[0].actions[0].value == "/Common/pool_api"


def test_explain_flow_falls_through_to_default_when_request_misses(tmp_path):
    """Policy whose specific rule misses should fall through to the unconditional default."""
    cfg = parse_bigip_conf(_CONF_WITH_POLICY)
    pcap = _build_pcap(
        [
            _build_packet(b"\xcb\x00\x71\x07", b"\x05\x06\x07\x08", 51423, 80, syn=True),
            _http_get_packet(
                b"\xcb\x00\x71\x07",
                b"\x05\x06\x07\x08",
                51423,
                80,
                host="other.example.com",
                path="/v1/health",
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_flow(p, {"u://x": cfg})
    se = report.sessions[0]
    pd = se.policy_decisions[0]
    fired = [r for r in pd.rules if r.fired]
    assert [r.rule for r in fired] == ["default_redirect"]
    assert fired[0].actions[0].target == "http-reply"
    assert fired[0].actions[0].value == "https://www.example.com/"


def test_explain_flow_policy_decisions_in_mcp_dict(tmp_path):
    cfg = parse_bigip_conf(_CONF_WITH_POLICY)
    pcap = _build_pcap(
        [
            _build_packet(b"\xcb\x00\x71\x07", b"\x05\x06\x07\x08", 51424, 80, syn=True),
            _http_get_packet(
                b"\xcb\x00\x71\x07",
                b"\x05\x06\x07\x08",
                51424,
                80,
                host="api.example.com",
                path="/v1/health",
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_flow(p, {"u://x": cfg})
    mcp = report_to_mcp_dict(report)
    session = mcp["sessions"][0]
    assert "policy_decisions" in session
    pd = session["policy_decisions"][0]
    assert pd["policy"] == "/Common/policy_api_routing"
    assert pd["strategy"] == "first-match"
    assert len(pd["fired"]) == 1
    fired = pd["fired"][0]
    assert fired["rule"] == "api_route"
    assert fired["actions"][0]["value"] == "/Common/pool_api"
    assert any(c["field"] == "http-host.host" for c in fired["matched_on"])


def test_compact_policy_decision_omits_unmatched_conditions():
    """`matched_on` in the compact MCP shape must list only conditions that matched."""
    from dialects.f5.bigip.explain_flow import _compact_policy_decision
    from dialects.f5.bigip.policy_eval import (
        ConditionTrace,
        PolicyDecision,
        RuleDecision,
    )

    pd = PolicyDecision(
        policy="/Common/p",
        strategy="first-match",
        rules=(
            RuleDecision(
                rule="r",
                ordinal=1,
                matched=True,
                fired=True,
                conditions=(
                    # Unmatched condition must be hidden from `matched_on`.
                    ConditionTrace(
                        operand="http-host",
                        selector="host",
                        operator="equals",
                        expected=("api.example.com",),
                        actual="other.com",
                        matched=False,
                        note="",
                    ),
                    ConditionTrace(
                        operand="http-method",
                        selector="",
                        operator="equals",
                        expected=("GET",),
                        actual="GET",
                        matched=True,
                        note="",
                    ),
                ),
                actions=(),
            ),
        ),
    )
    out = _compact_policy_decision(pd)
    assert len(out["fired"]) == 1
    matched_on = out["fired"][0]["matched_on"]
    assert len(matched_on) == 1
    assert matched_on[0]["field"] == "http-method"


def test_explain_flow_policy_decisions_text_report_mentions_decision(tmp_path):
    cfg = parse_bigip_conf(_CONF_WITH_POLICY)
    pcap = _build_pcap(
        [
            _build_packet(b"\xcb\x00\x71\x07", b"\x05\x06\x07\x08", 51425, 80, syn=True),
            _http_get_packet(
                b"\xcb\x00\x71\x07",
                b"\x05\x06\x07\x08",
                51425,
                80,
                host="api.example.com",
                path="/v1/health",
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_flow(p, {"u://x": cfg})
    text = report.text_report
    assert "ltm policy decisions" in text
    assert "/Common/policy_api_routing" in text
    assert "api_route" in text and "FIRED" in text
    assert "/Common/pool_api" in text


def test_extract_peer_tuple_from_trailer_legacy():
    """Legacy HIGH v0 trailer yields a peer 5-tuple."""
    trailer = _legacy_high_trailer(b"\x09\x09\x09\x09", b"\x0a\x0a\x0a\x0a", 12345, 8080)
    peer = _extract_peer_tuple_from_trailer(trailer)
    assert peer == ("9.9.9.9", 12345, "10.10.10.10", 8080)


def test_rst_detected_in_flow(tmp_path):
    pcap = _build_pcap(
        [
            _build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True),
            _build_packet(b"\x05\x06\x07\x08", b"\x01\x02\x03\x04", 443, 12345, rst=True),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    flows = extract_flows(p)
    rst_flows = [f for f in flows.values() if f.tcp_rst]
    assert len(rst_flows) == 1
    assert rst_flows[0].src_port == 443  # server-side issued the RST


def test_pair_sessions_links_front_and_back_via_trailer(tmp_path):
    """Two paired connections in a `:np`-style capture become one Session."""
    # Front side: client 1.2.3.4:11111 -> VIP 5.6.7.8:443.  Trailer
    # carries the proxied peer-side 5-tuple: TMM 10.0.0.5:22222
    # talking to pool member 10.0.0.10:8080.
    front_trailer = _legacy_high_trailer(b"\x0a\x00\x00\x0a", b"\x0a\x00\x00\x05", 8080, 22222)
    # Back side trailer points at the front peer.
    back_trailer = _legacy_high_trailer(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 11111, 443)
    pcap = _build_pcap(
        [
            _build_packet(
                b"\x01\x02\x03\x04",
                b"\x05\x06\x07\x08",
                11111,
                443,
                syn=True,
                trailer=front_trailer,
            ),
            _build_packet(
                b"\x05\x06\x07\x08",
                b"\x01\x02\x03\x04",
                443,
                11111,
                syn=False,
                trailer=front_trailer,
            ),
            _build_packet(
                b"\x0a\x00\x00\x05",
                b"\x0a\x00\x00\x0a",
                22222,
                8080,
                syn=True,
                trailer=back_trailer,
            ),
            _build_packet(
                b"\x0a\x00\x00\x0a",
                b"\x0a\x00\x00\x05",
                8080,
                22222,
                syn=False,
                trailer=back_trailer,
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    flows = extract_flows(p)
    # All four flows present.
    assert len(flows) == 4
    sessions = pair_sessions(flows)
    # Exactly one session pairs front with back.
    paired = [s for s in sessions if s.back is not None]
    assert len(paired) == 1
    sess = paired[0]
    assert sess.front.client.dst_ip == "5.6.7.8"
    assert sess.front.client.dst_port == 443
    assert sess.back is not None
    assert sess.back.client.dst_ip == "10.0.0.10"
    assert sess.back.client.dst_port == 8080


_CONF_NP = """\
ltm virtual /Common/vs_app {
    destination /Common/5.6.7.8:443
    pool /Common/pool_app
    rules { /Common/rule_app }
}
ltm pool /Common/pool_app {
    members { /Common/10.0.0.10:8080 { } }
}
ltm rule /Common/rule_app {
when CLIENT_ACCEPTED { log local0. "ok" }
}
"""


def test_explain_flow_session_includes_pool_and_snat(tmp_path):
    """`:np` capture should surface the chosen pool member and SNAT IP."""
    cfg = parse_bigip_conf(_CONF_NP)
    front_trailer = _legacy_high_trailer(b"\x0a\x00\x00\x0a", b"\x0a\x00\x00\x05", 8080, 22222)
    back_trailer = _legacy_high_trailer(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 11111, 443)
    pcap = _build_pcap(
        [
            _build_packet(
                b"\x01\x02\x03\x04",
                b"\x05\x06\x07\x08",
                11111,
                443,
                syn=True,
                trailer=front_trailer,
            ),
            _build_packet(
                b"\x0a\x00\x00\x05",
                b"\x0a\x00\x00\x0a",
                22222,
                8080,
                syn=True,
                trailer=back_trailer,
            ),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    assert report.matched_count == 1
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_app"
    assert se.pool_selected == "10.0.0.10:8080"
    assert se.snat_observed == "10.0.0.5:22222"


def test_reset_analysis_describes_termination(tmp_path):
    cfg = parse_bigip_conf(_CONF)
    pcap = _build_pcap(
        [
            _build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True),
            _build_packet(b"\x05\x06\x07\x08", b"\x01\x02\x03\x04", 443, 12345, rst=True),
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    se = report.sessions[0]
    assert "RST" in se.reset_analysis


_CONF_POLICIES = """\
ltm virtual /Common/vs_with_policy {
    destination /Common/5.6.7.8:443
    pool /Common/pool_app
    policies {
        /Common/policy_attached { }
    }
}
ltm pool /Common/pool_app {
    members { /Common/10.0.0.1:8080 { } }
}
ltm policy /Common/policy_attached { }
ltm policy /Common/policy_unrelated { }
"""


def test_ltm_policies_only_lists_attached(tmp_path):
    """Only policies in the VS's `policies { }` block should appear."""
    cfg = parse_bigip_conf(_CONF_POLICIES)
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_with_policy"
    # Only the attached policy is listed; the unrelated one is not.
    policy_blob = " ".join(se.ltm_policies)
    assert "policy_attached" in policy_blob
    assert "policy_unrelated" not in policy_blob


# ---------------------------------------------------------------------------
# HUD annotation tests (HTTP block / SNI route).
# ---------------------------------------------------------------------------


def _build_http_request_payload(method: str, uri: str, host: str, ua: str = "") -> bytes:
    parts = [
        f"{method} {uri} HTTP/1.1\r\n".encode(),
        f"Host: {host}\r\n".encode(),
    ]
    if ua:
        parts.append(f"User-Agent: {ua}\r\n".encode())
    parts.append(b"\r\n")
    return b"".join(parts)


_CONF_BLOCK_RULE = """\
ltm virtual /Common/vs_block {
    destination /Common/5.6.7.8:80
    pool /Common/pool_app
    profiles {
        /Common/tcp { }
        /Common/http { }
    }
    rules { /Common/rule_block }
}
ltm pool /Common/pool_app {
    members { /Common/10.0.0.1:8080 { } }
}
ltm profile tcp /Common/tcp { }
ltm profile http /Common/http { }
ltm rule /Common/rule_block {
when HTTP_REQUEST {
    if { [HTTP::host] equals "blocked.example.com" } {
        HTTP::respond 403 content "Blocked"
    }
}
}
"""


def test_hud_annotations_capture_host_value(tmp_path):
    """HUD annotations should surface the captured Host for `[HTTP::host]`."""
    cfg = parse_bigip_conf(_CONF_BLOCK_RULE)
    payload = _build_http_request_payload("GET", "/", "blocked.example.com", "curl/8.0")
    pcap = _build_pcap(
        [
            _build_packet(
                b"\x01\x02\x03\x04",
                b"\x05\x06\x07\x08",
                12345,
                80,
                syn=False,
                payload=payload,
            )
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    assert report.matched_count == 1
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_block"
    # The front-side flow captured Host + UA from the request payload.
    front = se.session.front.client
    assert front.http_host == "blocked.example.com"
    assert front.http_user_agent == "curl/8.0"
    # Annotations contain HTTP::host -> blocked.example.com.
    annotated_cmds = [(cmd, val) for _r, _e, anns in se.event_annotations for _ln, cmd, val in anns]
    assert ("HTTP::host", "blocked.example.com") in annotated_cmds


_CONF_SNI_ROUTE = """\
ltm virtual /Common/vs_sni {
    destination /Common/5.6.7.8:443
    pool /Common/pool_default
    profiles {
        /Common/tcp { }
        /Common/clientssl { }
    }
    rules { /Common/rule_sni }
}
ltm pool /Common/pool_default {
    members { /Common/10.0.0.1:443 { } }
}
ltm pool /Common/pool_api {
    members { /Common/10.0.0.2:443 { } }
}
ltm profile tcp /Common/tcp { }
ltm profile client-ssl /Common/clientssl { }
ltm rule /Common/rule_sni {
when CLIENTSSL_CLIENTHELLO {
    if { [SSL::extensions exists -type 0] } {
        set sni_name [SSL::extensions servername]
        if { $sni_name equals "api.example.com" } {
            pool /Common/pool_api
        }
    }
}
}
"""


def _build_tls_clienthello(sni: str) -> bytes:
    """Build a minimal TLS 1.2 ClientHello with an SNI extension."""
    sni_bytes = sni.encode("ascii")
    sni_ext = b"\x00\x00" + (5 + len(sni_bytes)).to_bytes(2, "big")  # ext_type, ext_len
    sni_list = (3 + len(sni_bytes)).to_bytes(2, "big")  # list len
    sni_entry = b"\x00" + len(sni_bytes).to_bytes(2, "big") + sni_bytes  # name_type=host_name
    sni_ext += sni_list + sni_entry
    extensions = sni_ext
    ext_block = len(extensions).to_bytes(2, "big") + extensions
    body = (
        b"\x03\x03"  # client_version TLS1.2
        + b"\x00" * 32  # random
        + b"\x00"  # session_id length 0
        + b"\x00\x02\x00\x35"  # cipher_suites len=2 + AES128-SHA
        + b"\x01\x00"  # compression methods: null
        + ext_block
    )
    handshake = b"\x01" + len(body).to_bytes(3, "big") + body
    record = b"\x16\x03\x03" + len(handshake).to_bytes(2, "big") + handshake
    return record


def test_hud_annotations_capture_sni_for_route_decision(tmp_path):
    """HUD annotation should expose SNI when the iRule branches on it."""
    cfg = parse_bigip_conf(_CONF_SNI_ROUTE)
    pcap = _build_pcap(
        [
            _build_packet(
                b"\x01\x02\x03\x04",
                b"\x05\x06\x07\x08",
                12345,
                443,
                syn=False,
                payload=_build_tls_clienthello("api.example.com"),
            )
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    assert report.matched_count == 1
    se = report.sessions[0]
    assert se.matched_vs == "/Common/vs_sni"
    assert se.session.front.client.tls_sni == "api.example.com"
    # The CLIENTSSL_CLIENTHELLO event uses [SSL::extensions ...]; the
    # annotation should map to the captured SNI value.
    annotated = [
        (cmd, val)
        for _r, ev, anns in se.event_annotations
        for _ln, cmd, val in anns
        if ev == "CLIENTSSL_CLIENTHELLO"
    ]
    cmds = [c for c, _v in annotated]
    # Either SSL::extensions or SNI tokens should resolve to the SNI value.
    # Word-boundary match so a substring inside a longer hostname /
    # path wouldn't satisfy the assertion (CodeQL
    # ``py/incomplete-url-substring-sanitization``).
    assert any(re.search(r"\bapi\.example\.com\b", v) for _c, v in annotated), annotated
    assert any(c.startswith("SSL::") or c == "SNI" for c in cmds), cmds


# ---------------------------------------------------------------------------
# MCP-compact report shape.
# ---------------------------------------------------------------------------


def test_report_to_mcp_dict_drops_noise(tmp_path):
    """The MCP shape pre-narrates per session and omits empty fields."""
    cfg = parse_bigip_conf(_CONF_BLOCK_RULE)
    payload = _build_http_request_payload("GET", "/", "blocked.example.com", "curl/8.0")
    pcap = _build_pcap(
        [
            _build_packet(
                b"\x01\x02\x03\x04",
                b"\x05\x06\x07\x08",
                12345,
                80,
                syn=False,
                payload=payload,
            )
        ]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)
    report = compute_explain_flow(p, {"u://x": cfg})
    compact = report_to_mcp_dict(report, max_event_body_lines=4)

    assert compact["matched"] == 1
    sess = compact["sessions"][0]
    # Compact shape: short summary, flow as ip:port strings, decisions
    # de-duped, no full per-flow Flow dicts visible.
    assert "summary" in sess
    assert re.search(r"\bblocked\.example\.com\b", sess["summary"])
    assert sess["flow"]["client"] == "1.2.3.4:12345"
    assert sess["flow"]["vip"] == "5.6.7.8:80"
    assert any(
        d.get("command") == "HTTP::host" and d.get("value") == "blocked.example.com"
        for d in sess.get("irule_decisions", [])
    )
    # Empty-noise fields should be absent.
    assert "explain_text" not in sess
    assert "captured_response" not in sess  # response side wasn't captured
    # Body truncated to <= max_event_body_lines + the truncated marker.
    for body in sess.get("irule_bodies", []):
        line_count = body["body"].count("\n") + 1
        assert line_count <= 5  # 4 lines + truncation marker
