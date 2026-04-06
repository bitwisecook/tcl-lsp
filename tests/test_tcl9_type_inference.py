"""Type inference tests derived from the Tcl 9.0.2 test suite.

Verifies that the SSA type propagation system correctly infers types for
patterns extracted from expr-old.test, expr.test, and set.test.
"""

from __future__ import annotations

import pytest

from core.compiler.types import TclType, TypeKind

from .helpers import analyse_types as _analyse
from .helpers import var_type as _var_type

# Literal type inference


class TestLiteralTypeInference:
    """Verify type inference for literal values assigned via set."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x 42", "x", TclType.INT, id="int-decimal"),
            pytest.param("set x -7", "x", TclType.INT, id="int-negative"),
            pytest.param("set x 0", "x", TclType.INT, id="int-zero"),
            # Hex/octal/binary prefixed literals are STRING at `set` level
            # (they become INT only when used in expr context)
            pytest.param("set x 0xFF", "x", TclType.STRING, id="set-hex-is-string"),
            pytest.param("set x 0xff", "x", TclType.STRING, id="set-hex-lower-is-string"),
            pytest.param("set x 0o15", "x", TclType.STRING, id="set-octal-is-string"),
            pytest.param("set x 0b1010", "x", TclType.STRING, id="set-binary-is-string"),
            pytest.param("set x 3.14", "x", TclType.DOUBLE, id="double"),
            pytest.param("set x -4.2", "x", TclType.DOUBLE, id="double-negative"),
            pytest.param("set x 2.0e2", "x", TclType.DOUBLE, id="double-sci-notation"),
            pytest.param("set x 1.5e10", "x", TclType.DOUBLE, id="double-large-exponent"),
            pytest.param("set x 62.0", "x", TclType.DOUBLE, id="double-trailing-zero"),
            pytest.param("set x true", "x", TclType.BOOLEAN, id="bool-true"),
            pytest.param("set x false", "x", TclType.BOOLEAN, id="bool-false"),
            pytest.param("set x yes", "x", TclType.BOOLEAN, id="bool-yes"),
            pytest.param("set x no", "x", TclType.BOOLEAN, id="bool-no"),
            pytest.param("set x on", "x", TclType.BOOLEAN, id="bool-on"),
            pytest.param("set x off", "x", TclType.BOOLEAN, id="bool-off"),
            pytest.param("set x hello", "x", TclType.STRING, id="string"),
            pytest.param("set x {hello world}", "x", TclType.STRING, id="string-braced"),
        ],
    )
    def test_literal_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Arithmetic result types


class TestArithmeticResultTypes:
    """Verify type inference for arithmetic expressions (from expr-old sections 1-2)."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            # INT op INT → INT
            pytest.param("set x [expr {4*6}]", "x", TclType.INT, id="expr-old-1.6"),
            pytest.param("set x [expr {36/12}]", "x", TclType.INT, id="expr-old-1.7"),
            pytest.param("set x [expr {27%4}]", "x", TclType.INT, id="expr-old-1.9"),
            pytest.param("set x [expr {2+2}]", "x", TclType.INT, id="expr-old-1.10"),
            pytest.param("set x [expr {2-6}]", "x", TclType.INT, id="expr-old-1.11"),
            pytest.param("set x [expr {1<<3}]", "x", TclType.INT, id="expr-old-1.12"),
            pytest.param("set x [expr {0xff>>2}]", "x", TclType.INT, id="expr-old-1.13"),
            pytest.param("set x [expr {36/5}]", "x", TclType.INT, id="expr-old-1.42"),
            pytest.param("set x [expr {36%5}]", "x", TclType.INT, id="expr-old-1.43"),
            pytest.param("set x [expr {2**10}]", "x", TclType.INT, id="power-int"),
            # DOUBLE ops → DOUBLE
            pytest.param("set x [expr {4.2*6.3}]", "x", TclType.DOUBLE, id="expr-old-2.7"),
            pytest.param("set x [expr {36.0/12.0}]", "x", TclType.DOUBLE, id="expr-old-2.8"),
            pytest.param("set x [expr {2.3+2.1}]", "x", TclType.DOUBLE, id="expr-old-2.10"),
            pytest.param("set x [expr {2.3-6.5}]", "x", TclType.DOUBLE, id="expr-old-2.11"),
            # Mixed INT+DOUBLE → DOUBLE
            pytest.param("set x [expr {27/4.0}]", "x", TclType.DOUBLE, id="expr-old-2.9"),
            pytest.param(
                "set a 1\nset b 2.0\nset x [expr {$a + $b}]",
                "x",
                TclType.DOUBLE,
                id="int-plus-double",
            ),
            # Comparison → BOOLEAN
            pytest.param("set x [expr {3>2}]", "x", TclType.BOOLEAN, id="expr-old-1.15"),
            pytest.param("set x [expr {3==2}]", "x", TclType.BOOLEAN, id="expr-old-1.27"),
            pytest.param("set x [expr {3!=2}]", "x", TclType.BOOLEAN, id="expr-old-1.29"),
            pytest.param("set x [expr {3>=2}]", "x", TclType.BOOLEAN, id="expr-old-1.21"),
            pytest.param("set x [expr {3<=2}]", "x", TclType.BOOLEAN, id="expr-old-1.24"),
            pytest.param("set x [expr {3<2}]", "x", TclType.BOOLEAN, id="expr-old-1.18"),
            pytest.param("set x [expr {3.1>2.1}]", "x", TclType.BOOLEAN, id="expr-old-2.12"),
            pytest.param("set x [expr {3.2==2.2}]", "x", TclType.BOOLEAN, id="expr-old-2.24"),
            # Logical → BOOLEAN
            pytest.param("set x [expr {0&&1}]", "x", TclType.BOOLEAN, id="expr-old-1.34"),
            pytest.param("set x [expr {0||1}]", "x", TclType.BOOLEAN, id="expr-old-1.37"),
            pytest.param("set x [expr {1.3&&3.3}]", "x", TclType.BOOLEAN, id="expr-old-2.31"),
            pytest.param("set x [expr {0.0||1.3}]", "x", TclType.BOOLEAN, id="expr-old-2.33"),
            # Bitwise → INT
            pytest.param("set x [expr {7&0x13}]", "x", TclType.INT, id="expr-old-1.31"),
            pytest.param("set x [expr {7^0x13}]", "x", TclType.INT, id="expr-old-1.32"),
            pytest.param("set x [expr {7|0x13}]", "x", TclType.INT, id="expr-old-1.33"),
            # String comparison → BOOLEAN
            pytest.param('set x [expr {"abc" eq "abd"}]', "x", TclType.BOOLEAN, id="str-eq"),
            pytest.param('set x [expr {"abc" ne "abd"}]', "x", TclType.BOOLEAN, id="str-ne"),
            pytest.param(
                "set a x\nset b y\nset x [expr {$a lt $b}]", "x", TclType.BOOLEAN, id="str-lt"
            ),
            pytest.param(
                "set a x\nset b y\nset x [expr {$a ge $b}]", "x", TclType.BOOLEAN, id="str-ge"
            ),
        ],
    )
    def test_arithmetic_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Math function return types


