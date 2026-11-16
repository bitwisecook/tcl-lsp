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

//! The `self { ... }` and `private { ... }` block forms: the side each member lands on and the retractions written under them.

use super::*;

// The `self { … }` / `private { … }` block forms.

#[test]
fn self_block_form_records_every_member_as_a_class_method() {
    // TP. Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C {
    //       self { method make {n} {…} ; method other {} {…} }
    //       method inst {} {…}
    //   }
    //   ::C make 7               -> made-7
    //   info object methods ::C  -> other make   (class-object side)
    //   info class methods ::C   -> inst         (instance side)
    //   oo::class create ::F {superclass ::C} ; ::F make 1
    //                            -> error: unknown method "make"
    // i.e. exactly what the `self method …` prefix form declares — which
    // is why both spellings land on the same recording path.
    let src = "oo::class create ::C {\n\
                   self {\n\
                   method make {n} { return $n }\n\
                   method other {} { return other }\n\
                   }\n\
                   method inst {} { return inst }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    for name in ["make", "other"] {
        let md = c
            .class_methods
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` must record as a class method"));
        assert_eq!(md.kind, "classmethod");
        assert!(
            md.is_self_method,
            "`self`-scoped members are not inherited by subclasses",
        );
        assert_eq!(md.visibility, "public");
    }
    // The instance side is untouched — `inst` stays an instance method and
    // the block's members never leak into it.
    assert!(c.methods.contains_key("inst"));
    assert!(!c.methods.contains_key("make"));
    assert!(!c.methods.contains_key("other"));
}

#[test]
fn self_block_form_records_through_oo_define_too() {
    // TP. Same block, reached through `oo::define` rather than the class
    // creation body (oracle: `oo::define ::D { self { method mk {} {…} } }`
    // → `info object methods ::D` is `mk`).
    let src = "oo::class create ::D {}\n\
                   oo::define ::D {\n\
                   self {\n\
                   method mk {} { return dmk }\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::D").expect("::D recorded");
    assert!(c.class_methods.contains_key("mk"));
}

#[test]
fn self_block_scoped_unexport_flips_the_class_side() {
    // TP. Oracle: `oo::class create ::E { self { method
    // hidden {} {…} ; unexport hidden } }` leaves `info object methods
    // ::E` empty while `-all -private` still lists `hidden`, and
    // `::E hidden` errors "unknown method" — so the block's `unexport`
    // really does apply to the class-object side. The member is recorded
    // *and* now carries the visibility the block gave it.
    let src = "oo::class create ::E {\n\
                   self {\n\
                   method hidden {} { return h }\n\
                   unexport hidden\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E").expect("::E recorded");
    assert!(c.class_methods.contains_key("hidden"));
    assert_eq!(c.class_methods["hidden"].visibility, "unexported");
}

#[test]
fn self_scoped_unexport_leaves_a_same_named_instance_method_alone() {
    // TN — block and prefix spellings both.
    // Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    //   oo::class create C { method m {} {return inst-m}
    //                        self { method m {} {return class-m}
    //                               unexport m } }
    //   info object methods ::C  ->            (class side unexported)
    //   info class methods ::C   -> m          (instance side untouched)
    //   ::C m                    -> unknown method "m"
    //   [::C new] m              -> inst-m
    for src in [
        "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self {\n\
             method m {} { return class }\n\
             unexport m\n\
             }\n\
             }",
        "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self method m {} { return class }\n\
             self unexport m\n\
             }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(
            c.class_methods["m"].visibility, "unexported",
            "class side must be flipped",
        );
        assert_eq!(
            c.methods["m"].visibility, "public",
            "instance side must NOT be flipped by a `self`-scoped unexport",
        );
        assert!(
            !c.unexports.contains("m"),
            "the class-level `unexports` set is the instance-side record; a \
                 class-object-side unexport must not enter it",
        );
    }
}

#[test]
fn unwrapped_unexport_leaves_the_class_side_alone() {
    // TN, the mirror direction. Oracle:
    //   oo::class create E2 { self { method onlyclass {} {…} } }
    //   oo::define E2 { unexport onlyclass }
    //   info object methods ::E2 -> onlyclass   (still exported)
    //   ::E2 onlyclass           -> oc          (still dispatches)
    // The unwrapped word acts on the instance side, where the name does
    // not exist — a silent no-op in real Tcl.
    let src = "oo::class create ::E2 {\n\
                   self { method onlyclass {} { return oc } }\n\
                   }\n\
                   oo::define ::E2 { unexport onlyclass }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E2").expect("::E2 recorded");
    assert_eq!(c.class_methods["onlyclass"].visibility, "public");
}

