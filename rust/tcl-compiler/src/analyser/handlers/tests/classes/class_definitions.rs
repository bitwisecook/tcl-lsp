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

//! Class creation, `oo::define` and `oo::objdefine`, and the join of two observations of one class.

use super::*;

fn class_observation(superclass: Option<&str>, factory: bool) -> ClassDef {
    ClassDef {
        name: "class".to_owned(),
        qualified_name: "::T::D::class".to_owned(),
        superclasses: superclass.into_iter().map(str::to_owned).collect(),
        factory: factory.then(|| ClassFactory {
            root_metaclass: "oo::class".to_owned(),
            ..ClassFactory::default()
        }),
        ..ClassDef::default()
    }
}

#[test]
fn parameterised_class_join_is_monotone_across_incomplete_observations() {
    let proved = class_observation(Some("::T::Mother"), true);
    let incomplete = class_observation(None, false);

    let later_incomplete = join_parameterised_class_observations(&proved, &incomplete).unwrap();
    assert_eq!(later_incomplete.superclasses, vec!["::T::Mother"]);
    assert!(later_incomplete.factory.is_some());

    let later_proof = join_parameterised_class_observations(&incomplete, &proved).unwrap();
    assert_eq!(later_proof.superclasses, vec!["::T::Mother"]);
    assert!(later_proof.factory.is_some());
}

#[test]
fn parameterised_class_join_abstains_on_conflicting_concrete_relations() {
    let left = class_observation(Some("::T::Mother"), true);
    let right = class_observation(Some("::T::OtherMother"), true);
    assert!(join_parameterised_class_observations(&left, &right).is_none());
}

#[test]
fn parameterised_class_join_ignores_an_unobserved_metaclass() {
    // The `oo::define` stub's `"oo::class"` is
    // `ClassDef::default()`'s stand-in, not something a walk read, so it
    // must neither contradict the proved `::T::Mother` nor survive into
    // the joined record.
    let stub = class_observation(None, false);
    assert_ne!(stub.metaclass_provenance, MetaclassProvenance::Observed);
    let proved = ClassDef {
        metaclass: "::T::Mother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(Some("::T::Mother"), true)
    };

    for (existing, observed) in [(&stub, &proved), (&proved, &stub)] {
        let joined = join_parameterised_class_observations(existing, observed)
            .expect("a stand-in metaclass is not conflicting evidence");
        assert_eq!(joined.metaclass, "::T::Mother");
        assert_eq!(joined.metaclass_provenance, MetaclassProvenance::Observed);
        assert_eq!(joined.superclasses, vec!["::T::Mother"]);
        assert!(joined.factory.is_some());
    }
}

#[test]
fn parameterised_class_join_abstains_on_two_observed_metaclasses() {
    // FP guard for the gate above: once *both* sides read a head word,
    // a disagreement is still a real conflict.
    let left = ClassDef {
        metaclass: "::T::Mother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(None, true)
    };
    let right = ClassDef {
        metaclass: "::T::OtherMother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(None, true)
    };
    assert!(join_parameterised_class_observations(&left, &right).is_none());
    // …and identical observed metaclasses still join.
    assert!(join_parameterised_class_observations(&left, &left).is_some());
}

#[test]
fn foreach_objdefine_records_per_object_facts_for_every_literal_element() {
    // TP — the sibling shape.  tclsh 9.0.4 and 8.6.16
    // both run `foreach o {::a ::b} { oo::objdefine $o { method probe
    // {} {…} } }` and report `probe` in `info object methods` for
    // *both* objects, so both must carry the per-object method fact.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    for obj in ["::a", "::b"] {
        let methods = r
            .object_methods
            .get(obj)
            .unwrap_or_else(|| panic!("{obj} has per-object methods: {:?}", r.object_methods));
        assert!(
            methods.iter().any(|m| m.def.name == "probe"),
            "{obj} gained probe: {methods:?}",
        );
    }
}

