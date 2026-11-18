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

//! Callback captures in methods nobody calls. Each fixture's class set is
//! closed (no further subclass, mixin, filter, per-object method, computed
//! name or `unknown` method), so `[self] X` / `my X` reaches the same target
//! from every entry and the generic entry of the uncalled method must answer.
//! The called twins live in `tcloo_navigation.rs`.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use crate::tcloo_navigation::{
    edit_lines, expected_title, lens_locations, location_lines, resolve_lens_on_line,
};

/// A Tk binding built with `[list [self] METHOD ...]` captures
/// the current object command and later invokes the exported method.  The
/// declaration's references, code lens, and rename must agree on that method
/// word even though it is data inside the prefix-building `list` call.
#[test]
fn tp_list_built_self_bind_callback_is_a_method_reference_everywhere_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require Tk\n\
         oo::class create Test {\n\
             method animTick {x y} { return {} }\n\
             method anim {wl} {\n\
                 bind $wl <ButtonPress-1> [list [self] animTick %x %y]\n\
             }\n\
         }\n",
    );

    let refs = lsp.references(&uri, 2, 11, false);
    assert_eq!(
        location_lines(&refs, &uri),
        std::collections::BTreeSet::from([4]),
        "Find All References must return the callback method word: {refs:?}"
    );

    let command = resolve_lens_on_line(&mut lsp, &uri, 2);
    assert_eq!(command["title"], expected_title(1), "{command:?}");
    assert_eq!(
        location_lines(&lens_locations(&command), &uri),
        std::collections::BTreeSet::from([4]),
        "the lens must open the same callback site: {command:?}"
    );

    let definition = lsp.definition(&uri, 4, 44);
    assert_eq!(
        location_lines(&definition, &uri),
        std::collections::BTreeSet::from([2]),
        "go-to-definition from the callback word must reach the method: {definition:?}"
    );
    let callback_refs = lsp.references(&uri, 4, 44, false);
    assert_eq!(
        location_lines(&callback_refs, &uri),
        std::collections::BTreeSet::from([4]),
        "references started on the callback must agree with the declaration: {callback_refs:?}"
    );

    let prepare = lsp.prepare_rename(&uri, 4, 44);
    assert_eq!(prepare["placeholder"], "animTick", "{prepare:?}");
    assert_eq!(prepare["range"]["start"]["line"], 4, "{prepare:?}");

    let edits = rename_edits(&lsp.rename(&uri, 2, 11, "frameTick"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![2, 4],
        "rename must update the declaration and callback word: {edits:?}"
    );
    let callback_edits = rename_edits(&lsp.rename(&uri, 4, 44, "frameTick"));
    assert_eq!(
        edit_lines(&callback_edits, &uri),
        vec![2, 4],
        "rename from the callback word must update the same family: {callback_edits:?}"
    );
}

/// A captured `[self]` object command in an inheriting class reaches
/// the provider's exported implementation — tclsh 8.6.16 / 9.0.4 both run
/// `Base`'s body for `[list [Child new] tick]` — so every provider must place
/// the occurrence in `Base::tick`'s family, from either direction.
#[test]
fn tp_inherited_list_built_self_callback_joins_the_provider_family_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Base {\n\
             method tick {} { return 1 }\n\
         }\n\
         oo::class create Child {\n\
             superclass Base\n\
             method wire {} {\n\
                 after idle [list [self] tick]\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(6).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    let from_declaration = lsp.references(&uri, 1, 11, false);
    assert!(
        location_lines(&from_declaration, &uri).contains(&6),
        "the inherited callback is a call site of the provider: {from_declaration:?}"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 6, callback_char), &uri),
        std::collections::BTreeSet::from([1]),
        "go-to-definition from the callback word must reach the provider"
    );
    let from_callback = lsp.references(&uri, 6, callback_char, false);
    assert_eq!(
        location_lines(&from_callback, &uri),
        location_lines(&from_declaration, &uri),
        "references started on the callback must agree with the declaration"
    );
    let command = resolve_lens_on_line(&mut lsp, &uri, 1);
    assert!(
        location_lines(&lens_locations(&command), &uri).contains(&6),
        "the lens must count and open the inherited callback: {command:?}"
    );
    assert_eq!(
        lsp.prepare_rename(&uri, 6, callback_char)["placeholder"],
        "tick"
    );
    let edits = rename_edits(&lsp.rename(&uri, 6, callback_char, "tock"));
    assert!(
        edit_lines(&edits, &uri).contains(&1) && edit_lines(&edits, &uri).contains(&6),
        "rename must edit the declaration and the callback together: {edits:?}"
    );
}

/// An override answers the capture, so the base's family must not
/// claim it — renaming `Base::tick` would otherwise rewrite a word that never
/// called it.
#[test]
fn tp_overriding_receiver_keeps_its_callback_in_its_own_family_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Base {\n\
             method tick {} { return 1 }\n\
         }\n\
         oo::class create Child {\n\
             superclass Base\n\
             method tick {} { return 2 }\n\
             method wire {} {\n\
                 after idle [list [self] tick]\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(7).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    assert!(
        !location_lines(&lsp.references(&uri, 1, 11, false), &uri).contains(&7),
        "the base declaration must not collect the override's capture"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 7, callback_char), &uri),
        std::collections::BTreeSet::from([5]),
        "the capture resolves to the override"
    );
    assert!(
        location_lines(&lsp.references(&uri, 5, 11, false), &uri).contains(&7),
        "the override's own family collects it"
    );
}

