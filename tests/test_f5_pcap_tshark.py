"""End-to-end validation of f5 pcap-remap against tshark.

Builds a byte-faithful synthetic libpcap containing both the legacy
HIGH v0 trailer (TMOS 9.4-13.x) and the DPT NOISE HIGH v1 trailer
(TMOS 14+).  Wireshark's f5ethtrailer dissector is the canonical
implementation of both formats; running ``tshark -V`` against the
input and the rewritten output proves:

1. Our schema offsets agree with Wireshark.  Before the remap, tshark
   reports the original peer-remote / peer-local IPs we crafted.
2. Our rewriter targets the correct bytes.  After the remap, tshark
   reports the *redacted* peer IPs at the same offsets.

Skipped when ``tshark`` isn't on PATH (CI runners may not have it; the
unit-level tests in ``test_f5_pcap_remap.py`` and ``test_f5_trailer.py``
still cover correctness independently).
"""

from __future__ import annotations

import io
import shutil
import struct
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.pcap_remap import remap_pcap
from core.bigip.redact_map import build_map

pytestmark = pytest.mark.skipif(shutil.which("tshark") is None, reason="tshark not on PATH")


# Offsets (within the trailer) ported from
# epan/dissectors/packet-f5ethtrailer.c.  See core/bigip/f5_trailer.py.
LEGACY_HIGH_V0_LEN = 42
DPT_HDR_MAGIC = 0xF5DEB0F5


def _ipv4_mapped(addr: str) -> bytes:
    import ipaddress

    return bytes(10) + b"\xff\xff" + ipaddress.IPv4Address(addr).packed


