"""``f5 explain-pcap`` core: trace each flow in a PCAP through the BIG-IP config.

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

1.  An always-available built-in walker (:mod:`core.bigip.pcap_remap`'s
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
:func:`core.bigip.parser.parse_bigip_conf` already produced.
"""

from __future__ import annotations

import ipaddress
import re
import shutil
import struct
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

from . import pcapng as _pcapng
from .explain import compute_explain
from .model import BigipConfig, BigipVirtualServer, ProfileType
from .pcap_remap import (
    _PCAP_MAGICS,
    _find_ip_offset,
    _ipv6_l4_locator,
)


@dataclass(slots=True)
class Flow:
    """One unique unidirectional L3/L4 flow extracted from a capture.

    Flows are keyed by exact 5-tuple ``(src_ip, src_port, dst_ip,
    dst_port, proto)`` — the two halves of a TCP connection occupy two
    flow entries that :func:`pair_connections` later joins into a single
    :class:`Connection`.  Each flow accumulates counts plus L7 hints
    observed on its direction (TLS ClientHello vs ServerHello, HTTP
    request line vs response status line) and any TCP RST + F5 reset
    cause information attached to the trailer.
    """

    src_ip: str
    src_port: int
    dst_ip: str
    dst_port: int
    proto: int  # IP protocol number (6 TCP, 17 UDP, 1 ICMP, 58 ICMPv6)
    packets: int = 0
    bytes_total: int = 0
    tcp_syn: bool = False
    tcp_synack: bool = False
    tcp_fin: bool = False
    tcp_rst: bool = False
    tcp_rst_count: int = 0
    tcp_rst_after_bytes: int = 0  # bytes seen on this side before the first RST
    tls_clienthello: bool = False
    tls_sni: str = ""
    tls_version: str = ""  # legacy version inside ClientHello
    tls_chosen_version: str = ""  # version negotiated (from tshark)
    tls_chosen_cipher: str = ""  # ciphersuite chosen (from tshark)
    tls_alpn: str = ""  # ALPN protocol selected
    tls_alert_seen: bool = False
    tls_alert_desc: str = ""
    http_request_seen: bool = False
    http_method: str = ""
    http_host: str = ""
    http_uri: str = ""
    http_response_seen: bool = False
    http_response_code: str = ""  # last response code observed (from tshark)
    f5_reset_causes: list[str] = field(default_factory=list)  # decoded RST cause strings
    # F5 trailer peer-side info (populated for `tcpdump -i <vlan>:np` captures
    # where each packet carries the proxied peer-side 5-tuple in the trailer).
    peer_remote_ip: str = ""
    peer_remote_port: int = 0
    peer_local_ip: str = ""
    peer_local_port: int = 0

    @property
    def key(self) -> tuple[str, int, str, int, int]:
        return (self.src_ip, self.src_port, self.dst_ip, self.dst_port, self.proto)

    @property
    def proto_name(self) -> str:
        return {6: "tcp", 17: "udp", 1: "icmp", 58: "icmpv6"}.get(self.proto, str(self.proto))

    def summary(self) -> str:
        parts = [
            f"{self.src_ip}:{self.src_port} -> {self.dst_ip}:{self.dst_port}",
            self.proto_name,
            f"{self.packets} pkt",
        ]
        if self.tcp_syn:
            parts.append("SYN")
        if self.tcp_synack:
            parts.append("SYN-ACK")
        if self.tcp_rst:
            parts.append(f"RST x{self.tcp_rst_count}")
        if self.tls_clienthello or self.tls_chosen_version:
            tls = "TLS"
            if self.tls_chosen_version:
                tls += f"/{self.tls_chosen_version}"
            elif self.tls_version:
                tls += f"/{self.tls_version}"
            if self.tls_sni:
                tls += f" SNI={self.tls_sni}"
            if self.tls_chosen_cipher:
                tls += f" cipher={self.tls_chosen_cipher}"
            if self.tls_alpn:
                tls += f" alpn={self.tls_alpn}"
            parts.append(tls)
        if self.http_request_seen:
            http = "HTTP"
            if self.http_method:
                http += f" {self.http_method}"
            if self.http_host:
                http += f" Host={self.http_host}"
            if self.http_uri:
                http += f" {self.http_uri}"
            parts.append(http)
        if self.http_response_code:
            parts.append(f"HTTP {self.http_response_code}")
        elif self.http_response_seen:
            parts.append("HTTP response")
        if self.f5_reset_causes:
            parts.append("f5-rst:" + ";".join(self.f5_reset_causes[:2]))
        return " | ".join(parts)


