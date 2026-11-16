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

//! Sided visibility sets and filters, per-object visibility and unwrapped retractions.

use super::*;

// The class-side visibility + filter channels.

#[test]
fn self_scoped_visibility_words_fill_the_class_side_sets() {
    // TP. A `self export` / `self unexport` has to leave a
    // record of its own or the flip never travels to another file's
    // class-command dispatch. Oracle, byte-identical on tclsh 9.0.4/8.6.14:
    //   oo::class create ::X { self { method cm {} { return cm } } }
    //   ::X cm                    ;# -> cm
    //   oo::define X { self unexport cm }
    //   ::X cm                    ;# -> unknown method "cm": must be create,
    //                             ;#    destroy or new
    //   info object methods ::X   ;# -> (empty)
    //   info object methods ::X -all -private ;# -> … cm …  (still defined)
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { method cm {} { return c } } }\n\
             oo::define ::C { self unexport cm }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_unexports.contains("cm"), "{:?}", c.class_unexports);
    assert_eq!(c.class_exports.len(), 0, "{:?}", c.class_exports);
    assert_eq!(c.class_methods["cm"].visibility, "unexported");
    // …and the instance-side pair — the one every existing consumer reads
    // as the instance record — stays untouched in both directions.
    assert_eq!(c.exports.len(), 0, "{:?}", c.exports);
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
}

#[test]
fn a_rename_carries_the_sources_visibility_not_the_new_names_default() {
    // CRITICAL pin. A rename is not a (re)declaration, so the
    // family's name-based export default (`[a-z]*` is exported for `TclOO`)
    // is NOT re-applied to the destination — the source's own visibility
    // travels with the body. Both directions, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::R4 { method Priv {} {…} ; renamemethod Priv pub }
    //   info class methods ::R4           ;# -> (empty)   `pub` is UNexported
    //   info class methods ::R4 -private  ;# -> pub       despite the name
    //   [::R4 new] pub                    ;# -> unknown method "pub"
    //   oo::class create ::R5 { method low {} {…} ; renamemethod low Up }
    //   info class methods ::R5           ;# -> Up        still EXPORTED
    //   info class methods ::R5 -private  ;# -> Up        despite the name
    //
    // Applying the destination's default instead would flip both of these
    // the wrong way — offering a `pub` no `$obj pub` can call, and hiding an
    // `Up` that dispatches fine.
    for (src_name, dst_name, want) in [("Priv", "pub", "unexported"), ("low", "Up", "public")] {
        let src = format!(
            "oo::class create ::C {{ method {src_name} {{}} {{ return 1 }}\n\
                 renamemethod {src_name} {dst_name} }}"
        );
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(
            c.methods[dst_name].visibility, want,
            "{src_name} -> {dst_name}",
        );
    }
}

#[test]
fn the_class_side_visibility_sets_keep_only_the_last_writer() {
    // The class-side pair takes the identical last-writer-exclusive rule as
    // the instance pair, because the workspace consumer reads any `exports`
    // entry as decisive. Oracle: `oo::class create ::H1 { self { method
    // lower {} {…} ; unexport lower ; export lower } }` leaves
    // `info object methods ::H1` -> lower on 9.0.4 and 8.6.14 alike.
    for (body, exported) in [
        ("unexport m\nexport m", true),
        ("export m\nunexport m", false),
    ] {
        let src =
            format!("oo::class create ::C {{ self {{ method m {{}} {{ return 1 }}\n{body} }} }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        let (winner, loser) = if exported {
            (&c.class_exports, &c.class_unexports)
        } else {
            (&c.class_unexports, &c.class_exports)
        };
        assert!(winner.contains("m"), "{body}");
        assert!(
            !loser.contains("m"),
            "{body}: superseded set still holds `m`"
        );
    }
}

