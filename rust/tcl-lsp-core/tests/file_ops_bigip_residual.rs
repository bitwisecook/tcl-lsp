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

//! Residual-coverage port tests for two small `tcl-lsp-core` files that the
//! existing suites leave in the high-80s%:
//!
//! * `src/file_ops.rs` — `compute_rename_edits` (rewrite `source FILE`
//!   literals on rename): `file://` URI / percent-decoding edges, the
//!   non-file-URI declines, absolute-vs-relative literal styles, the
//!   workspace-root fallback, and the pure path helpers.
//! * `src/bigip.rs` — F5 BIG-IP `*.conf` handling: `is_bigip_conf_name`
//!   basename matching and `document_symbols` stanza extraction
//!   (brace/quote/comment scanning, multi-word object-type resolution,
//!   nameless-singleton fallback, malformed / unterminated input).
//!
//! ## Proof discipline
//!
//! BIG-IP `*.conf` is **not** stock Tcl (it is a tree of
//! `module object-type id { … }` stanzas with embedded Tcl only inside
//! `ltm rule` bodies), and `file_ops` is LSP plumbing (URI handling + path
//! arithmetic). Neither encodes a stock-Tcl-language fact, so every
//! assertion here is **structural** — checked against each module's
//! documented contract, not against C-Tcl. No `tclsh-proof` comment is
//! therefore warranted.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::bigip::{document_symbols, is_bigip_conf_name};
use tcl_lsp_core::document_symbols::{DocumentSymbol, SymbolKind};
use tcl_lsp_core::file_ops::compute_rename_edits;
use tcl_lsp_core::workspace_index::WorkspaceIndex;

// file_ops.rs — `source`-literal rename edits + URI / path helpers.
//
// The module's only public entry is `compute_rename_edits`; the private
// helpers (`uri_to_path`, `percent_decode`, `source_resolves_to`,
// `compute_new_literal`, `normpath`, `relpath`, `content_span`) are driven
// through it. Each test below targets a specific branch / edge by choosing
// fixtures that route execution through it.

/// Build a one-document workspace index from `(uri, src)`.
fn index_of(uri: &str, src: &str) -> WorkspaceIndex {
    let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
    let mut idx = WorkspaceIndex::new();
    idx.add_document(uri, &analysis);
    idx
}

#[test]
fn rename_unrelated_index_yields_no_edits() {
    // A document that `source`s nothing → no edits (empty-result return).
    let idx = index_of("file:///proj/main.tcl", "puts hi\n");
    let edits = compute_rename_edits("file:///proj/a.tcl", "file:///proj/b.tcl", &idx, &[]);
    assert!(edits.is_empty());
}

#[test]
fn rename_non_file_old_uri_declines() {
    // A non-`file:` `old_uri` is unrenameable — the `let-else` at the top
    // returns an empty vec before scanning the index (line 44 branch).
    let idx = index_of("file:///proj/main.tcl", "source lib/old.tcl\n");
    // `untitled:` old URI: `uri_to_path` → None → decline.
    let edits = compute_rename_edits("untitled:Untitled-1", "file:///proj/lib/new.tcl", &idx, &[]);
    assert!(edits.is_empty(), "non-file old_uri must decline: {edits:?}");
}

#[test]
fn rename_non_file_new_uri_declines() {
    // Symmetric: a non-`file:` `new_uri` also declines (both halves of the
    // tuple must be `Some`).
    let idx = index_of("file:///proj/main.tcl", "source lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "vscode-vfs://host/new.tcl",
        &idx,
        &[],
    );
    assert!(edits.is_empty(), "non-file new_uri must decline: {edits:?}");
}

#[test]
fn rename_skips_substituted_source_path() {
    // `source $dynamic` is not a literal → the `!src.is_literal` guard
    // skips it (the conservative branch), leaving it untouched.
    let idx = index_of("file:///proj/main.tcl", "source $dynamic\n");
    let edits = compute_rename_edits("file:///proj/dynamic", "file:///proj/renamed", &idx, &[]);
    assert!(
        edits.is_empty(),
        "substituted source must be skipped: {edits:?}"
    );
}

