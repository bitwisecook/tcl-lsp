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

//! A pack's Tcl hook bodies: constant folding in the optimiser, workspace trust, and the new answer after a reload.

use super::*;

/// A pack whose `const_fold` is a **Tcl body**, folding a real call site in the
/// running server.
///
/// This is the whole hook feature in one file: the loader carries the body,
/// `tcl_spectcl::hooks` binds it to a registry slot, the worker thread that
/// runs the optimiser builds a sandboxed VM host for it, and the optimiser —
/// which knows nothing about packs — reads `spec.const_fold` exactly as it
/// reads a shipped one.
const FOLDER_PACK: &str = r"
speclib folder 1 {
    command folder::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set subject [lindex $words 0]
            if {![string is ascii $subject]} { return }
            fold [string length $subject]
        }
    }
}
";

/// A call site the optimiser will fold if — and only if — the pack's body runs.
const FOLDER_SOURCE: &str = "proc ::show {} {\n    puts [folder::strlen abcde]\n}\n::show\n";

/// The `source` field of an `optimiseDocument` reply.
fn optimised(result: &Value) -> String {
    result
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Every optimisation offer of `code` in an `optimiseDocument` reply, as
/// `(replacement, message)`.
pub(super) fn offers(result: &Value, code: &str) -> Vec<(String, String)> {
    result
        .get("optimisations")
        .and_then(Value::as_array)
        .map_or_default(|items| {
            items
                .iter()
                .filter(|item| item.get("code").and_then(Value::as_str) == Some(code))
                .map(|item| {
                    (
                        item.get("replacement")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        item.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    )
                })
                .collect()
        })
}

#[test]
fn a_pack_const_fold_body_folds_a_call_site_in_the_optimiser() {
    let root = workspace("const-fold");
    write(&root.join(".tcl-lsp/folder.tclspec"), FOLDER_PACK);
    let doc = root.join("app.tcl");
    write(&doc, FOLDER_SOURCE);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, FOLDER_SOURCE);
    await_pack_named(&mut lsp, "folder");

    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(!result.is_null(), "the optimiser answered");
    let folded = offers(&result, "O129");
    assert!(
        folded.iter().any(|(replacement, _)| replacement == "5"),
        "the pack's Tcl body computed `string length abcde`; offers were {folded:#?}\n\
         optimised source:\n{}",
        optimised(&result)
    );
    assert!(
        optimised(&result).contains("puts 5"),
        "and the rewrite applies it; got:\n{}",
        optimised(&result)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The control: the identical document with **no** pack in the workspace does
/// not fold, so the test above is measuring the hook body and not some
/// pre-existing constant folder.
#[test]
fn without_the_pack_the_same_call_site_does_not_fold() {
    let root = workspace("const-fold-none");
    let doc = root.join("app.tcl");
    write(&doc, FOLDER_SOURCE);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, FOLDER_SOURCE);
    assert!(
        !pack_names(&mut lsp).iter().any(|name| name == "folder"),
        "there is no workspace pack here"
    );

    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(
        offers(&result, "O129")
            .iter()
            .all(|(replacement, _)| replacement != "5"),
        "nothing folds an unknown command: {result:#?}"
    );
    assert!(
        !optimised(&result).contains("puts 5"),
        "and the source is untouched; got:\n{}",
        optimised(&result)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A server rooted at `root` whose client reports the workspace untrusted,
/// as VS Code does for a folder the user has not trusted: the `initialize`
/// handshake carries `initializationOptions.workspaceTrust`.
fn untrusted_at_root(root: &Path) -> Lsp {
    let mut lsp = Lsp::spawn(json!({ "features": { "linkedEditingRange": true } }));
    let root_uri = file_uri(root);
    lsp.request(
        "initialize",
        json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "workspaceFolders": [{ "uri": root_uri, "name": "e2e" }],
            "capabilities": {},
            "clientInfo": { "name": "tcl-lsp-e2e", "version": "1.0" },
            "initializationOptions": { "workspaceTrust": "untrusted" },
        }),
    );
    lsp.notify("initialized", json!({}));
    lsp
}

/// The dormant-hook notices among a pack file's diagnostics.
fn dormant_notices(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("is dormant"))
        })
        .collect()
}

