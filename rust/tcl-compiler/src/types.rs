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
//! representation. This module models the set of known intreps as
//! [`TypeShape`] terms — refining the registry's coarse [`TclType`]
//! vocabulary with the numeric tower's bignum rung and per-element container
//! facts — and tracks a **bounded union** of possible shapes per SSA value
//! through dataflow (`docs/design/compiler/type-tracking.md`, P2).
//!
//! Lattice order (bottom to top):
//!
//! ```text
//! UNKNOWN  <  UNION{s}  <  UNION{s1, s2}  <  …  <  OVERDEFINED
//! ```
//!
//! A singleton union is the old `KNOWN(t)`; a multi-member union generalises
//! the old `SHIMMERED(a, b)` pair — three or more differently-typed paths now
//! stay tracked instead of collapsing to `OVERDEFINED` (the documented lossy
//! point of the previous pair model). [`TypeKind`] survives as the coarse
//! classification consumers switch on.

use std::fmt;

use crate::bounded_set::BoundedSet;

// Re-export TclType from the registry crate (single source of truth).
pub use tcl_registry::TclType;

/// Maximum members of a type union before widening to `OVERDEFINED`. Six
/// covers every realistic per-variable type mix (the whole shape alphabet is
/// only ~11 constructors, and the numeric family collapses to one member).
pub const MAX_TYPE_UNION: usize = 6;

/// Maximum tracked per-position elements of a `List` shape. `list` calls with
/// more arguments keep the container shape but drop to [`Elements::Unknown`].
pub const MAX_EXACT_ELEMENTS: usize = 16;

/// Maximum container-nesting depth a shape records (`List<List<List<…>>>`).
/// Deeper structure widens the *inner* elements to [`Elements::Unknown`] —
/// bounding the lattice height so fixpoints over self-referential builders
/// (`lappend v [list $v]`) terminate.
pub const MAX_SHAPE_DEPTH: usize = 3;

/// Element facts for a `List` / `Dict` shape.
///
/// Faithful to the runtime: container elements are shared `Tcl_Obj`s, so a
/// computed element keeps its intrep inside the container (oracle-verified,
/// type-tracking.md corpus) — `[list [expr {1+1}] "x y"]` genuinely is a list
/// whose first element carries a numeric intrep.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Elements {
    /// No element facts (the lattice top for element structure).
    Unknown,
    /// Every element's shape is subsumed by the given shape.
    Uniform(Box<TypeShape>),
    /// Exactly `n` elements with per-position shapes; `None` positions carry
    /// no fact. The arity itself is a fact even when every position is
    /// `None` (`[list $a $b]` provably has `llength` 2).
    Exact(Box<[Option<TypeShape>]>),
}

impl Elements {
    /// The provable element count, when the structure pins one.
    #[must_use]
    pub fn known_len(&self) -> Option<usize> {
        match self {
            Elements::Exact(shapes) => Some(shapes.len()),
            Elements::Uniform(_) | Elements::Unknown => None,
        }
    }

    /// The shape of element `index`, when tracked.
    #[must_use]
    pub fn shape_at(&self, index: usize) -> Option<&TypeShape> {
        match self {
            Elements::Exact(shapes) => shapes.get(index).and_then(Option::as_ref),
            Elements::Uniform(shape) => Some(shape),
            Elements::Unknown => None,
        }
    }

    /// The single shape subsuming every element, when provable: a `Uniform`'s
    /// shape, or the join of an `Exact`'s fully-known positions.
    #[must_use]
    pub fn uniform_shape(&self) -> Option<TypeShape> {
        match self {
            Elements::Uniform(shape) => Some((**shape).clone()),
            Elements::Exact(shapes) => {
                let mut acc: Option<TypeShape> = None;
                for shape in &**shapes {
                    let shape = shape.as_ref()?;
                    acc = Some(match acc {
                        None => shape.clone(),
                        Some(prev) => shape_join(&prev, shape)?,
                    });
                }
                acc
            }
            Elements::Unknown => None,
        }
    }
}

