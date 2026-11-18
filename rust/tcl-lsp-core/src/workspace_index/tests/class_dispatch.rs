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

//! The class graph across documents: visibility along dispatch chains, retractions, arrivals, member enumeration and override families.

use super::*;

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
    let a = analyse_tcl("oo::class create ::C { method old {} { return 1 } }\n");
    let b = analyse_tcl("oo::define ::C { renamemethod old new }\n");
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
    let class = analyse_tcl("oo::class create ::C { method old {} {} }\n");
    let orphan = analyse_tcl("oo::define ::C { renamemethod old orphan_arrival }\n");
    let live = analyse_tcl("oo::define ::C { renamemethod old live_arrival }\n");
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
    let a = analyse_tcl("oo::class create ::C { method m {} { return 1 } }\n");
    let b = analyse_tcl("oo::define ::C { deletemethod m }\n");
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
        "oo::class create ::A::Base {}\nnamespace eval ::A {oo::class create Derived {\n    superclass Base\n}}\n",
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

/// The lenient `tcl` environment, where indexed documents are loaded.
fn analyse_tcl(source: &str) -> AnalysisResult {
    analyse_as(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
    )
}
