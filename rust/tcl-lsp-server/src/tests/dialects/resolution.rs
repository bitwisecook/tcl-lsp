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

//! The dialect an open or closed document resolves to.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn environment_kind_setting_reaches_the_notice() {
    let backend = test_backend();
    assert!(backend.environment_notice.enabled(), "on unless set off");
    backend
        .apply_global_config(&serde_json::json!({ "notifications": { "environmentKind": false } }))
        .await
        .unwrap();
    assert!(!backend.environment_notice.enabled());
    // A pull that does not carry the key leaves the last value in force.
    backend
        .apply_global_config(&serde_json::json!({}))
        .await
        .unwrap();
    assert!(!backend.environment_notice.enabled());
    backend
        .apply_global_config(&serde_json::json!({ "notifications": { "environmentKind": true } }))
        .await
        .unwrap();
    assert!(backend.environment_notice.enabled());
}

/// A configured session dialect that resolves is kept as written and marks
/// the dialect explicit; one that names no environment is not stored, the
/// session keeps the built-in default, and the warning names the value and
/// the selectable dialects. A blank value is an unset setting: the default
/// stays and nothing is warned. A rejected value is warned about once while
/// it stays configured, and again if it returns after a good or blank one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_dialect_naming_no_environment_falls_back_with_a_warning() {
    let backend = test_backend();
    assert_eq!(
        backend.apply_configured_session_dialect("irules").await,
        None
    );
    assert_eq!(backend.session_dialect().await, "irules");
    assert!(*backend.default_dialect_explicit.lock().await);

    let warning = backend
        .apply_configured_session_dialect("nonsense")
        .await
        .expect("an unknown dialect is reported");
    assert!(warning.contains("`nonsense`"), "{warning}");
    for name in ["jim", "tk", "xilinx-eda-tcl", "tcl9.1"] {
        assert!(warning.contains(name), "{name}: {warning}");
    }
    assert_eq!(backend.session_dialect().await, DEFAULT_SESSION_DIALECT);
    assert!(!*backend.default_dialect_explicit.lock().await);

    assert_eq!(
        backend.apply_configured_session_dialect("nonsense").await,
        None,
        "the same rejected value is not reported on every pull"
    );
    assert_eq!(backend.session_dialect().await, DEFAULT_SESSION_DIALECT);
    assert!(
        backend
            .apply_configured_session_dialect("other-nonsense")
            .await
            .is_some_and(|message| message.contains("`other-nonsense`")),
        "a different rejected value is reported"
    );

    for blank in ["", "  ", "\t"] {
        assert_eq!(
            backend.apply_configured_session_dialect("irules").await,
            None
        );
        assert_eq!(
            backend.apply_configured_session_dialect(blank).await,
            None,
            "a blank value is an unset setting: {blank:?}"
        );
        assert_eq!(backend.session_dialect().await, DEFAULT_SESSION_DIALECT);
        assert!(!*backend.default_dialect_explicit.lock().await);
    }

    assert!(
        backend
            .apply_configured_session_dialect("nonsense")
            .await
            .is_some(),
        "a rejected value that returns after a blank one is reported again"
    );
}

/// A closed file's dialect is resolved from its on-disk source the
/// way `did_open` would — an in-source directive, a BIG-IP basename, and a
/// dialect-specific extension all survive the close instead of defaulting to
/// generic Tcl.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dialect_for_closed_honours_directive_basename_and_extension() {
    let backend = test_backend();
    // In-source `# tcl-dialect:` directive pins the version.
    let versioned = Uri::from_str("file:///v.tcl").unwrap();
    assert_eq!(
        backend
            .dialect_for_closed(&versioned, "# tcl-dialect: tcl8.4\nset x 1\n")
            .await,
        "tcl8.4",
    );
    // A canonical BIG-IP basename routes to the config dialect.
    let bigip = Uri::from_str("file:///bigip.conf").unwrap();
    assert_eq!(
        backend
            .dialect_for_closed(&bigip, "ltm virtual v { }\n")
            .await,
        "f5-bigip",
    );
    // A dialect-specific extension routes without any editor language id.
    let irule = Uri::from_str("file:///r.irule").unwrap();
    assert_eq!(
        backend
            .dialect_for_closed(&irule, "when HTTP_REQUEST { }\n")
            .await,
        "f5-irules",
    );
    // A plain `.tcl` with no hint falls back to the session default.
    let plain = Uri::from_str("file:///p.tcl").unwrap();
    assert_eq!(
        backend.dialect_for_closed(&plain, "set x 1\n").await,
        "tcl8.6",
    );
}

