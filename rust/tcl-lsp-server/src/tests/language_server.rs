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

//! Handlers exercised directly: text synchronisation, code lenses, selection ranges and document links.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_open_stores_document() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///open.tcl").unwrap();
    Box::pin(backend.did_open(DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version: 7,
            text: "set x 1\n".to_owned(),
        },
    }))
    .await;
    let docs = backend.documents.lock("test").await;
    let doc = docs.get(&uri).expect("did_open should store the document");
    assert_eq!(&*doc.text, "set x 1\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_change_replaces_document_text() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///chg.tcl").unwrap();
    Box::pin(backend.did_open(DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version: 1,
            text: "set x 1\n".to_owned(),
        },
    }))
    .await;
    // A full-document replacement (`range: None`).
    backend
        .did_change(DidChangeTextDocumentParams {
            text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "set y 2\n".to_owned(),
            }],
        })
        .await;
    assert_eq!(
        &*backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .expect("doc")
            .text,
        "set y 2\n",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_removes_document_and_clears_pull_cache() {
    // `file:///close.tcl` has no on-disk source, so the closed-file
    // republish finds nothing to analyse and falls back to clearing the
    // badge — the untitled / deleted-file path.
    let backend = test_backend();
    let uri = Uri::from_str("file:///close.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: 0,
            diagnostics: Vec::new(),
        },
    );
    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;
    assert!(
        !backend.documents.lock("test").await.contains_key(&uri),
        "did_close should remove the open document",
    );
    assert!(
        !backend.pull_diag_cache.lock().await.contains_key(&uri),
        "a closed URI with no on-disk source should drop its pull-cache entry",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_change_on_unopened_document_is_dropped() {
    // Notification handlers run concurrently, so a
    // `didChange` can be processed before its `didOpen` or after its
    // `didClose`. It must NOT resurrect/create a phantom document.
    let backend = test_backend();
    let uri = Uri::from_str("file:///phantom.tcl").unwrap();
    backend
        .did_change(DidChangeTextDocumentParams {
            text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 5,
            },
            content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "set y 2\n".to_owned(),
            }],
        })
        .await;
    assert!(
        !backend.documents.lock("test").await.contains_key(&uri),
        "did_change on an unopened URI must not create a document",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selection_range_returns_one_entry_per_position() {
    // The LSP spec requires `result[i]` to answer
    // `positions[i]`. Any position that yields no chain must still produce
    // a range (a degenerate fallback at the cursor) rather than being
    // dropped, which would misalign every later cursor in a multi-cursor
    // Expand Selection. Assert the handler always returns exactly one range
    // per requested position, including out-of-range cursors.
    let backend = test_backend();
    let uri = Uri::from_str("file:///sel.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    let positions = vec![
        Position::new(0, 4),  // on `x`
        Position::new(99, 0), // far past the buffer
        Position::new(0, 0),  // line start
    ];
    let result = backend
        .selection_range(SelectionRangeParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            positions: positions.clone(),
        })
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        result.len(),
        positions.len(),
        "one selection range per requested position",
    );
}

#[tokio::test]
async fn code_lens_resolve_wires_show_references_command() {
    // The proc reference-count lens must resolve to a
    // *clickable* `tcl-lsp.showReferences` command carrying the URI,
    // anchor position, and reference locations — not a bare, inert title.
    let backend = test_backend();
    let uri = Uri::from_str("file:///refs.tcl").unwrap();
    // `helper` defined once and called twice → 2 references.
    register(&backend, &uri, "proc helper {} {}\nhelper\nhelper\n").await;
    let lens_params = CodeLensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let lenses = backend
        .code_lens(lens_params)
        .await
        .expect("ok")
        .expect("some lenses");
    let lens = lenses
        .into_iter()
        .find(|l| {
            l.data
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|d| d.get("qname"))
                .and_then(serde_json::Value::as_str)
                == Some("::helper")
        })
        .expect("a lens for ::helper");
    let resolved = backend.code_lens_resolve(lens).await.expect("resolve ok");
    let command = resolved.command.expect("resolved lens carries a command");
    assert_eq!(
        command.command, "tcl-lsp.showReferences",
        "lens must invoke the show-references wrapper, got {command:?}",
    );
    assert!(
        command.title.contains("references"),
        "title should show the count: {command:?}",
    );
    let args = command.arguments.expect("showReferences needs arguments");
    // [uriString, position, locations]
    assert_eq!(args.len(), 3, "{args:?}");
    assert_eq!(args[0], serde_json::Value::String(uri.to_string()));
    let locations = args[2].as_array().expect("locations array");
    assert_eq!(locations.len(), 2, "two call sites: {locations:?}");
}