class TestMathFunctionTypes:
    """Verify type inference for math function return types."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x [expr {sin(1.0)}]", "x", TclType.DOUBLE, id="sin"),
            pytest.param("set x [expr {cos(1.0)}]", "x", TclType.DOUBLE, id="cos"),
            pytest.param("set x [expr {tan(0.5)}]", "x", TclType.DOUBLE, id="tan"),
            pytest.param("set x [expr {sqrt(4.0)}]", "x", TclType.DOUBLE, id="sqrt"),
            pytest.param("set x [expr {exp(1.0)}]", "x", TclType.DOUBLE, id="exp"),
            pytest.param("set x [expr {log(2.7)}]", "x", TclType.DOUBLE, id="log"),
            pytest.param("set x [expr {log10(100.0)}]", "x", TclType.DOUBLE, id="log10"),
            pytest.param("set x [expr {pow(2.0, 3.0)}]", "x", TclType.DOUBLE, id="pow"),
            pytest.param("set x [expr {hypot(3.0, 4.0)}]", "x", TclType.DOUBLE, id="hypot"),
            pytest.param("set x [expr {double(5)}]", "x", TclType.DOUBLE, id="double"),
            pytest.param("set x [expr {rand()}]", "x", TclType.DOUBLE, id="rand"),
            pytest.param("set x [expr {srand(42)}]", "x", TclType.DOUBLE, id="srand"),
            pytest.param("set x [expr {int(3.14)}]", "x", TclType.INT, id="int"),
            pytest.param("set x [expr {round(3.14)}]", "x", TclType.INT, id="round"),
            pytest.param("set x [expr {ceil(3.14)}]", "x", TclType.INT, id="ceil"),
            pytest.param("set x [expr {floor(3.14)}]", "x", TclType.INT, id="floor"),
            pytest.param("set x [expr {wide(42)}]", "x", TclType.INT, id="wide"),
            pytest.param("set x [expr {entier(3.14)}]", "x", TclType.INT, id="entier"),
            pytest.param("set x [expr {isqrt(16)}]", "x", TclType.INT, id="isqrt"),
            pytest.param("set x [expr {bool(1)}]", "x", TclType.BOOLEAN, id="bool"),
        ],
    )
    def test_math_function_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Unary operator types


class TestUnaryOperatorTypes:
    """Verify type inference for unary operator results."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x [expr {!2}]", "x", TclType.BOOLEAN, id="expr-old-1.4"),
            pytest.param("set x [expr {!0}]", "x", TclType.BOOLEAN, id="expr-old-1.5"),
            pytest.param("set x [expr {!2.1}]", "x", TclType.BOOLEAN, id="expr-old-2.5"),
            pytest.param("set x [expr {!0.0}]", "x", TclType.BOOLEAN, id="expr-old-2.6"),
            pytest.param("set x [expr {~3}]", "x", TclType.INT, id="expr-old-1.3"),
            pytest.param("set a 5\nset x [expr {-$a}]", "x", TclType.INT, id="negate-int"),
            pytest.param("set a 5\nset x [expr {+$a}]", "x", TclType.INT, id="positive-int"),
            pytest.param("set a 5.0\nset x [expr {-$a}]", "x", TclType.DOUBLE, id="negate-double"),
        ],
    )
    def test_unary_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Ternary type join


