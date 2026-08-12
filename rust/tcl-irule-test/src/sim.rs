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

//! Reusable iRule simulation entry point — stand up a [`LiveSession`] on the
//! bytecode VM, load iRule source(s), fire a request, and read back the
//! pool/node decision, captured logs, and decision log.
//!
//! This is the config-agnostic core shared by the `f5 explain-flow --simulate`
//! path and the `tcl_lsp_py.simulate_irule` facade. It is **runtime-ready**: it
//! drives [`tcl-vm`](tcl_vm) in-process (no `tclsh`), so it lights up as the VM
//! grows the command surface the orchestrator needs (`HTTP::*`, `pool`/`node`,
//! `LB::*`, `when` dispatch, `class match`, …). Until then every VM /
//! orchestrator failure is captured into [`SimOutcome::error`] so callers can
//! still render their static analysis — nothing panics or blocks.

use std::fmt::Write as _;

use tcl_syntax::list::split_list_lenient;

use crate::live::LiveSession;

/// One captured HTTP request to drive through the simulated flow.
#[derive(Debug, Clone, Default)]
pub struct SimRequest {
    /// HTTP method (defaults to `GET` when empty).
    pub method: String,
    /// Request URI (defaults to `/` when empty).
    pub uri: String,
    /// `Host` header / authority (omitted when empty).
    pub host: String,
    /// TLS SNI (omitted when empty).
    pub sni: String,
    /// Additional request headers as `(name, value)` pairs.
    pub headers: Vec<(String, String)>,
}

/// The dynamic outcome of running iRule(s) under the embedded TMM-sim
/// orchestrator on `tcl-vm`. Empty/`false` fields mean "not observed"; a
/// non-empty [`error`](Self::error) means the run could not complete (the VM is
/// not yet able to execute the orchestrator), and every other field is then
/// left at its default.
#[derive(Debug, Default, Clone)]
pub struct SimOutcome {
    /// The pool the iRule selected (empty if none / not run).
    pub pool: String,
    /// The selected node as `addr:port` (empty if none / not run).
    pub node: String,
    /// Whether the iRule committed an HTTP response (`HTTP::respond` / redirect).
    pub response_committed: bool,
    /// Captured iRule log messages, one display string per entry.
    pub logs: Vec<String>,
    /// The decision log as `(category, action, value)` triples.
    pub decisions: Vec<(String, String, String)>,
    /// Non-empty when the simulation could not run; the reason (missing VM
    /// command, orchestrator error, …). Callers still render static analysis.
    pub error: String,
}

/// Simulate `sources` (iRule bodies) under `profiles` with `pools`
/// (`(name, member-node-names)`), optionally firing `request`.
///
/// When `HTTP` is among `profiles` and `request` is `Some`, the request is run
/// through the flow and the pool/node/logs/decisions are read back; otherwise a
/// bare `CLIENT_ACCEPTED` is fired to exercise L4 logic. Returns a best-effort
/// [`SimOutcome`]: any failure (including the VM not yet supporting a needed
/// command) lands in [`SimOutcome::error`] rather than panicking.
#[must_use]
pub fn simulate_irule(
    sources: &[String],
    profiles: &[String],
    pools: &[(String, Vec<String>)],
    request: Option<&SimRequest>,
) -> SimOutcome {
    let mut out = SimOutcome::default();

    if sources.is_empty() {
        out.error = String::from("no iRules to simulate");
        return out;
    }

    let mut sess = match LiveSession::embedded() {
        Ok(s) => s,
        Err(e) => {
            out.error = format!("cannot start orchestrator: {e}");
            return out;
        }
    };

    macro_rules! guard {
        ($e:expr) => {
            if let Err(e) = $e {
                out.error = e.to_string();
                return out;
            }
        };
    }

    guard!(sess.eval(&format!(
        "::orch::configure -profiles {{{}}}",
        profiles.join(" ")
    )));
    for src in sources {
        guard!(sess.load_irule(src));
    }
    for (name, members) in pools {
        let member_list = members
            .iter()
            .filter(|m| !m.is_empty())
            .map(|m| tcl_quote(m))
            .collect::<Vec<_>>()
            .join(" ");
        if member_list.is_empty() {
            continue;
        }
        guard!(sess.eval(&format!(
            "::orch::add_pool {} [list {member_list}]",
            tcl_quote(name)
        )));
    }

    let http = profiles.iter().any(|p| p == "HTTP");
    if let (true, Some(req)) = (http, request) {
        let method = if req.method.is_empty() {
            "GET"
        } else {
            &req.method
        };
        let uri = if req.uri.is_empty() { "/" } else { &req.uri };
        let mut args = format!("-method {} -uri {}", tcl_quote(method), tcl_quote(uri));
        if !req.host.is_empty() {
            let _ = write!(args, " -host {}", tcl_quote(&req.host));
        }
        if !req.sni.is_empty() {
            let _ = write!(args, " -sni {}", tcl_quote(&req.sni));
        }
        if !req.headers.is_empty() {
            let hdr_tokens: Vec<String> = req
                .headers
                .iter()
                .flat_map(|(k, v)| [tcl_quote(k), tcl_quote(v)])
                .collect();
            let _ = write!(args, " -headers [list {}]", hdr_tokens.join(" "));
        }
        guard!(sess.run_http_request(&args));
    } else {
        // Non-HTTP (or no request): fire CLIENT_ACCEPTED to exercise L4
        // logic; no request/response readback.
        guard!(sess.eval("::orch::fire CLIENT_ACCEPTED"));
        return out;
    }

    out.pool = sess.pool_selected().unwrap_or_default();
    let node_addr = sess.eval("set ::state::lb::node_addr").unwrap_or_default();
    if !node_addr.is_empty() {
        let node_port = sess.eval("set ::state::lb::node_port").unwrap_or_default();
        out.node = format!("{node_addr}:{node_port}");
    }
    // The flag lives directly in the `http` state namespace
    // (`::state::http::response_committed`), set by the `HTTP::respond` /
    // `HTTP::redirect` mocks. Reading the non-existent
    // `::state::http::response::response_committed` errored, and `is_ok_and`
    // swallowed the error so a committed response always read as `false`.
    out.response_committed = sess
        .eval("set ::state::http::response_committed")
        .is_ok_and(|v| v == "1" || v == "true");
    out.decisions = parse_decisions(&sess.decisions().unwrap_or_default());
    out.logs = parse_log_entries(&sess.logs().unwrap_or_default());
    out
}

