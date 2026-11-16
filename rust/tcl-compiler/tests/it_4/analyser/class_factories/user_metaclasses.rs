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

//! User metaclasses that manufacture classes: `create` overrides, prologues, relative superclasses and renamed metaclasses.

use super::*;

/// The Tk `library/megawidget.tcl` factory, reduced to its mechanism.
const MEGAWIDGET: &str = concat!(
    "oo::class create Megawidget {\n",
    "    superclass oo::class\n",
    "    self method create {name superclasses body} {\n",
    "        next $name [list superclass MegawidgetClass {*}$superclasses]\\;$body\n",
    "    }\n",
    "}\n",
    "oo::class create MegawidgetClass {\n",
    "    method TraceOption {a b} { return traced }\n",
    "}\n",
    "Megawidget create SimpleWidget {} {\n",
    "    method CreateHull {} { my TraceOption a b }\n",
    "}\n",
    "Megawidget create FocusableWidget SimpleWidget {\n",
    "    method CreateHull {} { my TraceOption c d }\n",
    "}\n",
    "Megawidget create IconList FocusableWidget {\n",
    "    method GetSpecs {} { return iconlist }\n",
    "}\n",
);

#[test]
fn user_metaclass_creates_real_classes() {
    // TP: a class whose own superclass chain reaches `oo::class` is a
    // class factory, so its `create` calls must introduce real
    // classes — entering `all_classes` with an outline entry,
    // references, and `next` resolution.
    let r = analysis(MEGAWIDGET, "tcl9.0");
    for name in ["::SimpleWidget", "::FocusableWidget", "::IconList"] {
        assert!(
            r.all_classes.contains_key(name),
            "{name} recorded: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn user_metaclass_body_argument_is_read_from_the_create_override() {
    // TP — the override's signature is `{name superclasses body}`, so
    // the body is argument 3, not the builtin `create Name Body`
    // argument 2.  Reading the builtin position would walk the
    // superclass word as a script and lose every member.
    assert!(
        class(MEGAWIDGET, "::SimpleWidget")
            .methods
            .contains_key("CreateHull")
    );
    assert!(
        class(MEGAWIDGET, "::IconList")
            .methods
            .contains_key("GetSpecs")
    );
}

#[test]
fn user_metaclass_splices_its_own_and_the_callers_superclasses() {
    // TP — matches `info class superclasses` on tclsh 9.0.4 / 8.6.16
    // exactly, including the implicit `MegawidgetClass` the factory
    // injects and the caller-supplied list spliced by `{*}$superclasses`.
    assert_eq!(
        class(MEGAWIDGET, "::SimpleWidget").superclasses,
        ["MegawidgetClass"]
    );
    assert_eq!(
        class(MEGAWIDGET, "::FocusableWidget").superclasses,
        ["MegawidgetClass", "SimpleWidget"]
    );
    assert_eq!(
        class(MEGAWIDGET, "::IconList").superclasses,
        ["MegawidgetClass", "FocusableWidget"]
    );
}

#[test]
fn a_factory_made_class_with_a_readable_override_is_not_opaque() {
    // TN — inheritance is fully known here, so method checks stay live
    // rather than being blanket-suppressed.
    assert!(!class(MEGAWIDGET, "::FocusableWidget").inheritance_unknown);
}

#[test]
fn an_unreadable_manufacturer_override_marks_inheritance_unknown() {
    // Abstention — the override builds its prologue from a runtime
    // value, so the superclass list cannot be known.  The class is
    // still recorded (it exists), but its inheritance is flagged
    // opaque so no method-existence check fires against a guess.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    self method create {name extra body} {\n",
        "        next $name [list superclass [pickBase $extra]]\\;$body\n",
        "    }\n",
        "}\n",
        "Meta create Widget somewhere {\n",
        "    method m {} { return 1 }\n",
        "}\n",
    );
    let cd = class(src, "::Widget");
    assert!(cd.inheritance_unknown, "{cd:?}");
    assert!(cd.superclasses.is_empty(), "{cd:?}");
    assert!(cd.methods.contains_key("m"), "{cd:?}");
}

/// A factory whose `create` override composes the definition body as a
/// **string** — no nested command anywhere — yet really does inject a
/// superclass.  tclsh 9.0.4 and 8.6.16 both report `StrWidget supers:
/// ::StrBase` for this and run `[[StrWidget new] inherited]`.
const STRING_PROLOGUE_META: &str = concat!(
    "oo::class create StrBase {\n",
    "    method inherited {} { return inherited-ran }\n",
    "}\n",
    "oo::class create StrMeta {\n",
    "    superclass oo::class\n",
    "    self method create {name body} {\n",
    "        next $name \"superclass StrBase\\n$body\"\n",
    "    }\n",
    "}\n",
    "StrMeta create StrWidget {\n",
    "    method own {} { return own-ran }\n",
    "}\n",
);

/// The same shape with the override passing `$body` straight through —
/// nothing composed, nothing injected.  tclsh 9.0.4 / 8.6.16 report
/// only the implicit `::oo::object` for it, i.e. a genuinely empty
/// injection.
const DIRECT_BODY_META: &str = concat!(
    "oo::class create PlainMeta {\n",
    "    superclass oo::class\n",
    "    self method create {name body} {\n",
    "        next $name $body\n",
    "    }\n",
    "}\n",
    "PlainMeta create PlainWidget {\n",
    "    method own {} { return plain-own }\n",
    "}\n",
);

#[test]
fn a_string_built_prologue_is_opaque_not_empty() {
    // TP — the prologue injects `superclass StrBase` with no command
    // substitution at all, so the scan finds nothing to read.  Claiming
    // a *known-empty* injection would assert the class has no
    // superclass and let W308 fire on every inherited method; the only
    // sound answer is opaque.
    let cd = class(STRING_PROLOGUE_META, "::StrWidget");
    assert!(cd.inheritance_unknown, "{cd:?}");
    assert!(
        cd.methods.contains_key("own"),
        "the class is still real: {cd:?}"
    );
}

#[test]
fn an_opaque_prologue_suppresses_w308_on_inherited_methods() {
    // TP (consumer) — `inherited` really runs on both interpreters, so
    // no unknown-method warning may fire for it.
    let src = concat!(
        "oo::class create StrBase {\n",
        "    method inherited {} { return inherited-ran }\n",
        "}\n",
        "oo::class create StrMeta {\n",
        "    superclass oo::class\n",
        "    self method create {name body} {\n",
        "        next $name \"superclass StrBase\\n$body\"\n",
        "    }\n",
        "}\n",
        "StrMeta create StrWidget {\n",
        "    method own {} { return own-ran }\n",
        "}\n",
        "set w [StrWidget new]\n",
        "$w inherited\n",
    );
    assert!(
        !fires(src, "tcl9.0", "W308"),
        "an unreadable prologue must abstain: {:?}",
        codes(src, "tcl9.0")
    );
}

#[test]
fn a_provably_direct_body_still_yields_a_known_empty_injection() {
    // TN — `next $name $body` composes nothing, so the injection is
    // known-empty and the class's inheritance is fully known.  This is
    // the half that must *not* become opaque, or the abstention above
    // would cost every precise diagnostic on a factory-made class.
    let cd = class(DIRECT_BODY_META, "::PlainWidget");
    assert!(!cd.inheritance_unknown, "{cd:?}");
    assert!(cd.superclasses.is_empty(), "{cd:?}");
    assert!(cd.methods.contains_key("own"), "{cd:?}");
}

#[test]
fn a_provably_direct_body_keeps_method_checks_live() {
    // FN guard — the known-empty case must stay precise: a method that
    // exists nowhere on the class still warns.
    let src = concat!(
        "oo::class create PlainMeta {\n",
        "    superclass oo::class\n",
        "    self method create {name body} {\n",
        "        next $name $body\n",
        "    }\n",
        "}\n",
        "PlainMeta create PlainWidget {\n",
        "    method own {} { return plain-own }\n",
        "}\n",
        "set w [PlainWidget new]\n",
        "$w nosuchmethod\n",
    );
    assert!(
        fires(src, "tcl9.0", "W308"),
        "a known-empty injection keeps method checks live: {:?}",
        codes(src, "tcl9.0")
    );
}

#[test]
fn a_relative_superclass_reaches_the_metaclass_in_its_own_namespace() {
    // TP — `::n::DerivedMeta` declares `superclass Meta`, which Tcl
    // resolves in the declaring class's own namespace: tclsh 9.0.4 /
    // 8.6.16 both report `info class superclasses ::n::DerivedMeta` =
    // `::n::Meta`, and `::n::DerivedMeta create ::NsWidget {…}` really
    // makes a class.  Looking only at `Meta` / `::Meta` missed it.
    let src = concat!(
        "namespace eval ::n {\n",
        "    oo::class create Meta {\n",
        "        superclass oo::class\n",
        "        self method create {name body} { next $name $body }\n",
        "    }\n",
        "    oo::class create DerivedMeta {\n",
        "        superclass Meta\n",
        "    }\n",
        "}\n",
        "::n::DerivedMeta create ::NsWidget {\n",
        "    method nsown {} { return ns-own }\n",
        "}\n",
    );
    let cd = class(src, "::NsWidget");
    assert!(cd.methods.contains_key("nsown"), "{cd:?}");
}

#[test]
fn a_relative_superclass_prefers_its_own_namespace_over_a_decoy() {
    // TN (cross-link guard) — a same-tailed class
    // in an unrelated namespace must not be picked when the declaring
    // namespace has its own.  Here `::other::Meta` is a plain class and
    // `::n::Meta` is the real metaclass; picking the decoy would leave
    // `::NsWidget` unrecorded.
    let src = concat!(
        "namespace eval ::other {\n",
        "    oo::class create Meta {\n",
        "        method notafactory {} { return 1 }\n",
        "    }\n",
        "}\n",
        "namespace eval ::n {\n",
        "    oo::class create Meta {\n",
        "        superclass oo::class\n",
        "        self method create {name body} { next $name $body }\n",
        "    }\n",
        "    oo::class create DerivedMeta {\n",
        "        superclass Meta\n",
        "    }\n",
        "}\n",
        "::n::DerivedMeta create ::NsWidget {\n",
        "    method nsown {} { return ns-own }\n",
        "}\n",
    );
    assert!(class(src, "::NsWidget").methods.contains_key("nsown"));
}

#[test]
fn a_class_declared_in_a_namespace_body_types_its_named_instance() {
    // TP — tclsh 8.6.18, 9.0.4 and 9.1.0 run this to
    // `unknown method "nosuch": must be destroy or m`: the namespace body
    // has run by the time `::n::C create x` is read.
    let src = concat!(
        "namespace eval ::n {oo::class create C {method m {} {}}}\n",
        "::n::C create x\n",
        "x m\n",
        "x nosuch\n",
    );
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let r = analysis(src, release);
        assert_eq!(
            r.instance_classes.get("x").map(String::as_str),
            Some("::n::C"),
            "{release}: {:?}",
            r.instance_classes
        );
        assert_eq!(count(src, release, "W308"), 1, "{release}");
    }
}

