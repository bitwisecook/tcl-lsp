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

//! Members installed reflectively by `oo::define` loops, `foreach` installers and brace expansion.

use super::*;

#[test]
fn oo_define_over_a_literal_foreach_list_extends_every_named_class() {
    // TP: the ticklecharts `etsb.tcl` monkey-patch.  Each
    // literal element names a real class, so each gets the injected
    // method; nothing lands under a synthetic `@dynclass@` key.
    let src = concat!(
        "oo::class create chart { method Render {} { return 1 } }\n",
        "oo::class create timeline { method Render {} { return 1 } }\n",
        "foreach cls {chart timeline} {\n",
        "    oo::define $cls {\n",
        "        method RenderTsb {} { return tsb }\n",
        "    }\n",
        "}\n",
    );
    let r = analysis(src, "tcl9.0");
    for name in ["::chart", "::timeline"] {
        assert!(
            r.all_classes[name].methods.contains_key("RenderTsb"),
            "{name} gained RenderTsb: {:?}",
            r.all_classes[name].methods.keys().collect::<Vec<_>>()
        );
    }
    assert!(
        !r.all_classes.keys().any(|k| k.contains("@dynclass@")),
        "no synthetic class survives: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_foreach_simulation_does_not_duplicate_body_diagnostics() {
    // FP guard — the simulation re-dispatches only the named-definition
    // installer, once per element.  A diagnostic raised inside the
    // injected member's own body must still be reported exactly once
    // per class it lands on, never once per (class × iteration).
    let src = concat!(
        "oo::class create chart { method Render {} { return 1 } }\n",
        "oo::class create timeline { method Render {} { return 1 } }\n",
        "foreach cls {chart timeline} {\n",
        "    oo::define $cls {\n",
        "        method RenderTsb {} { return $undefinedVar }\n",
        "    }\n",
        "}\n",
    );
    let spans: Vec<_> = analysis(src, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.span))
        .collect();
    let mut seen = std::collections::HashSet::new();
    for entry in &spans {
        assert!(
            seen.insert(entry.clone()),
            "duplicate diagnostic {entry:?} in {spans:?}",
        );
    }
}

#[test]
fn oo_define_over_a_dynamic_list_still_abstains() {
    // TN — the element list is a runtime value, so no class name is
    // knowable and the members are attributed to no class.
    let src = concat!(
        "oo::class create chart { method Render {} { return 1 } }\n",
        // A list only the run supplies; an unset `$classes` would raise
        // `can't read "classes"` (tclsh 8.6.18, 9.0.4, 9.1.0) before the loop.
        "foreach cls [lrange $argv 0 end] {\n",
        "    oo::define $cls {\n",
        "        method RenderTsb {} { return tsb }\n",
        "    }\n",
        "}\n",
    );
    let r = analysis(src, "tcl9.0");
    assert!(
        !r.all_classes["::chart"].methods.contains_key("RenderTsb"),
        "a runtime target must not be attributed to a literal class"
    );
    // Under a pinned release a configuration of a class the document does
    // not name records no class of its own (ruling of 17 November on
    // `oo::define` targets the document does not declare).
    assert_eq!(
        r.all_classes.keys().collect::<Vec<_>>(),
        ["::chart"],
        "a runtime target records no class"
    );
}

#[test]
fn oo_define_resolves_a_dominating_constant_target() {
    // TP — the narrowest form of the same fact: a target bound by a
    // plain `set` that dominates the call extends the real class.
    let src = concat!(
        "oo::class create Widget {\n",
        "    method real {} { return 1 }\n",
        "}\n",
        "set cls Widget\n",
        "oo::define $cls method added {} { return 2 }\n",
    );
    let cd = class(src, "::Widget");
    assert!(cd.methods.contains_key("real"), "{cd:?}");
    assert!(cd.methods.contains_key("added"), "{cd:?}");
}

#[test]
fn static_brace_expansion_splices_a_member_signature() {
    // TP: `{*}` of a braced literal is spliced by the parser,
    // so `method {*}{foo {} {…}}` defines a real `foo` (verified on
    // tclsh 9.0.4 and 8.6.16, in both the `oo::class create` body and
    // an `oo::define` body).
    let src = concat!(
        "oo::class create S {\n",
        "    method {*}{foo {x} {return foo-$x}}\n",
        "    constructor {*}{{a b} {return ctor}}\n",
        "}\n",
    );
    let cd = class(src, "::S");
    assert!(cd.methods.contains_key("foo"), "{cd:?}");
    assert_eq!(cd.methods["foo"].params.len(), 1);
    assert_eq!(cd.constructors.len(), 1, "{cd:?}");
    assert_eq!(cd.constructors[0].params.len(), 2);
}

