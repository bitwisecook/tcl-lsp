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

//! Error-recovery correctness contract — end-to-end against the packaged server.
//!
//! This suite is deliberately **implementation-agnostic**: it asserts the
//! *observable* recovery behaviour over the LSP protocol, never a server
//! internal or an implementation-specific diagnostic code. It is the contract
//! the recovery engine must satisfy in *any* implementation, so a rewrite of
//! the engine can be held to it unchanged.
//!
//! The stable contracts (each a property any correct recovery must have):
//!
//!   C1  an unterminated `[` / `"` / `{` is flagged with an error diagnostic;
//!   C2  recovery is non-fatal and bounded — the rest of the document is still
//!       analysed (a proc after the break appears in document symbols) instead of
//!       being swallowed to EOF;
//!   C3  the recovered token stream is well-formed (5-int groups, in-document
//!       positions) and re-lexes commands after the break;
//!   C4  the published diagnostic set never contains exact duplicates;
//!   C5  edits that introduce a break raise the error and edits that fix it clear
//!       it, version by version;
//!   C6  well-formed code produces no recovery error.
//!
//! Brittle, implementation-specific facts (the exact `E200` vs `E201` code, the
//! precise split offset) are intentionally NOT asserted here — they belong to the
//! language-internal unit/differential/fuzz suites.

use crate::common::helpers::*;
use crate::common::{Lsp, scaled_timeout, unique_uri};

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

/// Parser/recovery diagnostic family (unclosed/extra-delimiter). A conforming
/// implementation may pick any code in this family or word the message its own
/// way; the contract is only that *some* error in this space is reported.
const RECOVERY_CODES: &[&str] = &["E200", "E201", "E202", "E203", "E204", "E205", "E206"];
const RECOVERY_WORDS: &[&str] = &[
    "missing",
    "close",
    "unterminated",
    "unclosed",
    "unbalanced",
    "extra characters",
];

/// A diagnostic's `code`, normalised to a `String` (stringifying non-strings).
fn code_str(d: &Value) -> String {
    match d.get("code") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "None".to_owned(),
    }
}

/// The set of `code` strings carried by `diags`.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags.iter().map(code_str).collect()
}