#[test]
fn a_named_instance_is_labelled_in_every_environment() {
    // The lenient `tcl` environment pins no interpreter, so its label is
    // conditional; a release proves the same one. tclsh 8.6.18 and 9.0.4:
    // `info object class obj` answers `::C`.
    let src = "oo::class create C {method ping {} {}}; C create obj";
    for environment in ["tcl", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let r = analysis(src, environment);
        assert_eq!(
            r.instance_classes.get("obj").map(String::as_str),
            Some("::C"),
            "{environment}: {:?}",
            r.instance_classes
        );
        let cards =
                tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards::capture(
                    &r,
                )
                .unwrap_or_else(|| panic!("{environment}: class cards"));
        let labels: Vec<(String, &str)> = cards
            .named_instance_display_reports()
            .into_iter()
            .map(|(label, class)| (label, class.qualified_name.as_str()))
            .collect();
        assert_eq!(labels, [("::obj".to_owned(), "::C")], "{environment}");
    }
}

#[test]
fn a_relative_superclass_does_not_cross_link_to_another_namespace() {
    // TN — the declaring namespace's own `Meta` is an ordinary class,
    // so the chain ends there.  Reaching sideways into `::other::Meta`
    // (which *is* a metaclass) would manufacture a class creation real
    // Tcl never performs — `::n::Meta` shadows it from `::n`.
    let src = concat!(
        "namespace eval ::other {\n",
        "    oo::class create Meta {\n",
        "        superclass oo::class\n",
        "        self method create {name body} { next $name $body }\n",
        "    }\n",
        "}\n",
        "namespace eval ::n {\n",
        "    oo::class create Meta {\n",
        "        method notafactory {} { return 1 }\n",
        "    }\n",
        "    oo::class create DerivedMeta {\n",
        "        superclass Meta\n",
        "    }\n",
        "}\n",
        "::n::DerivedMeta create ::NsWidget {\n",
        "    method nsown {} { return ns-own }\n",
        "}\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::NsWidget"),
        "a non-factory chain must not manufacture a class: {:?}",
        analysis(src, "tcl9.0")
            .all_classes
            .keys()
            .collect::<Vec<_>>(),
    );
}