/// Resolve the single member lens on `src` whose declaration sits on
/// `decl_line` (0-based) — the harness shared by the `TclOO` method /
/// classmethod lens tests below.
async fn resolve_member_lens_at_line(
    backend: &Backend,
    uri: &Uri,
    src: &str,
    decl_line: u32,
) -> CodeLens {
    resolve_member_lens_at_line_in(backend, uri, src, decl_line, "tcl8.6").await
}

async fn resolve_member_lens_at_line_in(
    backend: &Backend,
    uri: &Uri,
    src: &str,
    decl_line: u32,
    dialect: &str,
) -> CodeLens {
    register_in_dialect(backend, uri, src, dialect).await;
    let lens_params = CodeLensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let lenses = backend
        .code_lens(lens_params)
        .await
        .expect("ok")
        .expect("some lenses");
    let lens = lenses
        .into_iter()
        .find(|l| l.range.start.line == decl_line)
        .unwrap_or_else(|| panic!("no lens anchored at line {decl_line}: src={src:?}"));
    backend.code_lens_resolve(lens).await.expect("resolve ok")
}

#[tokio::test]
async fn code_lens_resolve_wires_show_references_command_for_method() {
    // FN→TP: a TclOO class whose body declares `variable` and
    // `constructor` before the `method`, with the method body reading the
    // instance variable and the external dispatch nested in `puts [...]`.
    // The method lens must resolve to a *clickable*
    // `tcl-lsp.showReferences` command, not an inert bare title — the
    // count can be right while the command is empty.
    let backend = test_backend();
    let uri = Uri::from_str("file:///bar956.tcl").unwrap();
    let src = "oo::class create Bar {\n   variable _options\n    constructor {args} {\n         set _options $args\n    }\n\n    method get {key} {\n        return [dict get $_options $key]\n    }\n\n}\nset b [Bar new]\nputs [$b get foo]\n";
    let resolved = resolve_member_lens_at_line(&backend, &uri, src, 6).await;
    let command = resolved
        .command
        .expect("method lens must resolve to a command, not stay inert");
    assert_eq!(
        command.command, "tcl-lsp.showReferences",
        "method lens must invoke the show-references wrapper, got {command:?}",
    );
    assert!(
        !command.command.is_empty(),
        "command id must never be empty"
    );
    // `dict get` and the dynamic `$b get` head keep the census partial.
    assert_eq!(command.title, "at least 1 reference", "{command:?}");
    let args = command.arguments.expect("showReferences needs arguments");
    let locations = args[2].as_array().expect("locations array");
    assert_eq!(
        locations.len(),
        1,
        "the `puts [$b get foo]` dispatch is the one call site: {locations:?}"
    );
}

#[tokio::test]
async fn code_lens_resolve_wires_show_references_command_for_classmethod() {
    // TP: a classmethod lens resolves the same way a method lens does.
    let backend = test_backend();
    let uri = Uri::from_str("file:///classmethod.tcl").unwrap();
    let src = "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\nset f [Factory make]\n";
    // tclsh 8.6.18: invalid command name "classmethod"; 9.0.4/9.1.0 run
    // Factory make.
    let resolved = resolve_member_lens_at_line_in(&backend, &uri, src, 1, "tcl9.0").await;
    let command = resolved
        .command
        .expect("classmethod lens must resolve to a command, not stay inert");
    assert_eq!(command.command, "tcl-lsp.showReferences", "{command:?}");
}

