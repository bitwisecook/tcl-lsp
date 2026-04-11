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
        }
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
    if Some(kt) == shimmer.tcl_type || Some(kt) == shimmer.from_type {
        return shimmer.clone();
    }
    TypeLattice::overdefined()
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
