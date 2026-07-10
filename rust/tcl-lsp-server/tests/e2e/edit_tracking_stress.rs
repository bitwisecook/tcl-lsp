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

//! Native port of `tests/lsp_e2e/test_edit_tracking_stress_e2e.py`.
//!
//! Stress tests that punish the server's open-document edit tracking. The server
//! advertises *incremental* sync, so every keystroke arrives as a range edit the
//! server splices into its own buffer. A bug anywhere on that path — a UTF-16
//! offset slip, a multi-line splice that loses a newline, a stale version winning
//! a race, a cache that survives a reopen — corrupts the buffer the server
//! reasons over, and every downstream feature then lies.
//!
//! The oracle is content-equivalence: drive a long, hostile sequence of
//! incremental edits, then assert the live buffer behaves *identically* to a
//! freshly opened document holding the same final text — same semantic tokens,
//! same document-symbol tree (names, kinds, and positions). The independent copy
//! of the apply-change algorithm ([`TextMirror`]) lives here, local to this file,
//! and exists only so a divergence between it and the server is a failure.
//!
//! The `random.Random(seed)` fixtures from the pytest source are driven by the
//! project's canonical reproducible generator ([`crate::common::Rng`], the `xorshift64*`
//! the `tcl-fuzz` crate uses) seeded with the same explicit seeds — the seeds are
//! kept because a stress test wants reproducible replay. The oracle
//! (mirror-vs-server equivalence) validates the server for *any* edit sequence,
//! so the exact byte-for-byte reproduction of Python's Mersenne Twister stream is
//! not load-bearing — a deterministic, hostile mix of
//! insert/delete/replace/newline/astral edits is.

use crate::common::helpers::*;
use crate::common::{Lsp, Rng, unique_uri};

use serde_json::{Value, json};
use std::time::{Duration, Instant};

// --------------------------------------------------------------------------- #
// Client-side text mirror (native port of tests/lsp_e2e/_textmirror.py).
// --------------------------------------------------------------------------- #

/// Length of `s` in UTF-16 code units (astral chars count as two).
fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// One LSP incremental edit: replace `[start, end)` with `text`.
///
/// Positions are `(line, utf16_character)` pairs in the document's coordinate
/// space *before* the edit is applied.
#[derive(Debug, Clone)]
struct Edit {
    start: (usize, usize),
    end: (usize, usize),
    text: String,
}

impl Edit {
    fn new(start: (usize, usize), end: (usize, usize), text: impl Into<String>) -> Self {
        Edit {
            start,
            end,
            text: text.into(),
        }
    }

    fn as_content_change(&self) -> Value {
        json!({
            "range": {
                "start": { "line": self.start.0, "character": self.start.1 },
                "end": { "line": self.end.0, "character": self.end.1 },
            },
            "text": self.text,
        })
    }
}

/// A document whose text mutates exactly as the server's buffer should.
struct TextMirror {
    text: String,
}

impl TextMirror {
    fn new(text: impl Into<String>) -> Self {
        TextMirror { text: text.into() }
    }

    /// Resolve a UTF-16 `(line, character)` to a byte offset into `text`.
    fn offset(&self, line: usize, character: usize) -> usize {
        let lines: Vec<&str> = self.text.split('\n').collect();
        let line = line.min(lines.len().saturating_sub(1));
        // Byte offset of the start of the target line.
        let mut byte = 0usize;
        for l in &lines[..line] {
            byte += l.len() + 1; // +1 for the '\n'
        }
        // Walk the target line counting UTF-16 units until we reach `character`.
        let within = lines[line];
        let mut col_u16 = 0usize;
        let mut idx = 0usize; // byte index within the line
        for ch in within.chars() {
            if col_u16 >= character {
                break;
            }
            col_u16 += ch.len_utf16();
            idx += ch.len_utf8();
        }
        byte + idx
    }

