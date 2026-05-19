"""Tests for ``f5 pcap-remap`` and the underlying PCAP rewriter."""

from __future__ import annotations

import io
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.pcap_remap import remap_pcap
from dialects.f5.bigip.redact_map import build_map
from tooling.f5.main import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _ipv4_checksum(header: bytes) -> int:
    total = 0
    for i in range(0, len(header), 2):
        total += (header[i] << 8) | header[i + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return (~total) & 0xFFFF


def _build_tcp_packet(
    src_ip: bytes, dst_ip: bytes, src_port: int = 12345, dst_port: int = 80
) -> bytes:
    """Build an Ethernet+IPv4+TCP frame with a tiny SYN packet."""
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"  # ethertype IPv4
    payload = b""  # no TCP payload
    tcp_len = 20  # no options
    tcp_total = tcp_len + len(payload)
    ip_total = 20 + tcp_total
    ip = bytearray(
        bytes([0x45, 0x00])  # version+ihl, dscp
        + struct.pack(">H", ip_total)  # total length
        + bytes(
            [0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00]
        )  # id, flags, ttl, proto=6, cksum=0
        + src_ip
        + dst_ip
    )
    ip_cksum = _ipv4_checksum(bytes(ip))
    ip[10] = (ip_cksum >> 8) & 0xFF
    ip[11] = ip_cksum & 0xFF
    # TCP header (we leave checksum 0 for simplicity; the rewriter
    # will recompute it on rewrite, so we don't need a valid input
    # checksum for the test).
    tcp = (
        struct.pack(">HH", src_port, dst_port)
        + struct.pack(">II", 1, 0)  # seq, ack
        + bytes([0x50, 0x02])  # data offset 5 + flags=SYN
        + struct.pack(">H", 0xFFFF)  # window
        + struct.pack(">HH", 0, 0)  # checksum, urgent
    )
    return eth + bytes(ip) + tcp + payload


def _build_pcap(packets: list[bytes]) -> bytes:
    out = io.BytesIO()
    # Global header: magic, ver_major(2), ver_minor(4), thiszone(0),
    # sigfigs(0), snaplen(65535), network(1=Ethernet)
    out.write(struct.pack("<IHHIIII", 0xA1B2C3D4, 2, 4, 0, 0, 65535, 1))
    for packet in packets:
        ts_sec = 1000000
        ts_usec = 0
        out.write(struct.pack("<IIII", ts_sec, ts_usec, len(packet), len(packet)))
        out.write(packet)
    return out.getvalue()


def test_remap_pcap_ipv4_rewrites_addresses():
    src_real = b"\x01\x02\x03\x04"  # 1.2.3.4
    dst_real = b"\x05\x06\x07\x08"  # 5.6.7.8
    pcap_in = _build_pcap([_build_tcp_packet(src_real, dst_real)])
    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)
    assert result.packets_rewritten == 1
    assert result.addresses_rewritten == 2
    output = out.getvalue()
    assert src_real not in output[24:]  # original src absent in packet record area
    assert dst_real not in output[24:]


def test_remap_pcap_round_trip_via_reverse():
    """Forward then reverse must restore the source and destination IPs.

    The byte-level packet is allowed to differ from the original because
    we recompute checksums after each pass (a synthetic test packet
    typically lacks a valid TCP checksum to begin with), but the
    semantically-meaningful fields — IP src/dst — must round-trip exactly.
    """
    src_real = b"\x01\x02\x03\x04"
    dst_real = b"\x09\x08\x07\x06"
    pcap_in = _build_pcap([_build_tcp_packet(src_real, dst_real)])
    rm = build_map(text="1.2.3.4 9.8.7.6")

    forward_out = io.BytesIO()
    remap_pcap(io.BytesIO(pcap_in), forward_out, rm)

    reverse_out = io.BytesIO()
    remap_pcap(io.BytesIO(forward_out.getvalue()), reverse_out, rm, reverse=True)

    # IP header sits at: global hdr 24 + record hdr 16 + Ethernet 14.
    ip_off = 24 + 16 + 14
    final = reverse_out.getvalue()
    assert final[ip_off + 12 : ip_off + 16] == src_real
    assert final[ip_off + 16 : ip_off + 20] == dst_real


