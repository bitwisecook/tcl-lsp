//! Unified variable **Place** model — the single identity for what a Tcl
//! read/write touches.
//!
//! A *place* (an lvalue) is what a variable reference denotes: a scalar, a
//! specific array element, a whole array, a dict path, an OO instance
//! variable, an upvar alias edge, or — when it cannot be pinned down
//! statically — the top value [`PlaceKind::Unknown`].  Today variable identity
//! in the dataflow consumers is a bare `(name, version)` SSA string that folds
//! `a(k)` / `a(j)` / `$a` together and makes dynamic names opaque; this module
//! replaces that with a structured place plus two relations:
//!
//! * [`overlap`] — *does a read of `q` observe a write of `p`?*  The whole
//!   point: `a(foo)` does not clobber `a(bar)`, but the whole array overlaps
//!   every element, and [`PlaceKind::Unknown`] / dynamic indices overlap
//!   everything.  The relation **over-approximates** real aliasing (any
//!   uncertainty ⇒ `true`), so consumers built on it only ever *suppress* a
//!   finding, never *falsely flag*.
//! * [`places_read_to_form`] — the scalars read to *compute* a dynamic index,
//!   dict key, or runtime variable name (`$i` in `a($i)`, `X` in `set $X`).
//!
//! The `overlap` relation separates a dynamic *index* on a known base from a
//! dynamic *alias/name*.  The resolution layer ([`crate::var_resolve`]) maps a
//! syntactic reference + scope context to a canonical place.

/// Sentinel namespace marking a procedure-*local* scalar (frame-relative), as
/// opposed to a global / namespace variable which carries its real `::`-FQ
/// namespace.  Keeps the deliberate local-vs-global distinction the dataflow
/// guards rely on (`name.starts_with("::")` → `place.ns != LOCAL_NS`).
pub const LOCAL_NS: &str = "<local>";

/// What kind of storage location a [`Place`] denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// A scalar variable (`$x`); `ns` distinguishes local vs global/ns.
    Scalar,
    /// One element of an array (`a(k)`) — carries an [`Index`].
    ArrayElem,
    /// A whole array touched as a unit (`array set` / `array get` / `$a`).
    ArrayWhole,
    /// A dict accessed at a key path (`dict get $d a b`) — carries `keys`.
    DictPath,
    /// A `TclOO` instance variable (`owner` = class/object FQ name).
    InstanceVar,
    /// A local alias to a caller/namespace var (`upvar`); `owner` = target.
    UpvarAlias,
    /// Top — a place that cannot be pinned down (dynamic name, opaque).
    Unknown,
}

/// How an array index / dict key in the place sub-lattice is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexKind {
    /// A statically-known index/key text.
    Literal,
    /// A computed index/key (`a($i)`) — its `read_places` are read.
    Dynamic,
    /// An unconstrained index — whole-collection or unresolvable.
    Any,
}

/// An array index or dict key in the place sub-lattice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Index {
    /// How the index/key is known (literal text, computed, or unconstrained).
    pub kind: IndexKind,
    /// The literal text when [`IndexKind::Literal`]; empty otherwise.
    pub value: String,
    /// Vars read to form a [`IndexKind::Dynamic`] index/key.
    pub read_places: Vec<Place>,
}

impl Index {
    /// A statically-known index/key.
    #[must_use]
    pub fn literal(value: impl Into<String>) -> Self {
        Self {
            kind: IndexKind::Literal,
            value: value.into(),
            read_places: Vec::new(),
        }
    }

    /// A computed index/key carrying the scalars read to form it.
    #[must_use]
    pub fn dynamic(read_places: Vec<Place>) -> Self {
        Self {
            kind: IndexKind::Dynamic,
            value: String::new(),
            read_places,
        }
    }

    /// An unconstrained index (`ANY_INDEX`).
    #[must_use]
    pub fn any() -> Self {
        Self {
            kind: IndexKind::Any,
            value: String::new(),
            read_places: Vec::new(),
        }
    }
}

/// A storage location a read/write touches.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    /// Which kind of storage location this place denotes.
    pub kind: PlaceKind,
    /// FQ namespace; [`LOCAL_NS`] for proc-local scalars.
    pub ns: String,
    /// Base variable name (bare local or ns-tail).
    pub name: String,
    /// `ARRAY_ELEM` index.
    pub index: Option<Index>,
    /// `DICT_PATH` key path.
    pub keys: Vec<Index>,
    /// `INSTANCE_VAR` class/obj FQ; `UPVAR_ALIAS` caller target.
    pub owner: String,
    /// Under a `trace` — externally observable.
    pub observed: bool,
    /// Alias/level/name could not be resolved precisely.
    pub dynamic: bool,
    /// Vars read to form a dynamic name (`X` in `set $X`).
    pub name_reads: Vec<Place>,
}

