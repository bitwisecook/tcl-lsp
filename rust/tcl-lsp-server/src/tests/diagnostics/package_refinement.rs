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

//! W120 against packages, declared provides and the `source` graph.

use super::*;

fn w120_diag(pkg: &str) -> tcl_compiler::analyser::Diagnostic {
    use tcl_compiler::analyser::types::{CodeFix, Severity};
    tcl_compiler::analyser::Diagnostic {
        subject: Some(tcl_compiler::analyser::DiagnosticSubject::RequiredPackage(
            tcl_registry::native_package::NativePackageNameKey::from_native_units(
                pkg.as_bytes(),
                tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
            ),
        )),
        code: DiagCode::W120,
        span: tcl_lexer::Span::new(0, 1),
        message: format!("\"cmd\" requires `package require {pkg}`"),
        severity: Severity::Warning,
        fixes: vec![CodeFix {
            span: tcl_lexer::Span::new(0, 0),
            new_text: format!("package require {pkg}\n"),
            description: format!("Add 'package require {pkg}'"),
            safety: tcl_compiler::analyser::FixSafety::BehaviourHardening,
        }],
    }
}

fn has_w120(diags: &[tcl_compiler::analyser::Diagnostic]) -> bool {
    diags.iter().any(|d| d.code == DiagCode::W120)
}

#[test]
fn original_package_consumers_keep_constraints_and_dependency_preference() {
    // Implementation contract: naming.server.original-package-consumer-resolution
    // docs/design/analysis/name-resolution-proofs/server-original-package-consumer-resolution.md

    use tcl_lsp_core::package_resolver::{PackagePrefer, PackageRequirementAdvice};
    let store = vfs::MemoryStore::new();
    store.upsert(
        "/library/stable.tcl",
        b"proc old_command {} {}\npackage prefer latest\npackage require dep 1.0".to_vec(),
    );
    store.upsert("/library/beta.tcl", b"proc beta_command {} {}".to_vec());
    store.upsert("/library/dep-old.tcl", b"proc dep_old {} {}".to_vec());
    store.upsert("/library/dep-beta.tcl", b"proc dep_beta {} {}".to_vec());
    let mut resolver = PackageResolver::new();
    resolver.add_original_pkg_index(
            "package ifneeded w 1.2 {source stable.tcl}\npackage ifneeded w 1.3b1 {source beta.tcl}\npackage ifneeded dep 1.1 {source dep-old.tcl}\npackage ifneeded dep 1.2b1 {source dep-beta.tcl}",
            Path::new("/library"), Path::new("/library/pkgIndex.tcl"), &|_| true, &|_| Vec::new());
    let dialect = tcl_lsp_core::profile_for_dialect("tcl8.6");
    let call_source = "old_command\nbeta_command\ndep_old\ndep_beta";
    // Positioned absence advice requires the independently selected name
    // recipe. Logical declaration advice supplies no absence verdict.
    let logical_calls = Analyser::new().analyse(call_source, "tcl");
    assert!(logical_calls.allows_lexical_declaration_advice());
    assert!(
        logical_calls
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.unresolved_command().is_none())
    );
    let calls = Analyser::new().analyse(call_source, "tcl8.6");
    assert!(!calls.allows_lexical_declaration_advice());
    assert!(calls.original_completed_command_world().is_none());
    assert!(calls.matches_original_source_image(
        &tcl_lexer::SourceImage::document(call_source),
        calls.resolved_input.as_ref().unwrap().lexer_config(),
    ));
    assert_eq!(
        calls
            .diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.unresolved_command())
            .count(),
        4
    );
    let exact = Analyser::new()
        .structure_only()
        .analyse("package require -exact w 1.2", "tcl8.6");
    let requirements = exact
        .package_requires
        .iter()
        .map(|required| {
            PackageRequirementAdvice::from_source_requirement(required, PackagePrefer::Stable)
        })
        .collect::<Vec<_>>();
    let remaining = refine_original_w123_diagnostics(
        calls.diagnostics.clone(),
        &requirements,
        &resolver,
        &provider_source::ProviderSourceAccess::Standalone(&store),
        dialect,
    );
    let unresolved = remaining
        .iter()
        .filter_map(|diagnostic| diagnostic.unresolved_command())
        .map(|subject| subject.name_input().bytes().to_vec())
        .collect::<HashSet<_>>();
    assert_eq!(
        unresolved,
        HashSet::from([b"beta_command".to_vec(), b"dep_old".to_vec()])
    );
    let latest = Analyser::new()
        .structure_only()
        .analyse("package require w 1.0", "tcl8.6");
    let requirements = latest
        .package_requires
        .iter()
        .map(|required| {
            PackageRequirementAdvice::from_source_requirement(required, PackagePrefer::Latest)
        })
        .collect::<Vec<_>>();
    let remaining = refine_original_w123_diagnostics(
        calls.diagnostics,
        &requirements,
        &resolver,
        &provider_source::ProviderSourceAccess::Standalone(&store),
        dialect,
    );
    let unresolved = remaining
        .iter()
        .filter_map(|diagnostic| diagnostic.unresolved_command())
        .map(|subject| subject.name_input().bytes().to_vec())
        .collect::<HashSet<_>>();
    assert_eq!(
        unresolved,
        HashSet::from([
            b"old_command".to_vec(),
            b"dep_old".to_vec(),
            b"dep_beta".to_vec()
        ])
    );
}

