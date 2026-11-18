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

//! Which documents a changed workspace fact wakes.

use super::*;

/// Explicit default-source producer setup for the pre-existing orphan
/// fixture. No helper declaration or argument/body assumption is supplied.
fn retain_default_orphan_query_fixture(
    uri: &Uri,
    document: &mut DocumentState,
) -> Arc<Mutex<HashMap<Uri, DiagSlot>>> {
    let mut analyser = tcl_compiler::analyser::Analyser::new();
    let analysis = Arc::new(analyser.analyse(&document.text, &document.dialect));
    let registry = analysis.resolved_registry().unwrap();
    let index = core_workspace_index::WorkspaceIndex::default();
    let settlement = settle_cross_file_calls(&index, &analysis, registry, uri.as_str());
    // This actual default producer configuration has no Project callback
    // pass. It still runs the same source-authenticated settlement producer.
    // No Project provider ran in this actual default fixture. Its only
    // command-header dependency is the genuine settlement producer's
    // retained query receipt, just as in the production no-Project path.
    let queries = WorkspaceCommandQueries::NoRead.join(settlement.command_queries);
    assert!(
        tcl_compiler::source_graph::current_analysis(
            &tcl_lexer::normalise_lone_cr(&document.text),
            &analysis,
        )
        .is_some()
    );
    let slots = Arc::new(Mutex::new(HashMap::new()));
    document.orphan_command_dependencies = Some(Arc::new(OrphanCommandDependencies {
        uri: uri.clone(),
        source: Arc::clone(&document.text),
        dialect: document.dialect.clone(),
        revision: document.revision,
        version: document.version,
        analysis,
        expected_epoch: 0,
        current_epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        inputs_gate: Arc::new(tokio::sync::RwLock::new(())),
        slots: Arc::downgrade(&slots),
        slot_generation: None,
        factory_generation: Arc::new(tokio::sync::RwLock::new(0)),
        expected_factory_generation: 0,
        queries,
    }));
    slots
}

/// Legacy signature dependencies remain targeted by names and arity.
/// A body edit independently changes complete original source ownership.
#[test]
fn indexed_signature_changes_select_only_their_callers() {
    let lib = Uri::from_file_path("/proj/lib.tcl").unwrap();
    let caller = Uri::from_file_path("/proj/caller.tcl").unwrap();
    let unrelated = Uri::from_file_path("/proj/unrelated.tcl").unwrap();
    let mut index = ws_index(&[
        (&lib, "proc helper {a} {}\n"),
        (&caller, "helper 1 2\n"),
        (&unrelated, "other 1 2\n"),
    ]);
    let before = IndexedDiagnosticFacts::capture(&index, lib.as_str());
    let edited = Analyser::new()
        .analyse("proc helper {a b} {}\n", "tcl8.6")
        .clone();
    index.replace_document(lib.as_str(), &edited);
    let after = IndexedDiagnosticFacts::capture(&index, lib.as_str());
    let change = before.change_to(&after);
    assert_eq!(
        change.command_names,
        HashSet::from(["::helper".to_owned()]),
        "an arity edit changes the callable signature without changing its name",
    );
    assert_eq!(
        command_diagnostic_consumers(&index, &change.command_names),
        HashSet::from([caller.as_str().to_owned()]),
        "only the matching caller consumes this signature",
    );

    let stable = IndexedDiagnosticFacts::capture(&index, lib.as_str());
    let body_only = Analyser::new()
        .analyse("proc helper {a b} { return $a }\n", "tcl8.6")
        .clone();
    index.replace_document(lib.as_str(), &body_only);
    let body_change = stable.change_to(&IndexedDiagnosticFacts::capture(&index, lib.as_str()));
    assert!(body_change.command_names.is_empty());
    assert!(
        body_change.original_changed,
        "changed full source withdraws retained original ownership"
    );
    assert!(body_change.moved_workspace_facts());
}

fn diagnostic_tcl_document(source: &str) -> DocumentState {
    DocumentState::new(source.to_owned(), "tcl8.6".to_owned())
}

