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

//! Reference-client + harsh-editing differential tests for semantic tokens.
//!
//! Issue #333 is an eglot painter bug, but proving that requires proving the
//! *server* never drifts: a correct client, driven through brutal editing,
//! must always be able to reconstruct exactly the tokens a cold reopen would
//! produce. If it can, any staleness a user sees in eglot is eglot's fault,
//! not ours.
//!
//! So this file is deliberately harsh and deliberately CI-friendly (pure Rust,
//! no Emacs). It carries two things the elisp harness can't:
//!
//!   * a **spec-correct reference semantic-tokens client** ([`SemtokState`])
//!     that maintains token state across `full` / `full/delta` responses the
//!     way the LSP spec says an editor should — the "known-good client" the
//!     eglot harness is compared against; and
//!
//!   * **harsh edit sequences** — 5000+ line files, rapid-fire incremental
//!     edits, edit-then-undo (the exact trigger the reporter calls out in
//!     issue #333) — after which the server's `semanticTokens/full` and the
//!     reference client's reconstruction must both equal a cold reopen.
//!
//! If any of these fail, the server itself is losing track and the bug is
//! (partly) ours. As long as they pass, the server tracks correctly and the
//! elisp harness's eglot drift is upstream.
//!
//! # Content assertions vs latency assertions (issue #1082)
//!
//! These tests mix two kinds of claim, and they are load-sensitive in opposite
//! ways. Keeping them apart is what makes the file deterministic under CPU
//! contention without retiring anything it proves:
//!
//! * **Content** — "the tokens equal a cold reopen", "the viewport converges on
//!   the enriched tier". These must hold on any machine, however slow. Their
//!   only timing element is the *barrier* that waits for the server to get
//!   there, and a barrier that expires early turns a correct server into a
//!   failed content test. Two rules follow. First, wait on an explicit
//!   **progress signal** rather than a stack of wall-clock polls: the server
//!   logs `semantic_tokens.{full,range}_convergence.settled (uri=…, refresh=…)`
//!   the instant a request's coarse-vs-enriched decision is made, so a waiter
//!   keys on that (the #1072 pattern) instead of racing the debounced
//!   `workspace/semanticTokens/refresh` that only *sometimes* follows it.
//!   Second, every remaining backstop in the harness is multiplied by the
//!   machine's measured capacity (`common::load_factor`), so it stays a hang
//!   guard rather than a speed test.
//!
//! * **Latency** — issue #829's guarantee that a cold/large file's *first*
//!   token response is never starved behind the whole-file analysis. This is a
//!   real promise about the server's design and deleting or `#[ignore]`-ing it
//!   would retire it, but as a wall-clock absolute it is a claim about the
//!   *machine*, not the server. These assertions therefore go through
//!   [`LatencyBudget`], which measures this very server's no-op round-trip
//!   first and scales the budget by the measured scheduling capacity — so a
//!   quiet machine still enforces the tight original bound, while a loaded one
//!   tests the same guarantee in the currency of the capacity it actually has.
//!   The one thing that never happens is a plain widening of the constant.
//!
//! A convergence *round* is therefore: request tokens, await the settled
//! marker, and — when it reports `refresh=true` — await the refresh and
//! re-request, exactly as an editor would. A marker reporting `refresh=false`
//! after a coarse response can mean a cancelled enriched read, or a differing
//! enriched stream whose repeat refresh was deliberately suppressed; the loop
//! re-races both. That is a genuine source of non-determinism the old fixed
//! `await_server_request` deadline could only absorb by being generous, and it
//! is why these tests hung precisely when the machine was slow enough for the
//! two to overlap.

use crate::common::helpers::{SemToken, decode_semantic_tokens};
use crate::common::{LatencyBudget, Lsp, scaled_timeout, unique_uri};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// A local mirror of a document's text plus its LSP version, so the test can
/// send spec-correct incremental `didChange` ranges and know the exact text
/// the server should now hold. Content is kept ASCII so UTF-16 columns,
/// byte offsets and char counts all coincide.
struct Mirror {
    uri: String,
    text: String,
    version: i64,
}

impl Mirror {
    fn new(uri: &str, text: &str) -> Self {
        Self {
            uri: uri.to_owned(),
            text: text.to_owned(),
            version: 1,
        }
    }

    /// Byte offset of a (line, character) LSP position in the current text.
    fn offset_of(&self, line: usize, character: usize) -> usize {
        let mut off = 0usize;
        for (i, l) in self.text.split_inclusive('\n').enumerate() {
            if i == line {
                return off + character.min(l.trim_end_matches('\n').len());
            }
            off += l.len();
        }
        self.text.len()
    }

    /// Apply a single-range incremental edit locally and send the matching
    /// `didChange`, then block until the server has settled that version.
    fn edit(
        &mut self,
        lsp: &mut Lsp,
        (sl, sc): (usize, usize),
        (el, ec): (usize, usize),
        new_text: &str,
    ) {
        let start = self.offset_of(sl, sc);
        let end = self.offset_of(el, ec);
        self.text.replace_range(start..end, new_text);
        self.version += 1;
        lsp.change_document(
            &self.uri,
            self.version,
            json!([{
                "range": {
                    "start": { "line": sl, "character": sc },
                    "end": { "line": el, "character": ec },
                },
                "text": new_text,
            }]),
        );
        lsp.await_diagnostics_version(
            &self.uri,
            Some(self.version),
            std::time::Duration::from_secs(20),
        );
    }

    /// Fire a `didChange` without waiting for the server to settle — used to
    /// build rapid-fire bursts that mirror a user typing faster than the
    /// server can answer.
    fn edit_no_wait(
        &mut self,
        lsp: &mut Lsp,
        (sl, sc): (usize, usize),
        (el, ec): (usize, usize),
        new_text: &str,
    ) {
        let start = self.offset_of(sl, sc);
        let end = self.offset_of(el, ec);
        self.text.replace_range(start..end, new_text);
        self.version += 1;
        lsp.change_document(
            &self.uri,
            self.version,
            json!([{
                "range": {
                    "start": { "line": sl, "character": sc },
                    "end": { "line": el, "character": ec },
                },
                "text": new_text,
            }]),
        );
    }
}