def test_remap_pcap_recomputes_ipv4_checksum():
    src_real = b"\x01\x02\x03\x04"
    dst_real = b"\x05\x06\x07\x08"
    pcap_in = _build_pcap([_build_tcp_packet(src_real, dst_real)])
    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    remap_pcap(io.BytesIO(pcap_in), out, rm)
    output = out.getvalue()
    # Skip global header (24) + record header (16) + Ethernet (14).
    ip_off = 24 + 16 + 14
    ip = output[ip_off : ip_off + 20]
    saved_cksum_high = ip[10]
    saved_cksum_low = ip[11]
    zeroed = bytes(ip[:10]) + b"\x00\x00" + bytes(ip[12:])
    expected = _ipv4_checksum(zeroed)
    assert ((saved_cksum_high << 8) | saved_cksum_low) == expected


def _legacy_high_trailer(remote_v4: bytes, local_v4: bytes) -> bytes:
    """Build a legacy F5 HIGH v0 trailer (total 42 bytes) carrying two IPv4
    peer addresses in IPv4-mapped IPv6 form.  Wire length byte = 40
    (Wireshark adds 2 to compute total entry size — see
    packet-f5ethtrailer.c line 2763)."""
    ipv4_mapped_remote = bytes(10) + b"\xff\xff" + remote_v4
    ipv4_mapped_local = bytes(10) + b"\xff\xff" + local_v4
    body = (
        bytes([0x06])  # ipproto = TCP
        + struct.pack(">H", 0)  # vlan
        + ipv4_mapped_remote
        + ipv4_mapped_local
        + struct.pack(">H", 12345)
        + struct.pack(">H", 80)
    )
    return bytes([3, 40, 0]) + body  # type=HIGH, wire-len=40 (total=42), ver=0


def test_remap_pcap_rewrites_peer_ips_in_legacy_trailer():
    """Peer IPs inside a legacy F5 HIGH v0 trailer get rewritten via
    the parsed schema, not byte-sweep."""
    src_real = b"\x01\x02\x03\x04"
    dst_real = b"\x05\x06\x07\x08"
    peer_remote = b"\x09\x09\x09\x09"  # 9.9.9.9, public
    peer_local = b"\x14\x14\x14\x14"  # 20.20.20.20, public
    trailer = _legacy_high_trailer(peer_remote, peer_local)
    packet = _build_tcp_packet(src_real, dst_real) + trailer
    pcap_in = _build_pcap([packet])
    rm = build_map(text="1.2.3.4 5.6.7.8 9.9.9.9 20.20.20.20")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)
    output = out.getvalue()
    assert result.trailer_tlvs_total == 1
    assert result.trailer_tlvs_rewritten == 1
    assert result.trailer_tlvs_unknown == 0
    # IP-layer rewrite (2) + trailer peer-remote + peer-local (2) = 4
    assert result.addresses_rewritten == 4
    # Neither peer IP should appear anywhere in the rewritten record.
    record = output[24:]
    assert peer_remote not in record
    assert peer_local not in record


def test_remap_pcap_unknown_trailer_errors_by_default():
    """A structurally valid legacy trailer with an unknown (type, version)
    must trigger UnknownTrailerError unless --on-unknown is set."""
    from dialects.f5.bigip.pcap_remap import UnknownTrailerError

    # type=HIGH but version=99 — not in the schema registry.
    weird = bytes([3, 40, 99]) + b"\x00" * 39
    packet = _build_tcp_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08") + weird
    pcap_in = _build_pcap([packet])
    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    try:
        remap_pcap(io.BytesIO(pcap_in), out, rm)
    except UnknownTrailerError as exc:
        assert exc.fmt == "legacy"
        assert exc.version == 99
    else:
        raise AssertionError("expected UnknownTrailerError")


def test_remap_pcap_unknown_trailer_preserve_policy_leaves_bytes():
    weird_value = b"\xde\xad\xbe\xef" + b"\x01\x02\x03\x04" + b"\x00" * 31
    weird = bytes([3, 40, 99]) + weird_value
    packet = _build_tcp_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08") + weird
    pcap_in = _build_pcap([packet])
    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm, unknown_policy="preserve")
    assert result.trailer_tlvs_unknown == 1
    assert result.trailer_tlvs_rewritten == 0
    # The known-IP bytes inside the unknown TLV must be preserved.
    assert b"\x01\x02\x03\x04" in out.getvalue()