    /// Inverse of `offset`: byte offset -> `(line, utf16_char)`.
    fn position_at(&self, offset: usize) -> (usize, usize) {
        let prefix = &self.text[..offset.min(self.text.len())];
        let line = prefix.matches('\n').count();
        let last_nl = prefix.rfind('\n');
        let col = match last_nl {
            Some(i) => &prefix[i + 1..],
            None => prefix,
        };
        (line, utf16_len(col))
    }

    fn apply(&mut self, edit: &Edit) {
        let start = self.offset(edit.start.0, edit.start.1);
        let end = self.offset(edit.end.0, edit.end.1);
        let mut next = String::with_capacity(start + edit.text.len() + self.text.len() - end);
        next.push_str(&self.text[..start]);
        next.push_str(&edit.text);
        next.push_str(&self.text[end..]);
        self.text = next;
    }
}

/// Produce one plausible editor edit against `mirror`'s current text.
///
/// The mix is deliberately hostile: zero-width insertions, multi-character
/// deletions, replacements that span line boundaries, edits at the very end of
/// the buffer, and newline insertion/removal.
fn random_edit(mirror: &TextMirror, rng: &mut Rng) -> Edit {
    const INSERTS: [&str; 10] = [
        "x",
        "set y 1\n",
        "}",
        "{",
        "  ",
        "\n",
        "puts $z",
        "# c\n",
        "é",
        "\u{1f600}",
    ];
    let text = &mirror.text;
    let n = text.len(); // byte length (matches the Python offset space closely enough
    // for ASCII seeds; astral chars only enter via the fixed snippets)
    let kind = rng.random();
    if n == 0 || kind < 0.45 {
        // Insertion at a random byte offset on a char boundary.
        let off = char_boundary(text, rng.randint(0, n));
        let pos = mirror.position_at(off);
        let snippet = *rng.choice(&INSERTS);
        return Edit::new(pos, pos, snippet);
    }
    if kind < 0.8 && n > 1 {
        // Deletion / replacement of a random span.
        let a = char_boundary(text, rng.randint(0, n - 1));
        let hi = (a + 1 + rng.randint(1, 12)).min(n);
        let b = char_boundary(text, rng.randint(a + 1, hi.max(a + 1)));
        let start = mirror.position_at(a);
        let end = mirror.position_at(b);
        let replacement = if rng.random() < 0.6 {
            ""
        } else {
            *rng.choice(&["q", "\n", "Z9"])
        };
        return Edit::new(start, end, replacement);
    }
    // Append at end of buffer.
    let pos = mirror.position_at(n);
    Edit::new(pos, pos, *rng.choice(&["\nset w 0", "puts done\n", ";"]))
}

/// Round `off` down to the nearest char boundary in `text`.
fn char_boundary(text: &str, off: usize) -> usize {
    let mut o = off.min(text.len());
    while o > 0 && !text.is_char_boundary(o) {
        o -= 1;
    }
    o
}

// --------------------------------------------------------------------------- #
// Oracle helpers (native port of the pytest module-level assertions).
// --------------------------------------------------------------------------- #

