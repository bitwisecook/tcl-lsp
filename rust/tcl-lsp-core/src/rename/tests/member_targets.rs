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

//! Which member the cursor selects and with what access: receiver sides, class-body fallback and workspace callbacks.

use super::*;

#[test]
fn workspace_resolves_inherited_namespace_wrapped_callback_with_internal_access() {
    use crate::workspace_index::{MethodAccess, WorkspaceIndex};

    let base_src = "oo::class create Base {\n    method read {} {}\n    unexport read\n}\n";
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
    let base = analyse(base_src);
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    // The resolved target is the **receiver**, exactly as a bare `$obj
    // method` position resolves to one: the provider is whatever the
    // workspace chain rooted there reaches, and re-rooting the chain at
    // the provider instead would drop the receiver's own `export` /
    // `unexport` stub on the way.
    assert_eq!(
        method_target_with_access_in_workspace(
            child_src,
            3,
            60,
            &child,
            Some((&index, "file:///child.tcl")),
        ),
        Some((
            "::Child".to_owned(),
            "read".to_owned(),
            false,
            MethodAccess::Internal,
        )),
    );
    // …and that chain still names `Base` as the provider, so the
    // definition / lens / rename tiers reach the inherited declaration.
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "read", MethodAccess::Internal)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///base.tcl"),
    );
}

#[test]
fn workspace_rejects_inherited_namespace_wrapped_external_private_callback() {
    use crate::workspace_index::WorkspaceIndex;

    let base_src = "oo::class create Base {\n    method read {} {}\n    unexport read\n}\n";
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list [self] read]]\n    }\n}\n";
    let base = analyse(base_src);
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    assert!(
        method_target_with_access_in_workspace(
            child_src,
            3,
            64,
            &child,
            Some((&index, "file:///child.tcl")),
        )
        .is_none()
    );
}

#[test]
fn workspace_callback_dispatch_obeys_mixin_mro_over_child_override() {
    use crate::workspace_index::{MethodAccess, WorkspaceIndex};

    let base = analyse("oo::class create Base { method read {} {} }\n");
    let mixin = analyse("oo::class create Mixin { method read {} {} }\n");
    let child_src = "oo::class create Child {\n    superclass Base\n    mixin Mixin\n    method read {} {}\n    method wire {chan} { fileevent $chan readable [namespace code [list my read]] }\n}\n";
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///mixin.tcl", &mixin),
        ("file:///child.tcl", &child),
    ]);
    assert_eq!(
        method_target_with_access_in_workspace(
            child_src,
            4,
            75,
            &child,
            Some((&index, "file:///child.tcl")),
        ),
        Some((
            "::Child".to_owned(),
            "read".to_owned(),
            false,
            MethodAccess::Internal,
        )),
    );
    // The MRO fact this test exists for, pinned where it is decided: a
    // `mixin` is searched ahead of the class's own table, so the chain
    // rooted at the receiver enters `Mixin::read`, not `Child`'s override.
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "read", MethodAccess::Internal)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///mixin.tcl"),
    );
}

// `method_target_with_access`'s receiver-side split.

