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

use super::*;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    analyse_as(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn analyse_as(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, dialect.name).clone()
}

#[test]
fn original_declaration_inventory_retains_bytes_and_document_ownership() {
    // Proof naming.workspace.original-declaration-transport:
    // docs/design/analysis/name-resolution-proofs/workspace-original-declaration-transport.md
    // Implementation contract: opaque naming units, declaration metadata
    // and workspace document ownership survive removal of display maps.
    // Native naming outcomes are established by the separate byte probes.
    let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
    let mut analysis = analyse(source);
    assert_eq!(analysis.original_procedure_declarations().count(), 2);
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    let mut index = WorkspaceIndex::from_documents([
        ("file:///first.tcl", &analysis),
        ("file:///second.tcl", &analysis),
    ]);
    let declarations = index.original_procedure_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 4);
    let input = tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declarations[0].1.name_input().clone(),
    );
    let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
        analysis.original_namespace_scope_at(0).unwrap().clone(),
        &input,
    )
    .unwrap();
    let candidates = index.original_procedure_candidates(&lookup);
    assert_eq!(candidates.len(), 2);
    assert!(
        candidates
            .iter()
            .all(|(_, declaration)| { declaration.name_input().bytes() == b"p\xed\xa0\x80" })
    );
    for uri in ["file:///first.tcl", "file:///second.tcl"] {
        let entries = declarations
            .iter()
            .filter(|(owner, _)| *owner == uri)
            .map(|(_, declaration)| *declaration)
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name_input().bytes(), b"p\xed\xa0\x80");
        assert_eq!(entries[1].name_input().bytes(), b"p\xed\xa0\x81");
        assert_ne!(entries[0].name().slot(), entries[1].name().slot());
    }
    assert_eq!(
        index
            .invocations()
            .filter(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
            })
            .count(),
        2
    );
    index.remove_document("file:///first.tcl");
    assert!(
        index
            .original_procedure_declarations()
            .all(|(uri, _)| uri == "file:///second.tcl")
    );
    index.add_document("file:///first.tcl", &analyse("proc plain {} {}"));
    assert_eq!(index.original_procedure_declarations().count(), 3);
    assert!(
        index
            .original_procedure_declarations()
            .filter(|(uri, _)| *uri == "file:///first.tcl")
            .all(|(_, declaration)| declaration.name_input().bytes() == b"plain")
    );
}

#[test]
fn original_requirement_graph_keeps_constraints_preference_and_unknowns() {
    // Implementation contract: naming.workspace.original-package-requirement-transport
    // docs/design/analysis/name-resolution-proofs/workspace-original-package-requirement-transport.md

    use crate::package_resolver::{PackagePrefer, PackageRequirementAdviceKey};
    let source = "package require -exact p\\uD800 1.2\npackage prefer latest\npackage require p\\uD800 1.0 2.0\npackage require $computed\n";
    let analysis = analyse(source);
    let mut index = WorkspaceIndex::from_documents([("file:///library.tcl", &analysis)]);
    let requirements =
        index.original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Stable);
    assert_eq!(requirements.len(), 3);
    assert_eq!(
        requirements[0].key().original().unwrap().bytes(),
        b"p\xed\xa0\x80"
    );
    assert_eq!(requirements[0].requirements(), &["1.2"]);
    assert!(requirements[0].exact());
    assert_eq!(requirements[0].prefer(), PackagePrefer::Stable);
    assert_eq!(requirements[1].key(), requirements[0].key());
    assert_eq!(requirements[1].requirements(), &["1.0", "2.0"]);
    assert!(!requirements[1].exact());
    assert_eq!(requirements[1].prefer(), PackagePrefer::Latest);
    assert_eq!(requirements[2].key(), &PackageRequirementAdviceKey::Unknown);
    let edges = vec![
        (
            "file:///library.tcl".to_owned(),
            "file:///consumer.tcl".to_owned(),
        ),
        (
            "file:///consumer.tcl".to_owned(),
            "file:///library.tcl".to_owned(),
        ),
    ];
    let records =
        std::collections::HashMap::from([("file:///library.tcl".to_owned(), requirements.clone())]);
    assert_eq!(
        crate::source_graph::ancestor_requirements("file:///consumer.tcl", &edges, &records),
        requirements
    );
    assert!(
        index
            .original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Latest)
            .iter()
            .all(|required| required.prefer() == PackagePrefer::Latest)
    );
    index.remove_document("file:///library.tcl");
    assert_eq!(
        index.original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Stable),
        [] as [crate::package_resolver::PackageRequirementAdvice; 0]
    );
}

#[test]
fn literal_colon_namespace_keys_keep_definitions_and_variables_distinct() {
    let nested = analyse("namespace eval : {proc p {} {}; variable v 1}\n");
    let global = analyse("proc :p {} {}\nvariable :v 2\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///nested.tcl", &nested),
        ("file:///global.tcl", &global),
    ]);
    let nested_proc = index.proc_definitions_qualified(":::::p", "");
    let global_proc = index.proc_definitions_qualified(":::p", "");
    assert_eq!(nested_proc.len(), 1);
    assert_eq!(global_proc.len(), 1);
    assert_eq!(nested_proc[0].uri, "file:///nested.tcl");
    assert_eq!(global_proc[0].uri, "file:///global.tcl");
    let nested_var = index.variable_definitions_qualified(":::::v", "");
    assert_eq!(nested_var.len(), 1);
    assert_eq!(nested_var[0].uri, "file:///nested.tcl");
    let global_var = index.variable_definitions_qualified(":::v", "");
    assert_eq!(global_var.len(), 1);
    assert_eq!(global_var[0].uri, "file:///global.tcl");
    assert_eq!(index.documents_in_namespace(":::"), ["file:///nested.tcl"]);
    assert_eq!(index.documents_in_namespace("::"), ["file:///global.tcl"]);
    let namespaces = index.observable_namespaces();
    assert!(namespaces.contains(":"));
    assert!(namespaces.contains(""));
}

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
fn subclass_provided_methods_matches_the_per_class_linearisation() {
    // The bulk fold must agree with the per-class `class_linearisation`
    // walk (the per-class form rebuilds the edge maps every call —
    // O(classes²) — which is what times out the Performance suite).  The
    // fixture
    // covers the shapes the fold has to get right: a mixin-provided
    // template method, an unresolved base, and a private member that
    // must not travel.
    let base = analyse(
        "oo::class create ::Fmt { method run {} { my Render } }\n\
             oo::class create ::Mix { method Render {} { return m } }\n",
    );
    let sub = analyse(
        "oo::class create ::Html { superclass ::Fmt\n    method Render {} { return h } }\n\
             oo::class create ::Md { superclass ::Fmt\n    mixin ::Mix }\n\
             oo::class create ::Secretive { superclass ::Fmt\n\
             \x20   private method Hidden {} { return no } }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///base.tcl", &base), ("file:///sub.tcl", &sub)]);
    let map = index.subclass_provided_methods();
    let fmt = map.get("::Fmt").expect("::Fmt has descendants");
    assert!(fmt.contains("Render"), "subclass-written: {fmt:?}");
    assert!(
        fmt.contains("run"),
        "the base's own method rides along harmlessly: {fmt:?}"
    );
    assert!(
        !fmt.contains("Hidden"),
        "private members never travel: {fmt:?}"
    );
    // The mixin is an ancestor on ::Md's linearisation, so it gains
    // ::Md's dispatchable surface exactly as a superclass would.
    let mix = map.get("::Mix").expect("::Mix hosts a mixin consumer");
    assert!(mix.contains("run"), "{mix:?}");
    // Agreement with the per-class walk: every key's entry is what the
    // slow form computes for the same class set.
    for (ancestor, methods) in &map {
        let mut slow: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for d in index.live_classes().map(|c| c.qualified_name.clone()) {
            let lin = index.class_linearisation(&d);
            if lin.iter().all(|c| c == &d) || !lin.iter().any(|c| c == ancestor) || &d == ancestor {
                continue;
            }
            for x in &lin {
                slow.extend(
                    index
                        .effective_members(x)
                        .into_iter()
                        .filter(|m| {
                            m.method.kind != MethodKind::ClassMethod.as_str() && !m.method.private
                        })
                        .map(|m| m.name.to_owned()),
                );
            }
        }
        assert_eq!(methods, &slow, "bulk and per-class disagree on {ancestor}");
    }
}

#[test]
fn a_receivers_export_stub_decides_the_inherited_members_visibility() {
    // `export` / `unexport` accept a name their class does
    // not define, creating a body-less table entry whose only content is
    // the flag; the flag comes from the most specific spine class that
    // *mentions* the member while the implementation still comes from the
    // first class that declares a body.  Oracle, byte-identical on tclsh
    // 8.6.16 and 9.0.4:
    //   oo::class create ::Base  { method tick {} { return base } }
    //   oo::class create ::Child { superclass ::Base }
    //   oo::define ::Child { unexport tick }
    //   [::Child new] tick  ->  unknown method "tick": must be destroy
    //   [::Base  new] tick  ->  base
    let base = analyse("oo::class create ::Base { method tick {} { return 1 } }\n");
    let child = analyse(
        "oo::class create ::Child { superclass ::Base }\noo::define ::Child { unexport tick }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    assert!(
        index
            .method_dispatch_chain("::Child", "tick", MethodAccess::External)
            .is_empty(),
        "the receiver's own unexport suppresses the inherited implementation",
    );
    // `my` ignores the export flag entirely — it still reaches `Base`.
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "tick", MethodAccess::Internal)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///base.tcl"],
        "internal dispatch is not gated on the export flag",
    );
    // TN: the provider's own receiver is untouched by the subclass's flip.
    assert_eq!(
        index
            .method_dispatch_chain("::Base", "tick", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///base.tcl"],
    );
}

#[test]
fn a_receivers_export_stub_revives_an_unexported_inherited_member() {
    // The mirror direction, which a provider-only reading gets wrong the
    // other way.  Oracle, byte-identical on tclsh 8.6.16 and 9.0.4:
    //   oo::class create ::Base  { method tock {} { return b3 } ; unexport tock }
    //   oo::class create ::Child { superclass ::Base ; export tock }
    //   [::Child new] tock  ->  b3
    //   [::Base  new] tock  ->  unknown method "tock": must be destroy
    let base =
        analyse("oo::class create ::Base { method tock {} { return 1 }\n    unexport tock\n}\n");
    let child = analyse("oo::class create ::Child { superclass ::Base\n    export tock\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "tock", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///base.tcl"],
        "the receiver's export revives the inherited implementation",
    );
    assert!(
        index
            .method_dispatch_chain("::Base", "tock", MethodAccess::External)
            .is_empty(),
        "the provider itself stays unexported",
    );
}

#[test]
fn a_mixins_unexport_does_not_suppress_the_spine_provider() {
    // C enters each mixin with a fresh copy of the dispatch flags, so a
    // mixin's `unexport` empties only its own branch.  Oracle,
    // byte-identical on tclsh 8.6.16 and 9.0.4:
    //   oo::class create ::Mix { method tick {} { return mix } ; unexport tick }
    //   oo::class create ::Child { superclass ::Base ; mixin ::Mix }
    //   [::Child new] tick  ->  base
    let base = analyse("oo::class create ::Base { method tick {} { return 1 } }\n");
    let mix =
        analyse("oo::class create ::Mix { method tick {} { return 2 }\n    unexport tick\n}\n");
    let child = analyse("oo::class create ::Child { superclass ::Base\n    mixin ::Mix\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///mix.tcl", &mix),
        ("file:///child.tcl", &child),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "tick", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///base.tcl"],
        "the mixin's unexport must not suppress the superclass spine",
    );
}

#[test]
fn a_mixin_branch_decides_its_own_providers_visibility_across_files() {
    // A mixin that inherits the member from its
    // own superclass and unexports the name empties *that branch* only —
    // C enters each mixin with a fresh copy of the dispatch flags — so the
    // receiver's spine still answers.  Oracle, byte-identical on tclsh
    // 8.6.16 and 9.0.4:
    //   oo::class create ::MChild { superclass ::MBase } ; unexport m
    //   oo::class create ::D { superclass ::A ; mixin ::MChild }
    //   [::D new] m  ->  A-m   (not MBase-m)
    let a = analyse("oo::class create ::A { method m {} { return 1 } }\n");
    let mbase = analyse("oo::class create ::MBase { method m {} { return 2 } }\n");
    let mchild = analyse("oo::class create ::MChild { superclass ::MBase\n    unexport m\n}\n");
    let d = analyse("oo::class create ::D { superclass ::A\n    mixin ::MChild\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///mbase.tcl", &mbase),
        ("file:///mchild.tcl", &mchild),
        ("file:///d.tcl", &d),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::D", "m", MethodAccess::External)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///a.tcl"),
        "the suppressed mixin branch must not answer; the spine must",
    );
    // TN for the permission the family pass reads: `::D` dispatches into
    // `::A`'s family, not `::MBase`'s.
    assert!(
        index
            .external_dispatch_receivers(["::D"], &["::A"], "m", false)
            .contains("::D"),
    );
    assert!(
        index
            .external_dispatch_receivers(["::D"], &["::MBase"], "m", false)
            .is_empty(),
    );
}

#[test]
fn a_live_mixin_branch_outranks_the_spine_across_files() {
    // The control: with the branch left public the mixin's inherited
    // `::MBase::m` is what `[::D new] m` enters (tclsh 8.6.16 / 9.0.4 ->
    // MBase-m), so the capture belongs to *its* family instead.
    let a = analyse("oo::class create ::A { method m {} { return 1 } }\n");
    let mbase = analyse("oo::class create ::MBase { method m {} { return 2 } }\n");
    let mpub = analyse("oo::class create ::MPub { superclass ::MBase }\n");
    let d = analyse("oo::class create ::D { superclass ::A\n    mixin ::MPub\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///mbase.tcl", &mbase),
        ("file:///mpub.tcl", &mpub),
        ("file:///d.tcl", &d),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::D", "m", MethodAccess::External)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///mbase.tcl"),
    );
    assert!(
        index
            .external_dispatch_receivers(["::D"], &["::A"], "m", false)
            .is_empty(),
        "a receiver dispatching into another family must not join this one",
    );
}

#[test]
fn a_mixin_branch_revives_its_own_bases_unexported_member_across_files() {
    // The mirror direction inside a branch: `::NBase` unexports its own
    // `n`, the mixin `::NChild` exports the inherited name, and tclsh
    // 8.6.16 / 9.0.4 run `::NBase`'s body ahead of the spine's public
    // `::A2::n`.
    let a2 = analyse("oo::class create ::A2 { method n {} { return 1 } }\n");
    let nbase = analyse("oo::class create ::NBase { method n {} { return 2 }\n    unexport n\n}\n");
    let nchild = analyse("oo::class create ::NChild { superclass ::NBase\n    export n\n}\n");
    let d = analyse("oo::class create ::D { superclass ::A2\n    mixin ::NChild\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a2.tcl", &a2),
        ("file:///nbase.tcl", &nbase),
        ("file:///nchild.tcl", &nchild),
        ("file:///d.tcl", &d),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::D", "n", MethodAccess::External)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///nbase.tcl"),
        "the branch's own export revives its base's body",
    );
}

#[test]
fn cross_file_define_stub_retraction_removes_the_method_from_dispatch() {
    // A cross-file `oo::define` stub is already an
    // additive channel — a `method extra` written in b.tcl dispatches on a
    // class created in a.tcl — so a `deletemethod` written the same way has
    // to travel too, or the workspace advertises a method that sourcing the
    // extension deletes. Oracle, byte-identical on tclsh 9.0.4 / 8.6.14:
    //   oo::class create ::C { method m {} {…} }
    //   oo::define ::C { method extra {} {…} }
    //   oo::define ::C { deletemethod m }
    //   info class methods ::C  ->  extra
    //   [::C new] m             ->  unknown method "m": must be destroy or extra
    //   [::C new] extra         ->  2
    let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
    let b = analyse("oo::define ::C { method extra {} { return 2 } }\n");
    let d = analyse("oo::define ::C { deletemethod m }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///d.tcl", &d),
    ]);
    // TP: the additive half still resolves (the channel this rides on).
    let extra = index.method_dispatch_chain("::C", "extra", MethodAccess::External);
    assert_eq!(
        extra.iter().map(|c| c.uri.as_str()).collect::<Vec<_>>(),
        ["file:///b.tcl"],
        "the cross-file additive channel must keep working",
    );
    // TP: the retraction now travels the same way.
    assert!(
        index
            .method_dispatch_chain("::C", "m", MethodAccess::External)
            .is_empty(),
        "a cross-file `deletemethod` must remove the method from dispatch",
    );
    // …and from internal (`my`) dispatch too — the member is gone, not
    // merely unexported.
    assert_eq!(
        index.method_dispatch_chain("::C", "m", MethodAccess::Internal),
        [] as [&crate::workspace_index::WorkspaceClass; 0],
    );
}

#[test]
fn a_same_document_retraction_does_not_cancel_another_document() {
    // TN for the tombstone's scope. A retraction of a member the *same*
    // document declares is applied locally and leaves no tombstone, so a
    // second document that declares the name keeps it — cross-file order is
    // not knowable, and suppressing it would be an unsupported guess.
    let a = analyse(
        "oo::class create ::C {}\noo::define ::C { method m {} { return 1 }\n deletemethod m }\n",
    );
    let b = analyse("oo::define ::C { method m {} { return 2 } }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .method_dispatch_chain("::C", "m", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///b.tcl"],
    );
}

#[test]
fn a_cross_file_retraction_does_not_reach_an_inherited_provider() {
    // TN. Deleting a subclass's own override falls back to the superclass's
    // method rather than erasing the name — and deleting a member the
    // subclass never declared is a hard error, so a retraction never
    // crosses a class boundary. Oracle (9.0.4 / 8.6.14, identical):
    //   oo::class create B { method m {} {return base-m} }
    //   oo::class create D { superclass B; method m {} {return derived-m} }
    //   oo::define D { deletemethod m } ; [D new] m  ->  base-m
    //   oo::define D2 { deletemethod m }             ->  method m does not exist
    let b = analyse("oo::class create ::B { method m {} { return 1 } }\n");
    let d = analyse("oo::class create ::D {\n superclass ::B\n method m {} { return 2 }\n}\n");
    let x = analyse("oo::define ::D { deletemethod m }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///b.tcl", &b),
        ("file:///d.tcl", &d),
        ("file:///x.tcl", &x),
    ]);
    assert_eq!(
        index
            .method_dispatch_chain("::D", "m", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///b.tcl"],
        "the superclass's own `m` must still provide the dispatch entry",
    );
}

