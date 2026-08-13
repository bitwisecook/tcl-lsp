// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! tshark / EK enrichment: decode TLS/HTTP fields tshark can see that the
//! raw-bytes walker can't.
//!
//! Two entry points:
//!
//! * [`enrich_with_tshark`] overlays decoded L7 fields onto flows the built-in
//!   walker already produced (the `--tshark` / `--keylog` path).
//! * [`extract_flows_via_tshark`] drives tshark as the *canonical* flow source
//!   when a display filter is supplied (the `--tshark-filter` path), so only
//!   matching packets contribute to the report.
//!
//! Both run `tshark -T ek` (Elastic-Kibana newline-delimited JSON) so values
//! that contain delimiters (Cookies, URIs with query strings, F5 reset-cause
//! text) survive intact, and parse the dissection tree under `layers.<proto>`
//! by name. Any tshark failure (missing binary, non-zero exit, no parseable
//! output) degrades gracefully: enrichment returns `false`, extraction returns
//! an empty map, and the caller falls back to the built-in walker.

use std::path::Path;
use std::process::Command;

use indexmap::IndexMap;
use serde_json::Value;

use super::model::{Flow, FlowKey};

/// Whether a `tshark` binary is on `PATH`.
#[must_use]
pub fn tshark_available() -> bool {
    which_tshark().is_some()
}

/// Locate the `tshark` binary on `PATH`.
fn which_tshark() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("tshark");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// tshark-rendered TLS/DTLS version → friendly name.
fn name_tls_version(raw: &str) -> String {
    if raw.is_empty() {
        return raw.to_owned();
    }
    let s = raw.trim().to_ascii_lowercase();
    // number-drift-ok: a tshark `tls.record.version` field, not Tcl script
    // text. tshark renders it as `0x0303` or plain decimal regardless of any
    // Tcl release, so this grammar is tshark's and must not follow a dialect.
    let value = if let Some(hex) = s.strip_prefix("0x") {
        match u32::from_str_radix(hex, 16) {
            Ok(v) => v,
            Err(_) => return raw.to_owned(),
        }
    } else {
        match s.parse::<u32>() {
            Ok(v) => v,
            Err(_) => return raw.to_owned(),
        }
    };
    match value {
        0x0301 => "TLS1.0".to_owned(),
        0x0302 => "TLS1.1".to_owned(),
        0x0303 => "TLS1.2".to_owned(),
        0x0304 => "TLS1.3".to_owned(),
        0xfeff => "DTLS1.0".to_owned(),
        0xfefd => "DTLS1.2".to_owned(),
        _ => raw.to_owned(),
    }
}

/// Build a `tshark -T ek` command for newline-delimited JSON output.
///
/// `-l` flushes per-packet; `--no-duplicate-keys` collapses a repeated field
/// to its first value. `keylog_path` (when it points at an existing file) wires
/// `-o tls.keylog_file:<path>` so HTTPS payloads decrypt; `tshark_filter` is
/// passed as a `-Y` display filter so only matching packets reach the
/// dissector.
fn tshark_ek_cmd(pcap_path: &Path, keylog_path: &str, tshark_filter: &str) -> Command {
    let mut cmd = Command::new("tshark");
    cmd.arg("-r")
        .arg(pcap_path)
        .args(["-T", "ek", "-l", "--no-duplicate-keys"]);
    if !keylog_path.is_empty() {
        let kl = Path::new(keylog_path);
        if kl.is_file() {
            cmd.arg("-o").arg(format!("tls.keylog_file:{keylog_path}"));
        }
    }
    if !tshark_filter.is_empty() {
        cmd.arg("-Y").arg(tshark_filter);
    }
    cmd
}

/// Parse `tshark -T ek` stdout into the per-packet content objects.
///
/// EK alternates one-line `{"index": ...}` headers with one-line
/// `{"timestamp": ..., "layers": {...}}` payloads; we keep only objects that
/// carry a `layers` key.
fn parse_tshark_ek(stdout: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if obj.get("layers").is_some() {
            out.push(obj);
        }
    }
    out
}

