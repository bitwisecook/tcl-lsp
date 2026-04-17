"""Scoping and variable tests derived from the Tcl 9.0.2 test suite.

Tests that proc scoping, upvar, namespace eval, global/variable
declarations, and array variable patterns are analysed without crashing.
Derived from var.test, set.test, and upvar.test.
"""

from __future__ import annotations

import pytest

from core.analysis.analyser import analyse

# Helpers


def _no_crash(source: str):
    """Verify analysis completes without exception."""
    result = analyse(source)
    assert result is not None
    return result


# Proc scoping


class TestProcScoping:
    """Proc parameter and local variable patterns from set.test."""

    @pytest.mark.parametrize(
        "source",
        [
            # Simple global (set-1.11)
            pytest.param(
                "proc p {} {\n    global i\n    set i 54\n    set i\n}",
                id="set-1.11-global",
            ),
            # Simple local from parameter (set-1.12)
            pytest.param(
                "proc p {bar} {\n    set foo $bar\n    set foo\n}",
                id="set-1.12-local",
            ),
            # Multiple parameters
            pytest.param(
                "proc add {a b} {\n    return [expr {$a + $b}]\n}",
                id="proc-two-params",
            ),
            # Default parameter values
            pytest.param(
                'proc greet {{name World}} {\n    puts "Hello $name"\n}',
                id="proc-default-param",
            ),
            # args parameter
            pytest.param(
                "proc variadic {args} {\n    set n [llength $args]\n}",
                id="proc-args",
            ),
            # Mixed fixed and args
            pytest.param(
                "proc mixed {a b args} {\n    set n [llength $args]\n    return [expr {$a + $b + $n}]\n}",
                id="proc-mixed-args",
            ),
            # Local variable shadowing
            pytest.param(
                "set x 10\nproc p {} {\n    set x 20\n    return $x\n}",
                id="proc-local-shadow",
            ),
            # Nested proc definitions
            pytest.param(
                "proc outer {} {\n    proc inner {} {\n        return 42\n    }\n    return [inner]\n}",
                id="proc-nested",
            ),
            # Proc with expr in body
            pytest.param(
                "proc compute {x} {\n    set y [expr {$x * 2}]\n    set z [expr {$y + 1}]\n    return $z\n}",
                id="proc-expr-body",
            ),
            # Proc calling another proc
            pytest.param(
                "proc double {x} {\n    return [expr {$x * 2}]\n}\nproc quadruple {x} {\n    return [double [double $x]]\n}",
                id="proc-calling-proc",
            ),
            # Multiple globals
            pytest.param(
                "proc p {} {\n    global x y z\n    set x 1\n    set y 2\n    set z 3\n}",
                id="proc-multiple-globals",
            ),
            # Empty proc body
            pytest.param(
                "proc noop {} {}",
                id="proc-empty-body",
            ),
            # Proc with return value
            pytest.param(
                "proc identity {x} {\n    return $x\n}",
                id="proc-identity",
            ),
            # Proc with conditional
            pytest.param(
                "proc abs {x} {\n    if {$x < 0} {\n        return [expr {-$x}]\n    }\n    return $x\n}",
                id="proc-conditional",
            ),
            # Proc with loop
            pytest.param(
                "proc sum_to {n} {\n    set total 0\n    for {set i 1} {$i <= $n} {incr i} {\n        set total [expr {$total + $i}]\n    }\n    return $total\n}",
                id="proc-loop",
            ),
        ],
    )
    def test_proc_no_crash(self, source):
        _no_crash(source)


# Upvar patterns


