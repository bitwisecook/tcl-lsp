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

//! W315: a class or per-object definition that cannot run, and the move records behind it.

use super::*;

// W315, "this class definition cannot run".

#[test]
fn w315_fires_for_every_definition_aborting_retraction() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for (body, want) in [
        (
            "deletemethod ghost\nmethod ghost {} { return g }",
            "this class definition cannot run: method \"ghost\" does not exist",
        ),
        (
            "self { method cm {} { return c } }\ndeletemethod cm",
            "this class definition cannot run: method \"cm\" does not exist",
        ),
        (
            "method a {} { return a }\nmethod b {} { return b }\nrenamemethod a b",
            "this class definition cannot run: method called \"b\" already exists",
        ),
        (
            "method a {} { return a }\nrenamemethod a a",
            "this class definition cannot run: cannot rename method to itself",
        ),
        (
            "method b {} { return b }\nrenamemethod ghost b",
            "this class definition cannot run: method \"ghost\" does not exist",
        ),
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(w315_messages(&r), vec![want.to_string()], "body: {body}");
        assert!(r.all_classes.contains_key("::C"), "body: {body}");
    }
}

#[test]
fn w315_fires_on_the_class_side_too() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method im {} { return i }\nself { deletemethod im }",
        "method im {} { return i }\nself { renamemethod im other }",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(
            w315_messages(&r),
            vec!["this class definition cannot run: method \"im\" does not exist".to_string()],
            "body: {body}",
        );
        assert!(r.all_classes["::C"].methods.contains_key("im"), "{body}");
    }
}

fn logical_object_abort_analyser() -> Analyser {
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "explicit-object-abort-advice",
        &[],
        "Logical object definition advice",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let input = super::super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    assert!(input.has_logical_source_name_context());
    Analyser::new().with_resolved_input(input)
}

fn injected_class_abort_candidates() -> ClassDef {
    ClassDef {
        definition_aborts: [
            DefinitionAbortKind::MissingMember,
            DefinitionAbortKind::DestinationExists,
            DefinitionAbortKind::RenameToItself,
        ]
        .into_iter()
        .map(|kind| DefinitionAbort {
            kind,
            member: "missing".to_owned(),
            span: Span::new(0, 1),
        })
        .collect(),
        retracted_members: vec![MemberRetractionRecord::deletion(
            "missing".to_owned(),
            MemberSide::Instance,
        )],
        ..Default::default()
    }
}

#[test]
fn original_class_abort_refuses_native_and_hosted_reporting_tables() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Positive injected candidates check admission, not runtime retractions.
    let source = "oo::class create C {deletemethod ghost; method ghost {} {}}";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-tmsh",
        "f5-iapps",
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, dialect);
        assert!(!has_w315(&analyser.result), "{dialect}");
        analyser.result.lexical_declaration_advice = true;
        analyser.result.dialect = "explicit-object-abort-advice".to_owned();
        analyser.result.original_vendor_source_names.clear();
        analyser.result.all_classes.clear();
        let mut class = injected_class_abort_candidates();
        assert_eq!(class.definition_aborts.len(), 3);
        assert!(!class.via_define);
        analyser.emit_w315_definition_cannot_run(&mut class);
        assert!(
            !has_w315(&analyser.result),
            "{dialect}: reports grant no Native closure"
        );
        assert_eq!(
            class.definition_aborts,
            [] as [crate::analyser::types::DefinitionAbort; 0]
        );
        assert_eq!(class.retracted_members.len(), 1);
    }
}

#[test]
fn original_class_abort_refuses_missing_retained_input() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    let mut analyser = Analyser::new();
    analyser.result.lexical_declaration_advice = true;
    analyser.result.dialect = "explicit-object-abort-advice".to_owned();
    assert!(analyser.result.resolved_input.is_none());
    let mut class = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut class);
    assert!(!has_w315(&analyser.result));
    assert_eq!(
        class.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(class.retracted_members.len(), 1);
}