#[test]
fn refine_w120_conservative_drops_all_for_unknowable_require() {
    // `package require myTkPackage` is unknown to both the registry and an
    // empty package database, so it may load anything — drop the W120.
    let registry = CommandRegistry::build_default();
    let resolver = PackageResolver::new();
    let out = refine_w120_diagnostics(
        vec![w120_diag("Tk")],
        &["myTkPackage".to_owned()],
        &resolver,
        &vfs::NativeStore,
        &registry,
    );
    assert!(
        !has_w120(&out),
        "unknowable require ⇒ W120 dropped: {out:?}"
    );
}

#[test]
fn refine_w120_suppressed_when_wrapper_transitively_requires_tk() {
    // The precise case: a workspace package whose implementation does
    // `package require Tk` makes Tk available, so the Tk W120 is a false
    // positive.
    let ws = TmpWs::new("wrap");
    ws.write(
        "mytk/mytk.tcl",
        "package provide myTkPackage 1.0\npackage require Tk\nproc mytk::go {} {}\n",
    );
    ws.write(
        "mytk/pkgIndex.tcl",
        "package ifneeded myTkPackage 1.0 [list source [file join $dir mytk.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_tree(&ws.0, 100);
    let registry = CommandRegistry::build_default();
    let out = refine_w120_diagnostics(
        vec![w120_diag("Tk")],
        &["myTkPackage".to_owned()],
        &resolver,
        &vfs::NativeStore,
        &registry,
    );
    assert!(
        !has_w120(&out),
        "wrapper transitively requires Tk ⇒ W120 suppressed: {out:?}"
    );
}

#[test]
fn refine_w120_kept_when_resolvable_require_does_not_provide_tk() {
    // A resolvable package that does NOT pull in Tk leaves the Tk W120
    // standing — the refinement is precise, not a blanket suppression.
    let ws = TmpWs::new("plain");
    ws.write(
        "plain/plain.tcl",
        "package provide plain 1.0\nproc plain::p {} {}\n",
    );
    ws.write(
        "plain/pkgIndex.tcl",
        "package ifneeded plain 1.0 [list source [file join $dir plain.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_tree(&ws.0, 100);
    let registry = CommandRegistry::build_default();
    let out = refine_w120_diagnostics(
        vec![w120_diag("Tk")],
        &["plain".to_owned()],
        &resolver,
        &vfs::NativeStore,
        &registry,
    );
    assert!(
        has_w120(&out),
        "plain doesn't provide Tk ⇒ W120 kept: {out:?}"
    );
}

#[test]
fn auto_source_graph_inherits_entry_requires() {
    // app.tcl requires Tk and sources lib/util.tcl; util inherits Tk, so a
    // Tk W120 in util is suppressed with no configuration at all.
    let app = Uri::from_file_path("/proj/app.tcl").unwrap();
    let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
    let index = ws_index(&[
        (&app, "package require Tk\nsource lib/util.tcl\n"),
        (&util, "proc u {} {}\n"),
    ]);
    let inherited = compute_inherited_requires(&index, &util, &[], None);
    assert_eq!(inherited, vec!["Tk".to_owned()]);
    // The entry file itself inherits nothing.
    assert_eq!(
        compute_inherited_requires(&index, &app, &[], None),
        [] as [std::string::String; 0]
    );
}

#[test]
fn explicit_entry_points_override_source_graph() {
    // main.tcl requires Tk but does NOT source other.tcl; with main.tcl set
    // as an entry point, other.tcl still inherits Tk (project-wide), and the
    // auto source-graph is bypassed entirely.
    let root = PathBuf::from("/proj");
    let main = Uri::from_file_path("/proj/main.tcl").unwrap();
    let other = Uri::from_file_path("/proj/other.tcl").unwrap();
    let index = ws_index(&[(&main, "package require Tk\n"), (&other, "proc o {} {}\n")]);
    // Auto mode: other.tcl is not sourced by main, so it inherits nothing.
    assert_eq!(
        compute_inherited_requires(&index, &other, &[], Some(&root)),
        [] as [std::string::String; 0]
    );
    // Explicit entry point: other.tcl inherits main.tcl's Tk.
    let entries = vec!["main.tcl".to_owned()];
    let inherited = compute_inherited_requires(&index, &other, &entries, Some(&root));
    assert_eq!(inherited, vec!["Tk".to_owned()]);
}

/// A `source`d file's `package require` is available *after* the statement
/// and not before it — the C Tcl 9.0.4 behaviour recorded on
/// `source_graph::descendant_requires`.
#[test]
fn placed_requires_are_only_available_after_the_source_statement() {
    let inheritance = SourceInheritance {
        original: None,
        ambient: Vec::new(),
        placed: vec![tcl_lsp_core::source_graph::PlacedRequire {
            name: "Tk".to_owned(),
            at: 20,
            enclosing_body: None,
        }],
        unresolvable_source: false,
    };
    assert!(
        inheritance.available_at(5, None).is_empty(),
        "a call above the `source` runs before it",
    );
    assert_eq!(
        inheritance.available_at(40, None),
        vec!["Tk".to_owned()],
        "a call below the `source` runs after it",
    );
}

/// A load-level `source` counts for a call written inside *any* proc body,
/// wherever that body sits — the whole file loads before any body runs.
#[test]
fn a_load_level_source_covers_a_call_inside_a_proc_body() {
    let inheritance = SourceInheritance {
        original: None,
        ambient: Vec::new(),
        placed: vec![tcl_lsp_core::source_graph::PlacedRequire {
            name: "Tk".to_owned(),
            at: 90,
            enclosing_body: None,
        }],
        unresolvable_source: false,
    };
    // A call at offset 40 inside a body spanning 30..60 — textually above
    // the `source` at 90, but it cannot run until the body is invoked.
    let body = tcl_lexer::Span::new(30, 60);
    assert_eq!(
        inheritance.available_at(40, Some(body)),
        vec!["Tk".to_owned()],
    );
}

/// The whole up direction is inert for a document that `source`s nothing,
/// which is the shape of almost every file — and the abstention flag stays
/// down, so W120/W123 keep working normally.
#[test]
fn a_document_that_sources_nothing_inherits_nothing_and_does_not_abstain() {
    let uri = Uri::from_file_path("/proj/solo.tcl").unwrap();
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse("winfo exists .l\n", "tcl8.6")
        .clone();
    let index = ws_index(&[(&uri, "winfo exists .l\n")]);
    let got = compute_source_inheritance(
        &index,
        &uri,
        &analysis,
        &[],
        None,
        tcl_lsp_core::package_resolver::PackagePrefer::default(),
    );
    assert_eq!(
        got.placed,
        [] as [tcl_lsp_core::source_graph::PlacedRequirement<std::string::String>; 0]
    );
    assert!(!got.unresolvable_source);
    assert!(got.is_empty());
}

/// A `source` whose target is not an indexed document — outside the
/// workspace, absent, or a path no static fold can prove — sets the
/// abstention flag, and W120/W123 then go quiet document-wide.
#[test]
fn an_unfollowable_source_sets_the_abstention_flag() {
    let uri = Uri::from_file_path("/proj/main.tcl").unwrap();
    let src = "source nosuchfile.tcl\nwinfo exists .l\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(src, "tcl8.6")
        .clone();
    let index = ws_index(&[(&uri, src)]);
    let got = compute_source_inheritance(
        &index,
        &uri,
        &analysis,
        &[],
        None,
        tcl_lsp_core::package_resolver::PackagePrefer::default(),
    );
    assert!(
        got.unresolvable_source,
        "a `source` of a file the workspace does not hold is unknowable",
    );
}

/// The followable case: `main.tcl` sources `tkFile.tcl`, which requires
/// Tk. Tk arrives in `main.tcl`, and the abstention flag stays down —
/// followable source, so no abstention.
#[test]
fn a_followable_source_contributes_its_requires_without_abstaining() {
    let main = Uri::from_file_path("/proj/main.tcl").unwrap();
    let tk = Uri::from_file_path("/proj/tkFile.tcl").unwrap();
    let src = "source tkFile.tcl\nwinfo exists .l\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(src, "tcl8.6")
        .clone();
    let index = ws_index(&[(&main, src), (&tk, "package require Tk\n")]);
    let got = compute_source_inheritance(
        &index,
        &main,
        &analysis,
        &[],
        None,
        tcl_lsp_core::package_resolver::PackagePrefer::default(),
    );
    assert!(
        !got.unresolvable_source,
        "a literal `source` of an indexed file is followable",
    );
    assert_eq!(
        got.placed
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["Tk"],
        "{got:?}",
    );
}

/// **TN.** Sourcing a file that requires nothing contributes nothing, so a
/// Tk-command W120 in the sourcing file still stands. Option (2) must not
/// have been applied globally.
#[test]
fn sourcing_a_non_tk_file_contributes_nothing() {
    let main = Uri::from_file_path("/proj/main.tcl").unwrap();
    let plain = Uri::from_file_path("/proj/plain.tcl").unwrap();
    let src = "source plain.tcl\nwinfo exists .l\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(src, "tcl8.6")
        .clone();
    let index = ws_index(&[(&main, src), (&plain, "proc helper {} { return 1 }\n")]);
    let got = compute_source_inheritance(
        &index,
        &main,
        &analysis,
        &[],
        None,
        tcl_lsp_core::package_resolver::PackagePrefer::default(),
    );
    assert!(!got.unresolvable_source);
    assert!(got.placed.is_empty(), "{got:?}");
}

/// A module `source`d by an entry file that ran `package require`
/// inherits that require, so the sourced module's W120 for the same package
/// is suppressed by the automatic workspace `source` graph — no config.
#[tokio::test]
pub(in super::super) async fn source_graph_inheritance_suppresses_w120_in_sourced_module() {
    let backend = test_backend();
    // Resolver that knows `http` (so the require is not "unknowable").
    let http_ws = TmpWs::new("srcgraph-http");
    http_ws.write(
        "http/http.tcl",
        "package provide http 2.9\nproc http::register {a b c} {}\n",
    );
    http_ws.write(
        "http/pkgIndex.tcl",
        "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
    );
    {
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&http_ws.0, 100);
        *backend.package_resolver.write().await = resolver;
    }
    let app = Uri::from_file_path("/proj/app.tcl").unwrap();
    let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
    // util.tcl uses http::register with no local `package require`.
    let util_src = "http::register foo 80 bar\n";

    // Control: with the entry file NOT indexed, util inherits nothing, so
    // the single-file W120 stands.
    let unrefined = backend
        .full_diagnostics_for(&util, Arc::from(util_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        diag_codes(&unrefined).iter().any(|c| c == "W120"),
        "without the entry file indexed the W120 must stand, got: {:?}",
        diag_codes(&unrefined),
    );

    // Index the entry file: it requires http and sources lib/util.tcl.
    {
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a
            .analyse("package require http\nsource lib/util.tcl\n", "tcl8.6")
            .clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(app.as_str(), &analysis);
    }
    let refined = backend
        .full_diagnostics_for(&util, Arc::from(util_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&refined).iter().any(|c| c == "W120"),
        "the sourced module must inherit the entry file's `package require http`, got: {:?}",
        diag_codes(&refined),
    );
}

/// An explicitly configured `[project] entryPoints` makes that entry
/// file's requires available project-wide even when it does NOT `source` the
/// module — and it disables the automatic source-graph path.
#[tokio::test]
async fn explicit_entry_point_config_suppresses_w120_project_wide() {
    let backend = test_backend();
    let http_ws = TmpWs::new("entrypt-http");
    http_ws.write(
        "http/http.tcl",
        "package provide http 2.9\nproc http::register {a b c} {}\n",
    );
    http_ws.write(
        "http/pkgIndex.tcl",
        "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
    );
    {
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&http_ws.0, 100);
        *backend.package_resolver.write().await = resolver;
    }
    let folder = Uri::from_file_path("/proj").unwrap();
    let main = Uri::from_file_path("/proj/main.tcl").unwrap();
    let other = Uri::from_file_path("/proj/other.tcl").unwrap();
    // main.tcl requires http but does NOT source other.tcl.
    {
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse("package require http\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(main.as_str(), &analysis);
    }
    // Configure main.tcl as the project entry point.
    {
        let fc = FolderConfig {
            entry_points: Some(vec!["main.tcl".to_owned()]),
            ..FolderConfig::default()
        };
        *backend.folder_configs.lock().await = vec![(folder.clone(), fc)];
    }
    let other_src = "http::register foo 80 bar\n";
    let refined = backend
        .full_diagnostics_for(&other, Arc::from(other_src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&refined).iter().any(|c| c == "W120"),
        "an explicit entry point's requires must apply project-wide, got: {:?}",
        diag_codes(&refined),
    );
}

/// TP: a `load`-only `pkgIndex.tcl` whose directory holds no companion
/// `.tcl` is still a declared package, so `provides` answers `true` and
/// requiring it does not erase every other W120 in the document.
#[test]
fn load_only_pkg_index_declares_a_known_package() {
    let dir = Path::new("/pkg");
    let infos = tcl_lsp_core::package_resolver::parse_pkg_index(
        "package ifneeded pix 0.8 [list apply {dir {load [file join $dir libpix.so] Pix}} $dir]\n",
        dir,
        &dir.join("pkgIndex.tcl"),
        &|_| false,
        &|_| Vec::new(),
    );
    assert_eq!(infos.len(), 1, "the literal declaration must be recorded");
    assert_eq!(infos[0].name, "pix");
    assert_eq!(infos[0].version, "0.8");
    assert!(
        infos[0].source_files.is_empty(),
        "a binary-only package has no statically enumerable Tcl source",
    );

    // NEGATIVE control: text that is not a literal `package ifneeded NAME
    // VERSION` is still recorded as nothing.
    let none = tcl_lsp_core::package_resolver::parse_pkg_index(
        "package ifneeded $name $version [list load whatever]\n",
        dir,
        &dir.join("pkgIndex.tcl"),
        &|_| false,
        &|_| Vec::new(),
    );
    assert!(
        none.is_empty(),
        "a non-literal name/version declares no package: {none:?}",
    );
}

/// TN: requiring a `load`-only package must not suppress an unrelated
/// W120 elsewhere in the same document.
#[test]
fn requiring_a_load_only_package_keeps_other_w120s() {
    let ws = TmpWs::new("loadonly");
    ws.write(
            "testpix/pkgIndex.tcl",
            "package ifneeded testpix 0.8 [list apply {dir {load [file join $dir libtestpix.so] Pix}} $dir]\n",
        );
    let mut resolver = PackageResolver::new();
    resolver.scan_tree(&ws.0, 100);
    assert!(
        resolver.provides("testpix"),
        "a load-only package is declared, so the database knows it",
    );
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let kept = refine_w120_diagnostics(
        vec![w120_diag("http")],
        &["testpix".to_owned()],
        &resolver,
        &vfs::NativeStore,
        registry,
    );
    assert_eq!(
        kept.len(),
        1,
        "an unrelated W120 must survive a require of a binary-only package",
    );

    // NEGATIVE control: a package nothing declares is still "unknowable",
    // and the blanket suppression still applies.
    let dropped = refine_w120_diagnostics(
        vec![w120_diag("http")],
        &["neverheardof".to_owned()],
        &resolver,
        &vfs::NativeStore,
        registry,
    );
    assert!(
        dropped.is_empty(),
        "a genuinely unknown package must still suppress W120",
    );
}

/// The declared edge has to reach the *analyser*, not just
/// the backend field. A binary extension loads Tk with nothing in any Tcl
/// source to say so, and the Tk checks are gated on the document being Tk
/// — so TK1002 (a widget path whose parent was never created) is the
/// visible proof that the declaration activated Tk.
///
/// W120 is the wrong probe here: `myExtension` is unresolvable in an
/// empty workspace, and the server already drops every W120 for a
/// document requiring a package it cannot resolve (the conservative
/// rule). The gap this closes is everything W120's abstention does not
/// cover — the Tk checks, completions, and hover.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_package_provides_makes_tk_available_to_diagnostics() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///ext.tcl").unwrap();
    let src = "package require myExtension\nframe .outer.inner\n";
    let has_tk1002 = |diags: &[tower_lsp_server::ls_types::Diagnostic]| {
        diags.iter().any(|d| {
            matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "TK1002"
            )
        })
    };
    register(&backend, &uri, src).await;

    let before = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !has_tk1002(&before),
        "undeclared ⇒ not a Tk document: {before:?}",
    );

    backend
        .apply_global_config(&serde_json::json!({
            "packages": { "provides": { "myExtension": ["Tk"] } },
        }))
        .await
        .unwrap();
    let after = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        has_tk1002(&after),
        "declared ⇒ the Tk checks run: {after:?}",
    );
}

