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

//! `TclOO` classes: definitions, `oo::link`, and helper commands under their scoping and namespace-path rules.

// TclOO class definition extraction.
mod tcloo_classes {
    use super::super::*;
    use tcl_compiler::analyser::ClassDef;

    fn class(src: &str, qn: &str) -> ClassDef {
        class_in(src, qn, D)
    }

    /// [`class`] under an explicit dialect — for the members `TclOO` only
    /// gained in 9.0 (`classmethod`, `private`, `initialise`/`initialize`,
    /// `definitionnamespace`), which the 8.6 grammar rejects.
    fn class_in(src: &str, qn: &str, dialect: &str) -> ClassDef {
        Analyser::new()
            .analyse(src, dialect)
            .all_classes
            .get(qn)
            .cloned()
            .expect("class recorded")
    }

    #[test]
    fn class_create_basic() {
        let cd = class(
            "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\n",
            "::Dog",
        );
        assert_eq!(cd.name, "Dog");
        assert_eq!(cd.metaclass, "oo::class");
        assert!(cd.methods.contains_key("bark"));
        assert_eq!(cd.methods["bark"].kind, "method");
    }

    #[test]
    fn class_superclass() {
        let src = "oo::class create Animal {\n    method speak {} { error \"abstract\" }\n}\noo::class create Dog {\n    superclass Animal\n    method bark {} { return \"woof\" }\n}\n";
        assert_eq!(class(src, "::Dog").superclasses, ["Animal"]);
    }

    /// Integration (full analyse → resolved hierarchy): a bare
    /// `superclass Base` in a deeply-nested `::a::b::Sub`, where `Base` exists
    /// only in an *ancestor* namespace (`::a::Base`) and the tail is ambiguous
    /// (a second `::x::Base`), must NOT resolve — real Tcl errors there.  The
    /// former ancestor walk manufactured a `Sub ⊂ ::a::Base` edge.
    #[test]
    fn class_superclass_no_ancestor_namespace_walk() {
        let src = "namespace eval ::a {\n    oo::class create Base {}\n}\nnamespace eval ::x {\n    oo::class create Base {}\n}\nnamespace eval ::a::b {\n    oo::class create Sub {\n        superclass Base\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, D).clone();
        let h = analysis.class_hierarchy();
        assert!(
            !h.is_subtype("::a::b::Sub", "::a::Base"),
            "must not manufacture an ancestor-namespace inheritance edge"
        );
    }

    /// TP: a base in the class's own namespace resolves, so
    /// the inheritance edge is real.
    #[test]
    fn class_superclass_same_namespace_resolves() {
        let src = "namespace eval ::a {\n    oo::class create Base {}\n    oo::class create Sub {\n        superclass Base\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, D).clone();
        assert!(
            analysis
                .class_hierarchy()
                .is_subtype("::a::Sub", "::a::Base"),
            "a same-namespace superclass must resolve"
        );
    }

    #[test]
    fn class_variables_and_constructor() {
        let src = "oo::class create Dog {\n    variable name breed\n    constructor {n b} { set name $n; set breed $b }\n}\n";
        let cd = class(src, "::Dog");
        assert_eq!(cd.variables, ["name", "breed"]);
        assert_eq!(cd.constructors.len(), 1);
        assert_eq!(cd.constructors[0].kind, "constructor");
        assert_eq!(
            cd.constructors[0]
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["n", "b"]
        );
    }

