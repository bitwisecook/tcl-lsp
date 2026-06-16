//! The `explain-flow` verb — trace each flow in a PCAP through the BIG-IP
//! config.
//!
//! Port of `compute_explain_flow` / `_run_explain_flow`
//! (`dialects/f5/bigip/explain_flow.py`, `tooling/f5/verbs/explain_flow.py`).
//!
//! This increment lands the always-available built-in walker path: flow
//! extraction (`tcl_bigip::flow`), session pairing, virtual-server matching,
//! the reset-cause narrative, and the report formatter. The matched-session
//! detail (profile chain, iRule event chain, LTM policy trace) and the
//! `--tshark` / `--simulate` / `--json` paths land in following increments.

use std::path::Path;

use regex::Regex;
use tcl_bigip::flow::{Session, extract_flows, pair_sessions};
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::BigipConfig;
use tcl_cli_support::{OutputTarget, write_text_output};

/// One HUD annotation: `(source-line excerpt, command, captured value)`.
type Annotation = (String, String, String);
/// One iRule event body: `(rule path, event name, body text)`.
type EventBlock = (String, String, String);
/// Per-event annotation group: `(rule path, event name, annotations)`.
type EventAnnotation = (String, String, Vec<Annotation>);

/// Per-session explanation: which VS matched, the event chain, RST analysis.
/// Mirrors `SessionExplain`; fields beyond the current increment default empty.
#[derive(Default)]
struct SessionExplain {
    session: Option<SessionHolder>,
    matched_vs: String,
    /// Partition short-name; surfaced by the `--json` output increment.
    #[allow(dead_code)]
    matched_partition: String,
    profile_chain: Vec<String>,
    pool_selected: String,
    snat_observed: String,
    event_sequence: Vec<String>,
    event_blocks: Vec<EventBlock>,
    event_annotations: Vec<EventAnnotation>,
    ltm_policies: Vec<String>,
    apm_profile: String,
    gtm_wide_ips: Vec<String>,
    explain_text: String,
    reset_analysis: String,
}

/// Owns the [`Session`] referenced by a [`SessionExplain`].
struct SessionHolder(Session);

impl SessionExplain {
    fn session(&self) -> &Session {
        &self.session.as_ref().expect("session present").0
    }
}

/// The whole-report result of [`compute_explain_flow`]. The richer fields
/// carried by `ExplainFlowReport` in the reference (`pcap_path`, per-session
/// list, tshark/keylog flags) join when the `--json` output increment lands.
struct ExplainFlowReport {
    matched_count: usize,
    text_report: String,
}

/// Parse a VS destination string like `/Common/10.0.0.1:443` or `/p/[::1]:80`,
/// returning the canonical address and port. Mirrors `_parse_destination`.
fn parse_destination(dest: &str, re: &Regex) -> Option<(String, u32)> {
    let caps = re.captures(dest.trim())?;
    let addr = caps
        .name("addr")?
        .as_str()
        .trim_matches(|c| c == '[' || c == ']');
    let port_raw = caps.name("port")?.as_str();
    let canonical = canonicalise_ip(addr)?;
    let port = if port_raw == "any" {
        0
    } else {
        port_raw.parse::<u32>().ok()?
    };
    Some((canonical, port))
}

/// `str(ipaddress.ip_address(addr))` — canonical form, or `None` if invalid.
fn canonicalise_ip(addr: &str) -> Option<String> {
    addr.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string())
}

/// Find the virtual server whose destination matches `(dst_ip, dst_port)`.
/// Mirrors `_match_virtual`, iterating VSes in source order.
fn match_virtual(cfg: &BigipConfig, dst_ip: &str, dst_port: u16, re: &Regex) -> Option<String> {
    let flow_ip = canonicalise_ip(dst_ip).unwrap_or_else(|| dst_ip.to_owned());
    for placed in &cfg.objects {
        let ModelObject::VirtualServer(vs) = &placed.object else {
            continue;
        };
        let dest_text = vs
            .destination
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        let Some((vs_addr, vs_port)) = parse_destination(&dest_text, re) else {
            continue;
        };
        if vs_addr != flow_ip {
            continue;
        }
        if vs_port == 0 || vs_port == u32::from(dst_port) {
            return Some(vs.full_path.clone());
        }
    }
    None
}