@dataclass(frozen=True, slots=True)
class Connection:
    """A bidirectional TCP/UDP conversation, formed by pairing two flows.

    The ``client`` side is the SYN-bearer (or, for non-TCP and SYN-less
    captures, the first-seen direction).  ``server`` is the reverse
    5-tuple if the response side appears in the capture, otherwise
    ``None`` (one-direction capture).  The connection's ``key`` is the
    canonical ordered pair so that re-pairing is idempotent.
    """

    client: Flow
    server: Flow | None = None

    @property
    def proto(self) -> int:
        return self.client.proto

    @property
    def proto_name(self) -> str:
        return self.client.proto_name

    @property
    def reset_side(self) -> str:
        if self.client.tcp_rst and self.server and self.server.tcp_rst:
            return "both"
        if self.client.tcp_rst:
            return "client"
        if self.server and self.server.tcp_rst:
            return "server"
        return ""

    def reset_causes(self) -> list[str]:
        out: list[str] = []
        out.extend(self.client.f5_reset_causes)
        if self.server is not None:
            out.extend(self.server.f5_reset_causes)
        # de-dup while preserving order
        seen: set[str] = set()
        result: list[str] = []
        for c in out:
            if c not in seen:
                seen.add(c)
                result.append(c)
        return result

    def summary(self) -> str:
        head = (
            f"{self.client.src_ip}:{self.client.src_port} <-> "
            f"{self.client.dst_ip}:{self.client.dst_port} "
            f"({self.proto_name})"
        )
        c_pkts = self.client.packets
        s_pkts = self.server.packets if self.server is not None else 0
        head += f" | client→ {c_pkts} pkt"
        if self.server is not None:
            head += f", server→ {s_pkts} pkt"
        if self.client.tls_sni:
            head += f" | SNI={self.client.tls_sni}"
        if self.client.tls_chosen_version or self.client.tls_version:
            head += f" | TLS={self.client.tls_chosen_version or self.client.tls_version}"
        if self.client.http_method:
            head += f" | HTTP {self.client.http_method} {self.client.http_uri}"
        if self.server is not None and self.server.http_response_code:
            head += f" -> {self.server.http_response_code}"
        if self.reset_side:
            head += f" | RST({self.reset_side})"
        return head


@dataclass(frozen=True, slots=True)
class Session:
    """A logical BIG-IP-mediated conversation: client↔VIP plus VIP↔server.

    On `tcpdump -i <vlan>:np` captures, every packet that crosses TMM
    is emitted twice — once on the front (client-facing) side and once
    on the back (pool-member-facing) side, each carrying the peer
    5-tuple in its F5 ethernet trailer.  :func:`pair_sessions` groups
    those into one Session: ``front`` is the client↔VIP Connection,
    ``back`` is the TMM↔pool-member Connection (or ``None`` if the
    capture point only saw one side).

    For captures without ``:np`` (single-side capture) the Session
    holds a ``front`` Connection and ``back=None``.
    """

    front: Connection
    back: Connection | None = None

    @property
    def proto(self) -> int:
        return self.front.proto

    @property
    def proto_name(self) -> str:
        return self.front.proto_name

    def reset_side(self) -> str:
        if self.back is not None and self.back.reset_side:
            return f"server-side ({self.back.reset_side})"
        return self.front.reset_side or ""

    def reset_causes(self) -> list[str]:
        causes = list(self.front.reset_causes())
        if self.back is not None:
            for c in self.back.reset_causes():
                if c not in causes:
                    causes.append(c)
        return causes

    def summary(self) -> str:
        head = f"front: {self.front.summary()}"
        if self.back is not None:
            head += f"\n         back:  {self.back.summary()}"
        return head