#[test]
fn original_class_abort_keeps_retained_logical_advice_despite_report_mutations() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    let mut analyser = logical_object_abort_analyser();
    analyser.result = analyser.analyse("", "explicit-object-abort-advice");
    assert!(analyser.result.allows_retained_logical_declaration_advice());
    analyser.result.dialect = "tcl9.0".to_owned();
    analyser.result.all_classes.clear();
    analyser.result.command_invocations.clear();
    analyser.result.all_procs.clear();
    let mut class = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut class);
    assert_eq!(w315_messages(&analyser.result).len(), 3);
    assert_eq!(
        class.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(
        class.retracted_members,
        [] as [crate::analyser::types::MemberRetractionRecord; 0]
    );
    analyser.result.lexical_declaration_advice = false;
    let mut withdrawn = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut withdrawn);
    assert_eq!(w315_messages(&analyser.result).len(), 3);
    assert_eq!(
        withdrawn.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(withdrawn.retracted_members.len(), 1);
}

fn inject_object_abort_candidates(analyser: &mut Analyser) {
    analyser
        .result
        .instance_classes
        .insert("owned".to_owned(), "::C".to_owned());
    analyser
        .result
        .created_instance_commands
        .insert("owned".to_owned());
    analyser.result.object_methods.clear();
    analyser.objdefine_unresolved_receiver = false;
    for kind in [
        DefinitionAbortKind::MissingMember,
        DefinitionAbortKind::DestinationExists,
        DefinitionAbortKind::RenameToItself,
    ] {
        analyser.objdefine_abort_candidates.push(
            super::super::super::state::ObjdefineAbortCandidate {
                abort: DefinitionAbort {
                    kind,
                    member: "missing".to_owned(),
                    span: Span::new(0, 1),
                },
                receiver: "owned".to_owned(),
                prior_state_conditional: false,
            },
        );
    }
}

#[test]
fn original_object_abort_refuses_native_and_hosted_reporting_tables() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    // Injected report candidates test admission, without asserting execution.
    let source =
        "oo::class create C {}\nset o [C new]\noo::objdefine $o {method a {} {}; renamemethod a a}";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-tmsh",
        "f5-iapps",
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, dialect);
        assert!(!has_w315(&analyser.result), "{dialect}");
        analyser.result.lexical_declaration_advice = true;
        analyser.result.dialect = "explicit-object-abort-advice".to_owned();
        inject_object_abort_candidates(&mut analyser);
        assert_eq!(analyser.objdefine_abort_candidates.len(), 3);
        analyser.flush_objdefine_abort_diagnostics();
        assert!(
            !has_w315(&analyser.result),
            "{dialect}: report maps grant no Native table"
        );
        assert_eq!(
            analyser.objdefine_abort_candidates,
            [] as [crate::analyser::state::ObjdefineAbortCandidate; 0]
        );
    }
}

#[test]
fn original_object_abort_refuses_missing_retained_input() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    let mut analyser = Analyser::new();
    analyser.result.lexical_declaration_advice = true;
    assert!(analyser.result.resolved_input.is_none());
    inject_object_abort_candidates(&mut analyser);
    analyser.flush_objdefine_abort_diagnostics();
    assert!(!has_w315(&analyser.result));
    assert_eq!(
        analyser.objdefine_abort_candidates,
        [] as [crate::analyser::state::ObjdefineAbortCandidate; 0]
    );
}

#[test]
fn original_object_abort_keeps_explicit_logical_advice_despite_report_label() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    let mut analyser = logical_object_abort_analyser();
    analyser.result = analyser.analyse("", "explicit-object-abort-advice");
    assert!(analyser.result.allows_lexical_declaration_advice());
    assert!(
        analyser
            .result
            .resolved_input
            .as_ref()
            .unwrap()
            .has_logical_source_name_context()
    );
    analyser.result.dialect = "tcl9.0".to_owned();
    inject_object_abort_candidates(&mut analyser);
    analyser.flush_objdefine_abort_diagnostics();
    assert_eq!(w315_messages(&analyser.result).len(), 3);
}

// Per-object abort reports use explicitly selected Logical source advice.
// The String table supplies no Native own-method presence or completeness.

#[test]
fn w315_fires_for_a_per_object_retraction_of_a_never_declared_member() {
    // Explicit Logical compatibility uses the authored object's own table,
    // independently of inherited class metadata. No Native absence is proved.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method ghost {} { return g } }\n\
             set o [::C new]\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: method \"ghost\" does not exist".to_string()],
    );
}

