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

//! Authoring a `.sslictcl` document, end to end (#1543, epic #1524).
//!
//! The claim under test is the issue's: **a TLS declaration document is a
//! first-class thing to author in an editor**. So these tests drive the real
//! binary over JSON-RPC and assert on what an editor would show — where a
//! document routes, what completion offers inside a block, what the outline
//! lists, and which coded diagnostics appear over which exact characters.
//!
//! Two facts about the dialect shape almost every assertion below. The
//! vocabulary is *registry data* (`tcl_registry::commands::sslictcl` plus the
//! `SSLICTCL_*` definition-body grammars), so the editor surfaces come from
//! the same machinery every shipped Tcl command uses. And the document is
//! **never evaluated**, which is why the loader — not the analyser — owns the
//! verdict on an unrecognised word, and why a block body offers its own
//! members and nothing else.

use crate::common::helpers::{completion_labels, decode_semantic_tokens, hover_text, symbol_names};
use crate::common::{Lsp, unique_uri};
use serde_json::{Value, json};
use std::time::Duration;

/// The shipped sample — the document `docs/design/sslictcl-vocabulary.md`
/// describes and whose own header states which notices it deliberately raises.
const SAMPLE: &str = include_str!("../../../../samples/sslictcl/example.sslictcl");

/// A short well-formed document, for the surface tests that do not need the
/// whole sample.
const DOC: &str = "sslictcl 1\n\
                   endpoint /Common/www {\n\
                   \x20   hostname www.example.test\n\
                   \x20   protocols {tls1.2 tls1.3}\n\
                   \x20   hsts {\n\
                   \x20       enabled true\n\
                   \x20       max-age 31536000\n\
                   \x20   }\n\
                   }\n\
                   policy corporate {\n\
                   \x20   check modern {\n\
                   \x20       severity error\n\
                   \x20       predicate {expr {[llength $ciphers] > 0}}\n\
                   \x20   }\n\
                   \x20   grade {\n\
                   \x20       minimum A\n\
                   \x20   }\n\
                   }\n";

/// Three *independent* errors, one per family the loader can report: a value
/// outside its domain, an unknown member of a closed block, and a reference to
/// a name the document never declares.
const THREE_ERRORS: &str = "sslictcl 1\n\
                            endpoint /Common/a {\n\
                            \x20   hostname a.example.test\n\
                            \x20   hsts {\n\
                            \x20       enabled maybe\n\
                            \x20       nonsense 1\n\
                            \x20   }\n\
                            \x20   chain missing-chain\n\
                            }\n";

/// `(code, severity, start line, start character, end line, end character)`
/// for every diagnostic in `diags`, in publication order.
fn coded(diags: &[Value]) -> Vec<(String, i64, i64, i64, i64, i64)> {
    diags
        .iter()
        .map(|d| {
            let range = &d["range"];
            (
                d["code"].as_str().unwrap_or_default().to_owned(),
                d["severity"].as_i64().unwrap_or_default(),
                range["start"]["line"].as_i64().unwrap_or_default(),
                range["start"]["character"].as_i64().unwrap_or_default(),
                range["end"]["line"].as_i64().unwrap_or_default(),
                range["end"]["character"].as_i64().unwrap_or_default(),
            )
        })
        .collect()
}

/// Just the codes, sorted — for the assertions that care which findings exist
/// rather than where they sit.
fn codes(diags: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = diags
        .iter()
        .filter_map(|d| d["code"].as_str().map(ToOwned::to_owned))
        .collect();
    out.sort();
    out
}

/// The LSP `DiagnosticSeverity` values.
const ERROR: i64 = 1;
const HINT: i64 = 4;

/// The `tokenTypes` legend advertised in `initialize`.
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("tokenTypes legend")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The `(covered text, token type)` pairs of `uri`'s settled semantic tokens.
fn painted(lsp: &mut Lsp, legend: &[String], uri: &str, source: &str) -> Vec<(String, String)> {
    let raw = lsp.semantic_tokens_settled(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .filter_map(|token| {
            let line = source
                .split('\n')
                .nth(usize::try_from(token.line).ok()?)
                .unwrap_or_default();
            let start = usize::try_from(token.char).ok()?;
            let end = start + usize::try_from(token.length).ok()?;
            let text = line.get(start..end)?.to_owned();
            Some((
                text,
                legend
                    .get(usize::try_from(token.ttype).ok()?)
                    .cloned()
                    .unwrap_or_default(),
            ))
        })
        .collect()
}

/// The zero-based line of the first line of `source` containing `needle`.
fn line_of(source: &str, needle: &str) -> u32 {
    u32::try_from(
        source
            .split('\n')
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("fixture has no line containing {needle:?}")),
    )
    .expect("line fits u32")
}

