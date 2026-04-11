"""Tests for the Tcl internal representation type lattice."""

from __future__ import annotations

import pytest

from core.compiler.types import TclType, TypeKind, TypeLattice, type_join

UNKNOWN = TypeLattice.unknown()
OVERDEFINED = TypeLattice.overdefined()

# Concrete types
STRING = TypeLattice.of(TclType.STRING)
INT = TypeLattice.of(TclType.INT)
DOUBLE = TypeLattice.of(TclType.DOUBLE)
BOOLEAN = TypeLattice.of(TclType.BOOLEAN)
LIST = TypeLattice.of(TclType.LIST)
DICT = TypeLattice.of(TclType.DICT)
BYTEARRAY = TypeLattice.of(TclType.BYTEARRAY)
NUMERIC = TypeLattice.of(TclType.NUMERIC)


class TestFactories:
    def test_unknown(self):
        assert UNKNOWN.kind is TypeKind.UNKNOWN
        assert UNKNOWN.tcl_type is None

    def test_overdefined(self):
        assert OVERDEFINED.kind is TypeKind.OVERDEFINED
        assert OVERDEFINED.tcl_type is None

    def test_of(self):
        assert INT.kind is TypeKind.KNOWN
        assert INT.tcl_type is TclType.INT

    def test_shimmered(self):
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        assert s.kind is TypeKind.SHIMMERED
        # Canonical ordering: from_type < tcl_type by enum value
        assert s.from_type is not None
        assert s.tcl_type is not None

    def test_shimmered_canonical_order(self):
        """shimmered(A, B) == shimmered(B, A) regardless of argument order."""
        s1 = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        s2 = TypeLattice.shimmered(TclType.INT, TclType.STRING)
        assert s1 == s2

    def test_repr_unknown(self):
        assert "UNKNOWN" in repr(UNKNOWN)

    def test_repr_known(self):
        assert "INT" in repr(INT)

    def test_repr_shimmered(self):
        s = TypeLattice.shimmered(TclType.STRING, TclType.LIST)
        assert "shimmered" in repr(s)


class TestJoinIdentity:
    """UNKNOWN is the identity element."""

    @pytest.mark.parametrize(
        "x", [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]
    )
    def test_unknown_left(self, x):
        assert type_join(UNKNOWN, x) == x

    @pytest.mark.parametrize(
        "x", [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]
    )
    def test_unknown_right(self, x):
        assert type_join(x, UNKNOWN) == x


class TestJoinAbsorbing:
    """OVERDEFINED absorbs everything."""

    @pytest.mark.parametrize(
        "x", [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]
    )
    def test_overdefined_left(self, x):
        assert type_join(OVERDEFINED, x) == OVERDEFINED

    @pytest.mark.parametrize(
        "x", [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]
    )
    def test_overdefined_right(self, x):
        assert type_join(x, OVERDEFINED) == OVERDEFINED


class TestJoinIdempotency:
    """join(x, x) == x for all x."""

    @pytest.mark.parametrize(
        "x", [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]
    )
    def test_idempotent(self, x):
        assert type_join(x, x) == x

    def test_idempotent_shimmered(self):
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        assert type_join(s, s) == s


class TestJoinCommutativity:
    """join(a, b) == join(b, a) for all pairs."""

    ALL = [UNKNOWN, INT, DOUBLE, STRING, LIST, DICT, BYTEARRAY, BOOLEAN, NUMERIC, OVERDEFINED]

    @pytest.mark.parametrize("a", ALL)
    @pytest.mark.parametrize("b", ALL)
    def test_commutative(self, a, b):
        assert type_join(a, b) == type_join(b, a)


class TestNumericPromotions:
    """Numeric type hierarchy: BOOLEAN < INT, DOUBLE < NUMERIC."""

    def test_boolean_int(self):
        assert type_join(BOOLEAN, INT) == INT

    def test_boolean_double(self):
        assert type_join(BOOLEAN, DOUBLE) == NUMERIC

    def test_int_double(self):
        assert type_join(INT, DOUBLE) == NUMERIC

    def test_int_numeric(self):
        assert type_join(INT, NUMERIC) == NUMERIC

    def test_double_numeric(self):
        assert type_join(DOUBLE, NUMERIC) == NUMERIC

    def test_boolean_numeric(self):
        assert type_join(BOOLEAN, NUMERIC) == NUMERIC