/// Render a JSON scalar the way `str()` would (so `_ek_field`'s
/// `str(val)` matches: booleans capitalise, numbers print bare).
fn scalar_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Look up a field from a parsed EK `layers` object.
///
/// EK collapses dots in field names to underscores within each layer, so
/// `ip.src` lives at `layers.ip["ip_src"]`. A repeated field is a list (under
/// `--no-duplicate-keys`); we return the first element.
fn ek_field(layers: &Value, layer: &str, key: &str) -> String {
    let Some(layer_obj) = layers.get(layer).filter(|v| v.is_object()) else {
        return String::new();
    };
    match layer_obj.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Array(items)) => items.first().map_or_else(String::new, scalar_str),
        Some(val) => scalar_str(val),
    }
}

/// Insert `(key, value)` into a first-seen-ordered header list only when `key`
/// is absent.
fn header_setdefault(headers: &mut Vec<(String, String)>, key: &str, value: &str) {
    if !headers.iter().any(|(k, _)| k == key) {
        headers.push((key.to_owned(), value.to_owned()));
    }
}

/// Canonicalise an IP literal the way `ipaddress.ip_address(...)`
/// does (so e.g. compressed IPv6 forms match the walker's keys). Returns
/// `None` for anything that doesn't parse as an IP.
fn canonical_ip(raw: &str) -> Option<String> {
    raw.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string())
}

/// Resolve the L3/L4 5-tuple key for one EK packet, or `None` when it is not a
/// keyable TCP/UDP/IP packet.
#[allow(clippy::similar_names)] // ip_src / ip_dst / sp / dp are the domain 5-tuple.
fn packet_key(layers: &Value) -> Option<FlowKey> {
    let ip_src = {
        let v = ek_field(layers, "ip", "ip_src");
        if v.is_empty() {
            ek_field(layers, "ipv6", "ipv6_src")
        } else {
            v
        }
    };
    let ip_dst = {
        let v = ek_field(layers, "ip", "ip_dst");
        if v.is_empty() {
            ek_field(layers, "ipv6", "ipv6_dst")
        } else {
            v
        }
    };
    let tcp_sp = ek_field(layers, "tcp", "tcp_srcport");
    let tcp_dp = ek_field(layers, "tcp", "tcp_dstport");
    let udp_sp = ek_field(layers, "udp", "udp_srcport");
    let udp_dp = ek_field(layers, "udp", "udp_dstport");
    let (proto, sp, dp) = if !tcp_sp.is_empty() || !tcp_dp.is_empty() {
        (6u8, parse_port(&tcp_sp), parse_port(&tcp_dp))
    } else if !udp_sp.is_empty() || !udp_dp.is_empty() {
        (17u8, parse_port(&udp_sp), parse_port(&udp_dp))
    } else {
        return None;
    };
    if ip_src.is_empty() {
        return None;
    }
    let ip_src = canonical_ip(&ip_src)?;
    let ip_dst = canonical_ip(&ip_dst)?;
    Some((ip_src, sp, ip_dst, dp, proto))
}

/// Parse a port string the way `int(x) if x.isdigit() else 0` does
/// (all-ASCII-digits only; anything else, including empty, is 0).
fn parse_port(raw: &str) -> u16 {
    if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
        raw.parse().unwrap_or(0)
    } else {
        0
    }
}

/// EK flag fields render as `"1"`/`"0"` or `"True"`/`"False"`; treat both true
/// spellings as set.
fn flag_set(v: &str) -> bool {
    v == "1" || v == "True"
}