#[test]
fn an_ordinary_class_is_not_treated_as_a_factory() {
    // TN — `Dog` is a plain class, so `Dog create rex` makes an
    // *instance*, never a class.  Only a chain that reaches
    // `oo::class` manufactures classes.
    let src = concat!(
        "oo::class create Dog {\n",
        "    method bark {} { return woof }\n",
        "}\n",
        "Dog create rex\n",
    );
    assert!(
        !analysis(src, "tcl9.0").all_classes.contains_key("::rex"),
        "an instance is not a class"
    );
}

#[test]
fn a_factory_without_a_create_override_keeps_the_builtin_layout() {
    // TP — no override means the inherited `oo::class` manufacturer
    // runs, so `Meta create Name Body` has its body at argument 2 and
    // splices no extra superclass.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "}\n",
        "Meta create Plain {\n",
        "    method m {} { return 1 }\n",
        "}\n",
    );
    let cd = class(src, "::Plain");
    assert!(cd.methods.contains_key("m"), "{cd:?}");
    assert!(cd.superclasses.is_empty(), "{cd:?}");
    assert!(!cd.inheritance_unknown, "{cd:?}");
}

#[test]
fn anonymous_derived_metaclass_new_does_not_invent_a_class_name_from_its_body() {
    // FP guard: `new` on Tcl 9's derived metaclasses generates the class
    // command name. Its first word is the definition body, not a name, so
    // the static class index must not create a nonsense source key.
    let result = analysis(
        "set cls [oo::abstract new { method probe {} { return ok } }]\n",
        "tcl9.0",
    );
    assert!(
        result.all_classes.is_empty(),
        "an anonymous class has no statically nameable index key: {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn unexported_create_with_namespace_does_not_manufacture_a_class() {
    // TN pinned against C Tcl 9.0.4 and 8.6: the structural method exists
    // but is unexported, so ordinary command dispatch on oo::class fails.
    // The analyser must not index a class that the interpreter never made.
    let result = analysis(
        "oo::class createWithNamespace ::WithNs ::objns { method probe {} {} }\n",
        "tcl9.0",
    );
    assert!(!result.all_classes.contains_key("::WithNs"));
}

// A `rename`d metaclass command manufactures nothing, unless the rename
// is in effect and actually called through.

/// TP: `oo::class create ::R::M`, `rename ::R::M
/// ::R::Mk`, then `::R::Mk create ::R::W { method go {} {…} }` records
/// `::R::W` with `go`, exactly as calling the metaclass under its
/// original name would.
#[test]
fn a_renamed_metaclass_still_manufactures_its_class() {
    let src = concat!(
        "namespace eval ::R {}\n",
        "oo::class create ::R::M { superclass oo::class }\n",
        "rename ::R::M ::R::Mk\n",
        "::R::Mk create ::R::W { method go {} { return went } }\n",
    );
    let cd = class(src, "::R::W");
    assert!(
        cd.methods.contains_key("go"),
        "renamed-metaclass creation must still record members: {cd:?}"
    );
    assert!(
        !cd.inheritance_unknown,
        "a proved renamed-metaclass creation is not inheritance-unknown: {cd:?}"
    );
}

/// TP control — the un-renamed form (metaclass created directly as
/// `::R::Mk`) records the identical class, so the rename case above is
/// judged against its own oracle rather than an assumption.
#[test]
fn the_control_without_a_rename_records_the_same_class() {
    let src = concat!(
        "namespace eval ::R {}\n",
        "oo::class create ::R::Mk { superclass oo::class }\n",
        "::R::Mk create ::R::W { method go {} { return went } }\n",
    );
    let cd = class(src, "::R::W");
    assert!(cd.methods.contains_key("go"), "{cd:?}");
}

/// FN guard — a rename written *after* the creation call it would have
/// covered must not retroactively enable it: order-gating is inherited
/// unchanged from `indirection::walk`'s "in effect" rule.
#[test]
fn a_rename_written_after_the_creation_call_does_not_apply() {
    let src = concat!(
        "namespace eval ::R {}\n",
        "oo::class create ::R::M { superclass oo::class }\n",
        "::R::Mk create ::R::W { method go {} { return went } }\n",
        "rename ::R::M ::R::Mk\n",
    );
    let r = analysis(src, "tcl9.0");
    assert!(
        !r.all_classes.contains_key("::R::W"),
        "a rename written after the call site must not retroactively apply: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}

/// FN guard — a `rename` to a name that is never actually used as a
/// metaclass call must not spuriously manufacture anything (the rename
/// machinery is exercised, but the renamed-through call itself is
/// missing).
#[test]
fn a_rename_with_no_call_through_the_new_name_manufactures_nothing() {
    let src = concat!(
        "namespace eval ::R {}\n",
        "oo::class create ::R::M { superclass oo::class }\n",
        "rename ::R::M ::R::Mk\n",
    );
    let r = analysis(src, "tcl9.0");
    assert!(
        !r.all_classes.contains_key("::R::W"),
        "no creation call means no manufactured class: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}
