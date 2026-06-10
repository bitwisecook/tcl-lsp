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
6. **Parse** the F5 Ethernet trailer (everything past
   ``IP total_length`` — what ``tcpdump -i 0.0:nnnp`` adds) using
   :mod:`dialects.f5.bigip.f5_trailer`, locate each peer-IP field by its
   schema-known offset within the TLV, and rewrite it through the
   redaction map.  Both the legacy (TMOS 9.4–13.x) and the DPT
   (TMOS 14+) trailer formats are supported.  L4 payload bytes are
   left untouched (we don't peer into application data).

When a trailer entry is recognised structurally (TLV framing parses
cleanly) but its ``(type, version)`` doesn't have a registered IP
field schema, behaviour is configurable via *unknown_policy*:

- ``"error"``  — raise; default; safest.
- ``"preserve"`` — leave the entry untouched and report the count.
- ``"sweep"``  — fall back to byte-replacement for known IPs *within
  that single TLV's data section only*.  Bounded blast radius.

PCAP format support
-------------------

- Classic libpcap (magic 0xa1b2c3d4 little-endian, 0xd4c3b2a1
  big-endian, plus the µs/ns-precision variants).
- PCAPNG (Wireshark's modern default; via :mod:`dialects.f5.bigip.pcapng`).
  Auto-detected by the SHB block-type magic; per-block round-trip
  preserves SHB / IDB / NRB / ISB and other unknown blocks
  byte-for-byte; only EPB packet bytes are touched.
- Link-layer types: Ethernet (1), Raw IPv4 (101 / 228), Raw IPv6 (229),
  Linux SLL (113), Linux SLL2 (276), F5 capture (147 / DLT_USER0;
  treated as a thin shim — the inner IP layer is searched for at
  byte 0 of the packet payload, with a small heuristic offset scan).
"""

from __future__ import annotations

import ipaddress
import struct
from dataclasses import dataclass
from typing import BinaryIO

from .f5_trailer import (
    DPT_TLV_HDR_LEN,
    classify_v6_or_v4mapped,
    parse_trailer,
)
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
    trailer_tlvs_total: int = 0
    trailer_tlvs_rewritten: int = 0
    trailer_tlvs_unknown: int = 0


class UnknownTrailerError(ValueError):
    """Raised when *unknown_policy=='error'* and an unknown TLV is found."""

    def __init__(self, fmt: str, type_: int, version: int, length: int) -> None:
        super().__init__(
            f"f5 trailer entry has no registered schema: "
            f"format={fmt} type={type_} version={version} length={length}"
        )
        self.fmt = fmt
        self.type_ = type_
        self.version = version
        self.length = length


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


# IPv6 extension-header next_header values that we walk past to find
# the actual L4 header.  ESP (50) and AH (51) wrap the L4 in cryptographic
# state; we can't recompute those checksums.  Fragment (44) is walked
# past only if this packet IS the first fragment (offset == 0).
_IPV6_EXT_HEADERS: frozenset[int] = frozenset(
    {
        0,  # Hop-by-Hop Options
        43,  # Routing
        60,  # Destination Options
        135,  # Mobility
        139,  # HIP (RFC 5201)
        140,  # Shim6
    }
)
_IPV6_FRAGMENT = 44
_IPV6_ESP = 50
_IPV6_AH = 51


def _ipv6_l4_locator(packet: bytes | bytearray, ip_off: int) -> tuple[int, int, int] | None:
    """Walk IPv6 extension headers past *ip_off* to find the L4 header.

    Returns ``(final_next_header, l4_off, l4_len)`` or ``None`` when the
    L4 isn't recompute-safe (encrypted / fragmented continuation /
    truncated capture).

    *l4_len* is the number of bytes from *l4_off* through the end of the
    IPv6 packet, i.e. ``ip_payload_length - (extension-header bytes)``.
    """
    if ip_off + 40 > len(packet):
        return None
    ip_payload_len = _u16(packet, ip_off + 4)
    next_header = packet[ip_off + 6]
    cursor = ip_off + 40  # past fixed IPv6 header
    end_of_ip = ip_off + 40 + ip_payload_len
    if end_of_ip > len(packet):
        return None  # truncated capture
    # Walk at most a few headers (RFC 8200 implicitly bounds the chain;
    # 16 is safely past anything that occurs in practice).
    for _ in range(16):
        if next_header in _IPV6_EXT_HEADERS:
            if cursor + 2 > end_of_ip:
                return None
            ext_len_bytes = (packet[cursor + 1] + 1) * 8
            if cursor + ext_len_bytes > end_of_ip:
                return None
            next_next = packet[cursor]
            cursor += ext_len_bytes
            next_header = next_next
            continue
        if next_header == _IPV6_FRAGMENT:
            # Fragment header is fixed 8 bytes: next(1) reserved(1)
            # offset+M(2) id(4).  Continuation fragments (offset != 0)
            # have no L4 header at all; even first fragments have a
            # truncated L4 segment, so we don't recompute the checksum.
            if cursor + 8 > end_of_ip:
                return None
            return None
        if next_header in (_IPV6_ESP, _IPV6_AH):
            # Encrypted / authenticated payloads — checksum lives in
            # cleartext form that we don't have.
            return None
        # Not a recognised extension header — assume L4 starts here.
        return next_header, cursor, end_of_ip - cursor
    return None  # extension chain too long; bail rather than guess


def _is_ipv4_fragment(packet: bytes | bytearray, ip_off: int) -> bool:
    """True if the IPv4 packet at *ip_off* is a fragment (any but the
    last *and* offset 0 alone — both fragmented and reassembling cases).

    The L4 checksum is meaningful only over the entire L4 segment, which
    only the first fragment fully contains and only after reassembly.
    For continuation fragments there is no L4 header at all, so we must
    not write to that offset.
    """
    flags_frag = _u16(packet, ip_off + 6)
    mf = (flags_frag >> 13) & 0x1
    frag_off = flags_frag & 0x1FFF
    return mf == 1 or frag_off != 0


def _l4_checksum(
    packet: bytearray,
    ip_off: int,
    ihl_bytes: int,
    proto: int,
    is_v6: bool,
    *,
    v6_l4_off: int | None = None,
    v6_l4_len: int | None = None,
) -> int | None:
    """Recompute and return the L4 checksum for TCP/UDP/ICMP/ICMPv6.

    Returns ``None`` when no recompute is possible (unsupported proto,
    fragmented packet, encrypted, truncated capture, or invalid header
    lengths).  For IPv6 packets with extension headers the caller
    passes the resolved L4 offset and length from
    :func:`_ipv6_l4_locator`; *proto* is the post-extension next-header.
    """
    if proto not in (6, 17, 1, 58):  # TCP, UDP, ICMP, ICMPv6
        return None
    if not is_v6 and ihl_bytes < 20:
        return None  # malformed IPv4 header
    if not is_v6 and _is_ipv4_fragment(packet, ip_off):
        return None  # cannot recompute over partial L4 segment
    if is_v6:
        if v6_l4_off is None or v6_l4_len is None:
            return None
        src = bytes(packet[ip_off + 8 : ip_off + 24])
        dst = bytes(packet[ip_off + 24 : ip_off + 40])
        l4_off = v6_l4_off
        payload_len = v6_l4_len
    else:
        src = bytes(packet[ip_off + 12 : ip_off + 16])
        dst = bytes(packet[ip_off + 16 : ip_off + 20])
        l4_off = ip_off + ihl_bytes
        total_len = _u16(packet, ip_off + 2)
        payload_len = total_len - ihl_bytes

    if payload_len <= 0:
        return None

    # Truncated capture: the packet record holds fewer bytes than the IP
    # header claims.  Computing a checksum over the truncated tail would
    # produce a wrong-but-plausible-looking value that's worse than
    # leaving the original (which the receiver will reject anyway).
    available = len(packet) - l4_off
    if available < payload_len:
        return None

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

    v6_l4_off: int | None = None
    v6_l4_len: int | None = None

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
        # Walk extension headers to find the actual L4 protocol; if
        # we hit ESP/AH/Fragment-continuation, locator returns None
        # and we'll skip the L4 cksum recompute below.
        located = _ipv6_l4_locator(packet, ip_off)
        if located is not None:
            proto, v6_l4_off, v6_l4_len = located
        else:
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
        new_l4 = _l4_checksum(
            packet,
            ip_off,
            ihl_bytes,
            proto,
            is_v6,
            v6_l4_off=v6_l4_off,
            v6_l4_len=v6_l4_len,
        )
        if new_l4 is not None:
            # L4 checksum offsets within the L4 header:
            #   TCP cksum at +16, UDP at +6, ICMPv4/ICMPv6 at +2.
            if is_v6:
                l4_base = v6_l4_off if v6_l4_off is not None else ip_off + ihl_bytes
            else:
                l4_base = ip_off + ihl_bytes
            if proto == 6:  # TCP
                cksum_off = l4_base + 16
            elif proto == 17:  # UDP
                cksum_off = l4_base + 6
            elif proto == 1:  # ICMPv4
                cksum_off = l4_base + 2
            elif proto == 58:  # ICMPv6
                cksum_off = l4_base + 2
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

    Used as the fallback for unknown TLVs when ``unknown_policy=="sweep"``.
    Replacements are length-preserving.
    """
    table = rm.reverse if reverse else rm.forward
    if not table:
        return 0

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


def _rewrite_trailer_field(
    packet: bytearray,
    abs_off: int,
    kind: str,
    rm: RedactionMap,
    *,
    reverse: bool,
) -> int:
    """Rewrite a single IP field at *abs_off*; return 1 if changed else 0."""
    from .redact_map import map_address, unmap_address

    convert = unmap_address if reverse else map_address

    if kind == "v4":
        if abs_off + 4 > len(packet):
            return 0
        addr_str = str(ipaddress.IPv4Address(bytes(packet[abs_off : abs_off + 4])))
        new = convert(rm, addr_str)
        if new != addr_str:
            packet[abs_off : abs_off + 4] = ipaddress.IPv4Address(new).packed
            return 1
        return 0

    if kind == "v6":
        if abs_off + 16 > len(packet):
            return 0
        addr_str = str(ipaddress.IPv6Address(bytes(packet[abs_off : abs_off + 16])))
        new = convert(rm, addr_str)
        if new != addr_str:
            packet[abs_off : abs_off + 16] = ipaddress.IPv6Address(new).packed
            return 1
        return 0

    if kind == "v6_or_v4mapped":
        if abs_off + 16 > len(packet):
            return 0
        sixteen = bytes(packet[abs_off : abs_off + 16])
        actual_kind, v4_off = classify_v6_or_v4mapped(sixteen)
        if actual_kind == "v4":
            return _rewrite_trailer_field(packet, abs_off + v4_off, "v4", rm, reverse=reverse)
        return _rewrite_trailer_field(packet, abs_off, "v6", rm, reverse=reverse)

    return 0


def _rewrite_trailer(
    packet: bytearray,
    trailer_off: int,
    rm: RedactionMap,
    *,
    reverse: bool,
    unknown_policy: str,
) -> tuple[int, int, int, int]:
    """Parse and rewrite the F5 trailer in ``packet[trailer_off:]``.

    Returns ``(addresses_changed, tlvs_total, tlvs_rewritten,
    tlvs_unknown)``.  Raises :class:`UnknownTrailerError` when
    ``unknown_policy == 'error'`` and an unknown TLV is encountered.
    """
    if trailer_off >= len(packet):
        return 0, 0, 0, 0

    trailer_bytes = bytes(packet[trailer_off:])
    parse = parse_trailer(trailer_bytes)
    if parse.fmt is None:
        return 0, 0, 0, 0

    addrs_changed = 0
    tlvs_rewritten = 0
    unknown = 0

    for tlv in parse.tlvs:
        if not tlv.schema_known:
            unknown += 1
            if unknown_policy == "error":
                raise UnknownTrailerError(parse.fmt, tlv.type_, tlv.version, tlv.length)
            if unknown_policy == "sweep":
                start = trailer_off + tlv.offset + _tlv_header_len(parse.fmt)
                end = trailer_off + tlv.offset + tlv.length
                tlv_changes = _byte_replace_known_ips_in_range(
                    packet, rm, reverse=reverse, start=start, end=end
                )
                if tlv_changes:
                    addrs_changed += tlv_changes
                    tlvs_rewritten += 1
            # "preserve" -> do nothing
            continue

        tlv_changes = 0
        for ip_field in tlv.ip_fields:
            tlv_changes += _rewrite_trailer_field(
                packet,
                trailer_off + ip_field.offset,
                ip_field.kind,
                rm,
                reverse=reverse,
            )
        if tlv_changes:
            addrs_changed += tlv_changes
            tlvs_rewritten += 1

    return addrs_changed, len(parse.tlvs), tlvs_rewritten, unknown


def _tlv_header_len(fmt: str) -> int:
    return DPT_TLV_HDR_LEN if fmt == "dpt" else 3


# Keyed by (id(rm), reverse, len(table)).  The length component invalidates
# the cache whenever new IPs are appended to rm.forward/rm.reverse during
# packet processing — without it, an IP first seen on packet N would be
# missing from the lookup table when packet N+1's trailer is swept, even
# though the IP layer just added it.
_PACKED_LOOKUP_MEMO: dict[tuple[int, bool, int], tuple[dict[bytes, bytes], dict[bytes, bytes]]] = {}


def _packed_lookup_for(
    rm: RedactionMap, *, reverse: bool
) -> tuple[dict[bytes, bytes], dict[bytes, bytes]]:
    table = rm.reverse if reverse else rm.forward
    key = (id(rm), reverse, len(table))
    cached = _PACKED_LOOKUP_MEMO.get(key)
    if cached is not None:
        return cached
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
    # Drop any prior entries for this (id, reverse) so the memo doesn't
    # accumulate stale tables forever during a long capture rewrite.
    for stale in list(_PACKED_LOOKUP_MEMO):
        if stale[0] == key[0] and stale[1] == key[1] and stale[2] != key[2]:
            del _PACKED_LOOKUP_MEMO[stale]
    _PACKED_LOOKUP_MEMO[key] = (v4, v6)
    return v4, v6


@dataclass(slots=True)
class _PerPacketCounts:
    addresses_rewritten: int = 0
    rewrote_anything: bool = False
    tlvs_total: int = 0
    tlvs_rewritten: int = 0
    tlvs_unknown: int = 0


# F5 FILEINFO pseudo-packet magic (Wireshark: fileinfomagic1).
# tcpdump -i 0.0 emits one of these as the first frame of a capture
# carrying CMD/VER/HOST/PLAT/PROD/SESS metadata about the BIG-IP.
# It looks like an Ethernet frame (12 zero MAC + ethertype 0x05ff)
# but the body is null-terminated ASCII strings.  We rewrite any IP
# literals in the CMD line (typically a `tcpdump host A or host B`
# filter) so the metadata stays consistent with the rest of the
# redacted capture.
_FILEINFO_MAGIC = bytes(12) + b"\x05\xff" + b"F5-Pseudo-pkt\x00"


def _rewrite_fileinfo_packet(
    packet: bytearray,
    rm: RedactionMap,
    *,
    reverse: bool,
) -> int:
    """Rewrite IP literals inside a FILEINFO pseudo-packet.

    The packet body (after the magic) is null-terminated ASCII strings
    keyed by ``CMD: ``, ``VER: ``, ``HOST: ``, etc.  Only the CMD
    string typically contains IP literals, but rewriting any string
    is safe — :func:`_apply_map` only changes bytes that exactly
    match a known real IP from the map.
    """
    from .redact_map import apply_map

    if not packet.startswith(_FILEINFO_MAGIC):
        return 0
    body_off = len(_FILEINFO_MAGIC)
    body_text = bytes(packet[body_off:]).decode("utf-8", errors="replace")
    rewritten, count = apply_map(rm, body_text, reverse=reverse)
    if count == 0:
        return 0
    rewritten_bytes = rewritten.encode("utf-8", errors="replace")
    if len(rewritten_bytes) != len(body_text.encode("utf-8", errors="replace")):
        # Length-changing rewrite — bail rather than corrupt the frame.
        return 0
    packet[body_off : body_off + len(rewritten_bytes)] = rewritten_bytes
    return count


def _rewrite_one_packet(
    packet: bytearray,
    linktype: int,
    rm: RedactionMap,
    *,
    reverse: bool,
    unknown_policy: str,
) -> _PerPacketCounts:
    """Rewrite one packet in place; return counts.  Shared by libpcap + pcapng paths."""
    counts = _PerPacketCounts()

    # FILEINFO pseudo-packet: Ethernet ethertype 0x05ff with the F5
    # capture metadata.  No IP layer; rewrite IPs in the CMD string.
    if packet.startswith(_FILEINFO_MAGIC):
        n = _rewrite_fileinfo_packet(packet, rm, reverse=reverse)
        counts.addresses_rewritten = n
        counts.rewrote_anything = bool(n)
        return counts

    ip_pos = _find_ip_offset(bytes(packet), linktype)
    if ip_pos is None:
        return counts

    ip_off, is_v6 = ip_pos
    ip_changed = _rewrite_ip_layer(packet, ip_off, is_v6, rm, reverse=reverse)

    # F5 HSB trailer lives past `IP total_length` from ip_off.  Parse it
    # as a TLV chain (legacy or DPT) and rewrite IPs at schema-known
    # offsets.  L4 payload bytes are never scanned.
    if is_v6:
        payload_len = _u16(packet, ip_off + 4)
        trailer_off = ip_off + 40 + payload_len
    else:
        total_len = _u16(packet, ip_off + 2)
        trailer_off = ip_off + total_len

    trailer_changed = 0
    if 0 < trailer_off < len(packet):
        (
            trailer_changed,
            counts.tlvs_total,
            counts.tlvs_rewritten,
            counts.tlvs_unknown,
        ) = _rewrite_trailer(
            packet,
            trailer_off,
            rm,
            reverse=reverse,
            unknown_policy=unknown_policy,
        )

    counts.addresses_rewritten = ip_changed + trailer_changed
    counts.rewrote_anything = bool(ip_changed or trailer_changed)
    return counts


def remap_pcap(
    in_fh: BinaryIO,
    out_fh: BinaryIO,
    rm: RedactionMap,
    *,
    reverse: bool = False,
    unknown_policy: str = "error",
) -> PcapRemapResult:
    """Stream-rewrite a libpcap *or* PCAPNG file from *in_fh* to *out_fh*.

    File format is detected by the first 4 bytes of magic.  PCAPNG
    captures (Wireshark's modern default) and classic libpcap (both
    endianness variants, microsecond and nanosecond precision) are
    both supported.

    *unknown_policy* controls behaviour when an F5 trailer TLV is
    structurally valid but its (type, version) has no registered IP
    field schema:

    - ``"error"``    — raise :class:`UnknownTrailerError` (default).
    - ``"preserve"`` — leave that TLV's bytes unchanged; report it.
    - ``"sweep"``    — fall back to byte-replacement of known IPs
      *within that single TLV's data section only*.

    Both file handles must be binary-mode.
    """
    if unknown_policy not in {"error", "preserve", "sweep"}:
        raise ValueError(f"unknown_policy must be error/preserve/sweep, got {unknown_policy!r}")

    # Peek at the first 4 bytes to pick the format.
    first4 = in_fh.read(4)
    if len(first4) < 4:
        raise ValueError("pcap: file too short")
    in_fh.seek(0)

    from . import pcapng as _pcapng

    if _pcapng.is_pcapng_magic(first4):
        return _remap_pcapng(in_fh, out_fh, rm, reverse=reverse, unknown_policy=unknown_policy)
    return _remap_libpcap(in_fh, out_fh, rm, reverse=reverse, unknown_policy=unknown_policy)


def _remap_libpcap(
    in_fh: BinaryIO,
    out_fh: BinaryIO,
    rm: RedactionMap,
    *,
    reverse: bool,
    unknown_policy: str,
) -> PcapRemapResult:
    """Classic libpcap rewriter (24-byte global header + 16-byte records)."""
    header = in_fh.read(24)
    if len(header) != 24:
        raise ValueError("pcap: file too short to contain a global header")
    magic = struct.unpack("<I", header[:4])[0]
    if magic not in _PCAP_MAGICS:
        raise ValueError(f"pcap: unrecognised magic 0x{magic:08x}")
    endian, _ns = _PCAP_MAGICS[magic]
    out_fh.write(header)

    linktype = struct.unpack(endian + "I", header[20:24])[0]
    rec_fmt = endian + "IIII"

    total = 0
    rewritten_packets = 0
    rewritten_addrs = 0
    tlvs_total = 0
    tlvs_rewritten = 0
    tlvs_unknown = 0

    while True:
        rec_hdr = in_fh.read(16)
        if len(rec_hdr) == 0:
            break
        if len(rec_hdr) != 16:
            raise ValueError("pcap: truncated record header")
        _ts_sec, _ts_usec, incl_len, _orig_len = struct.unpack(rec_fmt, rec_hdr)
        body = in_fh.read(incl_len)
        if len(body) != incl_len:
            raise ValueError(f"pcap: truncated packet body ({len(body)}/{incl_len})")
        total += 1

        packet = bytearray(body)
        counts = _rewrite_one_packet(
            packet, linktype, rm, reverse=reverse, unknown_policy=unknown_policy
        )
        if counts.rewrote_anything:
            rewritten_packets += 1
        rewritten_addrs += counts.addresses_rewritten
        tlvs_total += counts.tlvs_total
        tlvs_rewritten += counts.tlvs_rewritten
        tlvs_unknown += counts.tlvs_unknown

        out_fh.write(rec_hdr)
        out_fh.write(bytes(packet))

    return PcapRemapResult(
        packets_total=total,
        packets_rewritten=rewritten_packets,
        addresses_rewritten=rewritten_addrs,
        trailer_tlvs_total=tlvs_total,
        trailer_tlvs_rewritten=tlvs_rewritten,
        trailer_tlvs_unknown=tlvs_unknown,
    )


def _remap_pcapng(
    in_fh: BinaryIO,
    out_fh: BinaryIO,
    rm: RedactionMap,
    *,
    reverse: bool,
    unknown_policy: str,
) -> PcapRemapResult:
    """PCAPNG rewriter.

    Walks the block stream, mutates EPB packet data via the shared
    per-packet rewriter, and writes every block (including SHB / IDB /
    NRB / etc.) back byte-for-byte except the EPBs we touched.  Each
    EPB carries an interface_id pointing at an IDB; we track per-IDB
    link-layer types within the current section so we know how to walk
    each packet to its IP layer.
    """
    from . import pcapng as _pcapng

    total = 0
    rewritten_packets = 0
    rewritten_addrs = 0
    tlvs_total = 0
    tlvs_rewritten = 0
    tlvs_unknown = 0

    # Per-section state: list of linktypes indexed by IDB declaration order.
    interface_linktypes: list[int] = []

    for block in _pcapng.read_blocks(in_fh):
        if block.block_type == _pcapng.BLOCK_TYPE_SHB:
            interface_linktypes = []
        elif block.block_type == _pcapng.BLOCK_TYPE_IDB:
            interface_linktypes.append(block.linktype or 0)
        elif block.block_type == _pcapng.BLOCK_TYPE_EPB and block.packet_data is not None:
            total += 1
            iface_idx = block.interface_id or 0
            if iface_idx < len(interface_linktypes):
                linktype = interface_linktypes[iface_idx]
            else:
                # Malformed EPB pointing at an undeclared interface — skip.
                _pcapng.write_block(out_fh, block)
                continue

            packet = bytearray(block.packet_data)
            counts = _rewrite_one_packet(
                packet, linktype, rm, reverse=reverse, unknown_policy=unknown_policy
            )
            if counts.rewrote_anything:
                rewritten_packets += 1
            rewritten_addrs += counts.addresses_rewritten
            tlvs_total += counts.tlvs_total
            tlvs_rewritten += counts.tlvs_rewritten
            tlvs_unknown += counts.tlvs_unknown
            block.packet_data = bytes(packet)

        _pcapng.write_block(out_fh, block)

    return PcapRemapResult(
        packets_total=total,
        packets_rewritten=rewritten_packets,
        addresses_rewritten=rewritten_addrs,
        trailer_tlvs_total=tlvs_total,
        trailer_tlvs_rewritten=tlvs_rewritten,
        trailer_tlvs_unknown=tlvs_unknown,
    )