/// A diagnostic's `message` text (empty if absent).
fn message(d: &Value) -> String {
    d.get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// Whether `d` is a recovery error: a code in the family, or an error-severity
/// diagnostic whose message uses recovery vocabulary.
fn is_recovery_error(d: &Value) -> bool {
    let code = code_str(d);
    if RECOVERY_CODES.contains(&code.as_str()) {
        return true;
    }
    let msg = message(d).to_lowercase();
    let sev = d.get("severity").and_then(Value::as_i64);
    sev == Some(1) && RECOVERY_WORDS.iter().any(|w| msg.contains(w))
}

/// Whether any diagnostic is a recovery error.
fn has_recovery_error(diags: &[Value]) -> bool {
    diags.iter().any(is_recovery_error)
}

/// The identity tuple of a diagnostic (code, start/end line/char, message),
/// rendered to a stable string, used to detect exact duplicates. (Serialised to
/// a `String` because `serde_json::Value` is not `Ord`/`Hash`.)
fn diag_identity(d: &Value) -> String {
    let rng = d.get("range").cloned().unwrap_or(Value::Null);
    let start = rng.get("start").cloned().unwrap_or(Value::Null);
    let end = rng.get("end").cloned().unwrap_or(Value::Null);
    format!(
        "{}|{}|{}|{}|{}|{}",
        code_str(d),
        start.get("line").cloned().unwrap_or(Value::Null),
        start.get("character").cloned().unwrap_or(Value::Null),
        end.get("line").cloned().unwrap_or(Value::Null),
        end.get("character").cloned().unwrap_or(Value::Null),
        d.get("message").cloned().unwrap_or(Value::Null),
    )
}

/// The `tokenTypes` legend from the server's `initialize` result.
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A semantic token with its resolved type name: decode the delta encoding,
/// then map each token's numeric type through the legend.
struct TypedToken {
    line: i64,
    length: i64,
    ttype: String,
}

fn typed(lsp: &mut Lsp, uri: &str) -> Vec<TypedToken> {
    let leg = legend(lsp);
    let raw = lsp.semantic_tokens(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|tok| TypedToken {
            line: tok.line,
            length: tok.length,
            ttype: leg
                .get(usize::try_from(tok.ttype).unwrap())
                .cloned()
                .unwrap_or_default(),
        })
        .collect()
}

/// The symbol names (depth-first) of a document-symbol result.
fn symbol_name_list(syms: &Value) -> Vec<String> {
    flatten_symbols(syms)
        .iter()
        .map(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

// --------------------------------------------------------------------------- //
// C1 — an unterminated delimiter is flagged
// --------------------------------------------------------------------------- //

#[test]
fn c1_unterminated_bracket() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar\nputs hi\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated [ should be flagged; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c1_unterminated_brace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n  set y [foo\n}\nset\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated delimiter expected; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c1_well_formed_is_not_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar]\nputs hi\n");
    assert!(
        !has_recovery_error(&diags),
        "well-formed flagged a recovery error: {:?}",
        codes(&diags)
    );
}

// --------------------------------------------------------------------------- //
// C2 — recovery is non-fatal: the tail is still analysed
// --------------------------------------------------------------------------- //

#[test]
fn c2_proc_after_unterminated_bracket_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo\nproc recovered_after_bracket {} {}\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_bracket"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_proc_after_unterminated_brace_body_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "namespace eval n {\nproc recovered_in_ns {} {}\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_in_ns"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_proc_after_multiple_breaks_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set a [foo\nset b 2\nset c [bar\nproc recovered_after_two {} {}\n";
    lsp.open_ready(&uri, src);
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_two"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_bare_set_after_break_still_arity_errors() {
    // A bare `set` after a single break must be analysed as a command (and so
    // raise its arity error) — proof the tail is parsed as code, not text.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar\nset\n");
    assert!(
        codes(&diags).contains("E002"),
        "tail `set` should arity-error; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c2_if_with_unterminated_expr_brace_flags_its_own_malformed_clause() {
    // Unlike an unterminated *bracket* substitution (`[foo bar`, whose
    // content is still live Tcl-command syntax the parser keeps
    // recognising even without a closing `]` — see
    // `c2_bare_set_after_break_still_arity_errors`), an unterminated
    // *brace* word has no such inherent structure: the lexer can't know in
    // advance it will be handed to `if` as a script argument, so with no
    // closing `}` the whole remainder of the file — condition *and*
    // `set\n` — is swallowed as one opaque, unparsed span (confirmed via
    // `tcl explore --show structuralIndex`: 1 unterminated brace, 0
    // command boundaries beyond EOF, the entire tail folded into a single
    // "inert" span). So `set` is genuinely never re-tokenised as its own
    // command here — this does *not* exercise the same tail-recovery path
    // the bracket sibling test does. What *is* still true, and is what
    // this test actually verifies: `if`'s own single swallowed argument
    // (condition only, no body) is itself an arity/shape defect — E002 (a
    // plain arity floor) or the more precise E004 (`if`'s dedicated
    // clause-shape check, which subsumes E002 for `if` — see
    // `tcl-compiler`'s `no_duplicate_e002_alongside_e004`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {$x > 5\nset\n");
    assert!(
        codes(&diags).contains("E002") || codes(&diags).contains("E004"),
        "if's own malformed (condition-only) clause should still arity/shape-error; got {:?}",
        codes(&diags)
    );
}

// --------------------------------------------------------------------------- //
// C3 — recovered token stream is well-formed
// --------------------------------------------------------------------------- //

#[test]
fn c3_command_after_break_is_tokenised() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo bar\nputs hi\n");
    let toks = typed(&mut lsp, &uri);
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "expected `puts` recovered as a function on line 1; got {:?}",
        toks.iter().map(|t| (t.line, &t.ttype)).collect::<Vec<_>>()
    );
}

#[test]
fn c3_nested_break_recovers_following_lines() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc p {} {\n  set x [foo\n}\nputs done\n");
    let toks = typed(&mut lsp, &uri);
    assert!(
        toks.iter().any(|t| t.line == 3 && t.ttype == "function"),
        "{:?}",
        toks.iter().map(|t| (t.line, &t.ttype)).collect::<Vec<_>>()
    );
}

#[test]
fn c3_tokens_well_formed_for_pathological_input() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n  set x [foo \"bar\n  if {1} {\n    puts [baz\n}\nputs end\n";
    lsp.open_ready(&uri, src);
    let raw = lsp.semantic_tokens(&uri);
    let data = raw.get("data").and_then(Value::as_array);
    assert!(
        data.is_some_and(|d| d.len() % 5 == 0),
        "token data must be 5-int groups"
    );
    let n_lines = i64::try_from(src.matches('\n').count() + 1).unwrap();
    for t in typed(&mut lsp, &uri) {
        assert!(
            (0..n_lines).contains(&t.line),
            "token line out of range: line={}",
            t.line
        );
        assert!(t.length >= 0);
    }
}

#[test]
fn c3_deeply_nested_unterminated_does_not_hang() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [a [b [c [d [e\nputs tail\n");
    // returned promptly, no hang/crash — `diags` is always a Vec here.
    let _: &Vec<Value> = &diags;
}

#[test]
fn c3_crlf_document_recovers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo\r\nproc recovered_crlf {} {}\r\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_crlf"),
        "CRLF tail not recovered; symbols={names:?}"
    );
}

// --------------------------------------------------------------------------- //
// C4 — no duplicate diagnostics
// --------------------------------------------------------------------------- //

#[test]
fn c4_no_exact_duplicate_published() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x \"\nif {1} {\n  puts [foo\n}\nset\n");
    let mut seen: BTreeSet<_> = BTreeSet::new();
    for d in &diags {
        let ident = diag_identity(d);
        assert!(
            seen.insert(ident.clone()),
            "duplicate diagnostic published: {ident:?}"
        );
    }
}

// --------------------------------------------------------------------------- //
// Inert close-bracket veto (issue #560) — recovery must not fabricate a fix
// that leaves the command incomplete.
// --------------------------------------------------------------------------- //

// The `]` command/comment-break heuristics must veto an inert offset. For
//   set x [foo {bar
//   puts baz}
// the `puts` word sits *inside* the balanced brace word `{bar … baz}`, so
// inserting `]` after `bar` yields `set x [foo {bar]…}` which C Tcl reports as
// incomplete — an objectively wrong fix. Before #560 the bracket path lacked the
// inert-offset veto the brace path already had. Observably, the unterminated `[`
// is still flagged, the tail is still analysed as code, nothing is published
// twice, and the analysis doesn't hang. The mid-word look-alike (a literal `"`
// mid-word) still recovers normally.

#[test]
fn inert_brace_word_break_still_flags_and_recovers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set x [foo {bar\nproc recovered_after_brace_word {} {}\n",
    );
    assert!(
        has_recovery_error(&diags),
        "unterminated [ should flag; got {:?}",
        codes(&diags)
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_brace_word"),
        "tail not recovered; symbols={names:?}"
    );
}

#[test]
fn inert_brace_word_break_no_duplicate_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo {bar\nputs baz}\nset\n");
    let mut seen: BTreeSet<_> = BTreeSet::new();
    for d in &diags {
        let ident = diag_identity(d);
        assert!(
            seen.insert(ident.clone()),
            "duplicate diagnostic published: {ident:?}"
        );
    }
    // The trailing bare `set` is still parsed as a command → arity errors,
    // proving the tail is analysed as code rather than swallowed.
    assert!(
        codes(&diags).contains("E002"),
        "tail `set` should arity-error; got {:?}",
        codes(&diags)
    );
}