#[test]
fn static_brace_expansion_splices_in_an_oo_define_body_too() {
    // TP — same mechanism through the `oo::define` body form.
    let src = concat!(
        "oo::class create S {}\n",
        "oo::define S {\n",
        "    method {*}{bar {} {return bar}}\n",
        "}\n",
    );
    assert!(class(src, "::S").methods.contains_key("bar"));
}

#[test]
fn substituted_brace_expansion_still_abstains() {
    // TN — `{*}[info class definition …]` reflects a signature whose
    // words are only known at run time, so the member is left
    // unrecorded rather than invented with wrong parameters or a body
    // span pointing at the wrong text.
    let src = concat!(
        "oo::class create chart3D {\n",
        "    constructor {*}[info class constructor chart]\n",
        "    method options {*}[classDef chart options]\n",
        "    method getType {} { return chart3D }\n",
        "}\n",
    );
    let cd = class(src, "::chart3D");
    assert!(cd.methods.contains_key("getType"), "{cd:?}");
    assert!(!cd.methods.contains_key("options"), "{cd:?}");
    assert!(cd.constructors.is_empty(), "{cd:?}");
    // …and the abstention is *recorded*, so the tables read as a lower
    // bound rather than as the class's whole surface.
    assert!(cd.member_set_incomplete, "{cd:?}");
}

#[test]
fn a_foreach_member_installer_marks_the_member_set_incomplete() {
    // TP: the ticklecharts `chart3D` installer
    // loop.  `foreach` is not a member word and carries a script the
    // member walk never descends into, so the class must say its member
    // tables are a lower bound. tclsh 9.0.4 / 8.6.16: `info class
    // methods ::C3` really lists `options` and `globalOptions`.
    //
    // The loop's own literal list means the two
    // *names* are not invisible either (see
    // `a_literal_foreach_installer_records_member_names_as_signature_unknown`
    // below for the full positive case) — but the set stays incomplete
    // regardless, because knowing these two names is not the same as
    // knowing there could never be a third from elsewhere.
    let src = concat!(
        "oo::class create C3 {\n",
        "    foreach m {options globalOptions} {\n",
        "        method $m {} { return $m }\n",
        "    }\n",
        "    method getType {} { return c3 }\n",
        "}\n",
    );
    let cd = class(src, "::C3");
    assert!(cd.methods.contains_key("getType"), "{cd:?}");
    assert!(cd.methods.contains_key("options"), "{cd:?}");
    assert!(cd.methods.contains_key("globalOptions"), "{cd:?}");
    assert!(cd.member_set_incomplete, "{cd:?}");
}

#[test]
fn a_literal_foreach_installer_records_member_names_as_signature_unknown() {
    // TP: the headline case: `foreach m {alpha beta gamma} {
    // method $m {args} {…} }`. tclsh 9.0.4 / 8.6.16 both agree `alpha`,
    // `beta`, `gamma` are real members of any arity (`args`); the walk can read their
    // *names* off the loop's own literal list even though it still
    // cannot say anything honest about their signatures.
    let src = concat!(
        "oo::class create C3 {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "    method getType {} { return c3 }\n",
        "}\n",
    );
    let cd = class(src, "::C3");
    for name in ["alpha", "beta", "gamma"] {
        let md = cd
            .methods
            .get(name)
            .unwrap_or_else(|| panic!("{name} recorded: {cd:?}"));
        assert!(md.params.is_empty(), "{name}: {md:?}");
        assert!(md.params_computed, "{name}: {md:?}");
        // Abstains rather than validating against an invented
        // parameter list (E002/E003/E005) — `arity()` reads as
        // unbounded, not "zero args".
        assert_eq!(
            md.arity(),
            tcl_registry::Arity::new(0, tcl_registry::Arity::UNLIMITED),
            "{name}: {md:?}"
        );
    }
    // A directly-written literal declaration is untouched by the loop.
    let getter = cd.methods.get("getType").expect("getType recorded");
    assert!(!getter.params_computed, "{getter:?}");
    // The lower bound still holds even though three names are now known
    // (the subtle half of the acceptance criteria: knowing three names
    // is not knowing them all).
    assert!(cd.member_set_incomplete, "{cd:?}");
}

