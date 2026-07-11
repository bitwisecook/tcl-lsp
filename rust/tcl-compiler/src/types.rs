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

//! Tcl internal representation (intrep) type lattice.
//!
//! Tcl values are always strings but may cache a typed internal
//! representation. This module models the set of known intreps
//! and defines a lattice for tracking them through SSA dataflow.
//!
//! Lattice order (bottom to top):
//!
//! ```text
//! UNKNOWN  <  KNOWN(t)  <  SHIMMERED(a, b)  <  OVERDEFINED
//! ```

use std::fmt;

// Re-export TclType from the registry crate (single source of truth).
pub use tcl_registry::TclType;

/// Lattice element kind, ordered bottom to top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// Bottom — no information.
    Unknown,
    /// Concrete type known.
    Known,
    /// Value has been shimmered between two types.
    Shimmered,
    /// Top — too many types to track.
    Overdefined,
}

/// A single element of the type lattice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeLattice {
    /// Lattice position.
    pub kind: TypeKind,
    /// The concrete type (for `Known` and `Shimmered`).
    pub tcl_type: Option<TclType>,
    /// The source type before shimmer (for `Shimmered` only).
    pub from_type: Option<TclType>,
    /// Class name (for `Object` types only).
    pub class_name: Option<String>,
    /// Element class (for `List` / `Dict` *of objects* only): the value is a
    /// collection every element of which is an `OBJECT(element_class)`.  Set by
    /// [`Self::collection_of`], harvested from `dict set/append VAR k [C new]`,
    /// `lappend VAR [C new]`, and `list`/`dict create` of object values, and
    /// read out at retrieval sites (`dict get` / `lindex`) so an element
    /// dispatched as `[dict get $coll $k] method …` resolves to `C`.  A join
    /// of two collections whose element classes differ drops it (the container
    /// is still `List`/`Dict`, just no longer object-homogeneous).
    pub element_class: Option<String>,
}

impl TypeLattice {
    /// The bottom element — no type information.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            kind: TypeKind::Unknown,
            tcl_type: None,
            from_type: None,
            class_name: None,
            element_class: None,
        }
    }

    /// The top element — too many types to track.
    #[must_use]
    pub fn overdefined() -> Self {
        Self {
            kind: TypeKind::Overdefined,
            tcl_type: None,
            from_type: None,
            class_name: None,
            element_class: None,
        }
    }

    /// A known concrete type.
    #[must_use]
    pub fn of(t: TclType) -> Self {
        Self {
            kind: TypeKind::Known,
            tcl_type: Some(t),
            from_type: None,
            class_name: None,
            element_class: None,
        }
    }

    /// A known `Object` type for a specific class.
    #[must_use]
    pub fn object_of(class_name: impl Into<String>) -> Self {
        Self {
            kind: TypeKind::Known,
            tcl_type: Some(TclType::Object),
            from_type: None,
            class_name: Some(class_name.into()),
            element_class: None,
        }
    }

    /// A known `List` / `Dict` whose elements are all `OBJECT(element_class)`.
    /// `container` must be [`TclType::List`] or [`TclType::Dict`]; any other
    /// type is treated as a plain known type with no element class (callers
    /// only ever pass the two container types).
    #[must_use]
    pub fn collection_of(container: TclType, element_class: impl Into<String>) -> Self {
        let element_class =
            matches!(container, TclType::List | TclType::Dict).then(|| element_class.into());
        Self {
            kind: TypeKind::Known,
            tcl_type: Some(container),
            from_type: None,
            class_name: None,
            element_class,
        }
    }

    /// A shimmered pair (canonical order: from < to by `Ord`).
    #[must_use]
    pub fn shimmered(from: TclType, to: TclType) -> Self {
        let (a, b) = if from <= to { (from, to) } else { (to, from) };
        Self {
            kind: TypeKind::Shimmered,
            tcl_type: Some(b),
            from_type: Some(a),
            class_name: None,
            element_class: None,
        }
    }

    /// The object class of this collection's elements, when it is a
    /// `List`/`Dict` known to be homogeneous in `OBJECT(class)` — the signal a
    /// `dict get` / `lindex` retrieval reads to type its result.
    #[must_use]
    pub fn element_class(&self) -> Option<&str> {
        (self.kind == TypeKind::Known
            && matches!(self.tcl_type, Some(TclType::List | TclType::Dict)))
        .then_some(self.element_class.as_deref())
        .flatten()
    }
}