#[test]
fn rename_literal_pointing_elsewhere_is_left_alone() {
    // A literal `source other.tcl` that does not resolve to the renamed
    // file → `source_resolves_to` false → no edit.
    let idx = index_of("file:///proj/main.tcl", "source other.tcl\n");
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///proj/lib/new.tcl",
        &idx,
        &[],
    );
    assert!(
        edits.is_empty(),
        "unrelated literal must be left alone: {edits:?}"
    );
}

#[test]
fn rename_relative_literal_bare_word_rewrites_full_span() {
    // A bare (undelimited) relative literal: the edit span covers the whole
    // path word (delimiter width 0 in `content_span`) and rewrites to the
    // re-relativised new path.
    let src = "source lib/old.tcl\n";
    let idx = index_of("file:///proj/main.tcl", src);
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///proj/lib/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].uri, "file:///proj/main.tcl");
    assert_eq!(edits[0].new_text, "lib/new.tcl");
    // The covered span is exactly the bare path word.
    assert_eq!(&src[edits[0].span.as_range()], "lib/old.tcl");
}

#[test]
fn rename_braced_and_quoted_literals_edit_content_only() {
    // For a `{…}` / `"…"` literal, `content_span` narrows the edit to the
    // path content so the opening/closing delimiter is preserved — applying
    // the edit must reproduce the delimiter-wrapped new path.
    for src in ["source {lib/old.tcl}\n", "source \"lib/old.tcl\"\n"] {
        let idx = index_of("file:///proj/main.tcl", src);
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert_eq!(edits.len(), 1, "{src:?}");
        assert_eq!(
            &src[edits[0].span.as_range()],
            "lib/old.tcl",
            "span for {src:?}"
        );
        let mut applied = src.to_string();
        applied.replace_range(edits[0].span.as_range(), &edits[0].new_text);
        assert_eq!(
            applied,
            src.replace("lib/old.tcl", "lib/new.tcl"),
            "{src:?}"
        );
    }
}

