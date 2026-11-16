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

//! The file extensions and environments a pack claims: advertised, indexed, retired with the pack and routed through the ingress.

use super::*;

/// A pack declaring a file extension nothing in the shipped catalogue owns,
/// alongside one that declares no dialect at all.
const EXTENSION_PACK: &str = r"
speclib extlib 1 {
    file_extension irulex -name {Extension Pack Rules} -dialect f5-irules
    file_extension packplain -name {Plain Pack Script}

    command extlib::noop {
        arity 1
    }
}
";

/// The server advertises the extensions its discovered packs claim, each
/// resolved to an editor language id a client can actually associate with.
///
/// The server half of lazy registration already exists — pack routing is
/// consulted ahead of the static catalogue by `dialect_from_extension` — but
/// an editor learns associations from a manifest written long before the
/// user's pack existed, so it has to be *told*. This is that channel.
#[test]
fn the_server_advertises_the_extensions_its_packs_claim() {
    let root = workspace("extensions");
    write(&root.join(".tcl-lsp/extlib.tclspec"), EXTENSION_PACK);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "extlib");

    let advertised = lsp
        .effective_config("")
        .get("pack_file_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = |ext: &str| -> Option<Value> {
        advertised
            .iter()
            .find(|row| row.get("extension").and_then(Value::as_str) == Some(ext))
            .cloned()
    };

    // A row with a `-dialect` lands on that dialect's own editor language —
    // no new language id can be created at runtime, so this is the whole
    // range of choice a client has.
    let irulex = row("irulex").expect("the pack's `irulex` row must be advertised");
    assert_eq!(irulex["dialect"], json!("f5-irules"));
    assert_eq!(irulex["language_id"], json!("tcl-irule"));
    assert_eq!(irulex["pack"], json!("extlib"));

    // A row with no `-dialect` rides plain `tcl`; the server's own detection
    // decides the dialect once the file is open.
    let plain = row("packplain").expect("a dialect-less row must still be advertised");
    assert_eq!(plain["dialect"], Value::Null);
    assert_eq!(plain["language_id"], json!("tcl"));

    // NEGATIVE control: everything the shipped catalogue owns is already
    // registered statically by every editor, so it is never advertised —
    // otherwise a client could not tell its own dynamic entries from the
    // static ones when a pack goes away.
    for statically_owned in ["irule", "tmsh", "sdc", "upf"] {
        assert!(
            row(statically_owned).is_none(),
            "catalogue-owned .{statically_owned} must not be advertised: {advertised:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack-claimed extension has to reach the index and the open document.
///
/// The distinction is the whole finding. Opening a `.irulex` file always
/// worked — `dialect_from_extension` consults pack routing, so the document
/// analyses correctly. But every predicate that decides which files the
/// server reaches on its own read the static `TCL_SOURCE_EXTENSIONS`, so a
/// **closed** `.irulex` file was invisible: its definitions never entered the
/// workspace index, and a reference from an ordinary `.tcl` file could not
/// resolve to them.
///
/// So this asserts on cross-file resolution from a file that is never opened,
/// which is exactly what the static-only predicate could not do.
#[test]
fn a_closed_file_under_a_pack_extension_is_indexed() {
    let root = workspace("extensions-indexed");
    write(&root.join(".tcl-lsp/extlib.tclspec"), EXTENSION_PACK);
    // Never opened by this test — only ever reached by the workspace scan.
    write(
        &root.join("helpers.irulex"),
        "proc extpack_only_helper {a b} {\n    return [expr {$a + $b}]\n}\n",
    );
    let source = "extpack_only_helper 1 2\n";
    let doc = root.join("caller.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "extlib");
    lsp.open_ready(&uri, source);

    // The call resolves to a definition that lives in a file nothing opened.
    let definitions = lsp.definition(&uri, 0, 4);
    let text = serde_json::to_string(&definitions).unwrap_or_default();
    assert!(
        text.contains("helpers.irulex"),
        "a closed `.irulex` file must be indexed once a pack claims the \
         extension; definition returned: {text}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The advertisement follows the packs: retire the pack and the extension
/// stops being advertised, so a client's association is retired with it.
#[test]
fn a_removed_pack_stops_advertising_its_extension() {
    let root = workspace("extensions-removed");
    let pack = root.join(".tcl-lsp/extlib.tclspec");
    write(&pack, EXTENSION_PACK);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "extlib");

    let claims = |lsp: &mut Lsp| -> bool {
        lsp.effective_config("")
            .get("pack_file_extensions")
            .and_then(Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .any(|row| row.get("extension").and_then(Value::as_str) == Some("irulex"))
            })
    };
    assert!(
        claims(&mut lsp),
        "the pack's extension must start advertised"
    );

    std::fs::remove_file(&pack).expect("remove pack");
    notify_pack_changed(&mut lsp, &pack, DELETED);

    let deadline = std::time::Instant::now() + scaled_timeout(std::time::Duration::from_secs(30));
    while claims(&mut lsp) {
        assert!(
            std::time::Instant::now() < deadline,
            "the extension was still advertised after its pack was deleted"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that declares an **environment** with a file-extension detection
/// row makes documents with that extension resolve to it — through the
/// ordinary ingress, with no dialect setting, no language id, and nothing in
/// the document saying so.
///
/// This is the production wiring of the environment-registration seam end to
/// end: discovery finds the pack, the load publishes it, publishing registers
/// its `environment` block into the one live registry the ingress resolves
/// against and republishes the extension routing detection reads — so
/// `getEffectiveConfig` on a `.pshx` buffer answers with the pack's own
/// environment id and display name rather than the session default.
///
/// And the other half of the same contract: delete the pack, and the
/// environment retires. A pack-declared environment that outlived its file
/// would be a dialect the user cannot get rid of.
#[test]
fn a_pack_declared_environment_routes_its_documents_through_the_ingress() {
    const ENVIRONMENT_PACK: &str = r"
speclib envprobe 2.0 {
    environment probe-shell-tcl {
        display_name   {Probe Shell}
        core           tcl 8.6
        ambient        ProbeCmds 1.0
        alias          probe-shell
        file_extension pshx -name {Probe Shell Script}
        policy         ambient-plus-require
    }
    command probe_open {
        arity 1
        hover { summary {Open a probe shell session.} }
    }
}
";
    let root = workspace("environment");
    let pack = root.join(".tcl-lsp/envprobe.tclspec");
    write(&pack, ENVIRONMENT_PACK);
    let source = "probe_open session\n";
    let doc = root.join("session.pshx");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "envprobe");
    lsp.open_ready(&uri, source);

    let dialect = |lsp: &mut Lsp, uri: &str| -> String {
        lsp.effective_config(uri)
            .get("dialect")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    // Resolved at open, from the extension alone: the barrier above means the
    // pack — and so its environment — was live before the document opened.
    assert_eq!(dialect(&mut lsp, &uri), "probe-shell-tcl");

    // The claim is advertised to the client too, so an editor that can
    // register associations at runtime opens `.pshx` as Tcl in the first
    // place — without that, every path but the user's own works.
    let advertised = lsp
        .effective_config("")
        .get("pack_file_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = advertised
        .iter()
        .find(|row| row.get("extension").and_then(Value::as_str) == Some("pshx"))
        .unwrap_or_else(|| panic!("the environment's extension is advertised: {advertised:#?}"));
    assert_eq!(
        row.get("dialect").and_then(Value::as_str),
        Some("probe-shell-tcl")
    );
    assert_eq!(
        row.get("display_name").and_then(Value::as_str),
        Some("Probe Shell Script")
    );

    // A plain `.tcl` buffer in the same workspace is untouched — the
    // environment claims its own extension, not the workspace.
    let plain = root.join("plain.tcl");
    write(&plain, source);
    let plain_uri = file_uri(&plain);
    lsp.open_ready(&plain_uri, source);
    assert_ne!(dialect(&mut lsp, &plain_uri), "probe-shell-tcl");

    // Delete the pack: the environment retires with it, and the extension
    // stops routing.
    std::fs::remove_file(&pack).expect("remove pack");
    notify_pack_changed(&mut lsp, &pack, DELETED);
    let deadline = std::time::Instant::now() + scaled_timeout(std::time::Duration::from_secs(30));
    loop {
        // A document's dialect is resolved when it opens, so the retirement
        // is observed the way a user would see it: close the buffer and open
        // it again.
        lsp.close_document(&uri);
        lsp.open_ready(&uri, source);
        if dialect(&mut lsp, &uri) != "probe-shell-tcl" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the environment was still routing after its pack was deleted"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = std::fs::remove_dir_all(&root);
}
