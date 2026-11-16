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

//! The command a call settles to: links and aliases, position-sensitive invocations, retired names and the cached derived views.

use super::*;

#[test]
fn literal_colon_import_sources_survive_workspace_ingestion() {
    let source = "namespace eval : {namespace eval src {proc p {} {}; namespace export p}; namespace import src::p; p}\n";
    let analysis = analyse(source);
    let index = WorkspaceIndex::from_documents([("file:///colon-import.tcl", &analysis)]);
    let link = index
        .command_links()
        .find(|link| link.linked_qname == ":::::p")
        .unwrap();
    assert_eq!(link.target_qname, ":::::src::p");
    assert_eq!(link.import_gate.as_ref().unwrap().source_ns, ":::::src");
    assert!(
        index
            .command_links()
            .all(|link| link.target_qname != "::src::p")
    );
    assert!(
        index
            .linked_invocations_of(":::::src::p", "")
            .iter()
            .any(|invocation| invocation.name == "p")
    );
}

#[test]
fn literal_colon_alias_targets_survive_workspace_and_indirection() {
    let source = "proc :target {} {}; interp alias {} :alias {} :target; :alias\n";
    let analysis = analyse(source);
    let index = WorkspaceIndex::from_documents([("file:///colon-alias.tcl", &analysis)]);
    let link = index
        .command_links()
        .find(|link| link.linked_qname == ":::alias")
        .unwrap();
    assert_eq!(link.target_qname, ":::target");
    assert_eq!(
        link.linked_source_name
            .as_ref()
            .unwrap()
            .slot()
            .simple
            .as_bytes(),
        b":alias"
    );
    let selected = link.target_source_name.as_ref().unwrap();
    assert!(selected.slot().namespace.is_root());
    assert_eq!(selected.slot().simple.as_bytes(), b":target");
    assert_eq!(selected.source_spelling(), None);
    assert!(!index.command_link_map().contains_key(":alias"));
    let hop = tcl_compiler::analyser::indirection::walk(
        &analysis,
        ":alias",
        u32::MAX,
        &tcl_syntax::naming::normalise_qualified_name,
    )
    .unwrap();
    assert_eq!(hop.target, ":::target");
    assert_eq!(hop.target_source_name.as_ref(), Some(selected));
    assert_eq!(hop.lookup_spelling(), None);
    assert!(
        index
            .linked_invocations_of(":::target", "")
            .iter()
            .any(|invocation| invocation.name == ":alias")
    );
}

#[test]
fn alias_target_navigation_uses_selected_global_or_caller_context() {
    // native_alias_target_context.tcl: C aliases select ROOT and Jim
    // aliases select LOCAL from the same original relative target operand.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let alias = if engine == "jimtcl" {
            "alias contextual target"
        } else {
            "interp alias {} contextual {} target"
        };
        let source = format!(
            "proc target {{}} {{return ROOT}}; {alias}; namespace eval N {{proc target {{}} {{return LOCAL}}; contextual}}\n"
        );
        let analysis = Analyser::new().analyse(&source, engine).clone();
        let call = analysis
            .command_invocations
            .iter()
            .find(|call| call.name == "contextual" && call.lookup.is_execution_site())
            .unwrap_or_else(|| panic!("{engine}: missing original caller"));
        let reference = call
            .resolved_command_reference
            .as_ref()
            .unwrap_or_else(|| panic!("{engine}: missing positioned caller receipt"));
        let selected = reference
            .linked_definition()
            .or_else(|| reference.definition())
            .unwrap_or_else(|| panic!("{engine}: missing selected target allocation"));
        let target = if engine == "jimtcl" {
            "::N::target"
        } else {
            "::target"
        };
        assert_eq!(selected.allocation().command, target, "{engine}");
        let index = WorkspaceIndex::from_documents([("file:///alias-context.tcl", &analysis)]);
        assert!(
            index
                .linked_invocations_of(target, "")
                .iter()
                .any(|call| call.name == "contextual"),
            "{engine}"
        );
    }
}