#[test]
fn foreach_objdefine_does_not_double_record_the_first_element() {
    // FP guard — the ordinary body walk covers the first element and
    // the simulation covers the rest, so the first element's site is
    // visited twice under the loop variable's own key.  One source site
    // can never declare the same member twice, so it must appear once.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    for (key, methods) in &r.object_methods {
        let probes = methods.iter().filter(|m| m.def.name == "probe").count();
        assert!(
            probes <= 1,
            "{key} recorded probe {probes} times: {methods:?}",
        );
    }
}

#[test]
fn foreach_objdefine_does_not_duplicate_body_diagnostics() {
    // FP guard — re-running the installer per element re-walks the
    // per-object method bodies, so a diagnostic raised inside one must
    // still be reported once per site, never once per iteration.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return $undefinedVar }\n    }\n}\n",
            "tcl9.0",
        );
    let mut seen = std::collections::HashSet::new();
    for diag in &r.diagnostics {
        assert!(
            seen.insert((diag.code.to_string(), diag.span)),
            "duplicate diagnostic {:?} at {:?}",
            diag.code,
            diag.span,
        );
    }
}

#[test]
fn foreach_objdefine_over_a_dynamic_list_abstains() {
    // TN — a runtime element list names no object the walk can know, so
    // the per-object facts stay under the loop variable's own key and
    // are never attributed to a literal object.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             set objects $argv\n\
             foreach o $objects {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    assert!(
        !r.object_methods.contains_key("::a"),
        "a runtime receiver must not be attributed to a literal object: {:?}",
        r.object_methods,
    );
    assert!(
        r.object_methods.contains_key("o"),
        "the facts stay under the receiver variable: {:?}",
        r.object_methods,
    );
}

// handle_oo_objdefine

/// `oo::objdefine` records its receiver word: a variable by its name,
/// braced or not, and a literal object by its spelling. tclsh 8.6.18 and
/// 9.0.4 print `1 2 3` for the three forms over one object.
#[test]
fn handle_oo_objdefine_records_its_receiver_word() {
    for receiver in ["$obj", "${obj}", "obj"] {
        let a = analysed(&format!(
            "oo::object create obj\nset obj obj\noo::objdefine {receiver} {{method m {{}} {{return 1}}}}\n"
        ));
        assert!(a.objdefined_vars.contains("obj"), "{receiver}");
    }
}

#[test]
fn oo_objdefine_body_methods_are_analysed() {
    // Before: the `oo::objdefine` body was never parsed, so nothing inside
    // a per-object method resolved.  Now the body walks like any method
    // body — the call it makes is recorded as an invocation.
    let mut a = Analyser::new();
    let src = "proc helper {} {}\n\
                   oo::class create Foo {}\n\
                   set o [Foo new]\n\
                   oo::objdefine $o {\n    \
                       method greet {} { helper }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.command_invocations.iter().any(|i| i.name == "helper"),
        "the call inside the per-object method body should be analysed: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>(),
    );
}

// handle_oo_class_command

#[test]
fn handle_oo_class_create_records_class() {
    let a = analysed("oo::class create MyClass\n");
    assert!(a.result.all_classes.contains_key("::MyClass"));
    let cls = &a.result.all_classes["::MyClass"];
    assert_eq!(cls.name, "MyClass");
}

#[test]
fn handle_oo_class_create_with_body() {
    let src = "oo::class create MyClass {method m {} {}}\n";
    let a = analysed(src);
    let open = u32::try_from(src.find('{').unwrap()).unwrap();
    assert_eq!(
        a.result.all_classes["::MyClass"].body_span,
        span(open, open + 15)
    );
}

#[test]
fn handle_oo_class_wrong_subcommand_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_class_command(
        "oo::class",
        &["destroy".to_string(), "MyClass".to_string()],
        &[
            esc_tok(span(0, 9)),
            esc_tok(span(10, 17)),
            esc_tok(span(18, 25)),
        ],
        &[],
        esc_tok(span(0, 9)),
    );
    assert!(!handled);
    assert_eq!(a.result.all_classes.len(), 0);
}

// handle_oo_class_command body walking