def test_remap_pcap_unknown_trailer_sweep_policy_replaces_known_ips():
    weird_value = b"\xde\xad\xbe\xef" + b"\x01\x02\x03\x04" + b"\x00" * 31
    weird = bytes([3, 40, 99]) + weird_value
    packet = _build_tcp_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08") + weird
    pcap_in = _build_pcap([packet])
    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm, unknown_policy="sweep")
    assert result.trailer_tlvs_unknown == 1
    assert result.trailer_tlvs_rewritten == 1
    # 1.2.3.4 must be rewritten by the sweep fallback within just this TLV.
    assert b"\x01\x02\x03\x04" not in out.getvalue()[24:]


def test_remap_pcap_does_not_touch_l4_payload():
    """4-byte sequences inside the TCP payload that happen to match a
    known IP must be LEFT ALONE — payload is application data."""
    # Build a fresh frame with explicit total_length covering payload
    # bytes so the rewriter classifies them as "still inside the L4
    # payload", not trailer:
    src = b"\x01\x02\x03\x04"
    dst = b"\x05\x06\x07\x08"
    ip_payload = b"\x01\x02\x03\x04AAAA"  # 8 bytes "L4 payload" containing the known IP
    ip_total = 20 + 8
    ip = bytearray(
        bytes([0x45, 0x00])
        + struct.pack(">H", ip_total)
        + bytes([0, 0, 0x40, 0, 0x40, 0xFD, 0, 0])  # proto 0xFD (no L4 cksum)
        + src
        + dst
    )
    cksum = _ipv4_checksum(bytes(ip))
    ip[10], ip[11] = (cksum >> 8) & 0xFF, cksum & 0xFF
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    packet = eth + bytes(ip) + ip_payload
    pcap_in = _build_pcap([packet])
    rm = build_map(text="1.2.3.4 5.6.7.8")

    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)
    # Only IP-layer src+dst should change; the 1.2.3.4 inside the L4
    # payload is left alone.
    assert result.addresses_rewritten == 2

    output = out.getvalue()
    # The 1.2.3.4 bytes inside the payload must STILL be present.
    record_body = output[24 + 16 :]
    # IP header sits at offset 14 (Ethernet); IP total_len is 28; so the
    # payload starts at byte 14 + 20 = 34 inside the record body.  Find
    # 1.2.3.4 in the payload region.
    payload_region = record_body[34:]
    assert b"\x01\x02\x03\x04" in payload_region


def test_pcap_remap_verb_round_trips(tmp_path, capsys):
    src_real = b"\x01\x02\x03\x04"
    dst_real = b"\x09\x08\x07\x06"
    pcap_in_bytes = _build_pcap([_build_tcp_packet(src_real, dst_real)])

    in_path = tmp_path / "in.pcap"
    in_path.write_bytes(pcap_in_bytes)

    rm = build_map(text="1.2.3.4 9.8.7.6")
    map_path = tmp_path / "map.toml"
    map_path.write_text(rm.to_toml())

    forward = tmp_path / "out.pcap"
    code, _o, err = _run(["pcap-remap", str(map_path), str(in_path), str(forward)], capsys)
    assert code == 0, err
    assert "1/1 packet(s) rewritten" in err
    assert forward.read_bytes() != pcap_in_bytes

    reverse = tmp_path / "rt.pcap"
    code, _o, _e = _run(
        ["pcap-remap", "--reverse", str(map_path), str(forward), str(reverse)], capsys
    )
    assert code == 0
    final = reverse.read_bytes()
    ip_off = 24 + 16 + 14
    assert final[ip_off + 12 : ip_off + 16] == src_real
    assert final[ip_off + 16 : ip_off + 20] == dst_real


def test_pcap_remap_rejects_bad_magic(tmp_path, capsys):
    junk = tmp_path / "junk.pcap"
    junk.write_bytes(b"\x00" * 32)
    rm = build_map(text="")
    map_path = tmp_path / "m.toml"
    map_path.write_text(rm.to_toml())
    out = tmp_path / "o.pcap"
    code, _o, err = _run(["pcap-remap", str(map_path), str(junk), str(out)], capsys)
    assert code == 2
    assert "magic" in err


# ── regression tests for issues found in PCAP code review ──────────


