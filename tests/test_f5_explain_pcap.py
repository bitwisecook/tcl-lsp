"""Tests for ``f5 explain-pcap`` and the underlying flow tracer."""

from __future__ import annotations

import io
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.explain_pcap import (
    _extract_event_blocks,
    _parse_destination,
    compute_explain_pcap,
    extract_flows,
)
from core.bigip.parser import parse_bigip_conf
from explorer.f5_cli import main

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
    src: bytes, dst: bytes, sp: int, dp: int, *, syn: bool = True, payload: bytes = b""
) -> bytes:
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    flags = 0x02 if syn else 0x18  # SYN or PSH+ACK
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
    return eth + bytes(ip) + tcp


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


def test_compute_explain_pcap_matches_vs_and_orders_events(tmp_path):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    cfg = parse_bigip_conf(_CONF)

    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    p = tmp_path / "in.pcap"
    p.write_bytes(pcap)

    report = compute_explain_pcap(p, {"u://x": cfg})
    assert report.flow_count == 1
    assert report.matched_count == 1
    fe = report.flows[0]
    assert fe.matched_vs == "/Common/vs_app"
    assert any("clientssl" in line for line in fe.profile_chain)
    # CLIENT_ACCEPTED must come before HTTP_REQUEST, which must come before
    # SERVER_CONNECTED in the canonical ordering.
    seq_events = [s.split("::")[-1] for s in fe.event_sequence]
    assert "CLIENT_ACCEPTED" in seq_events
    assert "HTTP_REQUEST" in seq_events
    assert "SERVER_CONNECTED" in seq_events
    assert seq_events.index("CLIENT_ACCEPTED") < seq_events.index("HTTP_REQUEST")
    assert seq_events.index("HTTP_REQUEST") < seq_events.index("SERVER_CONNECTED")
    # Event bodies are surfaced.
    assert any("X-Forwarded-For" in body for _, _, body in fe.event_blocks)


def test_explain_pcap_cli_text(tmp_path, capsys):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-pcap", str(pcap_path), str(conf_path)])
    out = capsys.readouterr().out
    assert code == 0
    assert "/Common/vs_app" in out
    assert "CLIENT_ACCEPTED" in out
    assert "HTTP_REQUEST" in out


def test_explain_pcap_cli_no_match_returns_1(tmp_path, capsys):
    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    # Destination 9.9.9.9 has no matching VS in the config.
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x09\x09\x09\x09", 12345, 80, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-pcap", str(pcap_path), str(conf_path)])
    assert code == 1
    out = capsys.readouterr().out
    assert "no virtual server matched" in out


def test_explain_pcap_cli_json(tmp_path, capsys):
    import json as _json

    conf_path = tmp_path / "bigip.conf"
    conf_path.write_text(_CONF)
    pcap = _build_pcap(
        [_build_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08", 12345, 443, syn=True)]
    )
    pcap_path = tmp_path / "in.pcap"
    pcap_path.write_bytes(pcap)

    code = main(["explain-pcap", "--json", str(pcap_path), str(conf_path)])
    out = capsys.readouterr().out
    assert code == 0
    data = _json.loads(out)
    assert data["matched_count"] == 1
    assert data["flows"][0]["matched_vs"] == "/Common/vs_app"
