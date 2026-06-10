"""Unified variable **Place** model — the single identity for what a Tcl
read/write touches (Phase 8).

A *place* (an lvalue) is what a variable reference denotes: a scalar, a
specific array element, a whole array, a dict path, an OO instance variable,
an upvar alias edge, or — when it cannot be pinned down statically — the top
value ``UNKNOWN``.  Today variable identity is a bare ``(name, version)`` SSA
string that folds ``a(k)``/``a(j)``/``$a`` together and makes dynamic names
opaque; this module replaces that with a structured place plus two relations:

* :func:`overlap` — *does a read of ``q`` observe a write of ``p``?*  The whole
  point: ``a(foo)`` does not clobber ``a(bar)``, but the whole array overlaps
  every element, and ``UNKNOWN`` / dynamic indices overlap everything.  The
  relation **over-approximates** real aliasing (any uncertainty ⇒ ``True``), so
  consumers built on it only ever *suppress* a finding, never *falsely flag*.

* :func:`places_read_to_form` — the scalars read to *compute* a dynamic index,
  dict key, or runtime variable name (``$i`` in ``a($i)``, ``X`` in ``set $X``).

This promotes :class:`compiler.memory_ssa.MemoryLocation`; the resolution layer
(:mod:`compiler.var_resolve`) maps a syntactic reference + scope context to a
canonical place.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

# Sentinel namespace marking a procedure-*local* scalar (frame-relative), as
# opposed to a global / namespace variable which carries its real ``::``-FQ
# namespace.  Keeps the deliberate local-vs-global distinction the dataflow
# guards rely on (``name.startswith("::")`` → ``place.ns != LOCAL_NS``).
LOCAL_NS = "<local>"


class PlaceKind(Enum):
    SCALAR = auto()
    """A scalar variable (``$x``); ``ns`` distinguishes local vs global/ns."""
    ARRAY_ELEM = auto()
    """One element of an array (``a(k)``) — carries an :class:`Index`."""
    ARRAY_WHOLE = auto()
    """A whole array touched as a unit (``array set``/``array get``/``$a``)."""
    DICT_PATH = auto()
    """A dict accessed at a key path (``dict get $d a b``) — carries ``keys``."""
    INSTANCE_VAR = auto()
    """A TclOO instance variable (``owner`` = class/object FQ name)."""
    UPVAR_ALIAS = auto()
    """A local alias to a caller/namespace var (``upvar``); ``owner`` = target."""
    UNKNOWN = auto()
    """Top — a place that cannot be pinned down (dynamic name, opaque)."""


class IndexKind(Enum):
    LITERAL = auto()
    """A statically-known index/key text."""
    DYNAMIC = auto()
    """A computed index/key (``a($i)``) — its ``read_places`` are read."""
    ANY = auto()
    """An unconstrained index — whole-collection or unresolvable."""


@dataclass(frozen=True, slots=True)
class Index:
    """An array index or dict key in the place sub-lattice."""

    kind: IndexKind
    value: str = ""  # the literal text when LITERAL; "" otherwise
    read_places: tuple["Place", ...] = ()  # vars read to form a DYNAMIC index/key


def literal_index(value: str) -> Index:
    return Index(IndexKind.LITERAL, value)


def dynamic_index(read_places: tuple["Place", ...] = ()) -> Index:
    return Index(IndexKind.DYNAMIC, "", read_places)


ANY_INDEX = Index(IndexKind.ANY)


@dataclass(frozen=True, slots=True)
class Place:
    """A storage location a read/write touches."""

    kind: PlaceKind
    ns: str = LOCAL_NS  # FQ namespace; LOCAL_NS for proc-local scalars
    name: str = ""  # base variable name (bare local or ns-tail)
    index: Index | None = None  # ARRAY_ELEM
    keys: tuple[Index, ...] = ()  # DICT_PATH key path
    owner: str = ""  # INSTANCE_VAR class/obj FQ; UPVAR_ALIAS caller target
    observed: bool = False  # under a `trace` — externally observable
    dynamic: bool = False  # alias/level/name could not be resolved precisely
    name_reads: tuple["Place", ...] = field(default=())  # vars read to form a dynamic name

    @property
    def is_global(self) -> bool:
        """True for a global / namespace variable (not a frame-local)."""
        return self.kind in (PlaceKind.SCALAR, PlaceKind.ARRAY_ELEM, PlaceKind.ARRAY_WHOLE) and (
            self.ns != LOCAL_NS
        )

    @property
    def base(self) -> "Place":
        """The whole-variable identity, dropping element index / dict keys.

        ``a(k)`` → ``ARRAY_WHOLE a``; ``dict d a b`` → ``ARRAY_WHOLE``-style
        root; a scalar is its own base.  This is the join point of overlap.
        """
        if self.kind is PlaceKind.ARRAY_ELEM:
            return Place(PlaceKind.ARRAY_WHOLE, self.ns, self.name, observed=self.observed)
        if self.kind is PlaceKind.DICT_PATH:
            return Place(PlaceKind.ARRAY_WHOLE, self.ns, self.name, observed=self.observed)
        return self


# Constructors -------------------------------------------------------------


def scalar(name: str, ns: str = LOCAL_NS, *, observed: bool = False) -> Place:
    return Place(PlaceKind.SCALAR, ns, name, observed=observed)


def array_elem(name: str, index: Index, ns: str = LOCAL_NS, *, observed: bool = False) -> Place:
    return Place(PlaceKind.ARRAY_ELEM, ns, name, index=index, observed=observed)


def array_whole(name: str, ns: str = LOCAL_NS, *, observed: bool = False) -> Place:
    return Place(PlaceKind.ARRAY_WHOLE, ns, name, observed=observed)


def dict_path(name: str, keys: tuple[Index, ...], ns: str = LOCAL_NS) -> Place:
    return Place(PlaceKind.DICT_PATH, ns, name, keys=keys)


def instance_var(name: str, owner: str, *, observed: bool = False) -> Place:
    return Place(PlaceKind.INSTANCE_VAR, name=name, owner=owner, observed=observed)


def upvar_alias(name: str, owner: str = "", *, dynamic: bool = False) -> Place:
    return Place(PlaceKind.UPVAR_ALIAS, name=name, owner=owner, dynamic=dynamic or not owner)


def unknown(name_reads: tuple[Place, ...] = ()) -> Place:
    return Place(PlaceKind.UNKNOWN, dynamic=True, name_reads=name_reads)


UNKNOWN = Place(PlaceKind.UNKNOWN, dynamic=True)


# Relations ----------------------------------------------------------------


def _index_overlap(a: Index, b: Index) -> bool:
    """Could indices/keys *a* and *b* denote the same slot?  Over-approximates:
    ANY or DYNAMIC overlaps anything; two literals overlap iff equal text."""
    if a.kind is IndexKind.ANY or b.kind is IndexKind.ANY:
        return True
    if a.kind is IndexKind.DYNAMIC or b.kind is IndexKind.DYNAMIC:
        return True
    return a.value == b.value


def _keys_overlap(a: tuple[Index, ...], b: tuple[Index, ...]) -> bool:
    """Dict key paths overlap iff one is a prefix of the other under
    :func:`_index_overlap` (a write to ``d a b`` is observed by a read of
    ``d a``; ``d x`` is not)."""
    for ia, ib in zip(a, b):
        if not _index_overlap(ia, ib):
            return False
    return True  # shorter is a prefix of the longer


def overlap(p: Place, q: Place) -> bool:
    """Does a read of *q* observe a write of *p* (and vice-versa)?

    Over-approximating and symmetric: ``UNKNOWN``, ``UPVAR_ALIAS``, dynamic
    aliases, and ``DYNAMIC``/``ANY`` indices all return ``True`` — so a
    consumer can only ever decide "these may be the same" conservatively.
    """
    # Top / unresolved alias edges may touch anything observable.
    if p.kind is PlaceKind.UNKNOWN or q.kind is PlaceKind.UNKNOWN:
        return True
    if p.kind is PlaceKind.UPVAR_ALIAS or q.kind is PlaceKind.UPVAR_ALIAS:
        # Two non-dynamic upvar aliases name a *resolved* caller var via their
        # ``owner``; they are the same storage iff the owners match — regardless
        # of the local alias name (``upvar 1 caller_x a; upvar 1 caller_x b`` ⇒
        # ``a`` and ``b`` are the same variable, a must-alias).  Otherwise
        # conservatively yes (the caller resolves alias sets before relying on
        # disjointness).
        if p.kind is q.kind and not p.dynamic and not q.dynamic:
            return p.owner == q.owner
        return True
    if p.dynamic or q.dynamic:
        # A ``dynamic`` place is one whose *alias target / frame* could not be
        # resolved (e.g. ``upvar 1 $x date`` — ``date`` aliases an unknown
        # caller var).  Crucially its *local base name* is still known, so it is
        # NOT a wildcard over all names (that is ``UNKNOWN``, handled above):
        #
        #  * two dynamic places of *different* names may still alias — two
        #    upvar aliases can point at the same caller storage — so stay
        #    conservative (``True``);
        #  * a dynamic place vs a *concrete* place of a *different* base name
        #    cannot alias: a caller-frame alias ``date`` does not touch a
        #    differently-named this-frame local ``date2`` (this is the
        #    precision the dead-store needs — see phase8-place-migration.md);
        #  * the *same* base name falls through to the structural comparison
        #    below, so distinct elements of a dynamic-aliased array
        #    (``date(k)`` vs ``date(j)``) are still disjoint.
        if (p.ns, p.name) != (q.ns, q.name):
            return p.dynamic and q.dynamic
        # same (ns, name): fall through to the structural element/kind logic.

    if p.kind is PlaceKind.INSTANCE_VAR or q.kind is PlaceKind.INSTANCE_VAR:
        return p.kind is q.kind and p.owner == q.owner and p.name == q.name

    if p.kind is PlaceKind.DICT_PATH or q.kind is PlaceKind.DICT_PATH:
        if p.kind is q.kind:
            return p.ns == q.ns and p.name == q.name and _keys_overlap(p.keys, q.keys)
        # A dict and its whole-array base overlap; a dict and a scalar/elem of
        # a different name do not.
        other = q if p.kind is PlaceKind.DICT_PATH else p
        dpath = p if p.kind is PlaceKind.DICT_PATH else q
        if other.kind is PlaceKind.ARRAY_WHOLE:
            return other.ns == dpath.ns and other.name == dpath.name
        return False

    # Scalar / array family — same namespace + base name required.
    if p.ns != q.ns or p.name != q.name:
        return False
    if p.kind is PlaceKind.SCALAR and q.kind is PlaceKind.SCALAR:
        return True
    if p.kind is PlaceKind.ARRAY_WHOLE or q.kind is PlaceKind.ARRAY_WHOLE:
        # whole overlaps whole and any element of the same array
        return True
    if p.kind is PlaceKind.ARRAY_ELEM and q.kind is PlaceKind.ARRAY_ELEM:
        assert p.index is not None and q.index is not None
        return _index_overlap(p.index, q.index)
    # SCALAR vs ARRAY_ELEM of the same name: distinct kinds, treat as disjoint
    # (a scalar and an array are different variables in Tcl).
    return False


def places_read_to_form(p: Place) -> frozenset[Place]:
    """Scalars read to *compute* this place's identity — dynamic array/dict
    indices and runtime variable names.  These are genuine reads at the access
    site (``$i`` in ``a($i)``, ``X`` in ``set $X v``)."""
    out: set[Place] = set(p.name_reads)
    if p.index is not None:
        out.update(p.index.read_places)
    for k in p.keys:
        out.update(k.read_places)
    return frozenset(out)


__all__ = [
    "LOCAL_NS",
    "PlaceKind",
    "IndexKind",
    "Index",
    "Place",
    "ANY_INDEX",
    "UNKNOWN",
    "literal_index",
    "dynamic_index",
    "scalar",
    "array_elem",
    "array_whole",
    "dict_path",
    "instance_var",
    "upvar_alias",
    "unknown",
    "overlap",
    "places_read_to_form",
]
