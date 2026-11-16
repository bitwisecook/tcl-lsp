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

//! Element facts of lists, dictionaries and arrays through builders, retrievals, `lassign`, `foreach` and in-place writes.

use super::*;

/// A `dict set VAR k [Class new]` collection retrieved by `dict get` types
/// the element as the class (the collection-of-objects shape).
#[test]
fn dict_of_objects_retrieval_types_element() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   dict set pins a [Pin new]\n\
                   set p [dict get $pins a]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    let pins_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "pins" && t.element_class() == Some("::Pin")
    });
    assert!(
        pins_ok,
        "pins should be Dict<OBJECT(::Pin)>; got {:?}",
        fu.types
            .iter()
            .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
            .collect::<Vec<_>>()
    );
    let p_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "p"
            && t.tcl_type() == Some(TclType::Object)
            && t.class_name() == Some("::Pin")
    });
    assert!(p_ok, "p (dict get) should be OBJECT(::Pin)");
}

/// A `lappend VAR [Class new]` list retrieved by `lindex` types the element.
#[test]
fn list_of_objects_lindex_types_element() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create Pin {}\n\
                   lappend pins [Pin new]\n\
                   set p [lindex $pins 0]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    let p_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "p"
            && t.tcl_type() == Some(TclType::Object)
            && t.class_name() == Some("::Pin")
    });
    assert!(p_ok, "p (lindex) should be OBJECT(::Pin)");
}

/// A collection written with two *different* object classes is not
/// homogeneous, so the element class widens away (retrieval stays untyped).
#[test]
fn heterogeneous_object_collection_drops_element_class() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create A {}\noo::class create B {}\n\
                   dict set d k1 [A new]\n\
                   dict set d k2 [B new]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    // The latest `d` version must not claim a single element class.
    let widened = fu
        .types
        .iter()
        .filter(|((name, _), _)| fu.ssa.var_name(*name) == "d")
        .all(|((_, ver), t)| *ver < 2 || t.element_class().is_none());
    assert!(
        widened,
        "mixed-class dict must drop its element class; types {:?}; operations {:?}",
        fu.types,
        fu.ssa
            .blocks
            .iter()
            .map(|(id, block)| (
                id,
                block
                    .statements
                    .iter()
                    .enumerate()
                    .map(|(index, statement)| (
                        &statement.defs,
                        &statement.uses,
                        crate::ssa::SsaSourceView::at_statement(&fu.ssa, *id, index).symbol("d"),
                    ))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
}

// registry-driven container element inference (type-tracking.md).

/// Helper: the joined lattice of every version of `var` in `func`.
fn type_of(cu: &crate::compilation_unit::CompilationUnit, func: &str, var: &str) -> TypeLattice {
    let fu = cu.function(func).unwrap();
    let mut symbols = HashSet::new();
    symbols.extend(fu.ssa.var_symbol(var));
    for (&block, cfg_block) in &fu.cfg.blocks {
        for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
            symbols
                .extend(crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var));
        }
    }
    let mut acc = TypeLattice::unknown();
    for ((sym, ver), t) in fu.types.iter() {
        if *ver > 0 && symbols.contains(sym) {
            acc = type_join(&acc, t);
        }
    }
    acc
}

/// Elements are shared objects — a committed computed element
/// keeps its intrep inside the list, and `lindex` retrieves it.
#[test]
fn list_builder_tracks_committed_element_and_lindex_retrieves_it() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set l [list [expr {2**20}] other]\nset first [lindex $l 0]",
        &registry(),
        false,
    );
    let l = type_of(&cu, "::top", "l");
    assert_eq!(l.tcl_type(), Some(TclType::List), "{l}");
    let elements = l.elements().expect("tracked elements").clone();
    assert_eq!(elements.known_len(), Some(2), "{l}");
    assert!(
        matches!(elements.shape_at(0), Some(s) if s.is_numeric_family()),
        "committed element 0 keeps its numeric intrep: {l}"
    );
    assert_eq!(
        elements.shape_at(1),
        None,
        "a literal word stays agnostic: {l}"
    );
    let first = type_of(&cu, "::top", "first");
    assert!(
        matches!(first.single_shape(), Some(s) if s.is_numeric_family()),
        "lindex retrieves the element's shape: {first}"
    );
}