/// A single value-type term — the compiler-internal refinement of the
/// registry's coarse [`TclType`] vocabulary.
///
/// Purity (`typePtr == NULL`) is deliberately **not** a shape: whether a
/// value's intrep is committed is a *program-point* property tracked by the
/// commit dataflow ([`crate::shimmer::commit`]), not a property of the value
/// term. A [`TypeShape::String`] is the "string rep" shape; whether it is
/// still pure at a given use is the commit pass's answer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeShape {
    /// String rep (pure or committed UTF cache — see the purity note above).
    String,
    /// Integer that fits a wide (`i64`).
    Int,
    /// Integer beyond a wide — the bignum rung of the numeric tower
    /// (`expr {2**64}`; promotion is by magnitude, never wrapping).
    Bignum,
    /// IEEE-754 double.
    Double,
    /// Boolean-valued (word booleans and 0/1 comparison results).
    Boolean,
    /// Abstract "some number" — the join of the numeric tower.
    Numeric,
    /// Binary data.
    ByteArray,
    /// Tcl list, with optional element facts.
    List(Elements),
    /// Tcl dict, with optional facts about its **values**.
    Dict(Elements),
    /// `TclOO` / snit object instance, with its class when known.
    Object(Option<Box<str>>),
    /// I/O channel handle.
    Channel,
}

impl TypeShape {
    /// The registry-vocabulary coarsening of this shape.
    #[must_use]
    pub fn coarse(&self) -> TclType {
        match self {
            TypeShape::String => TclType::String,
            TypeShape::Int | TypeShape::Bignum => TclType::Int,
            TypeShape::Double => TclType::Double,
            TypeShape::Boolean => TclType::Boolean,
            TypeShape::Numeric => TclType::Numeric,
            TypeShape::ByteArray => TclType::ByteArray,
            TypeShape::List(_) => TclType::List,
            TypeShape::Dict(_) => TclType::Dict,
            TypeShape::Object(_) => TclType::Object,
            TypeShape::Channel => TclType::Channel,
        }
    }

    /// The structure-free shape for a registry [`TclType`].
    #[must_use]
    pub fn from_coarse(t: TclType) -> Self {
        match t {
            TclType::String => TypeShape::String,
            TclType::Int => TypeShape::Int,
            TclType::Double => TypeShape::Double,
            TclType::Boolean => TypeShape::Boolean,
            TclType::Numeric => TypeShape::Numeric,
            TclType::ByteArray => TypeShape::ByteArray,
            TclType::List => TypeShape::List(Elements::Unknown),
            TclType::Dict => TypeShape::Dict(Elements::Unknown),
            TclType::Object => TypeShape::Object(None),
            TclType::Channel => TypeShape::Channel,
        }
    }

    /// Whether this shape is in the numeric family — every value coerces to a
    /// number, so it is subsumed by the abstract [`TypeShape::Numeric`].
    /// `Boolean` counts (Tcl booleans are `0`/`1` integers).
    #[must_use]
    pub fn is_numeric_family(&self) -> bool {
        matches!(
            self,
            TypeShape::Int
                | TypeShape::Bignum
                | TypeShape::Double
                | TypeShape::Boolean
                | TypeShape::Numeric
        )
    }

    /// Widen element facts nested deeper than [`MAX_SHAPE_DEPTH`] to
    /// [`Elements::Unknown`], bounding lattice height.
    #[must_use]
    fn clamp_depth(self) -> Self {
        fn clamp(shape: TypeShape, budget: usize) -> TypeShape {
            match shape {
                TypeShape::List(e) => TypeShape::List(clamp_elements(e, budget)),
                TypeShape::Dict(e) => TypeShape::Dict(clamp_elements(e, budget)),
                other => other,
            }
        }
        fn clamp_elements(e: Elements, budget: usize) -> Elements {
            // `budget` counts remaining *container* levels including the one
            // whose elements these are; at 1 the elements themselves would
            // exceed the cap, so they widen away.
            if budget <= 1 {
                return Elements::Unknown;
            }
            match e {
                Elements::Uniform(s) => Elements::Uniform(Box::new(clamp(*s, budget - 1))),
                Elements::Exact(shapes) => Elements::Exact(
                    shapes
                        .into_vec()
                        .into_iter()
                        .map(|s| s.map(|s| clamp(s, budget - 1)))
                        .collect(),
                ),
                Elements::Unknown => Elements::Unknown,
            }
        }
        clamp(self, MAX_SHAPE_DEPTH)
    }
}