#[test]
fn inert_midword_delimiter_still_recovers() {
    // `foo abc"` — the `"` is mid-word, an ordinary literal, so the following
    // line is a genuine command-break that must still recover (the veto must not
    // over-fire and strand an unfixable error).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "set x [foo abc\"\nproc recovered_after_midword {} {}\n",
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_midword"),
        "tail not recovered; symbols={names:?}"
    );
}

#[test]
fn inert_brace_word_break_tokens_well_formed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo {bar\nputs baz}\nputs end\n");
    let raw = lsp.semantic_tokens(&uri);
    let data = raw.get("data").and_then(Value::as_array);
    assert!(
        data.is_some_and(|d| d.len() % 5 == 0),
        "token data must be 5-int groups"
    );
}

// --------------------------------------------------------------------------- //
// C5 — edits toggle recovery
// --------------------------------------------------------------------------- //

fn del_close_bracket() -> Value {
    json!([
        {
            "range": {"start": {"line": 0, "character": 14}, "end": {"line": 0, "character": 15}},
            "text": "",
        }
    ])
}

fn insert_close_bracket() -> Value {
    json!([
        {
            "range": {"start": {"line": 0, "character": 14}, "end": {"line": 0, "character": 14}},
            "text": "]",
        }
    ])
}

#[test]
fn c5_break_then_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar]\nset\n");
    assert!(!has_recovery_error(&diags));
    lsp.change_document(&uri, 2, del_close_bracket());
    let diags = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(
        has_recovery_error(&diags),
        "break should flag; got {:?}",
        codes(&diags)
    );
    lsp.change_document(&uri, 3, insert_close_bracket());
    let diags = lsp.await_diagnostics_version(&uri, Some(3), Duration::from_secs(30));
    assert!(
        !has_recovery_error(&diags),
        "fix should clear; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c5_rapid_toggling_stays_consistent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo bar]\nset\n");
    let mut version = 1i64;
    for _ in 0..3 {
        version += 1;
        lsp.change_document(&uri, version, del_close_bracket());
        let diags = lsp.await_diagnostics_version(&uri, Some(version), Duration::from_secs(30));
        assert!(
            has_recovery_error(&diags),
            "v{version} broken; got {:?}",
            codes(&diags)
        );
        version += 1;
        lsp.change_document(&uri, version, insert_close_bracket());
        let diags = lsp.await_diagnostics_version(&uri, Some(version), Duration::from_secs(30));
        assert!(
            !has_recovery_error(&diags),
            "v{version} clean; got {:?}",
            codes(&diags)
        );
    }
}

