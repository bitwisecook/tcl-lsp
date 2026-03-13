"""Tcl internal representation (intrep) type lattice.

Tcl values are always strings but may cache a typed internal
representation.  This module models the set of known intreps
and defines a lattice for tracking them through SSA dataflow.

Lattice order (bottom to top)::

    UNKNOWN  <  KNOWN(t)  <  SHIMMERED(a, b)  <  OVERDEFINED

Join rules:

- ``UNKNOWN | x = x``
- ``OVERDEFINED | x = OVERDEFINED``
- ``KNOWN(t) | KNOWN(t) = KNOWN(t)``
- ``BOOLEAN | INT = INT``  (booleans are integers in Tcl)
- ``INT | DOUBLE = NUMERIC``
- Incompatible concrete types → ``SHIMMERED(a, b)``
- ``SHIMMERED | KNOWN(c)`` where *c* matches neither side → ``OVERDEFINED``
- ``SHIMMERED(a,b) | SHIMMERED(a,b) = SHIMMERED(a,b)``
- Different SHIMMERED pairs → ``OVERDEFINED``
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto


class TclType(Enum):
    """Known Tcl internal representation types."""

    STRING = auto()
    INT = auto()
    DOUBLE = auto()
    BOOLEAN = auto()
    LIST = auto()
    DICT = auto()
    BYTEARRAY = auto()
    NUMERIC = auto()  # abstract join of INT and DOUBLE


class TypeKind(Enum):
    """Lattice element kind, ordered bottom to top."""

    UNKNOWN = auto()
    KNOWN = auto()
    SHIMMERED = auto()
    OVERDEFINED = auto()


@dataclass(frozen=True, slots=True)
class TypeLattice:
    """A single element of the type lattice."""

    kind: TypeKind
    tcl_type: TclType | None = None
    from_type: TclType | None = None  # only for SHIMMERED

    @staticmethod
    def unknown() -> TypeLattice:
        return _UNKNOWN

    @staticmethod
    def overdefined() -> TypeLattice:
        return _OVERDEFINED

    @staticmethod
    def of(t: TclType) -> TypeLattice:
        return TypeLattice(TypeKind.KNOWN, t)

    @staticmethod
    def shimmered(from_type: TclType, to_type: TclType) -> TypeLattice:
        a, b = sorted((from_type, to_type), key=lambda t: t.value)
        return TypeLattice(TypeKind.SHIMMERED, tcl_type=b, from_type=a)

    def __repr__(self) -> str:
        match self.kind:
            case TypeKind.UNKNOWN:
                return "TypeLattice.UNKNOWN"
            case TypeKind.OVERDEFINED:
                return "TypeLattice.OVERDEFINED"
            case TypeKind.KNOWN:
                return f"TypeLattice.of({self.tcl_type!r})"
            case TypeKind.SHIMMERED:
                return f"TypeLattice.shimmered({self.from_type!r}, {self.tcl_type!r})"
            case _:  # pragma: no cover
                return f"TypeLattice({self.kind!r}, {self.tcl_type!r}, {self.from_type!r})"


_UNKNOWN = TypeLattice(TypeKind.UNKNOWN)
_OVERDEFINED = TypeLattice(TypeKind.OVERDEFINED)

# Numeric promotion table: maps unordered pair of TclTypes to their join.
# Only entries where the join is NOT OVERDEFINED are listed.
_NUMERIC_PROMOTIONS: dict[frozenset[TclType], TclType] = {
    frozenset({TclType.BOOLEAN, TclType.INT}): TclType.INT,
    frozenset({TclType.BOOLEAN, TclType.DOUBLE}): TclType.NUMERIC,
    frozenset({TclType.BOOLEAN, TclType.NUMERIC}): TclType.NUMERIC,
    frozenset({TclType.INT, TclType.DOUBLE}): TclType.NUMERIC,
    frozenset({TclType.INT, TclType.NUMERIC}): TclType.NUMERIC,
    frozenset({TclType.DOUBLE, TclType.NUMERIC}): TclType.NUMERIC,
}


def type_join(a: TypeLattice, b: TypeLattice) -> TypeLattice:
    """Compute the join (least upper bound) of two lattice elements."""
    # Identity / absorbing
    if a.kind is TypeKind.UNKNOWN:
        return b
    if b.kind is TypeKind.UNKNOWN:
        return a
    if a.kind is TypeKind.OVERDEFINED or b.kind is TypeKind.OVERDEFINED:
        return _OVERDEFINED

    # Both KNOWN
    if a.kind is TypeKind.KNOWN and b.kind is TypeKind.KNOWN:
        assert a.tcl_type is not None and b.tcl_type is not None
        if a.tcl_type is b.tcl_type:
            return a
        # Check numeric promotion
        pair = frozenset({a.tcl_type, b.tcl_type})
        promoted = _NUMERIC_PROMOTIONS.get(pair)
        if promoted is not None:
            return TypeLattice.of(promoted)
        # Incompatible concrete types → SHIMMERED
        return TypeLattice.shimmered(a.tcl_type, b.tcl_type)

    # Both SHIMMERED
    if a.kind is TypeKind.SHIMMERED and b.kind is TypeKind.SHIMMERED:
        if a == b:
            return a
        return _OVERDEFINED

    # One KNOWN, one SHIMMERED
    if a.kind is TypeKind.SHIMMERED and b.kind is TypeKind.KNOWN:
        a, b = b, a  # normalise: b is now SHIMMERED
    # a is KNOWN, b is SHIMMERED
    if a.tcl_type is b.tcl_type or a.tcl_type is b.from_type:
        return b
    return _OVERDEFINED