/// Overlay decoded fields from one EK `layers` object onto `flows`.
#[allow(clippy::too_many_lines)] // One field-by-field overlay; splitting would obscure it.
fn apply_packet_to_flows(layers: &Value, flows: &mut IndexMap<FlowKey, Flow>) {
    let Some(key) = packet_key(layers) else {
        return;
    };
    let Some(flow) = flows.get_mut(&key) else {
        return;
    };

    // TLS handshake.
    let sni = ek_field(layers, "tls", "tls_handshake_extensions_server_name");
    if !sni.is_empty() && flow.tls_sni.is_empty() {
        flow.tls_sni = sni;
        flow.tls_clienthello = true;
    }
    let cipher = ek_field(layers, "tls", "tls_handshake_ciphersuite");
    if !cipher.is_empty() && flow.tls_chosen_cipher.is_empty() {
        flow.tls_chosen_cipher = cipher;
    }
    let chosen_ver = {
        let v = ek_field(layers, "tls", "tls_handshake_version");
        if v.is_empty() {
            ek_field(layers, "tls", "tls_record_version")
        } else {
            v
        }
    };
    if !chosen_ver.is_empty() && flow.tls_chosen_version.is_empty() {
        flow.tls_chosen_version = name_tls_version(&chosen_ver);
    }
    let alpn = ek_field(layers, "tls", "tls_handshake_extensions_alpn_str");
    if !alpn.is_empty() && flow.tls_alpn.is_empty() {
        flow.tls_alpn = alpn;
    }
    let alert_desc = ek_field(layers, "tls", "tls_alert_message_desc");
    if !alert_desc.is_empty() {
        flow.tls_alert_seen = true;
        flow.tls_alert_desc = alert_desc;
    }
    let cert_subj = ek_field(layers, "x509sat", "x509sat_printableString");
    if !cert_subj.is_empty() && flow.tls_cert_subject.is_empty() {
        flow.tls_cert_subject = cert_subj;
    }
    let sig_alg = ek_field(layers, "tls", "tls_handshake_sig_hash_alg");
    if !sig_alg.is_empty() && flow.tls_signature_algos.is_empty() {
        flow.tls_signature_algos = sig_alg;
    }
    let supp_groups = ek_field(layers, "tls", "tls_handshake_extensions_supported_group");
    if !supp_groups.is_empty() && flow.tls_supported_groups.is_empty() {
        flow.tls_supported_groups = supp_groups;
    }

    // HTTP request.
    let method = ek_field(layers, "http", "http_request_method");
    if !method.is_empty() && flow.http_method.is_empty() {
        flow.http_method = method;
        flow.http_request_seen = true;
    }
    let host = ek_field(layers, "http", "http_host");
    if !host.is_empty() && flow.http_host.is_empty() {
        flow.http_host = host;
    }
    let uri = {
        let v = ek_field(layers, "http", "http_request_uri");
        if v.is_empty() {
            ek_field(layers, "http", "http_request_full_uri")
        } else {
            v
        }
    };
    if !uri.is_empty() && flow.http_uri.is_empty() {
        let (path, query) = match uri.split_once('?') {
            Some((p, q)) => (p.to_owned(), q.to_owned()),
            None => (uri.clone(), String::new()),
        };
        if flow.http_path.is_empty() {
            flow.http_path = path;
        }
        if flow.http_query.is_empty() {
            flow.http_query = query;
        }
        flow.http_uri = uri;
    }
    let http_ver = ek_field(layers, "http", "http_request_version");
    if !http_ver.is_empty() && flow.http_request_version.is_empty() {
        flow.http_request_version = http_ver;
    }
    let ua = ek_field(layers, "http", "http_user_agent");
    if !ua.is_empty() && flow.http_user_agent.is_empty() {
        header_setdefault(&mut flow.http_request_headers, "user-agent", &ua);
        flow.http_user_agent = ua;
    }
    let cookie = ek_field(layers, "http", "http_cookie");
    if !cookie.is_empty() && flow.http_cookie.is_empty() {
        header_setdefault(&mut flow.http_request_headers, "cookie", &cookie);
        flow.http_cookie = cookie;
    }
    let referer = ek_field(layers, "http", "http_referer");
    if !referer.is_empty() && flow.http_referer.is_empty() {
        header_setdefault(&mut flow.http_request_headers, "referer", &referer);
        flow.http_referer = referer;
    }
    let content_type = ek_field(layers, "http", "http_content_type");
    if !content_type.is_empty() && flow.http_request_content_type.is_empty() {
        header_setdefault(
            &mut flow.http_request_headers,
            "content-type",
            &content_type,
        );
        flow.http_request_content_type = content_type;
    }
    let content_length = ek_field(layers, "http", "http_content_length");
    if !content_length.is_empty() && flow.http_request_content_length.is_empty() {
        header_setdefault(
            &mut flow.http_request_headers,
            "content-length",
            &content_length,
        );
        flow.http_request_content_length = content_length;
    }

    // HTTP response.
    let code = ek_field(layers, "http", "http_response_code");
    if !code.is_empty() {
        flow.http_response_seen = true;
        flow.http_response_code = code;
    }
    let resp_phrase = ek_field(layers, "http", "http_response_phrase");
    if !resp_phrase.is_empty() && flow.http_response_phrase.is_empty() {
        flow.http_response_phrase = resp_phrase;
    }

    // F5 reset cause TLVs (DPT trailer).
    if flag_set(&ek_field(layers, "tcp", "tcp_flags_reset")) {
        flow.tcp_rst = true;
    }
    for ek_key in [
        "f5ethtrailer_rstcause_cause",
        "f5ethtrailer_rstcause_line",
        "f5ethtrailer_rstcause_peer",
    ] {
        let piece = ek_field(layers, "f5ethtrailer", ek_key);
        let tag = piece.trim();
        if !tag.is_empty() && !flow.f5_reset_causes.iter().any(|c| c == tag) {
            flow.f5_reset_causes.push(tag.to_owned());
        }
    }
}