/// A spec-correct semantic-tokens client: it maintains the packed token array
/// across `full` and `full/delta` responses exactly as the LSP specification
/// prescribes (§ `SemanticTokensDelta`), so its reconstruction is the
/// ground-truth an editor *should* arrive at. This is the "separate client
/// proven to track correctly" the server is measured against.
#[derive(Default)]
struct SemtokState {
    data: Vec<i64>,
    result_id: Option<String>,
}

impl SemtokState {
    /// Absorb a `full` result: replace the whole array.
    fn apply_full(&mut self, result: &Value) {
        self.data = as_i64_vec(result.get("data"));
        self.result_id = result
            .get("resultId")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }

    /// Absorb a `full/delta` result: apply `edits` in-place if present,
    /// otherwise fall back to the full `data` array (a server may always
    /// answer a delta request with a full stream — ours does).
    fn apply_delta(&mut self, result: &Value) {
        if let Some(edits) = result.get("edits").and_then(Value::as_array) {
            // Edits are ordered by ascending `start` and are non-overlapping;
            // apply them right-to-left so earlier offsets stay valid.
            let mut edits: Vec<&Value> = edits.iter().collect();
            edits.sort_by_key(|e| e.get("start").and_then(Value::as_i64).unwrap_or(0));
            for e in edits.into_iter().rev() {
                let start = usize::try_from(e.get("start").and_then(Value::as_i64).unwrap_or(0))
                    .unwrap_or(0);
                let del =
                    usize::try_from(e.get("deleteCount").and_then(Value::as_i64).unwrap_or(0))
                        .unwrap_or(0);
                let ins = as_i64_vec(e.get("data"));
                let start = start.min(self.data.len());
                let end = (start + del).min(self.data.len());
                self.data.splice(start..end, ins);
            }
        } else if result.get("data").is_some() {
            self.data = as_i64_vec(result.get("data"));
        }
        if let Some(id) = result.get("resultId").and_then(Value::as_str) {
            self.result_id = Some(id.to_owned());
        }
    }

    /// Decode the maintained array into absolute tokens.
    fn tokens(&self) -> Vec<SemToken> {
        decode_semantic_tokens(&json!({ "data": self.data }))
    }
}

fn as_i64_vec(v: Option<&Value>) -> Vec<i64> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

/// Compare two token lists for exact structural equality (position, length,
/// type, modifiers).
fn same_tokens(a: &[SemToken], b: &[SemToken]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.line == y.line
                && x.char == y.char
                && x.length == y.length
                && x.ttype == y.ttype
                && x.modifiers == y.modifiers
        })
}

/// First index where two token lists diverge, for readable failure messages.
fn first_divergence(server: &[SemToken], truth: &[SemToken]) -> String {
    let n = server.len().max(truth.len());
    for i in 0..n {
        match (server.get(i), truth.get(i)) {
            (Some(sv), Some(tv))
                if same_tokens(std::slice::from_ref(sv), std::slice::from_ref(tv)) => {}
            (sv, tv) => return format!("index {i}: server={sv:?} truth={tv:?}"),
        }
    }
    "identical".to_owned()
}

/// Backstop for settling one of this file's deliberately-pathological
/// documents (`generate_big_tcl(600)` is ~6000 lines).
///
/// A full debug-build analysis of that much Tcl, on a machine already running
/// the rest of the suite in parallel, is legitimately tens of seconds — so the
/// harness default stops being a hang guard here and becomes a bet on how many
/// cores this test happened to get. This is not a latency budget: nothing is
/// asserted by its size, and the guarantee about *response* latency lives in
/// [`LatencyBudget`], which is a separate, measured thing.
const BIG_DOC_SETTLE: Duration = Duration::from_mins(3);

/// How long one [`converge_via_refresh`] round may wait for its settled marker
/// before the loop gives up on *that round* and re-requests.
///
/// Load-scaled inside `try_await_log`, and deliberately a small fraction of the
/// (also load-scaled) outer convergence budget, so several rounds always fit
/// whatever the machine's capacity factor turns out to be.
const ROUND_SETTLE: Duration = Duration::from_secs(20);

/// How long a `refresh=true` decision has to actually produce the
/// `workspace/semanticTokens/refresh` request.
///
/// Its own budget rather than the convergence loop's leftover: the decision has
/// already been logged, so this times the refresh path alone (load-scaled inside
/// `await_server_request`).
const REFRESH_ARRIVAL: Duration = Duration::from_secs(20);

/// The tokens a cold reopen of `text` produces — the ground truth for what the
/// server *should* be serving after any edit sequence that lands on `text`.
fn cold_tokens(lsp: &mut Lsp, text: &str) -> Vec<SemToken> {
    let uri = unique_uri("tcl");
    lsp.open_ready_timeout(&uri, text, BIG_DOC_SETTLE);
    let raw = lsp.semantic_tokens(&uri);
    lsp.close_document(&uri);
    decode_semantic_tokens(&raw)
}

/// The enriched **range** tokens for `text`'s viewport `[start, end)`: a fresh
/// document is fully settled (so the compilation unit + analysis are memoised),
/// then a range request is made, which therefore returns the enriched tier. The
/// truth a coarse-then-converge range response (#844 Gap 4) must land on.
fn cold_range_tokens(
    lsp: &mut Lsp,
    text: &str,
    start: (u32, u32),
    end: (u32, u32),
) -> Vec<SemToken> {
    let uri = unique_uri("tcl");
    lsp.open_ready_timeout(&uri, text, BIG_DOC_SETTLE);
    let raw = lsp.semantic_tokens_range(&uri, start, end);
    lsp.close_document(&uri);
    decode_semantic_tokens(&raw)
}

/// Request `full/delta` against `uri` with `prev` as `previousResultId`.
fn request_delta(lsp: &mut Lsp, uri: &str, prev: Option<&str>) -> Value {
    let mut params = json!({ "textDocument": { "uri": uri } });
    if let Some(p) = prev {
        params["previousResultId"] = json!(p);
    }
    lsp.request("textDocument/semanticTokens/full/delta", params)
}