#[test]
fn rename_absolute_literal_becomes_new_absolute_path() {
    // An absolute literal resolves only to itself (`source_resolves_to`
    // leading-`/` branch) and rewrites to the new absolute path
    // (`compute_new_literal` leading-`/` branch).
    let idx = index_of("file:///proj/main.tcl", "source /proj/lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///elsewhere/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "/elsewhere/new.tcl");
}

#[test]
fn rename_resolves_relative_literal_via_workspace_root() {
    // `/proj/sub/main.tcl` does `source helper.tcl`, but the file lives at
    // the workspace root, not next to the script. Without a root it does
    // not match; with one the roots branch in `matched_base` hits and the
    // rewrite re-relativises against the *same* base it matched under (the
    // root) — so it stays root-relative (`helper2.tcl`), not `../helper2.tcl`
    // (which would resolve elsewhere at runtime, issue 178).
    let idx = index_of("file:///proj/sub/main.tcl", "source helper.tcl\n");
    let no_root = compute_rename_edits(
        "file:///proj/helper.tcl",
        "file:///proj/helper2.tcl",
        &idx,
        &[],
    );
    assert!(no_root.is_empty(), "no root → no match: {no_root:?}");

    let with_root = compute_rename_edits(
        "file:///proj/helper.tcl",
        "file:///proj/helper2.tcl",
        &idx,
        &["file:///proj".to_string()],
    );
    assert_eq!(with_root.len(), 1, "{with_root:?}");
    assert_eq!(with_root[0].new_text, "helper2.tcl");
}

#[test]
fn rename_drops_non_file_workspace_roots() {
    // A non-`file:` workspace root is filtered out (`uri_to_path` → None in
    // the `root_paths` `filter_map`), so a literal that would only resolve
    // via that root finds no base and yields no edit.
    let idx = index_of("file:///proj/sub/main.tcl", "source helper.tcl\n");
    let edits = compute_rename_edits(
        "file:///proj/helper.tcl",
        "file:///proj/helper2.tcl",
        &idx,
        // Only a non-file root → dropped → helper.tcl is unresolvable.
        &["untitled:workspace".to_string()],
    );
    assert!(edits.is_empty(), "non-file root must be dropped: {edits:?}");
}

#[test]
fn rename_percent_decoded_uri_path_matches() {
    // The dependent and renamed URIs carry a `%20` (space) in their
    // directory; `uri_to_path`'s `percent_decode` must decode it so the
    // path comparison still resolves. Exercises the `%`-escape branch +
    // `hex_val` for both nibbles.
    let idx = index_of("file:///my%20proj/main.tcl", "source lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:///my%20proj/lib/old.tcl",
        "file:///my%20proj/lib/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(
        edits.len(),
        1,
        "percent-encoded dir must resolve: {edits:?}"
    );
    assert_eq!(edits[0].new_text, "lib/new.tcl");
}

#[test]
fn rename_uri_with_authority_host_is_stripped_to_path() {
    // A `file://AUTHORITY/path` URI (note the *four* leading slashes:
    // `file:////host/…` → `rest == "//host/…"`) drives the
    // `strip_prefix("//")` + `find('/')` host-stripping branch in
    // `uri_to_path`: the `host` segment is dropped and the absolute path
    // kept (`/lib/old.tcl`). All three URIs share the host so they decode
    // consistently and the absolute literal still resolves.
    let idx = index_of("file:////host/main.tcl", "source /lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:////host/lib/old.tcl",
        "file:////host/lib/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(edits.len(), 1, "host-authority URI must resolve: {edits:?}");
    // The absolute literal rewrites to the host-stripped new path.
    assert_eq!(edits[0].new_text, "/lib/new.tcl");
}

#[test]
fn rename_dotdot_relative_literal_is_normalised() {
    // A `../lib/old.tcl` literal from `/proj/sub/main.tcl` resolves up one
    // level via `normpath`'s `..` handling; the rewrite re-relativises the
    // new path back to the script directory.
    let src = "source ../lib/old.tcl\n";
    let idx = index_of("file:///proj/sub/main.tcl", src);
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///proj/lib/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "../lib/new.tcl");
}

#[test]
fn rename_root_level_dependent_relativises_without_dotdot() {
    // A dependent at the filesystem root (`/main.tcl`) sourcing `old.tcl`
    // exercises `posix_dirname`'s `Some(0)` arm (dir is `"/"`) and a
    // common-prefix-only `relpath` (no `..` ups).
    let idx = index_of("file:///main.tcl", "source old.tcl\n");
    let edits = compute_rename_edits("file:///old.tcl", "file:///new.tcl", &idx, &[]);
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "new.tcl");
}

#[test]
fn rename_percent_decode_handles_upper_and_lower_hex_digits() {
    // `%2d` and `%2D` both decode to `-`, driving `hex_val`'s lowercase
    // (`a-f`) and uppercase (`A-F`) arms respectively. The dependent dir
    // `my%2dproj` and the renamed file's dir `my%2Dproj` both decode to
    // `my-proj`, so they resolve to the same directory and the rename
    // matches.
    let idx = index_of("file:///my%2dproj/main.tcl", "source lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:///my%2Dproj/lib/old.tcl",
        "file:///my%2Dproj/lib/new.tcl",
        &idx,
        &[],
    );
    assert_eq!(
        edits.len(),
        1,
        "mixed-case hex escapes must decode: {edits:?}"
    );
    assert_eq!(edits[0].new_text, "lib/new.tcl");
}

#[test]
fn rename_skips_dependent_with_non_file_uri() {
    // The renamed file's URIs are `file:`, but the *dependent* document is
    // `untitled:` — its source literal cannot be located on disk, so the
    // per-source `uri_to_path(&src.uri)` returns `None` and the loop
    // `continue`s (the dep-URI skip branch), yielding no edit.
    let idx = index_of("untitled:Untitled-1", "source /proj/lib/old.tcl\n");
    let edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///proj/lib/new.tcl",
        &idx,
        &[],
    );
    assert!(
        edits.is_empty(),
        "untitled dependent must be skipped: {edits:?}"
    );
}

#[test]
fn rename_slashless_dependent_and_target_resolve() {
    // A degenerate `file://word` URI decodes to a slash-free path
    // (`main.tcl` / `old.tcl`), exercising `posix_dirname`'s `None` arm
    // (dir == "") and `posix_join("", rel)`. The dependent `file://main.tcl`
    // sources a bare `old.tcl`; the renamed file is `file://old.tcl`. The
    // relative literal joins against the empty dir and *does* resolve — the
    // match succeeds and an edit is produced.
    let idx = index_of("file://main.tcl", "source old.tcl\n");
    let edits = compute_rename_edits("file://old.tcl", "file://new.tcl", &idx, &[]);
    assert_eq!(edits.len(), 1, "slash-free paths still resolve: {edits:?}");
    // `compute_new_literal` re-relativises against the empty directory:
    // `normpath("")` collapses to `.`, which `relpath` treats as one
    // component, so the rewrite is `../new.tcl`. (Real LSP file URIs are
    // always absolute `file:///…`, so this only exercises the degenerate
    // path-helper edges — there is no canonical relativisation of a path
    // with no directory.)
    assert_eq!(edits[0].new_text, "../new.tcl");
}