class TestUpvarPatterns:
    """Upvar patterns from upvar.test."""

    @pytest.mark.parametrize(
        "source",
        [
            # Simple upvar (upvar-1.1)
            pytest.param(
                "proc p1 {a b} {set c 22; set d 33; p2}\n"
                "proc p2 {} {upvar a x1 b x2 c x3 d x4; list $x1 $x2 $x3 $x4}",
                id="upvar-1.1",
            ),
            # Upvar with level (upvar-1.2)
            pytest.param(
                "proc p1 {a b} {set c 22; p2}\n"
                "proc p2 {} {p3}\n"
                "proc p3 {} {upvar 2 a x1 b x2 c x3; list $x1 $x2 $x3}",
                id="upvar-1.2-level",
            ),
            # Upvar with absolute level (upvar-1.3)
            pytest.param(
                "proc p1 {a b} {set c 22; p2}\n"
                "proc p2 {} {p3}\n"
                "proc p3 {} {upvar #1 a x1 b x2; list $x1 $x2}",
                id="upvar-1.3-absolute",
            ),
            # Upvar to global (upvar-1.4)
            pytest.param(
                "set x1 44\nset x2 55\n"
                "proc p1 {} {p2}\n"
                "proc p2 {} {\n    upvar 2 x1 x1 x2 a\n    upvar #0 x1 b\n    set c $b\n}",
                id="upvar-1.4-global",
            ),
            # Upvar array element (upvar-1.5)
            pytest.param(
                "proc p1 {} {set a(0) zeroth; set a(1) first; p2}\n"
                "proc p2 {} {upvar a(0) x; set x}",
                id="upvar-1.5-array",
            ),
            # Writing via upvar (upvar-2.1)
            pytest.param(
                "proc p1 {a b} {set c 22; set d 33; p2; list $a $b $c $d}\n"
                "proc p2 {} {\n    upvar a x1 b x2 c x3 d x4\n    set x1 14\n    set x4 88\n}",
                id="upvar-2.1-write",
            ),
            # Nested upvars (upvar-4.1)
            pytest.param(
                "set x1 88\n"
                "proc p1 {a b} {set c 22; p2}\n"
                "proc p2 {} {global x1; upvar c x2; p3}\n"
                "proc p3 {} {\n    upvar x1 a x2 b\n    list $a $b\n}",
                id="upvar-4.1-nested",
            ),
            # Upvar #0 for global alias
            pytest.param(
                "set g 100\nproc p {} {\n    upvar #0 g local\n    incr local\n}",
                id="upvar-global-alias",
            ),
            # Upvar in loop
            pytest.param(
                "proc swap {varA varB} {\n"
                "    upvar 1 $varA a $varB b\n"
                "    set tmp $a\n"
                "    set a $b\n"
                "    set b $tmp\n"
                "}",
                id="upvar-swap-pattern",
            ),
            # Uplevel with upvar
            pytest.param(
                "proc with_var {name body} {\n    upvar 1 $name v\n    uplevel 1 $body\n}",
                id="upvar-uplevel-combo",
            ),
        ],
    )
    def test_upvar_no_crash(self, source):
        _no_crash(source)


# Namespace patterns


