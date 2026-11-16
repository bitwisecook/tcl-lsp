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

//! The pack file as a document: its load notices, outline, hover, signature help, folding, formatting, quick fixes and links.

use super::*;

/// Load notices land as diagnostics **on the pack file**, at the line the
/// author wrote — and clear when the author fixes them.
#[test]
fn pack_load_notices_are_diagnostics_on_the_pack_file() {
    let root = workspace("notices");
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
         nonsense_property yes\n    }\n}\n",
    );

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let pack_uri = file_uri(&pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(15), |diags| {
            !diags.is_empty()
        });
    let notice = diagnostics
        .iter()
        .find(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("nonsense_property"))
        })
        .unwrap_or_else(|| {
            panic!("expected a notice about the unknown property: {diagnostics:#?}")
        });
    assert_eq!(
        notice.get("code").and_then(Value::as_str),
        Some("SPECTCL"),
        "notices carry the pack-load code"
    );
    assert_eq!(
        notice
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|p| p.get("line"))
            .and_then(Value::as_i64),
        Some(3),
        "on the line the author wrote it (1-based line 4)"
    );

    // Fix the pack: the badge must clear, not linger until restart.
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n    }\n}\n",
    );
    notify_pack_changed(&mut lsp, &pack, CHANGED);
    let cleared = lsp.await_diagnostics_settled(
        &pack_uri,
        std::time::Duration::from_secs(15),
        <[Value]>::is_empty,
    );
    assert!(cleared.is_empty(), "{cleared:#?}");

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.diagnostics.exclude` suppresses pack-load notices too.
///
/// Pack notices publish outside the analyser pipeline's exclusion gate, so
/// they need their own filter: without it, a `.tclspec` matching an exclude
/// glob keeps its `SPECTCL` badge indefinitely — the one diagnostic the
/// "produces no diagnostics at all" promise would still leak. Excluding the
/// pack after the notice is on screen also exercises the clearing path (the
/// config apply reloads packs, and the newly excluded URI joins the stale
/// set).
#[test]
fn diagnostics_exclude_suppresses_pack_load_notices() {
    let root = workspace("notices-excluded");
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
         nonsense_property yes\n    }\n}\n",
    );

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let pack_uri = file_uri(&pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(15), |diags| {
            !diags.is_empty()
        });
    assert!(
        diagnostics.iter().any(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("nonsense_property"))
        }),
        "expected a notice about the unknown property before excluding: {diagnostics:#?}"
    );

    // Exclude every pack file: the badge must clear without touching the pack.
    lsp.apply_configuration(json!({ "diagnostics": { "exclude": ["*.tclspec"] } }));
    let cleared = lsp.await_diagnostics_settled(
        &pack_uri,
        std::time::Duration::from_secs(15),
        <[Value]>::is_empty,
    );
    assert!(cleared.is_empty(), "{cleared:#?}");

    let _ = std::fs::remove_dir_all(&root);
}

// The pack file as a *document*: what a spec author sees while writing one.
//
// Everything above is about a pack's effect on the Tcl around it. These are
// about the `.tclspec` buffer itself — the Spec Studio's Pack DSL editor is a
// client of this very server, so whatever the server answers here is what a
// pack author gets, in the studio and in their own editor alike.

/// A pack with one command and enough shape to exercise every document
/// surface: nested blocks to outline and fold, statement words to hover, and
/// braced prose the formatter must not touch.
const AUTHORING_PACK: &str = "speclib mylib 1 {\n\
                              \x20   command mylib::x {\n\
                              \x20       arity 1\n\
                              \x20       hover {\n\
                              \x20           summary {Do a thing.}\n\
                              \x20       }\n\
                              \x20   }\n\
                              }\n";

/// Open a `.tclspec` in a workspace of its own and settle its diagnostics.
fn open_pack(name: &str, body: &str) -> (PathBuf, Lsp, String) {
    let root = workspace(name);
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, body);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&pack);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, body);
    (root, lsp, uri)
}

/// Every symbol name a document-symbol reply carries, parents before children.
fn symbol_names(node: &Value, out: &mut Vec<String>) {
    if let Some(name) = node.get("name").and_then(Value::as_str) {
        out.push(name.to_owned());
    }
    for child in node
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        symbol_names(child, out);
    }
}

/// A pack outlines as its declarations. Its dialect declares a document
/// grammar, so its *blocks* are its structure — the analyser's proc/namespace
/// symboliser has nothing to find in a file that declares rather than runs.
#[test]
fn a_pack_outlines_as_its_own_declarations() {
    let (root, mut lsp, uri) = open_pack("outline", AUTHORING_PACK);

    let symbols = lsp.document_symbols(&uri);
    let mut names = Vec::new();
    for node in symbols.as_array().into_iter().flatten() {
        symbol_names(node, &mut names);
    }
    assert!(
        names.iter().any(|n| n.contains("speclib mylib")),
        "the pack itself is the outline root: {symbols:#?}"
    );
    assert!(
        names.iter().any(|n| n.contains("command mylib::x")),
        "each declared command is a child: {symbols:#?}"
    );
    assert!(
        names.iter().any(|n| n.contains("hover")),
        "and a nested block is a child of the command: {symbols:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Hover on a pack's own statement word answers from the `SpecTcl` command
/// pack — through the real handler, not the provider in isolation.
#[test]
fn a_pack_statement_word_hovers_from_the_spectcl_surface() {
    let (root, mut lsp, uri) = open_pack("authoring-hover", AUTHORING_PACK);

    let speclib = hover_text(&lsp.hover(&uri, 0, 2));
    assert!(
        speclib.contains("Open a SpecTcl command pack."),
        "the document's root word hovers as SpecTcl vocabulary; got:\n{speclib}"
    );
    // …and a *property* word, which only means anything inside a `command`
    // body: the same word in an ordinary Tcl script is not a command at all.
    let arity = hover_text(&lsp.hover(&uri, 2, 9));
    assert!(
        arity.contains("Declare how many argument words the call takes."),
        "a property word inside a command body hovers too; got:\n{arity}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Signature help on a pack statement offers that statement's own form, not
/// an ordinary Tcl command's.
#[test]
fn a_pack_statement_offers_signature_help() {
    let (root, mut lsp, uri) = open_pack("authoring-signature", AUTHORING_PACK);

    let help = lsp.signature_help(&uri, 0, 8);
    let label = help
        .get("signatures")
        .and_then(Value::as_array)
        .and_then(|sigs| sigs.first())
        .and_then(|sig| sig.get("label"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    assert!(
        label.contains("speclib"),
        "signature help names the statement being written; got {help:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack folds on its blocks: the `speclib` body, the `command` body, and the
/// `hover` body inside it.
#[test]
fn a_packs_blocks_are_folding_ranges() {
    let (root, mut lsp, uri) = open_pack("authoring-folding", AUTHORING_PACK);

    let ranges = lsp.folding_range(&uri);
    let starts: Vec<i64> = ranges
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.get("startLine").and_then(Value::as_i64))
        .collect();
    for (line, what) in [(0, "speclib"), (1, "command"), (3, "hover")] {
        assert!(
            starts.contains(&line),
            "the `{what}` block must fold (line {line}); got {ranges:#?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// Formatting a pack must leave its byte-significant prose alone.
///
/// `summary` / `description` / `snippet` / `examples` are every byte between
/// their braces, verbatim — that is what lets a ported field compare byte for
/// byte with the `&'static str` it came from — so a formatter that rewrapped
/// one would quietly change what the editor shows for the command the pack
/// describes. (`tcl-spectcl`'s `pack_formatting` suite makes the same claim
/// over the shipped example packs, by reloading them; this one pins that the
/// real `textDocument/formatting` handler is the thing being asked.)
#[test]
fn formatting_a_pack_keeps_its_prose_verbatim() {
    const PROSE: &str = "Two  spaces and\n            an indented line.";
    let body = format!(
        "speclib mylib 1 {{\n\
         \x20   command mylib::x {{\n\
         \x20     arity 1\n\
         \x20       hover {{\n\
         \x20           summary {{{PROSE}}}\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let (root, mut lsp, uri) = open_pack("authoring-formatting", &body);

    let edits = lsp.formatting(&uri, 4, true);
    let formatted = edits
        .as_array()
        .into_iter()
        .flatten()
        .next()
        .and_then(|edit| edit.get("newText"))
        .and_then(Value::as_str)
        .map_or_else(|| body.clone(), ToOwned::to_owned);
    assert!(
        formatted.contains(PROSE),
        "the formatter rewrote the pack's prose:\n{formatted}"
    );
    assert!(
        formatted.contains("        arity 1"),
        "…while still doing its job on the declarations:\n{formatted}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// An **open** pack keeps its load notices while it is edited.
///
/// A `.tclspec` is both a spec pack and an analysed Tcl document, and
/// `publishDiagnostics` replaces a URI's whole set — so the two used to
/// overwrite each other and editing a pack showed the analyser's view alone
/// until the next reload put the notices back. The published set is now the
/// union, which is what this asserts: an analyser diagnostic the *edit*
/// introduces and the loader's `SPECTCL` notice, on screen together.
#[test]
fn an_open_pack_keeps_its_notices_while_it_is_edited() {
    /// A word the pack cannot already carry, so the settle barrier below is a
    /// fact of the edited buffer and not of the one it replaced.
    const ADDED: &str = "a_word_only_this_edit_carries";

    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
                nonsense_property yes\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("open-pack-notices", pack);

    let has_notice = |diags: &[Value]| {
        diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) == Some("SPECTCL")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("nonsense_property"))
        })
    };
    let settled =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), |diags| {
            has_notice(diags)
        });
    assert!(has_notice(&settled), "{settled:#?}");

    // Edit the buffer so the *analyser* has something of its own to say that
    // it could not have said before — the settle barrier must be a fact of the
    // edited text, or a stale pre-edit publication satisfies it.
    let edited = format!("{pack}\n{ADDED}\n");
    lsp.replace_document(&uri, 2, &edited);
    let mentions_edit = |diags: &[Value]| {
        diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) != Some("SPECTCL")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains(ADDED))
        })
    };
    let after = lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(20), |diags| {
        mentions_edit(diags)
    });
    assert!(
        mentions_edit(&after),
        "the analyser must have its say on the edited buffer: {after:#?}"
    );
    assert!(
        has_notice(&after),
        "…and the loader's notice must survive alongside it: {after:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A dropped property offers the word the author meant.
///
/// The `SpecTcl` vocabulary is closed — a property word is a member of the
/// grammar in force where it was written — so a notice naming a dropped word
/// is a typo with one computable correction.
#[test]
fn a_dropped_property_offers_a_did_you_mean_fix() {
    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("did-you-mean", pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), |diags| {
            diags.iter().any(|d| {
                d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("arty"))
            })
        });
    let notice = diagnostics
        .iter()
        .find(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("arty"))
        })
        .unwrap_or_else(|| panic!("the dropped-property notice: {diagnostics:#?}"))
        .clone();

    let actions = lsp.code_actions(&uri, notice["range"].clone(), json!([notice]));
    let fix = actions
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| {
            a.get("title")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains("arity"))
        })
        .unwrap_or_else(|| panic!("a did-you-mean quick fix: {actions:#?}"));
    assert_eq!(fix.get("kind").and_then(Value::as_str), Some("quickfix"));
    let edit = fix["edit"]["changes"][&uri][0].clone();
    assert_eq!(edit.get("newText").and_then(Value::as_str), Some("arity"));
    assert_eq!(edit["range"]["start"]["line"].as_i64(), Some(2));
    assert_eq!(edit["range"]["start"]["character"].as_i64(), Some(8));

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack's `include NAME` row is a document link to the sibling file it
/// names — the same file the loader's include resolver would read.
#[test]
fn a_pack_include_row_is_a_document_link() {
    let pack = "speclib mylib 1 {\n    include shared.tclspec\n    \
                command mylib::x {\n        arity 1\n    }\n}\n";
    let root = workspace("include-link");
    write(&root.join(".tcl-lsp/shared.tclspec"), "");
    let file = root.join(".tcl-lsp/mylib.tclspec");
    write(&file, pack);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&file);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, pack);

    let links = lsp.request(
        "textDocument/documentLink",
        json!({ "textDocument": { "uri": &uri } }),
    );
    let link = links
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| {
            l.get("target")
                .and_then(Value::as_str)
                .is_some_and(|t| t.ends_with("shared.tclspec"))
        })
        .unwrap_or_else(|| panic!("a link to the included pack: {links:#?}"));
    assert_eq!(link["range"]["start"]["line"].as_i64(), Some(1));

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.features.diagnostics = false` clears a pack's load notices too.
///
/// The notices are a layer the publisher unions into every set a `.tclspec`
/// gets — the pushed one and the pulled one alike — so a master switch that
/// only emptied the *analysed* set left the squiggles standing, which is the
/// one thing that switch promises not to do. Both paths are asserted, and
/// re-enabling has to bring the notice back: a clear that cannot be undone is
/// as wrong as one that never happens.
#[test]
fn the_diagnostics_master_switch_clears_a_packs_notices() {
    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("master-switch-notices", pack);

    let has_notice = |diags: &[Value]| {
        diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("SPECTCL"))
    };
    let pushed =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), has_notice);
    assert!(has_notice(&pushed), "{pushed:#?}");
    let pulled = lsp.pull_diagnostics(&uri);
    assert!(
        has_notice(&pulled),
        "the pull path carries it too: {pulled:#?}"
    );

    let since = lsp.notification_cursor();
    lsp.apply_configuration(json!({ "features": { "diagnostics": false } }));
    assert_eq!(
        lsp.await_diagnostics_master_off(&uri, std::time::Duration::from_secs(20), since),
        Vec::<Value>::new(),
        "the push path must clear the notice"
    );
    let pulled = lsp.pull_diagnostics(&uri);
    assert_eq!(
        pulled,
        Vec::<Value>::new(),
        "…and the pull path must agree with it: {pulled:#?}"
    );

    lsp.apply_configuration(json!({ "features": { "diagnostics": true } }));
    let pushed =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(20), has_notice);
    assert!(has_notice(&pushed), "re-enabling restores it: {pushed:#?}");
    let pulled = lsp.pull_diagnostics(&uri);
    assert!(has_notice(&pulled), "…on both paths: {pulled:#?}");

    let _ = std::fs::remove_dir_all(&root);
}