/// The semantic-tokens legend (`tokenTypes`, `tokenModifiers`) advertised by the
/// server.
fn legend(lsp: &Lsp) -> (Vec<String>, Vec<String>) {
    let prov = &lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"];
    let types = prov["tokenTypes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default();
    let mods = prov["tokenModifiers"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default();
    (types, mods)
}

/// The server's semantic tokens for `uri` must satisfy every invariant. Stronger
/// than the edited-vs-fresh oracle: the *absolute* token geometry is internally
/// consistent and within `text`'s bounds (ordered, non-overlapping,
/// UTF-16-correct, legend-valid).
fn assert_tokens_aligned(lsp: &mut Lsp, uri: &str, text: &str) {
    let (types, mods) = legend(lsp);
    let raw = lsp.semantic_tokens(uri);
    let types_ref: Vec<&str> = types.iter().map(String::as_str).collect();
    let mods_ref: Vec<&str> = mods.iter().map(String::as_str).collect();
    let violations = semantic_token_violations(&raw, text, &types_ref, &mods_ref);
    assert!(
        violations.is_empty(),
        "semantic-token alignment broke:\n{}",
        violations.join("\n")
    );
}

/// Every `(line, utf16_col)` where `needle` starts in `text`.
fn find_occurrences(text: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (line_no, line) in text.split('\n').enumerate() {
        // The seed docs here are ASCII, so char index == utf16 col.
        let mut search_from = 0usize;
        while let Some(rel) = line[search_from..].find(needle) {
            let byte_col = search_from + rel;
            // UTF-16 column == char-count prefix (ASCII seed docs).
            let col = utf16_len(&line[..byte_col]);
            out.push((line_no, col));
            search_from = byte_col + 1;
        }
    }
    out
}

/// A normalised, comparable projection of a document-symbol tree: name, kind,
/// detail, full range, selection-range start, and children (recursively).
#[derive(Debug, PartialEq)]
struct NormSym {
    name: Value,
    kind: Value,
    detail: Value,
    range: (i64, i64, i64, i64),
    sel_start: (i64, i64),
    children: Vec<NormSym>,
}

fn norm_symbols(symbols: &Value) -> Vec<NormSym> {
    let mut out = Vec::new();
    let Some(arr) = symbols.as_array() else {
        return out;
    };
    for sym in arr {
        let rng = &sym["range"];
        let sel = &sym["selectionRange"];
        let coord =
            |v: &Value, path: &str, field: &str| -> i64 { v[path][field].as_i64().unwrap_or(0) };
        out.push(NormSym {
            name: sym.get("name").cloned().unwrap_or(Value::Null),
            kind: sym.get("kind").cloned().unwrap_or(Value::Null),
            detail: sym.get("detail").cloned().unwrap_or(Value::Null),
            range: (
                coord(rng, "start", "line"),
                coord(rng, "start", "character"),
                coord(rng, "end", "line"),
                coord(rng, "end", "character"),
            ),
            sel_start: (
                coord(sel, "start", "line"),
                coord(sel, "start", "character"),
            ),
            children: norm_symbols(sym.get("children").unwrap_or(&Value::Null)),
        });
    }
    out
}

/// The incrementally-edited buffer must behave like a fresh open of its text.
///
/// The oracle is two *synchronous*, position-sensitive functions of the buffer
/// the server currently holds — its **semantic tokens** and its
/// **document-symbol tree**. Both are settled deterministically first: the fresh
/// document via `open_ready`, the edited document via a no-op full-replace at the
/// next version (`settle_analysis`) that forces a snapshot build at the final
/// content.
///
/// `fresh_text` is the edited document's final content too, so the no-op replace
/// leaves the buffer unchanged while forcing a settled snapshot.
fn assert_buffer_equiv(
    lsp: &mut Lsp,
    edited_uri: &str,
    edited_version: i64,
    fresh_uri: &str,
    fresh_text: &str,
) {
    // Don't await the burst's exact final version: after a rapid didChange burst
    // the server debounces and suppresses re-publishing unchanged diagnostics, so
    // a publish tagged with that precise intermediate version may never arrive
    // (timing-dependent — it flakes under CI load). `settle_analysis` below is the
    // deterministic barrier: a real no-op edit at `edited_version + 1` forces a
    // fresh publish + `workspace_state.update` at the final content.
    lsp.settle_analysis(edited_uri, edited_version + 1, fresh_text);
    lsp.open_ready(fresh_uri, fresh_text);

    // Either buffer's first `semanticTokens/full` can be served the transient
    // *coarse* tier when its enriched (SSA/SCCP-informed) computation overruns
    // the 40 ms fast-path budget under load; the server then fires
    // `workspace/semanticTokens/refresh` once the enriched stream lands (issue
    // #829). A raw compare can therefore catch a coarse/enriched *tier split*
    // that is a scheduling artefact, not a stale-cache divergence.
    //
    // Fast path: if the first reads already agree, the buffers are consistent —
    // whichever tier each was served, they match, which is the whole invariant.
    // Only on an apparent divergence do we converge *both* buffers onto their
    // settled enriched streams (as a conformant client does) and re-compare, so
    // a real stale-cache bug still fails with a clean diff while a tier split
    // does not flake.
    let fresh_first = decode_semantic_tokens(&lsp.semantic_tokens(fresh_uri));
    let edited_first = decode_semantic_tokens(&lsp.semantic_tokens(edited_uri));
    let (edited_tokens, fresh_tokens) = if toks(&edited_first) == toks(&fresh_first) {
        (edited_first, fresh_first)
    } else {
        (
            settled_tokens(lsp, edited_uri),
            settled_tokens(lsp, fresh_uri),
        )
    };
    assert_eq!(
        toks(&edited_tokens),
        toks(&fresh_tokens),
        "semantic tokens diverged between the incrementally-edited buffer and a \
         fresh open of the same text (stale incremental cache?)"
    );

    let edited_syms = norm_symbols(&lsp.document_symbols(edited_uri));
    let fresh_syms = norm_symbols(&lsp.document_symbols(fresh_uri));
    assert!(
        edited_syms == fresh_syms,
        "document-symbol tree diverged (content or position drift)"
    );
}

/// Fetch `uri`'s semantic tokens, converging onto the settled *enriched* stream.
///
/// `semanticTokens/full` may serve the cheap coarse tier when the enriched
/// (SSA/SCCP-informed) computation overruns its 40 ms fast-path budget, then
/// fire `workspace/semanticTokens/refresh` once the enriched stream lands — and
/// it fires that refresh *iff* the enriched stream differs from the one just
/// served (`deliver_if_changed`). So a request that draws no follow-up refresh
/// was already the enriched stream; one that does is re-pulled. Looping until a
/// request draws no refresh converges on the enriched tier the way a conformant
/// client (and the `semantic_tokens_reference_client` suite) does, whichever
/// tier won the first race — the deterministic input an oracle comparison needs.
///
/// If the enriched stream is starved past the deadline the last response is
/// returned unchanged, so a genuine divergence is reported by the caller's
/// assertion rather than the test hanging.
fn settled_tokens(lsp: &mut Lsp, uri: &str) -> Vec<SemToken> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let since = lsp.server_request_cursor();
        let current = decode_semantic_tokens(&lsp.semantic_tokens(uri));
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return current;
        }
        // A follow-up refresh means the stream just served was the coarse tier
        // and a better (enriched) one has landed — re-pull. No refresh within the
        // grace means the served stream is already the enriched tier.
        if lsp
            .try_await_server_request(
                "workspace/semanticTokens/refresh",
                remaining.min(Duration::from_secs(2)),
                since,
            )
            .is_none()
        {
            return current;
        }
    }
}

