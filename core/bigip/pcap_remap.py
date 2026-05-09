"""Apply a :class:`RedactionMap` to a PCAP capture file.

Powers ``f5 pcap-remap``.

Strategy
--------

For each packet record:

1. Parse the link-layer header (Ethernet / Linux SLL / F5 capture).
2. Locate the IPv4 or IPv6 header inside.
3. Rewrite source and destination addresses via the supplied map.
4. Recompute the IPv4 header checksum.
5. Recompute the TCP / UDP / ICMP checksum (which covers the
   pseudo-header containing src/dst — so any address change makes
   the previous checksum invalid).
6. Sweep the **F5 trailer** — anything past ``IP total_length`` —
   for 4-byte (or 16-byte) sequences matching a known real IP in
   the map, and rewrite them in place.  The F5 HSB trailer
   (`tcpdump -i 0.0:nnnp`) embeds peer IPs and TMM context after
   the captured frame; we don't dissect its TLV structure but each
   embedded address gets the same consistent rewrite the IP header
   received.  L4 payload bytes are left untouched (we don't peer
   into application data).

The byte sweep is conservative: it only rewrites bytes that match
*exact* full-IP entries from :attr:`RedactionMap.forward` (or, in
reverse mode, :attr:`RedactionMap.reverse`).  That avoids accidentally
rewriting unrelated 4-byte numeric fields (lengths, timestamps,
sequence numbers) inside the trailer.

PCAP format support
-------------------

- Classic libpcap (magic 0xa1b2c3d4 little-endian, 0xd4c3b2a1
  big-endian, plus the µs-precision variants).
- Link-layer types: Ethernet (1), Raw IPv4 (101 / 12), Raw IPv6 (229),
  Linux SLL (113), F5 capture (147 / DLT_USER0; treated as a thin
  shim — the inner IP layer is searched for at byte 0 of the
  packet payload, with a small heuristic offset scan).

PCAPNG is not supported in this revision; an upgrade path is to
swap the file reader for a PCAPNG one and keep the rewrite layer
unchanged.
"""

from __future__ import annotations

import ipaddress
import struct
from dataclasses import dataclass
from typing import BinaryIO

from .redact_map import RedactionMap

_PCAP_MAGICS = {
    0xA1B2C3D4: ("<", False),  # little-endian, microsecond
    0xD4C3B2A1: (">", False),  # big-endian, microsecond
    0xA1B23C4D: ("<", True),  # little-endian, nanosecond
    0x4D3CB2A1: (">", True),  # big-endian, nanosecond
}

# Link-layer types we know how to walk to the IP layer.
LINKTYPE_ETHERNET = 1
LINKTYPE_RAW = 101
LINKTYPE_RAW_IPV4 = 228
LINKTYPE_RAW_IPV6 = 229
LINKTYPE_LINUX_SLL = 113
LINKTYPE_LINUX_SLL2 = 276
LINKTYPE_F5_USER0 = 147  # DLT_USER0 — F5 captures with HSB trailer


@dataclass(frozen=True, slots=True)
class PcapRemapResult:
    packets_total: int
    packets_rewritten: int
    addresses_rewritten: int


def _u16(buf: bytes | bytearray, off: int) -> int:
    return (buf[off] << 8) | buf[off + 1]


def _set_u16(buf: bytearray, off: int, value: int) -> None:
    buf[off] = (value >> 8) & 0xFF
    buf[off + 1] = value & 0xFF