def test_remap_pcap_trailer_lookup_invalidates_across_packets():
    """Regression: in sweep mode the packed-lookup cache used to be
    keyed by id(rm) only, so an IP first added by packet N's IP layer
    was missing from packet N+1's sweep.  Cache is now keyed by
    len(forward) too."""
    p1 = _build_tcp_packet(b"\x01\x02\x03\x04", b"\x05\x06\x07\x08")
    weird = bytes([3, 40, 99]) + b"\x00" * 4 + b"\x01\x02\x03\x04" + b"\x00" * 31
    p2 = _build_tcp_packet(b"\x07\x07\x07\x07", b"\x08\x08\x08\x08") + weird
    pcap_in = _build_pcap([p1, p2])
    rm = build_map(text="1.2.3.4 5.6.7.8 7.7.7.7 8.8.8.8")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm, unknown_policy="sweep")
    assert result.packets_total == 2
    assert b"\x01\x02\x03\x04" not in out.getvalue()[24:]


def test_remap_pcap_handles_truncated_pcapng_gracefully():
    """Malformed PCAPNG SHB (correct block-type but no valid byte-order
    magic) should produce a clear error rather than a crash or silent
    success."""
    pcapng_header = b"\x0a\x0d\x0d\x0a" + b"\x00" * 28
    rm = build_map(text="")
    out = io.BytesIO()
    try:
        remap_pcap(io.BytesIO(pcapng_header), out, rm)
    except ValueError as exc:
        assert "pcapng" in str(exc).lower() or "byte-order" in str(exc).lower()
    else:
        raise AssertionError("expected ValueError for malformed PCAPNG input")


def test_remap_pcap_skips_l4_checksum_on_ipv4_fragment():
    """First-fragment packet (MF=1) — we must not write a TCP checksum
    derived from a partial L4 segment.  The L4 cksum bytes should
    survive untouched while src/dst still get rewritten."""
    src = b"\x01\x02\x03\x04"
    dst = b"\x05\x06\x07\x08"
    # Build an IPv4 packet with MF=1 and a fake small TCP header
    # (just to give us bytes at the cksum offset to compare).
    fake_l4 = bytes([0x12, 0x34] * 10)  # 20 bytes of "TCP-like" data
    ip_total = 20 + len(fake_l4)
    flags_frag_more = struct.pack(">H", 0x2000)  # MF=1, frag offset 0
    ip = bytearray(
        bytes([0x45, 0x00])
        + struct.pack(">H", ip_total)
        + bytes([0, 1])  # id
        + flags_frag_more
        + bytes([0x40, 0x06, 0x00, 0x00])  # ttl, proto=TCP, cksum
        + src
        + dst
    )
    cksum = _ipv4_checksum(bytes(ip))
    ip[10], ip[11] = (cksum >> 8) & 0xFF, cksum & 0xFF
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    packet = eth + bytes(ip) + fake_l4
    pcap_in = _build_pcap([packet])

    rm = build_map(text="1.2.3.4 5.6.7.8")
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)

    rewritten = out.getvalue()
    ip_off = 24 + 16 + 14
    # IP src/dst should still be rewritten…
    assert rewritten[ip_off + 12 : ip_off + 16] != src
    # …but the fake TCP cksum bytes must be left as the original.
    tcp_cksum_off = ip_off + 20 + 16  # offset of TCP cksum within fake L4
    assert rewritten[tcp_cksum_off : tcp_cksum_off + 2] == fake_l4[16:18]
    assert result.addresses_rewritten == 2  # only the IP layer


# ── IPv6 extension headers ─────────────────────────────────────────


def _ipv6_packet_with_hop_by_hop(src: bytes, dst: bytes) -> bytes:
    """Build IPv6+HopByHop+UDP frame.  HopByHop is 8 bytes of pad, then
    UDP src/dst ports + length + checksum + 0 bytes payload."""
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x86\xdd"  # ethertype IPv6
    udp = (
        struct.pack(">HH", 12345, 53)  # src/dst port
        + struct.pack(">H", 8)  # length (header only)
        + struct.pack(">H", 0)  # checksum (placeholder)
    )
    # Hop-by-Hop ext header: next(1)=UDP(17), hdr_ext_len(1)=0 (=> 8 bytes total),
    # then 6 bytes of options (PadN=1, len=4, 4 zero bytes).
    hop_by_hop = bytes([17, 0, 1, 4, 0, 0, 0, 0])
    ipv6_payload = hop_by_hop + udp
    ipv6 = (
        bytes([0x60, 0, 0, 0])  # version=6, tc=0, flowlabel=0
        + struct.pack(">H", len(ipv6_payload))  # payload length
        + bytes([0])  # next header = HopByHop
        + bytes([64])  # hop limit
        + src
        + dst
    )
    return eth + ipv6 + ipv6_payload