#[test]
fn a_computed_foreach_member_name_still_abstains_entirely() {
    // TN — the loop-installed name must be *exactly* a reference to the
    // loop variable; anything else built from it (or a name that isn't
    // the loop variable at all) is left exactly as opaque, matching
    // the pre-existing `method $someVar …` abstention.
    let src = concat!(
        "oo::class create C4 {\n",
        "    set prefix pre\n",
        "    foreach m {alpha beta} {\n",
        "        method ${prefix}_$m {args} { return $args }\n",
        "    }\n",
        "    method getType {} { return c4 }\n",
        "}\n",
    );
    let cd = class(src, "::C4");
    assert!(cd.methods.contains_key("getType"), "{cd:?}");
    assert!(!cd.methods.contains_key("alpha"), "{cd:?}");
    assert!(!cd.methods.contains_key("pre_alpha"), "{cd:?}");
    assert_eq!(cd.methods.len(), 1, "{cd:?}");
    assert!(cd.member_set_incomplete, "{cd:?}");
}

#[test]
fn a_multi_pair_foreach_installer_still_abstains_entirely() {
    // TN — two var/list pairs share one body; no single member `$var`
    // word could unambiguously mean one pair over the other, so this is
    // left exactly as opaque as any other unreadable installer.
    let src = concat!(
        "oo::class create C5 {\n",
        "    foreach m {alpha beta} n {1 2} {\n",
        "        method $m {args} { return $n }\n",
        "    }\n",
        "    method getType {} { return c5 }\n",
        "}\n",
    );
    let cd = class(src, "::C5");
    assert!(cd.methods.contains_key("getType"), "{cd:?}");
    assert!(!cd.methods.contains_key("alpha"), "{cd:?}");
    assert_eq!(cd.methods.len(), 1, "{cd:?}");
    assert!(cd.member_set_incomplete, "{cd:?}");
}

#[test]
fn an_ordinary_class_body_keeps_a_complete_member_set() {
    // TN — the flag is not a blanket "TclOO is hard" switch: a class
    // whose body is nothing but readable member declarations (including
    // the *statically spliceable* `{*}` form and a `self` wrapper block)
    // still reports a complete member set, so W308 keeps its teeth.
    let src = concat!(
        "oo::class create Plain {\n",
        "    superclass ::oo::object\n",
        "    variable a b\n",
        "    constructor {x} { set a $x }\n",
        "    method {*}{spliced {} {return 1}}\n",
        "    self { method make {} { return made } }\n",
        "    method run {} { return $a }\n",
        "    destructor { return }\n",
        "}\n",
    );
    assert!(!class(src, "::Plain").member_set_incomplete);
}

#[test]
fn w308_abstains_on_a_class_whose_members_are_installed_reflectively() {
    // TP: `$c3 options` must not draw
    // "Unknown method 'options'" on a call tclsh proves succeeds.  A
    // class whose member tables are a lower bound cannot support a
    // missing-method claim, so W308 must abstain — while the sibling
    // class in the same file, whose body *is* fully readable, keeps
    // being checked (the TN half, guarding against a blanket
    // suppression).
    let src = concat!(
        "oo::class create Donor {\n",
        "    method options {} { return 1 }\n",
        "}\n",
        "oo::class create C3 {\n",
        "    constructor {*}[info class constructor ::Donor]\n",
        "    foreach m {options} { method $m {*}[info class definition ::Donor $m] }\n",
        "}\n",
        "set c3 [C3 new]\n",
        "$c3 options\n",
        "set d [Donor new]\n",
        "$d nosuch\n",
    );
    let r = analysis(src, "tcl9.0");
    let codes: Vec<&str> = r
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W308")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        codes.len(),
        1,
        "only the readable class draws W308: {codes:?}"
    );
    assert!(codes[0].contains("nosuch"), "{codes:?}");
}

#[test]
fn foreach_installed_procs_are_enumerated_per_literal_element() {
    // The ticklecharts `etypes.tcl` ensemble shape: the
    // `foreach`-literal simulation covers `proc`, so every element's
    // proc is registered under its real qualified name.
    let src = concat!(
        "namespace eval ticklecharts {}\n",
        "foreach ptype {elist elist.n elist.s} {\n",
        "    proc ticklecharts::${ptype} {args} { return $args }\n",
        "}\n",
    );
    let r = analysis(src, "tcl8.6");
    for name in [
        "::ticklecharts::elist",
        "::ticklecharts::elist.n",
        "::ticklecharts::elist.s",
    ] {
        assert!(
            r.all_procs.contains_key(name),
            "{name} registered: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
    assert!(
        !r.all_procs.keys().any(|k| k.contains('$')),
        "no proc is filed under the unsubstituted template: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}
