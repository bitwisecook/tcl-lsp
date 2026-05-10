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
import json
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
    http_path: str = ""  # uri minus the query string
    http_query: str = ""
    http_request_version: str = ""  # "HTTP/1.1"
    http_user_agent: str = ""
    http_cookie: str = ""
    http_referer: str = ""
    http_request_content_type: str = ""
    http_request_content_length: str = ""
    http_request_headers: dict[str, str] = field(default_factory=dict)
    http_response_seen: bool = False
    http_response_code: str = ""
    http_response_phrase: str = ""
    http_response_content_type: str = ""
    http_response_content_length: str = ""
    http_response_headers: dict[str, str] = field(default_factory=dict)
    f5_reset_causes: list[str] = field(default_factory=list)
    # TLS handshake details (mostly tshark-sourced).
    tls_cert_subject: str = ""
    tls_cert_issuer: str = ""
    tls_supported_groups: str = ""  # curves / groups
    tls_signature_algos: str = ""
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
    # Per event-block: (rule_path, event, [(line, command, captured_value), ...]).
    event_annotations: tuple[tuple[str, str, tuple[tuple[str, str, str], ...]], ...] = ()
    ltm_policies: tuple[str, ...] = ()
    apm_profile: str = ""
    gtm_wide_ips: tuple[str, ...] = ()
    explain_text: str = ""
    reset_analysis: str = ""  # human-readable narrative of why connection ended
    # Outcome of running the iRule under the C-tcl test harness, when
    # ``--simulate`` is passed.  Empty/blank if simulation was disabled
    # or failed to start.
    simulated_pool: str = ""
    simulated_node: str = ""
    simulated_response_committed: bool = False
    simulated_logs: tuple[str, ...] = ()
    simulated_decisions: tuple[tuple[str, str, str], ...] = ()
    simulation_error: str = ""


@dataclass(frozen=True, slots=True)
class ExplainFlowReport:
    pcap_path: str
    flow_count: int
    session_count: int
    matched_count: int
    sessions: tuple[SessionExplain, ...] = ()
    text_report: str = ""
    used_tshark: bool = False
    keylog_path: str = ""
    tshark_filter: str = ""


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


# Field-path reference (tshark dotted name → EK lookup ``layers.<proto>``,
# field key with dots replaced by underscores).  We don't enumerate
# fields with ``-e`` any more — ``-T ek`` emits the entire dissection
# tree; :func:`_apply_packet_to_flows` and :func:`extract_flows_via_tshark`
# pull what they need by name.  Documented here as a quick lookup
# table for anyone extending the harvest:
#
#   ip.src                                    -> layers.ip.ip_src
#   ipv6.src                                  -> layers.ipv6.ipv6_src
#   tcp.srcport / tcp.dstport                 -> layers.tcp.tcp_srcport / tcp_dstport
#   tcp.flags.{syn,ack,fin,reset}             -> layers.tcp.tcp_flags_*
#   frame.len                                 -> layers.frame.frame_len
#   tls.handshake.extensions_server_name      -> layers.tls.tls_handshake_extensions_server_name
#   tls.handshake.ciphersuite                 -> layers.tls.tls_handshake_ciphersuite
#   tls.handshake.version / tls.record.version-> layers.tls.tls_handshake_version / tls_record_version
#   tls.handshake.extensions_alpn_str         -> layers.tls.tls_handshake_extensions_alpn_str
#   tls.alert_message.desc                    -> layers.tls.tls_alert_message_desc
#   tls.handshake.sig_hash_alg                -> layers.tls.tls_handshake_sig_hash_alg
#   tls.handshake.extensions_supported_group  -> layers.tls.tls_handshake_extensions_supported_group
#   x509sat.printableString                   -> layers.x509sat.x509sat_printableString
#   http.request.method / .uri / .version     -> layers.http.http_request_*
#   http.host / .user_agent / .cookie / .referer
#                                             -> layers.http.http_*
#   http.content_type / .content_length       -> layers.http.http_content_*
#   http.response.code / .phrase              -> layers.http.http_response_*
#   f5ethtrailer.rstcause.{cause,line,peer}   -> layers.f5ethtrailer.f5ethtrailer_rstcause_*
_TLS_VERSION_NAMES = {
    "0x0301": "TLS1.0",
    "0x0302": "TLS1.1",
    "0x0303": "TLS1.2",
    "0x0304": "TLS1.3",
    "0xfeff": "DTLS1.0",
    "0xfefd": "DTLS1.2",
}