/// After the mirror has settled on some text, assert that BOTH the server's
/// `full` response AND a reference client's `full/delta` reconstruction equal a
/// cold reopen. `client` is threaded through so delta application chains across
/// steps like a real editor's would.
fn assert_tracks(lsp: &mut Lsp, mirror: &Mirror, client: &mut SemtokState, label: &str) {
    let truth = cold_tokens(lsp, &mirror.text);

    let full_raw = lsp.semantic_tokens(&mirror.uri);
    let server_full = decode_semantic_tokens(&full_raw);
    assert!(
        same_tokens(&server_full, &truth),
        "[{label}] server `full` drifted from a cold reopen — {}",
        first_divergence(&server_full, &truth)
    );

    let delta_raw = request_delta(lsp, &mirror.uri, client.result_id.as_deref());
    client.apply_delta(&delta_raw);
    let client_tokens = client.tokens();
    assert!(
        same_tokens(&client_tokens, &truth),
        "[{label}] reference client drifted after applying full/delta — {}",
        first_divergence(&client_tokens, &truth)
    );
}

/// The outcome of driving a cold document to its settled enriched token stream
/// (see [`converge_via_refresh`]).
struct Convergence {
    /// The token stream the last request returned.
    tokens: Vec<SemToken>,
    /// How many convergence rounds were needed.
    rounds: usize,
    /// How many settled markers reported `refresh=true` — i.e. how many times
    /// the server decided the served (coarse) stream was worth replacing.
    refresh_decisions: usize,
    /// How many `workspace/semanticTokens/refresh` requests actually reached
    /// the client.
    refreshes: usize,
    /// How many rounds settled with the enriched read cancelled or already
    /// equal to what was served (`refresh=false`).
    no_refresh_decisions: usize,
}

/// What a single convergence round asks for: `full` or a fixed viewport.
#[derive(Clone, Copy)]
enum Tier {
    Full,
    Range { start: (u32, u32), end: (u32, u32) },
}

impl Tier {
    /// The settled-marker needle the server logs for this request kind.
    fn settled_marker(self) -> &'static str {
        match self {
            Self::Full => "semantic_tokens.full_convergence.settled",
            Self::Range { .. } => "semantic_tokens.range_convergence.settled",
        }
    }

    fn request(self, lsp: &mut Lsp, uri: &str) -> Vec<SemToken> {
        match self {
            Self::Full => decode_semantic_tokens(&lsp.semantic_tokens(uri)),
            Self::Range { start, end } => {
                decode_semantic_tokens(&lsp.semantic_tokens_range(uri, start, end))
            }
        }
    }
}

/// Drive `uri` to the settled enriched stream the way a conformant client does,
/// reporting what the server needed to get there.
///
/// Each round: snapshot the request/log cursors, ask for tokens, and — if they
/// are not yet `truth` — **await the convergence continuation's settled
/// marker**. That marker is the explicit progress signal this loop is built on
/// (#1072): it is logged the instant the coarse-vs-enriched decision for that
/// request is made, whatever the decision was, so the loop never has to guess
/// whether a refresh is still coming. When the marker says `refresh=true` — or
/// records a coalesced request whose holder will schedule one — the refresh
/// request must then actually arrive (asserted — that is the mechanism under
/// test). Full-token markers distinguish a response served enriched from a
/// detached comparison; a cancelled read or repeat-suppressed refresh re-races,
/// while a compared-equal response is final.
///
/// `budget` bounds the whole loop. It is a hang guard: every round makes real
/// progress against an explicit signal, so reaching it means the server never
/// converged at all.
///
/// # Scaling (review of #1089)
///
/// The outer deadline is `scaled_timeout(budget)`, not `budget`. Every wait
/// *inside* the loop goes through a harness `await_*`/`try_await_*` helper,
/// each of which multiplies its argument by [`common::load_factor`]
/// internally. An unscaled outer deadline is therefore not merely tighter than
/// the rounds it contains — it is inconsistent with them: on a machine where
/// the factor is 4, one legitimate round-settle wait (capped at
/// [`ROUND_SETTLE`]) already covers 80 s of a fixed 60 s outer budget, so a
/// round that behaved perfectly could exhaust the loop.
///
/// For the same reason the post-decision refresh wait below gets its own fixed
/// [`REFRESH_ARRIVAL`] budget rather than whatever is left of the outer one.
/// Handing it the leftover made its deadline a function of how long the
/// *previous* wait took: a round that legitimately consumed the budget left it
/// zero, and `await_server_request(.., 0, ..)` fails instantly — recreating
/// exactly the load-sensitive failure #1082 removed. Its own budget is the
/// honest bound anyway: by the time `refresh=true` has been logged the
/// debounced refresh is already scheduled, so what is being timed is the
/// refresh path, not the convergence.
fn converge_via_refresh(
    lsp: &mut Lsp,
    uri: &str,
    tier: Tier,
    truth: &[SemToken],
    budget: Duration,
) -> Convergence {
    let deadline = Instant::now() + scaled_timeout(budget);
    let mut out = Convergence {
        tokens: Vec::new(),
        rounds: 0,
        refresh_decisions: 0,
        refreshes: 0,
        no_refresh_decisions: 0,
    };
    loop {
        let req_since = lsp.server_request_cursor();
        let log_since = lsp.notification_cursor();
        out.tokens = tier.request(lsp, uri);
        out.rounds += 1;
        if same_tokens(&out.tokens, truth) {
            return out;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "[{}] never converged on the enriched stream after {} rounds \
             ({} refresh decisions, {} refreshes delivered, {} rounds whose enriched read was \
             cancelled or already equal) — {}",
            tier.settled_marker(),
            out.rounds,
            out.refresh_decisions,
            out.refreshes,
            out.no_refresh_decisions,
            first_divergence(&out.tokens, truth),
        );
        // A round can legitimately settle *without* a marker: the enriched
        // reads land inside the budget but the document has no salsa handles
        // (a `did_close` racing the two separate `db_files` reads), so the
        // coarse tier is served with no continuation at all. That is not a
        // failure, just a round that made no progress — re-request rather than
        // give up, and let the outer deadline be the only hang guard.
        let Some(settled) = lsp.try_await_log(
            &[tier.settled_marker(), uri],
            remaining.min(ROUND_SETTLE),
            log_since,
        ) else {
            out.no_refresh_decisions += 1;
            continue;
        };
        if settled.contains("refresh=true") || settled.contains("outcome=coalesced") {
            out.refresh_decisions += 1;
            // A coalesced request's holder schedules the refresh when it
            // releases its claim; otherwise the decision has already scheduled
            // it. This can only expire if that refresh path is broken, which is
            // exactly the regression worth failing on. Its own budget
            // (load-scaled inside the helper), never the outer leftover — see
            // this function's docs.
            lsp.await_server_request(
                "workspace/semanticTokens/refresh",
                REFRESH_ARRIVAL,
                req_since,
            );
            out.refreshes += 1;
        } else if matches!(tier, Tier::Full)
            && (settled.contains("outcome=cancelled")
                || settled.contains("outcome=refresh-suppressed"))
        {
            // Nothing was compared, or the server deliberately suppressed a
            // second refresh for identical differing bytes. The next request
            // may serve the enriched stream directly, so re-race it without
            // weakening the server's repeat-ask bound.
            out.no_refresh_decisions += 1;
        } else if matches!(tier, Tier::Full)
            && (settled.contains("outcome=served-enriched")
                || settled.contains("outcome=compared-equal")
                || settled.contains("outcome=no-analysis"))
        {
            // The server has settled and is asking for nothing further: this
            // stream is final. If it still differs from the truth that is a
            // genuine content divergence, which the caller reports — spinning
            // here would only turn it into an unhelpful timeout.
            out.no_refresh_decisions += 1;
            return out;
        } else if matches!(tier, Tier::Range { .. }) {
            if settled.contains("outcome=cancelled") {
                // Nothing was compared — a concurrent salsa write killed the
                // read. The next request re-races it.
                out.no_refresh_decisions += 1;
                continue;
            }
            out.no_refresh_decisions += 1;
            return out;
        } else {
            panic!("unexpected full semantic-token convergence marker: {settled}");
        }
    }
}