/// Run tshark and overlay decoded L7 fields onto `flows`.
///
/// Returns `true` when tshark ran successfully. Silently returns `false` when
/// tshark is missing, errors out, or produces no parseable output. When
/// `keylog_path` is set and the file exists, HTTPS payloads decrypt so the
/// HTTP method/URI/response fields populate for TLS-wrapped flows.
#[must_use]
pub fn enrich_with_tshark(
    flows: &mut IndexMap<FlowKey, Flow>,
    pcap_path: &Path,
    keylog_path: &str,
    tshark_filter: &str,
) -> bool {
    if !tshark_available() {
        return false;
    }
    let Some(stdout) = run_tshark(pcap_path, keylog_path, tshark_filter) else {
        return false;
    };
    for packet in parse_tshark_ek(&stdout) {
        if let Some(layers) = packet.get("layers") {
            apply_packet_to_flows(layers, flows);
        }
    }
    true
}

/// Extract flows from `pcap_path` by driving tshark with EK output.
///
/// Used when the caller passes a `--tshark-filter` so only packets matching the
/// display filter contribute. Returns the same `{5-tuple: Flow}` shape as the
/// built-in walker. F5 trailer peer-tuples are NOT populated by this path
/// (front/back session pairing for `:np` captures works only with the built-in
/// walker). On any tshark failure, returns an empty map; the caller falls back
/// to the built-in walker.
#[must_use]
pub fn extract_flows_via_tshark(
    pcap_path: &Path,
    keylog_path: &str,
    tshark_filter: &str,
) -> IndexMap<FlowKey, Flow> {
    let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
    if !tshark_available() {
        return flows;
    }
    let Some(stdout) = run_tshark(pcap_path, keylog_path, tshark_filter) else {
        return flows;
    };
    let parsed = parse_tshark_ek(&stdout);

    // First pass: establish flow keys + counts + TCP flags.
    for packet in &parsed {
        let Some(layers) = packet.get("layers") else {
            continue;
        };
        let Some(key) = packet_key(layers) else {
            continue;
        };
        let (ip_src, sp, ip_dst, dp, proto) = key.clone();
        let flow = flows
            .entry(key)
            .or_insert_with(|| Flow::new(ip_src, sp, ip_dst, dp, proto));

        flow.packets += 1;
        let frame_len = ek_field(layers, "frame", "frame_len");
        if let Ok(n) = frame_len.parse::<u64>() {
            flow.bytes_total += n;
        }
        if proto == 6 {
            let syn = flag_set(&ek_field(layers, "tcp", "tcp_flags_syn"));
            let ack = flag_set(&ek_field(layers, "tcp", "tcp_flags_ack"));
            let fin = flag_set(&ek_field(layers, "tcp", "tcp_flags_fin"));
            if syn && ack {
                flow.tcp_synack = true;
            } else if syn {
                flow.tcp_syn = true;
            }
            if fin {
                flow.tcp_fin = true;
            }
        }
    }

    // Second pass: HTTP / TLS / RST cause overlay.
    for packet in &parsed {
        if let Some(layers) = packet.get("layers") {
            apply_packet_to_flows(layers, &mut flows);
        }
    }
    flows
}