/// Produce a human-readable narrative of why the session ended.
/// Faithful port of `_analyse_reset`.
fn analyse_reset(session: &Session) -> String {
    let mut parts: Vec<String> = Vec::new();
    let front = &session.front;
    let back = &session.back;
    let causes = session.reset_causes();

    if front.client.tcp_rst {
        parts.push(format!(
            "client\u{2192}VIP RST after {} bytes ({}x)",
            front.client.tcp_rst_after_bytes, front.client.tcp_rst_count
        ));
    }
    if let Some(server) = &front.server
        && server.tcp_rst
    {
        parts.push(format!(
            "VIP\u{2192}client RST after {} bytes ({}x)",
            server.tcp_rst_after_bytes, server.tcp_rst_count
        ));
    }
    if let Some(back) = back {
        if back.client.tcp_rst {
            parts.push(format!(
                "TMM\u{2192}server RST after {} bytes ({}x)",
                back.client.tcp_rst_after_bytes, back.client.tcp_rst_count
            ));
        }
        if let Some(server) = &back.server
            && server.tcp_rst
        {
            parts.push(format!(
                "server\u{2192}TMM RST after {} bytes ({}x)",
                server.tcp_rst_after_bytes, server.tcp_rst_count
            ));
        }
    }

    if parts.is_empty() {
        let front_fin = front.client.tcp_fin || front.server.as_ref().is_some_and(|s| s.tcp_fin);
        if front_fin {
            return "graceful FIN teardown (no RST)".to_owned();
        }
        return "no termination observed in capture".to_owned();
    }

    if causes.is_empty() {
        parts.push("no F5 reset cause string in trailer (LOW/MED TLV absent or opaque)".to_owned());
    } else {
        parts.push(format!("F5 reset cause(s): {}", causes.join(" | ")));
    }

    if front.client.tls_alert_seen {
        parts.push(format!("client TLS alert: {}", front.client.tls_alert_desc));
    }
    if let Some(server) = &front.server
        && server.tls_alert_seen
    {
        parts.push(format!("server TLS alert: {}", server.tls_alert_desc));
    }

    parts.join(" ; ")
}

/// Build the per-session explanation for the capture against parsed configs.
fn compute_explain_flow(
    pcap_display: &str,
    pcap_bytes: &[u8],
    configs: &[BigipConfig],
) -> Result<ExplainFlowReport, String> {
    let dest_re = Regex::new(
        r"^(?P<path>/[^/\s]+/)?(?P<addr>\[[^\]]+\]|[0-9a-fA-F\.:]+)(?:%\d+)?:(?P<port>\d+|any)$",
    )
    .expect("static destination regex");

    let flows = extract_flows(pcap_bytes)?;
    let mut sessions = pair_sessions(&flows);

    // Sort: paired (front+back) first, then by descending front packet count.
    sessions.sort_by(|a, b| {
        let rank = |s: &Session| u8::from(s.back.is_none());
        let pkts = |s: &Session| -> i128 {
            let c = i128::from(s.front.client.packets);
            let sv = s.front.server.as_ref().map_or(0, |f| i128::from(f.packets));
            -(c + sv)
        };
        rank(a).cmp(&rank(b)).then_with(|| pkts(a).cmp(&pkts(b)))
    });

    let mut session_explains: Vec<SessionExplain> = Vec::new();
    let mut matched = 0usize;

    for session in sessions {
        let front = &session.front;

        let mut vs_path: Option<String> = None;
        let mut cfg_hit: Option<&BigipConfig> = None;
        for cfg in configs {
            let mut hit = match_virtual(cfg, &front.client.dst_ip, front.client.dst_port, &dest_re);
            if hit.is_none()
                && let Some(server) = &front.server
            {
                hit = match_virtual(cfg, &server.src_ip, server.src_port, &dest_re);
            }
            if hit.is_some() {
                vs_path = hit;
                cfg_hit = Some(cfg);
                break;
            }
        }

        let (Some(vs_path), Some(_cfg_hit)) = (vs_path, cfg_hit) else {
            let reset = analyse_reset(&session);
            session_explains.push(SessionExplain {
                session: Some(SessionHolder(session)),
                reset_analysis: reset,
                ..Default::default()
            });
            continue;
        };

        matched += 1;
        let partition = if vs_path.starts_with('/') {
            vs_path.split('/').nth(1).unwrap_or("").to_owned()
        } else {
            String::new()
        };
        let reset = analyse_reset(&session);

        // Matched-session detail (profiles, events, policies, resolved plan)
        // lands in the next increment; for now record the match + termination.
        session_explains.push(SessionExplain {
            session: Some(SessionHolder(session)),
            matched_vs: vs_path,
            matched_partition: partition,
            reset_analysis: reset,
            ..Default::default()
        });
    }

    let text = format_report(pcap_display, &session_explains);
    Ok(ExplainFlowReport {
        matched_count: matched,
        text_report: text,
    })
}