impl Place {
    fn bare(kind: PlaceKind) -> Self {
        Self {
            kind,
            ns: LOCAL_NS.to_owned(),
            name: String::new(),
            index: None,
            keys: Vec::new(),
            owner: String::new(),
            observed: false,
            dynamic: false,
            name_reads: Vec::new(),
        }
    }

    /// True for a global / namespace variable (not a frame-local).
    #[must_use]
    pub fn is_global(&self) -> bool {
        matches!(
            self.kind,
            PlaceKind::Scalar | PlaceKind::ArrayElem | PlaceKind::ArrayWhole
        ) && self.ns != LOCAL_NS
    }

    /// The whole-variable identity, dropping element index / dict keys.
    ///
    /// `a(k)` → `ARRAY_WHOLE a`; `dict d a b` → `ARRAY_WHOLE`-style root; a
    /// scalar is its own base.  This is the join point of [`overlap`].
    #[must_use]
    pub fn base(&self) -> Self {
        match self.kind {
            PlaceKind::ArrayElem | PlaceKind::DictPath => {
                array_whole(&self.name, &self.ns, self.observed)
            }
            _ => self.clone(),
        }
    }
}

// Constructors

/// A scalar place.
#[must_use]
pub fn scalar(name: impl Into<String>, ns: impl Into<String>, observed: bool) -> Place {
    Place {
        name: name.into(),
        ns: ns.into(),
        observed,
        ..Place::bare(PlaceKind::Scalar)
    }
}

/// An array-element place.
#[must_use]
pub fn array_elem(
    name: impl Into<String>,
    index: Index,
    ns: impl Into<String>,
    observed: bool,
) -> Place {
    Place {
        name: name.into(),
        ns: ns.into(),
        index: Some(index),
        observed,
        ..Place::bare(PlaceKind::ArrayElem)
    }
}

/// A whole-array place.
#[must_use]
pub fn array_whole(name: impl Into<String>, ns: impl Into<String>, observed: bool) -> Place {
    Place {
        name: name.into(),
        ns: ns.into(),
        observed,
        ..Place::bare(PlaceKind::ArrayWhole)
    }
}

/// A dict-path place.
#[must_use]
pub fn dict_path(name: impl Into<String>, keys: Vec<Index>, ns: impl Into<String>) -> Place {
    Place {
        name: name.into(),
        ns: ns.into(),
        keys,
        ..Place::bare(PlaceKind::DictPath)
    }
}

/// A `TclOO` instance-variable place.
#[must_use]
pub fn instance_var(name: impl Into<String>, owner: impl Into<String>, observed: bool) -> Place {
    Place {
        name: name.into(),
        owner: owner.into(),
        observed,
        ..Place::bare(PlaceKind::InstanceVar)
    }
}

/// An `upvar` alias place.  `dynamic` is forced on when `owner` is empty
/// (the caller target could not be resolved).
#[must_use]
pub fn upvar_alias(name: impl Into<String>, owner: impl Into<String>, dynamic: bool) -> Place {
    let owner = owner.into();
    let dynamic = dynamic || owner.is_empty();
    Place {
        name: name.into(),
        owner,
        dynamic,
        ..Place::bare(PlaceKind::UpvarAlias)
    }
}

/// The top place, carrying the scalars read to form its (dynamic) name.
#[must_use]
pub fn unknown(name_reads: Vec<Place>) -> Place {
    Place {
        dynamic: true,
        name_reads,
        ..Place::bare(PlaceKind::Unknown)
    }
}

/// The canonical top place with no name reads.
#[must_use]
pub fn unknown_top() -> Place {
    Place {
        dynamic: true,
        ..Place::bare(PlaceKind::Unknown)
    }
}

// Relations

/// Could indices/keys *a* and *b* denote the same slot?  Over-approximates:
/// `ANY` or `DYNAMIC` overlaps anything; two literals overlap iff equal text.
fn index_overlap(a: &Index, b: &Index) -> bool {
    if a.kind == IndexKind::Any || b.kind == IndexKind::Any {
        return true;
    }
    if a.kind == IndexKind::Dynamic || b.kind == IndexKind::Dynamic {
        return true;
    }
    a.value == b.value
}

