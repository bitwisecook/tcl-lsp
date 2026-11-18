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

//! Pack reloads: a pack added or changed after startup re-analyses open documents, and the startup reload cannot overwrite a newer pack set.

use super::*;

/// A pack file appearing after startup is picked up from the watcher event,
/// and the open document is re-analysed against it.
#[test]
fn a_pack_added_after_startup_reloads_and_re_analyses_open_documents() {
    let root = workspace("reload");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    assert!(
        !hover_text(&lsp.hover(&uri, 0, 4)).contains("Run a script with a caller variable bound."),
        "nothing is known about the command before the pack exists"
    );
    assert!(!pack_names(&mut lsp).iter().any(|name| name == "mylib"));

    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");
    // The pack name is visible before the open document is re-analysed
    // against it, and no publication marks the moment: the pack command had
    // no unresolved-command finding to lose. Poll the state the test asserts
    // (the hover), bounded, so a reload that never reaches the document fails
    // here rather than by a race.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while hover_text(&lsp.hover(&uri, 0, 4)).is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "and hovers from the pack once it loads; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A reload asks the client to re-pull the semantic tokens it is already
/// showing.
///
/// The registry is what classifies a token: a pack decides whether a head is
/// a `function` at all, whether it carries `defaultLibrary`, and — through
/// `arg 1 -role Body` — whether a braced word is tokenised as code or as one
/// opaque string. The server recomputes correctly because the salsa inputs are
/// keyed on the pack key, but a client only re-requests tokens when the
/// document changes. Without the push the editor keeps painting the old stream
/// over new meaning until the user happens to type in that buffer — which is
/// what left a Spec Studio pack edit with the sample's colours frozen while its
/// squiggles and hovers updated.
#[test]
fn a_pack_reload_asks_the_client_to_re_pull_semantic_tokens() {
    let root = workspace("token-refresh");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    // Take the tokens first, so the server is genuinely serving a stream the
    // reload is about to invalidate rather than one nobody has asked for.
    let _ = lsp.semantic_tokens_settled(&uri);

    let cursor = lsp.server_request_cursor();
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);
    await_pack_named(&mut lsp, "mylib");

    assert!(
        lsp.try_await_server_request(
            "workspace/semanticTokens/refresh",
            std::time::Duration::from_secs(20),
            cursor,
        )
        .is_some(),
        "a pack reload that changed the command surface must push \
         workspace/semanticTokens/refresh"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Changing a pack's argument role changes an already open document's tokens.
#[test]
fn changing_a_pack_body_role_refreshes_open_document_tokens() {
    let root = workspace("changed-body-role");
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);
    let uri = file_uri(&doc);
    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
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
    let is_inner_command = |lsp: &mut Lsp| {
        decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri))
            .iter()
            .any(|token| {
                token.line == 1
                    && token.char == 4
                    && token.length == 3
                    && legend[usize::try_from(token.ttype).unwrap()] == "function"
            })
    };
    assert!(
        is_inner_command(&mut lsp),
        "Body must tokenise the nested command"
    );
    let cursor = lsp.server_request_cursor();
    write(
        &pack,
        &MYLIB_PACK.replace("arg 1 -role Body", "arg 1 -role Value"),
    );
    notify_pack_changed(&mut lsp, &pack, CHANGED);
    assert!(
        lsp.try_await_server_request(
            "workspace/semanticTokens/refresh",
            std::time::Duration::from_secs(30),
            cursor,
        )
        .is_some(),
        "changing the role must request token refresh"
    );
    assert!(
        !is_inner_command(&mut lsp),
        "Value must leave the braced data opaque"
    );

    let cursor = lsp.server_request_cursor();
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CHANGED);
    assert!(
        lsp.try_await_server_request(
            "workspace/semanticTokens/refresh",
            std::time::Duration::from_secs(30),
            cursor,
        )
        .is_some(),
        "restoring the role must request token refresh"
    );
    assert!(
        is_inner_command(&mut lsp),
        "restored Body must tokenise the nested command"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that appears **while the startup reload is still running** must not
/// be lost.
///
/// Two reloads overlap here: the one `initialized` starts, which parses the
/// bundled EDA loadables and is the slower of the two, and the one this
/// notification starts. Each takes its own filesystem snapshot, so the startup
/// reload's view predates the file. When it finished second it republished
/// that view and the workspace pack vanished — and because the file was
/// already on disk with its one event consumed, nothing ever re-triggered:
/// the pack stayed missing for the session.
///
/// This is the **fast-path smoke test** for that scenario: no settling step,
/// so the two reloads overlap if the machine lets them, and no artificial
/// timing. It is not by itself a proof — whether they actually overlap depends
/// on how long the bundled load takes on the box running it, and on a fast
/// idle machine the startup reload has already finished by the time the
/// notification arrives, so it passes either way. What was observed in CI is a
/// pack set of exactly the six bundled EDA packs: the startup snapshot,
/// verbatim.
///
/// The proof is its neighbour,
/// [`a_stale_startup_snapshot_cannot_overwrite_a_newer_pack_set`], which forces
/// the interleaving through the server's hold seam and fails without the fix.
/// This one is kept because it is the version with no seam in it: it exercises
/// the real startup path with real timings, so a refactor that moves the pack
/// load out from under the ordering guarantees still has to keep the ordinary
/// case working.
#[test]
fn a_pack_created_during_the_startup_reload_survives_it() {
    let root = workspace("startup-race");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);

    // No settling step: write and notify immediately, so the notification's
    // reload races the one still in flight from `initialized`.
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");

    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the pack that won the race is the one on disk; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// How long the server sits on the startup reload's filesystem snapshot, via
/// its `TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS` seam.
///
/// The window the notification's reload has to land in. A full reload —
/// discovery plus the bundled load — measures ~200ms on this tree, so this is
/// several times what it needs. Not load-scaled: it is a *hold*, not a
/// deadline, and it is anchored to the server's own "snapshot taken" log
/// rather than to wall-clock guesswork, so a busy machine eats into the margin
/// rather than the premise.
const STARTUP_HOLD: std::time::Duration = std::time::Duration::from_millis(1500);

/// The log line the held startup reload emits once its snapshot is fixed —
/// `STARTUP_RELOAD_HELD_LOG` in the server.
const HELD_LOG: &str = "SpecTcl: startup reload holding its snapshot";

/// The same race as the test above, but **forced** — the proof that the
/// ordering fix is load-bearing.
///
/// The server is started with the startup reload holding its snapshot for
/// [`STARTUP_HOLD`]: it discovers and loads packs, announces that its view of
/// the disk is fixed, and only then sleeps before publishing. The pack file is
/// written *after* that announcement and its watcher event sent immediately,
/// so the startup reload is provably sitting on a view that predates the file
/// while the notification's reload reads the true state. That is the
/// interleaving CI hit and that a fast idle box will not reproduce on its own
/// — waiting for the signal is what makes it a proof rather than a hope, since
/// writing the pack too early just gives the startup reload a snapshot that
/// already contains it.
///
/// Two independent protections make it come out right, and either alone
/// suffices. The reload mutex serialises the whole discover → load → publish
/// sequence, so the notification's reload waits out the hold and then re-reads
/// the disk, finding the pack. The generation stamp orders publishes by *when
/// the disk was read*, so even with the two reloads overlapping, the startup
/// one's older snapshot cannot take the place of a set published from a newer
/// one. With neither, the held snapshot publishes last and the pack disappears
/// from a session that had already reported it — which is exactly what this
/// test sees when both are removed.
///
/// Which is why awaiting the pack is not the assertion. The notification's
/// reload publishes it early; the regression is that it is taken *away* again
/// when the stale snapshot finally lands. So this outlives the hold, requiring
/// the pack to still be there, and fails on the first poll that loses it.
#[test]
fn a_stale_startup_snapshot_cannot_overwrite_a_newer_pack_set() {
    let root = workspace("startup-race-forced");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let hold_ms = STARTUP_HOLD.as_millis().to_string();
    let mut lsp = Lsp::with_config_at_root_env(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
        &[("TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS", hold_ms.as_str())],
    );

    // The startup reload has read the disk and is now sitting on that reading.
    lsp.await_log(&[HELD_LOG], std::time::Duration::from_secs(30), 0);
    let held_at = std::time::Instant::now();

    // Written strictly inside the hold, so the snapshot upstairs cannot
    // contain it — and notified at once, so the second reload starts while the
    // first is still asleep.
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");

    // Outlive the held snapshot's publish, which lands one hold after the
    // signal. The slack is load-scaled — the publish is preceded by real work
    // on both sides — while the hold itself is not.
    let window = STARTUP_HOLD + scaled_timeout(std::time::Duration::from_secs(3));
    while held_at.elapsed() < window {
        let names = pack_names(&mut lsp);
        assert!(
            names.iter().any(|loaded| loaded == "mylib"),
            "the workspace pack was published and then lost {:?} into the hold — \
             the stale startup snapshot republished over a newer set; loaded: {names:?}",
            held_at.elapsed()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // And the editor-visible consequence, sampled once the race is settled.
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the pack on disk is the one that survives the race; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