// -- routing ------------------------------------------------------------

/// All three routes reach the `sslictcl` registry, and hovering `endpoint`
/// proves which pack answered.
///
/// The extension is the registration every editor makes; the explicit
/// `languageId` is what a contributed identity sends; and the mandatory
/// `sslictcl VERSION` header is the content signature that recognises a
/// document saved under a `.tcl` name. All three must land in the same place,
/// because a user who renames a file has not changed what it is.
#[test]
fn every_route_reaches_the_sslictcl_vocabulary() {
    let mut lsp = Lsp::tcl();
    let by_extension = unique_uri("sslictcl");
    lsp.open_ready(&by_extension, DOC);
    let by_language_id = unique_uri("tcl");
    lsp.open_ready_lang(&by_language_id, DOC, "sslictcl");
    let by_content_signature = unique_uri("tcl");
    lsp.open_ready(&by_content_signature, DOC);

    let line = line_of(DOC, "endpoint /Common/www");
    for uri in [&by_extension, &by_language_id, &by_content_signature] {
        let hover = hover_text(&lsp.hover(uri, line, 2));
        assert!(
            hover.contains("endpoint"),
            "hover on `endpoint` must name it ({uri}): {hover}"
        );
        assert!(
            hover.contains("Declare a TLS endpoint"),
            "hover must be the SslicTcl pack's own, whose text no other pack \
             carries ({uri}): {hover}"
        );
    }
}

/// A document of *any other* dialect does not gain the vocabulary: the same
/// words in a plain `.tcl` script hover as nothing.
#[test]
fn the_vocabulary_does_not_leak_into_plain_tcl() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // No `sslictcl` header, so nothing routes this document to the dialect.
    let source = "endpoint /Common/www {\n    hostname www.example.test\n}\n";
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 2));
    assert!(
        !hover.contains("Declare a TLS endpoint"),
        "`endpoint` must mean nothing outside the dialect: {hover}"
    );
}

// -- completion + signature help ---------------------------------------

/// A block body offers exactly its own members — nothing else is writable
/// there, because nothing in the document is evaluated.
#[test]
fn a_block_body_offers_exactly_its_members() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    // A blank line inside the `hsts { … }` body is the cursor position.
    let source = "sslictcl 1\n\
                  endpoint /Common/www {\n\
                  \x20   hostname www.example.test\n\
                  \x20   hsts {\n\
                  \x20       \n\
                  \x20   }\n\
                  }\n";
    lsp.open_ready(&uri, source);
    let mut labels = completion_labels(&lsp.completion(&uri, 4, 8));
    labels.sort();
    assert_eq!(
        labels,
        vec!["enabled", "include-subdomains", "max-age", "preload"],
        "an `hsts` body admits its four members and nothing else",
    );
}

/// The top level offers the declaration words. It is *open* — an unrecognised
/// statement is preserved as an extension — so the offer is not exhaustive
/// there, only complete.
#[test]
fn the_top_level_offers_the_declarations() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    let source = format!("{DOC}\n");
    lsp.open_ready(&uri, &source);
    let line = u32::try_from(source.split('\n').count() - 1).expect("line fits u32");
    let labels = completion_labels(&lsp.completion(&uri, line, 0));
    for declaration in [
        "certificate",
        "chain",
        "cipher",
        "endpoint",
        "policy",
        "protocol",
        "testssl-import",
        "trust-program",
    ] {
        assert!(
            labels.iter().any(|label| label == declaration),
            "top level must offer `{declaration}`",
        );
    }
}

/// Signature help on a declaration shows the shape the vocabulary document
/// states, from the pack's own hover synopsis.
#[test]
fn signature_help_states_a_declaration_shape() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, DOC);
    let line = line_of(DOC, "endpoint /Common/www");
    let help = lsp.signature_help(&uri, line, 9);
    let label = help["signatures"][0]["label"].as_str().unwrap_or_default();
    assert!(
        label.starts_with("endpoint"),
        "signature help must describe `endpoint`: {help}"
    );
    assert!(
        label.contains("name"),
        "the synopsis names the declared entity: {help}"
    );
}

// -- semantic tokens, folding, symbols ---------------------------------