def _ones_complement_sum(data: bytes) -> int:
    """Standard Internet checksum (RFC 1071) over *data*."""
    total = 0
    if len(data) % 2:
        data = data + b"\x00"
    for i in range(0, len(data), 2):
        total += (data[i] << 8) | data[i + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return (~total) & 0xFFFF


def _ipv4_header_checksum(packet: bytearray, ip_off: int, ihl_bytes: int) -> int:
    saved = (packet[ip_off + 10], packet[ip_off + 11])
    packet[ip_off + 10] = 0
    packet[ip_off + 11] = 0
    cksum = _ones_complement_sum(bytes(packet[ip_off : ip_off + ihl_bytes]))
    packet[ip_off + 10], packet[ip_off + 11] = saved
    return cksum


def _l4_checksum(
    packet: bytearray,
    ip_off: int,
    ihl_bytes: int,
    proto: int,
    is_v6: bool,
) -> int | None:
    """Recompute and return the L4 checksum for TCP/UDP/ICMP/ICMPv6."""
    if proto not in (6, 17, 1, 58):  # TCP, UDP, ICMP, ICMPv6
        return None
    if is_v6:
        src = bytes(packet[ip_off + 8 : ip_off + 24])
        dst = bytes(packet[ip_off + 24 : ip_off + 40])
        l4_off = ip_off + 40
        # Total length
        payload_len = _u16(packet, ip_off + 4)
    else:
        src = bytes(packet[ip_off + 12 : ip_off + 16])
        dst = bytes(packet[ip_off + 16 : ip_off + 20])
        l4_off = ip_off + ihl_bytes
        total_len = _u16(packet, ip_off + 2)
        payload_len = total_len - ihl_bytes

    l4_data = bytes(packet[l4_off : l4_off + payload_len])
    if not l4_data:
        return None

    if proto == 1:  # ICMPv4 — no pseudo-header
        if len(l4_data) < 4:
            return None
        cksum_off = l4_off + 2
        saved = (packet[cksum_off], packet[cksum_off + 1])
        packet[cksum_off] = 0
        packet[cksum_off + 1] = 0
        cksum = _ones_complement_sum(bytes(packet[l4_off : l4_off + payload_len]))
        packet[cksum_off], packet[cksum_off + 1] = saved
        return cksum

    # TCP / UDP / ICMPv6 use a pseudo-header.
    if proto == 58:
        # ICMPv6 pseudo-header: src + dst + length(4) + zero(3) + nh(1)
        pseudo = src + dst + struct.pack(">I", payload_len) + b"\x00\x00\x00" + bytes([proto])
        cksum_off = l4_off + 2
    elif is_v6:
        # IPv6 TCP/UDP pseudo-header
        pseudo = src + dst + struct.pack(">I", payload_len) + b"\x00\x00\x00" + bytes([proto])
        cksum_off = l4_off + 16 if proto == 6 else l4_off + 6
    else:
        # IPv4 TCP/UDP pseudo-header: src + dst + zero + proto + length
        pseudo = src + dst + bytes([0, proto]) + struct.pack(">H", payload_len)
        cksum_off = l4_off + 16 if proto == 6 else l4_off + 6

    saved = (packet[cksum_off], packet[cksum_off + 1])
    packet[cksum_off] = 0
    packet[cksum_off + 1] = 0
    sum_bytes = pseudo + bytes(packet[l4_off : l4_off + payload_len])
    cksum = _ones_complement_sum(sum_bytes)
    if proto == 17 and cksum == 0:
        cksum = 0xFFFF  # UDP-on-IPv4 zero-cksum special case
    packet[cksum_off], packet[cksum_off + 1] = saved
    return cksum


def _find_ip_offset(packet: bytes, linktype: int) -> tuple[int, bool] | None:
    """Return ``(offset, is_v6)`` for the IP header inside *packet*, or ``None``.

    F5 LINKTYPE_USER0 captures present an Ethernet frame with an
    additional F5 trailer at the end; the IP header position is the
    same as for an Ethernet frame.  When the heuristic doesn't find a
    sane IP version nibble we scan a few byte offsets — F5 captures
    sometimes prepend a small metadata shim depending on TMOS version.
    """
    if linktype == LINKTYPE_RAW or linktype == LINKTYPE_RAW_IPV4:
        return (0, False) if packet and (packet[0] >> 4) == 4 else None
    if linktype == LINKTYPE_RAW_IPV6:
        return (0, True) if packet and (packet[0] >> 4) == 6 else None

    # Ethernet (and F5 USER0): 14-byte header, ethertype at bytes 12-13.
    if linktype in (LINKTYPE_ETHERNET, LINKTYPE_F5_USER0) and len(packet) >= 14:
        ethertype = _u16(packet, 12)
        if ethertype == 0x0800:
            return (14, False)
        if ethertype == 0x86DD:
            return (14, True)
        # 802.1Q VLAN tag — ethertype is at offset 16.
        if ethertype == 0x8100 and len(packet) >= 18:
            inner = _u16(packet, 16)
            if inner == 0x0800:
                return (18, False)
            if inner == 0x86DD:
                return (18, True)

    if linktype == LINKTYPE_LINUX_SLL and len(packet) >= 16:
        proto = _u16(packet, 14)
        if proto == 0x0800:
            return (16, False)
        if proto == 0x86DD:
            return (16, True)

    if linktype == LINKTYPE_LINUX_SLL2 and len(packet) >= 20:
        proto = _u16(packet, 0)
        if proto == 0x0800:
            return (20, False)
        if proto == 0x86DD:
            return (20, True)

    # F5 USER0 fallback: scan first 64 bytes for an IP version nibble.
    if linktype == LINKTYPE_F5_USER0:
        for off in range(0, min(64, len(packet) - 1)):
            ver = packet[off] >> 4
            if ver in (4, 6):
                return (off, ver == 6)

    return None


def _rewrite_ip_layer(
    packet: bytearray, ip_off: int, is_v6: bool, rm: RedactionMap, *, reverse: bool
) -> int:
    """Rewrite IP src/dst at *ip_off*; return number of addresses changed."""
    from .redact_map import map_address, unmap_address

    convert = unmap_address if reverse else map_address
    changed = 0

    if is_v6:
        src_b = bytes(packet[ip_off + 8 : ip_off + 24])
        dst_b = bytes(packet[ip_off + 24 : ip_off + 40])
        src_str = str(ipaddress.IPv6Address(src_b))
        dst_str = str(ipaddress.IPv6Address(dst_b))
        new_src = convert(rm, src_str)
        new_dst = convert(rm, dst_str)
        if new_src != src_str:
            packet[ip_off + 8 : ip_off + 24] = ipaddress.IPv6Address(new_src).packed
            changed += 1
        if new_dst != dst_str:
            packet[ip_off + 24 : ip_off + 40] = ipaddress.IPv6Address(new_dst).packed
            changed += 1
        ihl_bytes = 40
        proto = packet[ip_off + 6]
    else:
        src_b = bytes(packet[ip_off + 12 : ip_off + 16])
        dst_b = bytes(packet[ip_off + 16 : ip_off + 20])
        src_str = str(ipaddress.IPv4Address(src_b))
        dst_str = str(ipaddress.IPv4Address(dst_b))
        new_src = convert(rm, src_str)
        new_dst = convert(rm, dst_str)
        if new_src != src_str:
            packet[ip_off + 12 : ip_off + 16] = ipaddress.IPv4Address(new_src).packed
            changed += 1
        if new_dst != dst_str:
            packet[ip_off + 16 : ip_off + 20] = ipaddress.IPv4Address(new_dst).packed
            changed += 1
        ihl_bytes = (packet[ip_off] & 0x0F) * 4
        proto = packet[ip_off + 9]

    if changed:
        # Recompute IPv4 header checksum (IPv6 has no header checksum).
        if not is_v6:
            new_cksum = _ipv4_header_checksum(packet, ip_off, ihl_bytes)
            _set_u16(packet, ip_off + 10, new_cksum)
        new_l4 = _l4_checksum(packet, ip_off, ihl_bytes, proto, is_v6)
        if new_l4 is not None:
            if proto == 6:  # TCP
                cksum_off = ip_off + ihl_bytes + 16 if is_v6 else ip_off + ihl_bytes + 16
            elif proto == 17:  # UDP
                cksum_off = ip_off + ihl_bytes + 6
            elif proto == 1:  # ICMPv4
                cksum_off = ip_off + ihl_bytes + 2
            elif proto == 58:  # ICMPv6
                cksum_off = ip_off + ihl_bytes + 2
            else:
                cksum_off = -1
            if cksum_off > 0:
                _set_u16(packet, cksum_off, new_l4)

    return changed


def _byte_replace_known_ips_in_range(
    packet: bytearray,
    rm: RedactionMap,
    *,
    reverse: bool,
    start: int,
    end: int,
) -> int:
    """Replace exact-match IP byte sequences in ``packet[start:end]``.

    Used for the F5 HSB trailer.  Replacements are length-preserving so
    embedded TLV structures stay valid.
    """
    table = rm.reverse if reverse else rm.forward
    if not table:
        return 0

    # Build packed lookup tables (built once per map; we can't slot-mutate
    # a frozen-by-default dataclass, so we use a function-local memo via
    # the map's id()).
    v4_table, v6_table = _packed_lookup_for(rm, reverse=reverse)

    changed = 0

    def _replace(width: int, lookup: dict[bytes, bytes]) -> int:
        nonlocal changed
        if not lookup:
            return 0
        local_changes = 0
        i = start
        last = end - width
        while i <= last:
            chunk = bytes(packet[i : i + width])
            replacement = lookup.get(chunk)
            if replacement is not None:
                packet[i : i + width] = replacement
                local_changes += 1
                i += width
            else:
                i += 1
        return local_changes

    changed += _replace(4, v4_table)
    changed += _replace(16, v6_table)
    return changed


_PACKED_LOOKUP_MEMO: dict[tuple[int, bool], tuple[dict[bytes, bytes], dict[bytes, bytes]]] = {}


def _packed_lookup_for(
    rm: RedactionMap, *, reverse: bool
) -> tuple[dict[bytes, bytes], dict[bytes, bytes]]:
    key = (id(rm), reverse)
    cached = _PACKED_LOOKUP_MEMO.get(key)
    if cached is not None:
        return cached
    table = rm.reverse if reverse else rm.forward
    v4: dict[bytes, bytes] = {}
    v6: dict[bytes, bytes] = {}
    for src, dst in table.items():
        try:
            addr = ipaddress.ip_address(src)
            tgt = ipaddress.ip_address(dst)
        except ValueError:
            continue
        if isinstance(addr, ipaddress.IPv4Address):
            v4[addr.packed] = tgt.packed
        else:
            v6[addr.packed] = tgt.packed
    _PACKED_LOOKUP_MEMO[key] = (v4, v6)
    return v4, v6


def remap_pcap(
    in_fh: BinaryIO, out_fh: BinaryIO, rm: RedactionMap, *, reverse: bool = False
) -> PcapRemapResult:
    """Stream-rewrite a libpcap file from *in_fh* to *out_fh*.

    Both must be binary-mode handles.  Returns a :class:`PcapRemapResult`.
    """
    header = in_fh.read(24)
    if len(header) != 24:
        raise ValueError("pcap: file too short to contain a global header")
    magic = struct.unpack("<I", header[:4])[0]
    if magic in _PCAP_MAGICS:
        endian, _ns = _PCAP_MAGICS[magic]
    else:
        magic_be = struct.unpack(">I", header[:4])[0]
        if magic_be in _PCAP_MAGICS:
            endian, _ns = _PCAP_MAGICS[magic_be]
            magic = magic_be
        else:
            raise ValueError(f"pcap: unrecognised magic 0x{magic:08x}")
    out_fh.write(header)

    linktype = struct.unpack(endian + "I", header[20:24])[0]
    rec_fmt = endian + "IIII"

    total = 0
    rewritten_packets = 0
    rewritten_addrs = 0

    while True:
        rec_hdr = in_fh.read(16)
        if len(rec_hdr) == 0:
            break
        if len(rec_hdr) != 16:
            raise ValueError("pcap: truncated record header")
        _ts_sec, _ts_usec, incl_len, orig_len = struct.unpack(rec_fmt, rec_hdr)
        body = in_fh.read(incl_len)
        if len(body) != incl_len:
            raise ValueError(f"pcap: truncated packet body ({len(body)}/{incl_len})")
        total += 1

        packet = bytearray(body)
        ip_pos = _find_ip_offset(bytes(packet), linktype)
        ip_changed = 0
        trailer_changed = 0
        if ip_pos is not None:
            ip_off, is_v6 = ip_pos
            ip_changed = _rewrite_ip_layer(packet, ip_off, is_v6, rm, reverse=reverse)
            # The F5 HSB trailer (and any padding) lives past `IP total length`
            # bytes from ip_off — i.e. after the L4 payload.  We sweep ONLY
            # that region for known IPs; the L4 payload itself is left
            # untouched so application data stays bit-identical.
            if is_v6:
                payload_len = _u16(packet, ip_off + 4)
                trailer_off = ip_off + 40 + payload_len
            else:
                total_len = _u16(packet, ip_off + 2)
                trailer_off = ip_off + total_len
            if 0 < trailer_off < len(packet):
                trailer_changed = _byte_replace_known_ips_in_range(
                    packet, rm, reverse=reverse, start=trailer_off, end=len(packet)
                )
        if ip_changed or trailer_changed:
            rewritten_packets += 1
        rewritten_addrs += ip_changed + trailer_changed

        # incl_len doesn't change because all rewrites are length-preserving.
        out_fh.write(rec_hdr)
        out_fh.write(bytes(packet))

    return PcapRemapResult(
        packets_total=total,
        packets_rewritten=rewritten_packets,
        addresses_rewritten=rewritten_addrs,
    )