#[test]
fn class_member_suppression_reads_the_same_chain_as_resolution() {
    // The predicate the in-document tier consults before
    // answering for a class its own document declares.  Oracle for the
    // suppressing shape (tclsh 9.0.4 / 8.6.14, byte-identical):
    //   a.tcl  oo::class create ::C { self { method cm {} { return 1 } } }
    //   b.tcl  oo::define ::C { self unexport cm }
    //   ::C cm  ->  unknown method "cm": must be create, destroy or new
    let a = analyse("oo::class create ::C { self { method cm {} { return 1 } } }\n");
    let b = analyse("oo::define ::C { self unexport cm }\n");
    let e = analyse("oo::define ::C { self export cm }\n");
    let d = analyse("oo::define ::C { self deletemethod cm }\n");

    // TP: a cross-file `self unexport` suppresses the external dispatch.
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));
    // …but the member is only unexported: internal dispatch still lands.
    assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::Internal));

    // TP: a cross-file `self deletemethod` suppresses both accesses — the
    // member is gone, not merely unexported.
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///d.tcl", &d)]);
    assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));
    assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::Internal));

    // TN (abstention): with only the declaring record in view there is no
    // suppressing evidence, and the in-document answer must stand.
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));

    // TN (export-wins union): an exporting record anywhere keeps the
    // member dispatchable — the standing unordered-cross-file caveat.
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///e.tcl", &e),
    ]);
    assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));

    // TN (abstention): a class the index holds no record of yields no
    // suppression evidence at all.
    assert!(!index.class_member_dispatch_suppressed("::Ghost", "cm", MethodAccess::External));
}

#[test]
fn a_cross_file_self_unexport_removes_the_member_from_class_dispatch() {
    // TP — the whole point of the class-side channel. Oracle,
    // byte-identical on tclsh 9.0.4 and 8.6.14:
    //   a.tcl  oo::class create ::C { self { method cm {} { return cm } } }
    //          ::C cm  ->  cm
    //   b.tcl  oo::define ::C { self unexport cm }
    //          ::C cm  ->  unknown method "cm": must be create, destroy or new
    //          info object methods ::C -all -private  ->  … cm …
    // Without a class-side channel the flip has nowhere to travel, b.tcl's
    // `self unexport` is invisible and `::C cm` still resolves.
    let a = analyse("oo::class create ::C { self { method cm {} { return 1 } } }\n");
    let b = analyse("oo::define ::C { self unexport cm }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .class_method_dispatch_chain("::C", "cm", MethodAccess::External)
            .is_empty(),
        "a cross-file `self unexport` must remove the member from class dispatch",
    );
    // …but the member is only *unexported*, not gone: an internal (`my`)
    // dispatch still reaches it, exactly as `info object methods -all
    // -private` still lists it.
    assert_eq!(
        index
            .class_method_dispatch_chain("::C", "cm", MethodAccess::Internal)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
    );
}

#[test]
fn a_cross_file_self_export_revives_a_class_side_member() {
    // TP, the other direction. `self export` travels the same way, and the
    // union rule is the class side's too: any record exporting the name
    // keeps it dispatchable, because true cross-file load order is not
    // knowable from the index.
    let a = analyse("oo::class create ::C { self { method Cm {} { return 1 }\n unexport Cm } }\n");
    let b = analyse("oo::define ::C { self export Cm }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .class_method_dispatch_chain("::C", "Cm", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
    );
}

#[test]
fn the_two_sides_visibility_channels_never_cross() {
    // TN (CRITICAL FP guard). A class that defines the
    // same name on both sides must have each side answer for itself:
    //   a.tcl  oo::class create ::C { method m {} {…}
    //                                 self { method m {} {…} } }
    //   b.tcl  oo::define ::C { self unexport m }
    //   ::C m        ->  unknown method "m"     (class side flipped)
    //   [::C new] m  ->  inst-m                 (instance side untouched)
    let a = analyse(
        "oo::class create ::C { method m {} { return 1 }\n\
             self { method m {} { return 2 } } }\n",
    );
    let b = analyse("oo::define ::C { self unexport m }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .class_method_dispatch_chain("::C", "m", MethodAccess::External)
            .is_empty(),
        "the class side is unexported",
    );
    assert_eq!(
        index
            .method_dispatch_chain("::C", "m", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
        "the instance side must be untouched by a `self unexport`",
    );
}

#[test]
fn an_unwrapped_cross_file_unexport_leaves_the_class_side_dispatchable() {
    // TN, the mirror. Oracle: `oo::class create E2 { self { method onlyclass
    // {} {…} } }` then `oo::define E2 { unexport onlyclass }` is a silent
    // no-op and `::E2 onlyclass` still answers (9.0.4 / 8.6.14).
    let a = analyse("oo::class create ::C { self { method m {} { return 1 } } }\n");
    let b = analyse("oo::define ::C { unexport m }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .class_method_dispatch_chain("::C", "m", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
    );
}

#[test]
fn a_cross_file_self_deletemethod_empties_only_the_class_chain() {
    // TP + TN for the sided tombstone, which the class-side chain now reads
    // as its own. `self deletemethod m` in b.tcl removes the class-object
    // side's `m` and leaves an identically-named instance method alone.
    let a = analyse(
        "oo::class create ::C { method m {} { return 1 }\n\
             self { method m {} { return 2 } } }\n",
    );
    let b = analyse("oo::define ::C { self deletemethod m }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .class_method_dispatch_chain("::C", "m", MethodAccess::Internal)
            .is_empty(),
        "the class-side member is gone, not merely unexported",
    );
    assert!(
        !index
            .method_dispatch_chain("::C", "m", MethodAccess::External)
            .is_empty(),
        "an unwrapped-side member must survive a `self deletemethod`",
    );
}

#[test]
fn a_cross_file_renamed_member_dispatches_under_its_new_name() {
    // TP — a same-file rename reaching the workspace. The rename happens inside
    // a.tcl, so the moved member is an ordinary indexed declaration by the
    // time b.tcl consumes it — `[::C new] new` really runs the old body
    // (`info class definition ::C new` -> `{} { return 1 }`, 9.0.4/8.6.14).
    let a = analyse("oo::class create ::C { method old {} { return 1 }\n renamemethod old new }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert_eq!(
        index
            .method_dispatch_chain("::C", "new", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
    );
    assert!(
        index
            .method_dispatch_chain("::C", "old", MethodAccess::External)
            .is_empty(),
        "the source name must not survive the move",
    );
}

/// TP: the `renamemethod` sits in a **cross-file**
/// `oo::define` stub, which has no `MethodDef` of its own to move.  The
/// stub tombstones the source and records the arrival; the workspace join
/// re-keys the defining file's record, so the member dispatches under its
/// new name instead of disappearing.
///
/// tclsh-proof (8.6.14) that this is what sourcing both files does:
///
/// ```tcl
/// oo::class create ::C { method old {} { return OLDBODY } }
/// oo::define ::C { renamemethod old new }
/// info class methods ::C   ;# -> new
/// [::C new] new            ;# -> OLDBODY
/// [::C new] old            ;# -> unknown method "old"
/// ```
#[test]
fn a_cross_file_stub_renamemethod_records_the_arrival_name() {
    let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
    let b = analyse("oo::define ::C { renamemethod old new }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .method_dispatch_chain("::C", "new", MethodAccess::External)
            .iter()
            .map(|c| c.uri.as_str())
            .collect::<Vec<_>>(),
        ["file:///a.tcl"],
        "the arrival resolves to the record holding the body",
    );
    assert!(
        index
            .method_dispatch_chain("::C", "old", MethodAccess::External)
            .is_empty(),
        "the source name must not survive the move",
    );
    // The family resolution sees it too, so rename / references seed from
    // the new name rather than finding nothing.
    assert_eq!(
        index
            .method_override_family("::C", "new")
            .iter()
            .map(|c| c.qualified_name.as_str())
            .collect::<Vec<_>>(),
        ["::C", "::C"],
        "both records of the class are in the family",
    );
}

/// TN: visibility travels with the **body**, not the arrival name's
/// leading-capital default.
///
/// tclsh-proof (8.6.14): `oo::class create ::R4 { method Priv {} {…} }` +
/// `oo::define ::R4 { renamemethod Priv pub }` leaves `info class methods
/// ::R4` empty (the member is still unexported) while `info class methods
/// ::R4 -private` lists `pub`.
#[test]
fn a_cross_file_arrival_keeps_the_sources_visibility() {
    let a = analyse("oo::class create ::C { method Priv {} { return 1 } }\n");
    let b = analyse("oo::define ::C { renamemethod Priv pub }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .method_dispatch_chain("::C", "pub", MethodAccess::External)
            .is_empty(),
        "the moved member is still unexported — the new name's default does not apply",
    );
    assert!(
        !index
            .method_dispatch_chain("::C", "pub", MethodAccess::Internal)
            .is_empty(),
        "…but it is a real member, reachable internally",
    );
}

/// TN: a plain cross-file `deletemethod` records no arrival, so nothing
/// arrives — the tombstone keeps its original meaning.
#[test]
fn a_cross_file_deletemethod_records_no_arrival() {
    let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
    let b = analyse("oo::define ::C { deletemethod m }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.method_dispatch_chain("::C", "m", MethodAccess::External),
        [] as [&crate::workspace_index::WorkspaceClass; 0],
    );
    assert!(
        index.classes().all(|c| !c.arrives_method("m")),
        "a deletion has no destination",
    );
}

/// TN: a **computed** destination (`renamemethod old $new`) names nothing
/// statically, so the move abstains and only the retraction stands.
#[test]
fn a_computed_cross_file_arrival_abstains() {
    let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
    let b = analyse("oo::define ::C { renamemethod old $target }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .classes()
            .all(|c| c.retracted_members.iter().all(|r| r.arrival.is_none())),
        "a computed destination records no arrival",
    );
}

/// TP: **enumeration** joins the arrival channel too.  The defining record
/// still declares the member as `old`, so reading the raw
/// `WorkspaceClass::methods` table directly advertises the pre-rename name
/// even where the resolution join is applied.
///
/// tclsh-proof (8.6.14), sourcing both files:
///
/// ```tcl
/// oo::class create ::C { method old {} { return OLDBODY } }
/// oo::define ::C { renamemethod old new }
/// info class methods ::C   ;# -> new
/// [::C new] old            ;# -> unknown method "old"
/// ```
#[test]
fn a_cross_file_arrival_re_keys_member_enumeration() {
    let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
    let b = analyse("oo::define ::C { renamemethod old new }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .effective_members("::C")
            .iter()
            .map(|em| em.name)
            .collect::<Vec<_>>(),
        ["new"],
        "the fold lists the member under the name the class dispatches it by",
    );
    // The body still lives in a.tcl; only the *name* moved.
    let em = index.effective_members("::C");
    let moved = em.first().expect("one member");
    assert_eq!(moved.declaring.uri, "file:///a.tcl");
    assert_eq!(moved.method.name, "old", "the declaring entry is untouched");
    assert_eq!(
        moved.name_uri, "file:///b.tcl",
        "the arrival word is the moved member's declaration site",
    );
    // …and `workspace/symbol` offers the new name, not the old one.
    let names: Vec<String> = index
        .symbols_matching("", 100)
        .into_iter()
        .filter(|s| s.kind == WorkspaceSymbolKind::Method)
        .map(|s| s.name)
        .collect();
    assert_eq!(
        names,
        ["new"],
        "the picker must not show the pre-rename name"
    );
}

#[test]
fn symbol_exclusions_apply_before_the_limit_and_member_fold() {
    let orphan = analyse("proc orphan_first {} {}\noo::define ::C { renamemethod old arrived }\n");
    let live = analyse("proc live_second {} {}\noo::class create ::C { method old {} {} }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///orphan.tcl", &orphan),
        ("file:///live.tcl", &live),
    ]);

    let capped = index.symbols_matching_excluding("", 1, ["file:///orphan.tcl"]);
    assert_eq!(
        capped.first().map(|symbol| symbol.name.as_str()),
        Some("live_second"),
        "an excluded document must not consume the result capacity",
    );
    let methods = index.symbols_matching_excluding("old", 10, ["file:///orphan.tcl"]);
    assert!(methods.iter().any(|symbol| {
        symbol.kind == WorkspaceSymbolKind::Method
            && symbol.name == "old"
            && symbol.uri == "file:///live.tcl"
    }));
    assert!(
        methods
            .iter()
            .all(|symbol| symbol.uri != "file:///orphan.tcl"),
        "an excluded arrival must neither re-key nor locate a live member",
    );
}

#[test]
fn symbol_exclusions_rebuild_duplicate_retractions_from_live_documents() {
    let class = analyse("oo::class create ::C { method old {} {} }\n");
    let orphan = analyse("oo::define ::C { renamemethod old orphan_arrival }\n");
    let live = analyse("oo::define ::C { renamemethod old live_arrival }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &class),
        ("file:///a-orphan.tcl", &orphan),
        ("file:///b-live.tcl", &live),
    ]);

    let symbols = index.symbols_matching_excluding("arrival", 10, ["file:///a-orphan.tcl"]);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "live_arrival");
    assert_eq!(symbols[0].uri, "file:///b-live.tcl");
    assert!(
        index
            .symbols_matching_excluding("old", 10, ["file:///a-orphan.tcl"])
            .iter()
            .all(|symbol| symbol.kind != WorkspaceSymbolKind::Method),
        "the remaining live retraction must still retire the old member name",
    );
}

/// TN: a cross-file `deletemethod` removes the member from
/// enumeration outright — nothing arrives, so nothing is listed.
///
/// tclsh-proof (8.6.14): `oo::class create ::C { method m {} {…} }` +
/// `oo::define ::C { deletemethod m }` leaves `info class methods ::C`
/// empty and `[::C new] m` erroring `unknown method "m"`.
#[test]
fn a_cross_file_deletion_drops_the_member_from_enumeration() {
    let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
    let b = analyse("oo::define ::C { deletemethod m }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index.effective_members("::C").is_empty(),
        "a deleted member is not a member",
    );
    assert!(
        index
            .symbols_matching("m", 100)
            .iter()
            .all(|s| s.kind != WorkspaceSymbolKind::Method),
        "the picker must not offer a deleted member",
    );
}

/// TN: with no retraction anywhere, enumeration is exactly
/// each record's own table — the fold must not perturb the ordinary case,
/// including a member declared by a cross-file `oo::define` stub and one
/// on the class-object side.
#[test]
fn enumeration_without_a_retraction_is_the_declared_table() {
    let a = analyse("oo::class create ::C { method one {} {}\n method two {} {} }\n");
    let b = analyse("oo::define ::C { method three {} {}\n self method cm {} {} }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index
            .effective_members("::C")
            .iter()
            .map(|em| em.name)
            .collect::<Vec<_>>(),
        ["one", "two", "three", "cm"],
        "record order then declaration order, unchanged",
    );
    assert!(
        index
            .effective_members("::C")
            .iter()
            .all(|em| em.name == em.method.name),
        "no rename, so no re-keying",
    );
}

/// TN: a `self renamemethod` moves the **class-object** side only.  An
/// identically-named instance method keeps its own name, which is the same
/// side-scoping the tombstone channel already enforces for resolution.
#[test]
fn a_cross_file_arrival_is_side_scoped_in_enumeration() {
    let a = analyse(
        "oo::class create ::C { method m {} { return 1 }\n self method m {} { return 2 } }\n",
    );
    let b = analyse("oo::define ::C { self renamemethod m cm }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    let mut listed: Vec<(&str, &str)> = index
        .effective_members("::C")
        .iter()
        .map(|em| (em.name, em.method.kind.as_str()))
        .collect();
    listed.sort_unstable();
    assert_eq!(
        listed,
        [("cm", "classmethod"), ("m", "method")],
        "only the class-object member moved",
    );
}

#[test]
fn a_superseded_export_does_not_outrank_the_last_unexport() {
    // `method m {} {}; export m; unexport m` leaves `m` unexported in real
    // Tcl ([L1 new] m -> unknown method, 9.0.4 / 8.6.14) — but this chain
    // reads *any* `exports` entry as decisive, so recording the name in
    // both sets would make cross-file go-to-definition treat a
    // runtime-inaccessible method as public.  Covers the unwrapped
    // spelling and the `private` one, in both writer orders.
    for (body, dialect, callable) in [
        ("export m\nunexport m", "tcl8.6", false),
        ("unexport m\nexport m", "tcl8.6", true),
        // `private` is a 9.0-only member word.
        ("private export m\nprivate unexport m", "tcl9.0", false),
        ("private unexport m\nprivate export m", "tcl9.0", true),
    ] {
        let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}\n");
        let a = analyse_as(
            &src,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let chain = index.method_dispatch_chain("::C", "m", MethodAccess::External);
        assert_eq!(!chain.is_empty(), callable, "{body}");
    }
}

/// A call site at the end of `uri` — the offset only matters to the
/// import-lifecycle gate (a `namespace forget` / source deletion earlier
/// in the *same* document), so tests with no such event may use it
/// freely.
/// Byte offset of `needle` in a test source.
fn app_src_offset(src: &str, needle: &str) -> usize {
    src.find(needle).expect("needle present")
}

fn call_from(uri: &str) -> CallSite<'_> {
    call_at(uri, u32::MAX)
}

/// A **top-level** call site at `at` in `uri` — the shape every ordering
/// test below uses, since a load-level call carries no enclosing body.
fn call_at(uri: &str, at: u32) -> CallSite<'_> {
    CallSite {
        uri,
        at,
        enclosing_body: None,
    }
}