class TestNamespacePatterns:
    """Namespace patterns from var.test and namespace.test."""

    @pytest.mark.parametrize(
        "source",
        [
            # Simple namespace eval (var-1.9)
            pytest.param(
                "namespace eval test_ns {\n    set v hello\n}",
                id="var-1.9-ns-set",
            ),
            # Namespace variable declaration (var-1.3)
            pytest.param(
                'namespace eval test_ns {\n    variable x "namespace value"\n}',
                id="var-1.3-ns-variable",
            ),
            # Global from namespace proc (var-1.2)
            pytest.param(
                "namespace eval test_ns {\n"
                "    proc p {} {\n"
                "        global x\n"
                "        return $x\n"
                "    }\n"
                "}",
                id="var-1.2-ns-global",
            ),
            # Namespace variable from proc (var-1.3)
            pytest.param(
                "namespace eval test_ns {\n"
                '    variable x "value"\n'
                "    proc q {} {\n"
                "        variable x\n"
                "        return $x\n"
                "    }\n"
                "}",
                id="var-1.3-ns-proc-variable",
            ),
            # Set global from namespace
            pytest.param(
                "namespace eval test_ns {\n    set ::y 789\n}",
                id="var-1.10-ns-set-global",
            ),
            # Nested namespace
            pytest.param(
                "namespace eval outer {\n"
                "    namespace eval inner {\n"
                "        variable x 42\n"
                "    }\n"
                "}",
                id="ns-nested",
            ),
            # Namespace with proc and variable
            pytest.param(
                "namespace eval math {\n"
                "    variable pi 3.14159\n"
                "    proc area {r} {\n"
                "        variable pi\n"
                "        return [expr {$pi * $r * $r}]\n"
                "    }\n"
                "}",
                id="ns-proc-with-variable",
            ),
            # Namespace upvar (from upvar.test)
            pytest.param(
                "namespace eval test_ns {\n"
                "    variable data {a b c}\n"
                "}\n"
                "proc access {} {\n"
                "    namespace upvar test_ns data d\n"
                "    return $d\n"
                "}",
                id="ns-upvar",
            ),
            # Qualified variable reference
            pytest.param(
                "namespace eval ns1 {\n    variable x 10\n}\nset y $ns1::x",
                id="ns-qualified-ref",
            ),
            # Multiple variables in namespace
            pytest.param(
                "namespace eval config {\n"
                "    variable host localhost\n"
                "    variable port 8080\n"
                "    variable debug false\n"
                "}",
                id="ns-multiple-vars",
            ),
            # Namespace ensemble pattern
            pytest.param(
                "namespace eval counter {\n"
                "    variable count 0\n"
                "    proc increment {} {\n"
                "        variable count\n"
                "        incr count\n"
                "    }\n"
                "    proc get {} {\n"
                "        variable count\n"
                "        return $count\n"
                "    }\n"
                "    namespace export increment get\n"
                "}",
                id="ns-ensemble",
            ),
            # Global-qualified set
            pytest.param(
                "set ::globalvar 42",
                id="global-qualified-set",
            ),
            # Namespace with upvar to namespace var
            pytest.param(
                "namespace eval test_ns {\n"
                "    variable result {}\n"
                "    namespace eval subns {\n"
                "        variable foo 2\n"
                "    }\n"
                "}",
                id="ns-nested-variables",
            ),
        ],
    )
    def test_namespace_no_crash(self, source):
        _no_crash(source)


# Array variables


class TestArrayVariables:
    """Array variable patterns from var.test and set.test."""

    @pytest.mark.parametrize(
        "source",
        [
            # Simple array set (set-1.6)
            pytest.param(
                "set a(foo) 37",
                id="set-1.6-array",
            ),
            # Array with multiple elements
            pytest.param(
                "set a(0) zeroth\nset a(1) first\nset a(2) second",
                id="array-multi-element",
            ),
            # array set command
            pytest.param(
                "array set arr {a 1 b 2 c 3}",
                id="array-set-cmd",
            ),
            # Array in proc
            pytest.param(
                "proc p {} {\n    set a(0) zeroth\n    set a(1) first\n    return $a(0)\n}",
                id="array-in-proc",
            ),
            # Array element in expr
            pytest.param(
                "set a(foo) 37\nset x [expr {$a(foo) + 1}]",
                id="array-in-expr",
            ),
            # Array size
            pytest.param(
                "array set arr {x 1 y 2 z 3}\nset n [array size arr]",
                id="array-size",
            ),
            # Array names
            pytest.param(
                "array set arr {x 1 y 2 z 3}\nset names [array names arr]",
                id="array-names",
            ),
            # Array exists
            pytest.param(
                "array set arr {x 1}\nset e [array exists arr]",
                id="array-exists",
            ),
            # Nested array access
            pytest.param(
                "set data(name) John\nset data(age) 30\n"
                'set msg "Name: $data(name), Age: $data(age)"',
                id="array-interpolation",
            ),
            # Array in loop
            pytest.param(
                "for {set i 0} {$i < 5} {incr i} {\n    set a($i) [expr {$i * 2}]\n}",
                id="array-in-loop",
            ),
        ],
    )
    def test_array_no_crash(self, source):
        _no_crash(source)


