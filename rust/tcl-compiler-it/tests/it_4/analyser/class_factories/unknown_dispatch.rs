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

//! Factories proved through an `unknown` dispatch that constructs and returns the word.

use super::*;

/// The Tk `library/megawidget.tcl` unknown-dispatch constructor, reduced
/// to its mechanism: an unrecognised first word is a widget path, so the
/// metaclass constructs the object and hands the path straight back.
///
/// tclsh 9.0.4 confirms the return value really is the word — with the
/// construction made globally (`[self] create ::$w`, which is what makes
/// the created command reachable from the caller's namespace rather than
/// the method's own), `set w [::D::Widget .w]; $w go` answers `went`.
const UNKNOWN_CTOR: &str = concat!(
    "oo::class create Meta {\n",
    "    superclass oo::class\n",
    "    method unknown {w args} {\n",
    "        if {[string match .* $w]} {\n",
    "            [self] create ::$w {*}$args\n",
    "            return $w\n",
    "        }\n",
    "        next $w {*}$args\n",
    "    }\n",
    "    unexport new unknown\n",
    "}\n",
);

fn factory_of(src: &str, qn: &str) -> tcl_compiler::analyser::ClassFactory {
    class(src, qn)
        .factory
        .unwrap_or_else(|| panic!("{qn} is a class factory"))
}

#[test]
fn unknown_dispatch_that_constructs_and_returns_the_word_is_proved() {
    crate::install_surfaces();
    // TP — the Tk idiom. The metaclass declares `unknown`, that
    // body constructs an object named from its first parameter, and it
    // returns exactly that parameter, so a bare `Widget .w` call binds an
    // instance.
    assert!(
        factory_of(UNKNOWN_CTOR, "::Meta").unknown_binds_instance,
        "the Tk megawidget unknown-dispatch constructor must be proved"
    );
}

#[test]
fn unknown_dispatch_cannot_construct_after_return_completion() {
    crate::install_surfaces();
    // FP guard: Tcl stops the path at `return`; a later construction is
    // unreachable and cannot prove that the returned word names an
    // object.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        return $w\n",
        "        [self] create ::$w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_ignores_an_unselected_construction_arm() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        if {0} { [self] create ::$w; return $w }\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_requires_construction_on_every_dynamic_return_path() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        if {$runtime} { [self] create ::$w }\n",
        "        return $w\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_dynamic_arm_cannot_leave_a_normal_fallthrough_path() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        if {$runtime} { [self] create ::$w; return $w }\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_next_to_a_normally_returning_provider_abstains() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create ParentMeta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} { return inherited }\n",
        "}\n",
        "oo::class create Meta {\n",
        "    superclass ParentMeta\n",
        "    method unknown {w args} {\n",
        "        if {[string match .* $w]} {\n",
        "            [self] create ::$w {*}$args\n",
        "            return $w\n",
        "        }\n",
        "        next $w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "a normal inherited fallback result prevents a handle proof"
    );
}

#[test]
fn unknown_dispatch_nextto_uses_its_registry_declared_anchor() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create ParentMeta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} { return inherited }\n",
        "}\n",
        "oo::class create Meta {\n",
        "    superclass ParentMeta\n",
        "    method unknown {w args} {\n",
        "        if {[string match .* $w]} {\n",
        "            [self] create ::$w {*}$args\n",
        "            return $w\n",
        "        }\n",
        "        nextto ParentMeta $w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_next_abstains_on_a_cyclic_hierarchy() {
    crate::install_surfaces();
    // Malformed-input conservatism: C Tcl 9.0.4 rejects the forward
    // mixin names below, and also rejects an equivalent post-definition
    // CycleA <-> CycleB update with "may not mix a class into itself".
    // The analyser still records enough partial structure to encounter
    // the relation cycle, but must never treat its truncated MRO as a
    // proved route to the terminal builtin fallback.
    let src = concat!(
        "oo::class create CycleA { mixin CycleB }\n",
        "oo::class create CycleB { mixin CycleA }\n",
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    mixin CycleA\n",
        "    method unknown {w args} {\n",
        "        if {[string match .* $w]} {\n",
        "            [self] create ::$w {*}$args\n",
        "            return $w\n",
        "        }\n",
        "        next $w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_next_abstains_on_an_incomplete_hierarchy() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class ::External\n",
        "    method unknown {w args} {\n",
        "        if {[string match .* $w]} {\n",
        "            [self] create ::$w {*}$args\n",
        "            return $w\n",
        "        }\n",
        "        next $w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_rejects_a_malformed_if_path() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        if {1}\n",
        "        [self] create ::$w\n",
        "        return $w\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_non_normal_return_does_not_prove_a_handle() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        [self] create ::$w\n",
        "        return -code error $w\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_stops_at_an_earlier_terminating_command() {
    crate::install_surfaces();
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        error stopped\n",
        "        [self] create ::$w\n",
        "        return $w\n",
        "    }\n",
        "}\n",
    );
    assert!(!factory_of(src, "::Meta").unknown_binds_instance);
}