#[test]
fn cross_file_supertypes_and_subtypes() {
    // Base in a.tcl; Dog (subclass) in b.tcl; Puppy (subclass of Dog) in c.tcl.
    let a = analyse("oo::class create Animal {}\n");
    let b = analyse("oo::class create Dog {\n    superclass Animal\n}\n");
    let c = analyse("oo::class create Puppy {\n    superclass Dog\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///c.tcl", &c),
    ]);
    // Dog's superclass Animal resolves cross-file (a.tcl).
    let sup = index.classes_named("Animal");
    assert_eq!(sup.len(), 1);
    assert_eq!(sup[0].uri, "file:///a.tcl");
    // Animal's subclasses: Dog (b.tcl).
    let subs = index.subclasses_of("::Animal");
    let names: Vec<&str> = subs.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Dog"]);
    // Dog's subclasses: Puppy (c.tcl).
    let dog_subs = index.subclasses_of("::Dog");
    assert_eq!(
        dog_subs.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["Puppy"]
    );
}

#[test]
fn owner_aware_super_resolution_picks_same_namespace_and_abstains_on_ambiguity() {
    // Two `Base` classes in disjoint namespaces.  A subclass in ::A that
    // writes a bare `superclass Base` must link to ::A::Base (its own
    // namespace), never ::B::Base — and a subclass in a *third*
    // namespace with no local Base must abstain (ambiguous tail), not
    // guess.
    let a = analyse(
        "oo::class create ::A::Base {}\noo::class create ::A::Derived {\n    superclass Base\n}\n",
    );
    let b = analyse("oo::class create ::B::Base {}\n");
    let c = analyse("oo::class create ::C::Widget {\n    superclass Base\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///c.tcl", &c),
    ]);
    // ::A::Base's subclasses: only ::A::Derived (owner-aware pick).
    let a_subs: Vec<&str> = index
        .subclasses_of("::A::Base")
        .iter()
        .map(|c| c.qualified_name.as_str())
        .collect();
    assert_eq!(
        a_subs,
        vec!["::A::Derived"],
        "owner-aware resolution mis-linked"
    );
    // ::B::Base gets no subclass from the ambiguous bare `Base` names.
    assert!(
        index.subclasses_of("::B::Base").is_empty(),
        "ownerless tail match manufactured a wrong subtype edge",
    );
    // ::C::Widget's supertypes abstain (Base is ambiguous, ::C has none).
    let widget = index
        .classes()
        .find(|c| c.qualified_name == "::C::Widget")
        .expect("Widget indexed");
    assert!(
        index.supertype_classes(widget).is_empty(),
        "ambiguous bare superclass should resolve to nothing",
    );
}

#[test]
fn cross_file_method_override_family() {
    // Base `speak` in a.tcl; Dog overrides it in b.tcl; Cat overrides it
    // in c.tcl; unrelated Engine::speak in d.tcl must stay out.
    let animal = analyse("oo::class create Animal {\n    method speak {} {}\n}\n");
    let dog = analyse("oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\n");
    let cat = analyse("oo::class create Cat {\n    superclass Animal\n    method speak {} {}\n}\n");
    let engine = analyse("oo::class create Engine {\n    method speak {} {}\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &animal),
        ("file:///b.tcl", &dog),
        ("file:///c.tcl", &cat),
        ("file:///d.tcl", &engine),
    ]);
    // Seed from Dog: family = Animal + Dog + Cat (across three files).
    let mut fam: Vec<&str> = index
        .method_override_family("::Dog", "speak")
        .iter()
        .map(|wc| wc.qualified_name.as_str())
        .collect();
    fam.sort_unstable();
    fam.dedup();
    assert_eq!(
        fam,
        vec!["::Animal", "::Cat", "::Dog"],
        "cross-file family wrong"
    );
    // Unrelated Engine::speak must not be pulled in.
    assert!(
        !index
            .method_override_family("::Dog", "speak")
            .iter()
            .any(|wc| wc.qualified_name == "::Engine"),
        "unrelated same-named method must stay out of the family",
    );
    // Seeding from a class that only *inherits* speak still finds the
    // family via the providing ancestor.
    let puppy = analyse("oo::class create Puppy {\n    superclass Dog\n}\n");
    let index2 = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &animal),
        ("file:///b.tcl", &dog),
        ("file:///e.tcl", &puppy),
    ]);
    let fam2: Vec<&str> = index2
        .method_override_family("::Puppy", "speak")
        .iter()
        .map(|wc| wc.qualified_name.as_str())
        .collect();
    assert!(
        fam2.contains(&"::Animal") && fam2.contains(&"::Dog"),
        "{fam2:?}"
    );
}

#[test]
fn cross_file_method_inheritor_classes() {
    // Base `speak` in a.tcl; a purely-inheriting Dog (no override) in
    // b.tcl; an unrelated Engine::speak hierarchy with its own inheritor
    // Car in c.tcl/d.tcl.  Seeding from Animal, Dog is an inheritor and
    // Car (disjoint hierarchy) is not.
    let animal = analyse("oo::class create Animal {\n    method speak {} {}\n}\n");
    let dog = analyse(
        "oo::class create Dog {\n    superclass Animal\n    method describe {} { my speak }\n}\n",
    );
    let engine = analyse("oo::class create Engine {\n    method speak {} {}\n}\n");
    let car = analyse("oo::class create Car {\n    superclass Engine\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &animal),
        ("file:///b.tcl", &dog),
        ("file:///c.tcl", &engine),
        ("file:///d.tcl", &car),
    ]);
    let inheritors: Vec<&str> = index
        .method_inheritor_classes("::Animal", "speak")
        .iter()
        .map(|wc| wc.qualified_name.as_str())
        .collect();
    assert_eq!(inheritors, vec!["::Dog"], "{inheritors:?}");
    // A definer is never returned as an inheritor.
    assert!(
        !index
            .method_inheritor_classes("::Animal", "speak")
            .iter()
            .any(|wc| wc.qualified_name == "::Animal"),
    );
}

#[test]
fn method_inheritor_abstains_on_disjoint_definer_ancestor() {
    // A class that multiply-inherits from two unrelated definers of the
    // same method could resolve to either; the family seeded from only one
    // must NOT claim it (sound abstention, no over-rename).
    let a = analyse("oo::class create A {\n    method run {} {}\n}\n");
    let b = analyse("oo::class create B {\n    method run {} {}\n}\n");
    let both = analyse("oo::class create Both {\n    superclass A B\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///both.tcl", &both),
    ]);
    // Family seeded from A does not include B, so `Both` (which can reach
    // B::run too) is abstained on.
    assert!(
        index.method_inheritor_classes("::A", "run").is_empty(),
        "must abstain when an out-of-family definer ancestor exists",
    );
}

#[test]
fn indexes_procs_from_multiple_documents() {
    let a = analyse("proc greet {name} {}\n");
    let b = analyse("proc farewell {} {}\nproc greet2 {x y} {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.procs().count(), 3);
    // Param counts captured.
    let greet = index.procs().find(|p| p.name == "greet").unwrap();
    assert_eq!(greet.param_count, 1);
    assert_eq!(greet.uri, "file:///a.tcl");
}

#[test]
fn procs_matching_excludes_current_doc_and_filters_prefix() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc alphabet {} {}\nproc beta {} {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // From a.tcl's perspective, only b.tcl procs with `alph`.
    let got = index.procs_matching("alph", "file:///a.tcl");
    let names: Vec<&str> = got.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["alphabet"]);
}

#[test]
fn proc_definitions_resolves_cross_document() {
    let a = analyse("proc helper {} {}\n");
    let b = analyse("helper\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // From b.tcl, `helper` resolves to a.tcl's definition.
    let defs = index.proc_definitions("helper", "file:///b.tcl");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].uri, "file:///a.tcl");
    // The same-document exclusion drops it when querying
    // from a.tcl itself.
    assert_eq!(
        index.proc_definitions("helper", "file:///a.tcl"),
        [] as [&crate::workspace_index::WorkspaceProc; 0]
    );
}

#[test]
fn remove_document_drops_its_entries() {
    let a = analyse("proc a {} {}\n");
    let b = analyse("proc b {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    assert_eq!(index.procs().count(), 2);
    index.remove_document("file:///a.tcl");
    assert_eq!(index.procs().count(), 1);
    assert_eq!(index.procs().next().map(|p| p.name.as_str()), Some("b"));
}

/// A removal is scoped to the removed document.  Every other
/// document's records survive it untouched, in their original order —
/// which is what lets the removal cost that one document's rows instead of
/// a pass over every table in the workspace.
#[test]
fn remove_document_leaves_the_other_documents_alone() {
    let a = analyse("proc a {} {}\na\n");
    let b = analyse("proc b {} {}\nb\n");
    let c = analyse("proc c {} {}\nc\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    index.add_document("file:///c.tcl", &c);
    index.remove_document("file:///b.tcl");
    let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "c"]);
    let call_uris: std::collections::BTreeSet<&str> =
        index.invocations().map(|i| i.uri.as_str()).collect();
    assert_eq!(
        call_uris,
        ["file:///a.tcl", "file:///c.tcl"].into_iter().collect()
    );
    assert_eq!(
        index.document_uris(),
        vec!["file:///a.tcl", "file:///c.tcl"]
    );
}

/// The remove-then-add every diagnostics publish performs must
/// neither grow the index nor shuffle the workspace — the document goes
/// back into the slot it just vacated.
#[test]
fn re_indexing_a_document_keeps_its_size_and_its_place() {
    let a = analyse("proc a {} {}\n");
    let b = analyse("proc b {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    for _ in 0..5 {
        index.remove_document("file:///a.tcl");
        index.add_document("file:///a.tcl", &a);
    }
    let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
}