#[test]
fn rename_handles_multiple_dependents_in_one_pass() {
    // Two distinct dependents both source the renamed file via different
    // literal styles; both get an edit (loop accumulates more than one).
    let mut idx = WorkspaceIndex::new();
    for (uri, src) in [
        ("file:///proj/a.tcl", "source lib/old.tcl\n"),
        ("file:///proj/b.tcl", "source /proj/lib/old.tcl\n"),
    ] {
        let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
        idx.add_document(uri, &analysis);
    }
    let mut edits = compute_rename_edits(
        "file:///proj/lib/old.tcl",
        "file:///proj/lib/new.tcl",
        &idx,
        &[],
    );
    edits.sort_by(|a, b| a.uri.cmp(&b.uri));
    assert_eq!(edits.len(), 2, "{edits:?}");
    assert_eq!(edits[0].uri, "file:///proj/a.tcl");
    assert_eq!(edits[0].new_text, "lib/new.tcl");
    assert_eq!(edits[1].uri, "file:///proj/b.tcl");
    assert_eq!(edits[1].new_text, "/proj/lib/new.tcl");
}

// bigip.rs — `is_bigip_conf_name` + `document_symbols` stanza outline.
//
// BIG-IP `*.conf` is not Tcl; these assertions are structural against the
// module contract (canonical basenames, three-level module→kind→object
// outline, no empty symbol names).

#[test]
fn bigip_recognises_canonical_basenames_case_insensitively() {
    // Every canonical name matches by basename (path + backslash stripped),
    // case-insensitively; non-canonical names do not.
    assert!(is_bigip_conf_name("file:///x/bigip.conf"));
    assert!(is_bigip_conf_name("bigip_base.conf"));
    assert!(is_bigip_conf_name("bigip_gtm.conf"));
    assert!(is_bigip_conf_name("bigip_script.conf"));
    assert!(is_bigip_conf_name("bigip_user.conf"));
    // Case-folding + a Windows-style backslash separator.
    assert!(is_bigip_conf_name("C:\\config\\BIGIP.CONF"));
    assert!(is_bigip_conf_name("file:///etc/BIGIP_BASE.CONF"));
    // A bare basename (no separator → `rsplit` yields the whole string).
    assert!(is_bigip_conf_name("bigip.conf"));
    // Non-matches.
    assert!(!is_bigip_conf_name("file:///x/app.tcl"));
    assert!(!is_bigip_conf_name("file:///x/notbigip.conf"));
    assert!(!is_bigip_conf_name("bigip.conf.bak"));
    assert!(!is_bigip_conf_name(""));
}

/// Collect every symbol name (depth-first) from an outline.
fn names(syms: &[DocumentSymbol]) -> Vec<String> {
    fn walk(syms: &[DocumentSymbol], out: &mut Vec<String>) {
        for s in syms {
            out.push(s.name.clone());
            walk(&s.children, out);
        }
    }
    let mut out = Vec::new();
    walk(syms, &mut out);
    out
}

/// Find a top-level module folder by name.
fn module<'a>(syms: &'a [DocumentSymbol], name: &str) -> Option<&'a DocumentSymbol> {
    syms.iter().find(|s| s.name == name)
}

#[test]
fn bigip_empty_source_yields_no_symbols() {
    // The `source.is_empty()` early return.
    assert!(document_symbols("").is_empty());
}

#[test]
fn bigip_non_stanza_text_yields_no_symbols() {
    // Lines with no `{ … }` block header never become symbols: a header
    // whose newline arrives before any `{` is skipped (the `hit_newline`
    // branch in `extract_blocks`), and a single bare word (`parts.len() < 2`
    // in `parse_header`) is dropped.
    assert!(document_symbols("just some words\nand more\n").is_empty());
    // A lone single-token line cannot form a `(module, type)` pair.
    assert!(document_symbols("ltm {\n}\n").is_empty());
}