def _ipv4_checksum(header: bytes) -> int:
    total = 0
    for i in range(0, len(header), 2):
        total += (header[i] << 8) | header[i + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return (~total) & 0xFFFF


def _build_tcp_packet(src_ip: bytes, dst_ip: bytes) -> bytes:
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    ip_total = 20 + 20  # no payload
    ip = bytearray(
        bytes([0x45, 0x00])
        + struct.pack(">H", ip_total)
        + bytes([0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00])
        + src_ip
        + dst_ip
    )
    cksum = _ipv4_checksum(bytes(ip))
    ip[10] = (cksum >> 8) & 0xFF
    ip[11] = cksum & 0xFF
    tcp = (
        struct.pack(">HH", 12345, 80)
        + struct.pack(">II", 1, 0)
        + bytes([0x50, 0x02])
        + struct.pack(">H", 0xFFFF)
        + struct.pack(">HH", 0, 0)
    )
    return eth + bytes(ip) + tcp


def _legacy_med_v10(flow: int, peer_flow: int) -> bytes:
    """A v10 MED trailer (total 21 bytes; wire len = 19) — needed so the
    HIGH trailer has a non-zero peer_flow and renders peer IPs."""
    body = struct.pack(">II", flow, peer_flow) + b"\x00" * 10
    assert len(body) == 18
    return bytes([2, 19, 0]) + body


def _legacy_high_trailer(remote_v4: str, local_v4: str) -> bytes:
    body = (
        bytes([0x06])  # ipproto = TCP
        + struct.pack(">H", 0)  # vlan
        + _ipv4_mapped(remote_v4)
        + _ipv4_mapped(local_v4)
        + struct.pack(">H", 12345)
        + struct.pack(">H", 80)
    )
    assert len(body) == 39
    # Wire length byte = total - 2 = 40.
    return bytes([3, LEGACY_HIGH_V0_LEN - 2, 0]) + body


def _dpt_med_v4_tlv(flow: int, peer_flow: int) -> bytes:
    """DPT NOISE MED v4 — sets peer_flow so the HIGH dissector renders peer IPs."""
    body = (
        struct.pack(">QQ", flow, peer_flow)  # flow + peer_flow (8 + 8)
        + struct.pack(">II", 0, 0)  # cf_flags2 + cf_flags (4 + 4)
        + bytes(8)  # flow_type(1) + ha_unit(1) + reserved(4) + priority(1) + rstcauselen(1)
    )
    assert len(body) == 32  # F5_MEDV4_LENMIN
    return struct.pack(">HHHH", 1, 2, 8 + len(body), 4) + body


def _dpt_high_v1_tlv(remote_v4: str, local_v4: str) -> bytes:
    inner = (
        bytes([0x06])
        + struct.pack(">H", 0)
        + _ipv4_mapped(remote_v4)
        + _ipv4_mapped(local_v4)
        + struct.pack(">H", 12345)
        + struct.pack(">H", 80)
    )
    return struct.pack(">HHHH", 1, 3, 8 + len(inner), 1) + inner


def _dpt_envelope(*tlvs: bytes) -> bytes:
    """Wrap a list of NOISE TLVs in the DPT envelope header."""
    payload = b"".join(tlvs)
    return struct.pack(">IHH", DPT_HDR_MAGIC, 8 + len(payload), 1) + payload


def _dpt_high_trailer(remote_v4: str, local_v4: str) -> bytes:
    """Full DPT trailer: envelope + MED (with peer_flow) + HIGH."""
    return _dpt_envelope(
        _dpt_med_v4_tlv(flow=0x12345678, peer_flow=0xABCDEF01),
        _dpt_high_v1_tlv(remote_v4, local_v4),
    )


def _build_pcap(packets: list[bytes]) -> bytes:
    out = io.BytesIO()
    out.write(struct.pack("<IHHIIII", 0xA1B2C3D4, 2, 4, 0, 0, 65535, 1))
    for packet in packets:
        out.write(struct.pack("<IIII", 1_000_000, 0, len(packet), len(packet)))
        out.write(packet)
    return out.getvalue()


_TSHARK_PREFS = [
    "-o",
    "eth.fcs:Never",  # don't strip the last 4 bytes as a fake FCS
    "-o",
    "f5ethtrailer.pref_walk_trailer:TRUE",  # find trailers after eth padding
]


def _tshark_filter(pcap_path: Path, fields: list[str]) -> list[list[str]]:
    """Run tshark -T fields and return parsed rows."""
    cmd = [
        "tshark",
        "-r",
        str(pcap_path),
        *_TSHARK_PREFS,
        "-T",
        "fields",
        "-E",
        "separator=|",
    ]
    for f in fields:
        cmd.extend(["-e", f])
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    rows = []
    for line in proc.stdout.splitlines():
        if not line.strip():
            continue
        rows.append(line.split("|"))
    return rows


def _tshark_decodes_trailer(pcap_path: Path) -> bool:
    """Sanity check that the installed tshark version dissects f5ethtrailer."""
    cmd = ["tshark", "-r", str(pcap_path), *_TSHARK_PREFS, "-V"]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return "F5 Ethernet Trailer" in proc.stdout or "f5ethtrailer" in proc.stdout


def _read_peer_ips(pcap_path: Path) -> list[tuple[str, str]]:
    """Return list of (peer_remote, peer_local) tuples per packet, as
    tshark sees them.

    f5ethtrailer field names in tshark:
      f5ethtrailer.peer_remote_addr  (IPv4 form, when address is mapped)
      f5ethtrailer.peer_local_addr   (IPv4 form)
    For pure IPv6 the suffix is .peer_remote_ip6addr / .peer_local_ip6addr.
    """
    rows = _tshark_filter(
        pcap_path,
        [
            "f5ethtrailer.peerremoteaddr",
            "f5ethtrailer.peerlocaladdr",
        ],
    )
    out = []
    for row in rows:
        if len(row) >= 2 and (row[0] or row[1]):
            out.append((row[0], row[1]))
    return out


@pytest.fixture
def synthetic_pcap(tmp_path):
    """A 2-packet capture: one with legacy MED+HIGH, one with DPT.

    Wireshark's HIGH trailer dissector only renders peer addresses
    when ``tdata->peer_flow != 0`` (line 1655 of packet-f5ethtrailer.c).
    That field is set by a preceding MED trailer's peer_id, so for
    legacy captures we always emit MED + HIGH together.
    """
    src = b"\x01\x02\x03\x04"
    dst = b"\x05\x06\x07\x08"
    p_legacy = (
        _build_tcp_packet(src, dst)
        + _legacy_med_v10(flow=0x12345678, peer_flow=0xABCDEF01)
        + _legacy_high_trailer("9.9.9.9", "11.11.11.11")
    )
    p_dpt = _build_tcp_packet(src, dst) + _dpt_high_trailer("8.8.8.8", "12.12.12.12")
    pcap_bytes = _build_pcap([p_legacy, p_dpt])
    path = tmp_path / "synthetic.pcap"
    path.write_bytes(pcap_bytes)
    return path


def test_tshark_dissects_synthetic_legacy_and_dpt_trailers(synthetic_pcap):
    """Sanity: the installed tshark recognises our crafted trailers
    as F5 ethernet trailers."""
    if not _tshark_decodes_trailer(synthetic_pcap):
        pytest.skip("installed tshark version doesn't ship f5ethtrailer dissector")


def test_tshark_reads_original_peer_ips_before_remap(synthetic_pcap):
    """Before any remap, tshark must agree the peer IPs are what we wrote.
    This proves our schema offsets match Wireshark's dissector."""
    if not _tshark_decodes_trailer(synthetic_pcap):
        pytest.skip("installed tshark version doesn't ship f5ethtrailer dissector")
    peer = _read_peer_ips(synthetic_pcap)
    assert ("9.9.9.9", "11.11.11.11") in peer, f"got {peer!r}"
    assert ("8.8.8.8", "12.12.12.12") in peer, f"got {peer!r}"


def test_remap_rewrites_peer_ips_in_both_trailer_formats(synthetic_pcap, tmp_path):
    """The full proof: redact both trailer formats, hand the redacted
    capture to tshark, and confirm the canonical dissector sees the
    new (redacted) IPs at the same offsets."""
    if not _tshark_decodes_trailer(synthetic_pcap):
        pytest.skip("installed tshark version doesn't ship f5ethtrailer dissector")

    rm = build_map(text="9.9.9.9 11.11.11.11 8.8.8.8 12.12.12.12 1.2.3.4 5.6.7.8")
    out_path = tmp_path / "redacted.pcap"
    with synthetic_pcap.open("rb") as in_fh, out_path.open("wb") as out_fh:
        result = remap_pcap(in_fh, out_fh, rm)

    # 2 packets x (MED + HIGH) = 4 TLVs total.  Only the HIGH ones
    # carry peer IPs, so only 2 are rewritten; none are unknown.
    assert result.trailer_tlvs_total == 4
    assert result.trailer_tlvs_rewritten == 2
    assert result.trailer_tlvs_unknown == 0

    # Now ask tshark to re-dissect the rewritten capture.  The peer IPs
    # it reports must be the redacted values from the map, NOT the
    # originals.
    peer_after = _read_peer_ips(out_path)
    flat = {ip for pair in peer_after for ip in pair}
    for original in ("9.9.9.9", "11.11.11.11", "8.8.8.8", "12.12.12.12"):
        assert original not in flat, (
            f"tshark still sees original peer IP {original} after remap; "
            f"schema offsets disagree with Wireshark.  Saw: {peer_after!r}"
        )

    # Reverse direction: round-trip back through unmap and confirm
    # tshark sees the originals again.
    round_path = tmp_path / "roundtrip.pcap"
    with out_path.open("rb") as in_fh, round_path.open("wb") as out_fh:
        remap_pcap(in_fh, out_fh, rm, reverse=True)
    peer_round = _read_peer_ips(round_path)
    assert ("9.9.9.9", "11.11.11.11") in peer_round
    assert ("8.8.8.8", "12.12.12.12") in peer_round
