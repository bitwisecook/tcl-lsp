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

//! `next` and `nextto` references: methods and constructor and destructor chains.

use super::*;

#[test]
fn method_references_include_next_dispatch() {
    // `Sub::greet`'s body invokes the super via `next`; that dispatch is a
    // polymorphic reference to `greet` and must appear among its
    // references.
    let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `greet` in Sub's declaration (line 5).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        13,
        &analysis,
        true,
    );
    // The `next` token sits on line 5, past the method's own `greet` name
    // (col 11) — inside the `{ next }` body.
    assert!(
        refs.iter()
            .any(|r| r.start_line == 5 && r.start_character > 15),
        "expected the `next` dispatch among refs: {refs:?}",
    );
}

// Constructor / destructor next-chain references.

#[test]
fn constructor_next_chain_reference_from_direct_subclass() {
    // `Sub`'s constructor calls plain `next`, chaining to `Base`'s —
    // cursor on `Base`'s own `constructor` keyword must surface that
    // chain as a reference.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
    let analysis = analyse(src);
    // `constructor` keyword on line 1, col 4.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        true,
    );
    // decl (line 1) + the `next` call site (line 5).
    assert_eq!(refs.len(), 2, "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_skips_non_overriding_subclass() {
    // `Sub` inherits `Base`'s constructor outright (declares none of its
    // own) — nothing to chain, so `Base`'s constructor stays unreferenced.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_not_counted_without_next() {
    // `Sub` declares its own constructor but never calls `next` — a
    // legitimate full override, not a chain.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { set x 1 }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_skips_ancestor_with_no_own_constructor() {
    // `Mid` declares no constructor of its own; `Sub`'s `next` must
    // still reach `Base` (the actual MRO-effective provider), not `Mid`.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Mid {\n    superclass Base\n}\noo::class create Sub {\n    superclass Mid\n    constructor {} { next }\n}\n";
    let analysis = analyse(src);
    // `Base`'s constructor (line 1) picks up the chain.
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(base_refs.len(), 1, "{base_refs:?}");
    assert_eq!(base_refs[0].start_line, 8, "{base_refs:?}");
}

#[test]
fn constructor_next_chain_reference_via_nextto_explicit_target() {
    // `nextto Grandparent` jumps past `Base` even though `Base` also has
    // an effective constructor — only `Grandparent` picks up a reference.
    let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto Grandparent }\n}\n";
    let analysis = analyse(src);
    let grandparent_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        false,
    );
    assert!(base_refs.is_empty(), "{base_refs:?}");
}

#[test]
fn constructor_next_chain_reference_via_braced_nextto_target() {
    // `nextto {Grandparent}` (a
    // braced target, functionally identical to the bare form) must
    // resolve exactly like `constructor_next_chain_reference_via_nextto_explicit_target`'s
    // bare `nextto Grandparent` — the decoded word, not the raw
    // delimited span, is what gets resolved against the class map.
    let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto {Grandparent} }\n}\n";
    let analysis = analyse(src);
    let grandparent_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        false,
    );
    assert!(base_refs.is_empty(), "{base_refs:?}");
}

#[test]
fn destructor_next_chain_reference_from_direct_subclass() {
    let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
    let analysis = analyse(src);
    // `destructor` keyword on line 1, col 4.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        5,
        &analysis,
        true,
    );
    assert_eq!(refs.len(), 2, "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
}

#[test]
fn constructor_shadowed_by_a_later_redeclaration_has_no_references() {
    // `oo::configurable` allows several constructors; only the last is
    // ever effective. A cursor on the shadowed first one resolves to
    // nothing (it has no reference story worth surfacing).
    let src = "oo::class create C {\n    constructor {} { }\n    constructor {} { }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        true,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method() {
    // A class can also declare a
    // `method` literally named `constructor` — an independent, ordinary
    // member sharing a name with the special keyword form. A cursor on
    // the special `constructor` keyword must still resolve its own
    // next-chain story, not fall through to the unrelated same-named
    // method's (ordinary) references.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n    method constructor {} { }\n}\n";
    let analysis = analyse(src);
    // `Base`'s `constructor` keyword, line 1.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(
        refs.len(),
        1,
        "must resolve the next-chain, not the unrelated same-named method: {refs:?}"
    );
    assert_eq!(refs[0].start_line, 5, "{refs:?}");
}
