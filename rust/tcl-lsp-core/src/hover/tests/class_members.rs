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

//! Class-member cards: sibling and `my` dispatch, member declarations and `$obj method` calls.

use super::*;

#[test]
fn hover_on_self_method_call_site_resolves() {
    // TP — `self method make {n} {...}` records into `class_methods`,
    // and `Widget make gadget`'s bare class-command receiver resolves
    // against it, so hover on the call site works end-to-end.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
    let analysis = analyse(src);
    // Cursor on `make` in `Widget make gadget` (line 3, col 8).
    let h = hover(src, 3, 8, &analysis, None).expect("expected hover for self-method call");
    assert!(h.value.contains("classmethod"), "{}", h.value);
    assert!(h.value.contains("Widget::make"), "{}", h.value);
}

#[test]
fn method_hover_notes_inheritance_and_override() {
    let src = "oo::class create A {\n    method greet {} {}\n}\noo::class create B {\n    superclass A\n    method greet {} {}\n}\noo::class create C {\n    superclass A\n}\n";
    let analysis = analyse(src);
    // B::greet overrides A::greet.
    let over = oo_method_resolution_note(&analysis, "::B", "greet").unwrap_or_default();
    assert!(
        over.contains("overrides") && over.contains("::A::greet"),
        "{over}"
    );
    // C inherits greet from A.
    let inh = oo_method_resolution_note(&analysis, "::C", "greet").unwrap_or_default();
    assert!(
        inh.contains("inherited from") && inh.contains("::A"),
        "{inh}"
    );
    // A::greet defined only here — no note.
    assert!(oo_method_resolution_note(&analysis, "::A", "greet").is_none());
}

// class-member hover

#[test]
fn class_member_hover_bare_sibling_method_without_link_abstains() {
    // FP — a bareword sibling method call is NOT
    // actually reachable from another method's body unless `link`
    // exposed it that way; real tclsh: "invalid command name". Must
    // abstain rather than falsely resolve.
    let src = "oo::class create C {\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` invocation (line 2, col 22).
    assert!(hover(src, 2, 22, &analysis, None).is_none());
}

#[test]
fn class_member_hover_linked_sibling_method_resolves() {
    // C9's selected link schema couples the alias and target name.
    let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method twice {} { greet ; greet }` — same text/col
    // as the un-linked test above, one line further down.
    let h = hover(src, 3, 22, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("C::greet"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
    assert!(!h.value.contains("linked from"), "{}", h.value);
}

#[test]
fn class_member_hover_resolves_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; the
    // `link`, the linked method, and the bareword call site that depends
    // on it are all added via a *separate*, later `oo::define Gadget
    // { ... }` block — the `ticklecharts::chart` shape. Hover on
    // the bareword `Helper` call must still resolve — the cursor sits
    // inside that separate block, which `class_member_hover_text`'s
    // `enclosing_class_at` containment check must recognise as part of
    // `Gadget`'s body too, not just the original creation block.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    constructor {} { link Helper }\n    method Helper {} { return hi }\n    method Caller {} { Helper }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 6: `    method Caller {} { Helper }` — cursor on the bareword
    // `Helper` call (col 23).
    let h = hover(src, 6, 23, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("Gadget::Helper"), "{}", h.value);
}

#[test]
fn class_member_hover_two_element_link_alias_notes_the_real_target() {
    // C9's two-field link keeps the alias distinct from its target member.
    let src = "oo::class create C {\n    constructor {} { link {shortcut realMethod} }\n    method realMethod {x} { return $x }\n    method bar {} { return [shortcut 42] }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method bar {} { return [shortcut 42] }` — col 28
    // lands on the `s` of `shortcut`.
    let h = hover(src, 3, 28, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("C::realMethod"), "{}", h.value);
    assert!(h.value.contains("linked from `shortcut`"), "{}", h.value);
}

// `my method` internal-dispatch hover