/// `initializationOptions.folderDialects` is authoritative for the folders
/// it names; every other folder takes its dialect from the pulled
/// per-folder config.
#[tokio::test]
async fn folder_dialect_overrides_prefer_initialisation_options() {
    let backend = test_backend();
    let pinned = Uri::from_str("file:///ws/pinned").unwrap();
    let pulled = Uri::from_str("file:///ws/pulled").unwrap();
    *backend.folder_dialects.lock().await = vec![(pinned.clone(), "tcl9.0".to_owned())];
    *backend.folder_configs.lock().await = vec![
        (
            pinned.clone(),
            FolderConfig {
                dialect: Some("tcl8.4".to_owned()),
                ..FolderConfig::default()
            },
        ),
        (
            pulled.clone(),
            FolderConfig {
                dialect: Some("f5-irules".to_owned()),
                ..FolderConfig::default()
            },
        ),
    ];
    let merged = backend.folder_dialect_overrides().await;
    let pinned_doc = Uri::from_str("file:///ws/pinned/a.tcl").unwrap();
    let pulled_doc = Uri::from_str("file:///ws/pulled/a.tcl").unwrap();
    assert_eq!(
        folder_dialect_for(&pinned_doc, &merged).as_deref(),
        Some("tcl9.0"),
        "the host-supplied mapping wins for the folder it names",
    );
    assert_eq!(
        folder_dialect_for(&pulled_doc, &merged).as_deref(),
        Some("f5-irules"),
        "an unnamed folder takes its pulled `tclLsp.dialect`",
    );
}

#[tokio::test]
async fn resolve_folder_dialect_picks_deepest_prefix() {
    let backend = test_backend();
    *backend.folder_dialects.lock().await = vec![
        (
            Uri::from_str("file:///workspace/").unwrap(),
            "tcl9.0".to_owned(),
        ),
        (
            Uri::from_str("file:///workspace/irules/").unwrap(),
            "f5-irules".to_owned(),
        ),
    ];
    let inside = Uri::from_str("file:///workspace/irules/rule.tcl").expect("parse target uri");
    assert_eq!(
        backend.resolve_folder_dialect(&inside).await,
        Some("f5-irules".to_owned()),
    );
    let outside_irules = Uri::from_str("file:///workspace/main.tcl").expect("parse target uri");
    assert_eq!(
        backend.resolve_folder_dialect(&outside_irules).await,
        Some("tcl9.0".to_owned()),
    );
    let unrelated = Uri::from_str("file:///elsewhere/x.tcl").unwrap();
    assert_eq!(backend.resolve_folder_dialect(&unrelated).await, None);
}

#[tokio::test]
async fn resolve_folder_dialect_respects_directory_boundary() {
    // A prefix-only
    // match would incorrectly select `file:///workspace/app`'s
    // dialect for a document inside `file:///workspace/app2/`.
    let backend = test_backend();
    *backend.folder_dialects.lock().await = vec![
        (
            Uri::from_str("file:///workspace/app").unwrap(),
            "f5-irules".to_owned(),
        ),
        (
            Uri::from_str("file:///workspace/app2/").unwrap(),
            "tcl9.0".to_owned(),
        ),
    ];
    let sibling = Uri::from_str("file:///workspace/app2/main.tcl").unwrap();
    assert_eq!(
        backend.resolve_folder_dialect(&sibling).await,
        Some("tcl9.0".to_owned()),
        "sibling folder must not inherit prefix-matched dialect",
    );
    let inside_app = Uri::from_str("file:///workspace/app/inner.tcl").unwrap();
    assert_eq!(
        backend.resolve_folder_dialect(&inside_app).await,
        Some("f5-irules".to_owned()),
    );
}