impl fmt::Display for TypeLattice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TypeKind::Unknown => write!(f, "UNKNOWN"),
            TypeKind::Overdefined => write!(f, "OVERDEFINED"),
            TypeKind::Known if self.tcl_type == Some(TclType::Object) => {
                write!(f, "OBJECT({})", self.class_name.as_deref().unwrap_or("?"))
            }
            TypeKind::Known if self.element_class().is_some() => write!(
                f,
                "{:?}<OBJECT({})>",
                self.tcl_type.unwrap(),
                self.element_class().unwrap()
            ),
            TypeKind::Known => write!(f, "{:?}", self.tcl_type.unwrap()),
            TypeKind::Shimmered => write!(
                f,
                "SHIMMERED({:?}, {:?})",
                self.from_type.unwrap(),
                self.tcl_type.unwrap()
            ),
        }
    }
}

/// Compute the join (least upper bound) of two type lattice elements.
#[must_use]
pub fn type_join(a: &TypeLattice, b: &TypeLattice) -> TypeLattice {
    // Identity / absorbing
    if a.kind == TypeKind::Unknown {
        return b.clone();
    }
    if b.kind == TypeKind::Unknown {
        return a.clone();
    }
    if a.kind == TypeKind::Overdefined || b.kind == TypeKind::Overdefined {
        return TypeLattice::overdefined();
    }

    // Both KNOWN
    if a.kind == TypeKind::Known && b.kind == TypeKind::Known {
        let at = a.tcl_type.unwrap();
        let bt = b.tcl_type.unwrap();
        if at == bt {
            if at == TclType::Object {
                if a.class_name == b.class_name {
                    return a.clone();
                }
                return TypeLattice::of(TclType::Object);
            }
            // A `List`/`Dict` carries an element class only while both sides
            // agree it is object-homogeneous in the *same* class; otherwise the
            // container type survives but the element class widens away.
            if matches!(at, TclType::List | TclType::Dict) && a.element_class != b.element_class {
                return TypeLattice::of(at);
            }
            return a.clone();
        }
        // Numeric promotion
        if let Some(promoted) = numeric_promotion(at, bt) {
            return TypeLattice::of(promoted);
        }
        return TypeLattice::shimmered(at, bt);
    }

    // Both SHIMMERED
    if a.kind == TypeKind::Shimmered && b.kind == TypeKind::Shimmered {
        if a == b {
            return a.clone();
        }
        return TypeLattice::overdefined();
    }

    // One KNOWN, one SHIMMERED — normalise so `known` is first
    let (known, shimmer) = if a.kind == TypeKind::Known {
        (a, b)
    } else {
        (b, a)
    };
    let kt = known.tcl_type.unwrap();
    let (from, to) = (shimmer.from_type.unwrap(), shimmer.tcl_type.unwrap());
    // Exact match on either side: the known type is already one of the two the
    // value shimmers between, so the merge stays that same shimmer.
    if kt == from || kt == to {
        return shimmer.clone();
    }
    // Numeric refinement: a `Known` numeric type (Int / Double / Boolean /
    // Numeric) meeting a *numeric* shimmer side is subsumed by that side —
    // `expr {$x + 1}` on an unknown-typed `$x` legitimately yields the coarser
    // `Numeric`, so an `Int` reaching that merge is not a third, conflicting
    // intrep. The value still oscillates between "numeric" and the other side,
    // so it must stay SHIMMERED rather than degrade to OVERDEFINED (which
    // silently masks the shimmer for S100/S101/S102 — a false negative). The
    // numeric side widens to `Numeric` so the result is a sound upper bound of
    // the `Known` type: `Double ⊔ SHIMMERED(Int, String)` = `SHIMMERED(Numeric,
    // String)`, which covers `Double` — `SHIMMERED(Int, …)` would not. Exact
    // equality is handled above, so widening here never narrows.
    //
    // NB: this must NOT be routed through `numeric_promotion`, which returns
    // `Some` whenever *either* operand is `Double` (so `numeric_promotion(Double,
    // String)` wrongly yields `Numeric`) — that would drop the genuinely
    // non-numeric side. The predicate below requires *both* the known type and
    // the matched side to be numeric-family.
    if is_numeric_family(kt) {
        if is_numeric_family(from) {
            return TypeLattice::shimmered(TclType::Numeric, to);
        }
        if is_numeric_family(to) {
            return TypeLattice::shimmered(from, TclType::Numeric);
        }
    }
    TypeLattice::overdefined()
}