/// Render the text report. Faithful port of `_format_report` (static path).
#[allow(clippy::too_many_lines)]
fn format_report(pcap_path: &str, sessions: &[SessionExplain]) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("explain-flow: {pcap_path}"));
    let matched = sessions.iter().filter(|s| !s.matched_vs.is_empty()).count();
    // `tshark` / `keylog` / `filter` annotations land with the tshark path.
    lines.push(format!(
        "  sessions: {} | matched: {} | tshark: no",
        sessions.len(),
        matched
    ));
    lines.push(String::new());

    for (i, se) in sessions.iter().enumerate() {
        let s = se.session();
        lines.push(format!("[session {}] {}", i + 1, s.proto_name()));
        lines.push(format!("  front: {}", s.front.summary()));
        if let Some(back) = &s.back {
            lines.push(format!("  back:  {}", back.summary()));
        }
        if se.matched_vs.is_empty() {
            lines.push("  (no virtual server matched this destination)".to_owned());
            if !se.reset_analysis.is_empty() {
                lines.push(format!("  termination: {}", se.reset_analysis));
            }
            lines.push(String::new());
            continue;
        }
        lines.push(format!("  matched virtual: {}", se.matched_vs));
        if !se.pool_selected.is_empty() {
            lines.push(format!(
                "  pool member chosen (observed): {}",
                se.pool_selected
            ));
        }
        if !se.snat_observed.is_empty() {
            lines.push(format!("  SNAT applied (observed): {}", se.snat_observed));
        }
        if !se.profile_chain.is_empty() {
            lines.push("  profiles (in attach order):".to_owned());
            for p in &se.profile_chain {
                lines.push(format!("    - {p}"));
            }
        }
        if !se.ltm_policies.is_empty() {
            lines.push("  ltm policies:".to_owned());
            for p in &se.ltm_policies {
                lines.push(format!("    - {p}"));
            }
        }
        // ltm policy decisions: deferred to the policy-evaluation increment.
        if !se.apm_profile.is_empty() {
            lines.push(format!("  apm: {}", se.apm_profile));
        }
        if !se.gtm_wide_ips.is_empty() {
            lines.push(
                "  gtm wide-ips in config (global inventory; may or may not point at this VS):"
                    .to_owned(),
            );
            for w in &se.gtm_wide_ips {
                lines.push(format!("    - {w}"));
            }
        }
        if !se.event_sequence.is_empty() {
            lines.push("  expected iRule event firing order:".to_owned());
            for ev in &se.event_sequence {
                lines.push(format!("    -> {ev}"));
            }
        }
        if !se.event_blocks.is_empty() {
            lines.push("  iRule event bodies (path through):".to_owned());
            for (rule, ev, body) in &se.event_blocks {
                lines.push(format!("    --- {rule} :: when {ev} ---"));
                for body_line in body.split('\n') {
                    lines.push(format!("      {body_line}"));
                }
            }
        }
        if !se.event_annotations.is_empty() {
            lines.push("  HUD-state captured for iRule commands:".to_owned());
            for (rule, ev, anns) in &se.event_annotations {
                lines.push(format!("    --- {rule} :: when {ev} ---"));
                for (line_excerpt, cmd, value) in anns {
                    lines.push(format!("      [{cmd}] = {}", py_repr(value)));
                    lines.push(format!("          ({line_excerpt})"));
                }
            }
        }
        // Captured request (front-side client flow).
        let fc = &s.front.client;
        if fc.http_request_seen || fc.tls_clienthello {
            lines.push("  captured request:".to_owned());
            if !fc.http_method.is_empty() || !fc.http_uri.is_empty() {
                let method = if fc.http_method.is_empty() {
                    "?"
                } else {
                    &fc.http_method
                };
                let req_line = format!("{method} {} {}", fc.http_uri, fc.http_request_version);
                lines.push(format!("    request line: {}", req_line.trim()));
            }
            if !fc.http_host.is_empty() {
                lines.push(format!("    host: {}", fc.http_host));
            }
            if !fc.http_user_agent.is_empty() {
                lines.push(format!("    user-agent: {}", fc.http_user_agent));
            }
            if !fc.http_referer.is_empty() {
                lines.push(format!("    referer: {}", fc.http_referer));
            }
            if !fc.http_cookie.is_empty() {
                lines.push(format!("    cookie: {}", fc.http_cookie));
            }
            if !fc.tls_sni.is_empty() {
                use std::fmt::Write as _;
                let mut tls_line = format!("    tls: SNI={}", fc.tls_sni);
                if !fc.tls_chosen_version.is_empty() || !fc.tls_version.is_empty() {
                    let v = if fc.tls_chosen_version.is_empty() {
                        &fc.tls_version
                    } else {
                        &fc.tls_chosen_version
                    };
                    let _ = write!(tls_line, " version={v}");
                }
                if !fc.tls_chosen_cipher.is_empty() {
                    let _ = write!(tls_line, " cipher={}", fc.tls_chosen_cipher);
                }
                if !fc.tls_alpn.is_empty() {
                    let _ = write!(tls_line, " alpn={}", fc.tls_alpn);
                }
                lines.push(tls_line);
            }
            if !fc.tls_cert_subject.is_empty() {
                lines.push(format!("    server cert subject: {}", fc.tls_cert_subject));
            }
        }
        // Captured response (server-side flow).
        if let Some(sc) = &s.front.server
            && (sc.http_response_seen || !sc.http_response_code.is_empty())
        {
            lines.push("  captured response:".to_owned());
            let status = format!("{} {}", sc.http_response_code, sc.http_response_phrase);
            let status = status.trim();
            let status = if status.is_empty() { "(none)" } else { status };
            lines.push(format!("    status: {status}"));
            if !sc.http_response_content_type.is_empty() {
                lines.push(format!(
                    "    content-type: {}",
                    sc.http_response_content_type
                ));
            }
            if !sc.http_response_content_length.is_empty() {
                lines.push(format!(
                    "    content-length: {}",
                    sc.http_response_content_length
                ));
            }
        }
        // iRule simulation outcome: deferred to the simulate increment.
        lines.push(format!("  termination: {}", se.reset_analysis));
        lines.push("  resolved plan:".to_owned());
        for explain_line in se.explain_text.split('\n') {
            if se.explain_text.is_empty() {
                break;
            }
            lines.push(format!("    {explain_line}"));
        }
        lines.push(String::new());
    }

    let joined = lines.join("\n");
    format!("{}\n", joined.trim_end())
}