impl fmt::Display for TypeShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeShape::Object(Some(class)) => write!(f, "OBJECT({class})"),
            TypeShape::Object(None) => write!(f, "Object"),
            TypeShape::List(e) => write_container(f, "List", e),
            TypeShape::Dict(e) => write_container(f, "Dict", e),
            other => write!(f, "{other:?}"),
        }
    }
}

/// Render `List` / `Dict` with element detail when any is tracked:
/// `List<Int, String>` (exact), `List<Int*>` (uniform), bare `List` otherwise.
fn write_container(f: &mut fmt::Formatter<'_>, name: &str, e: &Elements) -> fmt::Result {
    match e {
        Elements::Unknown => write!(f, "{name}"),
        Elements::Uniform(shape) => write!(f, "{name}<{shape}*>"),
        Elements::Exact(shapes) => {
            write!(f, "{name}<")?;
            for (i, shape) in shapes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match shape {
                    Some(s) => write!(f, "{s}")?,
                    None => write!(f, "?")?,
                }
            }
            write!(f, ">")
        }
    }
}

/// Join two shapes into a **single** shape, or `None` when no single shape
/// short of the union covers both (the caller keeps both as union members).
///
/// - Equal shapes join to themselves.
/// - Numeric-family shapes join per the tower: `Boolean ⊔ Int = Int`
///   (booleans are 0/1 ints); anything else in the family joins to the
///   abstract [`TypeShape::Numeric`].
/// - Two `List`s (or two `Dict`s) join element-wise.
/// - Two `Object`s keep the class only when it matches.
#[must_use]
pub fn shape_join(a: &TypeShape, b: &TypeShape) -> Option<TypeShape> {
    use TypeShape::{Boolean, Dict, Int, List, Numeric, Object};
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Boolean, Int) | (Int, Boolean) => Some(Int),
        _ if a.is_numeric_family() && b.is_numeric_family() => Some(Numeric),
        (List(ea), List(eb)) => Some(List(join_elements(ea, eb))),
        (Dict(ea), Dict(eb)) => Some(Dict(join_elements(ea, eb))),
        (Object(ca), Object(cb)) => Some(Object((ca == cb).then(|| ca.clone()).flatten())),
        _ => None,
    }
}

/// Join element facts of two same-kind containers.
///
/// `Unknown` (no facts) absorbs; equal-arity `Exact`s join pointwise (a
/// position keeps a shape only when a single joined shape covers both);
/// mismatched arities and `Uniform`s fall back to a uniform bound when every
/// tracked position admits one, else `Unknown`. Arity collapse on mismatch is
/// what makes loop-carried growth (`lappend` under a fixpoint) converge.
#[must_use]
pub fn join_elements(a: &Elements, b: &Elements) -> Elements {
    match (a, b) {
        (Elements::Unknown, _) | (_, Elements::Unknown) => Elements::Unknown,
        // An empty container contributes no elements to the *bound* but its
        // arity-0 fact must not vanish: joining `[list]` with `[list 1]`
        // keeps the element bound as a `Uniform` (the accumulator idiom —
        // `set acc [list]` then `lappend acc [Foo new]` — stays typed) while
        // dropping the other side's `Exact` arity, which the merged value
        // does not have (`llength` may be 0). Two empties keep arity 0.
        (Elements::Exact(ea), Elements::Exact(eb)) if ea.is_empty() && eb.is_empty() => {
            Elements::Exact(Box::from([]))
        }
        (Elements::Exact(e), other) | (other, Elements::Exact(e)) if e.is_empty() => {
            match other.uniform_shape() {
                Some(u) => Elements::Uniform(Box::new(u)),
                None => Elements::Unknown,
            }
        }
        (Elements::Exact(ea), Elements::Exact(eb)) if ea.len() == eb.len() => Elements::Exact(
            ea.iter()
                .zip(eb.iter())
                .map(|(x, y)| match (x, y) {
                    (Some(x), Some(y)) => shape_join(x, y),
                    _ => None,
                })
                .collect(),
        ),
        _ => match (a.uniform_shape(), b.uniform_shape()) {
            (Some(x), Some(y)) => match shape_join(&x, &y) {
                Some(joined) => Elements::Uniform(Box::new(joined)),
                None => Elements::Unknown,
            },
            _ => Elements::Unknown,
        },
    }
}