#[test]
fn original_diagnostic_invalidation_keeps_opaque_dependencies_and_foreign_surfaces_separate() {
    // Implementation contract: naming.editor.original-diagnostic-invalidation
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-invalidation.md
    let library = Uri::from_str("file:///opaque-provider.tcl").unwrap();
    let caller = Uri::from_str("file:///opaque-caller.tcl").unwrap();
    let sibling = Uri::from_str("file:///opaque-sibling.tcl").unwrap();
    let javascript = Uri::from_str("file:///foreign.js").unwrap();
    let config = Uri::from_str("file:///bigip.conf").unwrap();
    let before_source = r"proc p\uD800 {one} {}";
    let after_source = r"proc p\uD800 {one two} {}";
    let caller_source = r"p\uD800 1";
    let sibling_source = r"p\uD801 1";
    let mut original = Analyser::new().analyse(before_source, "tcl8.6");
    original.all_procs.clear();
    original.proc_declaration_sites.clear();
    let mut caller_analysis = Analyser::new().analyse(caller_source, "tcl8.6");
    for invocation in &mut caller_analysis.command_invocations {
        invocation.name = "counterfactual-report".to_owned();
        invocation.resolution_candidates = vec!["counterfactual-report".to_owned()];
    }
    let sibling_analysis = Analyser::new().analyse(sibling_source, "tcl8.6");
    assert!(
        caller_analysis
            .command_invocations
            .iter()
            .any(|invocation| invocation.original_lookup.is_some())
    );
    let mut index = core_workspace_index::WorkspaceIndex::from_documents([
        (library.as_str(), &original),
        (caller.as_str(), &caller_analysis),
        (sibling.as_str(), &sibling_analysis),
    ]);
    let before = IndexedDiagnosticFacts::capture(&index, library.as_str());
    let mut after_analysis = Analyser::new().analyse(after_source, "tcl8.6");
    after_analysis.all_procs.clear();
    after_analysis.proc_declaration_sites.clear();
    index.replace_document(library.as_str(), &after_analysis);
    let after = IndexedDiagnosticFacts::capture(&index, library.as_str());
    let change = before.change_to(&after);
    assert_eq!(before.procs, after.procs);
    assert_eq!(before.classes, after.classes);
    assert!(change.command_names.is_empty());
    assert!(change.original_changed && change.moved_workspace_facts());
    let mut docs = HashMap::from([
        (library.clone(), diagnostic_tcl_document(after_source)),
        (caller.clone(), diagnostic_tcl_document(caller_source)),
        (sibling.clone(), diagnostic_tcl_document(sibling_source)),
        (
            javascript.clone(),
            DocumentState::new(caller_source.to_owned(), "unknown-js".to_owned())
                .with_language_id("javascript".to_owned()),
        ),
        (
            config.clone(),
            DocumentState::new(caller_source.to_owned(), "f5-bigip".to_owned())
                .with_language_id("f5-bigip".to_owned()),
        ),
    ]);
    assert_eq!(
        original_diagnostic_consumers(&index, &docs, &change),
        HashSet::from([caller.as_str().to_owned()])
    );
    assert!(
        !after
            .change_to(&IndexedDiagnosticFacts::capture(&index, library.as_str()))
            .moved_workspace_facts()
    );
    // Missing actual caller lookup cannot be repaired from the report.
    caller_analysis
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.original_lookup.is_some())
        .unwrap()
        .original_lookup = None;
    index.replace_document(caller.as_str(), &caller_analysis);
    let all_tcl = HashSet::from([
        library.as_str().to_owned(),
        caller.as_str().to_owned(),
        sibling.as_str().to_owned(),
    ]);
    assert_eq!(
        original_diagnostic_consumers(&index, &docs, &change),
        all_tcl
    );
    assert!(
        original_diagnostic_consumers(&index, &docs, &IndexedDiagnosticChange::default())
            .is_empty()
    );
    // An unpublished changed source has unknown current dependencies.
    index.replace_document(
        caller.as_str(),
        &Analyser::new().analyse(caller_source, "tcl8.6"),
    );
    docs.get_mut(&caller).unwrap().text = Arc::from("# changed caller source");
    assert_eq!(
        original_diagnostic_consumers(&index, &docs, &change),
        all_tcl
    );
}