#[test]
fn folder_dialect_for_prefers_deepest_match() {
    let folders = vec![
        (Uri::from_str("file:///ws/").unwrap(), "tcl8.6".to_owned()),
        (
            Uri::from_str("file:///ws/irules/").unwrap(),
            "f5-irules".to_owned(),
        ),
    ];
    // A file under the nested folder picks up the nested
    // (deepest-prefix) dialect, not the parent's.
    let nested = Uri::from_str("file:///ws/irules/app.tcl").unwrap();
    assert_eq!(
        folder_dialect_for(&nested, &folders).as_deref(),
        Some("f5-irules"),
    );
    // A file only under the root folder gets the root dialect.
    let top = Uri::from_str("file:///ws/util.tcl").unwrap();
    assert_eq!(
        folder_dialect_for(&top, &folders).as_deref(),
        Some("tcl8.6"),
    );
    // A file outside every folder has no override.
    let outside = Uri::from_str("file:///other/x.tcl").unwrap();
    assert_eq!(folder_dialect_for(&outside, &folders), None);
}

#[tokio::test]
async fn dialect_for_open_falls_back_to_folder_dialect() {
    let backend = test_backend();
    *backend.folder_dialects.lock().await = vec![(
        Uri::from_str("file:///workspace/").unwrap(),
        "f5-irules".to_owned(),
    )];
    let doc = Uri::from_str("file:///workspace/main.tcl").unwrap();
    // The bare `tcl` language id names no specific version (editors send it
    // for every `.tcl` file), so the folder override *is* consulted — it is
    // the more specific signal.
    assert_eq!(
        backend.dialect_for_open(&doc, "tcl", "").await,
        "f5-irules".to_owned(),
    );
    // An unknown language id (`plaintext`) lets the folder
    // override take effect.
    assert_eq!(
        backend.dialect_for_open(&doc, "plaintext", "").await,
        "f5-irules".to_owned(),
    );
    // An explicit *versioned* language id is a deliberate choice and still
    // wins over the folder override.
    assert_eq!(
        backend.dialect_for_open(&doc, "tcl9.0", "").await,
        "tcl9.0".to_owned(),
    );
}

/// Opening a `.tclspec` activates the `SpecTcl` pack.
///
/// Every editor integration registers `.tclspec` under the plain `tcl`
/// language id (the extension is the registration `spec-packs.md` asks
/// for), so the routing that has to work is the bare-`tcl` path: the
/// shared detector reads the extension — and, for a pack saved under
/// another name, the `speclib` content signature — and answers `spectcl`.
/// The canonical name and the editor spellings are accepted directly too,
/// for MCP / direct callers and any integration that contributes its own
/// language id.
#[tokio::test]
async fn dialect_for_open_routes_tclspec_to_the_spectcl_dialect() {
    let backend = test_backend();
    // A folder override is in place precisely to prove the file's own
    // identity beats it: a `.tclspec` under an iRules folder is still a
    // spec pack.
    *backend.folder_dialects.lock().await = vec![(
        Uri::from_str("file:///workspace/").unwrap(),
        "f5-irules".to_owned(),
    )];
    let pack = Uri::from_str("file:///workspace/mylib.tclspec").unwrap();
    assert_eq!(
        backend
            .dialect_for_open(&pack, "tcl", "command foo { arity 1 }\n")
            .await,
        "spectcl".to_owned(),
        "the `.tclspec` extension selects the SpecTcl dialect",
    );
    // The `speclib` directive is a content signature, so a pack saved
    // under a `.tcl` name routes too.
    let misnamed = Uri::from_str("file:///workspace/mylib.tcl").unwrap();
    assert_eq!(
        backend
            .dialect_for_open(&misnamed, "tcl", "speclib mylib 1.0 {\n}\n")
            .await,
        "spectcl".to_owned(),
    );
    // An explicit language id — canonical or editor spelling — routes
    // directly, without consulting the document at all.
    for id in ["spectcl", "tclspec", "tcl-spec"] {
        assert_eq!(
            backend.dialect_for_open(&pack, id, "").await,
            "spectcl".to_owned(),
            "language id {id:?} names the SpecTcl dialect",
        );
    }
    // …and the registry that dialect resolves to really carries the pack.
    let reg = tcl_registry::model::ingress::static_context_for("spectcl").commands();
    assert!(reg.get("speclib").is_some(), "the SpecTcl pack is loaded");
    assert!(reg.get("set").is_some(), "core Tcl stays underneath it");
}

