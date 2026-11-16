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

#[test]
fn class_member_key_disambiguates_method_and_classmethod() {
    assert_eq!(
        class_member_key("::Bar", "get", false),
        "::Bar::method::get"
    );
    assert_eq!(
        class_member_key("::Bar", "get", true),
        "::Bar::classmethod::get"
    );
    // Same class, same member name, different map — must not collide.
    assert_ne!(
        class_member_key("::Bar", "get", false),
        class_member_key("::Bar", "get", true)
    );
}

#[test]
fn scope_default_is_global() {
    let s = Scope::default();
    assert_eq!(s.kind, ScopeKind::Global);
    assert_eq!(s.name, "::");
    assert_eq!(s.variables.len(), 0);
    assert_eq!(s.procs.len(), 0);
    assert_eq!(s.classes.len(), 0);
    assert_eq!(s.children, [] as [crate::analyser::types::Scope; 0]);
}

#[test]
fn analysis_result_default_is_empty() {
    let r = AnalysisResult::default();
    assert_eq!(r.global_scope.kind, ScopeKind::Global);
    assert_eq!(r.all_procs.len(), 0);
    assert_eq!(r.all_classes.len(), 0);
    assert_eq!(r.all_variables.len(), 0);
    assert_eq!(r.diagnostics, [] as [crate::analyser::types::Diagnostic; 0]);
    assert_eq!(
        r.command_invocations,
        [] as [crate::signature_scan::types::SignatureCommandInvocation; 0]
    );
    assert_eq!(
        r.package_requires,
        [] as [crate::signature_scan::types::SignaturePackageRequire; 0]
    );
    assert_eq!(
        r.source_targets,
        [] as [crate::signature_scan::types::SignatureSource; 0]
    );
    assert_eq!(r.command_aliases.len(), 0);
    assert_eq!(
        r.namespace_imports,
        [] as [crate::signature_scan::types::SignatureNamespaceImport; 0]
    );
    assert!(r.unknown_proc_info.is_none());
}

#[test]
fn class_def_default_has_oo_class_metaclass() {
    // ``metaclass`` is the only non-trivial default the rest of
    // the analyser depends on.
    let c = ClassDef::default();
    assert_eq!(c.metaclass, "oo::class");
    // …and it is a stand-in, not an observation.
    assert_eq!(c.metaclass_provenance, MetaclassProvenance::StandIn);
    assert_eq!(c.constructors, [] as [crate::analyser::types::MethodDef; 0]);
    assert!(c.destructor.is_none());
    assert_eq!(c.variables, [] as [std::string::String; 0]);
    assert_eq!(c.properties.len(), 0);
    assert_eq!(c.filters, [] as [std::string::String; 0]);
    assert_eq!(c.exports.len(), 0);
    assert_eq!(c.unexports.len(), 0);
    assert_eq!(c.doc, "");
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn string_set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn method(name: &str, params: &[&str]) -> MethodDef {
    MethodDef {
        name: name.to_owned(),
        params: params
            .iter()
            .map(|p| crate::signature_scan::types::ParamDef {
                name: (*p).to_owned(),
                has_default: false,
                default_value: None,
            })
            .collect(),
        params_computed: false,
        formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
            tcl_dialect::ParameterGrammar::Tcl,
        ),
        name_span: Span::new(0, 0),
        body_span: Span::new(0, 0),
        kind: "method".to_owned(),
        is_self_method: false,
        visibility: "public".to_owned(),
        doc: String::new(),
        forward_target: None,
    }
}

/// The pair this join sees: a computed-name creation and the
/// `oo::define` stub that extends it, the stub having run second.
fn creation_and_stub(creation: ClassDef, stub: ClassDef) -> (ClassDef, ClassDef) {
    (
        creation,
        ClassDef {
            via_define: true,
            ..stub
        },
    )
}