#[test]
fn analyse_oo_class_body_records_superclass_and_methods() {
    // End-to-end: ``oo::class create Sub`` with a body
    // declaring a superclass and a method.  After analyse
    // ``::Sub`` should carry both fields.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create Sub { superclass ::Base\nmethod greet {} { puts hi } }",
        "tcl",
    );
    assert!(r.all_classes.contains_key("::Sub"));
    let cls = &r.all_classes["::Sub"];
    assert_eq!(cls.superclasses, vec!["::Base"]);
    assert!(cls.methods.contains_key("greet"));
    assert_eq!(cls.methods["greet"].kind, "method");
}

#[test]
fn analyse_oo_class_body_records_classmethod_and_mixin() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { mixin ::M\nclassmethod build {} { return ok } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.mixins, vec!["::M"]);
    assert!(cls.class_methods.contains_key("build"));
    assert!(!cls.methods.contains_key("build"));
}

// handle_oo_define_command body walking

#[test]
fn analyse_oo_define_body_extends_existing_class() {
    // ``oo::class create C {}`` followed by ``oo::define
    // C { method m {} {} }`` — the method ends up in the
    // already-recorded class.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C {}\noo::define C { method m {} {} }",
        "tcl",
    );
    assert!(r.all_classes.contains_key("::C"));
    let cls = &r.all_classes["::C"];
    assert!(cls.methods.contains_key("m"));
}

#[test]
fn oo_define_extending_an_existing_class_adds_its_own_class_body_span() {
    // `class_body_spans` must record BOTH the
    // creation site's own span AND the separate `oo::define` block's
    // own span for the same qualified class — a class extended via a
    // *separate* `oo::define ClassName { ... }` block has textually
    // disjoint body spans, not one contiguous range, and `my`-dispatch
    // resolution (a lexical "which class's body am I inside" query)
    // needs every contributing span, not just the first one recorded.
    let mut a = crate::analyser::Analyser::new();
    let src = "oo::class create C {}\noo::define C { method m {} {} }";
    let r = a.analyse(src, "tcl");
    let spans: Vec<tcl_lexer::Span> = r
        .class_body_spans
        .iter()
        .filter(|(name, _)| name == "::C")
        .map(|(_, span)| *span)
        .collect();
    assert_eq!(spans.len(), 2, "{spans:?}");
    // One of the two recorded spans must cover the `oo::define`
    // block's own `method m {} {}` text — the token whose containment
    // `enclosing_class_at` checks for a cursor inside it.
    let inside_define_block = u32::try_from(src.find("method m").unwrap()).unwrap();
    assert!(
        spans
            .iter()
            .any(|span| span.start() <= inside_define_block && inside_define_block < span.end()),
        "no recorded span covers the oo::define block's own body: {spans:?}"
    );
}

#[test]
fn oo_define_extends_class_named_like_a_create_subcommand() {
    // `oo::define` / `oo::objdefine` carry a TclOO
    // `definition_body` but no `IS_OO_METACLASS` trait, so a class literally
    // named `create` must be *extended* by `handle_oo_define_command`, not
    // stolen by `handle_oo_class_command` (which would record a bogus class
    // named `method`).  The class-creator check requires the metaclass
    // trait, which define/objdefine lack.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::define create method greet {} { return hi }", "tcl");
    // The real class is `create`, with method `greet` — not a bogus class.
    let cls = r
        .all_classes
        .get("::create")
        .expect("class `create` recorded by oo::define");
    assert!(
        cls.methods.contains_key("greet"),
        "method greet must be recorded on class `create`: {:?}",
        cls.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_classes.contains_key("::method"),
        "no bogus class named `method` should be created",
    );
}

#[test]
fn analyse_oo_define_inline_form_extends_class() {
    // ``oo::define C method m {} {}`` — inline form,
    // single subcommand.  Works whether or not the class
    // was already declared (creates a stub if absent).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::define MyClass method greet {} { puts hi }", "tcl");
    assert!(r.all_classes.contains_key("::MyClass"));
    let cls = &r.all_classes["::MyClass"];
    assert!(cls.methods.contains_key("greet"));
}