def _name_tls_version(raw: str) -> str:
    """Look up a tshark-rendered TLS/DTLS version against the name table.

    tshark may emit the value in any of: ``0x0303`` / ``0x303`` /
    ``0xFEFD`` / ``"771"`` (decimal) — case- and width-normalise to
    ``0x%04x`` before lookup, falling back to the original string when
    we don't recognise it.
    """
    if not raw:
        return raw
    s = raw.strip().lower()
    try:
        if s.startswith("0x"):
            value = int(s, 16)
        else:
            value = int(s, 10)
    except ValueError:
        return raw
    key = f"0x{value:04x}"
    return _TLS_VERSION_NAMES.get(key, raw)


def _tshark_ek_cmd(pcap_path: Path, *, keylog_path: str = "", tshark_filter: str = "") -> list[str]:
    """Build a ``tshark -T ek`` command for newline-delimited JSON output.

    EK format (Elastic-Kibana) emits one JSON object per packet on its
    own line with all dissected fields under ``layers.<proto>``.  This
    avoids the ``|``-collision risk of ``-T fields`` when values
    contain delimiters (Cookies, URIs with query strings, F5 reset
    cause text), and gives the Python side a structured payload to
    parse rather than positional column slicing.

    A ``tshark_filter`` is passed as a display filter via ``-Y`` so
    only matching packets reach the dissector; this is the documented
    hook for the ``--tshark-filter`` CLI option.

    ``-l`` flushes per-packet so we don't sit on stdout buffering for
    long captures.
    """
    cmd = [
        "tshark",
        "-r",
        str(pcap_path),
        "-T",
        "ek",
        "-l",
        "--no-duplicate-keys",
    ]
    if keylog_path:
        kl = Path(keylog_path)
        if kl.is_file():
            cmd += ["-o", f"tls.keylog_file:{kl}"]
    if tshark_filter:
        cmd += ["-Y", tshark_filter]
    return cmd


def _parse_tshark_ek(stdout: str):
    """Yield each packet object emitted by ``tshark -T ek``.

    EK output alternates between one-line ``{"index": ...}`` headers
    and one-line content payloads ``{"timestamp": "...", "layers":
    {...}}``.  We skip the index lines and yield the content payloads
    as already-parsed dicts.
    """
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except (json.JSONDecodeError, ValueError):
            continue
        if isinstance(obj, dict) and "layers" in obj:
            yield obj


def _ek_field(layers: dict, layer: str, key: str) -> str:
    """Look up a field from a parsed EK ``layers`` object.

    EK collapses dots in field names to underscores within each
    layer, so ``ip.src`` lives at ``layers.ip['ip_src']``.  When the
    field is repeated tshark emits a list (with
    ``--no-duplicate-keys``); we return the first element.
    """
    layer_obj = layers.get(layer)
    if not isinstance(layer_obj, dict):
        return ""
    val = layer_obj.get(key)
    if val is None:
        return ""
    if isinstance(val, list):
        return str(val[0]) if val else ""
    return str(val)