/// In a workspace the editor has not trusted, a pack's `const_fold` body does
/// not run: the call site does not fold, and the body's row carries an
/// information notice saying why. Granting trust — the client's top-level
/// `workspaceTrust` push — reloads the packs with no file moved: the notice
/// clears and the same call site folds.
#[test]
fn granting_trust_reloads_and_installs_the_bodies() {
    let root = workspace("trust-grant");
    let pack = root.join(".tcl-lsp/folder.tclspec");
    write(&pack, FOLDER_PACK);
    let doc = root.join("app.tcl");
    write(&doc, FOLDER_SOURCE);

    let mut lsp = untrusted_at_root(&root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, FOLDER_SOURCE);
    await_pack_named(&mut lsp, "folder");

    let pack_uri = file_uri(&pack);
    let diagnostics =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(15), |diags| {
            !dormant_notices(diags).is_empty()
        });
    let [notice] = dormant_notices(&diagnostics)[..] else {
        panic!("one body, one notice: {diagnostics:#?}");
    };
    assert_eq!(
        notice.get("severity").and_then(Value::as_i64),
        Some(3),
        "information, not a warning: {notice:#?}"
    );
    assert_eq!(
        notice
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|p| p.get("line"))
            .and_then(Value::as_i64),
        Some(5),
        "on the `const_fold` row (1-based line 6): {notice:#?}"
    );
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(
        offers(&result, "O129")
            .iter()
            .all(|(replacement, _)| replacement != "5"),
        "a dormant body folds nothing: {result:#?}"
    );

    lsp.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "workspaceTrust": "trusted" } }),
    );
    let cleared =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(30), |diags| {
            dormant_notices(diags).is_empty()
        });
    assert!(cleared.is_empty(), "{cleared:#?}");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(
        offers(&result, "O129")
            .iter()
            .any(|(replacement, _)| replacement == "5"),
        "trusted, the body runs and folds; got:\n{}",
        optimised(&result)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The value-transfer page's executable example as a workspace pack
/// (`rust/tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec`, its
/// first declaration): `tenant::label NAME` answers `PREFIX` then the name,
/// from a declared implementation the bounded host runs under the analysed
/// release.
fn tenant_pack(prefix: &str) -> String {
    format!(
        "speclib tenant 2.2 {{\n\
         \x20   command tenant::label {{\n\
         \x20       arity 1\n\
         \x20       semantics {{\n\
         \x20           effects {{no_store_writes no_external_io}}\n\
         \x20           result -semantic string\n\
         \x20       }}\n\
         \x20       evaluate -implementation tenant.label.v1 -host bounded_tcl {{\n\
         \x20           inputs {{arg 0 exact}}\n\
         \x20           depends {{tcl_profile implementation_identity}}\n\
         \x20           budget {{-commands 2000 -wall-clock 20 -value-bytes 65536}}\n\
         \x20           body {{name}} {{ fold [string cat \"{prefix}\" $name] }}\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    )
}

/// A procedure whose condition is constant exactly because `tenant::label`
/// folds: true while the body answers `tenant:acme`, false once it answers
/// anything else.
const TENANT_SOURCE: &str = "\
proc check {} {
    if {[tenant::label acme] eq \"tenant:acme\"} {
        return yes
    }
    return no
}
";

/// The I230 messages in a published diagnostics set.
fn constant_conditions(diags: &[Value]) -> Vec<String> {
    diags
        .iter()
        .filter(|diag| diag.get("code").and_then(Value::as_str) == Some("I230"))
        .filter_map(|diag| diag.get("message").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

/// A pack reload after an edit yields the new answer on the memoised path.
///
/// `check`'s condition folds through the pack's declared implementation, and
/// an edit that leaves the procedure alone re-analyses the document with its
/// lattice served from the memo. The pack's body then changes on disk: the
/// reload publishes a new plan, takes the pack key and the evaluator epoch
/// the plan moved into the query database, and re-analyses the open
/// document with no edit of its own. The published condition turns from
/// always true to always false — the new body's answer, not the memoised
/// one's. A content edit moves the pack key as well, which alone re-keys the
/// lattices; the half only the epoch covers, a hook quarantined on a
/// worker, is the server's own
/// `the_pass_after_a_quarantine_takes_the_evaluator_epoch`.
#[test]
fn a_pack_reload_after_an_edit_yields_the_new_answer_on_the_memoised_path() {
    let root = workspace("evaluator-epoch");
    let pack = root.join(".tcl-lsp/tenant.tclspec");
    write(&pack, &tenant_pack("tenant:"));
    let doc = root.join("app.tcl");
    write(&doc, TENANT_SOURCE);

    let mut lsp = Lsp::with_config_at_root(json!({ "dialect": "tcl9.0" }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "tenant");
    let opened = constant_conditions(&lsp.open_ready(&uri, TENANT_SOURCE));
    assert!(
        opened.len() == 1 && opened[0].contains("is always true"),
        "the body folds `tenant:acme`, so the condition is constant: {opened:?}"
    );

    let edited = format!("{TENANT_SOURCE}check\n");
    lsp.replace_document(&uri, 2, &edited);
    let after_edit = constant_conditions(&lsp.await_diagnostics_version(
        &uri,
        Some(2),
        std::time::Duration::from_secs(30),
    ));
    assert!(
        after_edit.len() == 1 && after_edit[0].contains("is always true"),
        "the edit leaves `check` and its memoised lattice alone: {after_edit:?}"
    );

    write(&pack, &tenant_pack("org:"));
    notify_pack_changed(&mut lsp, &pack, CHANGED);
    let reloaded = constant_conditions(&lsp.await_diagnostics_settled(
        &uri,
        std::time::Duration::from_secs(30),
        |diags| {
            constant_conditions(diags)
                .iter()
                .any(|message| message.contains("is always false"))
        },
    ));
    assert_eq!(
        reloaded.len(),
        1,
        "one condition, answered by the reloaded body: {reloaded:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