/// Open `uri` with `text`, seed the reference client from the first `full`.
fn open_and_seed(lsp: &mut Lsp, uri: &str, text: &str) -> (Mirror, SemtokState) {
    lsp.open_ready(uri, text);
    let mut client = SemtokState::default();
    let full = lsp.semantic_tokens(uri);
    client.apply_full(&full);
    (Mirror::new(uri, text), client)
}

// -- tests ----------------------------------------------------------------

/// The reference client and the server must stay in lock-step through a mix of
/// inserts, deletes, renames and comment toggles applied as incremental edits,
/// checked after *every* edit.
#[test]
fn reference_client_tracks_through_mixed_edits() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let initial = "namespace eval ::myns {}\n\
                   set iniFile config.ini\n\
                   proc compute {a b} {\n\
                   \x20   set total [expr {$a + $b}]\n\
                   \x20   return $total\n\
                   }\n";
    let (mut mirror, mut client) = open_and_seed(&mut lsp, &uri, initial);
    assert_tracks(&mut lsp, &mirror, &mut client, "seed");

    // Rename `total` -> `grand_total` on the `set` line (line 3).
    mirror.edit(&mut lsp, (3, 8), (3, 13), "grand_total");
    assert_tracks(&mut lsp, &mirror, &mut client, "rename-set");

    // Prepend a comment line (shifts every token down one line).
    mirror.edit(&mut lsp, (0, 0), (0, 0), "# header comment\n");
    assert_tracks(&mut lsp, &mirror, &mut client, "prepend-comment");

    // Change the arithmetic operator inside the expr.
    // `$a + $b` now on line 4; replace the "+" with "*".
    let plus = mirror.text.find("$a + $b").expect("expr present") + 3;
    let (pl, pc) = offset_to_line_char(&mirror.text, plus);
    mirror.edit(&mut lsp, (pl, pc), (pl, pc + 1), "*");
    assert_tracks(&mut lsp, &mirror, &mut client, "op-swap");

    // Delete the whole `return` line.
    let ret_line = mirror
        .text
        .lines()
        .position(|l| l.contains("return"))
        .expect("return line");
    mirror.edit(&mut lsp, (ret_line, 0), (ret_line + 1, 0), "");
    assert_tracks(&mut lsp, &mirror, &mut client, "delete-return");
}

/// The server must answer a `full/delta` whose `previousResultId` matches the
/// last stream we served with a real minimal edit — `edits` present, no full
/// `data` re-send — like rust-analyzer. This is the payload win that keeps a
/// client's per-edit token round-trip (and eglot's stale-repaint window,
/// issue #333) small; a regression back to "always full" would silently undo
/// it, so pin the response shape.
#[test]
fn server_returns_real_delta_not_full_resend() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let initial = "namespace eval ::n {}\n\
                   proc a {x} { return [expr {$x + 1}] }\n\
                   proc b {y} { return [expr {$y * 2}] }\n";
    lsp.open_ready(&uri, initial);

    // Baseline full — capture the resultId the delta must reference.
    let full = lsp.semantic_tokens(&uri);
    let prev = full["resultId"].as_str().expect("full resultId").to_owned();
    assert!(
        full.get("data").and_then(Value::as_array).is_some(),
        "full response must carry data; got {full:#}"
    );

    // A token-changing edit: insert a whole new proc line up top.
    let mut mirror = Mirror::new(&uri, initial);
    mirror.edit(&mut lsp, (0, 0), (0, 0), "proc c {z} { return $z }\n");

    let resp = request_delta(&mut lsp, &uri, Some(&prev));
    assert!(
        resp.get("edits").and_then(Value::as_array).is_some(),
        "matching-resultId delta must return `edits`, not a full re-send; got {resp:#}"
    );
    assert!(
        resp.get("data").is_none(),
        "a delta response must not also carry a full `data` array; got {resp:#}"
    );

    // The reference client applies that delta and must match a cold reopen.
    let mut client = SemtokState::default();
    client.apply_full(&full);
    client.apply_delta(&resp);
    let truth = cold_tokens(&mut lsp, &mirror.text);
    assert!(
        same_tokens(&client.tokens(), &truth),
        "reference client diverged after applying the delta — {}",
        first_divergence(&client.tokens(), &truth)
    );

    // An unknown `previousResultId` must fall back to a full stream (safe).
    let fallback = request_delta(&mut lsp, &uri, Some("st-does-not-exist"));
    assert!(
        fallback.get("data").and_then(Value::as_array).is_some(),
        "unknown previousResultId must fall back to full `data`; got {fallback:#}"
    );
}