#[test]
fn an_unwrapped_visibility_word_never_touches_the_class_side_sets() {
    // TN, the mirror of `a_self_scoped_visibility_word_never_touches_the_
    // instance_sets`. The unwrapped spelling is instance-scoped, so an
    // identically-named class-side member keeps its own state — oracle:
    // `oo::class create E2 { self { method onlyclass {} {…} } }` then
    // `oo::define E2 { unexport onlyclass }` leaves `onlyclass` exported and
    // dispatchable via `::E2 onlyclass` on 9.0.4 and 8.6.14.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { method m {} { return c } } }\n\
             oo::define ::C { unexport m }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_exports.len(), 0, "{:?}", c.class_exports);
    assert_eq!(c.class_unexports.len(), 0, "{:?}", c.class_unexports);
    assert_eq!(c.class_methods["m"].visibility, "public");
    // The unwrapped word still records its own (instance-side) intent.
    assert!(c.unexports.contains("m"));
}

#[test]
fn filters_are_sided_by_the_wrapper_they_are_written_under() {
    // TP + TN — `filters` is sided like every other member table. The two
    // slots are genuinely independent in `TclOO` and
    // intercept different dispatches. Oracle, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::A { method real {} {…} ; method logit {args} {…}
    //                          filter logit }
    //   info class filters ::A   ;# -> logit      info object filters ::A -> {}
    //   [::A new] real           ;# logit fires, `self target` -> `::A real`
    //   oo::class create ::B { method inst {} {…}
    //       self { method cls {} {…} ; method logit {args} {…} ; filter logit } }
    //   info object filters ::B  ;# -> logit      info class filters ::B  -> {}
    //   ::B cls                  ;# logit fires, `self target` -> `::B cls`
    //   ::B new                  ;# logit fires too — the constructor path
    //   [::B new] inst           ;# logit does NOT fire
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::A { method real {} { return r }\n\
             method logit {args} { next }\n\
             filter logit }\n\
             oo::class create ::B { method inst {} { return i }\n\
             self { method cls {} { return c }\n\
             method logit {args} { next }\n\
             filter logit } }",
        "tcl9.0",
    );
    let a_cls = r.all_classes.get("::A").expect("::A recorded");
    assert_eq!(a_cls.filters, vec!["logit"]);
    assert_eq!(a_cls.class_filters.len(), 0, "{:?}", a_cls.class_filters);
    let b_cls = r.all_classes.get("::B").expect("::B recorded");
    assert_eq!(b_cls.class_filters, vec!["logit"]);
    assert_eq!(b_cls.filters.len(), 0, "{:?}", b_cls.filters);
}

#[test]
fn a_private_scoped_filter_is_instance_side() {
    // TN for the third spelling: `private` is a *visibility* wrapper over
    // the instance side, not a third side, so its `filter` lands in the
    // instance slot. Oracle (9.0 only — `private` does not exist on 8.6,
    // where the same body fails with `invalid command name "private"`):
    //   oo::class create ::P { private { method s {} {} ; filter s } }
    //   info class filters ::P   ;# -> s
    //   info object filters ::P  ;# -> (empty)
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { private { method s {} { return s }\n\
             filter s } }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.filters, vec!["s"]);
    assert_eq!(c.class_filters.len(), 0, "{:?}", c.class_filters);
}

#[test]
fn per_object_visibility_lands_in_object_member_state() {
    // `oo::objdefine $o { unexport m }` really works — oracle, 9.0.4 and
    // 8.6.14 alike:
    //   oo::class create ::C { method m {} {…} } ; set o [::C new]
    //   oo::objdefine $o { unexport m } ; $o m
    //   ;# -> unknown method "m": must be destroy or n
    // The flip lands in the receiver binding's `ObjectMemberState`
    // — the durable per-object home — while the class itself stays
    // untouched, and the throwaway holder never reaches `all_classes`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.methods["m"].visibility, "public");
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
    assert!(!r.all_classes.contains_key("::@objdefine@::o"));
    let states = r
        .object_member_state
        .get("o")
        .expect("the receiver binding has member state");
    assert_eq!(states.len(), 1, "{states:?}");
    assert!(states[0].unexports.contains("m"), "{states:?}");
    assert_eq!(states[0].exports.len(), 0, "{states:?}");
}