// --------------------------------------------------------------------------- //
// Known-command generality — a break just before a call to a command the
// *document itself* defines (a proc, a TclOO class, an `interp alias`) must
// recover exactly as well as a break before a call to a builtin. Before this
// fix, the "does the next line start with a known command?" recovery signal
// only ever consulted the static registry, so real-world files — almost all
// of which call their own procs — silently lost the rest of the document to
// analysis whenever no *builtin* call happened to follow the break.
// --------------------------------------------------------------------------- //

#[test]
fn user_defined_proc_recovers_the_tail_like_a_builtin() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc my_helper {x} {puts $x}\n\nset q {\n  aaa\nmy_helper\n",
    );
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    // The swallowed `my_helper` call (no args, but `my_helper` needs one) must
    // still be analysed as code — proof the tail isn't silently dropped just
    // because nothing *builtin* follows the break.
    assert!(
        diags.iter().any(|d| code_str(d) == "E002"),
        "tail `my_helper` call should arity-error; got {:?}",
        codes(&diags)
    );
}

#[test]
fn user_defined_class_recovers_the_tail() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Widget {\n  method draw {} {}\n}\n\nset q {\n    aaa\nproc recovered_after_class {} {}\n",
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_class"),
        "tail proc not recovered past a user-class recovery signal; symbols={names:?}"
    );
}

#[test]
fn namespace_qualified_proc_call_recovers_the_tail() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\n  aaa\nmyns::helper 1\nproc recovered_after_ns {} {}\n",
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_ns"),
        "tail proc not recovered past a namespace-qualified call; symbols={names:?}"
    );
}