/// True when `t` is a numeric-family intrep — one whose values all coerce to a
/// number, so it is subsumed by the coarse [`TclType::Numeric`]. `Boolean`
/// counts (Tcl booleans are `0`/`1` integers).
fn is_numeric_family(t: TclType) -> bool {
    matches!(
        t,
        TclType::Int | TclType::Double | TclType::Boolean | TclType::Numeric
    )
}

/// Numeric promotion for Tcl types.
fn numeric_promotion(a: TclType, b: TclType) -> Option<TclType> {
    // Use an unordered pair for symmetric matching.
    let pair = [a, b];
    let has = |t: TclType| pair.contains(&t);

    if has(TclType::Boolean) && has(TclType::Int) {
        return Some(TclType::Int);
    }
    let numeric_pair = (has(TclType::Boolean) || has(TclType::Int) || has(TclType::Double))
        && (has(TclType::Double) || has(TclType::Numeric));
    if numeric_pair {
        return Some(TclType::Numeric);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_bottom() {
        let u = TypeLattice::unknown();
        assert_eq!(u.kind, TypeKind::Unknown);
        let k = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&u, &k), k);
        assert_eq!(type_join(&k, &u), k);
    }

    #[test]
    fn overdefined_is_top() {
        let o = TypeLattice::overdefined();
        let k = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&o, &k).kind, TypeKind::Overdefined);
        assert_eq!(type_join(&k, &o).kind, TypeKind::Overdefined);
    }

    #[test]
    fn same_type_is_identity() {
        let a = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&a, &a), a);
    }

    #[test]
    fn bool_int_promotes_to_int() {
        let a = TypeLattice::of(TclType::Boolean);
        let b = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&a, &b), TypeLattice::of(TclType::Int));
    }

    #[test]
    fn int_double_promotes_to_numeric() {
        let a = TypeLattice::of(TclType::Int);
        let b = TypeLattice::of(TclType::Double);
        assert_eq!(type_join(&a, &b), TypeLattice::of(TclType::Numeric));
    }

    #[test]
    fn incompatible_types_shimmer() {
        let a = TypeLattice::of(TclType::Int);
        let b = TypeLattice::of(TclType::List);
        let joined = type_join(&a, &b);
        assert_eq!(joined.kind, TypeKind::Shimmered);
    }

    #[test]
    fn different_shimmered_is_overdefined() {
        let a = TypeLattice::shimmered(TclType::Int, TclType::List);
        let b = TypeLattice::shimmered(TclType::Double, TclType::Dict);
        assert_eq!(type_join(&a, &b).kind, TypeKind::Overdefined);
    }

    #[test]
    fn same_shimmered_is_identity() {
        let a = TypeLattice::shimmered(TclType::Int, TclType::List);
        assert_eq!(type_join(&a, &a), a);
    }

    #[test]
    fn known_matches_shimmered_side() {
        let shimmer = TypeLattice::shimmered(TclType::Int, TclType::List);
        let known = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&known, &shimmer), shimmer);
    }

    #[test]
    fn known_no_match_shimmered_is_overdefined() {
        let shimmer = TypeLattice::shimmered(TclType::Int, TclType::List);
        let known = TypeLattice::of(TclType::Dict);
        assert_eq!(type_join(&known, &shimmer).kind, TypeKind::Overdefined);
    }

    /// A `Known(Int)` meeting `SHIMMERED(Numeric, String)` must stay
    /// SHIMMERED, not degrade to OVERDEFINED: `Int` is subsumed by the
    /// `Numeric` side (`expr {$x + 1}` on an unknown `$x` yields `Numeric`),
    /// so the value still oscillates numeric↔string. Degrading here silently
    /// masks the shimmer for S100/S101/S102.
    #[test]
    fn known_int_matches_numeric_shimmer_side() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::String);
        let known = TypeLattice::of(TclType::Int);
        let joined = type_join(&known, &shimmer);
        assert_eq!(joined.kind, TypeKind::Shimmered);
        assert_eq!(
            joined,
            TypeLattice::shimmered(TclType::Numeric, TclType::String)
        );
        // Symmetric.
        assert_eq!(type_join(&shimmer, &known), joined);
    }

    /// `Boolean` (an int-family type) also matches a `Numeric` shimmer side.
    #[test]
    fn known_boolean_matches_numeric_shimmer_side() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::List);
        let known = TypeLattice::of(TclType::Boolean);
        assert_eq!(
            type_join(&known, &shimmer),
            TypeLattice::shimmered(TclType::Numeric, TclType::List)
        );
    }

    /// A `Known(Double)` meeting `SHIMMERED(Int, String)` must widen the
    /// numeric side to `Numeric` — `SHIMMERED(Int, String)` would not be a
    /// sound upper bound of `Double` (it does not cover `Double`). This keeps
    /// `type_join` a true least-upper-bound.
    #[test]
    fn known_double_widens_int_shimmer_side_to_numeric() {
        let shimmer = TypeLattice::shimmered(TclType::Int, TclType::String);
        let known = TypeLattice::of(TclType::Double);
        assert_eq!(
            type_join(&known, &shimmer),
            TypeLattice::shimmered(TclType::Numeric, TclType::String)
        );
    }

    /// A non-numeric `Known` type that matches neither side of a shimmer still
    /// degrades to OVERDEFINED — the numeric refinement must not blanket-keep
    /// every Known/Shimmered join shimmered.
    #[test]
    fn known_non_numeric_no_match_still_overdefined() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::List);
        let known = TypeLattice::of(TclType::Dict);
        assert_eq!(type_join(&known, &shimmer).kind, TypeKind::Overdefined);
    }

    /// An exact match on a non-numeric side is unchanged by the refinement:
    /// `Known(List) ⊔ SHIMMERED(Int, List)` stays `SHIMMERED(Int, List)`.
    #[test]
    fn known_exact_non_numeric_side_unchanged() {
        let shimmer = TypeLattice::shimmered(TclType::Int, TclType::List);
        let known = TypeLattice::of(TclType::List);
        assert_eq!(type_join(&known, &shimmer), shimmer);
    }

    #[test]
    fn object_same_class() {
        let a = TypeLattice::object_of("Foo");
        assert_eq!(type_join(&a, &a), a);
    }

    #[test]
    fn object_different_class_widens() {
        let a = TypeLattice::object_of("Foo");
        let b = TypeLattice::object_of("Bar");
        let joined = type_join(&a, &b);
        assert_eq!(joined, TypeLattice::of(TclType::Object));
    }

    #[test]
    fn display_variants() {
        assert_eq!(TypeLattice::unknown().to_string(), "UNKNOWN");
        assert_eq!(TypeLattice::overdefined().to_string(), "OVERDEFINED");
        assert_eq!(TypeLattice::of(TclType::Int).to_string(), "Int");
        assert_eq!(TypeLattice::object_of("Foo").to_string(), "OBJECT(Foo)");
        assert_eq!(
            TypeLattice::shimmered(TclType::Int, TclType::List).to_string(),
            "SHIMMERED(Int, List)"
        );
    }
}
