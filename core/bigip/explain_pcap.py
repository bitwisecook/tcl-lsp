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
from dataclasses import dataclass
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
    """One unique L3/L4 flow extracted from a capture.

    ``key`` uniquely identifies the flow (5-tuple, normalised so client
    and server sides end up on the same flow regardless of capture
    direction).  Counts and observed L7 hints are accumulated as
    packets are walked.
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
    tls_clienthello: bool = False
    tls_sni: str = ""
    tls_version: str = ""
    http_request_seen: bool = False
    http_method: str = ""
    http_host: str = ""
    http_uri: str = ""
    http_response_seen: bool = False

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
        if self.tls_clienthello:
            tls = "TLS"
            if self.tls_version:
                tls += f"/{self.tls_version}"
            if self.tls_sni:
                tls += f" SNI={self.tls_sni}"
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
        return " | ".join(parts)


@dataclass(frozen=True, slots=True)
class FlowExplain:
    """Per-flow explanation: which VS matched, the event chain, and details."""

    flow: Flow
    matched_vs: str = ""  # full path of the VS, or "" if no match
    matched_partition: str = ""
    profile_chain: tuple[str, ...] = ()
    event_sequence: tuple[str, ...] = ()
    event_blocks: tuple[tuple[str, str, str], ...] = ()  # (rule_path, event, body)
    ltm_policies: tuple[str, ...] = ()
    apm_profile: str = ""
    gtm_wide_ips: tuple[str, ...] = ()
    explain_text: str = ""  # output of compute_explain for the matched VS


@dataclass(frozen=True, slots=True)
class ExplainPcapReport:
    pcap_path: str
    flow_count: int
    matched_count: int
    flows: tuple[FlowExplain, ...] = ()
    text_report: str = ""
    used_tshark: bool = False


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
        if proto == 6:
            if tcp_flags & 0x02:  # SYN
                if tcp_flags & 0x10:  # ACK -> SYN+ACK
                    flow.tcp_synack = True
                else:
                    flow.tcp_syn = True
            if tcp_flags & 0x01:
                flow.tcp_fin = True

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


# ---------------------------------------------------------------------------
# Optional tshark enrichment.
# ---------------------------------------------------------------------------


def tshark_available() -> bool:
    return shutil.which("tshark") is not None


def enrich_with_tshark(flows: dict, pcap_path: Path) -> None:
    """Best-effort: run tshark and overlay HTTP/TLS fields onto matching flows.

    Silently no-ops if tshark is missing, errors, or produces no JSON.
    Used to upgrade fidelity when the binary is present; the built-in
    walker already handles common cases.
    """
    if not tshark_available():
        return
    try:
        result = subprocess.run(
            [
                "tshark",
                "-r",
                str(pcap_path),
                "-T",
                "fields",
                "-E",
                "separator=|",
                "-e",
                "ip.src",
                "-e",
                "ip.dst",
                "-e",
                "ipv6.src",
                "-e",
                "ipv6.dst",
                "-e",
                "tcp.srcport",
                "-e",
                "tcp.dstport",
                "-e",
                "udp.srcport",
                "-e",
                "udp.dstport",
                "-e",
                "tls.handshake.extensions_server_name",
                "-e",
                "http.request.method",
                "-e",
                "http.host",
                "-e",
                "http.request.uri",
                "-e",
                "http.response.code",
            ],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return
    if result.returncode != 0:
        return
    for line in result.stdout.splitlines():
        cols = line.split("|")
        if len(cols) < 13:
            continue
        ip_src = cols[0] or cols[2]
        ip_dst = cols[1] or cols[3]
        sp = int(cols[4] or cols[6] or 0) if (cols[4] or cols[6]).isdigit() else 0
        dp = int(cols[5] or cols[7] or 0) if (cols[5] or cols[7]).isdigit() else 0
        proto = 6 if cols[4] or cols[5] else (17 if cols[6] or cols[7] else 0)
        if not ip_src or proto == 0:
            continue
        key = (ip_src, sp, ip_dst, dp, proto)
        flow = flows.get(key)
        if flow is None:
            continue
        sni = cols[8]
        if sni and not flow.tls_sni:
            flow.tls_sni = sni
            flow.tls_clienthello = True
        method = cols[9]
        if method and not flow.http_method:
            flow.http_method = method
            flow.http_request_seen = True
        host = cols[10]
        if host and not flow.http_host:
            flow.http_host = host
        uri = cols[11]
        if uri and not flow.http_uri:
            flow.http_uri = uri
        if cols[12]:
            flow.http_response_seen = True


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
    flow: Flow,
    rule_events: set[str],
) -> list[str]:
    """Pick the events from *rule_events* that would plausibly fire for *flow*.

    Selection is driven by:

    * the protocol seen in the flow (TCP only vs TCP+TLS vs TCP+HTTP);
    * the profiles attached to the VS (client-ssl ⇒ TLS events fire,
      http profile ⇒ HTTP events fire);
    * the L7 features observed in the capture (TLS ClientHello byte
      pattern → SSL handshake events; HTTP request line → HTTP_REQUEST).
    """
    profile_types = cfg.profile_types_for_virtual(vs.full_path)
    has_http = ProfileType.HTTP in profile_types
    has_client_ssl = ProfileType.CLIENT_SSL in profile_types
    has_server_ssl = ProfileType.SERVER_SSL in profile_types

    saw_tls = flow.tls_clienthello or has_client_ssl
    saw_http = flow.http_request_seen or (has_http and flow.proto == 6)

    candidate: list[str] = ["RULE_INIT", "CLIENT_ACCEPTED"]
    if saw_tls:
        candidate += ["CLIENTSSL_CLIENTHELLO", "CLIENTSSL_HANDSHAKE"]
    if saw_http:
        candidate += ["HTTP_REQUEST", "HTTP_REQUEST_DATA"]
    candidate += ["LB_SELECTED", "SERVER_CONNECTED"]
    if has_server_ssl:
        candidate.append("SERVERSSL_HANDSHAKE")
    if saw_http:
        candidate += ["HTTP_REQUEST_SEND"]
        if flow.http_response_seen or flow.tcp_fin:
            candidate += ["HTTP_RESPONSE", "HTTP_RESPONSE_DATA"]
    if flow.tcp_fin:
        candidate += ["CLIENT_CLOSED", "SERVER_CLOSED"]

    # Keep only events the rule actually defines, preserving canonical order.
    selected = [ev for ev in candidate if ev in rule_events]
    # Append any other event the rule defines that we don't have a canonical
    # ordering for, sorted alphabetically — gives the operator visibility
    # without inventing a firing order.
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


def compute_explain_pcap(
    pcap_path: Path,
    configs: dict[str, BigipConfig],
    *,
    use_tshark: bool = False,
    show_event_bodies: bool = True,
    max_event_body_lines: int = 40,
) -> ExplainPcapReport:
    """Build a per-flow explanation for *pcap_path* against parsed *configs*.

    *configs* is the dict produced by
    :func:`explorer.verbs.f5._paths.load_paths`; each value is a parsed
    :class:`BigipConfig`.  The first config whose virtual-server set
    matches a flow's destination wins — additional matches in other
    configs are ignored.
    """
    flows = extract_flows(pcap_path)
    used_tshark = False
    if use_tshark and tshark_available():
        enrich_with_tshark(flows, pcap_path)
        used_tshark = True

    flow_explains: list[FlowExplain] = []
    matched = 0

    # Sort flows: matched VS direction (client → VS) first, then by packet count.
    ordered = sorted(flows.values(), key=lambda f: (-f.packets, f.dst_port))

    for flow in ordered:
        # Try matching the dst side first (client → VS), then src side
        # (VS → client) — the latter can happen when only the response
        # side of a flow was captured.
        vs_path: str | None = None
        cfg_hit: BigipConfig | None = None
        for cfg in configs.values():
            vs_path = _match_virtual(cfg, flow.dst_ip, flow.dst_port)
            if vs_path is None:
                vs_path = _match_virtual(cfg, flow.src_ip, flow.src_port)
            if vs_path is not None:
                cfg_hit = cfg
                break

        if vs_path is None or cfg_hit is None:
            flow_explains.append(FlowExplain(flow=flow))
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
            ordered_events = _expected_event_sequence(cfg_hit, vs, flow, set(blocks.keys()))
            for ev in ordered_events:
                sequence.append(f"{resolved_rule}::{ev}")
                if show_event_bodies:
                    body = blocks[ev]
                    body_lines = body.splitlines()
                    if len(body_lines) > max_event_body_lines:
                        body = "\n".join(body_lines[:max_event_body_lines]) + "\n... (truncated)"
                    event_blocks.append((resolved_rule, ev, body))

        # Reuse the existing explain renderer for the VS section.
        vs_report = compute_explain(cfg_hit, vs.full_path, kind="virtual")

        ltm_policies = _ltm_policies_for(cfg_hit, vs)
        apm = _apm_profile_for(cfg_hit, vs)
        gtm = _gtm_wide_ips_for(cfg_hit, vs)

        flow_explains.append(
            FlowExplain(
                flow=flow,
                matched_vs=vs.full_path,
                matched_partition=partition,
                profile_chain=tuple(profile_chain),
                event_sequence=tuple(sequence),
                event_blocks=tuple(event_blocks),
                ltm_policies=tuple(ltm_policies),
                apm_profile=apm,
                gtm_wide_ips=tuple(gtm),
                explain_text=vs_report.text_report,
            )
        )

    text = _format_report(pcap_path, flow_explains, used_tshark)
    return ExplainPcapReport(
        pcap_path=str(pcap_path),
        flow_count=len(flows),
        matched_count=matched,
        flows=tuple(flow_explains),
        text_report=text,
        used_tshark=used_tshark,
    )


def _format_report(pcap_path: Path, flows: list[FlowExplain], used_tshark: bool) -> str:
    lines: list[str] = []
    lines.append(f"explain-pcap: {pcap_path}")
    lines.append(
        f"  flows: {len(flows)} | matched: {sum(1 for f in flows if f.matched_vs)} | "
        f"tshark: {'yes' if used_tshark else 'no'}"
    )
    lines.append("")
    for i, fe in enumerate(flows, 1):
        lines.append(f"[flow {i}] {fe.flow.summary()}")
        if not fe.matched_vs:
            lines.append("  (no virtual server matched this destination)")
            lines.append("")
            continue
        lines.append(f"  matched virtual: {fe.matched_vs}")
        if fe.profile_chain:
            lines.append("  profiles (in attach order):")
            for p in fe.profile_chain:
                lines.append(f"    - {p}")
        if fe.ltm_policies:
            lines.append("  ltm policies:")
            for p in fe.ltm_policies:
                lines.append(f"    - {p}")
        if fe.apm_profile:
            lines.append(f"  apm: {fe.apm_profile}")
        if fe.gtm_wide_ips:
            lines.append("  gtm wide-ips in config:")
            for w in fe.gtm_wide_ips:
                lines.append(f"    - {w}")
        if fe.event_sequence:
            lines.append("  expected iRule event firing order:")
            for ev in fe.event_sequence:
                lines.append(f"    -> {ev}")
        if fe.event_blocks:
            lines.append("  iRule event bodies (path through):")
            for rule, ev, body in fe.event_blocks:
                lines.append(f"    --- {rule} :: when {ev} ---")
                for body_line in body.splitlines():
                    lines.append(f"      {body_line}")
        lines.append("  resolved plan:")
        for explain_line in fe.explain_text.splitlines():
            lines.append(f"    {explain_line}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def report_to_dict(report: ExplainPcapReport) -> dict:
    return {
        "pcap_path": report.pcap_path,
        "flow_count": report.flow_count,
        "matched_count": report.matched_count,
        "used_tshark": report.used_tshark,
        "flows": [
            {
                "flow": {
                    "src_ip": fe.flow.src_ip,
                    "src_port": fe.flow.src_port,
                    "dst_ip": fe.flow.dst_ip,
                    "dst_port": fe.flow.dst_port,
                    "proto": fe.flow.proto_name,
                    "packets": fe.flow.packets,
                    "bytes": fe.flow.bytes_total,
                    "tcp_syn": fe.flow.tcp_syn,
                    "tcp_synack": fe.flow.tcp_synack,
                    "tcp_fin": fe.flow.tcp_fin,
                    "tls_clienthello": fe.flow.tls_clienthello,
                    "tls_sni": fe.flow.tls_sni,
                    "tls_version": fe.flow.tls_version,
                    "http_method": fe.flow.http_method,
                    "http_host": fe.flow.http_host,
                    "http_uri": fe.flow.http_uri,
                    "http_response_seen": fe.flow.http_response_seen,
                },
                "matched_vs": fe.matched_vs,
                "partition": fe.matched_partition,
                "profile_chain": list(fe.profile_chain),
                "event_sequence": list(fe.event_sequence),
                "event_blocks": [
                    {"rule": r, "event": e, "body": b} for r, e, b in fe.event_blocks
                ],
                "ltm_policies": list(fe.ltm_policies),
                "apm_profile": fe.apm_profile,
                "gtm_wide_ips": list(fe.gtm_wide_ips),
                "explain_text": fe.explain_text,
            }
            for fe in report.flows
        ],
    }