/// Dict key paths overlap iff one is a prefix of the other under
/// [`index_overlap`] (a write to `d a b` is observed by a read of `d a`;
/// `d x` is not).
fn keys_overlap(a: &[Index], b: &[Index]) -> bool {
    // zip stops at the shorter path → the shorter is a prefix of the longer.
    a.iter().zip(b.iter()).all(|(ia, ib)| index_overlap(ia, ib))
}

/// Does a read of *q* observe a write of *p* (and vice-versa)?
///
/// Over-approximating and symmetric: [`PlaceKind::Unknown`],
/// [`PlaceKind::UpvarAlias`], dynamic aliases, and `DYNAMIC`/`ANY` indices all
/// return `true` — so a consumer can only ever decide "these may be the same"
/// conservatively.
#[must_use]
pub fn overlap(p: &Place, q: &Place) -> bool {
    // Top / unresolved alias edges may touch anything observable.
    if p.kind == PlaceKind::Unknown || q.kind == PlaceKind::Unknown {
        return true;
    }
    if p.kind == PlaceKind::UpvarAlias || q.kind == PlaceKind::UpvarAlias {
        // Two non-dynamic upvar aliases name a *resolved* caller var via their
        // `owner`; they are the same storage iff the owners match — regardless
        // of the local alias name.  Otherwise conservatively yes.
        if p.kind == q.kind && !p.dynamic && !q.dynamic {
            return p.owner == q.owner;
        }
        return true;
    }
    if p.dynamic || q.dynamic {
        // A `dynamic` place is one whose alias target / frame could not be
        // resolved, but whose *local base name* is still known (so it is NOT a
        // wildcard over all names — that is `Unknown`, handled above).
        //
        //  * two dynamic places of *different* names may still alias (two
        //    upvar aliases can point at the same caller storage) ⇒ conservative;
        //  * a dynamic place vs a *concrete* place of a *different* base name
        //    cannot alias (the dead-store precision win);
        //  * the *same* base name falls through to the structural comparison.
        if p.ns != q.ns || p.name != q.name {
            return p.dynamic && q.dynamic;
        }
        // same (ns, name): fall through to the structural element/kind logic.
    }

    if p.kind == PlaceKind::InstanceVar || q.kind == PlaceKind::InstanceVar {
        return p.kind == q.kind && p.owner == q.owner && p.name == q.name;
    }

    if p.kind == PlaceKind::DictPath || q.kind == PlaceKind::DictPath {
        if p.kind == q.kind {
            return p.ns == q.ns && p.name == q.name && keys_overlap(&p.keys, &q.keys);
        }
        // A dict and its whole-array base overlap; a dict and a scalar/elem of
        // a different name do not.
        let (dpath, other) = if p.kind == PlaceKind::DictPath {
            (p, q)
        } else {
            (q, p)
        };
        if other.kind == PlaceKind::ArrayWhole {
            return other.ns == dpath.ns && other.name == dpath.name;
        }
        return false;
    }

    // Scalar / array family — same namespace + base name required.
    if p.ns != q.ns || p.name != q.name {
        return false;
    }
    if p.kind == PlaceKind::Scalar && q.kind == PlaceKind::Scalar {
        return true;
    }
    if p.kind == PlaceKind::ArrayWhole || q.kind == PlaceKind::ArrayWhole {
        // whole overlaps whole and any element of the same array.
        return true;
    }
    if p.kind == PlaceKind::ArrayElem && q.kind == PlaceKind::ArrayElem {
        // Both are ARRAY_ELEM here, so both carry an index.
        return match (&p.index, &q.index) {
            (Some(pi), Some(qi)) => index_overlap(pi, qi),
            // Defensive: a malformed element with no index over-approximates.
            _ => true,
        };
    }
    // SCALAR vs ARRAY_ELEM of the same name: distinct kinds, disjoint (a scalar
    // and an array are different variables in Tcl).
    false
}