#[test]
fn per_object_export_and_unexport_are_last_writer_exclusive() {
    // The per-object pair keeps the same last-writer-exclusive contract
    // as the class-side pairs: a later `export m` cancels the earlier
    // flip, across blocks of the same binding.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }\n\
             oo::objdefine $o { export m }",
        "tcl9.0",
    );
    let states = r.object_member_state.get("o").expect("state recorded");
    assert_eq!(states.len(), 1, "one binding: {states:?}");
    assert!(states[0].exports.contains("m"), "{states:?}");
    assert!(!states[0].unexports.contains("m"), "{states:?}");
}

#[test]
fn per_object_cross_block_retraction_folds_and_stays_silent() {
    // The cross-block hazard that kept W315 out of `oo::objdefine`:
    // a second block retracting what the first declared
    // is legal (tclsh 9.0.4 / 8.6.14 both accept it), so the seeded walk
    // must remove the member silently — no W315 — and the folded state
    // must drop it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method im {} { return i } }\n\
             oo::objdefine $o { deletemethod im }",
        "tcl9.0",
    );
    assert_eq!(w315_messages(&r).len(), 0, "{:?}", r.diagnostics);
    let states = r.object_member_state.get("o").expect("state recorded");
    assert_eq!(states.len(), 1, "{states:?}");
    assert_eq!(
        states[0].methods.len(),
        0,
        "the retraction must fold: {states:?}"
    );
}

#[test]
fn a_retraction_of_a_locally_declared_member_leaves_no_tombstone() {
    // TN for the cross-document channel. Within one document the member is
    // removed outright and the order is known, so exporting a tombstone
    // would wrongly cancel a *different* document's redeclaration of the
    // name — no ordering evidence supports that.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method gone {} { return g } }\n\
             oo::define ::C { deletemethod gone }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("gone"));
    assert_eq!(c.retracted_members.len(), 0, "{:?}", c.retracted_members);
}

#[test]
fn visibility_sets_keep_only_the_last_writer() {
    // `exports` / `unexports` each record the
    // *last* explicit writer for a name, and the cross-file consumer
    // (`workspace_index::method_dispatch_chain`) reads any `exports` entry
    // as decisive — so a name must never sit in both. Oracle, identical on
    // tclsh 9.0.4 and 8.6.14:
    //   oo::class create L1 { method m {} {…}; export m; unexport m }
    //   info class methods ::L1  ->            ;  [L1 new] m -> unknown method
    //   oo::class create L2 { method m {} {…}; unexport m; export m }
    //   info class methods ::L2  -> m          ;  [L2 new] m -> 1
    // Both unwrapped and `private`-scoped writers must leave each name
    // in exactly the set selected by its last explicit visibility operation.
    for (body, last, want_vis) in [
        ("export m\nunexport m", "unexports", "unexported"),
        ("unexport m\nexport m", "exports", "public"),
        (
            "private export m\nprivate unexport m",
            "unexports",
            "unexported",
        ),
        ("private unexport m\nprivate export m", "exports", "public"),
        (
            "private { export m }\nprivate { unexport m }",
            "unexports",
            "unexported",
        ),
    ] {
        let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        let (winner, loser) = if last == "exports" {
            (&c.exports, &c.unexports)
        } else {
            (&c.unexports, &c.exports)
        };
        assert!(winner.contains("m"), "{body}: {last} must hold `m`");
        assert!(
            !loser.contains("m"),
            "{body}: the superseded set still holds `m` — cross-file dispatch \
                 reads any `exports` entry as decisive",
        );
        assert_eq!(c.methods["m"].visibility, want_vis, "{body}");
    }
}

#[test]
fn a_self_scoped_visibility_word_never_touches_the_instance_sets() {
    // TN for the same sets: they are the *instance*-side record, so a
    // class-object-side flip must stay out of them entirely (in either
    // direction) — otherwise it would re-state an unrelated instance
    // method's export bit cross-file.
    let src = "oo::class create ::C {\n\
                   method m {} { return 1 }\n\
                   self { method m {} { return 2 }\n\
                   unexport m\n\
                   export m\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.exports.len(), 0, "{:?}", c.exports);
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
    assert_eq!(c.methods["m"].visibility, "public");
    assert_eq!(c.class_methods["m"].visibility, "public");
}