/// The declared edges must survive an incomplete buffer.
/// `compute_base_analysis` switches to `recovery_analyser` on an unclosed
/// delimiter, and that analysis is published *and* indexed — so dropping
/// the edges there makes the Tk
/// checks and W120/H301 flicker for every keystroke between an opening
/// brace and its match, and republished an index entry without the
/// implied requires.
#[tokio::test]
async fn declared_package_provides_survive_an_incomplete_buffer() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///ext_recovery.tcl").unwrap();
    // Unclosed brace: configured Document completeness is false, so this takes the
    // recovery branch.
    let src = "package require myExtension
frame .outer.inner
proc p {} {
";
    assert!(
        tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::document(src),
            tcl_lexer::LexerConfig::for_file_grammar(
                tcl_lsp_core::profile_for_dialect("tcl8.6").grammar,
            ),
        ) == Ok(false),
        "the fixture must actually take the recovery branch"
    );
    let has_tk1002 = |diags: &[tower_lsp_server::ls_types::Diagnostic]| {
        diags.iter().any(|d| {
            matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "TK1002"
            )
        })
    };
    register(&backend, &uri, src).await;
    backend
        .apply_global_config(&serde_json::json!({
            "packages": { "provides": { "myExtension": ["Tk"] } },
        }))
        .await
        .unwrap();
    let diags = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        has_tk1002(&diags),
        "an unclosed brace must not switch Tk off: {diags:?}",
    );
}