/// Scalars read to *compute* this place's identity — dynamic array/dict
/// indices and runtime variable names.  These are genuine reads at the access
/// site (`$i` in `a($i)`, `X` in `set $X v`).  Returned de-duplicated,
/// first-seen order.
#[must_use]
pub fn places_read_to_form(p: &Place) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    let push = |pl: &Place, out: &mut Vec<Place>| {
        if !out.contains(pl) {
            out.push(pl.clone());
        }
    };
    for pl in &p.name_reads {
        push(pl, &mut out);
    }
    if let Some(idx) = &p.index {
        for pl in &idx.read_places {
            push(pl, &mut out);
        }
    }
    for k in &p.keys {
        for pl in &k.read_places {
            push(pl, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_scalar(name: &str) -> Place {
        scalar(name, LOCAL_NS, false)
    }

    // core scalar / array overlap

    #[test]
    fn scalar_overlap_basics() {
        assert!(overlap(&local_scalar("x"), &local_scalar("x")));
        // different name → disjoint
        assert!(!overlap(&local_scalar("x"), &local_scalar("y")));
        // local vs global of the same name → different ns → disjoint
        assert!(!overlap(&local_scalar("x"), &scalar("x", "::", false)));
    }

    #[test]
    fn array_elements_are_disjoint_by_literal_index() {
        // The headline precision: a(k) and a(j) do NOT overlap, so a write to
        // one is not observed by a read of the other (suppresses the W220 FP).
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        let aj = array_elem("a", Index::literal("j"), LOCAL_NS, false);
        assert!(overlap(&ak, &ak));
        assert!(!overlap(&ak, &aj));
        // but a different array entirely is also disjoint
        let bk = array_elem("b", Index::literal("k"), LOCAL_NS, false);
        assert!(!overlap(&ak, &bk));
    }

    #[test]
    fn whole_array_overlaps_every_element() {
        let whole = array_whole("a", LOCAL_NS, false);
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        assert!(overlap(&whole, &ak));
        assert!(overlap(&ak, &whole));
        // whole arrays of different names stay disjoint
        assert!(!overlap(&whole, &array_whole("b", LOCAL_NS, false)));
    }

    #[test]
    fn dynamic_index_overlaps_any_sibling_element() {
        // a($i) may touch any element of `a` …
        let dyn_elem = array_elem(
            "a",
            Index::dynamic(vec![local_scalar("i")]),
            LOCAL_NS,
            false,
        );
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        assert!(overlap(&dyn_elem, &ak));
        // … but not an element of a *different* array.
        let bk = array_elem("b", Index::literal("k"), LOCAL_NS, false);
        assert!(!overlap(&dyn_elem, &bk));
    }

    #[test]
    fn scalar_and_array_element_of_same_name_are_disjoint() {
        let s = local_scalar("a");
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        assert!(!overlap(&s, &ak));
    }

    // 8E refinement: dynamic *index* ≠ dynamic *alias*

    #[test]
    fn dynamic_alias_does_not_observe_differently_named_local() {
        // `upvar 1 $x date` makes `date` a dynamic-aliased array; a write to a
        // differently-named local `date2(ERA)` is NOT observed by reads of
        // `date(...)`.  This is the gregorian.tcl precision case from the
        // phase8 design doc.
        let mut date2 = array_elem("date2", Index::literal("ERA"), LOCAL_NS, false);
        let mut date_dyn = array_elem(
            "date",
            Index::dynamic(vec![local_scalar("i")]),
            LOCAL_NS,
            false,
        );
        date_dyn.dynamic = true; // aliases an unknown caller var
        // date2 is a concrete local; only the alias side is dynamic.
        assert!(!overlap(&date2, &date_dyn));
        // Two dynamic aliases of different names stay conservative (True):
        // they could point at the same caller storage.
        date2.dynamic = true;
        assert!(overlap(&date2, &date_dyn));
    }

    #[test]
    fn unknown_name_observes_everything() {
        // `set $X v` → UNKNOWN: could be any variable, so it overlaps a
        // concrete dynamic-aliased place of any name.
        let mut date_dyn = array_elem("date", Index::literal("k"), LOCAL_NS, false);
        date_dyn.dynamic = true;
        assert!(overlap(&unknown(vec![local_scalar("X")]), &date_dyn));
        assert!(overlap(&date_dyn, &unknown_top()));
    }

    // upvar aliases

    #[test]
    fn upvar_aliases_compare_by_owner() {
        // Same caller target ⇒ must-alias even with different local names.
        let a = upvar_alias("a", "::caller::x", false);
        let b = upvar_alias("b", "::caller::x", false);
        let c = upvar_alias("c", "::caller::y", false);
        assert!(overlap(&a, &b));
        assert!(!overlap(&a, &c));
        // A dynamic (unresolved) upvar alias is conservative against anything.
        let dyn_alias = upvar_alias("d", "", false); // empty owner ⇒ dynamic
        assert!(dyn_alias.dynamic);
        assert!(overlap(&dyn_alias, &c));
        assert!(overlap(&dyn_alias, &local_scalar("z")));
    }

    // instance vars + dict paths

    #[test]
    fn instance_vars_compare_by_owner_and_name() {
        let a1 = instance_var("v", "::C", false);
        let a2 = instance_var("v", "::C", false);
        let b = instance_var("v", "::D", false);
        let c = instance_var("w", "::C", false);
        assert!(overlap(&a1, &a2));
        assert!(!overlap(&a1, &b));
        assert!(!overlap(&a1, &c));
    }

    #[test]
    fn dict_paths_overlap_on_prefix() {
        let d_ab = dict_path(
            "d",
            vec![Index::literal("a"), Index::literal("b")],
            LOCAL_NS,
        );
        let d_a = dict_path("d", vec![Index::literal("a")], LOCAL_NS);
        let d_x = dict_path("d", vec![Index::literal("x")], LOCAL_NS);
        // a read of `d a` observes a write of `d a b` (prefix) …
        assert!(overlap(&d_ab, &d_a));
        // … but `d x` is a different path.
        assert!(!overlap(&d_ab, &d_x));
        // A dict and its whole-array base overlap; a differently-named one not.
        assert!(overlap(&d_a, &array_whole("d", LOCAL_NS, false)));
        assert!(!overlap(&d_a, &array_whole("e", LOCAL_NS, false)));
        // dict vs scalar of a different name → disjoint.
        assert!(!overlap(&d_a, &local_scalar("e")));
    }

    #[test]
    fn unknown_overlaps_anything() {
        assert!(overlap(&unknown_top(), &local_scalar("x")));
        assert!(overlap(&local_scalar("x"), &unknown_top()));
        assert!(overlap(&unknown_top(), &unknown_top()));
    }

    #[test]
    fn overlap_is_symmetric_on_a_sample_matrix() {
        let samples = vec![
            local_scalar("x"),
            scalar("x", "::", false),
            array_elem("a", Index::literal("k"), LOCAL_NS, false),
            array_elem("a", Index::literal("j"), LOCAL_NS, false),
            array_elem(
                "a",
                Index::dynamic(vec![local_scalar("i")]),
                LOCAL_NS,
                false,
            ),
            array_whole("a", LOCAL_NS, false),
            dict_path("d", vec![Index::literal("a")], LOCAL_NS),
            instance_var("v", "::C", false),
            upvar_alias("u", "::caller::x", false),
            unknown_top(),
        ];
        for p in &samples {
            for q in &samples {
                assert_eq!(
                    overlap(p, q),
                    overlap(q, p),
                    "overlap must be symmetric for {p:?} / {q:?}",
                );
            }
        }
    }

    // helpers

    #[test]
    fn base_drops_index_and_keys() {
        let ak = array_elem("a", Index::literal("k"), "::ns", true);
        let base = ak.base();
        assert_eq!(base.kind, PlaceKind::ArrayWhole);
        assert_eq!(base.name, "a");
        assert_eq!(base.ns, "::ns");
        assert!(base.observed);
        assert!(base.index.is_none());
        // a scalar is its own base
        let s = local_scalar("x");
        assert_eq!(s.base(), s);
    }

    #[test]
    fn is_global_distinguishes_local_from_namespaced() {
        assert!(!local_scalar("x").is_global());
        assert!(scalar("g", "::", false).is_global());
        assert!(scalar("v", "::ns", false).is_global());
        // upvar alias / instance var / unknown are not "global" scalars
        assert!(!upvar_alias("u", "::x", false).is_global());
        assert!(!unknown_top().is_global());
    }

    #[test]
    fn places_read_to_form_collects_index_and_name_reads() {
        // a($i) reads `i`.
        let elem = array_elem(
            "a",
            Index::dynamic(vec![local_scalar("i")]),
            LOCAL_NS,
            false,
        );
        assert_eq!(places_read_to_form(&elem), vec![local_scalar("i")]);
        // set $X → unknown reading `X`.
        assert_eq!(
            places_read_to_form(&unknown(vec![local_scalar("X")])),
            vec![local_scalar("X")]
        );
        // dedup: a($i)($i) style dict path reading `i` twice → one entry.
        let dp = dict_path(
            "d",
            vec![
                Index::dynamic(vec![local_scalar("i")]),
                Index::dynamic(vec![local_scalar("i")]),
            ],
            LOCAL_NS,
        );
        assert_eq!(places_read_to_form(&dp), vec![local_scalar("i")]);
        // a literal-index element reads nothing.
        let lit = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        assert!(places_read_to_form(&lit).is_empty());
    }
}