#[test]
fn unknown_dispatch_returning_something_else_abstains() {
    crate::install_surfaces();
    // TN — it constructs, but hands back the *class*, so the
    // caller's variable is not the new object's name. Guessing here would
    // type a handle that does not exist.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        [self] create ::$w {*}$args\n",
        "        return [self]\n",
        "    }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "an unknown that returns the class must not be read as returning the object"
    );
}

#[test]
fn unknown_dispatch_that_does_not_construct_abstains() {
    crate::install_surfaces();
    // TN — an `unknown` that merely echoes its argument creates
    // no object at all, so binding a handle to it would be a fabrication.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} { return $w }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "an unknown that constructs nothing must abstain"
    );
}

#[test]
fn unknown_dispatch_with_no_return_abstains() {
    crate::install_surfaces();
    // TN — a pure delegation to `next` proves nothing about the
    // value the call yields.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} { next $w {*}$args }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "an unknown with no proved result must abstain"
    );
}

#[test]
fn unknown_dispatch_returning_a_derived_word_abstains() {
    crate::install_surfaces();
    // TN — `return $w.hull` is not the caller's word, and a
    // consumer binding the caller's variable to the class would be wrong.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        [self] create ::$w {*}$args\n",
        "        return $w.hull\n",
        "    }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "a derived return word must abstain"
    );
}

#[test]
fn unknown_dispatch_constructing_a_derived_word_abstains() {
    crate::install_surfaces();
    // FP guard — the returned value is exactly the caller's word,
    // but the constructed command has a suffix. Seeing the parameter
    // somewhere in the name word is not proof that the returned handle
    // names that object.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        [self] create $w.hull {*}$args\n",
        "        return $w\n",
        "    }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "constructing a derived name must not type the returned word"
    );
}

#[test]
fn unknown_dispatch_cannot_combine_evidence_from_different_branches() {
    crate::install_surfaces();
    // FP guard: One branch constructs but does not return the
    // handle; the other returns the requested word without constructing
    // it. No successful path proves both facts, so flattening nested
    // scripts and combining their evidence would invent an object type.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "    method unknown {w args} {\n",
        "        if {$w eq .construct} { [self] create ::$w }\n",
        "        if {$w eq .echo} { return $w }\n",
        "        next $w {*}$args\n",
        "    }\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "construction and return evidence from disjoint branches must not be joined"
    );
}

#[test]
fn a_metaclass_without_unknown_dispatch_abstains() {
    crate::install_surfaces();
    // TN: the ordinary metaclass shape must keep answering `false` —
    // without the constructor idiom, a bare word must not silently
    // bind to an instance.
    let src = concat!(
        "oo::class create Meta {\n",
        "    superclass oo::class\n",
        "}\n",
    );
    assert!(
        !factory_of(src, "::Meta").unknown_binds_instance,
        "a metaclass with no unknown member must abstain"
    );
}

#[test]
fn unknown_dispatch_proof_survives_the_cross_document_rehome() {
    crate::install_surfaces();
    // The fact is a property of the metaclass's body and carries no
    // token, so it must cross a document boundary unchanged — otherwise
    // the Tk shape would resolve only in the file that declares the
    // metaclass.
    let elsewhere = tcl_lexer::Token::new(tcl_lexer::TokenType::Esc, tcl_lexer::Span::new(0, 1));
    assert!(
        factory_of(UNKNOWN_CTOR, "::Meta")
            .resolve_in_other_document(elsewhere, &|_| false)
            .unknown_binds_instance,
        "the proof must survive `resolve_in_other_document`"
    );
}