/// Run the tshark EK command and return its stdout, or `None` on any failure
/// (spawn error, non-zero exit).
fn run_tshark(pcap_path: &Path, keylog_path: &str, tshark_filter: &str) -> Option<String> {
    let output = tshark_ek_cmd(pcap_path, keylog_path, tshark_filter)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_tls_version_normalises_widths_and_bases() {
        assert_eq!(name_tls_version("0x0303"), "TLS1.2");
        assert_eq!(name_tls_version("0x303"), "TLS1.2");
        assert_eq!(name_tls_version("771"), "TLS1.2");
        assert_eq!(name_tls_version("0xFEFD"), "DTLS1.2");
        assert_eq!(name_tls_version(""), "");
        assert_eq!(name_tls_version("nonsense"), "nonsense");
        // Unknown but parseable values fall back to the raw string.
        assert_eq!(name_tls_version("0x0500"), "0x0500");
    }

    #[test]
    fn ek_field_collapses_lists_to_first() {
        let layers: Value = serde_json::json!({
            "ip": {"ip_src": "10.0.0.1", "ip_dst": ["10.0.0.2", "10.0.0.3"]},
            "tcp": {"tcp_srcport": 4433},
        });
        assert_eq!(ek_field(&layers, "ip", "ip_src"), "10.0.0.1");
        assert_eq!(ek_field(&layers, "ip", "ip_dst"), "10.0.0.2");
        // Numbers stringify bare.
        assert_eq!(ek_field(&layers, "tcp", "tcp_srcport"), "4433");
        assert_eq!(ek_field(&layers, "udp", "udp_srcport"), "");
        assert_eq!(ek_field(&layers, "ip", "missing"), "");
    }

    #[test]
    fn header_setdefault_is_first_wins() {
        let mut headers = vec![("user-agent".to_owned(), "curl".to_owned())];
        header_setdefault(&mut headers, "user-agent", "wget");
        header_setdefault(&mut headers, "cookie", "a=1");
        assert_eq!(
            headers,
            vec![
                ("user-agent".to_owned(), "curl".to_owned()),
                ("cookie".to_owned(), "a=1".to_owned()),
            ]
        );
    }

    #[test]
    fn apply_packet_overlays_http_and_tls_onto_matching_flow() {
        let key: FlowKey = ("10.0.0.1".to_owned(), 5000, "10.0.0.2".to_owned(), 443, 6);
        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        flows.insert(
            key.clone(),
            Flow::new("10.0.0.1".to_owned(), 5000, "10.0.0.2".to_owned(), 443, 6),
        );
        let layers: Value = serde_json::json!({
            "ip": {"ip_src": "10.0.0.1", "ip_dst": "10.0.0.2"},
            "tcp": {"tcp_srcport": "5000", "tcp_dstport": "443"},
            "tls": {
                "tls_handshake_extensions_server_name": "example.com",
                "tls_handshake_version": "0x0304",
                "tls_handshake_extensions_alpn_str": "h2",
            },
            "http": {
                "http_request_method": "GET",
                "http_request_uri": "/path?q=1",
                "http_user_agent": "curl/8",
            },
        });
        apply_packet_to_flows(&layers, &mut flows);
        let flow = &flows[&key];
        assert_eq!(flow.tls_sni, "example.com");
        assert!(flow.tls_clienthello);
        assert_eq!(flow.tls_chosen_version, "TLS1.3");
        assert_eq!(flow.tls_alpn, "h2");
        assert_eq!(flow.http_method, "GET");
        assert!(flow.http_request_seen);
        assert_eq!(flow.http_uri, "/path?q=1");
        assert_eq!(flow.http_path, "/path");
        assert_eq!(flow.http_query, "q=1");
        assert_eq!(flow.request_header("user-agent"), Some("curl/8"));
    }

    #[test]
    fn apply_packet_ignores_unknown_flow_key() {
        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        let layers: Value = serde_json::json!({
            "ip": {"ip_src": "10.0.0.9", "ip_dst": "10.0.0.2"},
            "tcp": {"tcp_srcport": "5000", "tcp_dstport": "443"},
            "http": {"http_request_method": "GET"},
        });
        apply_packet_to_flows(&layers, &mut flows);
        assert!(flows.is_empty());
    }

    #[test]
    fn parse_tshark_ek_keeps_only_layer_payloads() {
        let stdout = concat!(
            "{\"index\": {\"_index\": \"packets\"}}\n",
            "\n",
            "{\"timestamp\": \"1\", \"layers\": {\"frame\": {\"frame_len\": \"66\"}}}\n",
            "not json\n",
        );
        let packets = parse_tshark_ek(stdout);
        assert_eq!(packets.len(), 1);
        assert_eq!(ek_field(&packets[0]["layers"], "frame", "frame_len"), "66");
    }
}