#[test]
fn absolute_namespace_qualified_proc_call_recovers_the_tail() {
    // `signature_scan::qualify` always records a namespaced proc as
    // `::ns::name`; a recovery-point call written in the equally-valid
    // absolute form (`::myns::helper`, leading `::`) must be recognised
    // just as readily as the bare `myns::helper` form covered above.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\n  aaa\n::myns::helper\nproc recovered_after_absolute_ns {} {}\n",
    );
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    // The swallowed `::myns::helper` call (no args, but `helper` needs one)
    // must still be analysed as code — proof the tail isn't dropped.
    assert!(
        diags.iter().any(|d| code_str(d) == "E002"),
        "tail `::myns::helper` call should arity-error; got {:?}",
        codes(&diags)
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_absolute_ns"),
        "tail proc not recovered past an absolute namespace-qualified call; symbols={names:?}"
    );
}

// --------------------------------------------------------------------------- //
// Full name-resolution hierarchy — the known-command signal recovery consults
// extends past the registry + this-file's own procs/classes/aliases to two
// more layers: a `rename OLD NEW` target (this file), a proc defined in a
// *different* indexed workspace file, and a command a `package require`d
// library provides. Each layer gets a positive case (the swallowed call's own
// line is proven live — a `string` call chained onto the same line has no
// subcommand, which always draws its own E001 ("requires a subcommand") once
// analysed, independent of whatever recognised the target head) and a
// negative case (a name from none of these sources must still fall back
// honestly — no E001, since a line that stays swallowed opaque text is never
// analysed at all).
// --------------------------------------------------------------------------- //

#[test]
fn renamed_command_recovers_the_tail() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "rename puts my_puts\n\nset q {\n  aaa\nmy_puts hi; string\n",
    );
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| code_str(d) == "E001"),
        "the renamed-command call's line should be live code (proven by the \
         chained bare `string`'s own \
         missing-subcommand error); got {:?}",
        codes(&diags)
    );
}

#[test]
fn undefined_name_after_rename_statement_falls_back_honestly() {
    // The file *does* define a rename, but the swallowed call targets a
    // different, undefined name — no false positive from having a rename
    // statement merely present anywhere in the document.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "rename puts my_puts\n\nset q {\n  aaa\nnot_my_puts hi; string\n",
    );
    assert!(has_recovery_error(&diags), "got {:?}", codes(&diags));
    assert!(
        !diags.iter().any(|d| code_str(d) == "E001"),
        "an undefined name unrelated to the rename must not be mis-recovered; \
         got {:?}",
        codes(&diags)
    );
}

#[test]
fn sibling_workspace_file_proc_recovers_the_tail() {
    let mut lsp = Lsp::tcl();
    // Indexed first — `open_ready` blocks until this document's
    // `workspace_state.update` fires, so its proc is in the workspace index
    // before the broken document below is even opened.
    let sibling = unique_uri("tcl");
    lsp.open_ready(&sibling, "proc workspace_helper {x} {return $x}\n");

    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set q {\n  aaa\nworkspace_helper 1; string\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| code_str(d) == "E001"),
        "a call to a sibling-file proc should be live code (proven by the \
         chained bare `string`'s own \
         missing-subcommand error); got {:?}",
        codes(&diags)
    );
}

#[test]
fn absolute_sibling_workspace_file_proc_call_recovers_the_tail() {
    // Mirrors `absolute_namespace_qualified_proc_call_recovers_the_tail`
    // (the in-file case, above) one layer out: a workspace-indexed proc
    // referenced in absolute syntax (`::workspace_helper`, leading `::`)
    // must be recognised just as readily as the bare form.
    let mut lsp = Lsp::tcl();
    let sibling = unique_uri("tcl");
    lsp.open_ready(&sibling, "proc workspace_helper {x} {return $x}\n");

    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set q {\n  aaa\n::workspace_helper 1; string\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| code_str(d) == "E001"),
        "an absolute call to a sibling-file proc should be live code (proven \
         by the chained bare `string`'s own missing-subcommand error); got \
         {:?}",
        codes(&diags)
    );
}