#[tokio::test]
pub(super) async fn code_lens_resolve_disambiguates_method_and_classmethod_of_the_same_name() {
    // `method make` and `classmethod make` on the same class are two
    // distinct, independently-dispatched members (TclOO allows both:
    // one on the instance, one on the class object). Handing
    // `member_ref_count` / `method_references_for_class` only the bare name
    // combines `$obj make` with `Factory make` for *both* lenses; each lens
    // must count and resolve to *only* its own dispatch shape.
    let backend = test_backend();
    let uri = Uri::from_str("file:///dual_make.tcl").unwrap();
    let src = "oo::class create Factory {\n    method make {} { return 1 }\n    classmethod make {} { return [Factory new] }\n}\nset f [Factory new]\n$f make\nFactory make\n";
    let method_lens = resolve_member_lens_at_line(&backend, &uri, src, 1).await;
    let classmethod_lens = resolve_member_lens_at_line(&backend, &uri, src, 2).await;

    let method_cmd = method_lens
        .command
        .expect("method lens must resolve to a command");
    assert_eq!(method_cmd.title, "1 reference", "{method_cmd:?}");
    let method_locations = method_cmd.arguments.expect("args")[2]
        .as_array()
        .expect("locations array")
        .clone();
    assert_eq!(
        method_locations.len(),
        1,
        "method lens must count only `$f make` (l5), not `Factory make` (l6): {method_locations:?}"
    );

    let classmethod_cmd = classmethod_lens
        .command
        .expect("classmethod lens must resolve to a command");
    assert_eq!(classmethod_cmd.title, "1 reference", "{classmethod_cmd:?}");
    let classmethod_locations = classmethod_cmd.arguments.expect("args")[2]
        .as_array()
        .expect("locations array")
        .clone();
    assert_eq!(
        classmethod_locations.len(),
        1,
        "classmethod lens must count only `Factory make` (l6), not `$f make` (l5): {classmethod_locations:?}"
    );
    assert_ne!(
        method_locations, classmethod_locations,
        "the two lenses must resolve to different call sites"
    );
}

#[tokio::test]
async fn code_lens_resolve_for_unused_method_is_clickable_with_zero_locations() {
    // TN: a declared-but-never-dispatched method still resolves to a
    // real command (an empty peek is a valid "0 references" click
    // target) rather than staying inert because the count is zero.
    let backend = test_backend();
    let uri = Uri::from_str("file:///solo.tcl").unwrap();
    let src = "oo::class create Solo {\n    method ping {} { return pong }\n}\n";
    let resolved = resolve_member_lens_at_line(&backend, &uri, src, 1).await;
    let command = resolved
        .command
        .expect("even a 0-reference method lens must carry a real command");
    assert_eq!(command.command, "tcl-lsp.showReferences", "{command:?}");
    assert_eq!(command.title, "0 references", "{command:?}");
    let args = command.arguments.expect("showReferences needs arguments");
    let locations = args[2].as_array().expect("locations array");
    assert!(locations.is_empty(), "{locations:?}");
}

#[tokio::test]
async fn code_lens_resolve_disambiguates_same_named_methods_on_different_classes() {
    // FP guard: two unrelated classes each declaring a method named
    // `get` must resolve to *independent* lenses — the qname-based
    // resolve match (`::Bar::method::get` vs `::Foo::method::get`) must
    // never cross-contaminate the two, and each lens's clickable command
    // must carry only its own class's call site.
    let backend = test_backend();
    let uri = Uri::from_str("file:///two_classes.tcl").unwrap();
    let src = "oo::class create Bar {\n    method get {} { return 1 }\n}\noo::class create Foo {\n    method get {} { return 2 }\n}\nset b [Bar new]\nputs [$b get]\n";
    let bar_resolved = resolve_member_lens_at_line(&backend, &uri, src, 1).await;
    let bar_command = bar_resolved.command.expect("Bar::get must resolve");
    assert_eq!(bar_command.title, "1 reference", "{bar_command:?}");

    let foo_resolved = resolve_member_lens_at_line(&backend, &uri, src, 3).await;
    let foo_command = foo_resolved.command.expect("Foo::get must resolve");
    assert_eq!(
        foo_command.title, "0 references",
        "Foo::get must not see Bar's dispatch: {foo_command:?}"
    );
}