/// TP: a bare `ClassName member` naming a **class-side** member resolves as
/// a classmethod, so downstream cross-file resolution reads the
/// class-object side's tables.
///
/// `receiver_instance_class`'s last resort is "this bare word names a
/// class", so without the class-side gate ahead of it every bare
/// `ClassName member` classifies instance-side and no class-side member
/// can reach the cross-file class-command dispatch, whatever the
/// workspace index holds.
#[test]
fn method_target_classifies_a_bare_class_receiver_as_a_classmethod() {
    let src =
        "oo::class create Widget {\n    self { method make {} { return 1 } }\n}\nWidget make\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    assert_eq!(
        method_target_with_access(src, 3, 8, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            true,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN: an `$obj member` receiver is an object handle, never a class, so it
/// stays instance-side — the gate that keeps the reordering above confined
/// to the shape it is about.
#[test]
fn method_target_keeps_an_object_handle_receiver_instance_side() {
    let src = "oo::class create Widget {\n    method make {} { return 1 }\n}\nset w [Widget new]\n$w make\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 4, 4, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            false,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN: a bare class receiver whose class-object side has **no** such member
/// falls through to the instance branch unchanged.
#[test]
fn method_target_falls_through_when_the_class_side_lacks_the_member() {
    let src = "oo::class create Widget {\n    method make {} { return 1 }\n}\nWidget make\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 3, 8, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            false,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN (CRITICAL): a stock `self method` is **not inherited** — `Gadget
/// make` against a parent's `self method make` errors `unknown method
/// "make": must be create, destroy or new` on tclsh 8.6 and 9.0.4 alike —
/// so the subclass receiver must not resolve to the parent's class-side
/// member. `classmethod_dispatch_class` applies the same
/// `is_self_method` rule the in-document provider does.
#[test]
fn method_target_does_not_inherit_a_parents_self_method() {
    let src = concat!(
        "oo::class create Widget {\n",
        "    self { method make {} { return 1 } }\n",
        "}\n",
        "oo::class create Gadget {\n",
        "    superclass Widget\n",
        "}\n",
        "Gadget make\n",
    );
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let target = method_target_with_access(src, 6, 8, &analysis);
    assert!(
        target.is_none_or(|(_, _, is_classmethod, _)| !is_classmethod),
        "an inherited `self method` is not the subclass's classmethod",
    );
}

// `method_target_with_access`'s class-body fallback — the idx-113 link
// gate.

/// TP: the cursor on a method's own declaration name is the member.
#[test]
fn method_target_fires_on_the_members_own_declaration_name() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = analyse(src);
    let target = method_target_with_access(src, 1, 11, &analysis);
    assert_eq!(
        target,
        Some((
            "::Widget".to_owned(),
            "foo".to_owned(),
            false,
            crate::workspace_index::MethodAccess::Internal
        )),
        "cursor on the `foo` declaration must resolve to Widget's `foo`"
    );
}

/// TP: a bareword sibling call that `link` genuinely made callable is a
/// reference to the member — tclsh9.0-verified (`link foo` in the
/// constructor makes `[foo 42]` inside another method dispatch to
/// `Widget`'s `foo`; `link` does not exist in 8.6's `TclOO`).
#[test]
fn method_target_fires_on_a_linked_bareword_call() {
    let src = "oo::class create Widget {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let target = method_target_with_access(src, 3, 28, &analysis);
    assert_eq!(
        target,
        Some((
            "::Widget".to_owned(),
            "foo".to_owned(),
            false,
            crate::workspace_index::MethodAccess::Internal
        )),
        "a linked bareword sibling call must resolve to the member"
    );
}

/// FP guard: an **un-linked** bareword sibling call names
/// nothing — real Tcl raises `invalid command name "foo"` there
/// (tclsh9.0-verified) — so the resolver must abstain rather than hand
/// the word to the workspace resolver, which answers it
/// non-deterministically.
#[test]
fn method_target_abstains_on_an_unlinked_bareword_sibling_call() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 2, 28, &analysis),
        None,
        "an un-linked bareword sibling call is not a reference to the method"
    );
}

/// FP guard: a *parameter* that happens to share a sibling method's name
/// is not that method either — the old fallback fired on any
/// member-name-shaped word anywhere in the class body span.
#[test]
fn method_target_abstains_on_a_parameter_sharing_a_method_name() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {foo} { return $foo }\n}\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 2, 16, &analysis),
        None,
        "a parameter word must not resolve to the same-named method"
    );
}

/// The resolution the gate piggy-backs on answers for exactly the
/// positions the untargeted tier claims, and abstains elsewhere — so
/// broadening the gate cannot start refusing renames an earlier tier owns.
#[test]
fn untargeted_member_target_answers_only_inside_the_class_body() {
    let analysis = analyse(IDX79_HAZARD);
    let line_index = LineIndex::new(IDX79_HAZARD);
    let at =
        |line: u32, ch: u32| crate::definition::byte_offset_at(&line_index, IDX79_HAZARD, line, ch);
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", at(8, 11)),
        Some(("::Vector3d".to_owned(), "X".to_owned(), false)),
        "the export bareword names the member"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", at(4, 23)),
        Some(("::Vector3d".to_owned(), "X".to_owned(), false)),
        "the untracked call site's method word names the member"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "other", at(4, 18)),
        None,
        "a word that names no member of the enclosing class must abstain"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", 0),
        None,
        "outside any class body there is no member to resolve"
    );
}