#[test]
fn w315_fires_for_a_per_object_rename_onto_an_existing_member() {
    // Explicit Logical source order retains a destination declared by an
    // earlier block of the same authored binding. It grants no Native table.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method a {} { return a }\n method b {} { return b } }\n\
             oo::objdefine $o { renamemethod a b }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: method called \"b\" already exists".to_string()],
    );
}

#[test]
fn w315_fires_for_a_per_object_rename_to_itself() {
    // The Logical model selects the same source and destination name;
    // the compatibility assertion does not grant a Native worker receipt.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::objdefine $o { method x {} { return x }\n renamemethod x x }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: cannot rename method to itself".to_string()],
    );
}

#[test]
fn per_object_w315_abstains_without_the_receivers_construction_in_view() {
    // Logical metadata for an extension lacks a closed authored receiver
    // table; a declaration in another file remains possible.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn per_object_w315_abstains_when_any_key_declares_the_member() {
    // An authored alias can refer to a member declared under another key.
    // Logical metadata with that possibility cannot assert its absence.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set p [::C new]\n\
             oo::objdefine $p { method ghost {} { return g } }\n\
             set o [::C new]\n\
             set o $p\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn per_object_w315_abstains_when_any_receiver_is_unresolved() {
    // An unknown authored receiver can affect any object's source table.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine [pick] { method ghost {} { return g } }\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn w315_stays_silent_for_every_legal_order() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method a {} {}\nmethod b {} {}\ndeletemethod a",
        "self { method a {} {}\nmethod b {} {}\ndeletemethod a }",
        "method a {} {}\nrenamemethod a fresh",
        "method a {} {}\nself { method b {} {} }\nrenamemethod a b",
        "method a {} {}\ndeletemethod a\nmethod a {} {}",
        "method a {} {}\nmethod b {} {}\ndeletemethod b\nrenamemethod a b",
        "method a {} {}\nrenamemethod a b\nrenamemethod b c",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
    }
}

#[test]
fn a_cross_side_visibility_word_is_not_a_w315() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method onlyinst {} {}\nself unexport onlyinst",
        "self { method onlyclass {} {} }\nunexport onlyclass",
        "export ghost",
        "self export ghost",
        "filter ghost",
        "self filter ghost",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
    }
}

#[test]
fn w315_abstains_on_a_dynamic_source_name() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method m {} {}\ndeletemethod $gone",
        "method m {} {}\nrenamemethod $old new",
        "method m {} {}\ndeletemethod [pick]",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
        assert!(
            r.all_classes["::C"].methods.contains_key("m"),
            "body: {body}: a dynamic retraction must not remove a member",
        );
        assert_eq!(
            r.all_classes["::C"].retracted_members.len(),
            0,
            "body: {body}: {:?}",
            r.all_classes["::C"].retracted_members,
        );
    }
}

#[test]
fn a_dynamic_rename_destination_still_retracts_the_source() {
    // TN for the *destination* half, which is not symmetric with the source.
    // Whatever `$n` turns out to be, `renamemethod m $n` definitely takes
    // `m` away — `::C m` is an unknown method afterwards on 9.0.4 and
    // 8.6.14 alike — so retracting the source is the faithful answer even
    // though the arrival name is unknowable. Only the arrival abstains, and
    // with no static destination there is nothing to check for a collision.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m }\n\
             method n {} { return n }\n\
             renamemethod m $n }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("m"), "{:?}", c.methods);
    assert!(
        c.methods.contains_key("n"),
        "the collision guess is not made"
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn a_cross_file_stub_retraction_is_a_tombstone_not_a_w315() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }\n\
             oo::define ::E { renamemethod old new }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    assert_eq!(
        r.all_classes["::C"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "m".to_string(),
            MemberSide::Instance
        )],
    );
    assert!(r.all_classes["::C"].via_define);
}

#[test]
fn a_same_file_define_extending_a_local_class_is_not_a_stub() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this class definition cannot run: method \"ghost\" does not exist".to_string()],
    );
    assert!(!r.all_classes["::C"].via_define);
    assert_eq!(
        r.all_classes["::C"].retracted_members.len(),
        0,
        "{:?}",
        r.all_classes["::C"].retracted_members,
    );
}