/// Edit-then-undo is the exact trigger the reporter names in issue #333: make a
/// change (server recomputes), then revert it (server recomputes back). The
/// server must land on byte-identical tokens to before the edit — no residue.
#[test]
fn reference_client_tracks_edit_then_undo() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let initial = "set iniFile config.ini\n\
                   if {[::ini::exists $iniFile Options solver]} {\n\
                   \x20   ::ini::set $iniFile Options solver 0\n\
                   } else {\n\
                   \x20   ::ini::set $iniFile Options Solver 0\n\
                   }\n\
                   proc compute {a b} { return [expr {$a + $b}] }\n";
    let (mut mirror, mut client) = open_and_seed(&mut lsp, &uri, initial);
    let baseline = cold_tokens(&mut lsp, initial);
    assert_tracks(&mut lsp, &mirror, &mut client, "undo-seed");

    for round in 0..8 {
        // Edit: rename `Solver` -> `Disabled` on line 4.
        let sol = mirror.text.find("Solver 0").expect("Solver present");
        let (l, c) = offset_to_line_char(&mirror.text, sol);
        mirror.edit(&mut lsp, (l, c), (l, c + 6), "Disabled");
        assert_tracks(&mut lsp, &mirror, &mut client, "undo-edit");

        // Undo: rename it back.
        let dis = mirror.text.find("Disabled 0").expect("Disabled present");
        let (l, c) = offset_to_line_char(&mirror.text, dis);
        mirror.edit(&mut lsp, (l, c), (l, c + 8), "Solver");
        assert_tracks(&mut lsp, &mirror, &mut client, "undo-revert");

        // After a full edit/undo round the server must be byte-identical to
        // the original cold tokens — no drift accreted over the round.
        let now = decode_semantic_tokens(&lsp.semantic_tokens(&mirror.uri));
        assert!(
            same_tokens(&now, &baseline),
            "round {round}: tokens drifted after edit+undo — {}",
            first_divergence(&now, &baseline)
        );
    }
}

/// Rapid-fire bursts: fire several incremental edits without waiting, then
/// settle. The final `full` (and the reference client) must match a cold
/// reopen of the final text — coalesced didChanges must not desync the server.
#[test]
fn reference_client_tracks_rapid_fire_bursts() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let initial = "proc f {a b} {\n    set x 1\n    set y 2\n    set z 3\n    return $x\n}\n";
    let (mut mirror, mut client) = open_and_seed(&mut lsp, &uri, initial);

    for burst in 0..6 {
        // Three edits without waiting, then a settling edit.
        mirror.edit_no_wait(&mut lsp, (0, 0), (0, 0), "# burst\n");
        mirror.edit_no_wait(&mut lsp, (2, 8), (2, 9), "1");
        mirror.edit_no_wait(&mut lsp, (3, 8), (3, 9), "2");
        // Settling edit (waits): toggles a trailing comment.
        let last = mirror.text.lines().count();
        mirror.edit(
            &mut lsp,
            (last, 0),
            (last, 0),
            &format!("# settle {burst}\n"),
        );
        assert_tracks(&mut lsp, &mirror, &mut client, "rapid-settle");
    }
}

/// Large-file latency + correctness. The reporter says tokens take "several
/// seconds" on 5000+ LOC files and editing loses track. Measure the cold and
/// post-edit `full` latency and assert the server still returns correct tokens
/// after an edit deep in a big file. The timing is printed (see with
/// `--nocapture`); the assertion bound is generous so this is a smoke gate on
/// pathological latency, not a flaky micro-benchmark.
#[test]
fn large_file_latency_and_correctness() {
    let mut lsp = Lsp::tcl();
    // Measured before anything is opened, so the no-op sample times an idle
    // server rather than one mid-analysis (see [`LatencyBudget`]).
    let budget = LatencyBudget::probe(&mut lsp, Duration::from_secs(2));
    let uri = unique_uri("tcl");
    let big = generate_big_tcl(600); // ~5200 lines.
    let line_count = big.lines().count();

    // Open WITHOUT waiting for diagnostics, so the first `semanticTokens/full`
    // measures the isolated token round-trip (cold parse + tokenise + encode),
    // not the diagnostics pipeline — this is the latency eglot actually waits
    // on when it asks for tokens.
    lsp.open_document_lang(&uri, &big, "tcl", 1);
    let t_cold = Instant::now();
    let cold = lsp.semantic_tokens(&uri);
    let cold_ms = t_cold.elapsed().as_millis();
    let cold_toks = decode_semantic_tokens(&cold);
    assert!(
        cold_toks.len() > 1000,
        "expected a rich token stream for a {line_count}-line file, got {}",
        cold_toks.len()
    );

    // Warm round-trip — the memoised query should answer near-instantly.
    let t_warm = Instant::now();
    let _ = lsp.semantic_tokens(&uri);
    let warm_ms = t_warm.elapsed().as_millis();

    // Edit deep in the file, then time tokens WITHOUT first waiting on
    // diagnostics — the realistic "type a char, how long until fresh tokens"
    // path from issue #333.
    let mut mirror = Mirror::new(&uri, &big);
    let needle = "set v300 [expr {$v300 + 1}]";
    let pos = mirror.text.find(needle).expect("deep needle present");
    let (l, c) = offset_to_line_char(&mirror.text, pos);
    mirror.edit_no_wait(&mut lsp, (l, c + 4), (l, c + 8), "v_300"); // v300 -> v_300
    let t_edit = Instant::now();
    let edit_full = lsp.semantic_tokens(&uri);
    let edit_ms = t_edit.elapsed().as_millis();

    // Settle and assert correctness against a cold reopen.
    lsp.await_diagnostics_version(&uri, Some(mirror.version), BIG_DOC_SETTLE);
    let truth = cold_tokens(&mut lsp, &mirror.text);
    let after = decode_semantic_tokens(&edit_full);
    assert!(
        same_tokens(&after, &truth),
        "post-edit tokens on a big file drifted from a cold reopen — {}",
        first_divergence(&after, &truth)
    );

    eprintln!(
        "large_file_latency: {line_count} lines, {} tokens; \
         cold full={cold_ms}ms, warm full={warm_ms}ms, post-edit full={edit_ms}ms",
        cold_toks.len()
    );
    // Pathological-latency guard. A native tokeniser on ~5k lines should answer
    // in well under a second; multiple seconds means an O(n^2) regression in
    // the token path (the issue #333 latency the reporter feels as a "hang").
    // Debug builds are ~10-20x slower than the shipped release binary, so this
    // strict gate only runs for release builds; debug runs just print timings.
    //
    // The 2s bound is what an unloaded machine is held to; on a contended one
    // the same claim is made against the capacity that machine actually has
    // (see [`LatencyBudget`]) rather than against the wall clock, so an
    // O(n^2) regression still fails while CPU starvation does not.
    if cfg!(not(debug_assertions)) {
        let elapsed = std::time::Duration::from_millis(
            u64::try_from(edit_ms).expect("post-edit latency fits u64"),
        );
        assert!(
            budget.allows(elapsed),
            "post-edit semanticTokens/full took {edit_ms}ms on {line_count} lines \
             (release) — unexpectedly slow (issue #333 latency regression?); {}",
            budget.report(),
        );
    }
}