#[test]
fn unwrapped_deletemethod_does_not_touch_the_class_side() {
    // TN. The unwrapped word is instance-scoped; a class-object-side member
    // of the same name survives it. (Real Tcl goes further and makes the
    // cross-side form a hard definition-aborting error — `oo::define ::I5
    // { deletemethod cm }` over a `self`-only `cm` raises `method cm does
    // not exist` and leaves `info object methods ::I5` -> cm — so a
    // document that reaches this shape never ran; keeping the class-side
    // entry is the abstaining answer either way.)
    let src = "oo::class create ::C {\n\
                   self { method cm {} { return c } }\n\
                   }\n\
                   oo::define ::C { deletemethod cm }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_methods.contains_key("cm"));
}

#[test]
fn unwrapped_non_destructive_members_do_not_retract() {
    // TN for the registry flag: `export` / `unexport` / `filter` name a
    // method without removing it, so the unwrapped arm must leave them be.
    for member in ["export", "unexport", "filter"] {
        let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{member} m }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        assert!(
            r.all_classes["::C"].methods.contains_key("m"),
            "`{member}` must not retract",
        );
    }
}

#[test]
fn source_configuration_accumulates_redeclared_members_under_original_class_owner() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Conditional source chronology and canonical metadata, without an
    // executed configuration worker or a live Native method table.
    for source in [
        "oo::class create C {method m {} {return FIRST}}; oo::define C {deletemethod m}; oo::define C {method m {} {return LAST}}",
        "oo::class create C {method m {} {return FIRST}}; rename C Held; interp alias {} configure {} oo::define Held; configure {deletemethod m}; configure {method m {} {return LAST}}",
    ] {
        let mut result = Analyser::new().analyse(source, "tcl9.0");
        let declaration = result.original_class_declarations().next().unwrap();
        assert!(declaration.metadata().methods.contains_key("m"));
        let last = source.find("return LAST").unwrap();
        assert!(
            declaration.metadata().methods["m"]
                .body_span
                .as_range()
                .contains(&last)
        );
        let methods = declaration
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        let method = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"m")
            .unwrap();
        assert!(method.metadata().body_span.as_range().contains(&last));
        let canonical = declaration.clone();
        result.all_classes.clear();
        assert_eq!(
            result.original_class_declarations().next(),
            Some(&canonical)
        );
    }
    let source = "oo::class create C {}; rename C {}; oo::define C {method ghost {} {}}";
    let result = Analyser::new().analyse(source, "tcl9.0");
    assert_eq!(result.original_class_declarations().count(), 1);
    assert!(result.original_class_declarations().all(|declaration| {
        !declaration.metadata().methods.contains_key("ghost")
            && declaration
                .metadata()
                .original_members
                .declarations()
                .all(|method| method.original_name_input().bytes() != b"ghost")
    }));
}

#[test]
fn an_unwrapped_retracted_member_can_be_redeclared_by_a_later_body() {
    // Source-order guard, unwrapped half. Retraction happens where the body
    // is walked, so a later `oo::define` redeclaration wins rather than the
    // name being erased document-wide.
    let src = "oo::class create ::C { method m {} { return 1 } }\n\
                   oo::define ::C { deletemethod m }\n\
                   oo::define ::C { method m {} { return 2 } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(r.all_classes["::C"].methods.contains_key("m"));
}

#[test]
fn objdefine_deletemethod_drops_the_per_object_member() {
    // The per-object table is the same instance-side routing. Oracle:
    //   oo::objdefine $o { method im {} {…}; deletemethod im }
    //   info object methods $o  ->            (empty)
    // `oo::objdefine` records into a throwaway `ClassDef` that never
    // reaches `all_classes`, so this asserts the walk simply does not
    // panic or resurrect the name anywhere.
    let src = "oo::class create ::C { method m {} { return m } }\n\
                   set o [::C new]\n\
                   oo::objdefine $o { method im {} { return i }\n\
                   deletemethod im }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(!r.all_classes.contains_key("::@objdefine@::o"));
    assert!(r.all_classes["::C"].methods.contains_key("m"));
}