/// Quote `s` as a Tcl double-quoted word, escaping substitution / quoting
/// metacharacters so an arbitrary captured value can't break the command.
fn tcl_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if matches!(c, '$' | '[' | ']' | '"' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Parse the orchestrator's decision log (a Tcl list of `{category action args}`)
/// into `(category, action, value)` triples.
fn parse_decisions(list: &str) -> Vec<(String, String, String)> {
    split_list_lenient(list)
        .into_iter()
        .filter_map(|entry| {
            let parts = split_list_lenient(&entry);
            match parts.as_slice() {
                [cat, act, rest @ ..] => Some((
                    cat.to_string(),
                    act.to_string(),
                    rest.iter()
                        .map(|v| split_list_lenient(v).join(" "))
                        .collect::<Vec<_>>()
                        .join(" "),
                )),
                _ => None,
            }
        })
        .collect()
}

/// Parse the captured log list (each entry a `{facility level message …}` Tcl
/// list) into `" | "`-joined display strings.
fn parse_log_entries(list: &str) -> Vec<String> {
    split_list_lenient(list)
        .into_iter()
        .map(|entry| {
            let parts = split_list_lenient(&entry);
            if parts.is_empty() {
                entry.into_owned()
            } else {
                parts.join(" | ")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sources_reports_nothing_to_simulate() {
        let out = simulate_irule(&[], &[], &[], None);
        assert_eq!(out.error, "no iRules to simulate");
    }

    #[test]
    fn simulation_degrades_gracefully_until_vm_is_ready() {
        // The VM cannot yet execute the full orchestrator; the run must not
        // panic — it returns a populated `error` and empty results.
        let sources = vec!["when HTTP_REQUEST { pool web }".to_owned()];
        let req = SimRequest {
            method: "GET".to_owned(),
            uri: "/".to_owned(),
            ..SimRequest::default()
        };
        let out = simulate_irule(&sources, &["HTTP".to_owned()], &[], Some(&req));
        // Either it ran (VM complete) or it degraded with a reason — never a
        // panic, and `error` is the only signal that distinguishes the two.
        if !out.error.is_empty() {
            assert!(out.pool.is_empty() && out.node.is_empty());
        }
    }

    #[test]
    fn decision_list_split_handles_braces() {
        // Braced elements group, nested braces stay whole (Tcl list grammar
        // via `tcl_syntax::list`).
        assert_eq!(
            parse_decisions("{lb pool {web pool}} {http respond {301 moved}}"),
            vec![
                ("lb".to_owned(), "pool".to_owned(), "web pool".to_owned()),
                (
                    "http".to_owned(),
                    "respond".to_owned(),
                    "301 moved".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn log_entry_split_follows_tcl_list_semantics() {
        // Quoted elements group as one element (`"hello world"` is not split
        // at the space)…
        assert_eq!(
            parse_log_entries("{info local0. \"hello world\"}"),
            vec!["info | local0. | hello world"]
        );
        // …and backslash escapes decode per Tcl (`\t` is a tab, not a
        // literal `t`).
        assert_eq!(parse_log_entries(r"{warn a\tb}"), vec!["warn | a\tb"]);
        // FP guard: braces still group and separate elements stay separate.
        assert_eq!(parse_log_entries("{err {b c} d}"), vec!["err | b c | d"]);
    }
}