/// Issue #829: the *first* `semanticTokens/full` response for a large,
/// freshly-opened file — requested with no wait after `didOpen`, so nothing
/// has warmed the analysis yet — must never be starved behind the whole-file
/// analysis. This is the reported symptom verbatim: "in large files with many
/// diagnostic messages, the highlighting sometimes does not arrive at all
/// until forced again... It seems like there is a timeout after which the
/// editors give up." `generate_big_tcl(600)` is the same fixture
/// `large_file_latency_and_correctness` uses, whose measured *release-mode*
/// cold-enriched latency (~110ms) already comfortably exceeds
/// `SEMANTIC_TOKENS_FAST_PATH_BUDGET` (40ms) — so this request reliably
/// exercises the fast-path fallback, not just a coincidentally-fast enriched
/// computation.
#[test]
fn large_file_first_semantic_tokens_response_is_prompt() {
    let mut lsp = Lsp::tcl();
    let budget = LatencyBudget::probe(&mut lsp, Duration::from_secs(10));
    let uri = unique_uri("tcl");
    let big = generate_big_tcl(600); // ~6000 lines.
    let line_count = big.lines().count();
    lsp.open_document_lang(&uri, &big, "tcl", 1);

    let started = Instant::now();
    let first = lsp.semantic_tokens(&uri);
    let elapsed = started.elapsed();

    let toks = decode_semantic_tokens(&first);
    assert!(
        !toks.is_empty(),
        "the first response must carry real tokens (at minimum the coarse \
         tier), not an empty placeholder"
    );
    eprintln!(
        "large_file_first_semantic_tokens_response_is_prompt: {line_count} lines, \
         {} tokens in {elapsed:?}",
        toks.len(),
    );
    // Generous, environment-tolerant bound (unlike the release-only strict
    // gate above): the whole point of the fast-path/coarse-fallback design is
    // that first-response latency stops scaling with file size, so this holds
    // in debug builds too — not just release. Under CPU contention the same
    // claim is measured against this machine's own capacity rather than the
    // wall clock (see [`LatencyBudget`]): the guarantee is not that the
    // response takes under ten seconds on any hardware, it is that it does not
    // wait for the whole-file analysis.
    assert!(
        budget.allows(elapsed),
        "first semanticTokens/full on a {line_count}-line cold file took \
         {elapsed:?} — semantic tokens must never be starved behind the \
         whole-file analysis (issue #829); {}",
        budget.report(),
    );
}

#[test]
fn large_file_range_semantic_tokens_response_is_prompt() {
    // Mirrors the `full` test above for `semanticTokens/range`: a cold/large
    // indexed file must not block a viewport request on the whole-file
    // compilation-unit build + incremental analysis (issue #829's residual
    // gap for range requests -- `db_compilation_unit`/`db_file_analysis`
    // compute their query to completion with no fast-path budget, unlike
    // `semantic_tokens_full`'s race against `SEMANTIC_TOKENS_FAST_PATH_BUDGET`).
    let mut lsp = Lsp::tcl();
    let budget = LatencyBudget::probe(&mut lsp, Duration::from_secs(10));
    let uri = unique_uri("tcl");
    let big = generate_big_tcl(600); // ~6000 lines.
    let line_count = big.lines().count();
    lsp.open_document_lang(&uri, &big, "tcl", 1);

    // A viewport near the end of the file: if the fallback ever failed to
    // filter to the requested range (e.g. silently serving the whole
    // document), this range would still assert non-empty, but decoding
    // confirms every token's line falls inside it.
    let start = (u32::try_from(line_count).unwrap() - 20, 0);
    let end = (u32::try_from(line_count).unwrap(), 0);

    let started = Instant::now();
    let first = lsp.semantic_tokens_range(&uri, start, end);
    let elapsed = started.elapsed();

    let toks = decode_semantic_tokens(&first);
    assert!(
        !toks.is_empty(),
        "the first range response must carry real tokens (at minimum the \
         coarse tier), not an empty placeholder"
    );
    assert!(
        toks.iter()
            .all(|t| t.line >= i64::from(start.0) && t.line < i64::from(end.0)),
        "every token in a range response must fall inside the requested \
         viewport: {toks:?}",
    );
    eprintln!(
        "large_file_range_semantic_tokens_response_is_prompt: {line_count} lines, \
         {} tokens in {elapsed:?}",
        toks.len(),
    );
    assert!(
        budget.allows(elapsed),
        "first semanticTokens/range on a {line_count}-line cold file took \
         {elapsed:?} — a viewport request must never be starved behind the \
         whole-file analysis (issue #829); {}",
        budget.report(),
    );
}

