"""tshark/EK enrichment: decode TLS/HTTP fields tshark can see that raw bytes can't."""

from __future__ import annotations

import ipaddress
import json
import shutil
import subprocess
from pathlib import Path

from ._model import Flow


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