/// **A constants-only edit is a source-graph change.** Retargeting
/// `set dir /a` to `/b` leaves the `source [file
/// join $dir x.tcl]` row byte-identical, so comparing rows alone reports
/// "nothing changed" and neither the old child nor the new one is
/// rescheduled — both were stale.  The captured constants must move the
/// needle, and the old rows must reconstruct the **old** edge from the
/// old constants, not the post-edit ones.
#[test]
fn a_constants_only_edit_reschedules_both_children() {
    let parent = Uri::from_file_path("/proj/parent.tcl").unwrap();
    let old_child = Uri::from_file_path("/proj/a/x.tcl").unwrap();
    let new_child = Uri::from_file_path("/proj/b/x.tcl").unwrap();
    let parent_before = "set dir /proj/a\nsource [file join $dir x.tcl]\n";
    let mut index = ws_index(&[
        (&parent, parent_before),
        (&old_child, "proc a_helper {} {}\n"),
        (&new_child, "proc b_helper {} {}\n"),
    ]);
    let before = IndexedDiagnosticFacts::capture(&index, parent.as_str());
    let edited = Analyser::new()
        .analyse("set dir /proj/b\nsource [file join $dir x.tcl]\n", "tcl8.6")
        .clone();
    index.replace_document(parent.as_str(), &edited);
    let after = IndexedDiagnosticFacts::capture(&index, parent.as_str());
    let change = before.change_to(&after);
    // Rows also retain the sealed analysis lineage of the document they were
    // read from, which differs between any two byte-different documents; the
    // authored statement (parent, path text, span) is what must be untouched.
    let authored = |facts: &IndexedDiagnosticFacts| {
        facts
            .sources
            .iter()
            .map(|source| (source.uri.clone(), source.raw_path.clone(), source.range))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        authored(&before),
        authored(&after),
        "precondition: the source row text is untouched by this edit",
    );
    assert!(
        change.source_or_package_changed,
        "the constant moved, so the resolved graph moved",
    );

    // The old edge, reconstructed from the old constants.
    let stale = source_diagnostic_consumers_with_sources(
        &index,
        parent.as_str(),
        Some(ReplacedSources {
            rows: &before.sources,
            constants: &before.path_constants,
        }),
    );
    assert!(
        stale.contains(old_child.as_str()),
        "the child that is no longer sourced must be refreshed: {stale:?}",
    );
    // The new edge, from the current index.
    let fresh = source_diagnostic_consumers(&index, parent.as_str());
    assert!(
        fresh.contains(new_child.as_str()),
        "the newly sourced child must be refreshed: {fresh:?}",
    );
}

/// Source dependencies are directional. A changed child wakes its sourcing
/// ancestors and anything it sources in turn, but not a sibling under the
/// same parent.
#[test]
fn source_fact_changes_select_ancestors_and_descendants_not_siblings() {
    let root = Uri::from_file_path("/proj/root.tcl").unwrap();
    let child = Uri::from_file_path("/proj/child.tcl").unwrap();
    let grandchild = Uri::from_file_path("/proj/grandchild.tcl").unwrap();
    let sibling = Uri::from_file_path("/proj/sibling.tcl").unwrap();
    let index = ws_index(&[
        (&root, "source child.tcl\nsource sibling.tcl\n"),
        (&child, "source grandchild.tcl\npackage require Tk\n"),
        (&grandchild, "winfo exists .x\n"),
        (&sibling, "winfo exists .y\n"),
    ]);
    assert_eq!(
        source_diagnostic_consumers(&index, child.as_str()),
        HashSet::from([root.as_str().to_owned(), grandchild.as_str().to_owned(),]),
    );
}

/// **TP/TN for the hand-built wake set.**  A document that is open but not
/// in the index is woken — the transient open-before-first-publish race —
/// *unless* its backing file was deleted out of band, in which case
/// it is unindexed permanently rather than transiently: the publish path
/// removes rather than replaces it, so it would otherwise be re-woken by
/// every fact-moving publish anywhere in the workspace, for ever, to
/// re-derive the same answer.  An indexed document is never in this set at
/// all — the index-driven consumer queries find those.
#[test]
fn the_wake_set_skips_a_buffer_whose_backing_file_was_deleted() {
    let indexed = Uri::from_file_path("/proj/indexed.tcl").unwrap();
    let just_opened = Uri::from_file_path("/proj/just-opened.tcl").unwrap();
    let orphaned = Uri::from_file_path("/proj/orphaned.tcl").unwrap();
    let index = ws_index(&[(&indexed, "proc helper {a} {}\n")]);

    let mut docs: HashMap<Uri, DocumentState> = HashMap::new();
    for uri in [&indexed, &just_opened, &orphaned] {
        docs.insert(
            uri.clone(),
            DocumentState::new("helper 1\n".to_owned(), "tcl8.6".to_owned()),
        );
    }
    docs.get_mut(&orphaned).unwrap().backing_file_deleted = true;

    let woken: HashSet<String> = unindexed_open_documents(&index, &docs)
        .into_iter()
        .collect();
    assert_eq!(
        woken,
        HashSet::from([just_opened.as_str().to_owned()]),
        "only the transiently unindexed buffer is woken by hand",
    );
}