#[test]
fn absorb_declarations_lets_the_later_definition_set_visibility() {
    // `export` / `unexport`
    // are one bit per member, not two sets: on 8.6.16 and 9.0.4,
    // `oo::class create E1 { method m {} {}; export m }` then
    // `oo::define E1 { unexport m }` leaves `info class methods E1`
    // empty while `-private` still lists `m`, and `[E1 new] m` raises
    // `unknown method "m"`. The reverse order re-exports it. A union
    // would leave `m` in both sets, and workspace dispatch reads an
    // `exports` entry as decisive.
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            exports: string_set(&["m"]),
            class_exports: string_set(&["cm"]),
            ..ClassDef::default()
        },
        ClassDef {
            unexports: string_set(&["m"]),
            class_unexports: string_set(&["cm"]),
            ..ClassDef::default()
        },
    );
    creation.absorb_declarations(&stub);
    assert!(
        creation.exports.is_empty() && creation.unexports == string_set(&["m"]),
        "the later `unexport m` must clear the earlier `export m`; \
             exports={:?} unexports={:?}",
        creation.exports,
        creation.unexports
    );
    assert!(
        creation.class_exports.is_empty() && creation.class_unexports == string_set(&["cm"]),
        "the class-object side takes the identical update"
    );

    // Reverse: the creation unexports, the later define re-exports.
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            unexports: string_set(&["m"]),
            ..ClassDef::default()
        },
        ClassDef {
            exports: string_set(&["m"]),
            ..ClassDef::default()
        },
    );
    creation.absorb_declarations(&stub);
    assert!(
        creation.unexports.is_empty() && creation.exports == string_set(&["m"]),
        "the later `export m` must clear the earlier `unexport m`"
    );

    // …and with the stub as the *winner* of the join, the creation it
    // absorbs ran first, so the stub's own bit stands.
    let mut stub = ClassDef {
        via_define: true,
        unexports: string_set(&["m"]),
        ..ClassDef::default()
    };
    stub.absorb_declarations(&ClassDef {
        exports: string_set(&["m", "untouched"]),
        ..ClassDef::default()
    });
    assert!(
        stub.unexports == string_set(&["m"]) && stub.exports == string_set(&["untouched"]),
        "the stub ran second whichever side won the join; \
             exports={:?} unexports={:?}",
        stub.exports,
        stub.unexports
    );
}

#[test]
fn absorb_declarations_replaces_mixins_but_appends_variables_and_filters() {
    // The ordered slots are
    // *not* uniform, which is why each was measured rather than
    // assumed. On 8.6.16 and 9.0.4, a class created with
    // `mixin ::FH ::A; filter f; variable x` and then
    // `oo::define … { mixin ::FH ::B; filter g; variable y }` reports
    // mixins `::FH ::B` (replaced — `mixin -append` is the additive
    // form), filters `f g` and variables `x y` (both appended).
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            mixins: strings(&["::FH", "::A"]),
            filters: strings(&["f"]),
            class_filters: strings(&["cf"]),
            variables: strings(&["x"]),
            ..ClassDef::default()
        },
        ClassDef {
            mixins: strings(&["::FH", "::B"]),
            filters: strings(&["g"]),
            class_filters: strings(&["cg"]),
            variables: strings(&["y"]),
            ..ClassDef::default()
        },
    );
    creation.absorb_declarations(&stub);
    assert_eq!(creation.mixins, strings(&["::FH", "::B"]), "mixin replaces");
    assert_eq!(creation.filters, strings(&["f", "g"]), "filter appends");
    assert_eq!(creation.class_filters, strings(&["cf", "cg"]));
    assert_eq!(creation.variables, strings(&["x", "y"]), "variable appends");

    // Appends keep run order when the *stub* won the join, and never
    // duplicate a name the slot already carries (`variable x` twice
    // leaves one `x` on both interpreters).
    let mut stub = ClassDef {
        via_define: true,
        filters: strings(&["g"]),
        variables: strings(&["x", "y"]),
        ..ClassDef::default()
    };
    stub.absorb_declarations(&ClassDef {
        filters: strings(&["f"]),
        variables: strings(&["x"]),
        ..ClassDef::default()
    });
    assert_eq!(stub.filters, strings(&["f", "g"]), "earlier filter first");
    assert_eq!(
        stub.variables,
        strings(&["x", "y"]),
        "declared twice, kept once"
    );

    // A slot the later definition never sets keeps the earlier list.
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            mixins: strings(&["::A"]),
            ..ClassDef::default()
        },
        ClassDef::default(),
    );
    creation.absorb_declarations(&stub);
    assert_eq!(creation.mixins, strings(&["::A"]));
}