#[test]
fn undefined_name_with_workspace_sibling_present_falls_back_honestly() {
    // A sibling file is indexed (so the workspace lookup isn't a no-op), but
    // the swallowed call targets a name no sibling defines.
    let mut lsp = Lsp::tcl();
    let sibling = unique_uri("tcl");
    lsp.open_ready(&sibling, "proc workspace_helper {x} {return $x}\n");

    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set q {\n  aaa\nnot_workspace_helper 1; string\n");
    assert!(has_recovery_error(&diags), "got {:?}", codes(&diags));
    assert!(
        !diags.iter().any(|d| code_str(d) == "E001"),
        "an undefined name must not be mis-recovered just because *some* \
         workspace sibling is indexed; got {:?}",
        codes(&diags)
    );
}

/// Per-call counter so repeat runs of the package-fixture tests don't collide
/// on the temp dir (mirrors `diagnostics.rs`'s `RBC_LIB_N`).
static PKG_LIB_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Write a `pkgIndex.tcl` + implementation file declaring `mypkg`, providing
/// `mypkg_helper`, under a fresh temp dir; returns the dir (whose *parent* is
/// the `libraryPaths` root — `PackageResolver::scan_path` descends one level,
/// mirroring the `rbc/tclIndex` fixture in `diagnostics.rs`).
fn write_mypkg_fixture() -> std::path::PathBuf {
    use std::sync::atomic::Ordering;
    let libdir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-mypkg-{}-{}",
        std::process::id(),
        PKG_LIB_N.fetch_add(1, Ordering::Relaxed)
    ));
    let pkgdir = libdir.join("mypkg");
    std::fs::create_dir_all(&pkgdir).expect("mk mypkg lib dir");
    std::fs::write(
        pkgdir.join("impl.tcl"),
        "proc mypkg_helper {x} {return $x}\n",
    )
    .expect("write impl.tcl");
    std::fs::write(
        pkgdir.join("pkgIndex.tcl"),
        "package ifneeded mypkg 1.0 [list source [file join $dir impl.tcl]]\n",
    )
    .expect("write pkgIndex.tcl");
    libdir
}