/// Render a string the way Python's `repr()` does for the HUD annotations
/// (single-quoted, with `\\`, `\'`, `\n`, `\r`, `\t` escapes).
fn py_repr(s: &str) -> String {
    let use_double = s.contains('\'') && !s.contains('"');
    let quote = if use_double { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Entry point for `f5 explain-flow` (built-in walker / static path).
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn run_explain_flow(
    pcap: &Path,
    paths: &[std::path::PathBuf],
    tshark: bool,
    keylog: Option<&Path>,
    tshark_filter: Option<&str>,
    simulate: bool,
    _no_event_bodies: bool,
    _max_event_lines: usize,
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    if !pcap.is_file() {
        anyhow::bail!("not a file: {}", pcap.display());
    }
    if simulate {
        anyhow::bail!("`f5 explain-flow --simulate` is not yet implemented in the Rust port");
    }
    if json {
        anyhow::bail!("`f5 explain-flow --json` is not yet implemented in the Rust port");
    }
    let use_tshark = tshark || keylog.is_some() || tshark_filter.is_some();
    if use_tshark {
        anyhow::bail!(
            "`f5 explain-flow --tshark/--keylog/--tshark-filter` is not yet implemented in the Rust port"
        );
    }

    let opts = crate::cli::PassphraseArgs::default().to_options();
    let path_strs: Vec<String> = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let configs: Vec<BigipConfig> = tcl_bigip_io::load_paths(&path_strs, &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .map(|loaded| loaded.config)
        .collect();

    let pcap_bytes = std::fs::read(pcap)?;
    let report = compute_explain_flow(&pcap.display().to_string(), &pcap_bytes, &configs)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &report.text_report)?;

    Ok(u8::from(report.matched_count == 0))
}