/// Opening a `.sslictcl` activates the `SslicTcl` vocabulary.
///
/// The `SpecTcl` test above, for the sibling declarative dialect: the
/// `.sslictcl` extension routes under the bare `tcl` language id every
/// editor sends, the mandatory `sslictcl VERSION` header is the content
/// signature that recognises a document saved under a `.tcl` name, and the
/// canonical name — which is also the editor language id — routes
/// directly.
#[tokio::test]
async fn dialect_for_open_routes_sslictcl_to_the_sslictcl_dialect() {
    let backend = test_backend();
    // As above: a folder override proves the file's own identity wins.
    *backend.folder_dialects.lock().await = vec![(
        Uri::from_str("file:///workspace/").unwrap(),
        "f5-irules".to_owned(),
    )];
    let doc = Uri::from_str("file:///workspace/site.sslictcl").unwrap();
    assert_eq!(
        backend
            .dialect_for_open(&doc, "tcl", "sslictcl 1\nendpoint /Common/www {\n}\n")
            .await,
        "sslictcl".to_owned(),
        "the `.sslictcl` extension selects the SslicTcl dialect",
    );
    let misnamed = Uri::from_str("file:///workspace/site.tcl").unwrap();
    assert_eq!(
        backend
            .dialect_for_open(&misnamed, "tcl", "sslictcl 1\n")
            .await,
        "sslictcl".to_owned(),
        "the `sslictcl VERSION` header is the content signature",
    );
    for id in ["sslictcl"] {
        assert_eq!(
            backend.dialect_for_open(&doc, id, "").await,
            "sslictcl".to_owned(),
            "language id {id:?} names the SslicTcl dialect",
        );
    }
    // …and the registry that dialect resolves to really carries the pack.
    let reg = tcl_registry::model::ingress::static_context_for("sslictcl").commands();
    assert!(reg.get("endpoint").is_some(), "the SslicTcl pack is loaded");
    assert!(reg.get("set").is_some(), "core Tcl stays underneath it");
}

/// A `dialect =` key in `config.ini` / `.tcl-lsp.ini` sets the session
/// `default_dialect`; a normally-opened `.tcl` buffer (language id `"tcl"`,
/// which every editor sends) must resolve to it rather than pinning
/// `tcl8.6`.
#[tokio::test]
async fn dialect_for_open_respects_config_default_dialect() {
    let backend = test_backend();
    *backend.default_dialect.lock().await = "tcl9.0".to_owned();
    let doc = Uri::from_str("file:///workspace/main.tcl").unwrap();
    assert_eq!(
        backend.dialect_for_open(&doc, "tcl", "").await,
        "tcl9.0".to_owned(),
    );
    // An in-source dialect directive is still the most specific signal and
    // overrides the configured default.
    assert_eq!(
        backend
            .dialect_for_open(&doc, "tcl", "# tcl-dialect: tcl8.4\n")
            .await,
        "tcl8.4".to_owned(),
    );
}