/// Removing a URI the index never held changes nothing.
#[test]
fn removing_an_unindexed_document_changes_nothing() {
    let a = analyse("proc a {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.remove_document("file:///never-indexed.tcl");
    assert_eq!(index.procs().count(), 1);
}

/// Several analyses of one URI — one re-homed view per source-site
/// namespace — accumulate under that URI, and one removal drops the whole
/// set.
#[test]
fn several_views_of_one_document_accumulate_and_drop_together() {
    let a = analyse("proc helper {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///a.tcl", &a);
    assert_eq!(index.procs().count(), 2);
    index.remove_document("file:///a.tcl");
    assert_eq!(index.procs().count(), 0);
}

#[test]
fn indexes_classes() {
    let a = analyse("oo::class create Widget {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let defs = index.class_definitions("Widget", "file:///other.tcl");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].qualified_name, "::Widget");
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
fn glob_import_introduces_no_command_link() {
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

// Cross-document wildcard-import resolution:
// `resolve_wildcard_import` is the mechanism that DOES resolve a bare
// call through the glob import `glob_import_introduces_no_command_link`
// (above) proves records no fixed link for.

#[test]
fn resolve_wildcard_import_resolves_exported_proc_cross_document() {
    // TP — `::mymod` (a.k.a. `mymod.tcl`) exports `helper`; `app.tcl`
    // wildcard-imports it and calls it bare from inside `run`, whose
    // own namespace is `::app` (a proc runs in the namespace it was
    // defined in, regardless of call site).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn colon_run_glob_import_drives_cross_file_navigation_references_and_w123() {
    // Mutation guard: `rsplit_once("::")` sees the final two
    // colons in `::src:::im*`, while the Tcl owner sees source `::src`
    // and tail glob `im*`.  The same fixture proves the indexed target,
    // linked cross-file reference, and analyser W123 suppression.
    let src =
        analyse("namespace eval ::src {\n    proc image {} {}\n    namespace export image\n}\n");
    let app_src =
        "namespace eval ::app {\n    namespace import ::src:::im*\n    proc run {} { image }\n}\n";
    let app = analyse(app_src);
    assert!(
        app.diagnostics
            .iter()
            .all(|d| d.code != tcl_core_types::DiagCode::W123),
        "imported image must suppress W123: {:?}",
        app.diagnostics
    );
    let index =
        WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
    let call = index.linked_invocations_of("::src::image", "file:///src.tcl");
    assert!(
        call.iter()
            .any(|inv| inv.uri == "file:///app.tcl" && inv.name == "image"),
        "glob import must link the cross-file call: {call:?}"
    );
    let direct = index.invocations_of("::src::image", "file:///src.tcl");
    assert!(
        direct.is_empty(),
        "direct view must retain the local imported name"
    );
    assert_eq!(
        index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_owned(), "::helper".to_owned()],
            call_from("file:///caller.tcl"),
        ),
        None
    );
}

#[test]
fn non_suffix_glob_import_resolves_cross_file_reference() {
    let src =
        analyse("namespace eval ::src {\n    proc helper {} {}\n    namespace export helper\n}\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::src::he*r\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_owned(), "::helper".to_owned()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::src::helper"));
}

#[test]
fn resolve_wildcard_import_does_not_resolve_unexported_sibling_cross_document() {
    // FP guard (CRITICAL) — `::mymod` also declares `other`, but never
    // exports it; real Tcl's `namespace import ::mymod::*` never binds
    // it, so the resolver must abstain (matching the in-document
    // `wildcard_namespace_import_unexported_sibling_stays_unresolved`
    // guard in `definition.rs`).
    let mymod = analyse(
        "namespace eval ::mymod {\n    proc helper {} {}\n    proc other {} {}\n    namespace export helper\n}\n",
    );
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { other }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "other",
        &["::app::other".to_string(), "::other".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn resolve_wildcard_import_resolves_exported_class_cross_document() {
    // TP — the georgtree_tclopt corpus shape: `tclopt.tcl` exports a
    // TclOO class and `examples/*.tcl` wildcard-imports and instantiates
    // it bare.  The shared cross-document mechanism
    // (`workspace_command_exists` / `defined_command_names`) covers
    // classes exactly like procs; pinned here as class-side coverage.
    let mypkg = analyse(
        "namespace eval ::mypkg {\n    namespace export Widget\n    oo::class create Widget {\n        method run {} { return 42 }\n    }\n}\n",
    );
    let consumer = analyse("namespace import ::mypkg::*\nset w [Widget new]\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mypkg.tcl", &mypkg),
        ("file:///consumer.tcl", &consumer),
    ]);
    let resolved = index.resolve_wildcard_import(
        "Widget",
        &["::Widget".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mypkg::Widget"));
}

#[test]
fn resolve_wildcard_import_is_not_restricted_to_the_calling_document() {
    // TP — `namespace import` binds to the
    // *namespace*, not the file that wrote it: real Tcl reopening
    // `::app` in a later `namespace eval ::app { ... }` block sees
    // every import already recorded for `::app`, regardless of which
    // file recorded it. A common real-world shape is a shared
    // "imports.tcl" that does the import once, with sibling files
    // reopening the same namespace to call the bare name. Three
    // separate files: `mymod.tcl` exports `helper`; `imports.tcl` does
    // `namespace eval ::app { namespace import ::mymod::* }`;
    // `caller.tcl` does `namespace eval ::app { proc run {} { helper } }`
    // — the import statement and the call site are never in the same
    // document, so filtering candidate glob imports by `g.uri == uri`
    // (the calling document) would never admit this shape.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
    let caller = analyse("namespace eval ::app {\n    proc run {} { helper }\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

// Per-import-site export snapshots, cross-document tier.
// The workspace resolver applies the same shared decision function the
// same-document one does (`namespace_import::exported_at_import_site`),
// so the two tiers cannot disagree — but only events in the *import's own
// document* are ordered against it; another file's load order is not a
// static fact.

#[test]
fn resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file() {
    // TP, direction A, cross-document — the import and the later
    // `namespace export -clear` are both in `app.tcl`, so they *are*
    // ordered: the `-clear` runs after the import and cannot revoke what
    // it bound (oracle tclsh 8.6.14/9.0.4). `::mymod`'s own export is in
    // the other file and unordered, which the shared function keeps.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

// The `source` graph orders what the file boundary cannot.
//
// Sourcing a file inlines its whole
// body at the `source` statement's position, so the DFS of the source
// forest *is* the run order. Oracle for every case below, byte-identical
// on tclsh 8.6.14 and 9.0.4 — with
//
//   # exp.tcl:  namespace eval ::mymod { namespace export helper }
//   # imp.tcl:  namespace eval ::app   { namespace import ::mymod::* }
//   # mod.tcl:  namespace eval ::mymod { proc helper {} {return HELP} }
//
//   # app.tcl:  source mod.tcl; source exp.tcl; source imp.tcl
//   ::app::helper   ->  HELP
//   # app.tcl:  source mod.tcl; source imp.tcl; source exp.tcl
//   ::app::helper   ->  invalid command name "::app::helper"

/// The installed server resolver's source-plan and inventory contract,
/// with the file URI mapped to an explicit document filename.
fn test_resolve(
    parent_uri: &str,
    _raw_path: &str,
    expression: Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    source_offset: u32,
    raw_constants: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> Option<String> {
    let parent = parent_uri.strip_prefix("file://")?;
    let dir = std::path::Path::new(parent).parent()?;
    let expression = expression?;
    if !expression.matches_assignments(raw_constants)
        || expression.source_span().start() > source_offset
        || source_offset >= expression.source_span().end()
    {
        return None;
    }
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
        raw_constants,
        Some(parent),
        imported,
    );
    let raw = expression.evaluate(Some(parent), &|name| {
        tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
            &constants.at(source_offset),
            name,
        )
    })?;
    let child = crate::source_graph::resolve_under(dir, &raw);
    Some(format!("file://{}", child.display()))
}

/// The server's constant folder, in miniature — the counterpart of
/// [`test_resolve`], same URI-to-path convention.
fn test_fold(
    uri: &str,
    writes: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
    let path = uri.strip_prefix("file://");
    tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(writes, path, imported)
}

/// An index over `documents` with the `source`-path resolver installed —
/// the shape the real server builds ([`WorkspaceIndex::set_source_resolver`]).
fn sourced_index<'a>(
    documents: impl IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
) -> WorkspaceIndex {
    let mut index = WorkspaceIndex::new();
    index.set_source_resolver(test_resolve, test_fold);
    for (uri, analysis) in documents {
        index.add_document(uri, analysis);
    }
    index
}

/// The three module documents every source-order test below shares.
fn import_order_modules() -> (AnalysisResult, AnalysisResult, AnalysisResult) {
    (
        analyse("namespace eval ::mymod { proc helper {} { return HELP } }\n"),
        analyse("namespace eval ::mymod { namespace export helper }\n"),
        analyse("namespace eval ::app { namespace import ::mymod::* }\n"),
    )
}

#[test]
fn retained_source_routes_share_original_plans_across_graph_consumers() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let app = analyse(
        "interp alias {} build {} file join /proj; package require Tk; package prefer latest; source [build lib.tcl]",
    );
    let child = analyse("proc leaf {} {}");
    let index = sourced_index([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &child),
    ]);
    let edges = index.resolved_source_edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].child, "file:///proj/lib.tcl");
    assert_eq!(
        index.source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl"),
        vec!["Tk".to_owned()]
    );
    assert!(index.source_ancestor_prefers_latest_from_resolved("file:///proj/lib.tcl"));
    assert_eq!(
        index
            .source_seed_map(test_resolve)
            .get("file:///proj/lib.tcl"),
        Some(&std::collections::BTreeSet::from(["::".to_owned()]))
    );
    let shadow =
        analyse("proc file {args} {}; package require Tk; source [file join /proj lib.tcl]");
    let index = sourced_index([
        ("file:///proj/app.tcl", &shadow),
        ("file:///proj/lib.tcl", &child),
    ]);
    assert_ne!(
        index.sources().collect::<Vec<_>>(),
        [] as [&crate::workspace_index::WorkspaceSource; 0]
    );
    assert_eq!(
        index.resolved_source_edges(),
        [] as [crate::source_graph::RunEdge; 0]
    );
    assert!(index.source_seed_map(test_resolve).is_empty());
    assert_eq!(
        index.source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn an_export_sourced_before_the_import_resolves() {
    // TP: `app.tcl` sources the export before the import, so the import
    // really did see it — resolution here rests on a proved order rather
    // than an abstention.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn an_export_sourced_after_the_import_is_not_retroactive() {
    // FP guard (CRITICAL) — the whole point of the
    // order. The import runs first, `::mymod` has exported nothing yet,
    // so real Tcl installs no alias at all. Byte-identical documents to
    // the test above; only `app.tcl`'s two `source` lines swap.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///p/caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export sourced after the import cannot apply retroactively: {resolved:?}",
    );
}

#[test]
fn without_a_resolver_the_same_workspace_keeps_abstaining() {
    // TN for the deployment shape: an index with no `source` resolver
    // installed ranks no cross-document event — the foreign export counts
    // and the import resolves, whichever way the `source` statements are
    // written.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_re_sourced_export_file_keeps_abstaining() {
    // TN (CRITICAL): `exp.tcl` is sourced twice, so it has no unique
    // position and the order must not invent one — Tcl tolerates
    // re-sourcing, and guessing would silently drop a real alias. Falls
    // back to the pre-graph abstention: the export counts.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

// The `package require` half of the load order.
//
// Every test below builds the index with `WorkspaceIndex::from_documents`
// — no `source` resolver at all — because the package half needs none: a
// `package require NAME` names its provider through the index's own
// `package provide` records.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, with `pkg/lib.tcl`
// holding the provider and `pkg/pkgIndex.tcl` the usual
// `[list source [file join $dir lib.tcl]]`:
//
//   package require mymod
//   namespace eval ::mymod { namespace export -clear }
//   namespace eval ::app { namespace import ::mymod::* }
//     -> NO ALIAS (invalid command name "::app::helper")
//
//   namespace eval ::mymod { namespace export -clear }
//   package require mymod
//   namespace eval ::app { namespace import ::mymod::* }
//     -> HELP
//
// …and the second script, byte-identical, answers NO ALIAS when some
// other file required `mymod` before it ran.  That flip is the whole
// reason the second shape abstains.

/// The provider document every `package require` order test shares: it
/// defines and exports `helper`, and provides the package.
fn package_provider() -> AnalysisResult {
    analyse(
        "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\npackage provide mymod 1.0\n",
    )
}

/// Whether `helper` still resolves through `::app`'s wildcard import in
/// `index` — the question every test below asks.
fn helper_resolves(index: &WorkspaceIndex) -> Option<String> {
    index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///p/caller.tcl"),
    )
}

#[test]
fn a_clear_after_a_package_require_revokes_the_providers_export() {
    // TP (CRITICAL) — the movement the package half exists for. `app.tcl`
    // requires the package (so `lib.tcl` has run), clears `::mymod`'s
    // exports, and only then imports. Without the package order the
    // foreign export would count and the local `-clear` would revoke
    // nothing, so `helper` would resolve; real Tcl installs no alias at
    // all.
    let lib = package_provider();
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    let resolved = helper_resolves(&index);
    assert!(
        resolved.is_none(),
        "the `-clear` provably ran after the provider's export and before the import: {resolved:?}",
    );
    // Non-vacuity: with the `-clear` removed, the same workspace resolves,
    // so the assertion above is the ordering's doing.
    let app_no_clear =
        analyse("package require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/app.tcl", &app_no_clear),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_2_a_statement_above_the_require_stays_unordered() {
    // TN (CRITICAL) — uncertainty 2, the package may already be loaded,
    // in which case the `package require` runs nothing at all and the
    // provider's statements are *older* than everything in this file.
    //
    // A `source` in this shape settles it — "an export sourced after the
    // import is not retroactive", the test above. A `package require`
    // must not, and the oracle is a file whose answer flips with nothing
    // in it changed. `imp_body.tcl`, byte for byte:
    //
    //   namespace eval ::mymod {}
    //   namespace eval ::app { namespace import ::mymod::* }
    //   package require mymod
    //
    //   sourced on its own            -> NO ALIAS
    //   sourced after `package require mymod` elsewhere -> HELP
    //
    // So the order says nothing and the pre-existing "abstain toward
    // answering" stands: the foreign export counts, `helper` resolves.
    let lib = package_provider();
    let app = analyse(
        "namespace eval ::mymod {}\nnamespace eval ::app { namespace import ::mymod::* }\npackage require mymod\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    assert_eq!(
        helper_resolves(&index).as_deref(),
        Some("::mymod::helper"),
        "a require never proves the provider had *not* yet run",
    );
    // …and the same shape with a `-clear` above the require is equally
    // unordered: run alone the clear hits an empty `::mymod` and the
    // require then loads the exporting file (oracle: HELP), run after
    // any other file required `mymod` the very same text revokes
    // (oracle: NO ALIAS).
    let app = analyse(
        "namespace eval ::mymod { namespace export -clear }\npackage require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_1_two_documents_providing_one_package_order_nothing() {
    // TN — uncertainty 1, `auto_path` is mutable. Two indexed files
    // provide `mymod`; which one a `package require` runs is decided by
    // the runtime search-path order, and only one of them exports.
    // Oracle (8.6.14 / 9.0.4), two directories each holding a `mylib2
    // 1.0`: `lappend auto_path pkgA pkgB` gives `from A` with `p`
    // exported, `pkgB pkgA` gives `from B` with nothing exported. So no
    // edge, and the `-clear` that would otherwise revoke does not.
    let lib = package_provider();
    let other = analyse(
        "namespace eval ::mymod { proc helper {} { return OTHER } }\npackage provide mymod 1.0\n",
    );
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/other/lib.tcl", &other),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn abstention_3_an_indexed_ifneeded_script_orders_nothing() {
    // TN — uncertainty 3, `ifneeded` bodies are arbitrary. Once the
    // workspace holds the package's own index script, the statements a
    // `package require mymod` runs are that script's business, and it may
    // load something else entirely. Oracle (8.6.14 / 9.0.4): one
    // `pkgIndex.tcl` whose body branches on the environment answers
    // `::c::p` -> `real` (with `namespace export p` run) or -> `stub`
    // (with nothing exported), for the same `package require`.
    let lib = package_provider();
    let pkg_index = analyse("package ifneeded mymod 1.0 [list source [file join $dir lib.tcl]]\n");
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/pkgIndex.tcl", &pkg_index),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn a_conditional_or_dynamic_require_orders_nothing() {
    // TN: the optional-dependency idiom `if {[catch {package require …}]}`
    // may never run, and `package require $pkg` names nothing statically.
    // Both mirror the gates `package prefer latest` already applies.
    let lib = package_provider();
    for app_src in [
        "catch {package require mymod}\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        "set pkg mymod\npackage require $pkg\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    ] {
        let app = analyse(app_src);
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            helper_resolves(&index).as_deref(),
            Some("::mymod::helper"),
            "{app_src}",
        );
    }
}

#[test]
fn a_conditional_provide_does_not_make_a_document_the_provider() {
    // TN: the shim idiom. tcllib's `doctools2idx/import_json.tcl` writes
    // `package provide dict 1` inside `if {[package vcompare …] < 0} { if
    // {[catch {package require dict}]} { … } }` — a fake `dict` package
    // supplied only on an old interpreter. Reading that as "this document
    // provides `dict`" would name the wrong file as the provider on every
    // other interpreter, so a guarded provide establishes nothing.
    let shim = analyse(
        "if {[package vcompare [package present Tcl] 8.5] < 0} {\nnamespace eval ::mymod { proc helper {} { return SHIM } }\npackage provide mymod 1.0\n}\n",
    );
    let app = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &package_provider()),
        ("file:///p/shim.tcl", &shim),
        ("file:///p/app.tcl", &app),
    ]);
    // Only `lib.tcl` provides `mymod` unconditionally, so the edge stands
    // and the `-clear` revokes…
    assert!(helper_resolves(&index).is_none());
    // …but with `lib.tcl`'s provide made conditional too, nothing in the
    // workspace provably provides `mymod` and the order abstains.
    let guarded_lib = analyse(
        "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\nif {$::tcl_platform(platform) eq \"unix\"} { package provide mymod 1.0 }\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &guarded_lib),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
}

#[test]
fn one_provider_required_from_many_documents_orders_each_of_them() {
    // TP: a package required from three files is loaded once, by whichever
    // require runs first — so it has three upper bounds and no unique
    // position. Each requiring file is still ordered against it, which a
    // tree position (one entry site per document) could not express.
    let lib = package_provider();
    let clearing = analyse(
        "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
    );
    let bystander = analyse("package require mymod\nputs [::mymod::helper]\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///p/lib.tcl", &lib),
        ("file:///p/app.tcl", &clearing),
        ("file:///p/one.tcl", &bystander),
        ("file:///p/two.tcl", &bystander),
    ]);
    assert!(helper_resolves(&index).is_none());
}

#[test]
fn an_unfoldable_computed_source_path_orders_nothing() {
    // TN: `source $dir/exp.tcl` where `dir` comes from `[lindex $argv 0]`
    // names no document statically — `lindex` is outside the evaluator's
    // subset, so the constant never folds — the edge is dropped and the
    // export goes back to being unrankable: the deliberate abstention,
    // not an accident of path resolution.
    //
    // Spelling `dir` as `[file dirname [info script]]` instead would fold
    // through the chained single-assignment evaluator; that foldable
    // spelling has its own TP pin directly below.
    let (mod_doc, exp, imp) = import_order_modules();
    let app =
        analyse("set dir [lindex $argv 0]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn inherited_path_constants_cross_twelve_source_hops_and_retire_with_the_edge() {
    let root_source = "set dir /s\nsource /s/hop1.tcl\n";
    let root = tcl_compiler::analyser::Analyser::new().analyse(root_source, "tcl8.6");
    let mut documents = vec![("file:///s/root.tcl".to_owned(), root.clone())];
    for hop in 1..=12 {
        let source = if hop == 12 {
            "source $dir/leaf.tcl\n".to_owned()
        } else {
            format!("source /s/hop{}.tcl\n", hop + 1)
        };
        documents.push((
            format!("file:///s/hop{hop}.tcl"),
            tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6"),
        ));
    }
    documents.push((
        "file:///s/leaf.tcl".to_owned(),
        tcl_compiler::analyser::Analyser::new().analyse("proc leaf {} {}\n", "tcl8.6"),
    ));
    let mut index = sourced_index(
        documents
            .iter()
            .map(|(uri, analysis)| (uri.as_str(), analysis)),
    );
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .get("dir")
            .map(String::as_str),
        Some("/s")
    );
    assert!(
        index
            .source_seed_map(test_resolve)
            .contains_key("file:///s/leaf.tcl")
    );
    let removed = tcl_compiler::analyser::Analyser::new().analyse("set dir /s\n", "tcl8.6");
    index.replace_document("file:///s/root.tcl", &removed);
    assert!(
        !index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .contains_key("dir")
    );
    assert!(
        !index
            .source_seed_map(test_resolve)
            .contains_key("file:///s/leaf.tcl")
    );
    index.replace_document("file:///s/root.tcl", &root);
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/hop12.tcl")
            .get("dir")
            .map(String::as_str),
        Some("/s")
    );
}

#[test]
fn typed_source_imports_export_global_homes_and_keep_original_site_scope() {
    let parent = tcl_compiler::analyser::Analyser::new().analyse(
            "namespace eval N {set dir /EPHEMERAL; source /s/shared.tcl}; set globalDir /GLOBAL; source /s/shared.tcl", "jim");
    let reader = tcl_compiler::analyser::Analyser::new()
        .analyse("source $dir/missing.tcl; source $globalDir/core.tcl", "jim");
    let core = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
    let index = sourced_index([
        ("file:///s/start.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///GLOBAL/core.tcl", &core),
    ]);
    let imports = index.imported_path_constants_for("file:///s/shared.tcl");
    assert!(!imports.contains_key("dir"));
    // The earlier namespace route supplies no globalDir; agreement must
    // not use the later route's value for all executions.
    assert!(!imports.contains_key("globalDir"));
    assert_eq!(
        imports.naming_policy(),
        parent.path_constant_assignments.naming_policy()
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(!seeds.contains_key("file:///s/missing.tcl"));
}

#[test]
fn path_inventory_reset_and_snapshot_preserve_empty_policy_changes() {
    let c = tcl_compiler::analyser::Analyser::new().analyse("", "tcl8.6");
    let jim = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
    let mut index = sourced_index([("file:///empty.tcl", &c)]);
    let before = index.path_constant_assignments("file:///empty.tcl").clone();
    assert!(before.is_empty());
    index.add_document("file:///empty.tcl", &jim);
    assert_ne!(
        index.path_constant_assignments("file:///empty.tcl"),
        &before
    );
    assert_eq!(
        index
            .path_constant_assignments("file:///missing.tcl")
            .naming_policy(),
        None
    );
}

#[test]
fn a_cross_file_constant_resolves_a_readers_source_rows_1368() {
    // OSVVM's shape in miniature: two simulator entry scripts each
    // assign the same namespace constant and source the shared reader;
    // the reader sources a grandchild *through the imported value*.
    // Both parents live in one directory, so the agreement rule keeps
    // the name and the reader's own row resolves.
    let parent = analyse(
        "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource ${::osvvm::dir}/shared.tcl\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start1.tcl", &parent),
        ("file:///s/start2.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert_eq!(
        index
            .imported_path_constants_for("file:///s/shared.tcl")
            .get("::osvvm::dir")
            .map(String::as_str),
        Some("/s"),
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(
        seeds.contains_key("file:///s/core.tcl"),
        "the reader's own source row must resolve through the import: {seeds:?}",
    );
}

#[test]
fn disagreeing_parents_import_nothing_1368() {
    // TN: the same reader sourced from two directories — the constant
    // folds to a different value per parent, so no route-independent
    // value exists and the name must not import.  Whichever parent runs
    // decides, and this tier never guesses.
    let parent = analyse(
        "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource /s/shared.tcl\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///a/start1.tcl", &parent),
        ("file:///b/start2.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::osvvm::dir"),
    );
    let seeds = index.source_seed_map(test_resolve);
    assert!(
        !seeds.contains_key("file:///s/core.tcl"),
        "no agreed value, no edge: {seeds:?}",
    );
}

#[test]
fn a_body_local_source_route_imports_nothing_1368() {
    // A `source` inside a proc runs when the proc is called, not at its
    // lexical position: `set dir /a; proc load {} {source shared.tcl};
    // set dir /b; load` reaches the child with `/b`.  The lexical
    // position gate would say `/a`, so a body-local route must supply
    // nothing at all.
    let parent = analyse(
        "namespace eval ::cfg {\n    variable dir [file dirname [file normalize [info script]]]\n}\nproc load {} {\n    source /s/shared.tcl\n}\n",
    );
    let reader = analyse("source ${::cfg::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::cfg::dir"),
        "a body-local source route must not provide load-time constants",
    );
}

#[test]
fn an_assignment_after_the_source_does_not_import_1368() {
    // TN: the parent assigns the constant *after* sourcing the reader —
    // when the reader runs, the variable does not exist yet, and the
    // position gate must say so.
    let parent = analyse(
        "source /s/shared.tcl\nnamespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\n",
    );
    let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
    let core = analyse("proc core_helper {} {}\n");
    let index = sourced_index([
        ("file:///s/start1.tcl", &parent),
        ("file:///s/shared.tcl", &reader),
        ("file:///s/core.tcl", &core),
    ]);
    assert!(
        !index
            .imported_path_constants_for("file:///s/shared.tcl")
            .contains_key("::osvvm::dir"),
        "a write after the source has not happened when the child runs",
    );
}

#[test]
fn a_foldable_computed_source_path_orders_its_export() {
    // TP: the same shape with `dir` from `[file dirname
    // [info script]]` *does* fold through the chained constant, so
    // `$dir/exp.tcl` names its document and the load order ranks it.
    // With the order known the answer sharpens: app.tcl runs the
    // wildcard import *before* sourcing exp.tcl's `namespace export`,
    // and `namespace import` copies only currently-exported commands
    // (a later export does not retro-import), so `helper` is genuinely
    // not imported and resolution answers nothing — more precise than
    // the abstention the unfoldable spelling above falls back to.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse(
        "set dir [file dirname [info script]]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n",
    );
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        None,
    );
}

