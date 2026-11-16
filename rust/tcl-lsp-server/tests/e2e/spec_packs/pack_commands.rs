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

//! A workspace pack's commands behave like shipped ones: resolution, hover, argument roles, arity, discovery and the refusal to redefine a shipped command.

use super::*;

/// The headline case: a workspace pack makes an unknown command resolve, with
/// hover.
///
/// Without the pack, `mylib::with_var` is a command the server has never heard
/// of — W123, no hover. With a `.tclspec` under `.tcl-lsp/`, it hovers with the
/// pack's own summary and synopsis and stops being unknown. Nothing in the
/// document, the settings, or the language id says "pack": discovery finds it
/// by convention.
#[test]
fn a_workspace_pack_makes_an_unknown_command_resolve_with_hover() {
    let root = workspace("hover");
    write(&root.join(".tcl-lsp/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "hover came from the pack's own summary; got:\n{hover}"
    );
    assert!(
        hover.contains("mylib::with_var varName script ?mode?"),
        "hover carries the pack's synopsis; got:\n{hover}"
    );

    // Named, not counted: the bundled tier ships the EDA loadables, so the
    // list is never just this workspace's pack.
    let packs = loaded_packs(&mut lsp);
    let mine = packs
        .iter()
        .find(|pack| pack.get("name").and_then(Value::as_str) == Some("mylib"))
        .unwrap_or_else(|| panic!("the workspace pack: {packs:#?}"));
    assert_eq!(mine.get("tier").and_then(Value::as_str), Some("workspace"));

    let _ = std::fs::remove_dir_all(&root);
}

/// Pack argument roles must reach the memoised semantic-token query too, not
/// only hover and diagnostics. `Body` recurses into the script and `VarWrite`
/// paints the caller variable as a variable rather than a string.
#[test]
fn a_workspace_packs_argument_roles_drive_semantic_tokens() {
    let root = workspace("semantic-tokens");
    write(&root.join(".tcl-lsp/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, source);

    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
    assert!(
        tokens.iter().any(|token| {
            token.line == 1
                && token.char == 4
                && legend[usize::try_from(token.ttype).expect("token type index")] == "function"
        }),
        "the inner `set` must be tokenised through the pack's Body role: {tokens:?}"
    );
    assert!(
        tokens.iter().any(|token| {
            token.line == 0
                && token.char == 16
                && legend[usize::try_from(token.ttype).expect("token type index")] == "variable"
        }),
        "the `counter` argument must use the pack's VarWrite role: {tokens:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack-defined environment must retain its argument roles after edits,
/// whether selected by directive or the pack's extension (#2181).
#[test]
fn pack_environment_roles_survive_document_edits() {
    let root = workspace("environment-edit-roles");
    write(
        &root.join(".tcl-lsp/bugrepro.tclspec"),
        "speclib bugrepro 2.0 {\n  environment bugrepro {\n    display_name {Bug Repro}\n    core tcl 9.0\n    file_extension brp -name {Bug repro file}\n  }\n  command myblock {\n    arity 1\n    arg 0 -role Body\n  }\n  command other {\n    arity 1\n    arg 0 -role Expr\n  }\n}\n",
    );
    let body = "myblock {\n    myblock {\n        set x 1\n    }\n    other {$x > 1}\n}\n";
    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "bugrepro");
    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();

    for (name, directive) in [
        ("directive.tcl", true),
        ("extension.brp", false),
        ("control.tcl", false),
    ] {
        let source = if directive {
            format!("# tcl-dialect: bugrepro\n{body}")
        } else {
            body.to_owned()
        };
        let doc = root.join(name);
        write(&doc, &source);
        let uri = file_uri(&doc);
        let line = if directive { 3 } else { 2 };
        let mut diagnostics = lsp.open_ready(&uri, &source);
        let dialect = lsp.effective_config(&uri)["dialect"].clone();
        if name == "control.tcl" {
            assert_ne!(dialect, "bugrepro", "control must use a shipped dialect");
        } else {
            assert_eq!(dialect, "bugrepro", "pack dialect must route {name}");
        }
        for version in [1, 2] {
            if version == 2 {
                lsp.replace_document(&uri, 2, &format!("{source}\n"));
                diagnostics = lsp.await_diagnostics_version(
                    &uri,
                    Some(2),
                    std::time::Duration::from_secs(30),
                );
            }
            assert!(
                !diagnostics.iter().any(|diagnostic| {
                    diagnostic["code"] == "W123"
                        && diagnostic["message"].as_str().is_some_and(|message| {
                            message.contains("myblock") || message.contains("other")
                        })
                }),
                "pack commands became unknown for {name} version {version}: {diagnostics:#?}"
            );
            let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
            assert!(
                tokens.iter().any(|token| token.line == line
                    && token.char == 8
                    && token.length == 3
                    && legend[usize::try_from(token.ttype).unwrap()] == "function"),
                "nested set lost its function token for {name} version {version}: {tokens:?}"
            );
        }
        lsp.close_document(&uri);
        lsp.open_document_lang(&uri, &source, "tcl", 3);
        let reopened =
            lsp.await_diagnostics_version(&uri, Some(3), std::time::Duration::from_secs(30));
        assert!(
            !reopened.iter().any(|diagnostic| {
                diagnostic["code"] == "W123"
                    && diagnostic["message"].as_str().is_some_and(|message| {
                        message.contains("myblock") || message.contains("other")
                    })
            }),
            "pack commands became unknown when {name} reopened: {reopened:#?}"
        );
        let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
        assert!(
            tokens.iter().any(|token| token.line == line
                && token.char == 8
                && token.length == 3
                && legend[usize::try_from(token.ttype).unwrap()] == "function"),
            "nested set lost its function token when {name} reopened: {tokens:?}"
        );
        lsp.close_document(&uri);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The same command, with **no** pack in the workspace, is unknown — so the
/// test above is measuring the pack and not some pre-existing leniency.
#[test]
fn without_a_pack_the_same_command_is_unknown() {
    let root = workspace("no-pack");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        !hover.contains("Run a script with a caller variable bound."),
        "no pack, no pack hover; got:\n{hover}"
    );
    // "No packs" means no *workspace* pack: the bundled tier (the shipped EDA
    // loadables) is always there, and is what this control is controlling for.
    assert!(
        !pack_names(&mut lsp).iter().any(|name| name == "mylib"),
        "and the server reports no `mylib` pack at all; got {:?}",
        pack_names(&mut lsp)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.specPacks` reaches a pack that convention would never find — a
/// directory that is neither `.tcl-lsp/` nor beside a `tclpkg.tcl`.
#[test]
fn the_spec_packs_setting_reaches_a_pack_convention_would_miss() {
    let root = workspace("setting");
    write(&root.join("vendor/specs/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({
            "features": { "linkedEditingRange": true },
            "specPacks": ["vendor/specs"],
        }),
        &root,
    );
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the configured directory was discovered; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack beside a `tclpkg.tcl` manifest is found too — the rule that lets a
/// library ship its own specs next to its code.
#[test]
fn a_pack_beside_a_package_manifest_is_discovered() {
    let root = workspace("manifest");
    write(&root.join("lib/tclpkg.tcl"), "package require Tcl 8.6\n");
    write(&root.join("lib/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "found beside the manifest; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Two files naming one `speclib` merge into one pack, and both commands are
/// live — the "a pack is a logical unit, not a file" rule, observed from the
/// editor rather than from the loader's unit tests.
#[test]
fn a_multi_file_pack_merges_and_both_commands_resolve() {
    let root = workspace("multifile");
    write(
        &root.join(".tcl-lsp/alpha.tclspec"),
        "speclib mylib 1 {\n  command mylib::alpha {\n    arity 1\n    \
         hover { summary {The alpha command.} }\n  }\n}\n",
    );
    write(
        &root.join(".tcl-lsp/beta.tclspec"),
        "speclib mylib 1 {\n  command mylib::beta {\n    arity 1\n    \
         hover { summary {The beta command.} }\n  }\n}\n",
    );
    let source = "mylib::alpha one\nmylib::beta two\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, source);

    assert!(hover_text(&lsp.hover(&uri, 0, 4)).contains("The alpha command."));
    assert!(hover_text(&lsp.hover(&uri, 1, 4)).contains("The beta command."));

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that redeclares a shipped command does not change it — and says so
/// on the pack file. The one guarantee that keeps a private pack from quietly
/// redefining the standard library.
#[test]
fn a_pack_cannot_silently_redefine_a_shipped_command() {
    let root = workspace("collision");
    let pack = root.join(".tcl-lsp/loud.tclspec");
    write(
        &pack,
        "speclib loud 1 {\n  command lsort {\n    arity 99\n    \
         hover { summary {Definitely not lsort.} }\n  }\n}\n",
    );
    let source = "lsort {c a b}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    let diagnostics = lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 1));
    assert!(
        !hover.contains("Definitely not lsort."),
        "the shipped lsort wins; got:\n{hover}"
    );
    assert!(
        hover.contains("lsort"),
        "and the shipped hover is what is shown; got:\n{hover}"
    );
    // The pack claimed `arity 99`; if it had won, this perfectly ordinary
    // `lsort {c a b}` would be an arity error.
    let arity_complaints: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("99"))
        })
        .collect();
    assert!(
        arity_complaints.is_empty(),
        "the shipped arity is what the call is checked against: {arity_complaints:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A workspace pack command owned by a required package publishes the same
/// subcommand arity error over the protocol that `tcl lint` reports (#2242).
#[test]
fn a_workspace_pack_subcommand_arity_error_is_published() {
    let root = workspace("pack-arity");
    write(
        &root.join(".tcl-lsp/cad.tclspec"),
        "speclib cad 2.0 {\n  default dialects all-tcl\n  default required_package cad\n  \
         command cad {\n    arity 1..\n    arg 0 -role Keyword\n    \
         allow_unknown_subcommands no\n    subcommand box {\n      arity 3\n      \
         arg 0 -role Value -type Double\n      arg 1 -role Value -type Double\n      \
         arg 2 -role Value -type Double\n      synopsis {cad box x y z}\n    }\n  }\n}\n",
    );
    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "cad");
    for (name, directive) in [("default.tcl", ""), ("tcl9.tcl", "# tcl-dialect: tcl9.0\n")] {
        let source = format!(
            "{directive}package require cad 0.4\n\ncad box 10 20\ncad box 1 2 3\nstring length one two\n"
        );
        let doc = root.join(name);
        write(&doc, &source);
        let uri = file_uri(&doc);
        let diagnostics = lsp.open_ready(&uri, &source);
        if !directive.is_empty() {
            assert_eq!(lsp.effective_config(&uri)["dialect"], "tcl9.0");
        }
        let offset = i64::from(!directive.is_empty());
        let arity: Vec<&Value> = diagnostics
            .iter()
            .filter(|d| {
                d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("'cad box'"))
            })
            .collect();
        assert_eq!(arity.len(), 1, "{name}: {diagnostics:#?}");
        assert_eq!(arity[0]["range"]["start"]["line"], 2 + offset, "{arity:#?}");
        assert_eq!(arity[0]["code"], "E002", "{arity:#?}");
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic["code"] == "E003"
                    && diagnostic["range"]["start"]["line"] == 4 + offset
                    && diagnostic["message"]
                        .as_str()
                        .is_some_and(|message| message.contains("string length"))
            }),
            "built-in arity control must be published for {name}: {diagnostics:#?}"
        );
        let repaired = format!(
            "{directive}package require cad 0.4\n\ncad box 10 20 30\ncad box 1 2 3\nstring length one\n"
        );
        lsp.replace_document(&uri, 2, &repaired);
        let repaired_diagnostics =
            lsp.await_diagnostics_version(&uri, Some(2), std::time::Duration::from_secs(30));
        assert!(
            !repaired_diagnostics
                .iter()
                .any(|diagnostic| { diagnostic["code"] == "E002" || diagnostic["code"] == "E003" }),
            "repair must clear both errors for {name}: {repaired_diagnostics:#?}"
        );
        lsp.close_document(&uri);
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack keyword-definer whose implementation is an ordinary workspace proc.
const KEYWORD_DEFINER_PACK: &str = r"
speclib definer 1.0 {
    command MyDef {
        arity 1..3
        traits {LANGUAGE_KEYWORD DEFINES_PROCEDURE}
        defines_command_at 0
        arg 0 -role Name
        arg 1 -role ParamList
        arg 2 -role Body
    }
}
";

/// #2532: a workspace procedure implementing a pack command must not replace
/// the pack's spec once indexing publishes it to other documents — the head
/// stays a keyword and the body keeps its `Body` role, before and after.
#[test]
fn a_workspace_proc_implementing_a_pack_command_keeps_the_pack_spec() {
    let root = workspace("pack-implementation");
    write(&root.join(".tcl-lsp/definer.tclspec"), KEYWORD_DEFINER_PACK);
    let library = "proc MyDef {name params body} {}\n";
    write(&root.join("lib.tcl"), library);
    let source = "MyDef foo {a} { puts $a }\n";
    let doc = root.join("use.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "definer");
    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let assert_pack_tokens = |lsp: &mut Lsp, when: &str| {
        let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
        let kind_at = |char: i64| {
            tokens
                .iter()
                .find(|token| token.line == 0 && token.char == char)
                .map(|token| legend[usize::try_from(token.ttype).unwrap()].as_str())
        };
        assert_eq!(kind_at(0), Some("keyword"), "{when}: {tokens:?}");
        assert_eq!(kind_at(16), Some("function"), "{when}: {tokens:?}");
    };
    assert_pack_tokens(&mut lsp, "before the library is open");
    lsp.open_ready(&file_uri(&root.join("lib.tcl")), library);
    assert_pack_tokens(&mut lsp, "after the library is indexed");

    let _ = std::fs::remove_dir_all(&root);
}
