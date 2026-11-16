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

//! Barriers on diagnostics: pulls, the publisher's current report, and waiting for a publication that is first, settled, versioned, master-off or excluded.

use super::*;

impl Lsp {
    /// Issue a synchronous `textDocument/diagnostic` pull request for `uri` and
    /// return the reported diagnostics. The pull handler reads the *live* buffer
    /// and computes (or returns the already-settled) diagnostics for the current
    /// revision, so — unlike the debounced push channel — the response is a
    /// deterministic barrier: it only returns once the server has analysed the
    /// document's latest content, and cannot be dropped by the diagnostics
    /// debounce/coalescer.
    ///
    /// The server answers `ContentModified` with reason `pending`, `stale` or
    /// `cancelled` while the latest content is not yet analysed; that is
    /// retried within one load-scaled deadline, never read as empty findings.
    pub fn pull_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        const METHOD: &str = "textDocument/diagnostic";
        let deadline = Instant::now() + scaled_timeout(REQUEST_TIMEOUT);
        let report = loop {
            let response = self.request_response(
                METHOD,
                json!({ "textDocument": { "uri": uri } }),
                REQUEST_TIMEOUT,
            );
            let Some(err) = response.get("error") else {
                break response.get("result").cloned().unwrap_or(Value::Null);
            };
            let retryable = err.get("code").and_then(Value::as_i64) == Some(-32801)
                && matches!(
                    err.pointer("/data/reason").and_then(Value::as_str),
                    Some("pending" | "stale" | "cancelled")
                );
            assert!(
                retryable && Instant::now() < deadline,
                "{METHOD} -> error {err}"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        // A `full` report carries `items`; an `unchanged` report (only returned
        // when a matching `previousResultId` is replayed, which this never sends)
        // carries none.
        report
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    /// Query the existing publisher owner for this exact client-authored source.
    /// Only an actual complete report can support a negative assertion. Pending
    /// work retries inside one load-scaled deadline; unavailable/suppressed or
    /// malformed replies never become empty findings. This does not schedule
    /// analysis or infer completion from a diagnostic count/delivery event.
    pub fn current_complete_diagnostics(
        &mut self,
        uri: &str,
        version: i64,
        source: &str,
        timeout: Duration,
    ) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let method = "workspace/executeCommand";
        loop {
            assert!(
                Instant::now() < deadline,
                "current diagnostics exceeded {timeout:?}: {uri}"
            );
            let id = self.send_request_no_wait(
                method,
                json!({
                    "command": "tcl-lsp.currentDiagnostics",
                    "arguments": [{ "uri": uri, "version": version, "source": source }],
                }),
            );
            let response = self.await_response_before(id, method, timeout, deadline);
            assert!(
                Instant::now() < deadline,
                "current diagnostics exceeded {timeout:?}: {uri}"
            );
            assert!(
                response.get("error").is_none(),
                "current diagnostics failed: {response}"
            );
            let report = response
                .get("result")
                .expect("current diagnostics response has a result");
            match report.get("status").and_then(Value::as_str) {
                Some("complete") => {
                    assert_eq!(report.get("uri").and_then(Value::as_str), Some(uri));
                    assert_eq!(report.get("version").and_then(Value::as_i64), Some(version));
                    assert_eq!(report.get("source").and_then(Value::as_str), Some(source));
                    for field in ["revision", "inputsEpoch"] {
                        assert!(
                            report
                                .get(field)
                                .and_then(Value::as_str)
                                .is_some_and(|value| {
                                    !value.is_empty()
                                        && value.bytes().all(|byte| byte.is_ascii_digit())
                                }),
                            "current diagnostics has invalid {field}: {report}"
                        );
                    }
                    assert!(
                        report
                            .get("resultId")
                            .and_then(Value::as_str)
                            .is_some_and(|value| !value.is_empty()),
                        "current diagnostics lacks report identity: {report}"
                    );
                    let findings = report
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .expect("complete diagnostics must contain the actual findings array");
                    let _: Vec<tower_lsp_server::ls_types::Diagnostic> =
                        serde_json::from_value(Value::Array(findings.clone()))
                            .expect("completed findings must be valid LSP diagnostics");
                    assert!(
                        Instant::now() < deadline,
                        "current diagnostics exceeded {timeout:?}: {uri}"
                    );
                    return findings.clone();
                }
                Some("unavailable")
                    if matches!(
                        report.get("reason").and_then(Value::as_str),
                        Some("pending" | "stale")
                    ) => {}
                _ => panic!("current diagnostics is not complete for {uri}: {report}"),
            }
            std::thread::sleep(
                Duration::from_millis(2).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }

    /// Block until a `publishDiagnostics` for `uri` arrives; return its
    /// diagnostics array.
    pub fn await_diagnostics(&self, uri: &str) -> Vec<Value> {
        self.await_diagnostics_version(uri, None, DEFAULT_TIMEOUT)
    }

    /// Block until the first `publishDiagnostics` for `uri` arrives and return
    /// its diagnostics, whatever later publishes replace it with.
    ///
    /// For a test about what a document is *first* told: the server
    /// republishes as start-up work completes, so the settled result cannot
    /// say whether the first one was right.
    pub fn await_first_diagnostics(&self, uri: &str, timeout: Duration) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let first = notes.iter().find_map(|note| {
                if note.get("method").and_then(Value::as_str)
                    != Some("textDocument/publishDiagnostics")
                {
                    return None;
                }
                let params = note.get("params")?;
                if params.get("uri").and_then(Value::as_str) != Some(uri) {
                    return None;
                }
                Some(
                    params
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                )
            });
            if let Some(diagnostics) = first {
                return diagnostics;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(notes);
                panic!(
                    "no publishDiagnostics for {uri:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until the most recent `publishDiagnostics` for `uri` satisfies
    /// `settled`, returning it.
    ///
    /// For facts the server publishes **progressively**: a cross-file
    /// correction lands on a later publish than the document's
    /// own first result, because the project-wide call-site evidence is
    /// refreshed after publishing rather than in front of it — putting it in
    /// front delayed the semantic-token enrichment tier on a large document.
    /// [`Self::open_ready`] returns the *first* publish, so a test asserting a
    /// cross-file outcome has to wait for the converged one instead.
    pub fn await_diagnostics_settled(
        &self,
        uri: &str,
        timeout: Duration,
        settled: impl Fn(&[Value]) -> bool,
    ) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut latest: Option<Vec<Value>> = None;
            for note in notes.iter() {
                if note.get("method").and_then(Value::as_str)
                    != Some("textDocument/publishDiagnostics")
                {
                    continue;
                }
                let params = note.get("params").cloned().unwrap_or(Value::Null);
                if params.get("uri").and_then(Value::as_str) != Some(uri) {
                    continue;
                }
                latest = Some(
                    params
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Some(diags) = &latest
                && settled(diags)
            {
                return latest.unwrap_or_default();
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(notes);
                let last = latest.as_ref().map_or_else(
                    || "none published".to_owned(),
                    |diags| {
                        diags
                            .iter()
                            .map(|d| {
                                format!(
                                    "{} {}",
                                    d.get("code").and_then(Value::as_str).unwrap_or("?"),
                                    d.get("message").and_then(Value::as_str).unwrap_or("")
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" | ")
                    },
                );
                panic!(
                    "diagnostics for {uri:?} never settled within {timeout:?}; last published: [{last}]{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a `publishDiagnostics` for `uri` (optionally carrying exactly
    /// `version`) arrives; return the latest matching diagnostics array.
    pub fn await_diagnostics_version(
        &self,
        uri: &str,
        version: Option<i64>,
        timeout: Duration,
    ) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut matched: Option<Vec<Value>> = None;
            for note in notes.iter() {
                if note.get("method").and_then(Value::as_str)
                    != Some("textDocument/publishDiagnostics")
                {
                    continue;
                }
                let params = note.get("params").cloned().unwrap_or(Value::Null);
                if params.get("uri").and_then(Value::as_str) != Some(uri) {
                    continue;
                }
                if let Some(v) = version
                    && params.get("version").and_then(Value::as_i64) != Some(v)
                {
                    continue;
                }
                matched = Some(
                    params
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Some(diags) = matched {
                return diags;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Release the notifications lock before the (sleeping) probe so
                // the reader thread isn't blocked while we diagnose the timeout.
                drop(notes);
                panic!(
                    "no publishDiagnostics for {uri:?} version {version:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until the server's `[timing] diagnostics master-off` marker for
    /// `uri` is logged (searching entries at index `since` onward), then
    /// return the diagnostics from the `publishDiagnostics` immediately
    /// preceding it in the notification stream.
    ///
    /// This is the deterministic-barrier analogue of the VS Code extension
    /// suite's `waitForMasterOffDiagnostics` + `getDiagnostics(docUri)`
    /// (`editors/vscode/src/test/helper.ts`): the master switch
    /// (`tclLsp.features.diagnostics = false`) publishes its empty set and
    /// *then* logs this marker (`run_diagnostics_master_off` in
    /// `tcl-lsp-server/src/lib.rs`), so the marker is proof the publish this
    /// test cares about has already landed.
    ///
    /// Do **not** use [`Self::await_diagnostics_version`] for this: it
    /// returns as soon as *any* `publishDiagnostics` for `uri`/`version`
    /// matches, with no way to tell a publish computed under the old
    /// (diagnostics-on) config from one computed under the new
    /// (diagnostics-off) config apart — both carry the same document
    /// version, because a config change never bumps it. A `didOpen`'s own
    /// analysis can still have a later publish for that version in flight
    /// (e.g. a converged/cross-file correction) when the config flips off;
    /// if that stale publish lands in the buffer before the master-off one,
    /// `await_diagnostics_version` returns the stale non-empty result
    /// instead of waiting for the clear. Keying on the
    /// marker — and reading only the publish that precedes it — closes that
    /// window: this scans in one pass under the same lock used by the
    /// condvar wait, so there is no gap between "the marker was observed"
    /// and "the matching publish was read" for a later notification to land
    /// in.
    pub fn await_diagnostics_master_off(
        &self,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        self.await_marker_diagnostics("diagnostics master-off", uri, timeout, since)
    }

    /// Block until the server's `[timing] diagnostics excluded` marker for
    /// `uri` is logged, then return the diagnostics from the
    /// `publishDiagnostics` immediately preceding it — the
    /// `tclLsp.diagnostics.exclude` analogue of
    /// [`Self::await_diagnostics_master_off`], with the same rationale: the
    /// marker (`run_diagnostics_excluded` in `tcl-lsp-server/src/lib.rs`) is
    /// logged only after the exclusion's empty publish landed, so keying on it
    /// cannot confuse a pre-exclusion publish with the clear.
    pub fn await_diagnostics_excluded(
        &self,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        self.await_marker_diagnostics("diagnostics excluded", uri, timeout, since)
    }

    /// The shared scan behind the marker-keyed diagnostics barriers: find the
    /// first `window/logMessage` containing `marker` and `uri=<uri>` (from
    /// entry `since` onward) and return the last preceding
    /// `publishDiagnostics` for `uri`, waiting on the notification condvar
    /// until it appears or `timeout` (load-scaled) elapses.
    fn await_marker_diagnostics(
        &self,
        marker: &str,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        let needle_uri = format!("uri={uri}");
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut last_diags_before_marker: Option<Vec<Value>> = None;
            for note in notes.iter().skip(since) {
                match note.get("method").and_then(Value::as_str) {
                    Some("textDocument/publishDiagnostics") => {
                        let params = note.get("params").cloned().unwrap_or(Value::Null);
                        if params.get("uri").and_then(Value::as_str) == Some(uri) {
                            last_diags_before_marker = Some(
                                params
                                    .get("diagnostics")
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                    Some("window/logMessage") => {
                        let msg = note
                            .get("params")
                            .and_then(|p| p.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if msg.contains(marker) && msg.contains(&needle_uri) {
                            return last_diags_before_marker.unwrap_or_else(|| {
                                drop(notes);
                                panic!(
                                    "{marker} marker for {uri:?} logged with no preceding \
                                     publishDiagnostics for it — the marker/publish ordering \
                                     contract in the server broke"
                                )
                            });
                        }
                    }
                    _ => {}
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(notes);
                panic!(
                    "no {marker} marker for {uri:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }
}
