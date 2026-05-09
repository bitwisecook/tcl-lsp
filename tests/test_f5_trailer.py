"""Tests for the F5 Ethernet trailer parser."""

from __future__ import annotations

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.f5_trailer import (
    DPT_HDR_MAGIC,
    DPT_TLV_HDR_LEN,
    LEGACY_TYPE_HIGH,
    LEGACY_TYPE_LOW,
    LEGACY_TYPE_MED,
    classify_v6_or_v4mapped,
    load_schema_overlay,
    parse_trailer,
    register_dpt_schema,
    register_legacy_schema,
)

# ── helpers to construct trailer bytes ─────────────────────────────


def _ipv4_mapped(addr: str) -> bytes:
    """16-byte IPv4-mapped IPv6 form: ::ffff:a.b.c.d."""
    import ipaddress

    return bytes(10) + b"\xff\xff" + ipaddress.IPv4Address(addr).packed


def _legacy_high_v0(remote_v4: str, local_v4: str) -> bytes:
    """Build a legacy HIGH v0 entry (length=42) from two IPv4 addresses."""
    body = (
        bytes([0x06])  # ipproto = TCP
        + struct.pack(">H", 0)  # vlan
        + _ipv4_mapped(remote_v4)
        + _ipv4_mapped(local_v4)
        + struct.pack(">H", 12345)  # remote port
        + struct.pack(">H", 80)  # local port
    )
    assert len(body) == 39  # 1+2+16+16+2+2
    return bytes([LEGACY_TYPE_HIGH, 42, 0]) + body


def _legacy_low(length: int = 22) -> bytes:
    return bytes([LEGACY_TYPE_LOW, length, 0]) + b"\x00" * (length - 3)


def _legacy_med(length: int = 8) -> bytes:
    return bytes([LEGACY_TYPE_MED, length, 0]) + b"\x00" * (length - 3)


def _dpt_hdr(payload: bytes) -> bytes:
    total_len = 8 + len(payload)
    return struct.pack(">IHH", DPT_HDR_MAGIC, total_len, 1) + payload


def _dpt_tlv(provider: int, type_: int, version: int, value: bytes) -> bytes:
    length = DPT_TLV_HDR_LEN + len(value)
    return struct.pack(">HHHH", provider, type_, length, version) + value


def _dpt_high_v1(remote_v4: str, local_v4: str) -> bytes:
    body = (
        bytes([0x06])
        + struct.pack(">H", 0)
        + _ipv4_mapped(remote_v4)
        + _ipv4_mapped(local_v4)
        + struct.pack(">H", 12345)
        + struct.pack(">H", 80)
    )
    assert len(body) == 39
    return _dpt_tlv(provider=1, type_=LEGACY_TYPE_HIGH, version=1, value=body)


# ── format detection ───────────────────────────────────────────────


def test_parse_trailer_returns_none_for_random_bytes():
    parse = parse_trailer(b"random non-trailer bytes that happen to be long")
    assert parse.fmt is None


def test_parse_trailer_returns_none_for_too_short():
    parse = parse_trailer(b"\x01\x02")
    assert parse.fmt is None


def test_parse_trailer_legacy_format_detected():
    data = _legacy_low() + _legacy_med() + _legacy_high_v0("8.8.8.8", "1.1.1.1")
    parse = parse_trailer(data)
    assert parse.fmt == "legacy"
    assert len(parse.tlvs) == 3
    assert [t.type_ for t in parse.tlvs] == [
        LEGACY_TYPE_LOW,
        LEGACY_TYPE_MED,
        LEGACY_TYPE_HIGH,
    ]
    assert parse.bytes_consumed == len(data)


def test_parse_trailer_dpt_format_detected():
    payload = _dpt_tlv(1, LEGACY_TYPE_LOW, 1, b"\x00" * 4) + _dpt_high_v1("8.8.8.8", "1.1.1.1")
    data = _dpt_hdr(payload)
    parse = parse_trailer(data)
    assert parse.fmt == "dpt"
    assert len(parse.tlvs) == 2
    assert parse.tlvs[1].type_ == LEGACY_TYPE_HIGH
    assert parse.tlvs[1].version == 1


def test_parse_trailer_legacy_terminates_on_invalid_type():
    """The legacy walker stops when the next byte isn't 1/2/3."""
    data = _legacy_low() + b"\xff"  # bogus byte after the TLV chain
    parse = parse_trailer(data)
    assert parse.fmt == "legacy"
    assert len(parse.tlvs) == 1


def test_parse_trailer_legacy_terminates_on_oob_length():
    """Length running past the end of the buffer terminates the walk."""
    truncated = _legacy_high_v0("8.8.8.8", "1.1.1.1")[:30]  # claim 42, give 30
    parse = parse_trailer(truncated)
    assert parse.fmt == "legacy"
    assert len(parse.tlvs) == 0