#[test]
fn positioned_import_definition_keeps_linked_and_editable_views_separate() {
    let source = "namespace eval origin {proc helper {} {return ORIGIN}; namespace export helper}\nnamespace eval app {namespace import ::origin::helper; helper}\n";
    let analysis = analyse(source);
    let index = WorkspaceIndex::from_documents([("file:///imports.tcl", &analysis)]);
    let call = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "helper")
        .unwrap();
    assert!(call.resolved_definition.is_some());
    assert!(!call.resolved_user_definition);
    assert_eq!(index.linked_invocations_of("::origin::helper", "").len(), 1);
    assert_eq!(
        index.invocations_of("::origin::helper", ""),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
    assert_eq!(
        index.invocations_of("::app::helper", ""),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
}

#[test]
fn positioned_definition_precedes_later_forced_import_assistance() {
    let source = "namespace eval origin {proc helper {} {return ORIGIN}; namespace export helper}\nnamespace eval app {proc helper {} {return LOCAL}; helper; namespace import -force ::origin::helper; helper}\n";
    let analysis = analyse(source);
    let index = WorkspaceIndex::from_documents([("file:///timeline.tcl", &analysis)]);
    assert_eq!(index.invocations_of("::app::helper", "").len(), 1);
    assert_eq!(index.linked_invocations_of("::app::helper", "").len(), 1);
    assert_eq!(index.linked_invocations_of("::origin::helper", "").len(), 1);
    assert_eq!(
        index.invocations_of("::origin::helper", ""),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
}

#[test]
fn indexes_invocation_sites_per_document() {
    // a.tcl defines `helper`; b.tcl calls it twice.
    let a = analyse("proc helper {} {}\n");
    let b = analyse("helper\nhelper\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // From a.tcl's view, the two calls live in b.tcl.
    let calls = index.invocations_of("::helper", "file:///a.tcl");
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(calls.iter().all(|c| c.uri == "file:///b.tcl"));
}

#[test]
fn invocations_of_excludes_current_doc() {
    let a = analyse("proc helper {} {}\nhelper\n");
    let b = analyse("helper\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // Excluding a.tcl leaves only b.tcl's call.
    let calls = index.invocations_of("::helper", "file:///a.tcl");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].uri, "file:///b.tcl");
}

#[test]
fn settled_invocations_keep_a_definition_called_before_its_later_deletion() {
    // TP: the final command table has retired
    // `::foo::bar`, but `foo::caller` demonstrably ran before that
    // retirement.  The analyser's per-site resolution must outrank the
    // final-state name set for this one historical invocation.
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
    let call = u32::try_from(src.find("[bar]").expect("call") + 1).expect("offset");
    let analysis = analyse(src);
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

    assert!(
        !index.defined_command_names(false).contains("foo::bar"),
        "the final-state cache must still hide the retired name",
    );
    assert!(
        index
            .invocations_of("::foo::bar", "")
            .iter()
            .any(|inv| inv.range.start() == call),
        "the earlier call must remain attached to the local definition",
    );
    assert!(
        index
            .invocations_of("::bar", "")
            .iter()
            .all(|inv| inv.range.start() != call),
        "the earlier call must not also settle to the global fallback",
    );
}

#[test]
fn settled_invocations_fall_back_after_a_local_definition_is_deleted() {
    // FN guard: moving the execution point past the deletion changes the
    // same candidate list's winner to the global definition.
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
    let call = u32::try_from(src.find("[bar]").expect("call") + 1).expect("offset");
    let analysis = analyse(src);
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

    assert!(
        index
            .invocations_of("::foo::bar", "")
            .iter()
            .all(|inv| inv.range.start() != call),
        "the deleted local definition must not retain the later call",
    );
    assert!(
        index
            .invocations_of("::bar", "")
            .iter()
            .any(|inv| inv.range.start() == call),
        "the later call must settle to the live global definition",
    );
}

#[test]
fn a_later_alias_does_not_retarget_an_earlier_definition_call() {
    // FP/TN guard for the link-following reference view: final state has
    // re-established `target` as an alias of `replacement`, but the first
    // call ran while `target` still named its original proc.  A final-state
    // link must not rewrite that historical target.
    let src = "proc target {} {}\nproc replacement {} {}\ntarget\nrename target {}\ninterp alias {} target {} replacement\ntarget\n";
    let first = u32::try_from(src.find("\ntarget\n").expect("first call") + 1).expect("offset");
    let second = u32::try_from(src.rfind("\ntarget\n").expect("second call") + 1).expect("offset");
    let analysis = analyse(src);
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

    let original = index.linked_invocations_of("::target", "");
    assert!(original.iter().any(|inv| inv.range.start() == first));
    assert!(original.iter().all(|inv| inv.range.start() != second));
    let replacement = index.linked_invocations_of("::replacement", "");
    assert!(replacement.iter().all(|inv| inv.range.start() != first));
    assert!(replacement.iter().any(|inv| inv.range.start() == second));
}

#[test]
fn settled_class_invocations_are_position_sensitive_across_destruction() {
    // Type guard: classes install ordinary commands and use the same
    // generic per-site definition fact as procs.
    let before_src = "oo::class create Dog {}\nDog new\nrename Dog {}\n";
    let before_call = u32::try_from(before_src.find("Dog new").expect("call")).expect("offset");
    let before = analyse(before_src);
    let before_index = WorkspaceIndex::from_documents([("file:///before.tcl", &before)]);
    assert!(!before_index.defined_command_names(false).contains("Dog"));
    assert!(
        before_index
            .invocations_of("::Dog", "")
            .iter()
            .any(|inv| inv.range.start() == before_call),
        "a class call before destruction still reaches that class command",
    );

    let after_src = "oo::class create Dog {}\nrename Dog {}\nDog new\n";
    let after_call = u32::try_from(after_src.find("Dog new").expect("call")).expect("offset");
    let after = analyse(after_src);
    let after_index = WorkspaceIndex::from_documents([("file:///after.tcl", &after)]);
    assert!(
        after_index
            .invocations_of("::Dog", "")
            .iter()
            .all(|inv| inv.range.start() != after_call),
        "a destroyed class command must not regain the later call",
    );
}

#[test]
fn invocations_of_finds_namespaced_call_all_spellings() {
    // A namespaced proc in a.tcl, called three ways from b.tcl: fully
    // qualified, relative-qualified, and bare from inside the namespace.
    let a = analyse("namespace eval ns {\n    proc helper {} {}\n}\n");
    let b = analyse("::ns::helper\nns::helper\nnamespace eval ns {\n    helper\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    let calls = index.invocations_of("::ns::helper", "file:///a.tcl");
    assert_eq!(calls.len(), 3, "{calls:?}");
}

#[test]
fn invocations_of_resolves_namespace_path_across_files() {
    // A bare call reaches a namespaced proc in
    // *another* file via `namespace path`, while an unrelated file defines
    // the same simple name (which disables the bare-name fallback).
    // The file-local guess settles to `::app::helper` (the caller's
    // namespace), so only the workspace-wide candidate resolution finds it.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let other = analyse("namespace eval ::other { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///other.tcl", &other),
        ("file:///app.tcl", &app),
    ]);
    // The call resolves to `::mymod::helper` via the namespace path.
    let refs = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0].uri, "file:///app.tcl");
}