#[tokio::test]
async fn code_lens_resolve_for_inherited_method_counts_subclass_dispatch() {
    // TP: a subclass that inherits (does not override) a method is a
    // valid dispatch target for the ancestor's lens — mirrors the
    // existing `find_obj_method_call_sites` inheritance coverage, wired
    // through the resolve round-trip.
    let backend = test_backend();
    let uri = Uri::from_str("file:///inherit.tcl").unwrap();
    let src = "oo::class create Base {\n    method get {key} { return $key }\n}\noo::class create Sub {\n    superclass Base\n}\nset s [Sub new]\n$s get x\n";
    let resolved = resolve_member_lens_at_line(&backend, &uri, src, 1).await;
    let command = resolved.command.expect("Base::get must resolve");
    assert_eq!(
        command.title, "at least 1 reference",
        "`$s get x` dispatches to the inherited Base::get: {command:?}"
    );
}

#[tokio::test]
async fn no_code_lens_ever_carries_an_inert_empty_command() {
    // Broad guard for the inert-lens defect class: every
    // lens this server can emit for a rich TclOO document — proc,
    // class, method, classmethod, across inheritance — must resolve to
    // a real, non-empty command id.  A future lens kind that forgets to
    // carry a qname would be caught here rather than shipping silently
    // inert.
    let backend = test_backend();
    let uri = Uri::from_str("file:///rich.tcl").unwrap();
    let src = "proc helper {} {}\nhelper\noo::class create Base {\n    variable _options\n    constructor {args} { set _options $args }\n    method get {key} { return [dict get $_options $key] }\n    classmethod make {} { return [Base new] }\n    method unused {} { return 0 }\n}\noo::class create Sub {\n    superclass Base\n}\nset b [Base new]\nputs [$b get foo]\nBase make\nset s [Sub new]\n$s get bar\n";
    register(&backend, &uri, src).await;
    let lens_params = CodeLensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let lenses = backend
        .code_lens(lens_params)
        .await
        .expect("ok")
        .expect("some lenses");
    assert!(
        lenses.len() >= 4,
        "expected proc + class + 2 member lenses, got {lenses:?}"
    );
    for lens in lenses {
        let resolved = backend
            .code_lens_resolve(lens.clone())
            .await
            .expect("resolve ok");
        let command = resolved
            .command
            .unwrap_or_else(|| panic!("lens at {:?} resolved with no command at all", lens.range));
        assert!(
            !command.command.is_empty(),
            "lens at {:?} resolved to an inert empty command id: {command:?}",
            lens.range,
        );
    }
}

#[tokio::test]
async fn document_link_handler_bigip_conf_links_irule_pool_ref() {
    // A BIG-IP `.conf` document routes document links through the object-
    // reference resolver: the iRule's `pool /Common/web_pool` resolves to
    // the pool stanza on line 0 (a `#L1` fragment target).
    let backend = test_backend();
    let uri = Uri::from_str("file:///bigip.conf").unwrap();
    let src = "ltm pool /Common/web_pool { }\nltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/web_pool }\n}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
    );
    let params = DocumentLinkParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let result = backend
        .document_link(params)
        .await
        .expect("ok")
        .expect("some");
    let link = result
        .iter()
        .find(|l| l.tooltip.as_deref() == Some("Go to /Common/web_pool"))
        .expect("pool ref link");
    let target = link.target.as_ref().expect("resolved target");
    assert!(target.as_str().contains("#L1"), "target = {target:?}");
}