/// An explicit BIG-IP language id resolves to `f5-bigip` even when the
/// document basename is not a canonical `bigip*.conf` name — the
/// manual-language-mode case the `is_bigip_conf_uri` basename branch
/// alone would miss.
#[tokio::test]
async fn dialect_for_open_maps_bigip_language_id() {
    let backend = test_backend();
    let doc = Uri::from_str("file:///workspace/device_config.txt").unwrap();
    assert_eq!(
        backend.dialect_for_open(&doc, "tcl-bigip", "").await,
        "f5-bigip".to_owned(),
    );
    assert_eq!(
        backend.dialect_for_open(&doc, "f5-bigip", "").await,
        "f5-bigip".to_owned(),
    );
    // A generic Tcl language id on the same non-conf name does not get the
    // BIG-IP dialect (no basename match, no explicit BIG-IP id).
    assert_ne!(
        backend.dialect_for_open(&doc, "tcl", "").await,
        "f5-bigip".to_owned(),
    );
}

/// The basename route reads the URI's decoded last segment: a
/// percent-encoded canonical name is that name, and a literal backslash in
/// a POSIX name is part of the name rather than a separator.
#[tokio::test]
async fn dialect_for_open_reads_the_decoded_uri_basename() {
    let backend = test_backend();
    for raw in [
        "file:///workspace/bigip%2Econf",
        "file:///my%20dir/BIGIP_BASE.CONF",
    ] {
        let doc = Uri::from_str(raw).unwrap();
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl", "").await,
            "f5-bigip".to_owned(),
            "{raw}",
        );
    }
    let doc = Uri::from_str("file:///workspace/a%5Cbigip.conf").unwrap();
    assert_ne!(
        backend.dialect_for_open(&doc, "tcl", "").await,
        "f5-bigip".to_owned(),
    );
}

/// A session-default dialect change (as `tclLsp.selectDialect` pushes via
/// `workspace/didChangeConfiguration`) re-resolves already-open documents:
/// one that fell back to the default moves to the new default, while one
/// opened with an explicit language id keeps its dialect.
#[tokio::test]
async fn dialect_config_change_reresolves_open_documents() {
    let backend = test_backend();
    *backend.default_dialect.lock().await = "tcl8.6".to_owned();

    let plain = Uri::from_str("file:///plain.tcl").unwrap();
    let irule = Uri::from_str("file:///app.irule").unwrap();
    {
        let mut docs = backend.documents.lock("test").await;
        // No recognised language id → opened under the session default.
        docs.insert(
            plain.clone(),
            DocumentState::new("set x 1\n".to_owned(), "tcl8.6".to_owned())
                .with_language_id("plaintext".to_owned()),
        );
        // An explicit iRules language id → resolved independently of the
        // default.
        docs.insert(
            irule.clone(),
            DocumentState::new("when HTTP_REQUEST {}\n".to_owned(), "f5-irules".to_owned())
                .with_language_id("tcl-irule".to_owned()),
        );
    }

    // The user switches the session dialect to tcl9.0.
    *backend.default_dialect.lock().await = "tcl9.0".to_owned();
    backend.reresolve_open_document_dialects().await.unwrap();

    let docs = backend.documents.lock("test").await;
    assert_eq!(
        docs.get(&plain).unwrap().dialect,
        "tcl9.0",
        "a default-resolved document should follow the new session default",
    );
    assert_eq!(
        docs.get(&irule).unwrap().dialect,
        "f5-irules",
        "an explicit-language-id document keeps its dialect",
    );
}

#[test]
fn bigip_routing_uses_the_profile_availability_fact() {
    assert!(Backend::is_bigip_dialect("f5-bigip"));
    assert!(!Backend::is_bigip_dialect("f5-irules"));
    assert!(!Backend::is_bigip_dialect("not-a-real-dialect"));
}