#[test]
fn absorb_declarations_applies_a_later_retraction_to_the_member_table() {
    // A stub records
    // `deletemethod m` as a tombstone because it has no table to remove
    // from; the join is where the ordering is known, and compiler-side
    // consumers of `all_classes` never run the workspace fold. On
    // 8.6.16 and 9.0.4, `oo::class create D1 { method m …; method keep … }`
    // then `oo::define D1 { deletemethod m }` reports `info class
    // methods D1` = `keep`, and `[D1 new] m` raises `unknown method`.
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            methods: [
                ("m".to_owned(), method("m", &[])),
                ("keep".to_owned(), method("keep", &[])),
            ]
            .into_iter()
            .collect(),
            ..ClassDef::default()
        },
        ClassDef {
            retracted_members: vec![MemberRetractionRecord::deletion(
                "m".to_owned(),
                MemberSide::Instance,
            )],
            ..ClassDef::default()
        },
    );
    creation.absorb_declarations(&stub);
    assert_eq!(
        creation
            .methods
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["keep"],
        "the retracted member must leave the joined table"
    );
    assert_eq!(
        creation.retracted_members.len(),
        1,
        "the tombstone still rides along for the cross-file consumer"
    );

    // A move re-homes the member under its arrival name, taking the
    // destination word as its declaration site — `renamemethod m n`
    // leaves `n` alone in the table on both interpreters.
    let (mut creation, stub) = creation_and_stub(
        ClassDef {
            methods: [("m".to_owned(), method("m", &["a"]))]
                .into_iter()
                .collect(),
            ..ClassDef::default()
        },
        ClassDef {
            retracted_members: vec![MemberRetractionRecord {
                member: "m".to_owned(),
                side: MemberSide::Instance,
                arrival: Some("n".to_owned()),
                arrival_span: Some(Span::new(40, 41)),
            }],
            ..ClassDef::default()
        },
    );
    creation.absorb_declarations(&stub);
    let moved = creation
        .methods
        .get("n")
        .expect("the moved member arrives under its new name");
    assert!(
        !creation.methods.contains_key("m"),
        "the source name is gone"
    );
    assert_eq!(moved.params.len(), 1, "the moved member keeps its body");
    assert_eq!(moved.name, "n");
    assert_eq!(moved.name_span, Span::new(40, 41));

    // The reverse order retracts nothing: a `deletemethod` the creation
    // body itself ran already emptied its own table, and the stub's
    // members were declared afterwards.
    let mut stub = ClassDef {
        via_define: true,
        methods: [("m".to_owned(), method("m", &[]))].into_iter().collect(),
        ..ClassDef::default()
    };
    stub.absorb_declarations(&ClassDef {
        retracted_members: vec![MemberRetractionRecord::deletion(
            "m".to_owned(),
            MemberSide::Instance,
        )],
        ..ClassDef::default()
    });
    assert!(
        stub.methods.contains_key("m"),
        "an earlier retraction must not remove a later declaration"
    );
}

#[test]
fn absorb_declarations_unions_the_remaining_tables() {
    // The rules with no ordering component: keyed tables union with the
    // later record winning a collision, whole-body singletons follow
    // the replace rule, and `doc` fills only where absent.
    let mut into = ClassDef {
        linked_members: [("shared".to_owned(), "::mine".to_owned())]
            .into_iter()
            .collect(),
        doc: "mine".to_owned(),
        ..ClassDef::default()
    };
    let other = ClassDef {
        linked_members: [
            ("shared".to_owned(), "::theirs".to_owned()),
            ("only-theirs".to_owned(), "::theirs".to_owned()),
        ]
        .into_iter()
        .collect(),
        doc: "theirs".to_owned(),
        ..ClassDef::default()
    };
    into.absorb_declarations(&other);
    // Neither side is a stub, so this record keeps the colliding key.
    assert_eq!(
        into.linked_members.get("shared").map(String::as_str),
        Some("::mine")
    );
    assert_eq!(
        into.linked_members.get("only-theirs").map(String::as_str),
        Some("::theirs")
    );
    assert_eq!(into.doc, "mine");

    // …and with `other` the stub, the colliding key goes to it.
    let mut into = ClassDef {
        linked_members: [("shared".to_owned(), "::mine".to_owned())]
            .into_iter()
            .collect(),
        ..ClassDef::default()
    };
    into.absorb_declarations(&ClassDef {
        via_define: true,
        ..other
    });
    assert_eq!(
        into.linked_members.get("shared").map(String::as_str),
        Some("::theirs")
    );
    // A record with no doc of its own takes the other's.
    assert_eq!(into.doc, "theirs");
}