#[test]
fn my_dispatch_hover_resolves_a_plain_call_in_a_single_block_class() {
    // Explicit self-dispatch selects its member without a bareword link.
    let src = "oo::class create geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method WhichAmI {} { return [my GetType] }` — cursor
    // on `GetType` inside `my GetType` (col 38).
    let h = hover(src, 2, 38, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
}

#[test]
fn my_dispatch_hover_resolves_when_class_extended_via_separate_oo_define() {
    // TP — the two-block `oo::class create` + separate `oo::define`
    // pattern (constructor in `create`, every method including the
    // dispatched-on one in a later `define`), exercised here for hover
    // specifically; definition and references have their own tests for
    // the same shape.
    let src = "oo::class create geo::Plane {\n    constructor {args} {}\n}\noo::define geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
    let analysis = analyse(src);
    // Line 5: `    method WhichAmI {} { return [my GetType] }` — cursor
    // on `GetType` inside `my GetType` (col 38).
    let h = hover(src, 5, 38, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
}

#[test]
fn my_dispatch_hover_abstains_for_an_undefined_method() {
    // TN — `my` dispatching to a method the class genuinely doesn't
    // have must not fabricate a hover.
    let src = "oo::class create C {\n    method twice {} { return [my nope] }\n}\n";
    let analysis = analyse(src);
    // Line 1, cursor on `nope` inside `my nope` (col 34).
    assert!(hover(src, 1, 34, &analysis, None).is_none());
}

// A class member's own declaration hovers as that member, in the
// creation block and in a reopening `oo::define` block alike — the same
// way `proc` and the class name itself hover at their declarations.

#[test]
fn member_declaration_hover_resolves_in_the_creation_block() {
    // TP — the baseline: `method plain` hovers at its own name token.
    let src = "oo::class create Foo {\n    method plain {a} { return $a }\n}\n";
    let analysis = analyse(src);
    // Line 1, col 13 — inside `plain`.
    let h = hover(src, 1, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Foo::plain"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
}

#[test]
fn member_declaration_hover_resolves_in_a_reopening_oo_define_block() {
    // TP — the SpiceGenTcl
    // `generalClasses.tcl`: `oo::configurable create Parameter { … }`
    // then a separate `oo::define Parameter { method <WriteProp-value>
    // … }`).  tclsh 9.0.4 dispatches `$f reopened hi` to the reopened
    // block's method, so it is every bit `Foo`'s own member and must
    // hover identically to one written in the creation block.
    let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\noo::define Foo {\n    method reopened {val} { return $val }\n}\n";
    let analysis = analyse(src);
    // Line 4, col 13 — inside `reopened`, in the reopening block.
    let h = hover(src, 4, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Foo::reopened"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
}

#[test]
fn member_declaration_hover_covers_classmethods_and_properties() {
    // TP — the other two member tables render with their own labels.
    let src = "oo::configurable create C {\n    property size\n    self method make {n} { return $n }\n}\n";
    let analysis = analyse_for(src, "tcl9.0");
    let prop = hover(src, 1, 15, &analysis, None).expect("property hover");
    assert!(prop.value.contains("**property**"), "{}", prop.value);
    assert!(prop.value.contains("::C::size"), "{}", prop.value);
    let cm = hover(src, 2, 19, &analysis, None).expect("classmethod hover");
    assert!(cm.value.contains("**classmethod**"), "{}", cm.value);
    assert!(cm.value.contains("::C::make"), "{}", cm.value);
}

#[test]
fn member_declaration_hover_notes_an_override() {
    // TP — the declaration renders the same MRO note a call site does,
    // so the two descriptions of one member cannot disagree.
    let src = "oo::class create Base {\n    method run {} { return base }\n}\noo::class create Derived {\n    superclass Base\n    method run {} { return derived }\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 5, 12, &analysis, None).expect("hover");
    assert!(h.value.contains("::Derived::run"), "{}", h.value);
    assert!(h.value.contains("Base"), "MRO note expected: {}", h.value);
}

#[test]
fn member_declaration_hover_does_not_fire_on_a_same_named_word_elsewhere() {
    // FP guard — only the declaration's own name token hovers as the
    // member.  A same-spelled *data* word inside a method body is not a
    // member reference, and must keep falling through the tier order
    // (here: to nothing at all).
    let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\n";
    let analysis = analyse(src);
    // Line 1, col 30 — the bareword `plain` in `return plain`, which is
    // a value, not a call (an un-linked sibling name is not callable).
    assert!(hover(src, 1, 30, &analysis, None).is_none());
}

#[test]
fn member_declaration_hover_abstains_outside_a_class_body() {
    // TN — a proc whose name happens to match a method of some class in
    // the file must not borrow that class's member hover.
    let src =
        "oo::class create Foo {\n    method plain {} { return 1 }\n}\nproc plain {} { return 2 }\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 6, &analysis, None).expect("proc hover");
    assert!(
        !h.value.contains("**method**"),
        "the proc, not Foo's method: {}",
        h.value
    );
}

#[test]
fn class_member_hover_bare_sibling_classmethod_without_link_abstains() {
    // FP — same shape as the method case above,
    // for `classmethod`.
    let src =
        "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = analyse(src);
    assert!(hover(src, 2, 20, &analysis, None).is_none());
}

#[test]
fn class_member_hover_linked_sibling_classmethod_resolves() {
    // C9's link retains the selected classmethod's member side.
    let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method use {} { factory }` — same text/col as the
    // un-linked test above, one line further down.
    let h = hover(src, 3, 20, &analysis, None).expect("hover");
    assert!(h.value.contains("**classmethod**"), "{}", h.value);
    assert!(h.value.contains("C::factory"), "{}", h.value);
}

#[test]
fn class_member_hover_fires_for_constructor_keyword() {
    let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 2, 27, &analysis, None).expect("hover");
    assert!(h.value.contains("constructor"), "{}", h.value);
    // Class qualified name is `::C`.
    assert!(h.value.contains("::C"), "{}", h.value);
}

#[test]
fn class_member_hover_skipped_outside_class_body() {
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the bare `greet` outside the class body.
    // No proc / class / method match — should return None.
    assert!(hover(src, 3, 2, &analysis, None).is_none());
}

// $obj method dispatch

#[test]
fn obj_method_hover_fires_for_known_instance() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Line 4 `$d bark` — cursor on `bark` (col 3).
    let h = hover(src, 4, 3, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Dog::bark"), "{}", h.value);
}

#[test]
fn obj_method_hover_fires_for_a_literal_foreach_installed_method() {
    // The member's name comes from the loop's own literal
    // list, not a written `method NAME` word, but a call site must
    // still resolve to a hover — go-to-definition and hover share the
    // same member table this walk populates.
    let src = concat!(
        "oo::class create Widget {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "}\n",
        "set d [Widget new]\n",
        "$d alpha\n",
    );
    let analysis = analyse(src);
    // Line 6 `$d alpha` — cursor on `alpha` (col 3).
    let h = hover(src, 6, 3, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Widget::alpha"), "{}", h.value);
}

#[test]
fn obj_method_hover_none_for_unknown_instance() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
    let analysis = analyse(src);
    // `x` has no recorded class — no hover.
    assert!(hover(src, 3, 3, &analysis, None).is_none());
}

/// A Tk widget's instance command (a registry-modelled, self-referential
/// `object_class`, not a user-defined one) needs the
/// registry passed to resolve hover text at all.
#[test]
fn obj_method_hover_fires_for_bareword_widget() {
    let reg = tcl_registry::CommandRegistry::build_default();
    let src = "package require Tk\nttk::treeview .t\n.t instate {selected} {}\n";
    let analysis = analyse(src);
    // Line 2 `.t instate …` — cursor on `instate` (col 3).
    let h = hover(src, 2, 3, &analysis, Some(&reg)).expect("hover");
    assert!(h.value.contains("ttk::treeview instate"), "{}", h.value);
}

#[test]
fn widget_method_hover_follows_the_resolved_tk_lifecycle() {
    // ttk::treeview's `current` instance method is introduced by Tk 9.1.
    // The registry lookup is the only method-specific input here, so this
    // proves an older profile cannot render an impossible method hover.
    let src = "package require Tk\nttk::treeview .tree\n.tree current\n";
    for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
        let analysis = analyse_for(src, dialect);
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let hover = hover_with_profile(src, 2, 6, &analysis, Some(registry), profile);
        assert_eq!(
            hover.is_some(),
            expected,
            "ttk::treeview current hover under {dialect}: {hover:?}"
        );
    }

    // A document's unconditional package require can raise the profile
    // floor. The request-time hover path passes that explicit package
    // floor to the same registry lookup instead of baking Tk knowledge
    // into the provider.
    let raised = "package require Tk 9.1\nttk::treeview .tree\n.tree current\n";
    let raised_analysis = analyse_for(raised, "tcl9.0");
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let raised_hover = hover_with_profile(
        raised,
        2,
        6,
        &raised_analysis,
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
        tcl90,
    );
    assert!(
        raised_hover.is_some(),
        "an explicit Tk 9.1 floor must expose current: {raised_hover:?}"
    );
}

/// Without a registry passed, the same widget dispatch has nowhere to
/// look up the method — `obj_method_hover_text`'s registry fallback is
/// skipped, not a panic.
#[test]
fn obj_method_hover_none_for_widget_without_registry() {
    let src = "ttk::treeview .t\n.t instate {selected} {}\n";
    let analysis = analyse(src);
    assert!(hover(src, 1, 3, &analysis, None).is_none());
}

#[test]
fn obj_method_hover_fires_for_var_captured_widget() {
    let reg = tcl_registry::CommandRegistry::build_default();
    let src = "package require Tk\nset lb [listbox .l]\n$lb curselection\n";
    let analysis = analyse(src);
    // Line 2 `$lb curselection` — cursor on `curselection` (col 4).
    let h = hover(src, 2, 4, &analysis, Some(&reg)).expect("hover");
    assert!(h.value.contains("listbox curselection"), "{}", h.value);
}