/// Declaration words paint as keywords at every nesting level, and a
/// `predicate` body does not: its script is retained verbatim and is not part
/// of the vocabulary.
#[test]
fn declaration_words_paint_as_keywords_and_a_predicate_body_does_not() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, DOC);
    let tokens = painted(&mut lsp, &legend_names, &uri, DOC);
    for word in ["endpoint", "hostname", "hsts", "enabled"] {
        assert!(
            tokens
                .iter()
                .any(|(text, kind)| text == word && kind == "keyword"),
            "`{word}` must paint as a keyword: {tokens:?}",
        );
    }
    // The `predicate` statement itself is vocabulary; what is inside its
    // braces is not, so `llength` is not painted as a declaration row.
    assert!(
        tokens
            .iter()
            .any(|(text, kind)| text == "predicate" && kind == "keyword"),
        "the `predicate` statement is a keyword: {tokens:?}",
    );
    assert!(
        !tokens
            .iter()
            .any(|(text, kind)| text == "llength" && kind == "keyword"),
        "a predicate body is not a declaration block: {tokens:?}",
    );
}

/// Every block in the sample folds — the outer declarations and the nested
/// ones alike.
#[test]
fn every_block_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, DOC);
    let folds = lsp.folding_range(&uri);
    let regions: Vec<i64> = folds
        .as_array()
        .expect("folding ranges")
        .iter()
        .filter(|fold| fold["kind"].as_str() != Some("comment"))
        .filter_map(|fold| fold["startLine"].as_i64())
        .collect();
    for (block, expected_start) in [
        ("endpoint /Common/www", 1),
        ("hsts {", 4),
        ("policy corporate", 9),
        ("check modern", 10),
        ("grade {", 14),
    ] {
        assert!(
            regions.contains(&expected_start),
            "`{block}` must fold from line {expected_start}: {regions:?}",
        );
    }
}

/// The outline names the declarations, and nests a block under the block that
/// contains it.
#[test]
fn the_outline_names_the_declarations() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, DOC);
    let symbols = lsp.document_symbols(&uri);
    let top: Vec<String> = symbols
        .as_array()
        .expect("document symbols")
        .iter()
        .filter_map(|symbol| symbol["name"].as_str().map(ToOwned::to_owned))
        .collect();
    assert_eq!(top, vec!["endpoint /Common/www", "policy corporate"]);
    // Nested blocks are children, not siblings — `symbol_names` flattens the
    // whole tree, so it sees them wherever they sit.
    let all = symbol_names(&symbols);
    for nested in ["hsts", "check modern", "grade"] {
        assert!(all.contains(nested), "outline must list `{nested}`: {all:?}");
    }
}

// -- diagnostics -------------------------------------------------------

/// The shipped sample yields exactly the notices its own header documents,
/// and no error at all. It is the vocabulary document's worked example, so a
/// change that makes it complain is a change to the contract.
#[test]
fn the_sample_yields_exactly_its_documented_notices() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, SAMPLE);
    let diags = lsp.pull_diagnostics(&uri);
    let sslic: Vec<String> = codes(&diags)
        .into_iter()
        .filter(|code| code.starts_with("SSLIC"))
        .collect();
    assert_eq!(
        sslic,
        vec![
            "SSLIC1101", "SSLIC1101", "SSLIC1101", "SSLIC1101", "SSLIC1101", "SSLIC1103",
        ],
        "the sample's four extension words (one twice) and its one predicate",
    );
    assert!(
        diags
            .iter()
            .all(|d| d["severity"].as_i64() != Some(ERROR)),
        "the sample must load with no errors: {:?}",
        coded(&diags),
    );
    // The loader owns the verdict on an unrecognised word, so no analyser
    // unknown-command hint doubles up on one.
    assert!(
        !codes(&diags).iter().any(|code| code == "W123"),
        "an extension word must not also draw W123: {:?}",
        coded(&diags),
    );
}

/// Three independent errors are all reported, each ranged over exactly the
/// text at fault.
#[test]
fn independent_errors_are_all_reported_with_exact_ranges() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, THREE_ERRORS);
    let diags = lsp.pull_diagnostics(&uri);
    let sslic: Vec<_> = coded(&diags)
        .into_iter()
        .filter(|(code, ..)| code.starts_with("SSLIC"))
        .collect();
    assert_eq!(
        sslic,
        vec![
            // `enabled maybe` — the offending value word only.
            ("SSLIC1009".to_owned(), ERROR, 4, 16, 4, 21),
            // `nonsense 1` — the whole member statement.
            ("SSLIC1007".to_owned(), ERROR, 5, 8, 5, 18),
            // `chain missing-chain` — the referenced name.
            ("SSLIC1011".to_owned(), ERROR, 7, 10, 7, 23),
        ],
    );
}