# Global/variable declarations


class TestGlobalVariableDecl:
    """Global and variable declaration patterns."""

    @pytest.mark.parametrize(
        "source",
        [
            # global with multiple variables
            pytest.param(
                "proc p {} {\n    global a b c\n    set a 1\n    set b 2\n    set c 3\n}",
                id="global-multi",
            ),
            # variable in namespace
            pytest.param(
                "namespace eval ns {\n    variable x 10\n    variable y 20\n}",
                id="variable-multi",
            ),
            # global in nested proc
            pytest.param(
                "proc outer {} {\n"
                "    proc inner {} {\n"
                "        global g\n"
                "        return $g\n"
                "    }\n"
                "    return [inner]\n"
                "}",
                id="global-nested-proc",
            ),
            # variable without initial value
            pytest.param(
                "namespace eval ns {\n    variable x\n}",
                id="variable-no-value",
            ),
        ],
    )
    def test_decl_no_crash(self, source):
        _no_crash(source)


# Complex multi-scope patterns


class TestComplexScopePatterns:
    """Complex patterns combining multiple scoping features."""

    def test_proc_with_upvar_and_array(self):
        """Proc using upvar to access caller's array."""
        source = """\
proc sum_array {arrName} {
    upvar 1 $arrName arr
    set total 0
    foreach name [array names arr] {
        set total [expr {$total + $arr($name)}]
    }
    return $total
}
"""
        _no_crash(source)

    def test_namespace_proc_with_global(self):
        """Namespace proc accessing global via qualified name."""
        source = """\
set ::debug 1
namespace eval app {
    proc log {msg} {
        if {$::debug} {
            puts $msg
        }
    }
}
"""
        _no_crash(source)

    def test_recursive_proc(self):
        """Recursive proc pattern."""
        source = """\
proc factorial {n} {
    if {$n <= 1} {
        return 1
    }
    return [expr {$n * [factorial [expr {$n - 1}]]}]
}
"""
        _no_crash(source)

    def test_accumulator_pattern(self):
        """Upvar-based accumulator pattern."""
        source = """\
proc accumulate {varName value} {
    upvar 1 $varName acc
    if {![info exists acc]} {
        set acc {}
    }
    lappend acc $value
}
set result {}
accumulate result a
accumulate result b
"""
        _no_crash(source)

    def test_namespace_with_init_proc(self):
        """Namespace with an initialization proc pattern."""
        source = """\
namespace eval mylib {
    variable initialized 0
    proc init {} {
        variable initialized
        if {$initialized} return
        set initialized 1
    }
    proc run {} {
        init
        variable initialized
        return $initialized
    }
}
"""
        _no_crash(source)

    def test_coroutine_like_pattern(self):
        """Pattern resembling coroutine setup."""
        source = """\
proc generate {body} {
    set cmd [list apply [list {} $body]]
    return $cmd
}
"""
        _no_crash(source)

    def test_many_locals(self):
        """Proc with many local variables (from set-1.14)."""
        source = """\
proc many_locals {} {
    set a0 0; set a1 0; set a2 0; set a3 0; set a4 0
    set b0 0; set b1 0; set b2 0; set b3 0; set b4 0
    set c0 0; set c1 0; set c2 0; set c3 0; set c4 0
    set d0 0; set d1 0; set d2 0; set d3 0; set d4 0
    set e0 0; set e1 0; set e2 0; set e3 0; set e4 0
    return [expr {$a0 + $b0 + $c0 + $d0 + $e0}]
}
"""
        _no_crash(source)

    def test_variable_name_with_colons(self):
        """Variable names containing colons (var-1.14)."""
        source = """\
namespace eval test_ns {
    set v: 456
    set x:y: 789
}
"""
        _no_crash(source)