#[test]
fn a_forget_written_after_the_source_revokes_the_sourced_import() {
    // TP (CRITICAL), the `source lib.tcl ; namespace forget ::lib::p`
    // idiom: the install from the sourced file counts *and* the forget
    // beside the `source` revokes it. Oracle: `::app::helper` is an
    // `invalid command name` after the forget.
    let (mod_doc, exp, imp) = import_order_modules();
    let app = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
    );
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app),
    ]);
    // A call written after the forget no longer resolves…
    let after = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///p/app.tcl", u32::MAX),
    );
    assert!(
        after.is_none(),
        "a forget written after the `source` that installed the alias must revoke it: {after:?}",
    );
    // …and a call written *above* every `source` statement has no alias
    // yet either — the install has not run at that point.
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///p/app.tcl", 0),
            )
            .as_deref(),
        None,
    );
    // TP: with the forget removed, the same call site after the sources
    // does resolve — so the assertion above is the forget's doing and not
    // an accident of the graph.
    let app_no_forget = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &mod_doc),
        ("file:///p/exp.tcl", &exp),
        ("file:///p/imp.tcl", &imp),
        ("file:///p/app.tcl", &app_no_forget),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///p/app.tcl", u32::MAX),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_cross_file_import_conflict_is_decided_by_the_source_order() {
    // TP (CRITICAL) — two imports of one name from
    // different sources, in different files. Without an order neither
    // conflicts and the later one silently installs; with one, the file
    // sourced first owns the name and the second import raises `can't
    // import command "helper": already exists` and installs nothing.
    //
    // Oracle (8.6.14 / 9.0.4), with the two importers in separate files
    // sourced in this order:
    //   namespace origin ::app::helper  ->  ::first::helper
    let first = analyse(
        "namespace eval ::first { proc helper {} { return F }\n namespace export helper }\n",
    );
    let second = analyse(
        "namespace eval ::second { proc helper {} { return S }\n namespace export helper }\n",
    );
    let imp_a = analyse("namespace eval ::app { namespace import ::first::* }\n");
    let imp_b = analyse("namespace eval ::app { namespace import ::second::* }\n");
    let app = analyse("source first.tcl\nsource second.tcl\nsource impa.tcl\nsource impb.tcl\n");
    let docs = [
        ("file:///p/first.tcl", &first),
        ("file:///p/second.tcl", &second),
        ("file:///p/impa.tcl", &imp_a),
        ("file:///p/impb.tcl", &imp_b),
        ("file:///p/app.tcl", &app),
    ];
    assert_eq!(
        sourced_index(docs)
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::first::helper"),
        "the import sourced first owns the name; the second conflicts and installs nothing",
    );
    // Swap the two `source` lines and the winner swaps with them.
    let app = analyse("source first.tcl\nsource second.tcl\nsource impb.tcl\nsource impa.tcl\n");
    let docs = [
        ("file:///p/first.tcl", &first),
        ("file:///p/second.tcl", &second),
        ("file:///p/impa.tcl", &imp_a),
        ("file:///p/impb.tcl", &imp_b),
        ("file:///p/app.tcl", &app),
    ];
    assert_eq!(
        sourced_index(docs)
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_from("file:///p/caller.tcl"),
            )
            .as_deref(),
        Some("::second::helper"),
    );
}

#[test]
fn resolve_wildcard_import_ignores_a_same_file_export_written_after_the_import() {
    // FP guard (CRITICAL), direction B, cross-document — the import and
    // the export are both in `app.tcl` and the export comes *after*, so
    // real Tcl never binds `::app::helper` (oracle: `invalid command
    // name`). `mymod.tcl` exports nothing, so there is no unordered
    // event to fall back on and the resolver must abstain.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export written after the import in the same file must not \
             apply retroactively: {resolved:?}"
    );
}

#[test]
fn resolve_wildcard_import_keeps_answering_when_the_export_is_in_another_file() {
    // TN-for-abstention — which of two files loads first is not a static
    // fact, so a `namespace export -clear` in a *third* document cannot
    // be ordered against this import and must not silently revoke it.
    // Navigation keeps answering; the residual is documented in
    // `namespace_import`'s module docs.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let teardown = analyse("namespace eval ::mymod { namespace export -clear }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn wildcard_import_inside_a_body_sees_a_later_top_level_export() {
    // TP — a plain `at <= import_at` predicate is *weaker* than the
    // same-document tier's `indirection::in_effect` and rejects this real
    // alias. The import
    // sits in `::app::setup`'s body; the `namespace export` is written
    // further down the *same file* but at load level, so loading `app.tcl`
    // runs the export before `setup` can ever be called.
    //
    // Oracle (tclsh 8.6.14 / 9.0.4): with `::mymod::helper` defined
    // elsewhere, `namespace eval ::app {proc setup {} {namespace import
    // ::mymod::*}; proc run {} {helper}}` followed by `namespace eval
    // ::mymod {namespace export helper}`, then `::app::setup; ::app::run`
    // → `HELP`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* }\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "an import inside a body observes every top-level statement of \
             its own file, wherever written",
    );
}

#[test]
fn wildcard_import_inside_a_body_still_refuses_an_export_in_that_same_body() {
    // FN guard for the leniency above: the
    // "whole file loads first" exception does **not** extend to a
    // statement of the *same* body — there the offsets are in genuine
    // execution order. Oracle: `proc setup {} {namespace import ::m::*;
    // namespace eval ::m {namespace export helper}}` then `::a::setup`
    // leaves `::a::helper` an `invalid command name`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* ; namespace eval ::mymod { namespace export helper } }\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export written after the import in the same body is a later \
             statement of the running script: {resolved:?}",
    );
}

// The import edge's own lifecycle, cross-document.
//
// The workspace twin of `definition.rs`'s in-document block. Same oracle
// rows (tclsh 9.0.4 + 8.6.14, byte-identical), same shared decision
// function (`namespace_import::alias_live_at`), so the two tiers cannot
// drift.

#[test]
fn cross_file_forget_after_the_import_stops_resolving() {
    // TN — the source lives in another file, so only this tier can
    // answer; the forget and the call are in one document, so they are
    // ordered.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(
        resolved.is_none(),
        "a call after the forget must not resolve through the alias: {resolved:?}"
    );
}

#[test]
fn cross_file_call_before_the_forget_still_resolves() {
    // TP — the same document, a call written before the forget.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn a_forget_in_another_file_revokes_nothing() {
    // TN-for-abstention — no static load order between two files, so a
    // foreign forget is passed unordered and cannot silently drop a real
    // alias. Same direction as an unordered `namespace export -clear`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
    let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn cross_file_source_deletion_kills_the_alias() {
    // TN — `rename ::mymod::helper {}` destroys the command object, and
    // the alias holds the object. Ordered because the deletion and the
    // call share a document.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper {}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn cross_file_source_rename_leaves_the_alias_alive() {
    // TP — the asymmetry again: `rename ::mymod::helper ::mymod::h2`
    // moves the origin and keeps `::app::helper` working.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper ::mymod::h2\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn an_unforced_import_onto_an_existing_workspace_command_installs_nothing() {
    // TN — `::app` already defines `helper`, so the non-`-force` import
    // errors and installs nothing; the call reaches the local definition,
    // not `::mymod::helper` (oracle: `namespace origin ::app::helper` →
    // `::app::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::*\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///app.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_forced_import_onto_an_existing_workspace_command_installs() {
    // TP — the same program with `-force` replaces the local command, so
    // the call reaches `::mymod::helper` (oracle: `namespace origin
    // ::app::helper` → `::mymod::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::*\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///app.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn an_exact_import_onto_an_existing_workspace_command_installs_no_link() {
    // TN — the same conflict rule on the exact-import link path
    // (`live_command_links`), so definition / references / the existence
    // oracle all agree the import bound nothing.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "a conflicting exact import installs no link"
    );
}

#[test]
fn a_forced_exact_import_still_installs_its_link() {
    // FN guard for the rule above.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn a_cross_file_forget_cannot_revoke_on_unrelated_offsets() {
    // FP guard (CRITICAL) — the import lives in a short `imports.tcl`
    // (small byte offset); the forget lives in a long `caller.tcl` (large
    // byte offset) and names a namespace the caller never imported into
    // itself. Nothing orders the two files, so the forget must revoke
    // nothing — comparing the raw offsets would make the caller's larger
    // number "later" and drop a live alias.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
    // Padding so the forget's offset is numerically far past the import's.
    let pad = "# ".to_string() + &"x".repeat(400) + "\n";
    let caller_src =
        format!("{pad}namespace eval ::app {{\n    namespace forget ::mymod::helper\n}}\nhelper\n");
    let caller = analyse(&caller_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    let call = u32::try_from(app_src_offset(&caller_src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///caller.tcl", call),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "a forget in another document has no order against the import and \
             must not revoke it: {resolved:?}"
    );
}

#[test]
fn a_same_file_forget_after_the_import_still_revokes() {
    // TN, the other direction of the same-document rule — when the import, the forget
    // and the call really do share a document the offsets mean something
    // and the ordering is unchanged.
    let src = "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

// Call-site ordering of the install.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   namespace eval ::src { proc p {} {return P}; namespace export p }
//   namespace eval ::dst { p }                  ;# invalid command name "p"
//   namespace eval ::dst { namespace import ::src::* }
//   namespace eval ::dst { p }                  ;# P
//
// …and the body-scope half, same transcript run:
//
//   namespace eval ::app { proc run {} { helper } }
//   namespace eval ::app { namespace import ::src::* }
//   ::app::run                                  ;# HELP
//
//   namespace eval ::app2 { proc run2 {} {
//       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
//   ::app2::run2   ;# {invalid command name "helper"} HELP

#[test]
fn a_top_level_call_before_its_own_import_does_not_resolve() {
    // TN. Both call sites are top level, so the offsets are in genuine
    // execution order and the earlier one reaches nothing.
    let src = "rename auto_import savedPrelude; helper\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let candidates = ["::app::helper".to_string(), "::helper".to_string()];
    let before = u32::try_from(app_src_offset(src, "helper\n")).expect("tiny source");
    assert!(
        index
            .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", before))
            .is_none(),
        "a call written before its own import must not resolve through it",
    );
    // TP — the same call after the import still does.
    let after = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    assert_eq!(
        index
            .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", after))
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_body_local_call_resolves_through_an_import_written_later_in_its_file() {
    // TP (CRITICAL) — the reason the install gate is `in_effect_within`
    // and not `at < call`: the whole file loads, imports included, before
    // any body runs, so this shape (procs first, `namespace import` at the
    // bottom) still resolves. `tcllib`'s `modules/uev/uevent.tcl` is
    // exactly it; a plain-offset gate broke all five of its call sites.
    let src = "namespace eval ::app {\n    proc run {} { helper }\n}\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "helper }")).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        CallSite {
            uri: "file:///app.tcl",
            at: call,
            enclosing_body: app.innermost_definition_body_span(call),
        },
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "a body-local call observes its own file's later top-level import",
    );
}

#[test]
fn a_body_local_call_before_an_import_in_that_same_body_does_not_resolve() {
    // TN — the FN guard for the leniency above: an import written in the
    // *same* body after the call is an ordinary later statement of the
    // running script, exactly as the export snapshot already treats one.
    let src =
        "namespace eval ::app {\n    proc run {} { helper ; namespace import ::mymod::* }\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "helper ;")).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        CallSite {
            uri: "file:///app.tcl",
            at: call,
            enclosing_body: app.innermost_definition_body_span(call),
        },
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_call_before_an_import_in_another_file_still_resolves() {
    // TP — cross-file installs have no static load order, so the gate
    // cannot fire on a foreign offset (the same abstention every other
    // cross-file event takes here). `at` is deliberately tiny.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
    let caller = analyse("helper\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///caller.tcl", 0),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn the_body_span_column_matches_the_per_offset_lookup() {
    // `enclosing_body_spans` is a stack sweep standing in for one
    // `innermost_definition_body_span` per row (which is
    // O(procs × invocations)); nesting, shared starts and calls between
    // sibling bodies are where the two could part company.
    let a = analyse(
        "proc outer {} {\n  proc inner {} { set x 1 }\n  set y 2\n}\nset z 3\noo::class create C { method m {} { set w 4 } }\nset q 5\n",
    );
    let offsets: Vec<u32> = a
        .command_invocations
        .iter()
        .map(|i| i.range.start())
        .collect();
    assert!(offsets.len() > 5, "the fixture must exercise the sweep");
    let swept = WorkspaceIndex::enclosing_body_spans(&a, &offsets);
    let naive: Vec<Option<Span>> = offsets
        .iter()
        .map(|&o| a.innermost_definition_body_span(o))
        .collect();
    assert_eq!(swept, naive);
    // …and the column really is populated on the rows the resolver reads.
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert!(
        index.invocations().any(|i| i.enclosing_body.is_some()),
        "body-local invocations must carry their span",
    );
}

// Global-rooted imports go through the gate too.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   proc p {} { return GLOBAL }
//   namespace eval ::dst  { namespace import ::p }   ;# no error…
//   info commands ::dst::*                           ;# …and nothing bound
//   namespace export p
//   namespace eval ::dst2 { namespace import ::p }
//   info commands ::dst2::*                          ;# ::dst2::p

#[test]
fn a_global_rooted_exact_import_is_export_gated() {
    // TN — `::p` splits to an *empty* source namespace, which both tiers
    // read as "no source" and skipped, leaving the one import shape that
    // bypassed the gate entirely.
    let lib = analyse("proc p {} { return GLOBAL }\n");
    let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(
        index.resolve_command_target("::dst::p"),
        "::dst::p",
        "an unexported global command installs nothing",
    );
}

#[test]
fn a_global_rooted_exact_import_of_an_exported_name_still_binds() {
    // TP — the export at global level is recorded with `ns` = `::`, the
    // same spelling the gate now asks with, so the working case works.
    let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
    let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::p");
}

#[test]
fn a_global_rooted_glob_import_is_export_gated() {
    // TN — the glob spelling of the same shape, whose empty source
    // namespace makes it easy to drop rather than record.
    let lib = analyse("proc p {} { return GLOBAL }\n");
    let dst = analyse("namespace eval ::dst { namespace import ::* }\np\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    let candidates = ["::dst::p".to_string(), "::p".to_string()];
    assert!(
        index
            .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
            .is_none(),
        "an unexported global command is not reachable through `::*`",
    );
    // TP — with the export, it is.
    let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(
        index
            .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
            .as_deref(),
        Some("::p"),
    );
}

// Glob and exact import conflicts are symmetric.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, both orders:
//
//   namespace eval ::A { proc p {} {return A}; namespace export p }
//   namespace eval ::B { proc p {} {return B}; namespace export p }
//   namespace eval ::dst  { namespace import ::A::* ; namespace import ::B::p }
//   namespace eval ::dst2 { namespace import ::A::p ; namespace import ::B::* }
//   → both second imports: can't import command "p": already exists
//   → namespace origin ::dst::p  = ::A::p ; ::dst::p  → A
//   → namespace origin ::dst2::p = ::A::p ; ::dst2::p → A

/// The `::A` / `::B` sources both exporting `p`, used by the conflict
/// tests below.
fn two_exporting_sources() -> (AnalysisResult, AnalysisResult) {
    (
        analyse("namespace eval ::A { proc p {} { return A }\n namespace export p }\n"),
        analyse("namespace eval ::B { proc p {} { return B }\n namespace export p }\n"),
    )
}

#[test]
fn a_glob_import_conflicts_with_a_later_exact_import_of_the_same_name() {
    // TN (the gap) — the exact link's conflict check only ever compared
    // other *exact* links, so the earlier glob import was invisible to it
    // and `::dst::p` resolved to `::B::p`, the one binding Tcl refuses.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import ::B::p\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(
        index.resolve_command_target("::dst::p"),
        "::B::p",
        "the exact import installs nothing over the live glob alias",
    );
    // …and the name the call really reaches is the glob import's source.
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn an_exact_import_conflicts_with_a_later_glob_import_of_the_same_name() {
    // TN, the other direction of the same rule — the glob side asked only
    // about other glob imports.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::*\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert!(
        resolved.is_none_or(|r| r == "::A::p"),
        "the later glob import must not install over a live exact alias",
    );
    assert_eq!(index.resolve_command_target("::dst::p"), "::A::p");
}

#[test]
fn a_forced_exact_import_still_replaces_a_live_glob_alias() {
    // TP / FN guard — `-force` is exactly the case that *does* install
    // over whatever was there (oracle on `WorkspaceGlobImport::forced`),
    // so widening the conflict check must not swallow it.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import -force ::B::p\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
}

#[test]
fn a_body_local_import_conflicts_with_a_later_top_level_import() {
    // TN (CRITICAL) — the conflict check must order the two imports the
    // way they *run*, not the way they are written. Oracle, byte-identical
    // on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::A { proc x {} {return A}; namespace export x }
    //   namespace eval ::B { proc x {} {return B}; namespace export x }
    //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
    //   namespace eval ::dst { namespace import ::A::* }
    //   namespace origin ::dst::x  -> ::A::x   (after load)
    //   ::dst::p                   -> can't import command "x": already exists
    //   namespace origin ::dst::x  -> ::A::x   (unchanged)
    //
    // The whole file loads before any body runs, so the top-level `::A`
    // glob import owns the name by the time `p`'s body-local `::B` import
    // executes — and that one installs nothing. A plain offset compare
    // sees `::A` written *later* and lets `::B` install.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval ::dst {\n    namespace import ::A::*\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(
        index.resolve_command_target("::dst::p"),
        "::B::p",
        "the body-local import runs after the top-level one and installs nothing",
    );
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
        "the top-level glob import is the edge the name really has",
    );
}

/// The later import of `p` from `::B` against an earlier `::A` alias whose
/// forget sits where its position is unknown, as `src` is laid out.
fn later_import_after_an_unplaced_forget(src: &str, forget: Option<&str>) -> Option<String> {
    let (a, b) = two_exporting_sources();
    let dst = analyse(src);
    let forget = analyse(forget.unwrap_or(""));
    let index = sourced_index([
        ("file:///p/a.tcl", &a),
        ("file:///p/b.tcl", &b),
        ("file:///p/forgetA.tcl", &forget),
        ("file:///p/dst.tcl", &dst),
    ]);
    let call = u32::try_from(app_src_offset(src, "\np\n") + 1).expect("tiny source");
    index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_at("file:///p/dst.tcl", call),
    )
}