/// Arity is a fact even with unknown element values: `[list $a $b]`
/// provably has two elements.
#[test]
fn list_builder_tracks_arity_of_unknown_words() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "proc f {a b} { set l [list $a $b]\n return $l }",
        &registry(),
        false,
    );
    let l = type_of(&cu, "::f", "l");
    assert_eq!(
        l.elements().and_then(Elements::known_len),
        Some(2),
        "arity of [list $a $b] is exactly 2: {l}"
    );
}

/// The old object-collection behaviour, now through general elements:
/// `lappend`ing constructed objects yields `List<OBJECT(C)*>` and a
/// `lassign` / `lindex` retrieval resolves the class.
#[test]
fn lassign_of_committed_object_list_types_targets() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nlassign $l a b",
        &registry(),
        false,
    );
    let a = type_of(&cu, "::top", "a");
    assert_eq!(a.tcl_type(), Some(TclType::Object), "{a}");
    assert_eq!(a.class_name(), Some("::Foo"), "{a}");
    let b = type_of(&cu, "::top", "b");
    assert_eq!(b.class_name(), Some("::Foo"), "{b}");
}

/// FP-SH-17 parity: `lassign` targets from a *pure-literal* list stay
/// Overdefined — no shape claim is defensible for value-dependent
/// conversion checks.
#[test]
fn lassign_of_literal_list_targets_stay_overdefined() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set point [list 1 2 3]\nlassign $point x y z",
        &registry(),
        false,
    );
    let x = type_of(&cu, "::top", "x");
    assert_eq!(x.kind(), TypeKind::Overdefined, "{x}");
}

/// A `lassign` target past the container's arity receives the empty
/// string the command pads with.
#[test]
fn lassign_past_arity_is_empty_string() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set l [list [expr {1+1}]]\nlassign $l a b",
        &registry(),
        false,
    );
    let b = type_of(&cu, "::top", "b");
    assert_eq!(b.tcl_type(), Some(TclType::String), "{b}");
}

/// `dict create` of constructed objects → `Dict<OBJECT(C)*>`; `dict get`
/// resolves the element class (the old object retrieval, generalised).
#[test]
fn dict_create_and_get_track_object_values() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Pin {}\nset pins [dict create p1 [Pin new] p2 [Pin new]]\nset p [dict get $pins p1]",
        &registry(),
        false,
    );
    let pins = type_of(&cu, "::top", "pins");
    assert_eq!(pins.element_class(), Some("::Pin"), "{pins}");
    let p = type_of(&cu, "::top", "p");
    assert_eq!(p.class_name(), Some("::Pin"), "{p}");
}

/// `lappend var [C new]` evolves the variable's elements — the old
/// `collection_element_class` behaviour through the registry's
/// `VarElementsEffect`, aliased spellings included.
#[test]
fn lappend_object_evolves_element_class_through_rename() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset acc [list]\nlappend acc [Foo new]\nlappend acc [Foo new]",
        &registry(),
        false,
    );
    let acc = type_of(&cu, "::top", "acc");
    assert_eq!(acc.tcl_type(), Some(TclType::List), "{acc}");
    assert_eq!(acc.element_class(), Some("::Foo"), "{acc}");
}

/// A `foreach` element var over a tracked homogeneous list takes the
/// element shape at its def (the loop-header phi separately joins the
/// live-in as Overdefined, by the conditionally-assigned rule).
#[test]
fn foreach_var_takes_uniform_element_shape() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nforeach o $l { puts $o }",
        &registry(),
        false,
    );
    let fu = cu.function("::top").unwrap();
    let has_object_def = fu.types.iter().any(|((sym, ver), t)| {
        *ver > 0 && fu.ssa.var_name(*sym) == "o" && t.class_name() == Some("::Foo")
    });
    assert!(
        has_object_def,
        "some version of 'o' must carry the element class: {:?}",
        fu.types
    );
}