def test_remap_pcap_walks_ipv6_extension_headers():
    """An IPv6 packet with a Hop-by-Hop options header before the UDP
    layer must still get its UDP checksum recomputed correctly after
    src/dst rewrite."""
    src = b"\x20\x01\x0d\xb8" + bytes(12)  # 2001:db8::
    dst = b"\x20\x01\x0d\xb8" + bytes(11) + b"\x01"  # 2001:db8::1
    packet = _ipv6_packet_with_hop_by_hop(src, dst)
    pcap_in = _build_pcap([packet])

    # 2001:db8::/32 is the RFC3849 documentation prefix — flagged as
    # is_private by ipaddress, so we need remap_private=True.
    rm = build_map(
        text="2001:db8:: 2001:db8::1",
        target_pool_v6=("fd00::/8",),
        remap_private=True,
    )
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)
    assert result.addresses_rewritten == 2

    rewritten = out.getvalue()
    # IPv6 header at: pcap-global(24) + record(16) + eth(14) = 54.
    ip_off = 24 + 16 + 14
    # src/dst rewritten
    assert rewritten[ip_off + 8 : ip_off + 24] != src
    assert rewritten[ip_off + 24 : ip_off + 40] != dst

    # UDP starts at ip_off + 40 (IPv6 fixed) + 8 (HopByHop) = ip_off + 48.
    udp_off = ip_off + 48
    new_src = rewritten[ip_off + 8 : ip_off + 24]
    new_dst = rewritten[ip_off + 24 : ip_off + 40]
    udp_len = struct.unpack(">H", rewritten[udp_off + 4 : udp_off + 6])[0]

    # Recompute the UDP+pseudo-header checksum independently and compare.
    pseudo = new_src + new_dst + struct.pack(">I", udp_len) + b"\x00\x00\x00" + bytes([17])
    udp_bytes_zerocsum = (
        rewritten[udp_off : udp_off + 6] + b"\x00\x00" + rewritten[udp_off + 8 : udp_off + udp_len]
    )

    def _csum(data: bytes) -> int:
        if len(data) % 2:
            data = data + b"\x00"
        total = 0
        for i in range(0, len(data), 2):
            total += (data[i] << 8) | data[i + 1]
            total = (total & 0xFFFF) + (total >> 16)
        return (~total) & 0xFFFF

    expected = _csum(pseudo + udp_bytes_zerocsum)
    if expected == 0:
        expected = 0xFFFF
    actual = struct.unpack(">H", rewritten[udp_off + 6 : udp_off + 8])[0]
    assert actual == expected, f"UDP cksum: expected 0x{expected:04x}, got 0x{actual:04x}"


def test_remap_pcap_skips_l4_checksum_on_ipv6_fragment():
    """IPv6 packet with Fragment extension header (next-header 44).
    L4 segment is partial — skip cksum recompute; src/dst still rewritten."""
    src = b"\x20\x01\x0d\xb8" + bytes(12)
    dst = b"\x20\x01\x0d\xb8" + bytes(11) + b"\x01"
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x86\xdd"
    # Fragment ext header (8 bytes): next=UDP(17), reserved(1), offset+M(2)=0|MF, id(4).
    frag_more = struct.pack(">H", 0x0001)  # MF=1, offset=0
    fragment = bytes([17, 0]) + frag_more + bytes([0, 0, 0, 1])
    fake_udp_partial = bytes(4)  # truncated
    payload = fragment + fake_udp_partial
    ipv6 = bytes([0x60, 0, 0, 0]) + struct.pack(">H", len(payload)) + bytes([44, 64]) + src + dst
    packet = eth + ipv6 + payload
    pcap_in = _build_pcap([packet])

    rm = build_map(
        text="2001:db8:: 2001:db8::1",
        target_pool_v6=("fd00::/8",),
        remap_private=True,
    )
    out = io.BytesIO()
    result = remap_pcap(io.BytesIO(pcap_in), out, rm)
    # Only the IP-layer src/dst — no L4 cksum because the fragment is
    # partial.
    assert result.addresses_rewritten == 2

    rewritten = out.getvalue()
    ip_off = 24 + 16 + 14
    # The "fake UDP" bytes after the Fragment ext header must be untouched.
    fake_off = ip_off + 40 + 8
    assert rewritten[fake_off : fake_off + 4] == fake_udp_partial