class TestTernaryTypeJoin:
    """Verify type inference for ternary expression results."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x [expr {1 ? 44 : 66}]", "x", TclType.INT, id="int-int"),
            pytest.param("set x [expr {1 ? 44 : 66.0}]", "x", TclType.NUMERIC, id="int-double"),
            pytest.param("set x [expr {0 ? 44 : 66}]", "x", TclType.INT, id="false-int-int"),
        ],
    )
    def test_ternary_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Command return types


class TestCommandReturnTypes:
    """Verify type inference for command return types."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x [list a b c]", "x", TclType.LIST, id="list"),
            pytest.param("set x [split {a,b,c} ,]", "x", TclType.LIST, id="split"),
            pytest.param("set x [concat {a b} {c d}]", "x", TclType.LIST, id="concat"),
            pytest.param("set x [join {a b c} ,]", "x", TclType.STRING, id="join"),
            pytest.param("set x [llength {a b c}]", "x", TclType.INT, id="llength"),
            pytest.param("set x [string length hello]", "x", TclType.INT, id="string-length"),
            pytest.param("set x 0\nincr x", "x", TclType.INT, id="incr"),
            pytest.param("set x 0\nincr x 5", "x", TclType.INT, id="incr-amount"),
        ],
    )
    def test_command_return_types(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Variable reference propagation


class TestVariableReferencePropagation:
    """Verify type flows through variable references."""

    @pytest.mark.parametrize(
        "source,var,expected",
        [
            pytest.param("set x 42\nset y $x", "y", TclType.INT, id="int-ref"),
            pytest.param("set x 3.14\nset y $x", "y", TclType.DOUBLE, id="double-ref"),
            pytest.param(
                'set x 42\nset y "value is $x"', "y", TclType.STRING, id="string-interpolation"
            ),
        ],
    )
    def test_propagation(self, source, var, expected):
        analysis = _analyse(source)
        t = _var_type(analysis, var)
        assert t is not None
        assert t.tcl_type is expected


# Phi merge types


class TestPhiMergeTypes:
    """Verify type inference at phi merge points (from if/else branches)."""

    def test_same_type_merges(self):
        """INT in both branches → INT."""
        source = "set x 1\nif {1} {\n    set x 2\n} else {\n    set x 3\n}"
        analysis = _analyse(source)
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_incompatible_types_shimmer(self):
        """INT vs STRING branches → SHIMMERED or OVERDEFINED at phi."""
        source = "set x 1\nif {$cond} {\n    set x hello\n}"
        analysis = _analyse(source)
        all_types = {ver: t for (name, ver), t in analysis.types.items() if name == "x"}
        shimmer_versions = [
            t for t in all_types.values() if t.kind in (TypeKind.SHIMMERED, TypeKind.OVERDEFINED)
        ]
        assert shimmer_versions, f"Expected a SHIMMERED/OVERDEFINED version, got {all_types}"


# Abs preserves operand type


class TestIdentityFunctions:
    """Functions like abs that preserve operand type."""

    def test_abs_int(self):
        analysis = _analyse("set x -5\nset z [expr {abs($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_abs_double(self):
        analysis = _analyse("set x -5.0\nset z [expr {abs($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE


# Max/min join operand types


class TestVariadicJoinFunctions:
    """Functions like max/min that join operand types."""

    def test_max_int_int(self):
        analysis = _analyse("set z [expr {max(1, 2, 3)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_max_int_double(self):
        analysis = _analyse("set z [expr {max(1, 2.0)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.NUMERIC

    def test_min_double_double(self):
        analysis = _analyse("set z [expr {min(1.0, 2.0)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE
