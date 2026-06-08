"""Packet/byte decoding (pcap iteration, L3/L4 parsing, HTTP/TLS peek, F5 trailer)."""

from __future__ import annotations

import ipaddress
import re
import struct
from pathlib import Path
from typing import BinaryIO

from .. import pcapng as _pcapng
from ..pcap_remap import _PCAP_MAGICS, _find_ip_offset, _ipv6_l4_locator
from ._model import Flow


def _iter_pcap_packets(path: Path):
    """Yield ``(linktype, packet_bytes)`` for every packet in *path*."""
    with path.open("rb") as fh:
        first4 = fh.read(4)
        if len(first4) < 4:
            return
        fh.seek(0)
        if _pcapng.is_pcapng_magic(first4):
            yield from _iter_pcapng(fh)
        else:
            yield from _iter_libpcap(fh)


def _iter_libpcap(fh: BinaryIO):
    header = fh.read(24)
    if len(header) != 24:
        return
    magic = struct.unpack("<I", header[:4])[0]
    if magic not in _PCAP_MAGICS:
        raise ValueError(f"pcap: unrecognised magic 0x{magic:08x}")
    endian, _ns = _PCAP_MAGICS[magic]
    linktype = struct.unpack(endian + "I", header[20:24])[0]
    rec_fmt = endian + "IIII"
    while True:
        rec_hdr = fh.read(16)
        if len(rec_hdr) == 0:
            return
        if len(rec_hdr) != 16:
            return
        _ts_sec, _ts_usec, incl_len, _orig_len = struct.unpack(rec_fmt, rec_hdr)
        body = fh.read(incl_len)
        if len(body) != incl_len:
            return
        yield linktype, body


def _iter_pcapng(fh: BinaryIO):
    interface_linktypes: list[int] = []
    for block in _pcapng.read_blocks(fh):
        if block.block_type == _pcapng.BLOCK_TYPE_SHB:
            interface_linktypes = []
        elif block.block_type == _pcapng.BLOCK_TYPE_IDB:
            interface_linktypes.append(block.linktype or 0)
        elif block.block_type == _pcapng.BLOCK_TYPE_EPB and block.packet_data is not None:
            iface_idx = block.interface_id or 0
            if iface_idx < len(interface_linktypes):
                yield interface_linktypes[iface_idx], block.packet_data


def _l4_ports_and_flags(packet: bytes, ip_off: int, is_v6: bool) -> tuple[int, int, int, int]:
    """Return ``(proto, src_port, dst_port, tcp_flags)`` or zeros if undecodable."""
    if is_v6:
        located = _ipv6_l4_locator(packet, ip_off)
        if located is None:
            return 0, 0, 0, 0
        proto, l4_off, _l4_len = located
    else:
        if ip_off + 20 > len(packet):
            return 0, 0, 0, 0
        ihl = (packet[ip_off] & 0x0F) * 4
        if ihl < 20:
            return 0, 0, 0, 0
        proto = packet[ip_off + 9]
        l4_off = ip_off + ihl
    if proto in (6, 17) and l4_off + 4 <= len(packet):
        src_port = (packet[l4_off] << 8) | packet[l4_off + 1]
        dst_port = (packet[l4_off + 2] << 8) | packet[l4_off + 3]
        flags = 0
        if proto == 6 and l4_off + 14 <= len(packet):
            flags = packet[l4_off + 13]
        return proto, src_port, dst_port, flags
    return proto, 0, 0, 0


def _ip_addrs(packet: bytes, ip_off: int, is_v6: bool) -> tuple[str, str]:
    if is_v6:
        if ip_off + 40 > len(packet):
            return "", ""
        src = str(ipaddress.IPv6Address(packet[ip_off + 8 : ip_off + 24]))
        dst = str(ipaddress.IPv6Address(packet[ip_off + 24 : ip_off + 40]))
        return src, dst
    if ip_off + 20 > len(packet):
        return "", ""
    src = str(ipaddress.IPv4Address(packet[ip_off + 12 : ip_off + 16]))
    dst = str(ipaddress.IPv4Address(packet[ip_off + 16 : ip_off + 20]))
    return src, dst


def _l4_payload(packet: bytes, ip_off: int, is_v6: bool, proto: int) -> bytes:
    """Return the L4 payload bytes (after TCP/UDP header)."""
    if is_v6:
        located = _ipv6_l4_locator(packet, ip_off)
        if located is None:
            return b""
        _, l4_off, l4_len = located
    else:
        if ip_off + 20 > len(packet):
            return b""
        ihl = (packet[ip_off] & 0x0F) * 4
        l4_off = ip_off + ihl
        total = (packet[ip_off + 2] << 8) | packet[ip_off + 3]
        l4_len = total - ihl
    if proto == 6:
        if l4_off + 13 > len(packet):
            return b""
        data_off = ((packet[l4_off + 12] >> 4) & 0xF) * 4
        start = l4_off + data_off
        end = l4_off + l4_len
    elif proto == 17:
        start = l4_off + 8
        end = l4_off + l4_len
    else:
        return b""
    end = min(end, len(packet))
    if start >= end or start < 0:
        return b""
    return bytes(packet[start:end])


_HTTP_METHODS = (
    b"GET ",
    b"POST ",
    b"PUT ",
    b"DELETE ",
    b"HEAD ",
    b"OPTIONS ",
    b"PATCH ",
    b"CONNECT ",
)
_HTTP_RESPONSE_PREFIX = b"HTTP/"


def _split_http_headers(raw: bytes) -> dict[str, str]:
    """Decode CRLF-separated HTTP headers into a lowercase-keyed dict."""
    out: dict[str, str] = {}
    for line in raw.split(b"\r\n"):
        if not line or b":" not in line:
            continue
        name, _, value = line.partition(b":")
        key = name.decode("ascii", errors="replace").strip().lower()
        if not key:
            continue
        out[key] = value.decode("utf-8", errors="replace").strip()
    return out


def _peek_http(payload: bytes) -> dict:
    """Return decoded HTTP request/response fields from a payload prefix.

    The dict contains any of: ``is_response``, ``method``, ``uri``,
    ``path``, ``query``, ``host``, ``version``, ``headers`` (lowercased
    name → value), ``status``, ``phrase``.  Empty dict if the payload
    doesn't look like an HTTP request line or status line.
    """
    if not payload:
        return {}
    if payload.startswith(_HTTP_RESPONSE_PREFIX):
        try:
            head = payload.split(b"\r\n\r\n", 1)[0]
            first_line, _, rest = head.partition(b"\r\n")
            parts = first_line.split(b" ", 2)
            ver = parts[0].decode("ascii", errors="replace")
            status = parts[1].decode("ascii", errors="replace") if len(parts) > 1 else ""
            phrase = parts[2].decode("ascii", errors="replace") if len(parts) > 2 else ""
            return {
                "is_response": True,
                "version": ver,
                "status": status,
                "phrase": phrase,
                "headers": _split_http_headers(rest),
            }
        except (ValueError, IndexError):
            return {"is_response": True}
    for method in _HTTP_METHODS:
        if payload.startswith(method):
            try:
                head = payload.split(b"\r\n\r\n", 1)[0]
                first_line, _, rest = head.partition(b"\r\n")
                parts = first_line.split(b" ", 2)
                m = parts[0].decode("ascii", errors="replace")
                u = parts[1].decode("ascii", errors="replace") if len(parts) > 1 else ""
                ver = parts[2].decode("ascii", errors="replace") if len(parts) > 2 else ""
                path, _, query = u.partition("?")
                headers = _split_http_headers(rest)
                return {
                    "is_response": False,
                    "method": m,
                    "uri": u,
                    "path": path,
                    "query": query,
                    "version": ver,
                    "host": headers.get("host", ""),
                    "headers": headers,
                }
            except (ValueError, IndexError):
                return {}
    return {}


def _peek_tls_clienthello(payload: bytes) -> tuple[bool, str, str]:
    """Return ``(is_clienthello, version_text, sni)`` from a TLS ClientHello."""
    # TLS record: type(1)=22(handshake) + version(2) + length(2) + body...
    # Handshake: type(1)=1(client_hello) + length(3) + version(2) + random(32) + ...
    if len(payload) < 11 or payload[0] != 22:
        return False, "", ""
    if payload[5] != 1:
        return False, "", ""
    # Parse out the legacy_version inside ClientHello, then SNI.
    try:
        # offset 9..11 = client_hello legacy_version
        ver_major = payload[9]
        ver_minor = payload[10]
        version_text = {(3, 1): "TLS1.0", (3, 2): "TLS1.1", (3, 3): "TLS1.2", (3, 4): "TLS1.3"}.get(
            (ver_major, ver_minor), f"0x{ver_major:02x}{ver_minor:02x}"
        )
        # Skip random (32 bytes) -> offset 11+32 = 43
        cur = 43
        if cur >= len(payload):
            return True, version_text, ""
        # session_id
        sid_len = payload[cur]
        cur += 1 + sid_len
        if cur + 2 > len(payload):
            return True, version_text, ""
        cs_len = (payload[cur] << 8) | payload[cur + 1]
        cur += 2 + cs_len
        if cur >= len(payload):
            return True, version_text, ""
        cm_len = payload[cur]
        cur += 1 + cm_len
        if cur + 2 > len(payload):
            return True, version_text, ""
        ext_total = (payload[cur] << 8) | payload[cur + 1]
        cur += 2
        ext_end = min(cur + ext_total, len(payload))
        while cur + 4 <= ext_end:
            ext_type = (payload[cur] << 8) | payload[cur + 1]
            ext_len = (payload[cur + 2] << 8) | payload[cur + 3]
            cur += 4
            if cur + ext_len > ext_end:
                break
            if ext_type == 0:  # server_name
                # SNI list: list_len(2) + name_type(1) + name_len(2) + name
                if ext_len >= 5:
                    name_type = payload[cur + 2]
                    name_len = (payload[cur + 3] << 8) | payload[cur + 4]
                    if name_type == 0 and cur + 5 + name_len <= cur + ext_len:
                        sni = payload[cur + 5 : cur + 5 + name_len].decode(
                            "ascii", errors="replace"
                        )
                        return True, version_text, sni
            cur += ext_len
        return True, version_text, ""
    except (IndexError, ValueError):
        return True, "", ""


# Best-effort: scan an F5 ethernet trailer's LOW/MED TLV data for a printable
def _extract_peer_tuple_from_trailer(
    trailer_bytes: bytes,
) -> tuple[str, int, str, int] | None:
    """Return the peer-side ``(remote_ip, remote_port, local_ip, local_port)``
    from a HIGH TLV in the F5 ethernet trailer, or ``None``.

    HIGH TLVs are emitted on every TMM-handled packet on a
    ``-i <vlan>:np`` capture and carry the proxied peer-side 5-tuple
    so the operator can pair the front-side and back-side captures.

    Layout (see :mod:`dialects.f5.bigip.f5_trailer`):

    * Legacy HIGH v0 (length 42 from the ``[type, length, version]``
      header): peer_remote_addr at TLV-relative +6 (16-byte
      v6/v4-mapped), peer_local_addr at +22, peer_remote_port at +38,
      peer_local_port at +40.
    * DPT NOISE HIGH v1 (after the 8-byte DPT TLV header):
      peer_remote_addr at +11, peer_local_addr at +27,
      peer_remote_port at +43, peer_local_port at +45.
    """
    import struct as _s

    from ..f5_trailer import (
        DPT_HDR_LEN,
        DPT_HDR_MAGIC,
        DPT_PROVIDER_NOISE,
        DPT_TLV_HDR_LEN,
        LEGACY_TYPE_HIGH,
        looks_ipv4_mapped,
    )

    def _ip_of(sixteen: bytes) -> str:
        if len(sixteen) != 16:
            return ""
        try:
            if looks_ipv4_mapped(sixteen):
                return str(ipaddress.IPv4Address(sixteen[12:16]))
            return str(ipaddress.IPv6Address(sixteen))
        except (ValueError, OSError):
            return ""

    n = len(trailer_bytes)
    if n < 3:
        return None

    # DPT format: 4-byte magic, then a 4-byte envelope, then DPT TLVs.
    if n >= DPT_HDR_LEN and _s.unpack(">I", trailer_bytes[:4])[0] == DPT_HDR_MAGIC:
        total_len = _s.unpack(">H", trailer_bytes[4:6])[0]
        end = min(n, total_len)
        pos = DPT_HDR_LEN
        # NOISE/HIGH v1 layout: [provider:2][type:2][length:2][version:2]
        # then ipproto(1) + vlan(2) + peer_remote(16) + peer_local(16) +
        # remote_port(2) + local_port(2) = 8 + 39 = 47 bytes total.
        _NOISE_HIGH_V1_MIN_LEN = 47
        while pos + DPT_TLV_HDR_LEN <= end:
            provider = _s.unpack(">H", trailer_bytes[pos : pos + 2])[0]
            type_ = _s.unpack(">H", trailer_bytes[pos + 2 : pos + 4])[0]
            length = _s.unpack(">H", trailer_bytes[pos + 4 : pos + 6])[0]
            version = _s.unpack(">H", trailer_bytes[pos + 6 : pos + 8])[0]
            if length < DPT_TLV_HDR_LEN or pos + length > end:
                break
            # Require the schema we know how to parse (NOISE / HIGH / v1)
            # AND that the offsets we're about to read fit *inside this
            # TLV's declared length* — bounds against `end` alone could
            # let a too-short HIGH TLV read into the next entry.
            if (
                provider == DPT_PROVIDER_NOISE
                and type_ == LEGACY_TYPE_HIGH
                and version == 1
                and length >= _NOISE_HIGH_V1_MIN_LEN
            ):
                r_addr = trailer_bytes[pos + 11 : pos + 27]
                l_addr = trailer_bytes[pos + 27 : pos + 43]
                r_port = (trailer_bytes[pos + 43] << 8) | trailer_bytes[pos + 44]
                l_port = (trailer_bytes[pos + 45] << 8) | trailer_bytes[pos + 46]
                r_ip = _ip_of(r_addr)
                l_ip = _ip_of(l_addr)
                if r_ip and l_ip:
                    return r_ip, r_port, l_ip, l_port
            pos += length
        return None

    # Legacy format: walk the chain looking for a HIGH v0 entry (length 42).
    pos = 0
    while pos + 3 <= n:
        type_ = trailer_bytes[pos]
        wire_length = trailer_bytes[pos + 1]
        total_length = wire_length + 2
        if total_length < 5 or pos + total_length > n:
            break
        if type_ == LEGACY_TYPE_HIGH and total_length == 42:
            r_addr = trailer_bytes[pos + 6 : pos + 22]
            l_addr = trailer_bytes[pos + 22 : pos + 38]
            r_port = (trailer_bytes[pos + 38] << 8) | trailer_bytes[pos + 39]
            l_port = (trailer_bytes[pos + 40] << 8) | trailer_bytes[pos + 41]
            r_ip = _ip_of(r_addr)
            l_ip = _ip_of(l_addr)
            if r_ip and l_ip:
                return r_ip, r_port, l_ip, l_port
        pos += total_length
    return None


def _extract_rst_cause_from_trailer(trailer_bytes: bytes) -> list[str]:
    """Pull printable ASCII RST-cause strings out of LOW/MED trailer TLVs."""
    from ..f5_trailer import LEGACY_TYPE_LOW, LEGACY_TYPE_MED, parse_trailer

    parsed = parse_trailer(trailer_bytes)
    if parsed.fmt is None:
        return []
    out: list[str] = []
    for tlv in parsed.tlvs:
        if tlv.fmt == "legacy" and tlv.type_ not in (LEGACY_TYPE_LOW, LEGACY_TYPE_MED):
            continue
        # data slice = bytes inside this entry (skipping the type/length/version
        # header); for a best-effort scan we just walk the TLV's bytes.
        data = trailer_bytes[tlv.offset : tlv.offset + tlv.length]
        # Find ASCII runs >= 7 chars; keep ones that look like RST text.
        for run in re.findall(rb"[\x20-\x7e]{7,}", data):
            text = run.decode("ascii", errors="replace").strip()
            up = text.upper()
            if "RST" in up or "RESET" in up or "CAUSE" in up:
                if text not in out:
                    out.append(text)
    return out


def extract_flows(pcap_path: Path) -> dict[tuple[str, int, str, int, int], Flow]:
    """Walk *pcap_path* and accumulate one :class:`Flow` per unique 5-tuple.

    Flows are keyed as ``(src_ip, src_port, dst_ip, dst_port, proto)``
    and not direction-merged: the caller decides which side was the
    initiator (typically the SYN-bearer for TCP).
    """
    flows: dict[tuple[str, int, str, int, int], Flow] = {}
    for linktype, packet in _iter_pcap_packets(pcap_path):
        ip_pos = _find_ip_offset(packet, linktype)
        if ip_pos is None:
            continue
        ip_off, is_v6 = ip_pos
        src_ip, dst_ip = _ip_addrs(packet, ip_off, is_v6)
        if not src_ip:
            continue
        proto, sp, dp, tcp_flags = _l4_ports_and_flags(packet, ip_off, is_v6)
        if proto == 0:
            continue
        key = (src_ip, sp, dst_ip, dp, proto)
        flow = flows.get(key)
        if flow is None:
            flow = Flow(src_ip=src_ip, src_port=sp, dst_ip=dst_ip, dst_port=dp, proto=proto)
            flows[key] = flow
        flow.packets += 1
        flow.bytes_total += len(packet)
        # Compute trailer offset once; reused for peer-tuple and reset-cause.
        if is_v6:
            payload_len = (packet[ip_off + 4] << 8) | packet[ip_off + 5]
            trailer_off = ip_off + 40 + payload_len
        else:
            total_len = (packet[ip_off + 2] << 8) | packet[ip_off + 3]
            trailer_off = ip_off + total_len
        trailer_bytes = bytes(packet[trailer_off:]) if 0 < trailer_off < len(packet) else b""
        if trailer_bytes and not flow.peer_remote_ip:
            peer = _extract_peer_tuple_from_trailer(trailer_bytes)
            if peer is not None:
                (
                    flow.peer_remote_ip,
                    flow.peer_remote_port,
                    flow.peer_local_ip,
                    flow.peer_local_port,
                ) = peer

        if proto == 6:
            if tcp_flags & 0x02:  # SYN
                if tcp_flags & 0x10:  # ACK -> SYN+ACK
                    flow.tcp_synack = True
                else:
                    flow.tcp_syn = True
            if tcp_flags & 0x01:
                flow.tcp_fin = True
            if tcp_flags & 0x04:  # RST
                if not flow.tcp_rst:
                    flow.tcp_rst_after_bytes = flow.bytes_total
                flow.tcp_rst = True
                flow.tcp_rst_count += 1
                if trailer_bytes:
                    for c in _extract_rst_cause_from_trailer(trailer_bytes):
                        if c not in flow.f5_reset_causes:
                            flow.f5_reset_causes.append(c)

        payload = _l4_payload(packet, ip_off, is_v6, proto)
        if payload:
            is_ch, ver, sni = _peek_tls_clienthello(payload)
            if is_ch:
                flow.tls_clienthello = True
                if ver and not flow.tls_version:
                    flow.tls_version = ver
                if sni and not flow.tls_sni:
                    flow.tls_sni = sni
            http = _peek_http(payload)
            if http:
                if http.get("is_response"):
                    flow.http_response_seen = True
                    flow.http_response_code = flow.http_response_code or http.get("status", "")
                    flow.http_response_phrase = flow.http_response_phrase or http.get("phrase", "")
                    headers = http.get("headers") or {}
                    if not flow.http_response_headers:
                        flow.http_response_headers = dict(headers)
                    flow.http_response_content_type = (
                        flow.http_response_content_type or headers.get("content-type", "")
                    )
                    flow.http_response_content_length = (
                        flow.http_response_content_length or headers.get("content-length", "")
                    )
                elif http.get("method"):
                    flow.http_request_seen = True
                    flow.http_method = flow.http_method or http.get("method", "")
                    flow.http_uri = flow.http_uri or http.get("uri", "")
                    flow.http_path = flow.http_path or http.get("path", "")
                    flow.http_query = flow.http_query or http.get("query", "")
                    flow.http_host = flow.http_host or http.get("host", "")
                    flow.http_request_version = flow.http_request_version or http.get("version", "")
                    headers = http.get("headers") or {}
                    if not flow.http_request_headers:
                        flow.http_request_headers = dict(headers)
                    flow.http_user_agent = flow.http_user_agent or headers.get("user-agent", "")
                    flow.http_cookie = flow.http_cookie or headers.get("cookie", "")
                    flow.http_referer = flow.http_referer or headers.get("referer", "")
                    flow.http_request_content_type = flow.http_request_content_type or headers.get(
                        "content-type", ""
                    )
                    flow.http_request_content_length = (
                        flow.http_request_content_length or headers.get("content-length", "")
                    )
    return flows