#[test]
fn cross_side_unexport_is_a_silent_no_op_not_an_error() {
    // TN / oracle-shape pin. Unlike `deletemethod` (a hard error that
    // aborts the definition), naming the *other* side's method in an
    // `export`/`unexport` is accepted and does nothing:
    //   oo::class create E { method onlyinst {} {…} }
    //   oo::define E { self unexport onlyinst }   ;# no error
    //   info class methods ::E  -> onlyinst       (still exported)
    //   [::E new] onlyinst      -> oi
    // so we record no visibility change and — crucially — no member stub.
    let src = "oo::class create ::E3 { method onlyinst {} { return oi } }\n\
                   oo::define ::E3 { self unexport onlyinst }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E3").expect("::E3 recorded");
    assert_eq!(c.methods["onlyinst"].visibility, "public");
    assert!(!c.class_methods.contains_key("onlyinst"));
}

#[test]
fn self_block_export_reexports_a_class_side_member() {
    // TP, the export half — last explicit state wins.
    // Oracle: `oo::class create H1 { self { method lower {} {…} ;
    // unexport lower ; export lower } }` -> `info object methods ::H1`
    // is `lower` on 9.0.4 and 8.6.14 alike.
    let src = "oo::class create ::H1 {\n\
                   self {\n\
                   method lower {} { return l }\n\
                   unexport lower\n\
                   export lower\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::H1").expect("::H1 recorded");
    assert_eq!(c.class_methods["lower"].visibility, "public");
}

#[test]
fn private_scoped_visibility_members_flip_the_instance_side() {
    // TP, the third routing. `private` is 9.0-only (`invalid command
    // name "private"` on 8.6), and under it a visibility word acts on the
    // instance side exactly as the unwrapped spelling does. Oracle:
    //   oo::class create G1 { method m {} {…}; private { unexport m } }
    //   info class methods ::G1              ->            (unexported)
    //   info class methods ::G1 -all -private -> … m …
    //   oo::class create G3 { method m {} {…}; private export m }
    //   info class methods ::G3              -> m          (exported)
    for (src, want) in [
        (
            "oo::class create ::G1 {\n\
                 method m {} { return m }\n\
                 private { unexport m }\n\
                 }",
            "unexported",
        ),
        (
            "oo::class create ::G2 {\n\
                 method m {} { return m }\n\
                 private unexport m\n\
                 }",
            "unexported",
        ),
        (
            "oo::class create ::G3 {\n\
                 method m {} { return m }\n\
                 private export m\n\
                 }",
            "public",
        ),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r
            .all_classes
            .values()
            .find(|c| c.methods.contains_key("m"))
            .expect("class recorded");
        assert_eq!(c.methods["m"].visibility, want, "{src}");
    }
}

#[test]
fn private_block_form_records_instance_methods() {
    // TP, symmetric half — `private` is the other registry member marked
    // wrapper-with-block-body.
    let src = "oo::class create ::P {\n\
                   private {\n\
                   method secret {k} { return $k }\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::P").expect("::P recorded");
    assert_eq!(c.methods["secret"].visibility, "private");
    assert!(!c.class_methods.contains_key("secret"));
}

#[test]
fn self_block_does_not_declare_instance_variables() {
    // TN / abstention boundary. `self { variable x }` declares a variable
    // on the *class object*, not an instance variable of the class — the
    // prefix form (`self variable x`) is not recorded either, and the
    // block form must not start recording it as one.
    let src = "oo::class create ::V {\n\
                   self {\n\
                   variable classlevel\n\
                   }\n\
                   variable instancelevel\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::V").expect("::V recorded");
    assert_eq!(c.variables, vec!["instancelevel".to_string()]);
}

#[test]
fn self_introspection_prefix_is_not_a_block_member() {
    // TN. Inside a *method body*, `self class` is an introspection call,
    // not a definer member. The body is walked as a script (never as a
    // definition body), so nothing here may reach `class_methods`.
    let src = "oo::class create ::W {\n\
                   method whoami {} { return [self class] }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::W").expect("::W recorded");
    assert!(c.methods.contains_key("whoami"));
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
}