#[test]
fn two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge() {
    // TP — clay.tcl's `current_class`-based Ensemble DSL shape: two lexically
    // unrelated procs each write `oo::define $class method ... ` with a
    // same-named local `class` (an unremarkable choice for this exact
    // idiom) and each add their own method. Keying such a call on the raw
    // text `$class` gives both calls the identical key, so the second
    // `remove`+`insert` round-trip silently merges the first proc's
    // method into the second's ClassDef (and vice versa via the
    // dual-registration into `scope.classes`), producing a document
    // symbol whose child method range sits outside its own parent's.
    // The targets here are *parameters*, so no constant dominates
    // either call and both stay genuinely dynamic — the shape this
    // guard is about.  (A target bound to a dominating constant is a
    // different, statically-resolvable case: see
    // `oo_define_resolves_a_dominating_constant_target`.)
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc addFoo {class} {\n    oo::define $class method foo {} { return foo }\n}\n\
             proc addBar {class} {\n    oo::define $class method bar {} { return bar }\n}\n",
        "tcl8.6",
    );
    // The procedures are never called, so these are written words of
    // bodies not entered: whatever the analysis records, no class holds
    // both methods.
    assert!(
        !r.all_classes
            .values()
            .any(|c| c.methods.contains_key("foo") && c.methods.contains_key("bar")),
        "neither call site's method may leak into the other's class: {:?}",
        r.all_classes
            .iter()
            .map(|(k, c)| (k, c.methods.keys().collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dynamic_oo_define_target_does_not_touch_a_same_named_literal_class() {
    // FP guard — a class whose *written* name matches nothing knowable
    // (`$class` bound to a parameter) must never be found/extended
    // through a literal class that happens to share the variable's
    // source text: the synthetic key can never collide with a real
    // qualified name.  A target bound to a dominating constant *is*
    // resolvable and does extend the real class — tclsh 9.0.4/8.6.16
    // agree — which is
    // `oo_define_resolves_a_dominating_constant_target`'s case, not
    // this one.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Widget {\n    method real {} {}\n}\n\
             proc addDynamic {class} {\n    oo::define $class method dynamic {} {}\n}\n",
        "tcl8.6",
    );
    let widget = &r.all_classes["::Widget"];
    assert!(widget.methods.contains_key("real"), "{widget:?}");
    assert!(
        !widget.methods.contains_key("dynamic"),
        "the dynamic call must not extend the literal same-named class: {widget:?}",
    );
}

// handle_oo_define_command

#[test]
fn handle_oo_define_recognises_canonical_form() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_define_command(
        "oo::define",
        &["MyClass".to_string(), "method m {} {}".to_string()],
        &[],
        &[],
        &[],
    );
    assert!(handled);
}

#[test]
fn handle_oo_define_no_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_define_command("oo::define", &[], &[], &[], &[]);
    assert!(!handled);
}

// ClassDef extended fields + UnknownProcInfo

#[test]
fn analyse_oo_class_records_metaclass_from_command_name() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::class create C {}", "tcl");
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.metaclass, "oo::class");
}

#[test]
fn analyse_oo_class_body_records_constructors_and_destructor() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { constructor args { puts ctor }\ndestructor { puts dtor } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.constructors.len(), 1);
    assert_eq!(cls.constructors[0].kind, "constructor");
    assert!(cls.destructor.is_some());
    assert_eq!(cls.destructor.as_ref().unwrap().kind, "destructor");
}

#[test]
fn analyse_oo_class_body_records_variables_filters_exports() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { variable x y\nfilter log\nexport foo bar\nunexport hidden }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.variables, vec!["x", "y"]);
    assert_eq!(cls.filters, vec!["log"]);
    assert!(cls.exports.contains("foo"));
    assert!(cls.exports.contains("bar"));
    assert!(cls.unexports.contains("hidden"));
}

#[test]
fn analyse_oo_class_body_records_property_def() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { property colour -kind readwrite -get { return red } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    let pd = cls.properties.get("colour").expect("colour recorded");
    assert_eq!(pd.kind, "readwrite");
    assert!(pd.has_getter);
    assert!(!pd.has_setter);
}