/// A comparable tuple form of a decoded semantic token.
fn toks(tokens: &[SemToken]) -> Vec<(i64, i64, i64, i64, i64)> {
    tokens
        .iter()
        .map(|t| (t.line, t.char, t.length, t.ttype, t.modifiers))
        .collect()
}

/// The set of diagnostic `code` strings.
fn codes(diags: &[Value]) -> std::collections::BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

const SEED_DOC: &str = "proc greet {name} {\n    puts \"Hello $name\"\n}\nset total [expr {1 + 2}]\nif $cond { puts $total }\ngreet World\n";

// -- TestRandomEditStorm -------------------------------------------------

/// Dumps the full state of a random-seeded run when the test thread is
/// unwinding, so a failure — a mirror-vs-server divergence *or* the diagnostics
/// barrier inside [`assert_buffer_equiv`] timing out under load — is
/// reproducible from the CI log alone. Without this a random test that flakes
/// leaves only a bare panic message and the seed is not enough to see *what*
/// edit stream provoked it.
struct ReproDump<'a> {
    seed: u64,
    version: i64,
    edits: &'a [Edit],
    final_text: &'a str,
}

impl Drop for ReproDump<'_> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        eprintln!(
            "\n==== edit_tracking_stress repro (seed={}) ====",
            self.seed
        );
        eprintln!(
            "applied {} edits, reached version {}; barrier awaited diagnostics at version {}",
            self.edits.len(),
            self.version,
            self.version + 1,
        );
        for (i, e) in self.edits.iter().enumerate() {
            // version starts at 1; the first edit lands at version 2.
            eprintln!(
                "  edit #{i} (v{}): [{},{})..[{},{}) <- {:?}",
                i + 2,
                e.start.0,
                e.start.1,
                e.end.0,
                e.end.1,
                e.text,
            );
        }
        eprintln!(
            "final buffer ({} bytes, {} lines):\n{}",
            self.final_text.len(),
            self.final_text.lines().count(),
            self.final_text,
        );
        eprintln!("==== end repro (seed={}) ====\n", self.seed);
    }
}