/// Lattice element kind, ordered bottom to top — the coarse classification
/// consumers switch on. `Known` is a singleton union; `Shimmered` is any
/// multi-member union (two *or more* differently-typed paths — the pair
/// restriction of the old model is gone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// Bottom — no information.
    Unknown,
    /// Exactly one possible shape.
    Known,
    /// Two or more possible shapes (the value shimmers between them).
    Shimmered,
    /// Top — too many types to track.
    Overdefined,
}

/// The bounded union of possible shapes at a lattice node.
type ShapeSet = BoundedSet<TypeShape, MAX_TYPE_UNION>;

/// A single element of the type lattice: `UNKNOWN`, a canonicalised bounded
/// union of [`TypeShape`]s, or `OVERDEFINED`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeLattice {
    repr: Repr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Repr {
    Unknown,
    /// Invariant: canonical (sorted, subsumption-reduced, numeric-collapsed),
    /// never empty.
    Union(ShapeSet),
    Overdefined,
}

impl TypeLattice {
    /// The bottom element — no type information.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            repr: Repr::Unknown,
        }
    }

    /// The top element — too many types to track.
    #[must_use]
    pub fn overdefined() -> Self {
        Self {
            repr: Repr::Overdefined,
        }
    }

    /// A single known shape.
    #[must_use]
    pub fn of_shape(shape: TypeShape) -> Self {
        Self {
            repr: Repr::Union(ShapeSet::singleton(shape.clamp_depth())),
        }
    }

    /// A single known coarse type (structure-free shape).
    #[must_use]
    pub fn of(t: TclType) -> Self {
        Self::of_shape(TypeShape::from_coarse(t))
    }

    /// A known `Object` type for a specific class.
    #[must_use]
    pub fn object_of(class_name: impl Into<String>) -> Self {
        Self::of_shape(TypeShape::Object(Some(class_name.into().into_boxed_str())))
    }

    /// A known `List` / `Dict` whose elements are all `OBJECT(element_class)`.
    /// `container` must be [`TclType::List`] or [`TclType::Dict`]; any other
    /// type is treated as a plain known type with no element facts (callers
    /// only ever pass the two container types).
    #[must_use]
    pub fn collection_of(container: TclType, element_class: impl Into<String>) -> Self {
        let elements = Elements::Uniform(Box::new(TypeShape::Object(Some(
            element_class.into().into_boxed_str(),
        ))));
        match container {
            TclType::List => Self::of_shape(TypeShape::List(elements)),
            TclType::Dict => Self::of_shape(TypeShape::Dict(elements)),
            other => Self::of(other),
        }
    }

    /// A union of the two coarse types (the old "shimmered pair" constructor;
    /// members are stored canonically, so argument order is immaterial).
    #[must_use]
    pub fn shimmered(from: TclType, to: TclType) -> Self {
        Self::union_of([TypeShape::from_coarse(from), TypeShape::from_coarse(to)])
    }

    /// A union of the given shapes — canonicalised, numeric-collapsed, and
    /// widened to `OVERDEFINED` past [`MAX_TYPE_UNION`] distinct members.
    #[must_use]
    pub fn union_of(shapes: impl IntoIterator<Item = TypeShape>) -> Self {
        let mut members: Vec<TypeShape> = Vec::new();
        for shape in shapes {
            if !merge_member(&mut members, shape.clamp_depth()) {
                return Self::overdefined();
            }
        }
        match ShapeSet::from_iter_bounded(members) {
            Some(mut set) if !set.is_empty() => {
                set.canonicalise();
                Self {
                    repr: Repr::Union(set),
                }
            }
            Some(_) => Self::unknown(),
            None => Self::overdefined(),
        }
    }

    /// The coarse lattice classification.
    #[must_use]
    pub fn kind(&self) -> TypeKind {
        match &self.repr {
            Repr::Unknown => TypeKind::Unknown,
            Repr::Union(set) if set.len() == 1 => TypeKind::Known,
            Repr::Union(_) => TypeKind::Shimmered,
            Repr::Overdefined => TypeKind::Overdefined,
        }
    }

    /// The union members, in canonical order (empty for `UNKNOWN` /
    /// `OVERDEFINED`).
    #[must_use]
    pub fn shapes(&self) -> &[TypeShape] {
        match &self.repr {
            Repr::Union(set) => set.as_slice(),
            Repr::Unknown | Repr::Overdefined => &[],
        }
    }

    /// The single shape of a `Known` node.
    #[must_use]
    pub fn single_shape(&self) -> Option<&TypeShape> {
        match self.shapes() {
            [one] => Some(one),
            _ => None,
        }
    }

    /// The single *coarse* type of a `Known` node — the accessor most
    /// consumers key their type checks on.
    #[must_use]
    pub fn tcl_type(&self) -> Option<TclType> {
        self.single_shape().map(TypeShape::coarse)
    }

    /// The canonical coarse pair of a two-member union (the old
    /// `SHIMMERED(from, to)` reading; `from` sorts before `to`). `None` for
    /// any other node, including 3+-member unions — walk [`Self::shapes`]
    /// there.
    #[must_use]
    pub fn shimmer_pair(&self) -> Option<(TclType, TclType)> {
        match self.shapes() {
            [a, b] => Some((a.coarse(), b.coarse())),
            _ => None,
        }
    }

    /// The canonical outermost coarse pair of *any* multi-member union —
    /// `shimmer_pair` generalised to 3+-member unions for consumers whose
    /// report is inherently a two-type claim (the S102 oscillation message).
    #[must_use]
    pub fn shimmer_extremes(&self) -> Option<(TclType, TclType)> {
        match self.shapes() {
            [] | [_] => None,
            many => Some((many[0].coarse(), many[many.len() - 1].coarse())),
        }
    }

    /// The class of a `Known` object node.
    #[must_use]
    pub fn class_name(&self) -> Option<&str> {
        match self.single_shape()? {
            TypeShape::Object(class) => class.as_deref(),
            _ => None,
        }
    }

    /// Element facts of a `Known` `List` / `Dict` node.
    #[must_use]
    pub fn elements(&self) -> Option<&Elements> {
        match self.single_shape()? {
            TypeShape::List(e) | TypeShape::Dict(e) => Some(e),
            _ => None,
        }
    }

    /// The object class of this collection's elements, when it is a
    /// `List`/`Dict` known to be homogeneous in `OBJECT(class)` — the signal a
    /// `dict get` / `lindex` retrieval reads to type its result. Both element
    /// carriers qualify: a `Uniform` object bound, and an `Exact` whose every
    /// position holds the same class.
    #[must_use]
    pub fn element_class(&self) -> Option<&str> {
        match self.elements()? {
            Elements::Uniform(shape) => match shape.as_ref() {
                TypeShape::Object(Some(class)) => Some(class),
                _ => None,
            },
            Elements::Exact(shapes) => {
                let TypeShape::Object(Some(class)) = shapes.first()?.as_ref()? else {
                    return None;
                };
                shapes
                    .iter()
                    .all(|s| matches!(s, Some(TypeShape::Object(Some(c))) if c == class))
                    .then_some(&**class)
            }
            Elements::Unknown => None,
        }
    }
}