/// **The other half of the same buffer's story.** Being
/// absent from the index is two facts about an orphan, not one: it must
/// not *contribute* — its path is dead, and re-adding it resurrects the
/// ghost `did_change_watched_files` retired — but it does still *consume*.
/// The buffer is on screen and its squiggles are read, so when a peer
/// changes the arity of a proc it calls, its E002/W123 must be recomputed.
/// Nothing else can name it: the index-driven consumer queries read its
/// call-site records from the index, and it has none there.
///
/// TP: the orphan calls `helper`, whose signature moved. TN: a change to
/// an unrelated name leaves it alone, so this stays a targeted wake and
/// not a blanket re-wake.
#[test]
fn an_orphan_is_still_woken_by_a_signature_it_consumes() {
    let orphan = Uri::from_file_path("/proj/orphan.tcl").unwrap();
    let live = Uri::from_file_path("/proj/live.tcl").unwrap();
    let mut docs: HashMap<Uri, DocumentState> = HashMap::new();
    docs.insert(
        orphan.clone(),
        DocumentState::new("helper 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    docs.insert(
        live.clone(),
        DocumentState::new("helper 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    docs.get_mut(&orphan).unwrap().backing_file_deleted = true;
    let _source_query_slots =
        retain_default_orphan_query_fixture(&orphan, docs.get_mut(&orphan).unwrap());

    let consumed = IndexedDiagnosticChange {
        command_names: HashSet::from(["::helper".to_owned()]),
        source_or_package_changed: false,
        ..IndexedDiagnosticChange::default()
    };
    assert_eq!(
        orphaned_fact_consumers(&docs, &consumed),
        vec![orphan.as_str().to_owned()],
        "the orphan calls `helper`, so a change to its signature reaches it",
    );

    let unrelated = IndexedDiagnosticChange {
        command_names: HashSet::from(["::unrelated".to_owned()]),
        source_or_package_changed: false,
        ..IndexedDiagnosticChange::default()
    };
    assert!(
        orphaned_fact_consumers(&docs, &unrelated).is_empty(),
        "a name the orphan never mentions must not wake it",
    );

    // A source-graph or package move can change what the orphan's own
    // calls resolve to without renaming anything, so it is not filterable
    // by name.
    let graph_moved = IndexedDiagnosticChange {
        command_names: HashSet::new(),
        source_or_package_changed: true,
        ..IndexedDiagnosticChange::default()
    };
    assert_eq!(
        orphaned_fact_consumers(&docs, &graph_moved),
        vec![orphan.as_str().to_owned()],
    );
}

/// The same defect stated where it bites, over the **union** of every
/// consumer set a publish builds. `live.tcl` changes
/// `helper`'s arity; the orphan calls `helper 1` and is on screen with a
/// now-wrong E002. It is absent from the index, so
/// [`command_diagnostic_consumers`] cannot see its call site, and it is
/// excluded from [`unindexed_open_documents`] — without
/// [`orphaned_fact_consumers`] no set names it and its squiggles stay
/// stale with no signal.
#[test]
fn every_consumer_set_together_still_names_an_orphaned_caller() {
    let live = Uri::from_file_path("/proj/live.tcl").unwrap();
    let orphan = Uri::from_file_path("/proj/orphan.tcl").unwrap();
    // The index holds the publishing document only: the orphan was removed
    // from it by its own publish, exactly as `remove_document` leaves it.
    let index = ws_index(&[(&live, "proc helper {a b} {}\n")]);

    let mut docs: HashMap<Uri, DocumentState> = HashMap::new();
    docs.insert(
        live.clone(),
        DocumentState::new("proc helper {a b} {}\n".to_owned(), "tcl8.6".to_owned()),
    );
    docs.insert(
        orphan.clone(),
        DocumentState::new("helper 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    docs.get_mut(&orphan).unwrap().backing_file_deleted = true;

    let change = IndexedDiagnosticChange {
        command_names: HashSet::from(["::helper".to_owned()]),
        source_or_package_changed: false,
        ..IndexedDiagnosticChange::default()
    };
    let mut consumers = command_diagnostic_consumers(&index, &change.command_names);
    consumers.extend(unindexed_open_documents(&index, &docs));
    consumers.extend(orphaned_fact_consumers(&docs, &change));
    assert!(
        consumers.contains(orphan.as_str()),
        "an on-screen orphan consuming the changed signature must be woken; got {consumers:?}",
    );
}