def enrich_with_tshark(
    flows: dict, pcap_path: Path, *, keylog_path: str = "", tshark_filter: str = ""
) -> bool:
    """Run tshark and overlay decoded L7 fields onto *flows*.

    Returns True if tshark ran successfully.  Silently returns False if
    tshark is missing, errors out, or produces no parseable output.
    When *keylog_path* is set and the file exists, passes
    ``-o tls.keylog_file:<path>`` so HTTPS payloads decrypt and the
    HTTP method/URI/response-code fields populate for TLS-wrapped
    flows.  When *tshark_filter* is set it's passed as a display
    filter (``-Y <filter>``) so only matching packets contribute
    enrichment data.
    """
    if not tshark_available():
        return False
    cmd = _tshark_ek_cmd(pcap_path, keylog_path=keylog_path, tshark_filter=tshark_filter)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=120, check=False)
    except (OSError, subprocess.SubprocessError):
        return False
    if result.returncode != 0:
        return False

    for packet in _parse_tshark_ek(result.stdout):
        layers = packet.get("layers") or {}
        _apply_packet_to_flows(layers, flows)
    return True


def _apply_packet_to_flows(layers: dict, flows: dict) -> None:
    """Overlay decoded fields from one EK ``layers`` object onto *flows*.

    Field paths follow tshark's EK convention: ``layers.<proto>``
    contains underscored field names (``ip.src`` -> ``ip_src``).  We
    pick the first occurrence when tshark emits a list.  Non-existent
    fields return empty string and are silently ignored.
    """
    ip_src = _ek_field(layers, "ip", "ip_src") or _ek_field(layers, "ipv6", "ipv6_src")
    ip_dst = _ek_field(layers, "ip", "ip_dst") or _ek_field(layers, "ipv6", "ipv6_dst")
    tcp_sp = _ek_field(layers, "tcp", "tcp_srcport")
    tcp_dp = _ek_field(layers, "tcp", "tcp_dstport")
    udp_sp = _ek_field(layers, "udp", "udp_srcport")
    udp_dp = _ek_field(layers, "udp", "udp_dstport")
    if tcp_sp or tcp_dp:
        proto = 6
        sp = int(tcp_sp) if tcp_sp.isdigit() else 0
        dp = int(tcp_dp) if tcp_dp.isdigit() else 0
    elif udp_sp or udp_dp:
        proto = 17
        sp = int(udp_sp) if udp_sp.isdigit() else 0
        dp = int(udp_dp) if udp_dp.isdigit() else 0
    else:
        return
    if not ip_src:
        return
    try:
        ip_src = str(ipaddress.ip_address(ip_src))
        ip_dst = str(ipaddress.ip_address(ip_dst))
    except ValueError:
        return

    key = (ip_src, sp, ip_dst, dp, proto)
    flow = flows.get(key)
    if flow is None:
        return

    # TLS handshake
    sni = _ek_field(layers, "tls", "tls_handshake_extensions_server_name")
    if sni and not flow.tls_sni:
        flow.tls_sni = sni
        flow.tls_clienthello = True
    cipher = _ek_field(layers, "tls", "tls_handshake_ciphersuite")
    if cipher and not flow.tls_chosen_cipher:
        flow.tls_chosen_cipher = cipher
    chosen_ver = _ek_field(layers, "tls", "tls_handshake_version") or _ek_field(
        layers, "tls", "tls_record_version"
    )
    if chosen_ver and not flow.tls_chosen_version:
        flow.tls_chosen_version = _name_tls_version(chosen_ver)
    alpn = _ek_field(layers, "tls", "tls_handshake_extensions_alpn_str")
    if alpn and not flow.tls_alpn:
        flow.tls_alpn = alpn
    alert_desc = _ek_field(layers, "tls", "tls_alert_message_desc")
    if alert_desc:
        flow.tls_alert_seen = True
        flow.tls_alert_desc = alert_desc
    cert_subj = _ek_field(layers, "x509sat", "x509sat_printableString")
    if cert_subj and not flow.tls_cert_subject:
        flow.tls_cert_subject = cert_subj
    sig_alg = _ek_field(layers, "tls", "tls_handshake_sig_hash_alg")
    if sig_alg and not flow.tls_signature_algos:
        flow.tls_signature_algos = sig_alg
    supp_groups = _ek_field(layers, "tls", "tls_handshake_extensions_supported_group")
    if supp_groups and not flow.tls_supported_groups:
        flow.tls_supported_groups = supp_groups

    # HTTP request
    method = _ek_field(layers, "http", "http_request_method")
    if method and not flow.http_method:
        flow.http_method = method
        flow.http_request_seen = True
    host = _ek_field(layers, "http", "http_host")
    if host and not flow.http_host:
        flow.http_host = host
    uri = _ek_field(layers, "http", "http_request_uri") or _ek_field(
        layers, "http", "http_request_full_uri"
    )
    if uri and not flow.http_uri:
        flow.http_uri = uri
        path, _, query = uri.partition("?")
        flow.http_path = flow.http_path or path
        flow.http_query = flow.http_query or query
    http_ver = _ek_field(layers, "http", "http_request_version")
    if http_ver and not flow.http_request_version:
        flow.http_request_version = http_ver
    ua = _ek_field(layers, "http", "http_user_agent")
    if ua and not flow.http_user_agent:
        flow.http_user_agent = ua
        flow.http_request_headers.setdefault("user-agent", ua)
    cookie = _ek_field(layers, "http", "http_cookie")
    if cookie and not flow.http_cookie:
        flow.http_cookie = cookie
        flow.http_request_headers.setdefault("cookie", cookie)
    referer = _ek_field(layers, "http", "http_referer")
    if referer and not flow.http_referer:
        flow.http_referer = referer
        flow.http_request_headers.setdefault("referer", referer)
    content_type = _ek_field(layers, "http", "http_content_type")
    if content_type and not flow.http_request_content_type:
        flow.http_request_content_type = content_type
        flow.http_request_headers.setdefault("content-type", content_type)
    content_length = _ek_field(layers, "http", "http_content_length")
    if content_length and not flow.http_request_content_length:
        flow.http_request_content_length = content_length
        flow.http_request_headers.setdefault("content-length", content_length)

    # HTTP response
    code = _ek_field(layers, "http", "http_response_code")
    if code:
        flow.http_response_seen = True
        flow.http_response_code = code
    resp_phrase = _ek_field(layers, "http", "http_response_phrase")
    if resp_phrase and not flow.http_response_phrase:
        flow.http_response_phrase = resp_phrase

    # F5 reset cause TLVs (DPT trailer)
    rst_flag = _ek_field(layers, "tcp", "tcp_flags_reset")
    if rst_flag in ("1", "True"):
        flow.tcp_rst = True
    for ek_key in (
        "f5ethtrailer_rstcause_cause",
        "f5ethtrailer_rstcause_line",
        "f5ethtrailer_rstcause_peer",
    ):
        piece = _ek_field(layers, "f5ethtrailer", ek_key)
        if piece:
            tag = piece.strip()
            if tag and tag not in flow.f5_reset_causes:
                flow.f5_reset_causes.append(tag)