class TestShimmeredProduction:
    """Incompatible concrete types produce SHIMMERED."""

    def test_string_int(self):
        result = type_join(STRING, INT)
        assert result.kind is TypeKind.SHIMMERED

    def test_string_list(self):
        result = type_join(STRING, LIST)
        assert result.kind is TypeKind.SHIMMERED

    def test_list_dict(self):
        result = type_join(LIST, DICT)
        assert result.kind is TypeKind.SHIMMERED

    def test_bytearray_string(self):
        result = type_join(BYTEARRAY, STRING)
        assert result.kind is TypeKind.SHIMMERED

    def test_bytearray_int(self):
        result = type_join(BYTEARRAY, INT)
        assert result.kind is TypeKind.SHIMMERED

    def test_list_int(self):
        result = type_join(LIST, INT)
        assert result.kind is TypeKind.SHIMMERED

    def test_dict_string(self):
        result = type_join(DICT, STRING)
        assert result.kind is TypeKind.SHIMMERED

    def test_shimmered_preserves_types(self):
        result = type_join(STRING, INT)
        types_in_result = {result.tcl_type, result.from_type}
        assert types_in_result == {TclType.STRING, TclType.INT}


class TestShimmeredJoin:
    """SHIMMERED joined with compatible or incompatible types."""

    def test_shimmered_with_matching_known(self):
        """KNOWN(A) | SHIMMERED(A, B) = SHIMMERED(A, B)."""
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        result = type_join(STRING, s)
        assert result == s

    def test_shimmered_with_other_matching_known(self):
        """KNOWN(B) | SHIMMERED(A, B) = SHIMMERED(A, B)."""
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        result = type_join(INT, s)
        assert result == s

    def test_shimmered_with_unrelated_known(self):
        """KNOWN(C) | SHIMMERED(A, B) where C != A and C != B → OVERDEFINED."""
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        result = type_join(LIST, s)
        assert result == OVERDEFINED

    def test_different_shimmered_pairs(self):
        """SHIMMERED(A, B) | SHIMMERED(C, D) → OVERDEFINED."""
        s1 = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        s2 = TypeLattice.shimmered(TclType.LIST, TclType.DICT)
        assert type_join(s1, s2) == OVERDEFINED

    def test_same_shimmered(self):
        s = TypeLattice.shimmered(TclType.STRING, TclType.INT)
        assert type_join(s, s) == s


class TestMonotonicity:
    """The lattice is monotone: if a <= b then join(a, x) <= join(b, x).

    The partial order accounts for the numeric subtype hierarchy:
    BOOLEAN < INT < NUMERIC, DOUBLE < NUMERIC.
    """

    _NUMERIC_SUBTYPES = {
        TclType.BOOLEAN: {TclType.BOOLEAN, TclType.INT, TclType.NUMERIC},
        TclType.INT: {TclType.INT, TclType.NUMERIC},
        TclType.DOUBLE: {TclType.DOUBLE, TclType.NUMERIC},
        TclType.NUMERIC: {TclType.NUMERIC},
    }

    def _leq(self, a: TypeLattice, b: TypeLattice) -> bool:
        """Partial order on the type lattice."""
        if a == b:
            return True
        if a.kind is TypeKind.UNKNOWN:
            return True
        if b.kind is TypeKind.OVERDEFINED:
            return True
        # KNOWN < SHIMMERED < OVERDEFINED
        if a.kind is TypeKind.KNOWN and b.kind in (TypeKind.SHIMMERED, TypeKind.OVERDEFINED):
            return True
        if a.kind is TypeKind.SHIMMERED and b.kind is TypeKind.OVERDEFINED:
            return True
        # Within KNOWN: subtype ordering (BOOLEAN < INT < NUMERIC, DOUBLE < NUMERIC)
        if a.kind is TypeKind.KNOWN and b.kind is TypeKind.KNOWN:
            supertypes = self._NUMERIC_SUBTYPES.get(a.tcl_type) if a.tcl_type is not None else None
            if supertypes and b.tcl_type in supertypes:
                return True
        return False

    def test_unknown_leq_known_monotone(self):
        """join(UNKNOWN, x) <= join(KNOWN(INT), x) for all x."""
        for x in [INT, STRING, LIST, DOUBLE, BOOLEAN]:
            j1 = type_join(UNKNOWN, x)
            j2 = type_join(INT, x)
            assert self._leq(j1, j2), f"Failed for x={x}: {j1} not <= {j2}"

    def test_known_leq_overdefined_monotone(self):
        """join(KNOWN, x) <= join(OVERDEFINED, x) for all x."""
        for x in [INT, STRING, LIST, DOUBLE, BOOLEAN]:
            j1 = type_join(INT, x)
            j2 = type_join(OVERDEFINED, x)
            assert self._leq(j1, j2), f"Failed for x={x}: {j1} not <= {j2}"