#[test]
fn package_required_library_command_recovers_the_tail() {
    let libdir = write_mypkg_fixture();
    let mut lsp = Lsp::with_config(json!({
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));
    let uri = unique_uri("tcl");
    lsp.open_document(
        &uri,
        "package require mypkg\nset q {\n  aaa\nmypkg_helper 1; string\n",
    );

    // The package database is (re)built asynchronously at startup; poll the
    // deterministic pull path until recovery reflects it or the deadline
    // (mirrors `autoload_library_command_not_unknown_issue_832`).
    let mut diags = lsp.pull_diagnostics(&uri);
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    while !diags.iter().any(|d| code_str(d) == "E001") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert!(
        has_recovery_error(&diags),
        "unterminated {{ should flag; got {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| code_str(d) == "E001"),
        "a call to a package-provided command should be live code (proven by \
         the chained bare `string`'s own \
         missing-subcommand error); got {:?}",
        codes(&diags)
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

#[test]
fn undefined_name_with_package_required_falls_back_honestly() {
    // The package is required and resolvable, but the swallowed call targets
    // a name `mypkg` does not provide.
    let libdir = write_mypkg_fixture();
    let mut lsp = Lsp::with_config(json!({
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));

    // The package database is (re)built asynchronously; a fixed sleep raced it,
    // and a not-yet-loaded DB leaves `mypkg` unresolvable — which would satisfy
    // the negative assertion below trivially, without ever exercising the "some
    // required package IS resolvable" condition this test exists to guard.
    // Prove the DB is live first: a control document calling mypkg's *real*
    // helper draws E001 (its chained bare `string` has no subcommand) only once
    // `mypkg` resolves.  Poll the deterministic pull path until it lands
    // (mirrors the positive `package_required_library_command_recovers_the_tail`).
    let control = unique_uri("tcl");
    lsp.open_document(
        &control,
        "package require mypkg\nset q {\n  aaa\nmypkg_helper 1; string\n",
    );
    let mut cdiags = lsp.pull_diagnostics(&control);
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    while !cdiags.iter().any(|d| code_str(d) == "E001") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        cdiags = lsp.pull_diagnostics(&control);
    }
    assert!(
        cdiags.iter().any(|d| code_str(d) == "E001"),
        "control: mypkg_helper must resolve once the package DB is live, proving \
         the DB loaded before the negative assertion runs; got {:?}",
        codes(&cdiags)
    );

    // Now the real case: with the DB provably live, the swallowed call targets a
    // name `mypkg` does NOT provide, so it must not be mis-recovered as live
    // code (no E001 from a chained bare `string`).
    let uri = unique_uri("tcl");
    lsp.open_document(
        &uri,
        "package require mypkg\nset q {\n  aaa\nnot_mypkg_helper 1; string\n",
    );
    let diags = lsp.pull_diagnostics(&uri);
    assert!(has_recovery_error(&diags), "got {:?}", codes(&diags));
    assert!(
        !diags.iter().any(|d| code_str(d) == "E001"),
        "an undefined name must not be mis-recovered just because *some* \
         required package is resolvable; got {:?}",
        codes(&diags)
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

// --------------------------------------------------------------------------- //
// Short-form unterminated quote / brace — a delimiter left open with content
// on the *same* line as the opener (the overwhelmingly common real-world
// typo) must be flagged exactly like a long multi-line run. Before this fix,
// both detectors required the run to already span multiple lines, so
// `set x "hello` / `set x {hello` (content then EOF, or content then a
// single line break) went completely unflagged — not even the generic
// fallback fired.
// --------------------------------------------------------------------------- //

#[test]
fn short_unterminated_quote_with_no_newline_is_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x \"hello");
    assert!(
        has_recovery_error(&diags),
        "a same-line unterminated quote must be flagged; got {:?}",
        codes(&diags)
    );
}

#[test]
fn short_unterminated_brace_with_no_newline_is_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x {hello");
    assert!(
        has_recovery_error(&diags),
        "a same-line unterminated brace must be flagged; got {:?}",
        codes(&diags)
    );
}

#[test]
fn unterminated_quote_with_content_before_one_line_break_is_flagged() {
    // Content on the opening line, then a single break before EOF — one
    // newline short of the old (removed) multi-line threshold.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x \"hello\nworld\n");
    assert!(has_recovery_error(&diags), "got {:?}", codes(&diags));
}

#[test]
fn unterminated_brace_with_content_before_one_line_break_is_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x {hello\nworld\n");
    assert!(has_recovery_error(&diags), "got {:?}", codes(&diags));
}

#[test]
fn well_formed_empty_and_short_delimiters_stay_silent() {
    // Regression guard alongside the short-form fixes above: an empty `""`
    // / `{}` and an ordinary short closed string/brace must never be
    // (mis)flagged now that the line-count gate is gone.
    for src in [
        "set x \"\"\n",
        "set x {}\n",
        "set x \"hello\"\n",
        "set x {hello}\n",
    ] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(
            !has_recovery_error(&diags),
            "well-formed {src:?} flagged a recovery error: {:?}",
            codes(&diags)
        );
    }
}

// --------------------------------------------------------------------------- //
// E200 tight highlighting — the generic fallback (fires only when neither the
// E201/E202/E203 detectors nor E103's stolen-brace detector can pin the
// precise delimiter) must still anchor its range at the actual unclosed
// delimiter, not spread across the whole (possibly multi-line) partial
// command through EOF.
// --------------------------------------------------------------------------- //

#[test]
fn generic_fallback_does_not_span_the_whole_document() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // The outer class body's `{` never closes; its content contains a stray
    // `}` from the balanced `method bar {}` parameter list, which routes
    // detection to the generic fallback rather than the precise E203 path.
    let src = "oo::class create Foo {\n  method bar {} {\n    puts hi\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_recovery_error(&diags),
        "unterminated class body should flag; got {:?}",
        codes(&diags)
    );
    let n_lines = i64::try_from(src.matches('\n').count() + 1).unwrap();
    for d in diags.iter().filter(|d| is_recovery_error(d)) {
        let end_line = d["range"]["end"]["line"].as_i64().unwrap_or(-1);
        assert!(
            end_line < n_lines - 1,
            "recovery diagnostic spans to (or past) the document's last line \
             — expected a tight anchor near the unclosed delimiter, not a \
             whole-document underline: {d:?}"
        );
    }
}