# ── schema lookup ──────────────────────────────────────────────────


def test_legacy_high_v0_ip_fields_at_known_offsets():
    data = _legacy_high_v0("8.8.8.8", "1.1.1.1")
    parse = parse_trailer(data)
    high = parse.tlvs[0]
    assert high.schema_known
    assert len(high.ip_fields) == 2
    # Offsets within the trailer: header(3) + ipproto(1) + vlan(2) = 6 for first.
    assert high.ip_fields[0].offset == 6
    assert high.ip_fields[1].offset == 22  # 6 + 16


def test_dpt_high_v1_ip_fields_at_known_offsets():
    data = _dpt_hdr(_dpt_high_v1("8.8.8.8", "1.1.1.1"))
    parse = parse_trailer(data)
    high = parse.tlvs[0]
    assert high.schema_known
    # DPT TLV starts after 8-byte DPT header; field offsets are absolute
    # within the trailer, i.e. tlv_offset(8) + tlv_header(8) + 3 = 19.
    assert high.ip_fields[0].offset == 8 + DPT_TLV_HDR_LEN + 3
    assert high.ip_fields[1].offset == 8 + DPT_TLV_HDR_LEN + 3 + 16


def test_unknown_tlv_marked_schema_unknown():
    """A type/version not in the registry should still parse but be flagged."""
    weird = bytes([LEGACY_TYPE_HIGH, 8, 99]) + b"\x00" * 5  # bogus version 99
    parse = parse_trailer(weird)
    assert parse.fmt == "legacy"
    assert len(parse.tlvs) == 1
    assert not parse.tlvs[0].schema_known


# ── address-form classifier ────────────────────────────────────────


def test_classify_ipv4_mapped_v6_returns_v4():
    bytes16 = bytes(10) + b"\xff\xff" + b"\x08\x08\x08\x08"
    assert len(bytes16) == 16
    kind, off = classify_v6_or_v4mapped(bytes16)
    assert kind == "v4"
    assert off == 12


def test_classify_real_ipv6_returns_v6():
    bytes16 = b"\x20\x01" + b"\x0d\xb8" + b"\x00" * 12
    kind, _off = classify_v6_or_v4mapped(bytes16)
    assert kind == "v6"


def test_classify_f5_route_domain_treated_as_v4():
    """The F5 route-domain wrapper holds an IPv4 in its last 4 bytes."""
    bytes16 = (
        bytes.fromhex("262000000c10f50100")  # 9 bytes of fixed prefix; we only
        # actually need the first 10 bytes to match — Wireshark's prefix is
        # 10 bytes total.  Build it correctly:
    )
    bytes16 = bytes.fromhex("262000000c10f5010000")  # 10 bytes prefix
    bytes16 += b"\x00\x05"  # route domain id = 5
    bytes16 += b"\x0a\x00\x00\x01"  # 10.0.0.1
    assert len(bytes16) == 16
    kind, off = classify_v6_or_v4mapped(bytes16)
    assert kind == "v4"
    assert off == 12


# ── overlay loader ─────────────────────────────────────────────────


def test_load_schema_overlay_round_trips_via_toml():
    overlay = """
    [[legacy]]
    type = 99
    version = 7
    length = 12
    ip_fields = [
        { offset = 3, kind = "v4" },
    ]

    [[dpt]]
    provider = 1
    type = 99
    version = 5
    ip_fields = [
        { offset = 8, kind = "v6" },
    ]
    """
    load_schema_overlay(overlay)

    legacy_data = bytes([99, 12, 7]) + b"\x01\x02\x03\x04" + b"\x00" * 5
    parse = parse_trailer(legacy_data)
    assert parse.fmt == "legacy"
    assert parse.tlvs[0].schema_known
    assert parse.tlvs[0].ip_fields[0].offset == 3
    assert parse.tlvs[0].ip_fields[0].kind == "v4"


def test_register_legacy_schema_directly():
    register_legacy_schema(LEGACY_TYPE_LOW, 0, 7, [(5, "v4")])
    parse = parse_trailer(bytes([LEGACY_TYPE_LOW, 7, 0]) + b"\x01\x02\x03\x04")
    assert parse.tlvs[0].schema_known
    assert parse.tlvs[0].ip_fields[0].offset == 5


def test_register_dpt_schema_directly():
    register_dpt_schema(1, 42, 1, [(8, "v4")])
    payload = _dpt_tlv(1, 42, 1, b"\x10\x20\x30\x40\x00\x00")
    parse = parse_trailer(_dpt_hdr(payload))
    assert parse.tlvs[0].schema_known