#[test]
fn a_sourced_forget_between_two_imports_does_not_make_the_later_one_conflict() {
    // tclsh 8.6 / 9.0: `namespace import ::A::*`, then `source forgetA.tcl`
    // (which forgets `::A::p`), then `namespace import ::B::*` leaves
    // `namespace origin ::dst::p` at `::B::p`. The forget is in another
    // document, so the earlier alias is not certainly live at the later
    // import and no conflict cancels the install Tcl performs.
    let src = "namespace eval ::dst { namespace import ::A::* }\nsource forgetA.tcl\nnamespace eval ::dst { namespace import ::B::* }\np\n";
    assert_eq!(
        later_import_after_an_unplaced_forget(
            src,
            Some("namespace eval ::dst { namespace forget ::A::p }\n")
        )
        .as_deref(),
        Some("::B::p"),
    );
}

#[test]
fn a_branch_forget_between_two_imports_does_not_make_the_later_one_conflict() {
    // tclsh 8.6 / 9.0 with the branch taken: `::B::p`. A forget inside an
    // `if` body is not straight-line sequence, so it keeps the earlier
    // alias possibly dead and the later install stands.
    let src = "namespace eval ::dst { namespace import ::A::* }\nif {1} { namespace eval ::dst { namespace forget ::A::p } }\nnamespace eval ::dst { namespace import ::B::* }\np\n";
    assert_eq!(
        later_import_after_an_unplaced_forget(src, None).as_deref(),
        Some("::B::p"),
    );
}

#[test]
fn a_clear_in_the_sourcing_document_stays_conditional_for_the_sourced_export() {
    // Cross-document order stays conditional even along a `source` edge:
    // the clear in `app.tcl` is not ordered after the pattern `lib.tcl`
    // exports, so it revokes nothing.
    let lib = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app_src = "source lib.tcl\nnamespace eval ::mymod { namespace export -clear }\n";
    let app = analyse(app_src);
    let declaration = lib
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .clone();
    let pattern = lib.original_namespace_exports().next().unwrap().span();
    let tombstone = app.original_namespace_exports().next().unwrap().span();
    let index = sourced_index([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    let order = index.run_order();
    let first = RunPoint {
        uri: "file:///p/lib.tcl",
        at: pattern.start(),
        enclosing_body: None,
    };
    let clear = RunPoint {
        uri: "file:///p/app.tcl",
        at: tombstone.start(),
        enclosing_body: None,
    };
    for (a, b) in [(first, clear), (clear, first)] {
        assert!(order.navigation_precedence(a, b).is_none_or(|relation| {
            relation.purpose() == crate::source_graph::SourceNavigationOrderPurpose::Conditional
                && relation.modeled_source_exclusion().is_none()
        }));
    }
    let query = RunPoint {
        uri: "file:///p/app.tcl",
        at: u32::try_from(app_src.len()).unwrap(),
        enclosing_body: None,
    };
    assert_eq!(
        crate::namespace_import::NamespaceExportOracle::exported_at_original(
            &*index.export_snapshot(),
            declaration.slot(),
            declaration.policy(),
            query
        ),
        ExportVerdict::Exported
    );
}

#[test]
fn a_top_level_import_is_not_conflicted_by_a_later_top_level_one() {
    // TN guard — at load level the offsets *are* execution order, so the
    // first import still wins and the second is the one that installs
    // nothing. Oracle (8.6.14 / 9.0.4): after `namespace import ::A::*`
    // then `namespace import ::B::p`, the second raises `can't import
    // command "p": already exists` and `namespace origin ::dst::p` stays
    // `::A::p`. Making the check body-aware must not reorder these.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::p\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn a_later_import_in_the_same_body_has_not_run_yet() {
    // TN guard, the other half — inside one body the offsets are genuine
    // execution order again. Oracle (8.6.14 / 9.0.4): `proc q {} {
    // namespace import ::A::* ; namespace import ::B::p }` → the first
    // succeeds, the second raises `already exists`, origin stays `::A::p`.
    // So the *first* must not be conflicted away by the second.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn a_glob_import_in_another_file_does_not_conflict_with_an_exact_one() {
    // FP guard — two imports in different documents have no static load
    // order, so conflicting on a guess would drop a link Tcl installed.
    // The abstention is the same one every other cross-file event takes.
    let (a, b) = two_exporting_sources();
    let globbed = analyse("namespace eval ::dst { namespace import ::A::* }\n");
    let exact = analyse("namespace eval ::dst { namespace import ::B::p }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///glob.tcl", &globbed),
        ("file:///exact.tcl", &exact),
    ]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
}

#[test]
fn an_exact_import_link_dies_on_a_same_file_forget() {
    // TN (CRITICAL) — an exact `namespace import ::mymod::helper`
    // produces a fixed link rather than a per-call glob lookup, so the
    // lifecycle events must reach it: otherwise the link stays live after
    // the forget and cross-document definition / references keep
    // resolving `::app::helper` to `::mymod::helper`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n    namespace forget ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "a forgotten exact import installs no live link"
    );
    assert!(!index.workspace_command_exists("::app::helper"));
}

#[test]
fn an_exact_import_link_keeps_a_conditional_same_file_forget() {
    // The link reads the document after it completes normally. A forget
    // behind a possible early exit, or in a procedure body, has run only
    // on some paths (tclsh 8.6 / 9.0: `x` true, or `teardown` never
    // called, leaves `::app::helper` imported), so the link stays live.
    // The renamed `auto_import` keeps the imports themselves modelled.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    for source in [
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n}\nif {$x} return\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\n",
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n}\nproc teardown {} {\n    namespace eval ::app {namespace forget ::mymod::helper}\n}\n",
    ] {
        let app = analyse(source);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper",
            "{source}"
        );
    }
}

#[test]
fn an_exact_import_link_survives_a_forget_written_before_it() {
    // FN guard for the row above — the forget is ordered against the
    // *import*, and one written before it is undone by the import itself
    // (a re-import after a forget reinstalls — oracle).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace forget ::mymod::helper\n    namespace import ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn a_destroyed_source_kills_a_body_local_exact_import_link() {
    // TN (CRITICAL): the destruction is not a timeline event — the command
    // object is gone workspace-wide and no load order brings it back — so
    // it must revoke however the import is written. A **body-local**
    // import gate is the case that separates the two encodings: the
    // load-order rule reads a removal written outside the import's own
    // body as having run before it, which is right for a `namespace
    // forget` (the import then undoes it) and backwards for a destruction
    // the import cannot undo. Oracle: `rename ::mymod::helper {}` makes
    // `::app::helper` an `invalid command name` and empties `info commands
    // ::app::*`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nrename ::mymod::helper {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "destroying the source command revokes the link wherever the import sits"
    );
    assert!(!index.workspace_command_exists("::app::helper"));
    // …and the top-level spelling of the same import, for the control.
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nrename ::mymod::helper {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper"
    );
}

#[test]
fn a_body_local_exact_import_survives_a_top_level_forget() {
    // FN guard for the row above, and the reason the destruction cannot
    // simply be encoded as "a removal at `u32::MAX`": a `namespace forget`
    // at the file's load level runs *before* a body-local import, which
    // then reinstalls the alias. Oracle: `::app::setup` followed by
    // `namespace origin ::app::helper` answers `::mymod::helper`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper",
        "a load-level forget runs before the body-local import that reinstalls it"
    );
}

#[test]
fn an_exact_import_link_ignores_a_forget_in_another_file() {
    // FP guard — the same-document ordering rule on the link tier: a forget with
    // no static order against the import revokes nothing.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
    let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn an_exact_import_link_dies_when_the_source_command_is_destroyed() {
    // TN — destroying the source is not order-ambiguous the way a forget
    // is: the command *object* the alias holds is gone workspace-wide
    // (oracle: `rename ::mymod::helper {}` makes `::app::helper` an
    // `invalid command name` and empties `info commands ::app::*`), so the
    // link dies even though the deletion sits in another document.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
    let teardown = analyse("rename ::mymod::helper {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "destroying the source destroys the link"
    );
}

#[test]
fn an_exact_import_link_dies_when_the_imported_name_is_redefined() {
    // TN, cross-document — a `proc ::app::helper`
    // after the import recreates the name as an ordinary command
    // (oracle: rc 0, `namespace origin` → `::app::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nproc ::app::helper {} {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper"
    );
}

#[test]
fn a_cross_file_live_alias_is_an_import_conflict_for_a_different_source() {
    // TN, cross-document — `::dst` imports `::A::*`
    // and then, further down the same file, `::B::*` without `-force`.
    // Oracle: the second import raises `can't import command "p": already
    // exists` and `namespace origin ::dst::p` stays `::A::p`.
    let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::*\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::A::p"),
        "the failed second import leaves the first alias in place: {resolved:?}"
    );
}

#[test]
fn a_cross_file_forced_import_replaces_a_live_alias() {
    // TP, the other half — with `-force` the second import wins (oracle:
    // `namespace origin ::dst::p` → `::B::p`).
    let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import -force ::B::*\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::B::p"));
}

#[test]
fn a_cross_file_exact_link_conflicts_with_an_earlier_different_source() {
    // TN — the live-alias conflict on the exact-link tier.
    let a = analyse("namespace eval ::A { proc p {} {}\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} {}\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::p\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_eq!(
        index.resolve_command_target("::dst::p"),
        "::A::p",
        "the second exact import fails and the first alias stays"
    );
}

#[test]
fn a_cross_file_import_chain_follows_to_the_original_source() {
    // TP — `::A` imports `::B::*`, `::B` imported `::C::*`, each in its
    // own file. Oracle: `::A::p` runs `::C`'s body and `namespace origin
    // ::A::p` → `::C::p`. The middle hop is in no workspace proc table,
    // so a single-hop walk abstained.
    let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { namespace import ::C::*\n namespace export p }\n");
    let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///c.tcl", &c),
        ("file:///b.tcl", &b),
        ("file:///a.tcl", &a),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::A::p".to_string(), "::p".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::C::p"), "{resolved:?}");
}

#[test]
fn a_cross_file_chain_hop_that_was_never_re_exported_abstains() {
    // FN guard — the middle hop keeps its own export gate.
    let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { namespace import ::C::* }\n");
    let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///c.tcl", &c),
        ("file:///b.tcl", &b),
        ("file:///a.tcl", &a),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::A::p".to_string(), "::p".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_mutually_importing_pair_terminates_cross_file() {
    // FP/hang guard — bounded by `MAX_COMMAND_NAME_HOPS`.
    let a = analyse("namespace eval ::A { namespace import ::B::*\n namespace export * }\n");
    let b = analyse("namespace eval ::B { namespace import ::A::*\n namespace export * }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .resolve_wildcard_import(
                "p",
                &["::A::p".to_string()],
                call_from("file:///caller.tcl"),
            )
            .is_none()
    );
}

// Exact (non-glob) `namespace import` is export-gated too.
// Real Tcl silently installs nothing when the name is
// not exported at the import's own position (oracle: `namespace eval
// ::m {proc helper {} {}}; namespace eval ::a {namespace import
// ::m::helper}` leaves `info commands ::a::*` empty and raises no error).

#[test]
fn exact_import_of_an_unexported_name_installs_no_link() {
    // FP guard (CRITICAL) — `::mymod` never exports `helper`, so the
    // exact import binds nothing: no `::app::helper` exists, the bare
    // call is not a reference to the source, and the pattern token is not
    // a link span.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(
        !index.workspace_command_exists("::app::helper"),
        "an unexported exact import installs no command",
    );
    assert!(
        index
            .linked_invocations_of("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "the bare call reaches nothing, so it is no reference",
    );
    assert!(
        index
            .link_target_spans("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "a link that was never installed has no target span",
    );
}

#[test]
fn exact_import_ignores_an_export_written_after_it() {
    // FP guard, direction B for the exact form — the export lands after
    // the import in the same file, so real Tcl still binds nothing
    // (oracle: `info commands ::a5::*` empty).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(!index.workspace_command_exists("::app::helper"));
}

#[test]
fn exact_import_survives_a_later_export_clear() {
    // TP, direction A for the exact form — the `-clear` runs after the
    // import, so the alias it already installed stays.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(index.workspace_command_exists("::app::helper"));
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
}

#[test]
fn exact_import_from_an_unseen_namespace_keeps_its_link() {
    // Abstention guard (CRITICAL for W123 silence) — `::msgcat` lives in
    // an installed package, not in any indexed document, so the index
    // holds no export declaration for it and never will. Treating that
    // silence as "not exported" would revoke `::app::mc` and hand the
    // unknown-command pass a false positive on every bare `mc` call. The
    // gate only fires where the workspace can see the namespace's own
    // definitions or exports.
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::msgcat::mc\n    proc run {} { mc hello }\n}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    assert!(
        index.workspace_command_exists("::app::mc"),
        "an import from a namespace the workspace cannot observe must not \
             be gated away",
    );
}