#[test]
fn a_move_records_the_member_state_it_ran_against() {
    // The moved member's declaration site is the
    // `renamemethod`'s destination word, so a *rename* of it rewrites that
    // word — and the gate that decides whether the result still runs needs
    // the table as it stood at the move. Captured with the source already
    // removed and the destination not yet inserted, which is exactly what
    // the interpreter reads at that point.
    let src = "oo::class create ::C { method old {} { return 1 }\n\
                   method sib {} { return 2 }\n\
                   renamemethod old new }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.renamed_members.len(), 1, "{:?}", c.renamed_members);
    let moved = &c.renamed_members[0];
    assert_eq!(moved.source, "old");
    assert_eq!(moved.destination, "new");
    assert_eq!(moved.side, MemberSide::Instance);
    assert_eq!(
        &src[moved.destination_span.start() as usize..moved.destination_span.end() as usize],
        "new",
    );
    // `sib` is live at the move; `old` is not (it was just retracted) and
    // `new` is not (it has not arrived yet).
    assert_eq!(moved.blocked, vec!["sib".to_string()]);
}

#[test]
fn the_move_record_answers_which_new_names_would_abort() {
    // The shared fold both consumers use: the walker asks it about the
    // destination actually written (that is how W315 is decided) and the
    // rename gate asks it about the name the user typed. Oracle, identical
    // on tclsh 9.0.4 and 8.6.14 — no class is created in either case:
    //   { method old {} {…} ; renamemethod old old }
    //        -> cannot rename method to itself
    //   { method old {} {…} ; method sib {} {…} ; renamemethod old sib }
    //        -> method called sib already exists
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             renamemethod old new }",
        "tcl9.0",
    );
    let moved = &r.all_classes["::C"].renamed_members[0];
    assert_eq!(
        moved.abort_if_renamed_to("old"),
        Some(DefinitionAbortKind::RenameToItself),
    );
    assert_eq!(
        moved.abort_if_renamed_to("sib"),
        Some(DefinitionAbortKind::DestinationExists),
    );
    // TN: a fresh name is fine, and so is the destination it already has.
    assert_eq!(moved.abort_if_renamed_to("fresh"), None);
    // The mirror direction, from the same record.
    assert!(moved.collides_with_renaming("sib", "new"));
    assert!(!moved.collides_with_renaming("sib", "other"));
}

#[test]
fn a_cross_side_name_is_not_in_the_move_snapshot() {
    // TN (CRITICAL). Collisions are side-local: `method old {} {…}` +
    // `self { method sib {} {…} }` + `renamemethod old sib` runs fine and
    // leaves `info class methods ::C1` -> sib (9.0.4 / 8.6.14). So a
    // class-side `sib` must not appear in an instance-side move's snapshot.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             self { method sib {} { return 2 } }\n\
             renamemethod old new }",
        "tcl9.0",
    );
    let moved = &r.all_classes["::C"].renamed_members[0];
    assert_eq!(moved.blocked.len(), 0, "{:?}", moved.blocked);
    assert_eq!(moved.abort_if_renamed_to("sib"), None);
}

#[test]
fn the_move_snapshot_honours_body_order_in_both_directions() {
    // TN ×2, both oracle-pinned legal on 9.0.4 and 8.6.14:
    //   { method old {} {} ; method sib {} {} ; deletemethod sib
    //     renamemethod old sib }                 -> info class methods -> sib
    //   { method old {} {} ; renamemethod old sib ; method sib {} {} }
    //                                            -> info class methods -> sib
    // so a name deleted *before* the move is free, and one declared
    // *after* it never collides with it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             deletemethod sib\n\
             renamemethod old new }",
        "tcl9.0",
    );
    assert_eq!(
        r.all_classes["::C"].renamed_members[0].abort_if_renamed_to("sib"),
        None,
        "a name deleted before the move is free",
    );
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::D { method old {} { return 1 }\n\
             renamemethod old new\n\
             method sib {} { return 2 } }",
        "tcl9.0",
    );
    assert_eq!(
        r.all_classes["::D"].renamed_members[0].abort_if_renamed_to("sib"),
        None,
        "a name declared after the move never collides with it",
    );
}

#[test]
fn a_rejected_rename_records_no_move() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method a {} {}\nrenamemethod a a",
        "method a {} {}\nmethod b {} {}\nrenamemethod a b",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(
            r.all_classes["::C"].renamed_members.len(),
            0,
            "body: {body}: {:?}",
            r.all_classes["::C"].renamed_members,
        );
        assert!(has_w315(&r), "body: {body}");
    }
}