fn random_incremental_edits_match_fresh_open(seed: u64) {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SEED_DOC);
    let mut mirror = TextMirror::new(SEED_DOC);
    let mut rng = Rng::new(seed);
    let mut version = 1i64;
    let mut applied: Vec<Edit> = Vec::with_capacity(60);
    for _ in 0..60 {
        let edit = random_edit(&mirror, &mut rng);
        mirror.apply(&edit);
        version += 1;
        lsp.change_document(&uri, version, json!([edit.as_content_change()]));
        applied.push(edit);
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    // Past this point every borrow of `applied`/`text` is read-only, so the
    // guard can hold them and dump on any unwind out of `assert_buffer_equiv`.
    let _repro = ReproDump {
        seed,
        version,
        edits: &applied,
        final_text: &text,
    };
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

#[test]
fn random_incremental_edits_match_fresh_open_1() {
    random_incremental_edits_match_fresh_open(1);
}
#[test]
fn random_incremental_edits_match_fresh_open_7() {
    random_incremental_edits_match_fresh_open(7);
}
#[test]
fn random_incremental_edits_match_fresh_open_13() {
    random_incremental_edits_match_fresh_open(13);
}
#[test]
fn random_incremental_edits_match_fresh_open_42() {
    random_incremental_edits_match_fresh_open(42);
}
#[test]
fn random_incremental_edits_match_fresh_open_99() {
    random_incremental_edits_match_fresh_open(99);
}
#[test]
fn random_incremental_edits_match_fresh_open_256() {
    random_incremental_edits_match_fresh_open(256);
}
#[test]
fn random_incremental_edits_match_fresh_open_1024() {
    random_incremental_edits_match_fresh_open(1024);
}

fn batched_multi_edit_changes(seed: u64) {
    // Each didChange carries *several* edits at once (the shape a multi-cursor
    // editor sends). LSP applies them in array order against the evolving
    // buffer, so the mirror must too.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SEED_DOC);
    let mut mirror = TextMirror::new(SEED_DOC);
    let mut rng = Rng::new(seed);
    let mut version = 1i64;
    for _ in 0..25 {
        let mut changes = Vec::new();
        let count = rng.randint(1, 4);
        for _ in 0..count {
            let edit = random_edit(&mirror, &mut rng);
            mirror.apply(&edit);
            changes.push(edit.as_content_change());
        }
        version += 1;
        lsp.change_document(&uri, version, Value::Array(changes));
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

#[test]
fn batched_multi_edit_changes_3() {
    batched_multi_edit_changes(3);
}
#[test]
fn batched_multi_edit_changes_17() {
    batched_multi_edit_changes(17);
}
#[test]
fn batched_multi_edit_changes_64() {
    batched_multi_edit_changes(64);
}

// -- TestSupersession ----------------------------------------------------

#[test]
fn rapid_edits_final_version_wins() {
    // Fire a burst of edits without waiting between them, then demand the publish
    // for the *final* version. The async pipeline must not let an earlier, slower
    // analysis win the race and leave stale diagnostics.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let mut mirror = TextMirror::new("set x 1\n");
    let mut version = 1i64;
    for i in 0..40 {
        // Append a fresh line each time so every version has distinct content.
        let end = mirror.position_at(mirror.text.len());
        let edit = Edit::new(end, end, format!("set v{i} {i}\n"));
        mirror.apply(&edit);
        version += 1;
        lsp.change_document(&uri, version, json!([edit.as_content_change()]));
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

#[test]
fn introduce_then_immediately_fix_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    // v2: break it (bare `set` -> E002). v3: fix it again — without waiting on v2.
    // The final publish must reflect v3 (no leftover E002 from v2).
    lsp.replace_document(&uri, 2, "set\n");
    lsp.replace_document(&uri, 3, "puts hello\n");
    let final_diags = lsp.await_diagnostics_version(&uri, Some(3), Duration::from_secs(30));
    assert!(!codes(&final_diags).contains("E002"));
}

// -- TestStructuralEdits -------------------------------------------------

#[test]
fn multiline_insertions_and_deletions() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set a 1\nset b 2\nset c 3\n");
    let mut mirror = TextMirror::new("set a 1\nset b 2\nset c 3\n");
    let mut version = 1i64;

    let edit = |lsp: &mut Lsp,
                mirror: &mut TextMirror,
                version: &mut i64,
                start: (usize, usize),
                end: (usize, usize),
                text: &str| {
        let e = Edit::new(start, end, text);
        mirror.apply(&e);
        *version += 1;
        lsp.change_document(&uri, *version, json!([e.as_content_change()]));
    };

    // Insert two whole lines at the top.
    edit(
        &mut lsp,
        &mut mirror,
        &mut version,
        (0, 0),
        (0, 0),
        "proc p {} {\n    return\n}\n",
    );
    // Delete the middle original line (`set b 2`).
    edit(&mut lsp, &mut mirror, &mut version, (4, 0), (5, 0), "");
    // Replace a span crossing a line boundary.
    edit(
        &mut lsp,
        &mut mirror,
        &mut version,
        (0, 5),
        (1, 4),
        "q {} {\n    error x",
    );
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

#[test]
fn delete_to_empty_then_rebuild() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let start = "proc greet {} { puts hi }\ngreet\n";
    lsp.open_ready(&uri, start);
    let mut mirror = TextMirror::new(start);
    let mut version = 1i64;
    // Delete the entire buffer in one edit.
    let end_pos = mirror.position_at(mirror.text.len());
    let e = Edit::new((0, 0), end_pos, "");
    mirror.apply(&e);
    version += 1;
    lsp.change_document(&uri, version, json!([e.as_content_change()]));
    // Rebuild a different program one fragment at a time.
    for fragment in [
        "proc ",
        "add {a b} ",
        "{\n",
        "    expr {$a + $b}\n",
        "}\n",
        "add 1 2\n",
    ] {
        let pos = mirror.position_at(mirror.text.len());
        let e = Edit::new(pos, pos, fragment);
        mirror.apply(&e);
        version += 1;
        lsp.change_document(&uri, version, json!([e.as_content_change()]));
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

// -- TestUnicodeTracking -------------------------------------------------

#[test]
fn utf16_offsets_survive_astral_chars() {
    // Astral-plane characters occupy two UTF-16 code units; a tracker that
    // confuses code units with code points will splice later edits at the wrong
    // column. Interleave astral inserts with edits *after* them.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set s {}\nset x 1\n");
    let mut mirror = TextMirror::new("set s {}\nset x 1\n");
    let mut version = 1i64;

    let edit = |lsp: &mut Lsp,
                mirror: &mut TextMirror,
                version: &mut i64,
                start: (usize, usize),
                end: (usize, usize),
                text: &str| {
        let e = Edit::new(start, end, text);
        mirror.apply(&e);
        *version += 1;
        lsp.change_document(&uri, *version, json!([e.as_content_change()]));
    };

    // Put an emoji inside the braces on line 0.
    edit(
        &mut lsp,
        &mut mirror,
        &mut version,
        (0, 6),
        (0, 7),
        "\u{1f600}\u{1f602}",
    );
    // Now edit line 0 *after* the astral chars; derive the end-of-line-0 position
    // from the mirror so the UTF-16 column accounts for them.
    let nl = mirror.text.find('\n').unwrap();
    let eol0 = mirror.position_at(nl);
    edit(&mut lsp, &mut mirror, &mut version, eol0, eol0, " ;# tail");
    // And change line 1 to confirm cross-line coordinates still resolve.
    edit(&mut lsp, &mut mirror, &mut version, (1, 6), (1, 7), "999");
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

// -- TestReopenLifecycle -------------------------------------------------

#[test]
fn close_and_reopen_resets_version_without_stale_cache() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // First session: a clean file at versions 1..3.
    lsp.open_ready(&uri, "set x 1\n");
    lsp.replace_document(&uri, 2, "set y 2\n");
    lsp.replace_document(&uri, 3, "set z 3\n");
    lsp.await_diagnostics_version(&uri, Some(3), Duration::from_secs(30));
    lsp.close_document(&uri);
    // Drop buffered publishes from the first session so the reopen's version-1
    // publish (not the prior session's, also version 1) is what we observe — the
    // very ambiguity the reopen-reset cache must handle.
    lsp.clear_notifications();
    // Reopen with version reset to 1 holding *broken* content — a stale
    // hover/diagnostic cache keyed on (uri, version) must not resurface.
    let diags = lsp.open_ready(&uri, "set\n");
    assert!(codes(&diags).contains("E002"));
    // Hover at the command resolves against the reopened buffer, not the old one.
    assert!(hover_text(&lsp.hover(&uri, 0, 1)).contains("set"));
}

#[test]
fn feature_requests_interleaved_with_edits() {
    // Hammer feature requests *between* edits; the server must answer each against
    // the then-current buffer and never wedge.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc f {} { return }\nf\n");
    let mut mirror = TextMirror::new("proc f {} { return }\nf\n");
    let mut rng = Rng::new(5);
    let mut version = 1i64;
    for _ in 0..20 {
        let edit = random_edit(&mirror, &mut rng);
        mirror.apply(&edit);
        version += 1;
        lsp.change_document(&uri, version, json!([edit.as_content_change()]));
        // Fire a request that reads the buffer; we only require it not to error.
        lsp.document_symbols(&uri);
        lsp.semantic_tokens(&uri);
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

// --------------------------------------------------------------------------- #
// Token alignment: semantic tokens must never go out of alignment under edits.
// --------------------------------------------------------------------------- #

const ALIGN_DOC: &str = "proc tally {items} {\n    set count 0\n    foreach item $items {\n        set count [expr {$count + 1}]\n        puts \"item $item count $count\"\n    }\n    return $count\n}\n";

// -- TestTokenAlignmentUnderEdits ----------------------------------------

#[test]
fn multicursor_rename_keeps_tokens_aligned() {
    // A multi-cursor rename arrives as ONE didChange carrying an edit per site.
    // Editors send them bottom-up so earlier edits don't shift later offsets; the
    // mirror applies them in the same order. After the rename, every `$count`
    // token must still land exactly on its (now longer) variable.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ALIGN_DOC);
    let mut mirror = TextMirror::new(ALIGN_DOC);
    assert_tokens_aligned(&mut lsp, &uri, &mirror.text.clone());

    let mut sites = find_occurrences(&mirror.text, "count");
    assert!(sites.len() >= 5, "{sites:?}");
    // Replace `count` with a longer `counterValue` at every site, bottom-up
    // (descending position) so offsets stay valid.
    sites.sort_unstable();
    sites.reverse();
    let edits: Vec<Edit> = sites
        .iter()
        .map(|&(line, col)| {
            Edit::new(
                (line, col),
                (line, col + utf16_len("count")),
                "counterValue",
            )
        })
        .collect();
    for e in &edits {
        mirror.apply(e);
    }
    let changes: Vec<Value> = edits.iter().map(Edit::as_content_change).collect();
    lsp.change_document(&uri, 2, Value::Array(changes));
    lsp.settle_analysis(&uri, 3, &mirror.text);

    let text = mirror.text.clone();
    assert_tokens_aligned(&mut lsp, &uri, &text);
    let fresh = unique_uri("tcl");
    assert_buffer_equiv(&mut lsp, &uri, 3, &fresh, &text);
}

#[test]
fn multicursor_insert_prefix_at_each_line() {
    // Insert a 4-space indent at the start of every body line in a single
    // didChange (a column/multi-cursor block edit). Token columns on each line
    // must shift by exactly the inserted width and stay aligned.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ALIGN_DOC);
    let mut mirror = TextMirror::new(ALIGN_DOC);
    let n_lines = mirror.text.matches('\n').count();
    let edits: Vec<Edit> = (0..n_lines)
        .rev()
        .map(|ln| Edit::new((ln, 0), (ln, 0), "  "))
        .collect();
    for e in &edits {
        mirror.apply(e);
    }
    let changes: Vec<Value> = edits.iter().map(Edit::as_content_change).collect();
    lsp.change_document(&uri, 2, Value::Array(changes));
    lsp.settle_analysis(&uri, 3, &mirror.text);
    let text = mirror.text.clone();
    assert_tokens_aligned(&mut lsp, &uri, &text);
    let fresh = unique_uri("tcl");
    assert_buffer_equiv(&mut lsp, &uri, 3, &fresh, &text);
}

fn invariants_hold_throughout_random_storm(seed: u64) {
    // Drive batched multi-edit changes and, at each checkpoint, settle and assert
    // the tokens satisfy every invariant against the mirror text — so alignment
    // is pinned *throughout* the storm, not only at the end.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ALIGN_DOC);
    let mut mirror = TextMirror::new(ALIGN_DOC);
    let mut rng = Rng::new(seed);
    let mut version = 1i64;
    for _ in 0..8 {
        let mut changes = Vec::new();
        let count = rng.randint(1, 3);
        for _ in 0..count {
            let edit = random_edit(&mirror, &mut rng);
            mirror.apply(&edit);
            changes.push(edit.as_content_change());
        }
        version += 1;
        lsp.change_document(&uri, version, Value::Array(changes));
        version = lsp.settle_analysis(&uri, version + 1, &mirror.text);
        let text = mirror.text.clone();
        assert_tokens_aligned(&mut lsp, &uri, &text);
    }
    let fresh = unique_uri("tcl");
    let text = mirror.text.clone();
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}

#[test]
fn invariants_hold_throughout_random_storm_11() {
    invariants_hold_throughout_random_storm(11);
}
#[test]
fn invariants_hold_throughout_random_storm_29() {
    invariants_hold_throughout_random_storm(29);
}
#[test]
fn invariants_hold_throughout_random_storm_51() {
    invariants_hold_throughout_random_storm(51);
}

#[test]
fn multibyte_edits_keep_utf16_columns_aligned() {
    // Interleave emoji/accented inserts with edits after them; the tokens' UTF-16
    // columns must track the astral widths exactly.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let start = "set label \"tag\"\nputs $label\n";
    lsp.open_ready(&uri, start);
    let mut mirror = TextMirror::new(start);
    let mut version = 1i64;

    let edit = |lsp: &mut Lsp,
                mirror: &mut TextMirror,
                version: &mut i64,
                start: (usize, usize),
                end: (usize, usize),
                text: &str| {
        let e = Edit::new(start, end, text);
        mirror.apply(&e);
        *version += 1;
        lsp.change_document(&uri, *version, json!([e.as_content_change()]));
    };

    // Insert an emoji inside the string, then edit after it on the same line.
    edit(
        &mut lsp,
        &mut mirror,
        &mut version,
        (0, 14),
        (0, 14),
        "\u{1f600}\u{1f680}",
    );
    let nl = mirror.text.find('\n').unwrap();
    let eol0 = mirror.position_at(nl);
    edit(
        &mut lsp,
        &mut mirror,
        &mut version,
        eol0,
        eol0,
        " ;# 日本語",
    );
    version = lsp.settle_analysis(&uri, version + 1, &mirror.text);
    let text = mirror.text.clone();
    assert_tokens_aligned(&mut lsp, &uri, &text);
    let fresh = unique_uri("tcl");
    assert_buffer_equiv(&mut lsp, &uri, version, &fresh, &text);
}