impl fmt::Display for TypeLattice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Unknown => write!(f, "UNKNOWN"),
            Repr::Overdefined => write!(f, "OVERDEFINED"),
            Repr::Union(set) => match set.as_slice() {
                [one] => write!(f, "{one}"),
                many => {
                    // The two-member rendering keeps the historical
                    // `SHIMMERED(a, b)` spelling; wider unions enumerate.
                    write!(f, "SHIMMERED(")?;
                    for (i, shape) in many.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{shape}")?;
                    }
                    write!(f, ")")
                }
            },
        }
    }
}

/// Merge `shape` into `members`, applying subsumption: numeric-family members
/// collapse per [`shape_join`]'s tower rules, and same-constructor containers
/// / objects join structurally so a union never holds two `List` members.
/// Returns `false` when the union must widen (never happens today — the
/// alphabet is small — but guards the cap invariant).
fn merge_member(members: &mut Vec<TypeShape>, shape: TypeShape) -> bool {
    let mut incoming = shape;
    let mut i = 0;
    while i < members.len() {
        if let Some(joined) = shape_join(&members[i], &incoming) {
            members.remove(i);
            incoming = joined;
            // Restart: the joined shape may now subsume other members
            // (e.g. Int absorbing Boolean then meeting Double → Numeric).
            i = 0;
        } else {
            i += 1;
        }
    }
    members.push(incoming);
    members.len() <= MAX_TYPE_UNION
}