#[test]
fn alias_and_rename_links_are_never_export_gated() {
    // TN — only a `namespace import` link carries an export snapshot.
    // `interp alias` and `rename` introduce a name with no reference to
    // any export list, and must keep working unchanged.
    let lib = analyse("proc ::mymod::helper {} {}\n");
    let app = analyse(
        "interp alias {} ::app::a {} ::mymod::helper\nrename ::mymod::helper ::mymod::gone\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert!(
        index.workspace_command_exists("::app::a"),
        "an interp alias is not gated by any namespace export",
    );
    assert!(
        index.workspace_command_exists("::mymod::gone"),
        "a rename is not gated by any namespace export",
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
fn cross_file_oo_define_stub_does_not_hide_superclass() {
    // `::B` defines `greet`; `::C` (superclass `::B`) inherits it; a
    // cross-file `oo::define ::C` adds `extra` and names no superclass,
    // recording a second `::C` entry with empty parents.  The parent walk
    // must union both entries — otherwise the stub hides the `::B` edge and
    // `::C` is wrongly dropped from `greet`'s inheritor set.
    let b = analyse("oo::class create B {\n    method greet {} {}\n}\n");
    let c = analyse("oo::class create C {\n    superclass B\n}\n");
    let stub = analyse("oo::define C {\n    method extra {} {}\n}\n");
    // The stub is indexed *before* the real class, so a first-match parent
    // lookup would pick the stub's empty superclasses — the adversarial
    // ordering the union guards against.
    let index = WorkspaceIndex::from_documents([
        ("file:///b.tcl", &b),
        ("file:///ext.tcl", &stub),
        ("file:///c.tcl", &c),
    ]);
    let inheritors: Vec<&str> = index
        .method_inheritor_classes("::B", "greet")
        .iter()
        .map(|wc| wc.qualified_name.as_str())
        .collect();
    assert!(inheritors.contains(&"::C"), "{inheritors:?}");
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
fn namespace_declarations_and_refs_are_indexed_across_documents() {
    // TP — the declaring `namespace eval` blocks live in
    // one document and the `namespace children ::mypkg` consumer in
    // another; both spellings name the one namespace.  Oracle (tclsh
    // 9.0.4 / 8.6.16, byte-identical): reopening `::mypkg` extends the
    // same namespace, and `namespace children ::mypkg` lists its
    // children rather than erroring.
    let decl = analyse("namespace eval mypkg {}\nnamespace eval mypkg {}\n");
    let user = analyse("set t [namespace children ::mypkg]\n");
    let index =
        WorkspaceIndex::from_documents([("file:///decl.tcl", &decl), ("file:///user.tcl", &user)]);
    assert_eq!(
        index
            .namespace_declarations_qualified("::mypkg", "")
            .iter()
            .map(|n| n.uri.as_str())
            .collect::<Vec<_>>(),
        vec!["file:///decl.tcl", "file:///decl.tcl"],
        "both declaring blocks are definition sites",
    );
    assert_eq!(
        index
            .namespace_refs_of("::mypkg", "")
            .iter()
            .map(|n| n.uri.as_str())
            .collect::<Vec<_>>(),
        vec!["file:///user.tcl"],
    );
    // The declaring document is excluded on request, which is how the
    // cross-document definition tier avoids re-reporting local answers.
    assert_eq!(
        index.namespace_declarations_qualified("::mypkg", "file:///decl.tcl"),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0],
    );
}

/// Declaring rows whose name is a **strict descendant** of
/// the cell: the workspace half of the implicit-parent answer.
///
/// tclsh-proof (9.0.4 / 8.6.16, byte-identical): `namespace eval
/// ::p::q::r {}` leaves `namespace exists ::p::q` -> 1 and `namespace
/// exists ::p` -> 1, while `namespace eval ::pq::r {}` leaves `namespace
/// exists ::p` -> 0.
#[test]
fn namespace_declarations_under_finds_implicit_parent_rows() {
    let decl = analyse("namespace eval ::p::q::r {}\n");
    let other = analyse("namespace eval ::pq::r {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///decl.tcl", &decl),
        ("file:///other.tcl", &other),
    ]);
    // TP — both ancestors are answered by the one deeper block.
    assert_eq!(
        index
            .namespace_declarations_under("::p::q", "")
            .iter()
            .map(|n| n.qualified_name.as_str())
            .collect::<Vec<_>>(),
        vec!["::p::q::r"],
    );
    assert_eq!(index.namespace_declarations_under("::p", "").len(), 1);
    // TN — an exact match is a real declaration, not an implicit one.
    assert_eq!(
        index.namespace_declarations_under("::p::q::r", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0],
    );
    // TN — a segment prefix is a different namespace entirely.
    assert!(
        index
            .namespace_declarations_under("::p", "file:///decl.tcl")
            .is_empty(),
        "excluding the declaring document leaves only `::pq::r`, which is not under `::p`",
    );
    // TN — the global namespace is created by the interpreter, so nothing
    // implicitly creates it.
    assert_eq!(
        index.namespace_declarations_under("::", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
    assert_eq!(
        index.namespace_declarations_under("", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
}

#[test]
fn namespace_rows_are_dropped_with_their_document() {
    // A re-index (`remove_document` then `add_document`) must not leave
    // stale namespace rows behind — the same discipline every other
    // table follows.
    let a = analyse("namespace eval gone {}\n");
    let mut index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert_eq!(
        index.namespace_declarations_qualified("::gone", "").len(),
        1
    );
    index.remove_document("file:///a.tcl");
    assert_eq!(
        index.namespace_declarations_qualified("::gone", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
    assert!(index.namespace_refs().next().is_none());
}

#[test]
fn a_relative_namespace_word_is_indexed_rooted() {
    // TP — the analyser roots a relative spelling against its own
    // namespace before the index sees it, so a sibling document can
    // match it by exact qualified name.  Oracle: inside `namespace eval
    // ::outer`, `namespace children inner` means `::outer::inner`; the
    // same words at global scope mean `::inner` (both interpreters).
    let a = analyse(
        "namespace eval ::outer {\n    namespace eval inner {}\n    namespace children inner\n}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert_eq!(
        index
            .namespace_declarations_qualified("::outer::inner", "")
            .len(),
        1,
    );
    assert_eq!(index.namespace_refs_of("::outer::inner", "").len(), 1);
    assert_eq!(
        index.namespace_refs_of("::inner", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
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

/// TP — a document whose only stake
/// in `::ns::v` is an alias written from *outside* `::ns` is found by the
/// alias table, and by nothing else.
///
/// A global `proc p {} { namespace upvar ::ns v local; … }` declares
/// nothing in `::ns`, holds no namespace-scoped declaration of the cell,
/// and writes no qualified occurrence of it — so all three of the older
/// candidate sources miss it, while the alias it holds breaks the moment
/// the declaration moves without it (tclsh 9.0.4 / 8.6.16: `can't read
/// "local": no such variable`).
#[test]
fn indexes_a_cell_alias_written_from_another_namespace() {
    let decl = analyse("namespace eval ns {\n    variable v 1\n}\n");
    let aliaser = analyse("proc p {} {\n    namespace upvar ::ns v local\n    return $local\n}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///decl.tcl", &decl);
    index.add_document("file:///aliaser.tcl", &aliaser);

    assert_eq!(
        index.documents_aliasing_variable("::ns::v"),
        vec![
            "file:///aliaser.tcl".to_string(),
            "file:///decl.tcl".to_string()
        ],
        "both the `variable v` declaration and the out-of-namespace \
             `namespace upvar` are aliases of the one cell",
    );
    assert!(
        !index
            .documents_in_namespace("::ns")
            .contains(&"file:///aliaser.tcl".to_string()),
        "the aliasing document declares nothing in ::ns — which is why \
             the alias table is needed",
    );
    assert!(
        index
            .variable_refs_of("::ns::v", "")
            .iter()
            .all(|r| r.uri != "file:///aliaser.tcl"),
        "and it writes no qualified occurrence either",
    );
}

/// TN (same finding) — the alias table is keyed on the cell, so an alias
/// of a *different* cell never drags its document into another cell's
/// rename.
#[test]
fn cell_alias_index_does_not_match_a_different_cell() {
    let other = analyse("proc p {} {\n    namespace upvar ::ns other local\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///other.tcl", &other)]);
    assert_eq!(
        index.documents_aliasing_variable("::ns::v"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        index.documents_aliasing_variable("::ns::other"),
        vec!["file:///other.tcl".to_string()]
    );
}

/// TP — a namespace variable declared in one
/// document and read, qualified, from another is matched across the two by
/// its one `::`-rooted cell name.
#[test]
fn indexes_namespace_variables_and_their_qualified_occurrences() {
    let decl = analyse("namespace eval app::colors {\n    variable palette red\n}\n");
    let user = analyse("puts $app::colors::palette\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///decl.tcl", &decl);
    index.add_document("file:///user.tcl", &user);

    let defs = index.variable_definitions_qualified("::app::colors::palette", "");
    assert_eq!(defs.len(), 1, "one declaration: {defs:?}");
    assert_eq!(defs[0].uri, "file:///decl.tcl");
    assert_eq!(defs[0].name, "palette");

    let refs = index.variable_refs_of("::app::colors::palette", "file:///decl.tcl");
    assert_eq!(
        refs.iter().map(|r| r.uri.as_str()).collect::<Vec<_>>(),
        vec!["file:///user.tcl"],
        "the sibling document's qualified read is a reference: {refs:?}",
    );

    // TN: an unrelated cell name matches nothing.
    assert_eq!(
        index.variable_definitions_qualified("::app::colors::other", ""),
        [] as [&crate::workspace_index::WorkspaceVariable; 0],
    );
    index.remove_document("file:///decl.tcl");
    assert!(
        index
            .variable_definitions_qualified("::app::colors::palette", "")
            .is_empty(),
        "removing the declaring document drops its variables",
    );
}

/// TN — a proc **local** never enters the variable table: it has no
/// qualified name a sibling document could spell.
#[test]
fn proc_locals_are_not_indexed_as_namespace_variables() {
    let a = analyse("proc ns::f {} {\n    set localOnly 1\n}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert!(
        !index.variables().any(|v| v.name == "localOnly"),
        "indexed variables: {:?}",
        index.variables().collect::<Vec<_>>(),
    );
}

/// The generation counter must advance on every mutation, so a consumer
/// can tell the index changed without diffing it.
#[test]
fn generation_advances_on_every_mutation() {
    let a = analyse("proc helper {} {}\n");
    let mut index = WorkspaceIndex::new();
    let start = index.generation();
    index.add_document("file:///a.tcl", &a);
    let after_add = index.generation();
    assert_ne!(start, after_add, "add_document must bump the generation");
    index.remove_document("file:///a.tcl");
    assert_ne!(
        after_add,
        index.generation(),
        "remove_document must bump the generation",
    );
}

#[test]
fn document_revision_changes_only_with_that_document() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc beta {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let a_revision = index.document_revision("file:///a.tcl").unwrap();

    index.add_document("file:///b.tcl", &b);
    assert_eq!(
        index.document_revision("file:///a.tcl"),
        Some(a_revision),
        "an unrelated mutation must not invalidate a source snapshot",
    );
    index.replace_document("file:///a.tcl", &a);
    assert_ne!(
        index.document_revision("file:///a.tcl"),
        Some(a_revision),
        "replacing the document must invalidate its source snapshot",
    );
    index.remove_document("file:///a.tcl");
    assert_eq!(index.document_revision("file:///a.tcl"), None);
}

#[test]
fn conditional_revision_removal_preserves_a_newer_replacement() {
    let old = analyse("proc old {} {}\n");
    let new = analyse("proc new {} {}\n");
    let mut index = WorkspaceIndex::new();
    let old_revision = index.replace_document_with_revision("file:///a.tcl", &old);
    let new_revision = index.replace_document_with_revision("file:///a.tcl", &new);

    assert!(
        !index.remove_document_if_revision("file:///a.tcl", old_revision),
        "an obsolete publisher must not remove a newer replacement",
    );
    assert_eq!(index.document_revision("file:///a.tcl"), Some(new_revision));
    assert!(index.workspace_command_exists("::new"));
    assert!(!index.workspace_command_exists("::old"));
    assert!(index.remove_document_if_revision("file:///a.tcl", new_revision));
    assert!(!index.contains_document("file:///a.tcl"));
}

#[test]
fn document_revision_survives_slot_reuse_aba_1854() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc beta {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let old_revision = index.document_revision("file:///a.tcl").unwrap();

    index.remove_document("file:///a.tcl");
    index.add_document("file:///b.tcl", &b);
    index.add_document("file:///a.tcl", &a);

    assert_ne!(
        index.document_revision("file:///a.tcl"),
        Some(old_revision),
        "re-adding a URI must not recycle its old token after slot churn",
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

#[test]
fn remove_document_drops_invocations_too() {
    let a = analyse("helper\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert!(index.invocations().next().is_some());
    index.remove_document("file:///a.tcl");
    assert!(index.invocations().next().is_none());
}

#[test]
fn indexes_and_removes_package_requires() {
    let a = analyse("package require Tk\npackage require http\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert_eq!(
        index.package_requires_for("file:///a.tcl"),
        vec!["Tk".to_owned(), "http".to_owned()]
    );
    index.remove_document("file:///a.tcl");
    assert!(index.package_requires().next().is_none());
}

#[test]
fn source_ancestor_requires_walks_the_graph() {
    // app.tcl requires Tk and sources lib/util.tcl; util inherits Tk.
    let app = analyse("package require Tk\nsource lib/util.tcl\n");
    let util = analyse("proc u {} {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib/util.tcl", &util),
    ]);
    // Resolver mirrors the server's: join the raw path onto the parent's
    // directory (path portion of the file URI).
    let resolve = |parent: &str, raw: &str| -> Option<String> {
        let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
        Some(format!("{dir}/{raw}"))
    };
    let got = index.source_ancestor_package_requires("file:///proj/lib/util.tcl", resolve);
    assert_eq!(got, vec!["Tk".to_owned()]);
    // The entry file itself inherits nothing.
    assert_eq!(
        index.source_ancestor_package_requires("file:///proj/app.tcl", resolve),
        [] as [std::string::String; 0]
    );
}

#[test]
fn source_ancestor_requires_ignores_nonliteral_sources() {
    // A computed `source $path` produces no resolvable edge.
    let app = analyse("package require Tk\nsource $dir/util.tcl\n");
    let util = analyse("proc u {} {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/util.tcl", &util),
    ]);
    let resolve = |_p: &str, _r: &str| -> Option<String> { panic!("non-literal must not resolve") };
    assert_eq!(
        index.source_ancestor_package_requires("file:///proj/util.tcl", resolve),
        [] as [std::string::String; 0]
    );
}

/// `package prefer latest` is interpreter-global, and
/// along the `source` graph "ran first" is a static fact.
///
/// tclsh-proof (8.6.14), with `lib.tcl` holding `puts [package prefer]`:
/// `app.tcl` written as `package prefer latest; source lib.tcl` prints
/// `latest`, and as `source lib.tcl; package prefer latest` prints
/// `stable`.
#[test]
fn source_ancestor_prefer_latest_crosses_documents() {
    let resolve = |parent: &str, raw: &str| -> Option<String> {
        let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
        Some(format!("{dir}/{raw}"))
    };
    let lib = analyse("package require w\n");

    // TP — the raise runs before the `source`.
    let app = analyse("package prefer latest\nsource lib.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
    // …and the entry file itself inherits nothing (its own raise is the
    // position-sensitive single-document question).
    assert!(!index.source_ancestor_prefers_latest("file:///proj/app.tcl", resolve));

    // FP guard — the raise runs after the `source`.
    let app = analyse("source lib.tcl\npackage prefer latest\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));

    // FP guard — a conditional raise is never recorded at all.
    let app = analyse("if {$::c} { package prefer latest }\nsource lib.tcl\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///proj/app.tcl", &app),
        ("file:///proj/lib.tcl", &lib),
    ]);
    assert!(index.package_prefers().next().is_none());
    assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
}

/// The prefer rows drop with their document, like every other table.
#[test]
fn package_prefers_are_dropped_with_their_document() {
    let a = analyse("package prefer latest\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert_eq!(index.package_prefers().count(), 1);
    index.remove_document("file:///a.tcl");
    assert!(index.package_prefers().next().is_none());
}

// Both tiers on the two-file `namespace import -force` shadow.
//
// `MAIN` below is one document, byte-for-byte identical in every test of
// this group. What changes is the *rest of the program*, and with it the
// correct answer — which is exactly why the single-document tier needed a
// whole-program export oracle. Oracle transcript (tclsh 8.6.14 and 9.0.4,
// byte-identical), running `MAIN` from a loader that does or does not also
// source the `namespace export helper`:
//
//   with the export sourced first: call -> SRC    origin -> ::src::helper
//   with nothing exporting it:     call -> LOCAL  origin -> ::app::helper

/// The pinned two-file shape's main document.
const MAIN: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

/// What each tier says about the `helper` call at the end of [`MAIN`],
/// given the rest of the program.
///
/// Returns `(in-document target, cross-document target)`, where a tier's
/// "target" is the qualified name the call reaches: the local definition
/// when the `-force` import bound nothing, the import's source when it
/// did, and `None` when the tier cannot say. The two are computed
/// independently — the single-document resolver over `MAIN`'s own analysis
/// plus the index's export oracle, and the workspace's own
/// [`WorkspaceIndex::resolve_wildcard_import`] — so agreeing is a real
/// result and not a tautology.
fn both_tiers_on_main(others: &[(&str, &AnalysisResult)]) -> (Option<String>, Option<String>) {
    let main = analyse(MAIN);
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///main.tcl", &main);
    for (uri, analysis) in others {
        index.add_document(uri, analysis);
    }
    let call = u32::try_from(MAIN.rfind("    helper\n").expect("call present") + 4)
        .expect("tiny test source");
    let candidates =
        tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");

    let exports = index.export_snapshot();
    let in_document = crate::definition::resolve_called_proc(
        &main,
        MAIN,
        "::app",
        "helper",
        call,
        crate::definition::CallResolution::document_only().in_program(
            crate::definition::ProgramExports {
                uri: "file:///main.tcl",
                oracle: exports.as_ref(),
            },
        ),
    )
    .map(|p| p.qualified_name.clone());

    // The cross-document tier answers the import question; when no import
    // is live the call reaches whatever the workspace defines under the
    // first candidate, which is the local proc.
    let cross_document = index
        .resolve_wildcard_import(
            "helper",
            &candidates,
            CallSite {
                uri: "file:///main.tcl",
                at: call,
                enclosing_body: main.innermost_definition_body_span(call),
            },
        )
        .or_else(|| {
            candidates
                .iter()
                .find(|c| index.defines_command(c))
                .cloned()
        });
    (in_document, cross_document)
}

#[test]
fn both_tiers_shadow_when_another_file_holds_the_covering_export() {
    // TP (CRITICAL) — the export that decides it lives in `exports.tcl`.
    // Oracle: SRC / `::src::helper`.
    let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
    let (in_document, cross_document) = both_tiers_on_main(&[("file:///exports.tcl", &exports)]);
    assert_eq!(in_document.as_deref(), Some("::src::helper"));
    assert_eq!(
        in_document, cross_document,
        "the two tiers must reach the same command",
    );
}

#[test]
fn both_tiers_keep_the_local_when_the_whole_program_exports_nothing() {
    // TN, byte-identical `MAIN` — nothing anywhere exports `helper`, so
    // the `-force` import binds only `other` and the local definition
    // survives. Oracle: LOCAL / `::app::helper`.
    let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
    let (in_document, cross_document) =
        both_tiers_on_main(&[("file:///unrelated.tcl", &unrelated)]);
    assert_eq!(in_document.as_deref(), Some("::app::helper"));
    assert_eq!(
        in_document, cross_document,
        "the two tiers must reach the same command",
    );
}

#[test]
fn both_tiers_shadow_when_the_source_namespace_is_wholly_in_another_file() {
    // TP — the third oracle shape: `::src`'s procs *and* its export are
    // elsewhere, so this document sees only the `-force` import and the
    // local proc it deletes. Oracle: SRC / `::src::helper`.
    let main = analyse(WHOLLY_FOREIGN);
    let lib = analyse(
        "namespace eval src {\n    proc helper {} { return SRC }\n    namespace export helper\n}\n",
    );
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///main.tcl", &main);
    index.add_document("file:///lib.tcl", &lib);
    let call = u32::try_from(WHOLLY_FOREIGN.rfind("    helper\n").expect("call present") + 4)
        .expect("tiny test source");
    let candidates =
        tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");
    let exports = index.export_snapshot();
    // In-document: the source is in no local table, so the resolver cannot
    // *name* the target — but it must refuse to answer with the local
    // definition the import deleted, which is the abstention the shadow
    // gate exists for.
    let ctx = crate::definition::CallResolution::document_only().in_program(
        crate::definition::ProgramExports {
            uri: "file:///main.tcl",
            oracle: exports.as_ref(),
        },
    );
    assert!(crate::definition::forced_import_shadows_call(
        &main,
        ctx,
        "helper",
        &candidates,
        call,
    ));
    assert!(
        crate::definition::resolve_called_proc(&main, WHOLLY_FOREIGN, "::app", "helper", call, ctx)
            .is_none(),
        "the deleted local definition must not be the answer",
    );
    // …and the cross-document tier supplies the name.
    assert_eq!(
        index.resolve_wildcard_import(
            "helper",
            &candidates,
            CallSite {
                uri: "file:///main.tcl",
                at: call,
                enclosing_body: main.innermost_definition_body_span(call),
            },
        ),
        Some("::src::helper".to_owned()),
    );
}

/// The `-force` shape whose source namespace is wholly in another file.
const WHOLLY_FOREIGN: &str = "namespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

#[test]
fn the_settled_call_moves_to_the_import_source_when_the_shadow_is_live() {
    // The same fact on the *settle* path find-references reads: a
    // candidate naming the command a `-force` import deleted must not
    // settle the call, or find-references files it under a definition
    // go-to-definition no longer answers with.
    let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
    let main = analyse(MAIN);
    let index = WorkspaceIndex::from_documents([
        ("file:///main.tcl", &main),
        ("file:///exports.tcl", &exports),
    ]);
    assert_eq!(
        index
            .linked_invocations_of("::src::helper", "file:///exports.tcl")
            .len(),
        1,
        "the shadowed call is a reference to the import source",
    );
    assert!(
        index
            .linked_invocations_of("::app::helper", "file:///exports.tcl")
            .is_empty(),
        "…and not to the command the import deleted",
    );
}

#[test]
fn the_settled_call_stays_local_when_the_program_exports_nothing() {
    // TN, byte-identical `MAIN` — with nothing exporting `helper` the
    // import binds nothing and the call still belongs to the local proc.
    let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
    let main = analyse(MAIN);
    let index = WorkspaceIndex::from_documents([
        ("file:///main.tcl", &main),
        ("file:///unrelated.tcl", &unrelated),
    ]);
    assert_eq!(
        index
            .linked_invocations_of("::app::helper", "file:///unrelated.tcl")
            .len(),
        1,
        "the surviving local definition owns the call",
    );
    assert!(
        index
            .linked_invocations_of("::src::helper", "file:///unrelated.tcl")
            .is_empty(),
        "…and an import that bound nothing creates no reference",
    );
}

#[test]
fn the_export_snapshot_answers_unknown_for_a_namespace_no_file_declares() {
    // The abstention, stated directly on the oracle: `::msgcat` is an
    // installed package, in no indexed document. Silence about its exports
    // is not evidence that it exports nothing — the same rule
    // `live_command_links` already applies to exact imports.
    use crate::namespace_import::NamespaceExportOracle as _;
    let app = analyse("namespace eval app {\n    namespace import -force ::msgcat::*\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    let snapshot = index.export_snapshot();
    let site = RunPoint {
        uri: "file:///app.tcl",
        at: 0,
        enclosing_body: None,
    };
    assert_eq!(
        snapshot.exported_at("::msgcat", "mc", site),
        ExportVerdict::Unknown,
    );
    // …while a namespace the workspace *can* see gives a real negative.
    assert_eq!(
        snapshot.exported_at("::app", "mc", site),
        ExportVerdict::NotExported,
    );
}

// The settlement walk's cost must not scale with
// (invocations x in-scope imports x export rows).
//
// Asked per call, that product is a 26 s `textDocument/references` on a
// 113-file workspace.  The shape that produces it is entirely ordinary and
// is reproduced below: many load-level `namespace import NS::*`, a source
// namespace with many `namespace export` patterns, and a great many bare
// calls that resolve to no workspace command at all (every `set`, `if`,
// `expr` in the project), each of which falls through to the wildcard
// import tier.  Each such call would ask, per in-scope import, per export
// row, "had this export run when that import ran?" — a `RunOrder` walk
// over `HashMap`s keyed by owned URI strings.
//
// Nothing about an import's position depends on the call being resolved,
// so that whole question is decided once per recorded import at
// index-build time (`ExportGate`), leaving a hash probe and a glob match
// on the per-call path.

/// A workspace of that shape, sized so a per-call gate costs seconds and
/// the indexed one costs milliseconds.
fn wildcard_import_heavy_workspace(
    importers: usize,
    exports: usize,
    calls_per_file: usize,
) -> Vec<(String, AnalysisResult)> {
    use std::fmt::Write as _;
    let patterns: Vec<String> = (0..exports).map(|i| format!("Exported{i}")).collect();
    let lib = format!(
        "namespace eval ::Lib {{\n    namespace export {}\n    proc helper {{}} {{}}\n}}\n",
        patterns.join(" "),
    );
    let mut docs = vec![("file:///lib.tcl".to_owned(), analyse(&lib))];
    for f in 0..importers {
        // A load-level glob import, then bare calls that settle to no
        // workspace command — the ones that reach the wildcard tier.
        let mut src = String::from("namespace import ::Lib::*\n");
        for c in 0..calls_per_file {
            writeln!(src, "noSuchCommand{c} a b").expect("writing to a String cannot fail");
        }
        docs.push((format!("file:///f{f}.tcl"), analyse(&src)));
    }
    docs
}

#[test]
fn settling_invocations_does_not_scale_with_imports_times_exports() {
    let docs = wildcard_import_heavy_workspace(60, 40, 60);
    let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
    let started = std::time::Instant::now();
    // The first call builds the settled-target view for the whole
    // workspace, which is where the cost lived.
    let hits = index.linked_invocations_of("::Lib::helper", "");
    let elapsed = started.elapsed();
    assert!(
        hits.is_empty(),
        "no call names ::Lib::helper, so this is the empty answer the walk had to work for",
    );
    // Measured on this shape, debug build: 81 ms with the gate hoisted,
    // 7.59 s with `import_hop` restored to asking `exports_name_at` per
    // call.  The bound sits between them with a 25x margin below and a
    // 3.8x margin above, so it cannot flake on a loaded CI box and still
    // fails outright if the per-call export walk comes back.
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "settling {} documents took {elapsed:?} — the per-call export walk is back (#1297)",
        docs.len(),
    );
}

/// The same workspace, asserting the *memo* rather than the wall clock:
/// a second reading of an unchanged index must serve the cached view, so
/// the cost above is paid once per generation and not once per request.
/// (`code_lenses` asks once per proc *and* once per class.)
#[test]
fn the_settled_target_view_is_served_from_cache_within_a_generation() {
    let docs = wildcard_import_heavy_workspace(8, 6, 4);
    let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
    let _ = index.linked_invocations_of("::Lib::missing", "");
    let first = index.settled_invocations[1].settled_documents();
    let _ = index.linked_invocations_of("::Lib::missing", "");
    assert!(
        first == index.settled_invocations[1].settled_documents(),
        "an unchanged index must serve the settled-target cache, not rebuild it",
    );
}

/// TP/TN on the hoisted export gate itself, through the public answer: an
/// exported name really does resolve through a wildcard import, and an
/// unexported sibling really does not.  This is the behaviour the
/// `ExportGate` precomputation must preserve exactly — the perf tests
/// above would happily pass on a gate that answered "no" to everything.
#[test]
fn the_precomputed_export_gate_still_admits_exactly_what_is_exported() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc shown {} {}\n    proc hidden {} {}\n    namespace export shown\n}\n",
    );
    let app = analyse("namespace import ::Lib::*\nshown\nhidden\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    // TP: `shown` is exported, so the bare call reaches it through the import.
    assert_eq!(
        index.linked_invocations_of("::Lib::shown", "").len(),
        1,
        "an exported name must still resolve through the wildcard import",
    );
    // TN: `hidden` is not exported, so the bare call reaches nothing —
    // tclsh 9.0.4 answers `invalid command name` for it.
    assert!(
        index.linked_invocations_of("::Lib::hidden", "").is_empty(),
        "an unexported sibling must not be reachable through the import",
    );
}

/// FN guard for the literal/glob split inside [`ExportGate`]: a *glob*
/// export pattern must keep working, since only metacharacter-free
/// patterns may take the hash-set path.
#[test]
fn a_glob_export_pattern_still_covers_the_names_it_matches() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc getThing {} {}\n    proc setThing {} {}\n    namespace export get*\n}\n",
    );
    let app = analyse("namespace import ::Lib::*\ngetThing\nsetThing\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert_eq!(index.linked_invocations_of("::Lib::getThing", "").len(), 1);
    assert!(
        index
            .linked_invocations_of("::Lib::setThing", "")
            .is_empty(),
        "`get*` does not cover `setThing`",
    );
}

// The import-conflict rule's "already exists" side must
// include the commands the *registry* declares, not only the procs and
// classes the workspace declares.
//
// Every expectation below is oracle-verified on tclsh 9.0.4 (built from
// core-9-0-4) and 8.6.14, byte-identical:
//
//   namespace eval ::Foo { proc set {a b} {…}; namespace export set }
//   namespace import ::Foo::*   -> can't import command "set": already exists
//                                  namespace origin ::set  == ::set
//   namespace import ::Foo::set -> same error (the exact-pattern twin)
//   namespace import -force ::Foo::*  -> namespace origin ::set == ::Foo::set
//   namespace eval ::Bar { namespace import ::Foo::* }
//                               -> namespace origin ::Bar::set == ::Foo::set
//   # …and `+` is NOT a global command, so it does not conflict:
//   info commands ::+           -> {}      (+ 1 2 -> invalid command name)

/// TP: the defect itself.  `::Foo` exports `set`; real Tcl refuses the
/// import, so the bare `set x 1` reaches the builtin and is **not** a
/// reference to `::Foo::set`.
#[test]
fn a_wildcard_import_does_not_bind_a_name_the_registry_already_declares() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::*\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index.linked_invocations_of("::Foo::set", "").is_empty(),
        "the import raised `already exists` and installed nothing, so the \
             bare call reaches the builtin",
    );
}

/// TN: `-force` is the one import that *does* replace a command the
/// target namespace already holds, so the call really is a reference.
#[test]
fn a_forced_wildcard_import_still_shadows_a_registry_builtin() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import -force ::Foo::*\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Foo::set", "").len(),
        1,
        "`namespace origin ::set` is `::Foo::set` after a -force import",
    );
}