/// `Base` and `Child` in separate documents.  The workspace tier must
/// answer with the same effective provider the single-document tier does, in
/// references, lens, rename and call hierarchy alike.
#[test]
fn tp_cross_file_inherited_self_callback_agrees_everywhere_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method tick {} { return 1 }\n}\n",
    );
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let callback_char =
        u32::try_from(child_src.lines().nth(3).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&child, child_src);

    assert_eq!(
        location_lines(&lsp.definition(&child, 3, callback_char), &base),
        std::collections::BTreeSet::from([1]),
        "the cross-file provider answers go-to-definition"
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&3),
        "cross-file callback missing from references: {refs:?}"
    );
    let lens = resolve_lens_on_line(&mut lsp, &base, 1);
    assert!(
        location_lines(&lens_locations(&lens), &child).contains(&3),
        "cross-file callback missing from lens: {lens:?}"
    );
    let edits = rename_edits(&lsp.rename(&child, 3, callback_char, "tock"));
    assert!(edit_lines(&edits, &base).contains(&1), "{edits:?}");
    assert!(edit_lines(&edits, &child).contains(&3), "{edits:?}");

    let item = lsp.prepare_call_hierarchy(&base, 1, 11)[0].clone();
    let incoming = lsp.incoming_calls(item);
    assert!(
        incoming.to_string().contains("wire"),
        "cross-file callback missing from hierarchy: {incoming:?}"
    );
}

/// The mirror of the suppression case, and the one a provider-only
/// reading gets wrong in the other direction.  `Base` unexports its own
/// `tock`, the sibling document's `Child` exports the inherited name, and
/// tclsh 8.6.16 / 9.0.4 run `Base`'s body for `[Child new] tock`.
#[test]
fn tp_cross_file_receiver_reexport_revives_the_inherited_callback_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method tock {} { return 1 }\n    unexport tock\n}\n",
    );
    let child_src = "oo::class create Child {\n    superclass Base\n    export tock\n    method wire {} {\n        after idle [list [self] tock]\n    }\n}\n";
    let callback_char =
        u32::try_from(child_src.lines().nth(4).unwrap().rfind("tock").unwrap()).unwrap();
    lsp.open_ready(&child, child_src);

    assert_eq!(
        location_lines(&lsp.definition(&child, 4, callback_char), &base),
        std::collections::BTreeSet::from([1]),
        "a re-exporting receiver revives the inherited callback target"
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&4),
        "the revived cross-file callback is a reference: {refs:?}"
    );
}

/// The receiver class is local but its inherited private provider is
/// in a sibling file.  The typed `my` callback must seed the workspace MRO,
/// so definition, references, lens, rename and hierarchy agree on `Base::read`.
#[test]
fn tp_cross_file_inherited_wrapped_my_callback_agrees_everywhere_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method read {} { return {} }\n    unexport read\n}\n",
    );
    lsp.open_ready(
        &child,
        "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n",
    );

    assert_eq!(
        location_lines(&lsp.definition(&child, 3, 60), &base),
        std::collections::BTreeSet::from([1])
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&3),
        "cross-file callback missing from references: {refs:?}"
    );
    let lens = resolve_lens_on_line(&mut lsp, &base, 1);
    assert!(
        location_lines(&lens_locations(&lens), &child).contains(&3),
        "cross-file callback missing from lens: {lens:?}"
    );
    let edits = rename_edits(&lsp.rename(&child, 3, 60, "consume"));
    assert!(edit_lines(&edits, &base).contains(&1), "{edits:?}");
    assert!(edit_lines(&edits, &child).contains(&3), "{edits:?}");

    let item = lsp.prepare_call_hierarchy(&base, 1, 11)[0].clone();
    let incoming = lsp.incoming_calls(item);
    assert!(
        incoming.to_string().contains("wire"),
        "cross-file callback missing from hierarchy: {incoming:?}"
    );
}

/// A tcllib-shaped namespace capture may wrap a command-prefix builder.
/// `my` retains current-object/private dispatch, so all navigation providers
/// must agree even when the target was explicitly unexported.
#[test]
fn tp_namespace_wrapped_my_callback_agrees_across_navigation_providers_in_an_uncalled_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\n\
         oo::class create C {\n\
             method read {} { return {} }\n\
             unexport read\n\
             method wire {chan} {\n\
                 fileevent $chan readable [namespace code [list my read]]\n\
             }\n\
         }\n",
    );

    let refs = lsp.references(&uri, 2, 11, false);
    assert_eq!(
        location_lines(&refs, &uri),
        std::collections::BTreeSet::from([3, 5])
    );
    let command = resolve_lens_on_line(&mut lsp, &uri, 2);
    assert_eq!(command["title"], expected_title(2), "{command:?}");
    assert_eq!(
        location_lines(&lens_locations(&command), &uri),
        std::collections::BTreeSet::from([3, 5]),
        "{command:?}"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 5, 51), &uri),
        std::collections::BTreeSet::from([2])
    );
    assert_eq!(lsp.prepare_rename(&uri, 5, 51)["placeholder"], "read");
    assert_eq!(
        edit_lines(&rename_edits(&lsp.rename(&uri, 5, 51, "consume")), &uri),
        vec![2, 5, 3]
    );
}