    #[test]
    fn class_method_params() {
        let src = "oo::class create Calc {\n    method add {a b} { expr {$a + $b} }\n}\n";
        let cd = class(src, "::Calc");
        assert_eq!(
            cd.methods["add"]
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn class_destructor() {
        let cd = class(
            "oo::class create Conn {\n    destructor { close $fd }\n}\n",
            "::Conn",
        );
        let d = cd.destructor.as_ref().expect("destructor");
        assert_eq!(d.kind, "destructor");
    }

    #[test]
    fn class_mixins() {
        let cd = class(
            "oo::class create Dog {\n    mixin Serializable Comparable\n}\n",
            "::Dog",
        );
        assert_eq!(cd.mixins, ["Serializable", "Comparable"]);
    }

    #[test]
    fn class_forward() {
        let cd = class(
            "oo::class create Dog {\n    forward run ::dog::run_impl\n}\n",
            "::Dog",
        );
        assert_eq!(cd.methods["run"].kind, "forward");
    }

    #[test]
    fn class_filter_export_unexport() {
        let src = "oo::class create Dog {\n    method bark {} {}\n    method internal {} {}\n    filter myfilter\n    export bark\n    unexport internal\n}\n";
        let cd = class(src, "::Dog");
        assert_eq!(cd.filters, ["myfilter"]);
        assert!(cd.exports.contains("bark"));
        assert!(cd.unexports.contains("internal"));
    }

    #[test]
    fn oo_define_body_merges_methods() {
        // `classmethod` is 9.0-only, so this merge case runs under tcl9.0.
        let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\noo::define Dog {\n    method fetch {item} { return $item }\n    classmethod count {} { return 0 }\n}\n";
        let cd = class_in(src, "::Dog", "tcl9.0");
        assert!(cd.methods.contains_key("bark"));
        assert!(cd.methods.contains_key("fetch"));
        assert!(cd.class_methods.contains_key("count"));
    }

    #[test]
    fn oo_define_inline_and_partial_class() {
        let cd = class(
            "oo::define Dog method sit {} { return \"sitting\" }",
            "::Dog",
        );
        assert!(cd.methods.contains_key("sit"));
        // oo::define on an unseen class creates a partial ClassDef.
        let cd2 = class(
            "oo::define MyClass {\n    method foo {} {}\n}\n",
            "::MyClass",
        );
        assert!(cd2.methods.contains_key("foo"));
    }

    #[test]
    fn metaclass_variants() {
        let cfg = class(
            "oo::configurable create Point {\n    property x y\n}\n",
            "::Point",
        );
        assert_eq!(cfg.metaclass, "oo::configurable");
        assert!(cfg.properties.contains_key("x") && cfg.properties.contains_key("y"));
        assert_eq!(
            class(
                "oo::abstract create Shape {\n    method area {} { error \"abstract\" }\n}\n",
                "::Shape"
            )
            .metaclass,
            "oo::abstract"
        );
        assert_eq!(
            class(
                "oo::singleton create Logger {\n    method log {msg} { puts $msg }\n}\n",
                "::Logger"
            )
            .metaclass,
            "oo::singleton"
        );
    }

    #[test]
    fn fully_qualified_oo_class_head_normalised() {
        let cd = class(
            "::oo::class create ::Dog {\n    method bark {} { return woof }\n}\n",
            "::Dog",
        );
        assert_eq!(cd.metaclass, "oo::class");
    }

    #[test]
    fn method_body_instance_variables_in_scope() {
        let src = "oo::class create Dog {\n    variable name\n    constructor {n} { set name $n }\n    method bark {} { return $name }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let bark = r
            .global_scope
            .children
            .iter()
            .find(|s| s.name.ends_with("Dog::bark"))
            .expect("bark scope");
        assert!(bark.variables.contains_key("name"));
    }

    #[test]
    fn classmethod_and_private_method() {
        // Both members are 9.0-only (TIP 478); under 8.6 they draw W002
        // instead, which `oo_90_only_members_*` below pins.
        let cd = class_in(
            "oo::class create Counter {\n    classmethod instances {} { return 0 }\n}\n",
            "::Counter",
            "tcl9.0",
        );
        assert_eq!(cd.class_methods["instances"].kind, "classmethod");
        let cd2 = class_in(
            "oo::class create Foo {\n    private method helper {} { return 1 }\n}\n",
            "::Foo",
            "tcl9.0",
        );
        assert_eq!(cd2.methods["helper"].visibility, "private");
    }

    #[test]
    fn class_in_namespace_and_registered_on_scope() {
        let src = "namespace eval shapes {\n    oo::class create Circle {\n        variable radius\n        method area {} { expr {3.14 * $radius * $radius} }\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.all_classes.contains_key("::shapes::Circle"));
        let cd = &r.all_classes["::shapes::Circle"];
        assert_eq!(cd.variables, ["radius"]);
        assert!(cd.methods.contains_key("area"));
        // ClassDef registered on the enclosing scope by simple name.
        let r2 = Analyser::new().analyse("oo::class create Dog {\n    method bark {} {}\n}\n", D);
        assert!(r2.global_scope.classes.contains_key("Dog"));
    }

    #[test]
    fn self_method_body_is_walked_for_internal_diagnostics() {
        // TP: a wrong-arity call inside a `self method`/`private method`
        // body must draw a diagnostic — the body itself must be walked,
        // not just the literal keywords
        // "method"/"classmethod"/"constructor"/"destructor", with
        // "self"/"private" falling through untouched. `string length`
        // takes exactly one argument.
        let src = "oo::class create Widget {\n    self method make {n} {\n        string length a b c d\n        return \"made $n\"\n    }\n}\n";
        assert_eq!(
            count(src, D, "E003"),
            1,
            "the wrong-arity call inside a self-method body must now be walked: {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn private_method_body_is_also_walked_for_internal_diagnostics() {
        // TP — the sibling gap `private method` shared with `self method`
        // (both go through the same `unwrap_wrapper_member`-based
        // `collect_method_body`), confirming the fix isn't scoped to
        // `self` alone.
        // `private` is 9.0-only, so the body walk is exercised under tcl9.0.
        let src = "oo::class create Widget {\n    private method helper {} {\n        string length a b c d\n    }\n}\n";
        assert_eq!(
            count(src, "tcl9.0", "E003"),
            1,
            "the wrong-arity call inside a private-method body must now be walked: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// The `TclOO` definition members Tcl added in 9.0 — `classmethod`
    /// (TIP 478), `private`, `initialise`/`initialize`, and
    /// `definitionnamespace` (TIP 524) — do not exist in the 8.6 grammar.
    ///
    /// Oracle: each of the three call shapes below fails on real tclsh8.6
    /// with `invalid command name "<member>"` and succeeds on tclsh9.0
    /// (probes cm2 / cm3 / cm4). The registry's hover text already said
    /// "Tcl 9.0 also added the classmethod, private, initialise/initialize,
    /// and definitionnamespace subcommands (none exist under Tcl 8.6)"
    /// while the analyser accepted them silently.
    #[test]
    fn oo_90_only_members_are_flagged_under_86() {
        // Shape 1 — a member inside the metaclass's own `create` body.
        for member in [
            "classmethod count {} { return 0 }",
            "private { method secret {} { return s } }",
            "initialise { set n 1 }",
            "initialize { set n 1 }",
            "definitionnamespace ::mydefs",
        ] {
            let src =
                format!("oo::class create Dog {{\n    method bark {{}} {{}}\n    {member}\n}}\n");
            assert!(
                fires(&src, D, "W002"),
                "[create body] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&src, D)
            );
            // Shape 2 — the same member inside an `oo::define Cls { … }` block.
            let block = format!(
                "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {{\n    {member}\n}}\n"
            );
            assert!(
                fires(&block, D, "W002"),
                "[oo::define block] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&block, D)
            );
            // Shape 3 — the single-command `oo::define Cls <member> …` form.
            let single = format!(
                "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {member}\n"
            );
            assert!(
                fires(&single, D, "W002"),
                "[oo::define single] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&single, D)
            );
        }
    }

    /// TN — the same three shapes are clean under tcl9.0, where the members
    /// exist.
    #[test]
    fn oo_90_only_members_are_clean_under_90() {
        for member in [
            "classmethod count {} { return 0 }",
            "private { method secret {} { return s } }",
            "initialise { set n 1 }",
            "definitionnamespace ::mydefs",
        ] {
            for src in [
                format!("oo::class create Dog {{\n    method bark {{}} {{}}\n    {member}\n}}\n"),
                format!(
                    "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {{\n    {member}\n}}\n"
                ),
                format!(
                    "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {member}\n"
                ),
            ] {
                assert!(
                    !fires(&src, "tcl9.0", "W002"),
                    "{member:?} exists in 9.0 and must be clean; got {:?}",
                    codes(&src, "tcl9.0")
                );
            }
        }
    }

    /// Reporting a 9.0-only member must not *erase* it: the analyser still
    /// records the member under 8.6, so go-to-definition, references,
    /// rename, document symbols, and code lenses keep working over the code
    /// the user actually wrote. Same contract as the whole-command W002,
    /// which reports a dialect-unavailable command while the analyser goes
    /// on modelling the call.
    #[test]
    fn a_gated_oo_member_is_reported_but_still_recorded() {
        let cd = class_in(
            "oo::class create Counter {\n    classmethod instances {} { return 0 }\n}\n",
            "::Counter",
            D,
        );
        assert!(
            cd.class_methods.contains_key("instances"),
            "the classmethod must still be recorded under tcl8.6; got {:?}",
            cd.class_methods.keys().collect::<Vec<_>>()
        );
        let cd2 = class_in(
            "oo::class create Dog { }\noo::define Dog classmethod count {} { return 0 }\n",
            "::Dog",
            D,
        );
        assert!(
            cd2.class_methods.contains_key("count"),
            "the single-command form's member must still be recorded; got {:?}",
            cd2.class_methods.keys().collect::<Vec<_>>()
        );
    }

    /// TN — the members that have existed since `TclOO` shipped in 8.6 are
    /// never gated, in any of the three shapes.
    #[test]
    fn always_available_oo_members_are_clean_under_86() {
        for member in [
            "method bark {} { return \"woof\" }",
            "constructor {n} { set n $n }",
            "destructor { return }",
            "variable name",
            "forward run puts",
        ] {
            for src in [
                format!("oo::class create Dog {{\n    {member}\n}}\n"),
                format!("oo::class create Dog {{ }}\noo::define Dog {{\n    {member}\n}}\n"),
                format!("oo::class create Dog {{ }}\noo::define Dog {member}\n"),
            ] {
                assert!(
                    !fires(&src, D, "W002"),
                    "{member:?} is 8.6 `TclOO` and must be clean; got {:?}",
                    codes(&src, D)
                );
            }
        }
    }

    #[test]
    fn self_method_declaration_site_recorded_as_a_classmethod() {
        // TP — proves Part 1 alone (record shape + span), independent of
        // the definition.rs-side call-site receiver-resolution bug Part 2
        // fixes.
        let cd = class(
            "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\n",
            "::Widget",
        );
        assert!(!cd.methods.contains_key("make"));
        let md = cd
            .class_methods
            .get("make")
            .expect("recorded as a classmethod");
        assert_eq!(md.kind, "classmethod");
        assert!(md.is_self_method);
    }
}

// oo::Helpers::link — ClassDef::linked_members population.
// Consumer-side (definition/hover resolution) is covered in
// tcl-lsp-core; this module is about `collect_oo_links` itself.
mod oo_link {
    use super::super::*;
    use tcl_compiler::analyser::ClassDef;

    fn class(src: &str, qn: &str) -> ClassDef {
        Analyser::new()
            .analyse(src, D)
            .all_classes
            .get(qn)
            .cloned()
            .expect("class recorded")
    }

    #[test]
    fn single_name_link_records_alias_to_itself() {
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
    }

    #[test]
    fn two_element_link_records_alias_to_a_different_target() {
        let cd = class(
            "oo::class create C {\n    constructor {} { link {shortcut realMethod} }\n    method realMethod {x} { return $x }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("shortcut").map(String::as_str),
            Some("realMethod")
        );
    }

    #[test]
    fn link_called_from_destructor_is_recorded() {
        // TP — `collect_method_body` covers constructor/method/destructor
        // uniformly, and the link scan must too.
        let cd = class(
            "oo::class create C {\n    method foo {x} {return $x}\n    destructor { link foo }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
    }

    #[test]
    fn multiple_single_name_aliases_in_one_link_call_all_record() {
        // `link foo bar` is TWO independent one-element arguments, not a
        // single two-element pair — both must be recorded.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo bar }\n    method foo {x} {return $x}\n    method bar {y} {return $y}\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
        assert_eq!(
            cd.linked_members.get("bar").map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn links_from_different_method_bodies_all_accumulate() {
        // Links recorded from different member bodies (not just the
        // constructor) must all accumulate onto the same ClassDef.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method setup {} { link bar }\n    method foo {x} {return $x}\n    method bar {y} {return $y}\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
        assert_eq!(
            cd.linked_members.get("bar").map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn dynamic_link_target_is_not_recorded() {
        // TN — a dynamic `link $which` is skipped (mirrors
        // `detect_interp_alias`'s literal-only requirement): no
        // fabricated alias, conservative fallback to "not reachable".
        let cd = class(
            "oo::class create C {\n    variable which\n    constructor {w} { set which $w; link $which }\n    method foo {x} {return $x}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.is_empty(), "{:?}", cd.linked_members);
    }

    #[test]
    fn link_for_one_name_does_not_blanket_legitimize_others() {
        // Precision guard — a link for a DIFFERENT name must not
        // blanket-legitimise every bareword in the class.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} {return $x}\n    method other {y} {return $y}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.contains_key("foo"));
        assert!(!cd.linked_members.contains_key("other"));
    }

    #[test]
    fn link_nested_inside_a_conditional_is_not_recorded() {
        // Deliberately shallow — only a top-level `link` call is
        // recognised, not one nested inside an `if`/`catch`/… body
        // argument (matches `scan_my_method_region`'s own accepted
        // scope boundary for this class of problem).
        let cd = class(
            "oo::class create C {\n    constructor {} { if {1} { link foo } }\n    method foo {x} {return $x}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.is_empty(), "{:?}", cd.linked_members);
    }

    #[test]
    fn link_is_not_recorded_under_a_dialect_without_it() {
        // The head is recognised through the registry's
        // `TCLOO_BINDS_METHOD_ALIAS` trait, not by its spelling, so an 8.5
        // dialect — which has no TclOO at all, let alone `oo::Helpers::link`
        // — records nothing. (tclsh 8.6.14 without `ooutil` likewise:
        // `invalid command name "link"` even inside a method.)
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
            "::C",
        );
        assert!(cd.linked_members.contains_key("foo"));
        let cd85 = Analyser::new()
            .analyse(
                "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
                "tcl8.5",
            )
            .all_classes
            .get("::C")
            .cloned();
        assert!(
            cd85.is_none_or(|c| c.linked_members.is_empty()),
            "no `link` in the 8.5 registry, so no aliases",
        );
    }
}

// The `oo::Helpers` family is method-context-scoped.
//
// tclsh 9.0.4, at the top level:
//     link foo          -> invalid command name "link"
//     my foo            -> invalid command name "my"
//     next / nextto     -> invalid command name "next" / "nextto"
//     self              -> invalid command name "self"
//     classvariable v   -> invalid command name "classvariable"
//     info commands ::link  -> {}   (empty)
// and inside `oo::class create C { method m {} { … } }`:
//     namespace current -> ::oo::Obj22       namespace path -> ::oo::Helpers
//     namespace which -command link -> ::oo::Helpers::link
//     namespace which -command my   -> ::oo::Obj22::my   (NOT a helper)
// while an `apply` lambda written inside that same method body loses the
// context entirely (`invalid command name "link"` / `"my"` / `"self"`).
//
// tclsh 8.6.14 agrees for the four members it has (`next`/`nextto`/`self`
// are `::oo::Helpers::*`, `my` is `::oo::ObjN::my`), and adding
// `::oo::Helpers::link` the way Tcllib's `ooutil` does still leaves the
// top-level bare `link` an `invalid command name`.
mod oo_helpers_scoping {
    use super::super::*;

    /// Every family member, with the dialect that has it.
    const FAMILY: &[(&str, &str)] = &[
        ("link", "tcl9.0"),
        ("my", "tcl8.6"),
        ("next", "tcl8.6"),
        ("nextto", "tcl8.6"),
        ("self", "tcl8.6"),
        ("classvariable", "tcl9.0"),
    ];

    #[test]
    fn top_level_use_draws_w123() {
        for (word, dialect) in FAMILY {
            let src = format!("{word} foo\n");
            assert!(
                fires(&src, dialect, "W123"),
                "top-level `{word}` is `invalid command name` in real Tcl",
            );
        }
    }

    #[test]
    fn method_body_use_is_clean() {
        for (word, dialect) in FAMILY {
            let src = format!(
                "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    method m {{}} {{ {word} foo }}\n}}\n"
            );
            assert!(
                !fires(&src, dialect, "W123"),
                "`{word}` resolves inside a method body: {:?}",
                codes(&src, dialect),
            );
        }
    }

    #[test]
    fn constructor_destructor_and_class_side_bodies_are_clean() {
        // tclsh 9.0.4: `link`/`self`/`my` all work in a constructor, a
        // destructor, a `self method` body, and an `oo::objdefine method`
        // body.
        for (word, dialect) in FAMILY {
            for body in [
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    constructor {{}} {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    destructor {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    self method cm {{}} {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{}}\nC create c1\noo::objdefine c1 {{\n    method om {{}} {{ {word} foo }}\n}}\n"
                ),
            ] {
                assert!(
                    !fires(&body, dialect, "W123"),
                    "`{word}` resolves in every method context: {:?}\n{body}",
                    codes(&body, dialect),
                );
            }
        }
    }

    #[test]
    fn apply_lambda_inside_a_method_body_draws_w123() {
        // TP — `apply` runs its body in the global namespace, so the object
        // context is gone (tclsh 9.0.4, inside a method:
        // `apply {{} { link Helper }}` -> `invalid command name "link"`).
        for (word, dialect) in FAMILY {
            let src = format!(
                "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    method m {{}} {{ apply {{{{}} {{ {word} foo }}}} }}\n}}\n"
            );
            assert!(
                fires(&src, dialect, "W123"),
                "an apply lambda loses the method context: {:?}",
                codes(&src, dialect),
            );
        }
    }

    #[test]
    fn a_linked_bareword_resolves_inside_the_objects_method_bodies() {
        // TN — `link foo` installs a real command `foo` in the object's own
        // namespace that dispatches `my foo` (tclsh 9.0.4: after `link
        // Helper` in a constructor, `namespace which -command Helper`
        // answers `::oo::ObjN::Helper` from every method body), so a later
        // bare `foo 1` is not an unknown command.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn an_unlinked_sibling_method_bareword_still_draws_w123() {
        // TP — `link` for one name must not blanket-legitimise every
        // bareword in the class: tclsh 9.0.4 raises `invalid command name
        // "other"` for an un-linked sibling method called bare.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method other {} { return 2 }\n    method bar {} {\n        link foo\n        return [other]\n    }\n}\n";
        assert!(fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn a_linked_bareword_still_draws_w123_at_the_top_level() {
        // TP — the alias lives in the object's namespace, so it is not
        // callable from outside a method body.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { link foo }\n}\nfoo 1\n";
        assert!(fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn proc_body_draws_w123() {
        // TP — an ordinary proc is not a method context, however it is
        // nested.
        for (word, dialect) in FAMILY {
            let src = format!("proc helper {{}} {{ {word} foo }}\n");
            assert!(fires(&src, dialect, "W123"), "`{word}` in a plain proc");
        }
    }

    #[test]
    fn qualified_spelling_resolves_anywhere() {
        // TN — `::oo::Helpers::link` is a genuine global command
        // (`info commands ::oo::Helpers::link` answers it under tclsh
        // 9.0.4); calling it outside a method is a *runtime* error, not an
        // unknown command.
        for spelling in [
            "::oo::Helpers::link foo",
            "oo::Helpers::link foo",
            "::oo::Helpers::self",
            "::oo::Helpers::classvariable v",
        ] {
            let src = format!("{spelling}\n");
            assert!(
                !fires(&src, "tcl9.0", "W123"),
                "{spelling}: {:?}",
                codes(&src, "tcl9.0"),
            );
        }
    }

    #[test]
    fn a_user_proc_of_the_same_name_still_resolves_at_the_top_level() {
        // FP guard — the scope rule must not swallow a real, user-defined
        // global command that happens to share the name.
        let src = "proc link {args} { return $args }\nlink foo\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    /// A class-level `initialise` / `initialize` body, both spellings.
    fn init_body(word: &str, spelling: &str) -> String {
        format!(
            "oo::class create C {{\n    {spelling} {{\n        {word} foo\n    }}\n    method m {{}} {{ return 1 }}\n}}\n"
        )
    }

    /// **W123 keys on resolution, and the family genuinely resolves in a
    /// class `initialise` body** — so it must stay silent there.
    ///
    /// tclsh 9.0.4, inside `oo::class create ::P { initialize { … } }`:
    /// `namespace current` is `::oo::Obj20`, `namespace path` is
    /// `::oo::Helpers ::oo`, and `namespace which -command link` answers
    /// `::oo::Helpers::link` — exactly as it does in a method body. Warning
    /// "unknown command" here would be a plain false positive.
    #[test]
    fn init_body_use_draws_no_w123() {
        for (word, _) in FAMILY {
            for spelling in ["initialize", "initialise"] {
                let src = init_body(word, spelling);
                assert!(
                    !fires(&src, "tcl9.0", "W123"),
                    "`{word}` resolves in an `{spelling}` body: {:?}",
                    codes(&src, "tcl9.0"),
                );
            }
        }
    }

    /// ...but **callability** is a different fact, and the two diverge in
    /// exactly this body.
    ///
    /// Same interpreter, same frame: every `::oo::Helpers` member raises
    /// `… may only be called from inside a method` when actually called,
    /// while `my` runs — it is `::oo::Obj20::my`, the *class object's* own
    /// dispatch command, and `my new` there really does make an instance.
    /// So the scope tree must mark the init frame as "resolves, but not a
    /// method frame".
    #[test]
    fn init_body_is_not_a_method_frame_but_still_reaches_oo_helpers() {
        use tcl_compiler::analyser::{
            innermost_scope_is_oo_method_frame, innermost_scope_reaches_oo_helpers,
        };
        let src = init_body("link", "initialize");
        let mut a = Analyser::new();
        let result = a.analyse(&src, "tcl9.0").clone();
        let init_off = u32::try_from(src.find("link foo").expect("the init-body call")).unwrap();
        let method_off = u32::try_from(src.find("return 1").expect("the method body")).unwrap();
        assert!(
            innermost_scope_reaches_oo_helpers(&result.global_scope, init_off),
            "an `initialize` body has ::oo::Helpers on its namespace path",
        );
        assert!(
            !innermost_scope_is_oo_method_frame(&result.global_scope, init_off),
            "an `initialize` body is not a method invocation",
        );
        // A real method body answers `true` to both, and the top level to
        // neither — the two predicates must not have collapsed into one.
        assert!(innermost_scope_reaches_oo_helpers(
            &result.global_scope,
            method_off
        ));
        assert!(innermost_scope_is_oo_method_frame(
            &result.global_scope,
            method_off
        ));
        assert!(!innermost_scope_reaches_oo_helpers(&result.global_scope, 0));
        assert!(!innermost_scope_is_oo_method_frame(&result.global_scope, 0));
    }

    /// The per-class `initialise` scoping must
    /// survive being marked "not a method frame": two sibling classes'
    /// same-named class variables stay independent.
    #[test]
    fn init_body_scoping_still_separates_sibling_classes() {
        let src = "oo::class create A {\n    initialize { variable Colours {red} }\n}\noo::class create B {\n    initialize { variable Colours {blue} }\n}\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }
}

// A TclOO member body's implicit `namespace path`.
//
// tclsh 8.6.16 and 9.0.4, inside `oo::class create C { method m {} { … } }`:
//     namespace current -> ::oo::ObjN        namespace path -> ::oo::Helpers
// so a bare word is looked up in the object namespace, then `::oo::Helpers`,
// then global. The documented "TclOO Tricks" idiom installs helpers directly
// (`proc ::oo::Helpers::callback {…} {…}`) and calls them bare from method
// bodies — which is what nico-robert/ticklecharts does, with the helper in
// `utils.tcl` and the calls in `esnap.tcl`.
//
// The candidate list is where that path belongs: `resolve_workspace_symbols`
// (the one cross-file resolver) walks it, so recording it once here fixes
// definition, references, rename, call hierarchy, and W123 together, across
// files, instead of each consumer growing its own lenient fallback.
mod oo_helpers_namespace_path {
    use super::super::Analyser;

    fn candidates_for(src: &str, dialect: &str, name: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|inv| inv.name == name)
            .map_or_default(|inv| inv.resolution_candidates.clone())
    }

    fn resolution_of(src: &str, dialect: &str, name: &str) -> Option<String> {
        Analyser::new()
            .analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|inv| inv.name == name)
            .and_then(|inv| inv.resolved_qualified_name.clone())
    }

    #[test]
    fn tp_a_method_body_call_carries_the_helpers_candidate_first() {
        // TP — `::oo::Helpers::callback` must be a candidate, and must rank
        // ahead of the global one: with both defined, real Tcl reaches the
        // helper.
        let src = "oo::class create C {\n    method Go {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            candidates_for(src, "tcl9.0", "callback"),
            ["::oo::Helpers::callback", "::callback"],
        );
    }

    #[test]
    fn tp_the_same_holds_for_a_constructor_body() {
        let src = "oo::class create C {\n    constructor {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            candidates_for(src, "tcl9.0", "callback"),
            ["::oo::Helpers::callback", "::callback"],
        );
    }

    #[test]
    fn tp_a_helper_declared_in_the_same_file_settles_the_call_onto_it() {
        // TP — with the helper present the call settles on it, not on a
        // phantom `::callback`.
        let src = "proc ::oo::Helpers::callback {m args} { return $m }\noo::class create C {\n    method Go {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            resolution_of(src, "tcl9.0", "callback").as_deref(),
            Some("::oo::Helpers::callback"),
        );
    }

    #[test]
    fn fp_a_top_level_call_gets_no_helpers_candidate() {
        // FP guard — `::oo::Helpers` is on a *method body's* namespace path
        // only. tclsh raises `invalid command name` for a bare `callback`
        // at the top level, so the candidate must not appear there.
        let src = "proc ::oo::Helpers::callback {m args} { return $m }\ncallback Read\n";
        assert_eq!(candidates_for(src, "tcl9.0", "callback"), ["::callback"]);
    }

    #[test]
    fn fp_an_apply_lambda_inside_a_method_loses_the_path() {
        // FP guard — `apply` runs its body in the global namespace, so the
        // object context (and its path) is gone. tclsh 9.0.4 raises
        // `invalid command name "link"` for exactly this shape.
        let src = "oo::class create C {\n    method Go {} {\n        apply {{} { callback Read }}\n    }\n}\n";
        assert_eq!(candidates_for(src, "tcl9.0", "callback"), ["::callback"]);
    }

    #[test]
    fn tn_a_snit_method_body_gets_no_helpers_candidate() {
        // TN — snit member bodies run in the type namespace with no
        // injected path; only the TclOO grammar declares one.
        let src = "snit::type T {\n    method Go {} {\n        callback Read\n    }\n}\n";
        let cands = candidates_for(src, "tcl9.0", "callback");
        assert!(
            !cands.iter().any(|c| c.starts_with("::oo::Helpers")),
            "snit gets no TclOO helper path: {cands:?}",
        );
    }

    #[test]
    fn tn_an_ordinary_builtin_still_settles_globally_from_a_method_body() {
        // TN — the extra candidate must not divert an ordinary call: `puts`
        // has no `::oo::Helpers` member, so it still reaches `::puts`.
        let src = "oo::class create C {\n    method Go {} {\n        puts hi\n    }\n}\n";
        assert_eq!(
            resolution_of(src, "tcl9.0", "puts").as_deref(),
            Some("::puts"),
        );
    }
}

mod oo_define_configuration {
    use super::super::*;

    #[test]
    fn a_class_keeps_its_methods_after_oo_define_adds_one() {
        // tclsh 8.6.18, 9.0.4 and 9.1.0: after `oo::define C {method b …}`,
        // `obj b` returns 1 and `puts hi` runs; `obj nosuch` raises
        // `unknown method "nosuch": must be a, b or destroy`.
        let prologue = concat!(
            "oo::class create C {method a {} {}}\n",
            "oo::define C {method b {} {return 1}}\n",
            "C create obj\n",
        );
        let rows: [(&str, &[&str]); 2] = [("obj b\nputs hi\n", &[]), ("obj nosuch\n", &["W308"])];
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (tail, expected) in rows {
                let src = format!("{prologue}{tail}");
                let codes: Vec<String> = analyser_diags(&src, release)
                    .into_iter()
                    .map(|(code, _, _)| code)
                    .filter(|code| code.starts_with("W3"))
                    .collect();
                assert_eq!(codes, expected, "{release}: {src}");
            }
        }
    }

    #[test]
    fn a_class_reached_through_a_substituted_word_is_no_closed_configuration() {
        // tclsh 8.6.18, 9.0.4 and 9.1.0 run `oo::define $c …` on `C` too, but
        // the class is selected by a value, a relation a rename cannot follow:
        // the configuration stays open, so the instance keeps no method table
        // and `obj nosuch` draws no W308.
        let src = concat!(
            "oo::class create C {method a {} {}}\n",
            "set c C\n",
            "oo::define $c {method b {} {return 1}}\n",
            "C create obj\n",
            "obj nosuch\n",
        );
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let codes: Vec<String> = analyser_diags(src, release)
                .into_iter()
                .map(|(code, _, _)| code)
                .filter(|code| code == "W308")
                .collect();
            assert!(codes.is_empty(), "{release}: {codes:?}");
        }
    }
}

mod reopened_classes {
    use super::super::Analyser;

    #[test]
    fn a_class_another_file_declares_receives_a_contribution_not_a_class() {
        // a.tcl: `oo::class create ::C { self { method cm {} { return 1 } } }`.
        // tclsh 8.6.18, 9.0.4 and 9.1.0: `source a.tcl; source b.tcl; ::C cm`
        // raises `unknown method "cm"`, so b.tcl's members reach the class
        // a.tcl declares; b.tcl alone raises `::C does not refer to an
        // object`, so b.tcl declares no class. Its members are a contribution
        // keyed by the first path the original lookup tries.
        let rows = [
            ("oo::define ::C { self unexport cm }\n", "::C"),
            (
                "namespace eval a {\n    oo::define C { self unexport cm }\n}\n",
                "::a::C",
            ),
        ];
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (src, key) in rows {
                let analysis = Analyser::new().analyse(src, release);
                assert!(analysis.all_classes.is_empty(), "{release}: {src}");
                let contribution = analysis
                    .external_class_contributions
                    .get(key)
                    .unwrap_or_else(|| panic!("{release}: no {key} contribution for {src}"));
                assert!(
                    contribution.class_unexports.contains("cm"),
                    "{release}: {src}: {:?}",
                    contribution.class_unexports
                );
            }
        }
    }
}