#[test]
fn invocations_of_does_not_cross_link_the_colliding_namespace() {
    // The same call must NOT be reported as a reference of the *other*
    // same-named proc: `namespace path ::mymod` resolves it to
    // `::mymod::helper`, never `::other::helper`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let other = analyse("namespace eval ::other { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///other.tcl", &other),
        ("file:///app.tcl", &app),
    ]);
    let refs = index.invocations_of("::other::helper", "file:///other.tcl");
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn bare_call_without_path_does_not_reach_unrelated_namespace() {
    // A bare `helper` in `::app` with no `namespace path` and no local
    // `::app::helper` resolves to nothing (real tclsh: invalid command
    // name), so it is a reference of neither namespaced proc.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse("namespace eval ::app {\n    proc run {} { helper }\n}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let refs = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn namespace_import_call_site_references_the_source_command() {
    // `::app` imports `::mymod::helper`, then calls a bare `helper`.  The
    // call names the local imported `::app::helper`, which runs
    // `::mymod::helper` — so it is a reference to the source command.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    // Following the import link, the bare call resolves to the source.
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0].uri, "file:///app.tcl");
    // The direct-only resolver (which rename uses) must NOT rewrite that
    // call: it names the local imported command, not the source.
    assert!(
        index
            .invocations_of("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "direct resolver must not claim the imported call site",
    );
    // The import pattern token is a defining-side reference rename rewrites.
    let spans = index.link_target_spans("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(spans.len(), 1, "{spans:?}");
    assert_eq!(spans[0].0, "file:///app.tcl");
}

#[test]
fn interp_alias_call_site_references_the_target_command() {
    // `a` aliases `::mymod::helper`; a bare `a` call runs the target.  The
    // alias `TARGET` word is itself a first-class invocation (a command
    // prefix), so references see two sites: the `TARGET` word and the `a`
    // call reaching the target through the alias link.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse("interp alias {} a {} ::mymod::helper\na\n");
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(
        refs.iter().any(|r| r.name == "a"),
        "the aliased call should reference the target: {refs:?}",
    );
    // The direct-only resolver (rename) sees just the `TARGET` word, never
    // the `a` call — that call names the alias, which keeps its own name.
    let direct = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(
        direct.iter().all(|r| r.name != "a"),
        "rename must not rewrite the alias call site: {direct:?}",
    );
    // The alias `TARGET` word needs no separate link span — it is already
    // an invocation the ordinary reference/rename path covers.
    assert_eq!(
        index.link_target_spans("::mymod::helper", "file:///mymod.tcl"),
        [] as [(std::string::String, tcl_lexer::Span); 0],
    );
}

#[test]
fn rename_new_name_call_site_references_the_old_command() {
    // `rename ::mymod::helper h` makes `h` run what `::mymod::helper`
    // was. Same shape as the `interp alias` case above: the `OLD` word is
    // itself a first-class invocation, so
    // references see two sites — the `OLD` word and the `h` call
    // reaching the target through the rename link.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse("rename ::mymod::helper h\nh\n");
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(
        refs.iter().any(|r| r.name == "h"),
        "the renamed call should reference the target: {refs:?}",
    );
    assert!(
        refs.iter()
            .any(|r| r.name == "::mymod::helper" && r.uri == "file:///app.tcl"),
        "the rename's own OLD word is a reference too: {refs:?}",
    );
    // The direct-only resolver (rename) sees just the `OLD` word, never
    // the `h` call — that call names the local renamed alias, which
    // keeps its own name.
    let direct = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(
        direct.iter().all(|r| r.name != "h"),
        "rename must not rewrite the renamed-to call site: {direct:?}",
    );
    // The `OLD` word needs no separate link span — it is already an
    // invocation the ordinary reference/rename path covers.
    assert_eq!(
        index.link_target_spans("::mymod::helper", "file:///mymod.tcl"),
        [] as [(std::string::String, tcl_lexer::Span); 0],
    );
}

#[test]
fn resolve_command_target_follows_a_chain_and_leaves_plain_names() {
    // `b` aliases `a`, `a` aliases `::mymod::helper`: `b` ultimately runs
    // the source.  A name with no link is returned unchanged.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse("interp alias {} a {} ::mymod::helper\ninterp alias {} b {} a\n");
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(index.resolve_command_target("::b"), "::mymod::helper");
    assert_eq!(index.resolve_command_target("::a"), "::mymod::helper");
    assert_eq!(
        index.resolve_command_target("::mymod::helper"),
        "::mymod::helper"
    );
    // A bare call through the two-hop alias still resolves to the source.
    let app2 = analyse("interp alias {} a {} ::mymod::helper\ninterp alias {} b {} a\nb\n");
    let index2 =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app2)]);
    let refs = index2.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert!(
        refs.iter().any(|r| r.name == "b"),
        "two-hop aliased call should reference the source: {refs:?}",
    );
}