#[test]
fn a_blocked_rename_keeps_both_members_navigable() {
    // Navigation-resilience pin for the aborting shapes. Real Tcl creates no
    // class at all, so there is no "right" answer to mirror — the useful one
    // keeps every declaration the author wrote reachable rather than letting
    // the wreckage take one with it.
    let src = "oo::class create ::C { method a {} { return a }\n\
                   method b {} { return b }\n\
                   renamemethod a b }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.methods.contains_key("a"), "source kept: {:?}", c.methods);
    let b = c.methods.get("b").expect("destination kept");
    assert!(
        src[b.body_span.start() as usize..b.body_span.end() as usize].contains("return b"),
        "the destination keeps its OWN body",
    );
}

#[test]
fn w315_is_reported_once_per_offending_word() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }\n\
             oo::define ::C { method extra {} { return e } }\n\
             oo::define ::C { deletemethod other }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec![
            "this class definition cannot run: method \"ghost\" does not exist".to_string(),
            "this class definition cannot run: method \"other\" does not exist".to_string(),
        ],
    );
}

#[test]
fn w315_points_at_the_offending_word() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let src = "oo::class create ::C { deletemethod ghost }";
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(src, "explicit-object-abort-advice");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == tcl_core_types::DiagCode::W315)
        .expect("W315");
    assert_eq!(
        &src[d.span.start() as usize..d.span.end() as usize],
        "ghost"
    );
}

#[test]
fn a_stub_retraction_of_an_undeclared_member_leaves_a_tombstone() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Explicit Logical cross-document advisory channel; no Native class or retraction.
    // TP for the cross-document channel. This document
    // is the `oo::define ::C { deletemethod m }` half of a two-file program:
    // it has no local `m` to remove, so the removal has to travel as a
    // tombstone or the workspace keeps advertising a method that sourcing
    // this file deletes. The side travels with it — a `self`-scoped
    // retraction tombstones the class-object side.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        r.all_classes["::C"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "m".to_string(),
            MemberSide::Instance
        )],
    );
    assert_eq!(
        r.all_classes["::D"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "cm".to_string(),
            MemberSide::ClassObject
        )],
    );
}

#[test]
fn a_stub_renamemethod_tombstones_only_the_source_name() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Explicit Logical cross-document advisory channel; no Native move.
    // TN for the tombstone's shape: `renamemethod old new` removes `old` and
    // *creates* `new` (oracle: `info class methods ::I3` -> new, and
    // `[::I3 new] new` answers), so tombstoning `new` would suppress a live
    // member another document legitimately declares. Which arguments a
    // retracting word removes is registry data
    // ([`tcl_registry::definer::MemberRetraction::FirstArgument`]), not a
    // keyword match here.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { renamemethod old new }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        r.all_classes["::C"]
            .retracted_members
            .iter()
            .map(|r| (r.member.clone(), r.side))
            .collect::<Vec<_>>(),
        vec![("old".to_string(), MemberSide::Instance)],
    );
    // …and the arrival travels with it: the stub has no
    // `MethodDef` to move, so the destination name is what lets the
    // workspace join re-key the defining file's record.
    assert_eq!(
        r.all_classes["::C"].retracted_members[0].arrival.as_deref(),
        Some("new"),
    );
}

#[test]
fn missing_class_source_cannot_publish_native_definition_stubs() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    let source = "oo::define ::C {deletemethod m}; oo::define ::D {renamemethod old new}";
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"] {
        let result = Analyser::new().analyse(source, profile);
        assert!(!result.all_classes.contains_key("::C"), "{profile}");
        assert!(!result.all_classes.contains_key("::D"), "{profile}");
    }
    let mut missing = logical_object_abort_analyser();
    let mut baseline = logical_object_abort_analyser();
    missing.result = baseline.analyse(source, "explicit-object-abort-advice");
    assert!(missing.result.all_classes.contains_key("::C"));
    missing.result.resolved_input = None;
    missing.result.all_classes.clear();
    missing.source = source.to_owned().into();
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        missing.lexer_config(),
    )
    .remove(0);
    assert!(missing.handle_oo_define_command(
        command.name(),
        command.args(),
        command.arg_tokens(),
        command.arg_single_token(),
        &[],
    ));
    assert!(missing.result.all_classes.is_empty());
}