@dataclass(frozen=True, slots=True)
class SessionExplain:
    """Per-session explanation: which VS matched, event chain, RST analysis."""

    session: Session
    matched_vs: str = ""
    matched_partition: str = ""
    profile_chain: tuple[str, ...] = ()
    pool_selected: str = ""  # pool path observed via the back-side flow's dst
    snat_observed: str = ""  # SNAT IP observed if back.client.src_ip != front.client.src_ip
    event_sequence: tuple[str, ...] = ()
    event_blocks: tuple[tuple[str, str, str], ...] = ()  # (rule_path, event, body)
    ltm_policies: tuple[str, ...] = ()
    apm_profile: str = ""
    gtm_wide_ips: tuple[str, ...] = ()
    explain_text: str = ""
    reset_analysis: str = ""  # human-readable narrative of why connection ended


@dataclass(frozen=True, slots=True)
class ExplainPcapReport:
    pcap_path: str
    flow_count: int
    session_count: int
    matched_count: int
    sessions: tuple[SessionExplain, ...] = ()
    text_report: str = ""
    used_tshark: bool = False
    keylog_path: str = ""


# ---------------------------------------------------------------------------
# Packet walking — extract flows from a libpcap or pcapng file.
# ---------------------------------------------------------------------------


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


_HTTP_METHODS = (b"GET ", b"POST ", b"PUT ", b"DELETE ", b"HEAD ", b"OPTIONS ", b"PATCH ", b"CONNECT ")
_HTTP_RESPONSE_PREFIX = b"HTTP/"


def _peek_http(payload: bytes) -> tuple[str, str, str, bool]:
    """Return ``(method, uri, host, is_response)`` from a request/response prefix."""
    if not payload:
        return "", "", "", False
    if payload.startswith(_HTTP_RESPONSE_PREFIX):
        return "", "", "", True
    for method in _HTTP_METHODS:
        if payload.startswith(method):
            try:
                head, _ = payload.split(b"\r\n\r\n", 1) if b"\r\n\r\n" in payload else (payload, b"")
                first_line, _, rest = head.partition(b"\r\n")
                parts = first_line.split(b" ")
                m = parts[0].decode("ascii", errors="replace")
                u = parts[1].decode("ascii", errors="replace") if len(parts) > 1 else ""
                host = ""
                for line in rest.split(b"\r\n"):
                    if line.lower().startswith(b"host:"):
                        host = line.split(b":", 1)[1].strip().decode("ascii", errors="replace")
                        break
                return m, u, host, False
            except (ValueError, IndexError):
                return "", "", "", False
    return "", "", "", False


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
                        sni = payload[cur + 5 : cur + 5 + name_len].decode("ascii", errors="replace")
                        return True, version_text, sni
            cur += ext_len
        return True, version_text, ""
    except (IndexError, ValueError):
        return True, "", ""


# Best-effort: scan an F5 ethernet trailer's LOW/MED TLV data for a printable
# RST cause string.  Wireshark surfaces these as `f5ethtrailer.rstcause.cause`
# and `.line`; we don't claim to match the dissector byte-for-byte, just to
# flag the presence of an RST cause when one is encoded.
_RST_HINT_RE = re.compile(rb"(?:RST(?:_| )?(?:cause|reason)?[: ]?)?([A-Z][A-Z0-9_/.\- ]{6,80})")