#[test]
fn bigip_builds_three_level_module_kind_object_outline() {
    // A named `ltm pool /Common/p1 { … }` stanza becomes
    // module(`ltm`) → kind(`pool`) → object(`/Common/p1`), with the right
    // `SymbolKind` at each level.
    let src = "ltm pool /Common/p1 {\n    members {\n        1.2.3.4:80 { }\n    }\n}\n";
    let syms = document_symbols(src);
    let ltm = module(&syms, "ltm").expect("an `ltm` module folder");
    assert_eq!(ltm.kind, SymbolKind::Module);
    let pool = ltm
        .children
        .iter()
        .find(|c| c.name == "pool")
        .expect("a `pool` kind group");
    assert_eq!(pool.kind, SymbolKind::Class);
    let obj = &pool.children[0];
    assert_eq!(obj.name, "/Common/p1");
    assert_eq!(obj.kind, SymbolKind::Variable);
    // Selection range mirrors the outer range for a leaf.
    assert_eq!(obj.range, obj.selection_range);
}

#[test]
fn bigip_nameless_singleton_falls_back_to_kind_label() {
    // `auth password-policy { … }` is a two-token header → identifier is
    // empty (`parts.len() == 2` arm), so the leaf falls back to the
    // object-type label rather than carrying an empty name.
    let src = "auth password-policy {\n    lockout-duration 10\n}\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(all.contains(&"auth".to_owned()), "{all:?}");
    assert!(all.contains(&"password-policy".to_owned()), "{all:?}");
    assert!(all.iter().all(|n| !n.is_empty()), "no empty name: {all:?}");
}

#[test]
fn bigip_multi_word_object_type_resolved_via_registry() {
    // `sys diags ihealth { … }` — the registry knows `diags ihealth` as a
    // multi-word object type with no identifier, so the longest-prefix /
    // whole-tail singleton branch in `parse_header` resolves it to
    // module(`sys`) → kind(`diags ihealth`).
    let src = "sys diags ihealth {\n    user admin\n}\n";
    let syms = document_symbols(src);
    let sys_module = module(&syms, "sys").expect("a `sys` module: {syms:?}");
    assert!(
        sys_module
            .children
            .iter()
            .any(|c| c.name == "diags ihealth"),
        "expected a `diags ihealth` kind group: {:?}",
        names(&syms),
    );
}

#[test]
fn bigip_heuristic_fallback_for_unknown_three_token_header() {
    // A module whose three-token header is *not* in the registry index
    // falls through to the heuristic: middle token(s) = object type, last
    // token = identifier. `zzz unknownkind myid { }` → module `zzz`, kind
    // `unknownkind`, object `myid`.
    let src = "zzz unknownkind myid {\n}\n";
    let syms = document_symbols(src);
    let zzz = module(&syms, "zzz").expect("a `zzz` module folder");
    let kind = &zzz.children[0];
    assert_eq!(kind.name, "unknownkind");
    assert_eq!(kind.children[0].name, "myid");
}

#[test]
fn bigip_quoted_identifier_with_embedded_space_is_one_token() {
    // A quoted name with an embedded space stays a single identifier token
    // (the `tokenise_header` in-quote branch), so it is not mis-split into
    // a separate object-type word.
    let src = "security bot-defense signature \"/Common/Has Space\" {\n}\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(
        all.contains(&"/Common/Has Space".to_owned()),
        "quoted space-bearing identifier kept whole: {all:?}",
    );
}

#[test]
fn bigip_backslash_escape_in_header_is_literal() {
    // A backslash escape in the header (`\"`-style) is consumed literally by
    // `tokenise_header`'s escape branch — the escaped quote does not toggle
    // quote mode. Here `ltm node a\ b { }` keeps the escaped space inside
    // the identifier word.
    let src = "ltm node a\\ b {\n}\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(
        all.contains(&"a b".to_owned()),
        "escaped space joins token: {all:?}"
    );
}

#[test]
fn bigip_comment_and_blank_lines_are_skipped() {
    // A leading `#` comment line and surrounding blank lines are skipped by
    // `extract_blocks` (the comment + whitespace branches) before the real
    // stanza is parsed.
    let src = "# a comment\n\n  # another\nltm pool /Common/p {\n}\n";
    let syms = document_symbols(src);
    assert!(module(&syms, "ltm").is_some(), "{:?}", names(&syms));
    // No symbol is named after comment text.
    let all = names(&syms);
    assert!(!all.iter().any(|n| n.contains("comment")), "{all:?}");
}