def extract_flows_via_tshark(
    pcap_path: Path,
    *,
    keylog_path: str = "",
    tshark_filter: str = "",
) -> dict[tuple[str, int, str, int, int], Flow]:
    """Extract flows from *pcap_path* by driving tshark with EK output.

    Used when the caller passes a ``--tshark-filter`` so only packets
    matching the display filter contribute to the flow set; tshark's
    reassembly + decryption (when a keylog file is supplied) means
    HTTP/TLS fields populate even for traffic the raw walker can't
    reach (e.g. fragmented HTTP, decrypted HTTPS).

    Returns the same ``{(src, sp, dst, dp, proto): Flow}`` shape as
    :func:`extract_flows`.  Counts and TCP flag bits come from
    ``tcp.flags.*`` and ``frame.len`` in the EK payload.  F5 trailer
    peer-tuples are NOT populated by this path — :func:`pair_sessions`
    for ``:np`` captures works only with the built-in walker.

    On any tshark failure, returns an empty dict; the caller is
    expected to fall back to the built-in walker.
    """
    if not tshark_available():
        return {}
    cmd = _tshark_ek_cmd(pcap_path, keylog_path=keylog_path, tshark_filter=tshark_filter)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=120, check=False)
    except (OSError, subprocess.SubprocessError):
        return {}
    if result.returncode != 0:
        return {}

    flows: dict[tuple[str, int, str, int, int], Flow] = {}
    # Two passes over the same JSON: first establishes flow keys +
    # counts + TCP flags from the layer info; second reuses
    # _apply_packet_to_flows to overlay HTTP/TLS/RST cause.
    parsed_packets = list(_parse_tshark_ek(result.stdout))

    for packet in parsed_packets:
        layers = packet.get("layers") or {}
        ip_src = _ek_field(layers, "ip", "ip_src") or _ek_field(layers, "ipv6", "ipv6_src")
        ip_dst = _ek_field(layers, "ip", "ip_dst") or _ek_field(layers, "ipv6", "ipv6_dst")
        tcp_sp = _ek_field(layers, "tcp", "tcp_srcport")
        tcp_dp = _ek_field(layers, "tcp", "tcp_dstport")
        udp_sp = _ek_field(layers, "udp", "udp_srcport")
        udp_dp = _ek_field(layers, "udp", "udp_dstport")
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
        try:
            ip_src = str(ipaddress.ip_address(ip_src))
            ip_dst = str(ipaddress.ip_address(ip_dst))
        except ValueError:
            continue
        key = (ip_src, sp, ip_dst, dp, proto)
        flow = flows.get(key)
        if flow is None:
            flow = Flow(src_ip=ip_src, src_port=sp, dst_ip=ip_dst, dst_port=dp, proto=proto)
            flows[key] = flow

        flow.packets += 1
        frame_len = _ek_field(layers, "frame", "frame_len")
        try:
            flow.bytes_total += int(frame_len) if frame_len else 0
        except ValueError:
            pass
        if proto == 6:
            syn = _ek_field(layers, "tcp", "tcp_flags_syn") in ("1", "True")
            ack = _ek_field(layers, "tcp", "tcp_flags_ack") in ("1", "True")
            fin = _ek_field(layers, "tcp", "tcp_flags_fin") in ("1", "True")
            if syn and ack:
                flow.tcp_synack = True
            elif syn:
                flow.tcp_syn = True
            if fin:
                flow.tcp_fin = True

    # Second pass: HTTP / TLS / RST cause overlay.
    for packet in parsed_packets:
        layers = packet.get("layers") or {}
        _apply_packet_to_flows(layers, flows)
    return flows