/// Issue #829, the other half of the loop the previous test proves the start
/// of: when the first token request for a large/cold file is served from the
/// cheap coarse tier (the enriched computation did not land within the
/// fast-path budget), the server must eventually push
/// `workspace/semanticTokens/refresh` once the enriched result lands, so the
/// editor re-requests and converges on the fully enriched tokens — "do the
/// bulk of the semantic tokens first, then update the ones resolved in
/// deeper analysis."
#[test]
fn large_file_semantic_tokens_refresh_delivers_enriched_result() {
    let mut lsp = Lsp::tcl();
    // `generate_big_tcl` alone uses no construct the enriched tier treats
    // differently from the coarse one (no `regexp`, no object dispatch), so
    // the coarse and enriched streams would be byte-identical regardless of
    // which one served the request — useless for detecting which tier
    // actually won. Append one proc with a provably-constant regex source
    // (the same shape `semantic_tokens_retags_constant_regex_source_true_positive`
    // in tcl-lsp-db proves the enriched tier retags) so `same_tokens` below
    // can actually tell the two tiers apart.
    let big = format!(
        "{}\nproc ::bench::regex_check {{}} {{\n    set my_re \".*abc\"\n    regexp $my_re $s\n}}\n",
        generate_big_tcl(600)
    );

    // Compute the enriched truth from a settled separate document *before* the
    // document under test exists. Opening (or closing) any document mutates the
    // salsa `Project` (`set_files`), which cancels an in-flight convergence
    // continuation — so computing the truth *after* the first request, as this
    // test used to, could cancel the very enriched read whose refresh it then
    // waited fifteen seconds for. That window widens with machine load, which
    // is exactly the shape of the flake in issue #1082. Done up front it cannot
    // interfere. (The range twin below has always done it this way.)
    let truth = cold_tokens(&mut lsp, &big);

    let uri = unique_uri("tcl");
    lsp.open_document_lang(&uri, &big, "tcl", 1);

    let converged = converge_via_refresh(
        &mut lsp,
        &uri,
        Tier::Full,
        &truth,
        // A hang guard only: every round waits on the settled marker, so this
        // expires solely if the server never makes a convergence decision.
        Duration::from_mins(1),
    );

    // `converge_via_refresh` has already asserted that every `refresh=true`
    // decision produced a real `workspace/semanticTokens/refresh`. The content
    // claim is asserted unconditionally: however the tiers raced, a settled
    // document must serve the fully enriched stream.
    assert!(
        same_tokens(&converged.tokens, &truth),
        "after workspace/semanticTokens/refresh, a re-request must return the \
         fully enriched tokens — {}",
        first_divergence(&converged.tokens, &truth)
    );
    if converged.refreshes == 0 {
        // The race was won this run (fast machine / the debounced diagnostics
        // worker happened to prime the analysis first): the first response was
        // already fully enriched, so there was nothing to refresh. The tiering
        // mechanism itself is covered deterministically by tcl-lsp-db's
        // `semantic_tokens_retags_constant_regex_source_true_positive` and
        // `semantic_tokens_shares_incremental_analysis_with_diagnostics`.
        eprintln!(
            "large_file_semantic_tokens_refresh_delivers_enriched_result: \
             fast path won the race, refresh not exercised this run"
        );
    }
    eprintln!(
        "large_file_semantic_tokens_refresh_delivers_enriched_result: converged in {} rounds \
         ({} refreshes; {} rounds settled with a cancelled/equal enriched read)",
        converged.rounds, converged.refreshes, converged.no_refresh_decisions,
    );
}

/// #844 Gap 4: the range analogue of the `_full` convergence test above. A
/// cold/large viewport is served the coarse tier immediately (the enriched
/// CU/analysis overrun the 40ms budget); the server must then push a
/// `workspace/semanticTokens/refresh` once they land and the viewport genuinely
/// differs, so a static cold viewport converges on the enriched tier instead of
/// staying coarse until the next scroll or edit (the gap `semantic_tokens_range`
/// had that `_full` did not).
#[test]
fn large_file_range_semantic_tokens_converges_via_refresh() {
    let mut lsp = Lsp::tcl();
    // The same provably-constant regex source the `_full` test uses, so the
    // enriched tier (which retags it) differs from the coarse tier — otherwise
    // the two are byte-identical and no refresh is warranted.
    let big = format!(
        "{}\nproc ::bench::regex_check {{}} {{\n    set my_re \".*abc\"\n    regexp $my_re $s\n}}\n",
        generate_big_tcl(600)
    );
    let line_count = u32::try_from(big.lines().count()).unwrap();

    // A viewport over the appended regexp proc (the last few lines), where the
    // coarse and enriched tiers provably differ.
    let start = (line_count.saturating_sub(6), 0);
    let end = (line_count, 0);

    // Compute the enriched truth from a settled separate document *before* the
    // main document exists: opening (or closing) any document mutates the salsa
    // `Project` (`set_files`), which cancels an in-flight convergence
    // continuation — so doing it mid-test would suppress the very refresh under
    // test.  Done up front, it cannot interfere.
    let truth = cold_range_tokens(&mut lsp, &big, start, end);

    let uri = unique_uri("tcl");
    lsp.open_document_lang(&uri, &big, "tcl", 1);

    let converged = converge_via_refresh(
        &mut lsp,
        &uri,
        Tier::Range { start, end },
        &truth,
        // A hang guard only — see the `_full` twin.
        Duration::from_mins(1),
    );
    assert!(
        !converged.tokens.is_empty(),
        "the cold range must still serve the coarse tier immediately"
    );

    // After the refresh, a re-request converges on the enriched viewport — the
    // range loop actually converges, not just "some refresh fired". Asserted
    // however the tiers raced.
    assert!(
        same_tokens(&converged.tokens, &truth),
        "after workspace/semanticTokens/refresh, the re-requested range must be \
         the enriched tier — {}",
        first_divergence(&converged.tokens, &truth)
    );
    if converged.refreshes == 0 {
        // The race was won this run — the cold viewport was already enriched, so
        // there was nothing to converge. The tiering itself is covered
        // deterministically in tcl-lsp-db.
        eprintln!(
            "large_file_range_semantic_tokens_converges_via_refresh: \
             fast path won the race, refresh not exercised this run"
        );
    }
    eprintln!(
        "large_file_range_semantic_tokens_converges_via_refresh: converged in {} rounds \
         ({} refreshes; {} rounds settled with a cancelled/equal enriched read)",
        converged.rounds, converged.refreshes, converged.no_refresh_decisions,
    );
}