/// Compute the join (least upper bound) of two type lattice elements.
#[must_use]
pub fn type_join(a: &TypeLattice, b: &TypeLattice) -> TypeLattice {
    match (&a.repr, &b.repr) {
        (Repr::Unknown, _) => b.clone(),
        (_, Repr::Unknown) => a.clone(),
        (Repr::Overdefined, _) | (_, Repr::Overdefined) => TypeLattice::overdefined(),
        (Repr::Union(sa), Repr::Union(sb)) => {
            TypeLattice::union_of(sa.iter().chain(sb.iter()).cloned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_bottom() {
        let u = TypeLattice::unknown();
        assert_eq!(u.kind(), TypeKind::Unknown);
        let k = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&u, &k), k);
        assert_eq!(type_join(&k, &u), k);
    }

    #[test]
    fn overdefined_is_top() {
        let o = TypeLattice::overdefined();
        let k = TypeLattice::of(TclType::Int);
        assert_eq!(type_join(&o, &k).kind(), TypeKind::Overdefined);
        assert_eq!(type_join(&k, &o).kind(), TypeKind::Overdefined);
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
        assert_eq!(joined.kind(), TypeKind::Shimmered);
        assert_eq!(joined.shimmer_pair(), Some((TclType::Int, TclType::List)));
    }

    /// The un-masking at the heart of P2: a three-way merge of incompatible
    /// types stays a tracked union instead of collapsing to `OVERDEFINED`
    /// (the old pair model's documented lossy point).
    #[test]
    fn three_way_merge_stays_tracked_union() {
        let ab = type_join(
            &TypeLattice::of(TclType::Int),
            &TypeLattice::of(TclType::List),
        );
        let abc = type_join(&ab, &TypeLattice::of(TclType::Dict));
        assert_eq!(abc.kind(), TypeKind::Shimmered);
        assert_eq!(abc.shapes().len(), 3, "{abc}");
        // And a fourth non-numeric member still tracks.
        let abcd = type_join(&abc, &TypeLattice::of(TclType::ByteArray));
        assert_eq!(abcd.shapes().len(), 4, "{abcd}");
    }

    /// Two disjoint shimmer pairs merge member-wise: the numeric members
    /// collapse (`Int ⊔ Double = Numeric`) and the containers survive —
    /// previously OVERDEFINED.
    #[test]
    fn different_shimmered_pairs_merge_memberwise() {
        let a = TypeLattice::shimmered(TclType::Int, TclType::List);
        let b = TypeLattice::shimmered(TclType::Double, TclType::Dict);
        let joined = type_join(&a, &b);
        assert_eq!(joined.kind(), TypeKind::Shimmered);
        assert_eq!(
            joined,
            TypeLattice::union_of([
                TypeShape::Numeric,
                TypeShape::List(Elements::Unknown),
                TypeShape::Dict(Elements::Unknown)
            ])
        );
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

    /// A `Known(Int)` meeting `SHIMMERED(Numeric, String)` must stay
    /// SHIMMERED: `Int` is subsumed by the `Numeric` member.
    #[test]
    fn known_int_matches_numeric_shimmer_side() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::String);
        let known = TypeLattice::of(TclType::Int);
        let joined = type_join(&known, &shimmer);
        assert_eq!(joined, shimmer);
        assert_eq!(type_join(&shimmer, &known), joined);
    }

    /// `Boolean` (an int-family type) also folds into a `Numeric` member.
    #[test]
    fn known_boolean_matches_numeric_shimmer_side() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::List);
        let known = TypeLattice::of(TclType::Boolean);
        assert_eq!(type_join(&known, &shimmer), shimmer);
    }

    /// A `Known(Double)` meeting `SHIMMERED(Int, String)` widens the numeric
    /// member to `Numeric` — the union stays a sound upper bound.
    #[test]
    fn known_double_widens_int_shimmer_side_to_numeric() {
        let shimmer = TypeLattice::shimmered(TclType::Int, TclType::String);
        let known = TypeLattice::of(TclType::Double);
        assert_eq!(
            type_join(&known, &shimmer),
            TypeLattice::shimmered(TclType::Numeric, TclType::String)
        );
    }

    /// A non-numeric `Known` type meeting a 2-union becomes a 3-union —
    /// tracked, where the pair model degraded to OVERDEFINED.
    #[test]
    fn known_non_numeric_no_match_becomes_three_union() {
        let shimmer = TypeLattice::shimmered(TclType::Numeric, TclType::List);
        let known = TypeLattice::of(TclType::Dict);
        let joined = type_join(&known, &shimmer);
        assert_eq!(joined.kind(), TypeKind::Shimmered);
        assert_eq!(joined.shapes().len(), 3);
    }

    /// An exact match on a non-numeric member is unchanged:
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
        assert_eq!(type_join(&a, &b), TypeLattice::of(TclType::Object));
    }

    /// Element facts: same-arity exact lists join pointwise; mismatched
    /// arities fall back to a uniform bound; a `Unknown` member absorbs.
    #[test]
    fn list_element_joins() {
        let exact2 = |a: TypeShape, b: TypeShape| {
            TypeShape::List(Elements::Exact(vec![Some(a), Some(b)].into_boxed_slice()))
        };
        // Pointwise: (Int, String) ⊔ (Boolean, String) = (Int, String).
        let j = type_join(
            &TypeLattice::of_shape(exact2(TypeShape::Int, TypeShape::String)),
            &TypeLattice::of_shape(exact2(TypeShape::Boolean, TypeShape::String)),
        );
        assert_eq!(
            j,
            TypeLattice::of_shape(exact2(TypeShape::Int, TypeShape::String))
        );
        // Arity mismatch → uniform bound when one exists.
        let j2 = type_join(
            &TypeLattice::of_shape(exact2(TypeShape::Int, TypeShape::Int)),
            &TypeLattice::of_shape(TypeShape::List(Elements::Exact(
                vec![Some(TypeShape::Int)].into_boxed_slice(),
            ))),
        );
        assert_eq!(
            j2,
            TypeLattice::of_shape(TypeShape::List(Elements::Uniform(Box::new(TypeShape::Int))))
        );
        // Unknown absorbs.
        let j3 = type_join(
            &TypeLattice::of_shape(exact2(TypeShape::Int, TypeShape::Int)),
            &TypeLattice::of(TclType::List),
        );
        assert_eq!(j3, TypeLattice::of(TclType::List));
    }

    /// Homogeneous object collections keep their element class through joins
    /// (the old `element_class` behaviour, now via `Elements`).
    #[test]
    fn object_collection_element_class_survives_and_widens() {
        let a = TypeLattice::collection_of(TclType::Dict, "Foo");
        assert_eq!(a.element_class(), Some("Foo"));
        assert_eq!(type_join(&a, &a).element_class(), Some("Foo"));
        let b = TypeLattice::collection_of(TclType::Dict, "Bar");
        let joined = type_join(&a, &b);
        // Classes differ → the container survives, the class widens away.
        assert_eq!(joined.tcl_type(), Some(TclType::Dict));
        assert_eq!(joined.element_class(), None);
    }

    /// Exact arity is a fact even with no per-position shapes.
    #[test]
    fn exact_arity_without_shapes() {
        let l = TypeShape::List(Elements::Exact(vec![None, None].into_boxed_slice()));
        let tl = TypeLattice::of_shape(l);
        assert_eq!(tl.elements().and_then(Elements::known_len), Some(2));
    }

    /// Depth clamping bounds nested structure (fixpoint termination): the
    /// shape survives but its nesting is cut at `MAX_SHAPE_DEPTH`.
    #[test]
    fn depth_clamp_bounds_nesting() {
        fn max_depth(s: &TypeShape) -> usize {
            match s {
                TypeShape::List(Elements::Uniform(inner))
                | TypeShape::Dict(Elements::Uniform(inner)) => 1 + max_depth(inner),
                _ => 1,
            }
        }
        let mut shape = TypeShape::Int;
        for _ in 0..10 {
            shape = TypeShape::List(Elements::Uniform(Box::new(shape)));
        }
        let tl = TypeLattice::of_shape(shape);
        assert!(max_depth(tl.single_shape().unwrap()) <= MAX_SHAPE_DEPTH);
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
        assert_eq!(
            TypeLattice::collection_of(TclType::List, "C").to_string(),
            "List<OBJECT(C)*>"
        );
        assert_eq!(
            TypeLattice::of_shape(TypeShape::List(Elements::Exact(
                vec![Some(TypeShape::Int), None].into_boxed_slice()
            )))
            .to_string(),
            "List<Int, ?>"
        );
    }
}