#[test]
fn bigip_nested_and_quoted_braces_inside_body_dont_break_scan() {
    // The brace-balanced scan respects a `"…}…"` quoted brace and a nested
    // `{ }` block inside the body, so the stanza's closing brace is found at
    // the right depth and the following stanza is parsed independently.
    let src =
        "ltm rule /Common/r {\n    set x \"a}b\"\n    if { 1 } { }\n}\nltm pool /Common/p {\n}\n";
    let syms = document_symbols(src);
    let ltm = module(&syms, "ltm").expect("ltm module");
    let all = names(&syms);
    // Both the rule and the pool survive as separate objects.
    assert!(all.contains(&"/Common/r".to_owned()), "{all:?}");
    assert!(all.contains(&"/Common/p".to_owned()), "{all:?}");
    // `ltm` groups two kinds (`rule`, `pool`), sorted by the BTreeMap.
    assert_eq!(ltm.children.len(), 2, "{all:?}");
}

#[test]
fn bigip_backslash_escape_inside_body_does_not_unbalance() {
    // A backslash inside the stanza body (`\}`) is consumed by the depth
    // scan's escape arm (`b'\\' if pos + 1 < length => pos += 1`), so the
    // escaped `}` is *not* counted as a closing brace and the real closer is
    // found correctly. The next stanza then parses independently.
    let src = "ltm rule /Common/r {\n    set x a\\}b\n}\nltm pool /Common/p {\n}\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(all.contains(&"/Common/r".to_owned()), "{all:?}");
    assert!(
        all.contains(&"/Common/p".to_owned()),
        "escaped brace kept balance: {all:?}"
    );
}

#[test]
fn bigip_escaped_quote_inside_quoted_body_span_is_respected() {
    // A `\"` *inside* a quoted span in the body drives the inner
    // `if bytes[pos] == b'\\' && pos + 1 < length` skip (line 122): the
    // escaped quote does not close the quoted span, so the `}` that follows
    // the closing real `"` is the stanza closer. Both stanzas survive.
    let src = "ltm rule /Common/r {\n    set x \"a\\\"}b\"\n}\nltm pool /Common/p {\n}\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(all.contains(&"/Common/r".to_owned()), "{all:?}");
    assert!(
        all.contains(&"/Common/p".to_owned()),
        "escaped quote inside quoted body kept balance: {all:?}",
    );
}

#[test]
fn bigip_unterminated_block_still_recorded() {
    // A header with an opening `{` but no closing `}` runs the depth loop to
    // end-of-input and still records the block (the `depth > 0` loop exits
    // on `pos >= length`). The object is surfaced, not dropped.
    let src = "ltm pool /Common/unterminated {\n    members {\n";
    let syms = document_symbols(src);
    let all = names(&syms);
    assert!(
        all.contains(&"/Common/unterminated".to_owned()),
        "unterminated stanza still recorded: {all:?}",
    );
}

#[test]
fn bigip_header_running_off_end_without_brace_is_dropped() {
    // Input that ends mid-header with no `{` at all (and no newline) hits
    // the `pos >= length` run-off-the-end `break` in `extract_blocks` and
    // produces no block.
    assert!(document_symbols("ltm pool /Common/p").is_empty());
}

#[test]
fn bigip_multiple_objects_same_kind_are_grouped_and_sorted() {
    // Two `ltm pool` stanzas group under one `pool` kind folder, ordered by
    // source position (the `entries.sort_by_key(start byte)` path), and the
    // kind/module ranges span both (`span_of` / `span_over_children` fold).
    let src = "ltm pool /Common/first {\n}\nltm pool /Common/second {\n}\n";
    let syms = document_symbols(src);
    let ltm = module(&syms, "ltm").expect("ltm module");
    let pool = &ltm.children[0];
    assert_eq!(pool.name, "pool");
    let objs: Vec<&str> = pool.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(objs, ["/Common/first", "/Common/second"], "source order");
    // The kind range spans from the first object's start to the second's
    // end (later end-line than the first object alone).
    assert!(
        pool.range.end_line >= pool.children[1].range.start_line,
        "kind range covers both objects: {:?}",
        pool.range,
    );
}