#[test]
pub(super) fn glob_import_introduces_no_command_link() {
    // `namespace import ::mymod::*` names no single command, so it must
    // not manufacture a link that a bare call could resolve through.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    // A glob import records no `::app::helper` link, so the bare call does
    // not resolve to the source through a (non-existent) link.
    assert!(
        index
            .link_target_spans("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "glob import should record no link span",
    );
}

#[test]
fn wildcard_import_call_site_is_a_reference_to_the_source_command() {
    // TP — `linked_invocations_of` (find-references' cross-document
    // mechanism) must reach the bare call site through a wildcard
    // import exactly like it already does for an exact import
    // (`namespace_import_call_site_references_the_source_command`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0].uri, "file:///app.tcl");
    assert_eq!(refs[0].name, "helper");
}

#[test]
fn wildcard_import_unexported_sibling_call_site_is_not_a_reference() {
    // FP guard — the call site for an unexported sibling must not
    // surface as a reference to it either.
    let mymod = analyse(
        "namespace eval ::mymod {\n    proc helper {} {}\n    proc other {} {}\n    namespace export helper\n}\n",
    );
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { other }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let refs = index.linked_invocations_of("::mymod::other", "file:///mymod.tcl");
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn oo_forward_target_is_a_reference_to_the_command() {
    // A `forward` method delegates to `::logger::write`; that `TARGET` word
    // is a reference to the command, so finding references (and rename) of
    // `::logger::write` must include it — like a direct call.
    let logger = analyse("namespace eval ::logger { proc write {} {} }\n");
    let widget = analyse("oo::class create ::Widget {\n    forward log ::logger::write\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///logger.tcl", &logger),
        ("file:///widget.tcl", &widget),
    ]);
    let refs = index.invocations_of("::logger::write", "file:///logger.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0].uri, "file:///widget.tcl");
}

#[test]
fn workspace_command_exists_covers_procs_and_classes() {
    let a = analyse("namespace eval ns { proc p {} {} }\noo::class create ::C {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert!(index.workspace_command_exists("::ns::p"));
    assert!(index.workspace_command_exists("ns::p")); // leading `::` optional
    assert!(index.workspace_command_exists("::C"));
    assert!(!index.workspace_command_exists("::ns::missing"));
}

#[test]
fn nested_proc_flagged_and_workspace_command_exists_for_call_excludes_it_from_a_builtin() {
    // TP — a `proc ::set {...}` written inside another proc's body (the
    // "rename the builtin away, install a shadow, restore it" idiom)
    // must be recorded as `nested`, and must not count as "::set exists
    // in the workspace" once a builtin is in play — the cross-file twin
    // of `resolve_called_proc`'s same-file gate.
    let a = analyse("proc outer {} {\n    proc ::set {v val} {}\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let set_proc = index
        .procs()
        .find(|p| p.qualified_name == "::set")
        .expect("nested ::set proc indexed");
    assert!(
        set_proc.nested,
        "the shadow proc must be recorded as nested"
    );
    assert!(
        index.workspace_command_exists("::set"),
        "the unconditional existence check (no builtin gate) still finds it",
    );
    assert!(
        !index.workspace_command_exists_for_call("::set", true),
        "a nested shadow of a real builtin must not count as existing \
             for call-target resolution",
    );
    assert!(
        index.workspace_command_exists_for_call("::set", false),
        "with no colliding builtin, the nested definition still counts \
             (e.g. `namespace which -command` probes, W120 existence checks)",
    );
}

#[test]
fn top_level_proc_workspace_command_exists_for_call_regardless_of_builtin() {
    // TN — an *unnested* (top-level) proc named after
    // a builtin unconditionally overrides it for the rest of the file,
    // exactly like real Tcl's `proc puts {args} {...}`; it must keep
    // counting as existing even when a builtin of the same name is
    // known, unlike the nested case above.
    let a = analyse("proc ::puts {args} {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let puts_proc = index
        .procs()
        .find(|p| p.qualified_name == "::puts")
        .expect("top-level ::puts proc indexed");
    assert!(!puts_proc.nested, "a top-level proc is never nested");
    assert!(index.workspace_command_exists_for_call("::puts", true));
}

#[test]
fn command_retirement_hides_the_vacated_name_but_keeps_the_rename_target() {
    // TP/TN, Tcl 9.0.4: `rename helper moved` transfers one command
    // object.  The old direct name is gone, while the new link remains
    // callable.  The workspace view must not resurrect `helper` from its
    // historical proc declaration when a second document asks about it.
    let lifecycle = analyse("proc ::helper {} {}\nrename ::helper ::moved\n");
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
    assert!(
        !index.workspace_command_exists_for_call("::helper", false),
        "the name vacated by rename is not callable",
    );
    assert!(
        index.workspace_command_exists_for_call("::moved", false),
        "the rename target keeps the command object",
    );
    assert!(
        index.live_procs().all(|proc_def| proc_def.name != "helper"),
        "the direct command view must not expose the vacated name",
    );
    assert!(
        !index.command_names().contains("helper"),
        "the W123 cache must be built from live definitions",
    );
    assert!(
        !index.defined_command_names(false).contains("helper"),
        "the settled-target cache must not retain the vacated source name",
    );
    assert!(
        index.defined_command_names(true).contains("moved"),
        "the link-aware settled-target cache keeps the rename destination",
    );
}

#[test]
fn later_redefinition_revives_a_retired_command_name() {
    // FN guard: retirement is ordered within its document, rather than a
    // permanent workspace tombstone.  Tcl permits a fresh proc after a
    // rename, and both the moved command and the new definition live.
    let lifecycle = analyse(
        "proc ::helper {} {return old}\nrename ::helper ::moved\nproc ::helper {} {return new}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
    assert!(index.workspace_command_exists_for_call("::helper", false));
    assert!(index.workspace_command_exists_for_call("::moved", false));
    assert_eq!(index.live_procs().filter(|p| p.name == "helper").count(), 1);
}

#[test]
fn class_command_retirement_is_shared_by_command_and_class_lookups() {
    // TP: TclOO class creation installs a command.  Deleting that command
    // makes `Dog new` unknown; it cannot remain a class provider merely
    // because the class body was recorded earlier in the document.
    let lifecycle = analyse("oo::class create ::Dog {}\nrename ::Dog {}\n");
    let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
    assert!(!index.workspace_command_exists_for_call("::Dog", false));
    assert!(
        index
            .live_classes()
            .all(|class_def| class_def.name != "Dog")
    );
    assert!(!index.command_names().contains("Dog"));
    assert!(!index.defined_command_names(false).contains("Dog"));
}

#[test]
fn top_level_alias_workspace_command_exists_for_call_regardless_of_builtin() {
    // TP — `workspace_command_exists_for_call`'s `has_builtin` branch drops
    // *every* `command_links` entry (aliases / renames / imports), not
    // just conditional ones, so a permanent top-level `interp alias {}
    // set {} ::my_set` — exactly like a top-level `proc set` — must keep
    // counting as "::set exists" even with a same-named builtin in play.
    let a = analyse("proc my_set {args} {}\ninterp alias {} set {} ::my_set\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let link = index
        .command_links()
        .find(|l| unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == "set")
        .expect("top-level alias link indexed");
    assert!(!link.nested, "a top-level alias is never nested");
    assert!(
        index.workspace_command_exists_for_call("::set", true),
        "an unconditional alias of a builtin name must still count as existing",
    );
}

#[test]
fn nested_alias_workspace_command_exists_for_call_excludes_it_from_a_builtin() {
    // TP — the link-kind twin of
    // `nested_proc_flagged_and_workspace_command_exists_for_call_excludes_it_from_a_builtin`:
    // an `interp alias` written inside a proc body only takes effect
    // while that proc is running, so it must not permanently count as
    // "::set exists" once a same-named builtin is in play.
    let a = analyse(
        "proc my_set {args} {}\nproc withShadow {} {\n    interp alias {} set {} ::my_set\n}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let link = index
        .command_links()
        .find(|l| unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == "set")
        .expect("nested alias link indexed");
    assert!(
        link.nested,
        "the alias inside withShadow must be recorded as nested"
    );
    assert!(
        !index.workspace_command_exists_for_call("::set", true),
        "a nested alias of a real builtin must not count as existing \
             for call-target resolution",
    );
    assert!(
        index.workspace_command_exists_for_call("::set", false),
        "with no colliding builtin, the nested alias still counts",
    );
}

/// The derived command-name set is cached between mutations and rebuilt
/// after one — the bound that keeps the cross-file unknown-command check
/// off the per-request cost curve.
#[test]
fn command_names_are_cached_until_the_index_changes() {
    let a = analyse("proc ::alpha {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let first = index.command_names();
    assert!(first.contains("alpha"), "{first:?}");
    assert!(
        Arc::ptr_eq(&first, &index.command_names()),
        "an unchanged index must serve the cache, not rebuild it",
    );

    let b = analyse("oo::class create ::Beta {}\n");
    index.add_document("file:///b.tcl", &b);
    let third = index.command_names();
    assert!(
        !Arc::ptr_eq(&first, &third),
        "indexing a document must drop the cache",
    );
    assert!(third.contains("Beta"), "{third:?}");

    // A clone starts with a fresh (empty) cache and rebuilds identically.
    let cloned = index.clone();
    assert_eq!(*cloned.command_names(), *third);
}

/// `defined_command_names` caches its `HashSet` — one slot per
/// `include_links` value — and drops it only on a mutation, same
/// `Arc::ptr_eq` proof as [`command_names_are_cached_until_the_index_changes`].
#[test]
fn defined_command_names_are_cached_per_generation() {
    let a = analyse("proc ::alpha {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let first = index.defined_command_names(false);
    assert!(first.contains("alpha"), "{first:?}");
    assert!(
        Arc::ptr_eq(&first, &index.defined_command_names(false)),
        "an unchanged index must serve the cache, not rebuild it",
    );
    // The `include_links` variants are cached independently: reading one
    // must not populate (or invalidate) the other's slot.
    let with_links_first = index.defined_command_names(true);
    assert!(
        !Arc::ptr_eq(&first, &with_links_first),
        "the two `include_links` slots are distinct caches",
    );
    assert!(
        Arc::ptr_eq(&with_links_first, &index.defined_command_names(true)),
        "the with-links slot must also serve from cache once built",
    );

    let b = analyse("proc ::beta {} {}\n");
    index.add_document("file:///b.tcl", &b);
    let after_mutation = index.defined_command_names(false);
    assert!(
        !Arc::ptr_eq(&first, &after_mutation),
        "indexing a document must drop both cached slots",
    );
    assert!(after_mutation.contains("beta"), "{after_mutation:?}");
}

/// `command_link_map` caches its `HashMap` directly rather than rebuilding
/// it from `live_command_links` per call, same discipline as
/// `command_names`.
#[test]
fn command_link_map_is_cached_per_generation() {
    let a = analyse("proc ::real {} {}\ninterp alias {} ::aliased {} ::real\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let first = index.command_link_map();
    assert_eq!(first.get("aliased").map(String::as_str), Some("real"));
    assert!(
        Arc::ptr_eq(&first, &index.command_link_map()),
        "an unchanged index must serve the cache, not rebuild it",
    );
    index.remove_document("file:///a.tcl");
    assert!(
        !Arc::ptr_eq(&first, &index.command_link_map()),
        "a mutation must drop the cache",
    );
}

/// `invocations_of` (via `settled_sites`) must not re-settle every
/// invocation in the workspace, nor rebuild `WildcardImportIndex`, per
/// call — `code_lenses` calls it once per proc *and* once per class in the
/// document. The settled-target grouping is cached per generation, so
/// repeated `invocations_of` calls against an unchanged index share it.
#[test]
fn settled_target_index_is_cached_until_its_inputs_change() {
    let a = analyse("proc helper {} {}\nproc other {} {}\n");
    let b = analyse("helper\nhelper\nother\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);

    // Querying different targets against the same generation must hit
    // the same cached map, not re-settle per target.
    let helper_calls = index.invocations_of("::helper", "");
    let after_first = index.settled_invocations[0].settled_documents();
    let other_calls = index.invocations_of("::other", "");
    assert_eq!(helper_calls.len(), 2, "{helper_calls:?}");
    assert_eq!(other_calls.len(), 1, "{other_calls:?}");
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        after_first,
        "an unchanged index must serve the settled-target cache",
    );

    // A mutation invalidates the cache and the next call re-settles
    // against the new state.
    let c = analyse("helper\n");
    index.add_document("file:///c.tcl", &c);
    assert_eq!(index.invocations_of("::helper", "").len(), 3);
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        after_first + 3,
        "a command-set change must re-settle all three documents",
    );
}

#[test]
fn body_only_replacement_re_settles_only_the_changed_document() {
    // TP + performance proof.  The replacement changes both
    // call sites in `a.tcl`, so a cache that merely kept the old whole
    // view would be wrong; it must remove that document's old contribution
    // and insert its new one.  `b.tcl` supplies the same command table as
    // before, so it must not be visited again.
    let a = analyse("proc ::caller {} { helper }\nhelper\n");
    let b = analyse("proc ::helper {} {}\nproc ::other {} {}\n");
    let mut index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.invocations_of("::helper", "").len(), 2);
    let cold = index.settled_invocations[0].settled_documents();
    assert_eq!(cold, 2, "the cold read settles both documents");

    let edited_a = analyse("proc ::caller {} { other }\nother\n");
    index.replace_document("file:///a.tcl", &edited_a);
    assert!(
        index.invocations_of("::helper", "").is_empty(),
        "the old contribution must be removed",
    );
    assert_eq!(index.invocations_of("::other", "").len(), 2);
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        cold + 1,
        "a body-only replacement must settle one document, not the workspace",
    );
    let wholesale =
        WorkspaceIndex::from_documents([("file:///a.tcl", &edited_a), ("file:///b.tcl", &b)]);
    for target in ["::helper", "::other"] {
        assert_eq!(
            index.invocations_of(target, "").len(),
            wholesale.invocations_of(target, "").len(),
            "incremental replacement must agree with a wholesale rebuild for {target}",
        );
    }
}

#[test]
fn resolution_input_replacement_re_settles_cross_document_callers() {
    // FN guard: a naive document-local cache would leave `a.tcl` filed
    // under `::helper` after `b.tcl` removes that definition.  Definitions
    // are part of SettlementDependencies, so this intentionally takes the
    // full path and agrees with a fresh workspace build.
    let a = analyse("helper\n");
    let b = analyse("proc ::helper {} {}\n");
    let mut index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.invocations_of("::helper", "").len(), 1);
    let cold = index.settled_invocations[0].settled_documents();

    let edited_b = analyse("# helper was removed\n");
    index.replace_document("file:///b.tcl", &edited_b);
    assert!(
        index.invocations_of("::helper", "").is_empty(),
        "a caller in another document must be re-settled",
    );
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        cold + 2,
        "a command-table change must re-settle every document",
    );
    let wholesale =
        WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &edited_b)]);
    assert_eq!(
        index.invocations_of("::helper", "").len(),
        wholesale.invocations_of("::helper", "").len(),
        "the cross-document invalidation must agree with a wholesale rebuild",
    );
}

#[test]
fn export_replacement_re_settles_wildcard_import_callers() {
    // TN/FP guard for the non-definition half of the dependency surface:
    // an export change can make a different document's wildcard-imported
    // call disappear, even though its own text did not change.
    let lib =
        analyse("namespace eval ::lib {\n    proc helper {} {}\n    namespace export helper\n}\n");
    let app = analyse("namespace import ::lib::*\nhelper\n");
    let mut index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert_eq!(index.linked_invocations_of("::lib::helper", "").len(), 1);
    let cold = index.settled_invocations[1].settled_documents();

    let edited_lib = analyse("namespace eval ::lib {\n    proc helper {} {}\n}\n");
    index.replace_document("file:///lib.tcl", &edited_lib);
    assert!(
        index.linked_invocations_of("::lib::helper", "").is_empty(),
        "the caller must not retain an export which no longer exists",
    );
    assert_eq!(index.settled_invocations[1].settled_documents(), cold + 2);
}

#[test]
fn defined_command_names_are_cached_per_reading_and_dropped_on_mutation() {
    // `workspace_command_exists` asks for this set once per
    // candidate inside the import-chain loop, so it has to be a derived
    // view like `command_names`, not a fresh O(procs + classes + links)
    // walk. The two `include_links` readings are separate views: folding
    // the link names into the direct one would let rename rewrite a call
    // that merely spells an imported name.
    let a = analyse(
        "namespace eval ::lib { proc alpha {} {}\n namespace export alpha }\nnamespace eval ::app { namespace import ::lib::alpha }\n",
    );
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);

    let direct = index.defined_command_names(false);
    let linked = index.defined_command_names(true);
    assert!(direct.contains("lib::alpha"));
    assert!(
        !direct.contains("app::alpha"),
        "the direct reading must not admit a link: {direct:?}",
    );
    assert!(
        linked.contains("app::alpha"),
        "the linked reading must: {linked:?}",
    );
    assert!(
        Arc::ptr_eq(&direct, &index.defined_command_names(false))
            && Arc::ptr_eq(&linked, &index.defined_command_names(true)),
        "an unchanged index must serve both caches, not rebuild them",
    );

    let b = analyse("proc ::beta {} {}\n");
    index.add_document("file:///b.tcl", &b);
    let after = index.defined_command_names(false);
    assert!(
        !Arc::ptr_eq(&direct, &after),
        "indexing a document must drop the cache",
    );
    assert!(after.contains("beta"), "{after:?}");
    // A clone starts with a fresh cache and rebuilds identically.
    assert_eq!(*index.clone().defined_command_names(false), *after);
}