# ---------------------------------------------------------------------------
# Config matching.
# ---------------------------------------------------------------------------


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
        parsed = _parse_destination(vs.destination)
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
    for pref in vs.profiles:
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


def _profiles_for_orchestrator(cfg: BigipConfig, vs: BigipVirtualServer) -> list[str]:
    """Return the orchestrator profile labels (TCP / CLIENTSSL / HTTP / …) for *vs*."""
    types = cfg.profile_types_for_virtual(vs.full_path)
    out: list[str] = []
    if ProfileType.TCP in types or vs.profiles:
        out.append("TCP")
    if ProfileType.UDP in types:
        out.append("UDP")
    if ProfileType.CLIENT_SSL in types:
        out.append("CLIENTSSL")
    if ProfileType.SERVER_SSL in types:
        out.append("SERVERSSL")
    if ProfileType.HTTP in types:
        out.append("HTTP")
    return out or ["TCP"]


def _simulate_irule_for_session(
    cfg: BigipConfig,
    vs: BigipVirtualServer,
    session: Session,
) -> dict:
    """Run the matched VS's iRule under the C-tcl orchestrator with captured state.

    Returns a dict with keys: ``pool``, ``node``, ``response_committed``,
    ``logs`` (list of preformatted strings), ``decisions`` (list of
    ``(category, action, value)``), ``error`` (str, empty on success).

    Best-effort — any exception starting the orchestrator or running
    the request is captured into ``error`` and returned, so the caller
    can still emit static analysis even when the simulation backend is
    unavailable (e.g. no `tclsh` on PATH).
    """
    import asyncio

    out: dict = {
        "pool": "",
        "node": "",
        "response_committed": False,
        "logs": [],
        "decisions": [],
        "error": "",
    }

    rules = []
    for rref in vs.rules:
        resolved = cfg.resolve_rule(rref) or rref
        rule = cfg.rules.get(resolved)
        if rule is not None:
            rules.append(rule.source)
    if not rules:
        out["error"] = "no iRules attached to VS — nothing to simulate"
        return out

    front = session.front.client
    headers: dict[str, str] = dict(front.http_request_headers)
    method = front.http_method or "GET"
    uri = front.http_uri or "/"
    host = front.http_host or ""
    sni = front.tls_sni or ""

    profiles = _profiles_for_orchestrator(cfg, vs)

    async def _run() -> dict:
        try:
            from core.irule_test import IruleTestSession
        except ImportError as exc:
            return {**out, "error": f"irule_test framework unavailable: {exc}"}

        try:
            sess = IruleTestSession(profiles=profiles)
        except Exception as exc:  # pragma: no cover - defensive
            return {**out, "error": f"cannot construct IruleTestSession: {exc}"}

        try:
            async with sess:
                for source in rules:
                    sess.load_irule(source)
                # Add every pool referenced by the VS (default + any
                # ``pool foo`` calls inside iRules will resolve through
                # this set).
                for pname, pool_obj in cfg.pools.items():
                    members = [m.name.rsplit("/", 1)[-1] for m in pool_obj.members if m.name]
                    if members:
                        await sess.add_pool(pname.rsplit("/", 1)[-1], members)
                if vs.pool:
                    resolved_pool = cfg.resolve_pool(vs.pool)
                    if resolved_pool is not None:
                        members = [
                            m.name.rsplit("/", 1)[-1]
                            for m in cfg.pools[resolved_pool].members
                            if m.name
                        ]
                        if members:
                            await sess.add_pool(resolved_pool.rsplit("/", 1)[-1], members)
                if "HTTP" in profiles:
                    result = await sess.run_http_request(
                        method=method, uri=uri, host=host, headers=headers or None, sni=sni
                    )
                else:
                    # Non-HTTP: just fire CLIENT_ACCEPTED to exercise
                    # any L4 logic the iRule has.
                    await sess.fire_event("CLIENT_ACCEPTED")
                    result = None
        except Exception as exc:  # pragma: no cover - subprocess failure path
            return {**out, "error": f"orchestrator failure: {exc}"}

        if result is None:
            return out

        decisions: list[tuple[str, str, str]] = []
        for d in result.decisions or []:
            if isinstance(d, (list, tuple)) and len(d) >= 2:
                cat = str(d[0])
                act = str(d[1])
                val = ""
                if len(d) > 2:
                    val = " ".join(
                        str(x) for x in (d[2] if isinstance(d[2], (list, tuple)) else [d[2]])
                    )
                decisions.append((cat, act, val))
        log_lines: list[str] = []
        for entry in result.logs or []:
            if isinstance(entry, (list, tuple)):
                log_lines.append(" | ".join(str(x) for x in entry))
            else:
                log_lines.append(str(entry))

        return {
            "pool": result.pool_selected or "",
            "node": result.node_selected or "",
            "response_committed": bool(result.http_response_committed),
            "logs": log_lines,
            "decisions": decisions,
            "error": "",
        }

    try:
        return asyncio.run(_run())
    except RuntimeError as exc:
        # Already inside an event loop (rare from CLI but possible from MCP).
        return {**out, "error": f"asyncio: {exc}"}


def compute_explain_flow(
    pcap_path: Path,
    configs: dict[str, BigipConfig],
    *,
    use_tshark: bool = False,
    keylog_path: str = "",
    tshark_filter: str = "",
    show_event_bodies: bool = True,
    max_event_body_lines: int = 40,
    simulate: bool = False,
) -> ExplainFlowReport:
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
        for pref in vs.profiles:
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
        if simulate:
            sim = _simulate_irule_for_session(cfg_hit, vs, session)
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