/// `grade` is reserved as a check identifier — the grade rule reports its own
/// finding under that id — so declaring `check grade { … }` is `SSLIC1009`.
#[test]
fn a_reserved_check_identifier_is_reported() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    let source = "sslictcl 1\n\
                  policy corporate {\n\
                  \x20   check grade {\n\
                  \x20       severity error\n\
                  \x20   }\n\
                  }\n";
    lsp.open_ready(&uri, source);
    let diags = lsp.pull_diagnostics(&uri);
    assert_eq!(
        coded(&diags)
            .into_iter()
            .filter(|(code, ..)| code.starts_with("SSLIC"))
            .collect::<Vec<_>>(),
        vec![("SSLIC1009".to_owned(), ERROR, 2, 10, 2, 15)],
    );
}

/// Fixing the document clears its errors — the diagnostics track the buffer,
/// not the file as first opened.
#[test]
fn editing_the_document_clears_its_errors() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, THREE_ERRORS);
    assert!(
        !codes(&lsp.pull_diagnostics(&uri)).is_empty(),
        "baseline errors expected before the fix"
    );
    lsp.settle_analysis(&uri, 2, DOC);
    let after = lsp.pull_diagnostics(&uri);
    assert!(
        after
            .iter()
            .all(|d| d["severity"].as_i64() != Some(ERROR)),
        "the corrected document has no errors left: {:?}",
        coded(&after),
    );
}

/// `tclLsp.diagnostics.SSLIC1101 = false` suppresses that code and leaves
/// every other one standing — the loader's codes honour the per-code switch
/// exactly as the analyser's do.
#[test]
fn a_disabled_code_is_suppressed_and_the_others_stand() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    // One preserved extension (`SSLIC1101`, a hint) beside one real error.
    let source = "sslictcl 1\n\
                  site-owner {web platform}\n\
                  endpoint /Common/a {\n\
                  \x20   hostname a.example.test\n\
                  \x20   chain missing-chain\n\
                  }\n";
    lsp.open_ready(&uri, source);
    let before = codes(&lsp.pull_diagnostics(&uri));
    assert!(before.iter().any(|code| code == "SSLIC1101"), "{before:?}");
    assert!(before.iter().any(|code| code == "SSLIC1011"), "{before:?}");

    lsp.apply_configuration_settle(
        json!({ "diagnostics": { "SSLIC1101": false } }),
        &uri,
        |config| {
            config["disabled_diagnostics"]
                .as_array()
                .is_some_and(|codes| codes.iter().any(|code| code == "SSLIC1101"))
        },
    );
    // A configuration change never bumps the document version, so the pull
    // cache still holds the pre-change report until the reschedule the config
    // change triggers republishes: wait for that publish rather than racing it.
    let after = codes(&lsp.await_diagnostics_settled(
        &uri,
        Duration::from_secs(15),
        |diags| !codes(diags).iter().any(|code| code == "SSLIC1101"),
    ));
    assert!(
        !after.iter().any(|code| code == "SSLIC1101"),
        "the disabled code is gone: {after:?}"
    );
    assert!(
        after.iter().any(|code| code == "SSLIC1011"),
        "every other code stands: {after:?}"
    );
}

/// `tclLsp.diagnostics.exclude` is a *file* glob (#1556): an excluded
/// `.sslictcl` document publishes nothing at all, loader findings included.
#[test]
fn an_excluded_document_publishes_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, THREE_ERRORS);
    assert!(!codes(&lsp.pull_diagnostics(&uri)).is_empty());

    let since = lsp.notification_cursor();
    lsp.apply_configuration(json!({ "diagnostics": { "exclude": ["*.sslictcl"] } }));
    assert_eq!(
        lsp.await_diagnostics_excluded(&uri, Duration::from_secs(15), since),
        Vec::<Value>::new(),
        "an excluded document publishes an empty set",
    );
    assert!(
        lsp.pull_diagnostics(&uri).is_empty(),
        "and its pulled report is empty too",
    );
}

/// The notices carry the severity the loader gave them: a preserved extension
/// is a hint, not a warning or an error.
#[test]
fn a_preserved_extension_is_a_hint() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("sslictcl");
    lsp.open_ready(&uri, "sslictcl 1\nsite-owner {web platform}\n");
    let diags = lsp.pull_diagnostics(&uri);
    let notice = coded(&diags)
        .into_iter()
        .find(|(code, ..)| code == "SSLIC1101")
        .expect("the unknown top-level declaration is preserved");
    assert_eq!(notice.1, HINT, "a preserved extension is a hint");
    assert_eq!(
        diags
            .iter()
            .find(|d| d["code"].as_str() == Some("SSLIC1101"))
            .and_then(|d| d["source"].as_str()),
        Some("tcl-lsp"),
        "and it carries the same `source` as every other diagnostic",
    );
}