#[test]
fn dynamic_self_block_abstains() {
    // Abstention boundary: a non-literal block word cannot be re-segmented
    // statically, so the expansion declines and the class records nothing
    // rather than guessing.
    let src = "set b {method make {n} { return $n }}\n\
                   oo::class create ::Dyn {\n\
                   self $b\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::Dyn").expect("::Dyn recorded");
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
    assert_eq!(c.methods.len(), 0, "{:?}", c.methods);
}

#[test]
fn self_block_destructive_member_drops_the_member_it_deletes() {
    // TP. Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C1 {
    //       self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
    //   }
    //   info object methods ::C1  ->  kept
    //   ::C1 gone                 ->  unknown method "gone"
    // Retaining `gone` would show a stale document symbol and let
    // navigation resolve a name the interpreter does not have.
    let src = "oo::class create ::C {\n                   self {\n                   method gone {} { return g }\n                   method kept {} { return k }\n                   deletemethod gone\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(
        !c.class_methods.contains_key("gone"),
        "{:?}",
        c.class_methods
    );
    assert!(c.class_methods.contains_key("kept"));
}

#[test]
fn self_block_renamemethod_moves_the_member_on_the_class_side() {
    // TP, the class-side half. Oracle, byte-identical on tclsh
    // 9.0.4 and 8.6.14:
    //   oo::class create ::R2 { self { method old {} {return CLSOLD}
    //                                  renamemethod old new } }
    //   info object methods ::R2  -> new
    //   ::R2 new                  -> CLSOLD   (shadowing the stock `new`!)
    //   ::R2 old                  -> unknown method
    // The move stays on the wrapper's own side: the instance table is not
    // touched in either direction.
    let src = "oo::class create ::C {\n                   self {\n                   method old {} { return o }\n                   renamemethod old new\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(
        !c.class_methods.contains_key("old"),
        "{:?}",
        c.class_methods
    );
    let md = c.class_methods.get("new").expect("`new` is a class member");
    assert_eq!(md.name, "new");
    assert!(src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),);
    assert_eq!(
        c.methods.len(),
        0,
        "instance side untouched: {:?}",
        c.methods
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn private_block_destructive_member_drops_the_instance_side_member() {
    // TP, symmetric half. Oracle (9.0 — `private` is a 9.0 member):
    //   oo::class create ::C5 {
    //       private { method secret {} {…} ; method other {} {…} ;
    //                 deletemethod secret }
    //   }
    //   info class methods ::C5 -all -private   ->  no `secret`
    let src = "oo::class create ::C {\n                   private {\n                   method secret {} { return s }\n                   method other {} { return o }\n                   deletemethod secret\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("secret"), "{:?}", c.methods);
    assert!(c.methods.contains_key("other"));
}

#[test]
fn wrapper_block_and_prefix_forms_retract_identically() {
    // The block form is normalised into the prefix form, so both spellings
    // must land on the same result. Oracle for the prefix side:
    //   oo::define ::C7 { self method sgone {} {…} }
    //   info object methods ::C7                  ->  sgone
    //   oo::define ::C7 { self deletemethod sgone }
    //   info object methods ::C7                  ->  (empty)
    let src = "oo::class create ::C {}\n                   oo::define ::C { self method gone {} { return g } }\n                   oo::define ::C { self deletemethod gone }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
}

#[test]
fn a_retraction_does_not_cross_to_the_other_side() {
    // TN. `self deletemethod` touches only the class-object side. Oracle:
    //   oo::class create ::D { method m {} {return inst}
    //                          self { method m {} {return cls} } }
    //   oo::define ::D { self deletemethod m }
    //   info class methods ::D   ->  m        (instance side survives)
    //   info object methods ::D  ->  (empty)
    //   [::D new] m              ->  inst
    let src = "oo::class create ::D {\n                   method m {} { return inst }\n                   self { method m {} { return cls } }\n                   }\n                   oo::define ::D { self deletemethod m }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::D").expect("::D recorded");
    assert!(c.methods.contains_key("m"), "instance side must survive");
    assert!(!c.class_methods.contains_key("m"), "class side must go");
}