def _extract_peer_tuple_from_trailer(
    trailer_bytes: bytes,
) -> tuple[str, int, str, int] | None:
    """Return the peer-side ``(remote_ip, remote_port, local_ip, local_port)``
    from a HIGH TLV in the F5 ethernet trailer, or ``None``.

    HIGH TLVs are emitted on every TMM-handled packet on a
    ``-i <vlan>:np`` capture and carry the proxied peer-side 5-tuple
    so the operator can pair the front-side and back-side captures.

    Layout (see :mod:`core.bigip.f5_trailer`):

    * Legacy HIGH v0 (length 42 from the ``[type, length, version]``
      header): peer_remote_addr at TLV-relative +6 (16-byte
      v6/v4-mapped), peer_local_addr at +22, peer_remote_port at +38,
      peer_local_port at +40.
    * DPT NOISE HIGH v1 (after the 8-byte DPT TLV header):
      peer_remote_addr at +11, peer_local_addr at +27,
      peer_remote_port at +43, peer_local_port at +45.
    """
    import struct as _s

    from .f5_trailer import (
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
        while pos + DPT_TLV_HDR_LEN <= end:
            provider = _s.unpack(">H", trailer_bytes[pos : pos + 2])[0]
            type_ = _s.unpack(">H", trailer_bytes[pos + 2 : pos + 4])[0]
            length = _s.unpack(">H", trailer_bytes[pos + 4 : pos + 6])[0]
            if length < DPT_TLV_HDR_LEN or pos + length > end:
                break
            if (
                provider == DPT_PROVIDER_NOISE
                and type_ == LEGACY_TYPE_HIGH
                and pos + 47 <= end
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
    from .f5_trailer import LEGACY_TYPE_LOW, LEGACY_TYPE_MED, parse_trailer

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
        trailer_bytes = (
            bytes(packet[trailer_off:]) if 0 < trailer_off < len(packet) else b""
        )
        if trailer_bytes and not flow.peer_remote_ip:
            peer = _extract_peer_tuple_from_trailer(trailer_bytes)
            if peer is not None:
                flow.peer_remote_ip, flow.peer_remote_port, flow.peer_local_ip, flow.peer_local_port = peer

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
            method, uri, host, is_resp = _peek_http(payload)
            if method:
                flow.http_request_seen = True
                flow.http_method = flow.http_method or method
                flow.http_uri = flow.http_uri or uri
                flow.http_host = flow.http_host or host
            if is_resp:
                flow.http_response_seen = True
    return flows


def pair_connections(flows: dict[tuple[str, int, str, int, int], Flow]) -> list[Connection]:
    """Pair opposite-direction flows into bidirectional :class:`Connection`s.

    The SYN-bearer is preferred as the client side.  Each flow appears
    in exactly one connection; orphan flows (no reverse seen) are
    emitted as a connection with ``server=None``.
    """
    remaining = dict(flows)
    out: list[Connection] = []
    keys_in_order = sorted(
        remaining.keys(),
        key=lambda k: (
            0 if remaining[k].tcp_syn else (1 if remaining[k].tcp_synack else 2),
            -remaining[k].packets,
        ),
    )
    for key in keys_in_order:
        if key not in remaining:
            continue
        flow = remaining.pop(key)
        rev = (flow.dst_ip, flow.dst_port, flow.src_ip, flow.src_port, flow.proto)
        peer = remaining.pop(rev, None)
        if peer is not None and not flow.tcp_syn and peer.tcp_syn:
            flow, peer = peer, flow
        out.append(Connection(client=flow, server=peer))
    return out


def pair_sessions(
    flows: dict[tuple[str, int, str, int, int], Flow],
) -> list[Session]:
    """Pair flows into Connections, then pair Connections into Sessions.

    On a `tcpdump -i <vlan>:np` capture every TMM-mediated packet
    appears twice — once on the front (client-facing) side and once on
    the back (pool-member-facing) side.  The F5 ethernet trailer's
    HIGH TLV carries the proxied peer-side 5-tuple, so we can match a
    front-side Connection ``(client_ip:cport <-> vip:vport)`` with the
    back-side Connection it generated ``(snat_ip:sport <-> member_ip:mport)``
    by looking at the front-side client flow's
    ``(peer_remote_ip, peer_remote_port, peer_local_ip, peer_local_port)``
    values.

    Sessions emit only-front when no peer info is present (single-side
    capture) and only-back if a back-side Connection exists with no
    matching front (rare; usually means the front got dropped before
    the trailer was emitted).
    """
    conns = pair_connections(flows)

    # Index every connection by the 5-tuple of its client flow so we can
    # look up the back-side via the peer info on the front-side client flow.
    by_client_key: dict[tuple[str, int, str, int, int], Connection] = {}
    for c in conns:
        by_client_key[c.client.key] = c

    used: set[int] = set()
    sessions: list[Session] = []
    for c in conns:
        if id(c) in used:
            continue
        # Try treating *c* as the front side; look up the back-side
        # connection whose client 5-tuple matches the front client's
        # peer tuple.
        f = c.client
        if f.peer_remote_ip and f.peer_remote_port and f.peer_local_ip and f.peer_local_port:
            # Front-side client → VIP, peer = local→remote on the back side
            # (TMM as the proxied client, member as the proxied server).
            back_key = (
                f.peer_local_ip,
                f.peer_local_port,
                f.peer_remote_ip,
                f.peer_remote_port,
                f.proto,
            )
            back = by_client_key.get(back_key)
            if back is not None and id(back) != id(c):
                used.add(id(c))
                used.add(id(back))
                sessions.append(Session(front=c, back=back))
                continue
        used.add(id(c))
        sessions.append(Session(front=c, back=None))
    return sessions


# ---------------------------------------------------------------------------
# Optional tshark enrichment.
# ---------------------------------------------------------------------------


def tshark_available() -> bool:
    return shutil.which("tshark") is not None


_TSHARK_FIELDS = (
    "ip.src",
    "ip.dst",
    "ipv6.src",
    "ipv6.dst",
    "tcp.srcport",
    "tcp.dstport",
    "udp.srcport",
    "udp.dstport",
    "tls.handshake.extensions_server_name",
    "http.request.method",
    "http.host",
    "http.request.uri",
    "http.response.code",
    "tls.handshake.ciphersuite",
    "tls.record.version",
    "tls.handshake.version",
    "tls.handshake.extensions_alpn_str",
    "tls.alert_message.desc",
    "tcp.flags.reset",
    "f5ethtrailer.rstcause.cause",
    "f5ethtrailer.rstcause.line",
    "f5ethtrailer.rstcause.peer",
)
_TLS_VERSION_NAMES = {
    "0x0301": "TLS1.0",
    "0x0302": "TLS1.1",
    "0x0303": "TLS1.2",
    "0x0304": "TLS1.3",
    "0x00fe": "DTLS1.0",
    "0x0fefd": "DTLS1.2",
}


def enrich_with_tshark(
    flows: dict, pcap_path: Path, *, keylog_path: str = ""
) -> bool:
    """Run tshark and overlay decoded L7 fields onto *flows*.

    Returns True if tshark ran successfully.  Silently returns False if
    tshark is missing, errors out, or produces no parseable output.
    When *keylog_path* is set and the file exists, passes
    ``-o tls.keylog_file:<path>`` so HTTPS payloads decrypt and the
    HTTP method/URI/response-code fields populate for TLS-wrapped
    flows.
    """
    if not tshark_available():
        return False
    cmd = [
        "tshark",
        "-r",
        str(pcap_path),
        "-T",
        "fields",
        "-E",
        "separator=|",
        "-E",
        "occurrence=f",
    ]
    if keylog_path:
        kl = Path(keylog_path)
        if kl.is_file():
            cmd += ["-o", f"tls.keylog_file:{kl}"]
    for f in _TSHARK_FIELDS:
        cmd += ["-e", f]
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return False
    if result.returncode != 0:
        return False
    for line in result.stdout.splitlines():
        cols = line.split("|")
        if len(cols) < len(_TSHARK_FIELDS):
            continue
        ip_src = cols[0] or cols[2]
        ip_dst = cols[1] or cols[3]
        tcp_sp, tcp_dp = cols[4], cols[5]
        udp_sp, udp_dp = cols[6], cols[7]
        if tcp_sp or tcp_dp:
            proto = 6
            sp = int(tcp_sp) if tcp_sp.isdigit() else 0
            dp = int(tcp_dp) if tcp_dp.isdigit() else 0
        elif udp_sp or udp_dp:
            proto = 17
            sp = int(udp_sp) if udp_sp.isdigit() else 0
            dp = int(udp_dp) if udp_dp.isdigit() else 0
        else:
            continue
        if not ip_src:
            continue
        key = (ip_src, sp, ip_dst, dp, proto)
        flow = flows.get(key)
        if flow is None:
            continue

        sni, method, host, uri, code = cols[8], cols[9], cols[10], cols[11], cols[12]
        cipher, rec_ver, hs_ver, alpn, alert_desc = cols[13], cols[14], cols[15], cols[16], cols[17]
        rst_flag, rst_cause, rst_line, rst_peer = cols[18], cols[19], cols[20], cols[21]

        if sni and not flow.tls_sni:
            flow.tls_sni = sni
            flow.tls_clienthello = True
        if method and not flow.http_method:
            flow.http_method = method
            flow.http_request_seen = True
        if host and not flow.http_host:
            flow.http_host = host
        if uri and not flow.http_uri:
            flow.http_uri = uri
        if code:
            flow.http_response_seen = True
            flow.http_response_code = code
        if cipher and not flow.tls_chosen_cipher:
            flow.tls_chosen_cipher = cipher
        chosen_ver = hs_ver or rec_ver
        if chosen_ver and not flow.tls_chosen_version:
            flow.tls_chosen_version = _TLS_VERSION_NAMES.get(chosen_ver.lower(), chosen_ver)
        if alpn and not flow.tls_alpn:
            flow.tls_alpn = alpn
        if alert_desc:
            flow.tls_alert_seen = True
            flow.tls_alert_desc = alert_desc
        if rst_flag in ("1", "True"):
            flow.tcp_rst = True
        for piece in (rst_cause, rst_line, rst_peer):
            if piece:
                tag = piece.strip()
                if tag and tag not in flow.f5_reset_causes:
                    flow.f5_reset_causes.append(tag)
    return True


# ---------------------------------------------------------------------------
# Config matching.
# ---------------------------------------------------------------------------


_DEST_RE = re.compile(
    r"^(?P<path>/[^/\s]+/)?(?P<addr>\[[^\]]+\]|[0-9a-fA-F\.:]+)(?:%\d+)?:(?P<port>\d+|any)$"
)


def _parse_destination(dest: str) -> tuple[str, int] | None:
    """Parse a VS destination string like ``/Common/10.0.0.1:443`` or ``/p/[::1]:80``."""
    m = _DEST_RE.match(dest.strip())
    if not m:
        return None
    addr = m.group("addr").strip("[]")
    port_raw = m.group("port")
    try:
        ipaddress.ip_address(addr)
    except ValueError:
        return None
    port = 0 if port_raw == "any" else int(port_raw)
    return addr, port


def _match_virtual(cfg: BigipConfig, dst_ip: str, dst_port: int) -> str | None:
    for path, vs in cfg.virtual_servers.items():
        parsed = _parse_destination(vs.destination)
        if parsed is None:
            continue
        vs_addr, vs_port = parsed
        if vs_addr != dst_ip:
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
        (server is not None and (server.http_response_seen or server.http_response_code))
        or client.http_response_seen
    )
    saw_close = client.tcp_fin or client.tcp_rst or (
        server is not None and (server.tcp_fin or server.tcp_rst)
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
    """Return ``ltm policy`` paths whose body references the VS, or attached."""
    out: list[str] = []
    for key, obj in cfg.generic_objects.items():
        if obj.module != "ltm" or obj.object_type != "policy":
            continue
        # Best-effort: include the policy if its identifier appears in vs.profiles
        # or vs name appears in the policy's header (we don't keep policy bodies).
        out.append(obj.identifier or key)
    return out


def _gtm_wide_ips_for(cfg: BigipConfig, vs: BigipVirtualServer) -> list[str]:
    """Return GTM wide-IP identifiers in the parsed config (best effort)."""
    out: list[str] = []
    for key, obj in cfg.generic_objects.items():
        if obj.module != "gtm":
            continue
        if obj.object_type.startswith("wideip") or obj.object_type == "wideip-a" or "wideip" in obj.object_type:
            out.append(obj.identifier or key)
    return out


def _apm_profile_for(cfg: BigipConfig, vs: BigipVirtualServer) -> str:
    for pref in vs.profiles:
        resolved = cfg.resolve_profile(pref) or pref
        if "/access" in resolved or resolved.endswith("access"):
            return resolved
        # generic apm profile
        for key, obj in cfg.generic_objects.items():
            if obj.module == "apm" and (resolved == obj.identifier or resolved.endswith(obj.identifier)):
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


def compute_explain_pcap(
    pcap_path: Path,
    configs: dict[str, BigipConfig],
    *,
    use_tshark: bool = False,
    keylog_path: str = "",
    show_event_bodies: bool = True,
    max_event_body_lines: int = 40,
) -> ExplainPcapReport:
    """Build a per-session explanation for *pcap_path* against parsed *configs*.

    Flows are paired into bidirectional Connections, which are then
    paired into Sessions via the F5 ethernet trailer's peer-tuple
    (front-side client↔VIP + back-side TMM↔pool-member from the same
    `:np` capture).

    *configs* is the dict produced by
    :func:`explorer.verbs.f5._paths.load_paths`; the first config whose
    virtual-server set matches a session's destination wins.

    *keylog_path* — when set and tshark is available, passed through
    as ``-o tls.keylog_file:<path>`` so HTTPS payloads decrypt and the
    HTTP request/response fields populate for TLS-wrapped sessions.
    """
    flows = extract_flows(pcap_path)
    used_tshark = False
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
                vs_path = _match_virtual(
                    cfg, front.server.src_ip, front.server.src_port
                )
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
        for pref in vs.profiles:
            resolved = cfg_hit.resolve_profile(pref) or pref
            prof = cfg_hit.profiles.get(resolved)
            if prof is not None:
                profile_chain.append(f"{resolved} ({prof.profile_type.name.lower()})")
            else:
                profile_chain.append(f"{pref} (unresolved)")

        event_blocks: list[tuple[str, str, str]] = []
        sequence: list[str] = []
        for rref in vs.rules:
            resolved_rule = cfg_hit.resolve_rule(rref) or rref
            rule = cfg_hit.rules.get(resolved_rule)
            if rule is None:
                continue
            blocks = _extract_event_blocks(rule.source)
            ordered_events = _expected_event_sequence(
                cfg_hit, vs, front, set(blocks.keys())
            )
            for ev in ordered_events:
                sequence.append(f"{resolved_rule}::{ev}")
                if show_event_bodies:
                    body = blocks[ev]
                    body_lines = body.splitlines()
                    if len(body_lines) > max_event_body_lines:
                        body = "\n".join(body_lines[:max_event_body_lines]) + "\n... (truncated)"
                    event_blocks.append((resolved_rule, ev, body))

        vs_report = compute_explain(cfg_hit, vs.full_path, kind="virtual")
        ltm_policies = _ltm_policies_for(cfg_hit, vs)
        apm = _apm_profile_for(cfg_hit, vs)
        gtm = _gtm_wide_ips_for(cfg_hit, vs)

        # Pool selection + SNAT inferred from the back-side flow if present.
        pool_selected = ""
        snat_observed = ""
        if session.back is not None:
            bc = session.back.client
            pool_selected = f"{bc.dst_ip}:{bc.dst_port}"
            if bc.src_ip != front.client.src_ip:
                snat_observed = f"{bc.src_ip}:{bc.src_port}"

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
                ltm_policies=tuple(ltm_policies),
                apm_profile=apm,
                gtm_wide_ips=tuple(gtm),
                explain_text=vs_report.text_report,
                reset_analysis=_analyse_reset(session),
            )
        )

    text = _format_report(pcap_path, session_explains, used_tshark, keylog_path)
    return ExplainPcapReport(
        pcap_path=str(pcap_path),
        flow_count=len(flows),
        session_count=len(sessions),
        matched_count=matched,
        sessions=tuple(session_explains),
        text_report=text,
        used_tshark=used_tshark,
        keylog_path=keylog_path,
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
        "http_response_code": f.http_response_code,
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
) -> str:
    lines: list[str] = []
    lines.append(f"explain-pcap: {pcap_path}")
    lines.append(
        f"  sessions: {len(sessions)} | "
        f"matched: {sum(1 for s in sessions if s.matched_vs)} | "
        f"tshark: {'yes' if used_tshark else 'no'}"
        f"{f' | keylog: {keylog_path}' if keylog_path else ''}"
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
        if se.apm_profile:
            lines.append(f"  apm: {se.apm_profile}")
        if se.gtm_wide_ips:
            lines.append("  gtm wide-ips in config:")
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
        lines.append(f"  termination: {se.reset_analysis}")
        lines.append("  resolved plan:")
        for explain_line in se.explain_text.splitlines():
            lines.append(f"    {explain_line}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def report_to_dict(report: ExplainPcapReport) -> dict:
    return {
        "pcap_path": report.pcap_path,
        "flow_count": report.flow_count,
        "session_count": report.session_count,
        "matched_count": report.matched_count,
        "used_tshark": report.used_tshark,
        "keylog_path": report.keylog_path,
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
                "event_blocks": [
                    {"rule": r, "event": e, "body": b} for r, e, b in se.event_blocks
                ],
                "ltm_policies": list(se.ltm_policies),
                "apm_profile": se.apm_profile,
                "gtm_wide_ips": list(se.gtm_wide_ips),
                "explain_text": se.explain_text,
                "reset_analysis": se.reset_analysis,
            }
            for se in report.sessions
        ],
    }