/// #844 Gap 4 (negative): the convergence continuation must fire **no**
/// `workspace/semanticTokens/refresh` when the coarse and enriched viewports
/// already agree. Every other Gap-4 test appends a provably-retagged construct so
/// `enriched != served` is guaranteed whenever the comparison runs — so none of
/// them exercises the `enriched == served` branch. A regression that made that
/// comparison spuriously report a difference (an off-by-one viewport clip,
/// non-deterministic token order) would fire a refresh on *every* cold viewport
/// request; the refresh carries no URI, so a client re-pulls tokens for every open
/// document — a visible flicker of already-correct tokens.
#[test]
fn range_semantic_tokens_no_spurious_refresh_when_converged() {
    let mut lsp = Lsp::tcl();
    // Plain `generate_big_tcl` (no regex / object-dispatch construct): the coarse
    // (segmenter + registry) and enriched (CU + analysis) tiers are identical —
    // that is exactly why the convergence tests above have to *append* a regex
    // proc to force a difference. The file is still large/cold enough that the
    // CU/analysis reads overrun `SEMANTIC_TOKENS_FAST_PATH_BUDGET`, so `pending`
    // is `Some` and the continuation actually runs the coarse-vs-enriched compare.
    let big = generate_big_tcl(600);
    let line_count = u32::try_from(big.lines().count()).unwrap();
    let start = (line_count.saturating_sub(6), 0);
    let end = (line_count, 0);

    let uri = unique_uri("tcl");
    lsp.open_document_lang(&uri, &big, "tcl", 1);
    let req_since = lsp.server_request_cursor();
    let log_since = lsp.notification_cursor();
    let first = decode_semantic_tokens(&lsp.semantic_tokens_range(&uri, start, end));
    assert!(
        !first.is_empty(),
        "the cold range must still serve the coarse tier immediately"
    );

    // Deterministic readiness via message passing — not a fixed sleep.  Every
    // range request logs
    // `[timing] semantic_tokens.range_convergence.settled (uri=…, refresh=…, outcome=…)`
    // once its convergence decision is final, so waiting for exactly that line
    // makes "settled without a refresh" distinguishable from "the decision has
    // not been made yet".  The timeout is only a backstop against a total hang,
    // never the synchronisation.
    //
    // `outcome` says which shape the run took.  The one under test is
    // `compared`: the cold 600-proc file overran the 40ms fast-path budget, the
    // coarse tier was served, and the detached continuation (#844 Gap 4) diffed
    // it against the recomputed enriched viewport.  A loaded machine can instead
    // reach `no-analysis` (the document is not yet in the salsa db when the
    // request lands) or `cancelled`, and a very quick one `served-enriched` —
    // none of which run the compare.  `coalesced` (#1147) is a fourth: a
    // continuation for this document was already in flight, so this request rode
    // along with it.  The refresh assertion below is asserted in
    // *every* case, because "no spurious refresh" must hold on all of them; the
    // outcome is reported so a run that did not exercise the compare is visible
    // rather than silently vacuous. This wait observes the detached deep-tier
    // comparison; first-response latency is asserted separately, so it needs
    // enough headroom for a debug server on a loaded CI runner.
    let settled = lsp.await_log(
        &["semantic_tokens.range_convergence.settled", uri.as_str()],
        Duration::from_mins(1),
        log_since,
    );
    eprintln!("range_semantic_tokens_no_spurious_refresh_when_converged: {settled}");
    // The decision is recorded in the marker itself: `refresh=false` means
    // `request_refresh_coalesced` was not called, so no debounced
    // `workspace/semanticTokens/refresh` can ever fire for this request.  This is
    // the primary, race-free guard against the spurious-refresh regression.
    assert!(
        settled.contains("refresh=false"),
        "the converged viewport must settle without asking for a refresh: {settled:?}"
    );
    // Belt-and-suspenders: confirm no refresh request actually reached the client.
    let refreshes = lsp
        .server_requests()
        .into_iter()
        .skip(req_since)
        .filter(|r| {
            r.get("method").and_then(|m| m.as_str()) == Some("workspace/semanticTokens/refresh")
        })
        .count();
    assert_eq!(
        refreshes, 0,
        "no workspace/semanticTokens/refresh must fire when the coarse and enriched \
         viewports are identical — a spurious refresh flickers every open document"
    );
}

// -- generators / small utils --------------------------------------------

/// Byte offset -> (line, character) for an ASCII string.
fn offset_to_line_char(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut last_nl = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if i == offset {
            break;
        }
        if b == b'\n' {
            line += 1;
            last_nl = i + 1;
        }
    }
    (line, offset - last_nl)
}

/// Generate a large but realistic Tcl file: `n` small procs with bodies that
/// exercise several token kinds (keywords, vars, strings, numbers, comments,
/// namespaces, expr). ~8-9 lines per proc.
fn generate_big_tcl(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(n * 200);
    s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
    for i in 0..n {
        let _ = write!(
            s,
            "# proc number {i}\n\
             proc ::bench::step{i} {{a b}} {{\n\
             \x20   set v{i} [expr {{$a + $b}}]\n\
             \x20   set msg \"step {i} = $v{i}\"\n\
             \x20   if {{$v{i} > 10}} {{\n\
             \x20       set v{i} [expr {{$v{i} + 1}}]\n\
             \x20   }}\n\
             \x20   return $v{i}\n\
             }}\n\n"
        );
    }
    s
}