#[test]
fn non_retracting_method_reference_members_keep_their_member() {
    // TN, the discrimination the registry flag exists for: `export`,
    // `unexport`, and `filter` are the sibling `MemberRefKind::Method`
    // members — they *name* a method without removing it, so they must not
    // retract. Oracle:
    //   oo::class create ::A { self { method a {} {…} ; unexport a ; export a } }
    //   info object methods ::A  ->  a   ;  ::A a  ->  1
    //   oo::class create ::B { self { method f {} {…} ; filter f } }
    //   info object methods ::B  ->  f
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::A {\n self {\n method a {} { return 1 }\n unexport a\n export a\n }\n}",
        "tcl9.0",
    );
    assert!(
        r.all_classes["::A"].class_methods.contains_key("a"),
        "export/unexport must not retract",
    );
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::B {\n self {\n method f {} { return 1 }\n filter f\n }\n}",
        "tcl9.0",
    );
    assert!(
        r.all_classes["::B"].class_methods.contains_key("f"),
        "filter must not retract",
    );
}

#[test]
fn a_retracted_member_can_be_redeclared_by_a_later_body() {
    // Source-order guard. Oracle:
    //   oo::class create ::C { method m {} {return 1} }
    //   oo::define ::C { self method m {} {return 2} }
    //   oo::define ::C { self deletemethod m }
    //   oo::define ::C { self method m {} {return 3} }
    //   ::C m        ->  3     (class side, redeclared after the delete)
    //   [::C new] m  ->  1     (instance side, never touched)
    // Retraction happens where the body is walked, so a later declaration
    // wins — it is not a whole-document erasure of the name.
    let src = "oo::class create ::C { method m {} { return 1 } }\n                   oo::define ::C { self method m {} { return 2 } }\n                   oo::define ::C { self deletemethod m }\n                   oo::define ::C { self method m {} { return 3 } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_methods.contains_key("m"), "redeclaration must win");
    assert!(c.methods.contains_key("m"), "instance side untouched");
}

#[test]
fn unwrapped_deletemethod_drops_the_instance_side_member() {
    // TP. An *unwrapped* `deletemethod` — in a class-creation
    // body or an `oo::define` body, with no `self` / `private` wrapper —
    // deletes on the instance side. Oracle, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::I1 { method gone {} {…}; method kept {} {…}
    //                           deletemethod gone }
    //   info class methods ::I1  ->  kept
    //   oo::class create ::I4 { method gone {} {…} }
    //   oo::define ::I4 { deletemethod gone }
    //   info class methods ::I4  ->            (empty)
    for src in [
        "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             deletemethod gone\n\
             }",
        "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             }\n\
             oo::define ::C { deletemethod gone }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("gone"), "deleted member retained");
        assert!(c.methods.contains_key("kept"), "sibling member dropped");
    }
}

#[test]
fn unwrapped_renamemethod_moves_the_member_to_the_new_name() {
    // TP. Oracle,
    // byte-identical on tclsh 9.0.4 and 8.6.14:
    //   oo::class create ::I3 { method old {} {return o}; renamemethod old new }
    //   info class methods ::I3        ->  new
    //   [::I3 new] new                 ->  o        (the old body runs)
    //   info class definition ::I3 new ->  {} { return o }
    //   ::I3 old                       ->  unknown method
    // so `new` is a fully navigable member carrying `old`'s body: `old` goes,
    // `new` arrives with the *same* `MethodDef`, its name span moved onto
    // the `renamemethod` call's destination word (the only place the new
    // name is written) and its body span left on the one real body.
    let mut a = Analyser::new();
    let src = "oo::class create ::C { method old {} { return o }\n\
             renamemethod old new }";
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("old"));
    let md = c.methods.get("new").expect("`new` is a member");
    assert_eq!(md.name, "new");
    // The name span is the `renamemethod`'s destination word …
    assert_eq!(
        &src[md.name_span.start() as usize..md.name_span.end() as usize],
        "new"
    );
    // … and it is the *second* `new` in the source (the one in the
    // `renamemethod` call), not any earlier text.
    assert!(md.name_span.start() as usize > src.find("renamemethod").expect("call"));
    // … while the body span still points at the original body.
    assert!(
        src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),
        "body span must stay on the original body",
    );
    // A rename is not a (re)declaration, so the family's name-based export
    // default is not re-applied — the source's visibility travels with it.
    assert_eq!(md.visibility, "public");
    // TN: a rename onto a fresh name is a legal order.
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    // …and the *destination* name leaves no cross-document tombstone: the
    // rename creates `new`, it does not delete it, so another document's
    // `method new` must survive. Registry data decides which arguments a
    // retracting word removes (`MemberRetraction::FirstArgument`).
    assert!(
        !c.retracted_members
            .iter()
            .any(|r| r.member == "new" || r.member == "old"),
        "a locally-declared retraction needs no tombstone: {:?}",
        c.retracted_members,
    );
}