/// constant-keyed array elements are independent variables — each
/// carries its own type ("array elements behave as independent
/// scalars"), and the conflated base claims nothing.
#[test]
fn array_elements_type_independently() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} {\n    set arr(a) 5\n    set arr(b) \"hello world\"\n    return $arr(a)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let type_of = |name: &str| -> Vec<Option<TclType>> {
        fu.types
            .iter()
            .filter(|((sym, ver), _)| *ver > 0 && fu.ssa.var_name(*sym) == name)
            .map(|(_, t)| t.tcl_type())
            .collect()
    };
    assert!(
        type_of("arr(a)").iter().all(|t| *t == Some(TclType::Int)),
        "arr(a) is INT in every version: {:?}",
        fu.types
    );
    assert!(
        type_of("arr(b)")
            .iter()
            .all(|t| *t == Some(TclType::String)),
        "arr(b) is STRING independently: {:?}",
        fu.types
    );
    assert!(
        type_of("arr").iter().all(Option::is_none),
        "the base symbol claims no value type: {:?}",
        fu.types
    );
}

/// a dynamic-key write is a may-write over every known element —
/// the element's type JOINS with the written type (INT ⊔ INT stays
/// INT; INT ⊔ STRING widens) instead of trusting either side.
#[test]
fn dynamic_key_write_joins_element_types() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {k} {\n    set a(x) 5\n    set a($k) 9\n    return $a(x)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let final_is_int = fu.return_type.tcl_type() == Some(TclType::Int);
    assert!(
        final_is_int,
        "INT ⊔ INT across the may-write stays INT: {:?}",
        fu.types
    );

    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc g {k} {\n    set a(x) 5\n    set a($k) \"s t\"\n    return $a(x)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::g").unwrap();
    let widened = fu.return_type.tcl_type() == Some(TclType::Int);
    assert!(
        !widened,
        "INT ⊔ STRING must not stay a single INT claim: {:?}",
        fu.types
    );
}

/// Dict-value semantics, verified on tclsh 8.6/9.0:
/// a multi-key `dict set` stores a *dict* under the first key; `dict
/// lappend` stores a *list*; `dict append` concatenates (intreps do
/// not survive), with only object dispatch-identity flowing.
#[test]
fn dict_value_effects_match_oracle() {
    for profile in ["tcl8.6", "tcl9.0"] {
        assert_dict_value_effects(
            tcl_dialect::DialectProfile::find(profile).expect("native Tcl fixture"),
        );
    }
}

fn assert_dict_value_effects(profile: &'static tcl_dialect::DialectProfile) {
    let reg = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let elements_of = |src: &str, qname: &str, var: &str| -> Vec<Option<TclType>> {
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
        let fu = cu.function(qname).unwrap();
        let mut symbols = HashSet::new();
        symbols.extend(fu.ssa.var_symbol(var));
        for (&block, cfg_block) in &fu.cfg.blocks {
            for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
                symbols.extend(
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var),
                );
            }
        }
        fu.types
            .iter()
            .filter(|((sym, ver), _)| *ver > 0 && symbols.contains(sym))
            .map(|(_, t)| {
                t.elements()
                    .and_then(Elements::uniform_shape)
                    .map(|s| s.coarse())
            })
            .collect()
    };
    // Multi-key set: the top-level value is a DICT, never the leaf shape.
    let multi = elements_of(
        "proc f {} {\n    set d {}\n    dict set d outer inner [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        multi.iter().flatten().all(|t| *t == TclType::Dict),
        "multi-key dict set stores a nested dict: {multi:?}"
    );
    // Single-key set keeps the leaf's shape.
    let single = elements_of(
        "proc f {} {\n    set d {}\n    dict set d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        single
            .iter()
            .any(|t| matches!(t, Some(TclType::Int | TclType::Numeric))),
        "single-key dict set keeps the leaf shape: {single:?}"
    );
    // lappend: the value is a LIST (elements unknown — the prior scalar
    // becomes element 0).
    let lap = elements_of(
        "proc f {} {\n    set d {}\n    dict lappend d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        lap.iter().flatten().all(|t| *t == TclType::List),
        "dict lappend listifies the value: {lap:?}"
    );
    // append: concatenation — a numeric argument's intrep must NOT
    // become the value's element fact.
    let app = elements_of(
        "proc f {} {\n    set d {}\n    dict append d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        !app.iter()
            .flatten()
            .any(|t| matches!(t, TclType::Int | TclType::Numeric)),
        "dict append must not claim a numeric value intrep: {app:?}"
    );
}