/// FP guard: the rule is about the **target** namespace's command table,
/// and the builtins live in `::`.  Importing into `::Bar` binds
/// `::Bar::set` with no conflict at all, so a call from `::Bar` must
/// still resolve — the gate must not fire on the bare name alone.
#[test]
fn importing_a_builtin_name_into_a_sub_namespace_still_binds() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace eval ::Bar {\n    namespace import ::Foo::*\n    proc go {} { set x 1 }\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Foo::set", "").len(),
        1,
        "`::Bar::set` is not a builtin, so the import installs and the \
             call from ::Bar reaches it",
    );
}

/// TN: an ordinary, non-builtin exported name is unaffected — the gate
/// must not have made the whole wildcard tier answer "no".
#[test]
fn a_wildcard_import_of_a_non_builtin_name_is_unaffected() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc mything {} {}\n    namespace export mything\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::*\nmything\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.linked_invocations_of("::Foo::mything", "").len(), 1);
}

/// FN guard for the operator spellings, which is where a naive
/// "`registry.get(name).is_some()`" gate would break a *working* import:
/// `+` is not a command in `::` at all (`info commands ::+` is empty in a
/// fresh tclsh 9.0.4), so exporting `+` and importing it into `::`
/// genuinely binds — `namespace origin ::+` answers `::Ops::+`.
#[test]
fn importing_an_operator_name_into_the_global_namespace_still_binds() {
    let a = analyse_as(
        "namespace eval ::Ops {\n    proc + {a b} { return OPS }\n    namespace export +\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Ops::*\n+ 1 2\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Ops::+", "").len(),
        1,
        "a bare operator spelling is the post-`namespace import \
             ::tcl::mathop::*` form, not a command a fresh interpreter holds",
    );
}

/// The exact-pattern twin: `namespace import ::Foo::set` is refused for
/// the identical reason, so the link it would introduce is not live.
#[test]
fn an_exact_import_of_a_registry_builtin_installs_no_link() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::set\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index.linked_invocations_of("::Foo::set", "").is_empty(),
        "an exact import of an already-existing name installs nothing either",
    );
}

/// Dialect sensitivity: the gate must read the *document's own* dialect,
/// not a workspace-wide guess.  `HTTP::uri` is a command only the
/// `f5-irules` registry declares, so an import of it conflicts there and
/// binds under plain Tcl.
#[test]
fn the_builtin_gate_reads_each_documents_own_dialect() {
    let lib = "namespace eval ::Mine {\n    proc uri {} {}\n    namespace export uri\n}\n";
    let importer =
        "namespace eval ::HTTP {\n    namespace import ::Mine::*\n    proc go {} { uri }\n}\n";
    // Plain Tcl knows no `::HTTP::uri`, so the import installs.
    let a = analyse_as(
        lib,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        importer,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let plain = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        plain.linked_invocations_of("::Mine::uri", "").len(),
        1,
        "plain Tcl has no ::HTTP::uri to conflict with",
    );
    // The iRules registry does, so the same source conflicts.
    let a = analyse_as(lib, tcl_dialect::DialectProfile::irules());
    let b = analyse_as(importer, tcl_dialect::DialectProfile::irules());
    let irules = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        irules.linked_invocations_of("::Mine::uri", "").is_empty(),
        "f5-irules declares ::HTTP::uri, so the unforced import installs nothing",
    );
}

// `import_hop` must not scan a namespace's whole import list per call.
//
// Both of its first two filters — the tail pattern covers the word, and
// the export gate admits it — depend only on the word, so the admissible
// set is a build-time fact (`NamespaceImports`). On a large corpus such a
// scan is 28 369 calls over 3 987 739 rows, ~390 ms of the ~420 ms a
// `textDocument/references` takes after an edit.
//
// The risk in indexing it is order: `import_hop` breaks ties on the
// position *within the filtered sequence*, so a reordering — or a
// duplicate from a row carrying both literal and glob export patterns —
// would silently change which import wins. These pin that.

/// The admitted sequence must be exactly what a full scan would yield,
/// in the same order, over every shape the gate can take: literal-only,
/// glob-only, both at once (the duplicate hazard), and a tail pattern
/// that excludes a name the gate would otherwise admit.
#[test]
fn admitting_matches_a_full_scan_in_order() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc alpha {} {}\n    proc beta {} {}\n    proc gamma {} {}\n    namespace export alpha beta gamma\n}\n",
    );
    let globby = analyse(
        "namespace eval ::Glob {\n    proc alpha {} {}\n    proc beta {} {}\n    namespace export a*\n}\n",
    );
    let app = analyse(
        "namespace import ::Lib::*\nnamespace import ::Glob::*\nnamespace import ::Lib::b*\nalpha\nbeta\ngamma\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///lib.tcl", &lib),
        ("file:///glob.tcl", &globby),
        ("file:///app.tcl", &app),
    ]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci
        .imports_by_ns
        .get("::")
        .expect("the three imports are all at global level");
    for word in ["alpha", "beta", "gamma", "delta", "", "a", "*"] {
        let scanned: Vec<_> = imports
            .rows
            .iter()
            .filter(|row| tcl_syntax::glob::string_match(&row.imp.tail_pattern, word))
            .filter(|row| row.exported.covers(word))
            .map(|row| (row.imp.uri.as_str(), row.imp.at))
            .collect();
        let admitted: Vec<_> = imports
            .admitting(word)
            .map(|row| (row.imp.uri.as_str(), row.imp.at))
            .collect();
        assert_eq!(
            admitted, scanned,
            "admitting({word:?}) must equal the full scan, in order",
        );
    }
}

/// A row whose gate holds **both** a literal and a glob pattern is
/// reachable through both halves of the index and must still appear once.
/// A duplicate would shift every later row's tie-break position.
#[test]
fn a_row_indexed_twice_is_admitted_once() {
    let lib = analyse(
        "namespace eval ::Both {\n    proc alpha {} {}\n    namespace export alpha a*\n}\n",
    );
    let app = analyse("namespace import ::Both::*\nalpha\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci.imports_by_ns.get("::").expect("one global import");
    assert_eq!(
        imports.admitting("alpha").count(),
        1,
        "`alpha` matches both the literal and the `a*` pattern of one gate",
    );
}

/// TP through the public answer: the indexed path still resolves a call
/// that only a wildcard import can reach. The perf guards below would be
/// happy with an index that admitted nothing at all.
#[test]
fn the_admission_index_still_resolves_a_wildcard_imported_call() {
    let lib =
        analyse("namespace eval ::Lib {\n    proc helper {} {}\n    namespace export helper\n}\n");
    let app = analyse("namespace import ::Lib::*\nhelper\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert_eq!(index.linked_invocations_of("::Lib::helper", "").len(), 1);
}

/// A gate that exports nothing admits nothing, so such rows drop out of
/// every lookup — the case that makes the index worth having, since a
/// workspace routinely imports namespaces it holds no exports for
/// (`::tcl::mathop`, `::tcltest`).
#[test]
fn an_import_of_an_unexporting_namespace_admits_no_word() {
    let app = analyse("namespace import ::tcl::mathop::*\nset x 1\n+ 1 2\n");
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci.imports_by_ns.get("::").expect("one global import");
    for word in ["set", "+", "puts", "anything"] {
        assert_eq!(
            imports.admitting(word).count(),
            0,
            "no export rows for ::tcl::mathop, so nothing is admissible",
        );
    }
}

#[test]
fn conditional_namespace_horizon_keeps_binding_and_report_purposes_separate() {
    let (module, export, import) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &module),
        ("file:///p/exp.tcl", &export),
        ("file:///p/imp.tcl", &import),
        ("file:///p/app.tcl", &app),
    ]);
    let candidates = ["::app::helper".to_owned(), "::helper".to_owned()];
    let advice = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_from("file:///p/caller.tcl"),
        )
        .unwrap();
    assert!(
        advice.compatibility_report().is_none(),
        "the source route cannot use a later export"
    );
    assert!(advice.has_unresolved_operation_debt());
    assert!(
        advice
            .debts()
            .contains(&ConditionalNamespaceDebt::UnknownAlternatives)
    );
    assert!(advice.matches_index(&index));
    assert_eq!(
        index
            .resolve_wildcard_import_for_binding(
                "helper",
                &candidates,
                call_from("file:///p/caller.tcl")
            )
            .as_deref(),
        Some("::mymod::helper"),
        "readonly horizon advice cannot suppress a binding candidate"
    );
}

#[test]
fn conditional_namespace_forget_preserves_unknowns_and_current_index_owner() {
    let (module, export, import) = import_order_modules();
    let app = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app {namespace forget ::mymod::helper}\n",
    );
    let mut index = sourced_index([
        ("file:///p/mod.tcl", &module),
        ("file:///p/exp.tcl", &export),
        ("file:///p/imp.tcl", &import),
        ("file:///p/app.tcl", &app),
    ]);
    let candidates = ["::app::helper".to_owned(), "::helper".to_owned()];
    let advice = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_at("file:///p/app.tcl", u32::MAX),
        )
        .unwrap();
    assert!(advice.compatibility_report().is_none());
    assert!(
        advice.has_unresolved_operation_debt(),
        "empty report is never namespace absence"
    );
    assert_eq!(
        index
            .resolve_wildcard_import_for_binding(
                "helper",
                &candidates,
                call_at("file:///p/app.tcl", u32::MAX)
            )
            .as_deref(),
        Some("::mymod::helper")
    );
    let dynamic = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app {namespace forget $unknown}\n",
    );
    index.replace_document("file:///p/app.tcl", &dynamic);
    assert!(!advice.matches_index(&index));
    let unknown = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_at("file:///p/app.tcl", u32::MAX),
        )
        .unwrap();
    assert_eq!(unknown.compatibility_report(), Some("::mymod::helper"));
    assert!(
        unknown
            .debts()
            .contains(&ConditionalNamespaceDebt::UnrepresentedLifecycle)
    );
}
