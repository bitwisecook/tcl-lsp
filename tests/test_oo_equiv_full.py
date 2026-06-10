"""Full oo.test equivalence port — comprehensive pytest coverage of TclOO.

Each test mirrors a specific test case from Tcl 9.0's oo.test or ooNext2.test.
Tests are grouped by oo.test section number.
"""

from __future__ import annotations

import pytest

from tooling.vm.interp import TclInterp
from tooling.vm.types import TclError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def tcl(script: str) -> TclInterp:
    """Create an interpreter and evaluate the script."""
    interp = TclInterp()
    interp.eval(script)
    return interp


def tcl_eval(interp: TclInterp, script: str) -> str:
    """Evaluate and return the string result."""
    return interp.eval(script).value


# ---------------------------------------------------------------------------
# oo-0.x: Package loading / basics
# ---------------------------------------------------------------------------


class TestOO0Basics:
    def test_oo_0_1_oo_class_exists(self) -> None:
        """oo-0.1: oo::class is a known command."""
        interp = TclInterp()
        result = tcl_eval(interp, "info object isa object oo::class")
        assert result == "1"

    def test_oo_0_2_oo_object_exists(self) -> None:
        """oo-0.2: oo::object is a known command."""
        interp = TclInterp()
        result = tcl_eval(interp, "info object isa object oo::object")
        assert result == "1"

    def test_oo_0_3_oo_class_is_class(self) -> None:
        """oo-0.3: oo::class is a class."""
        interp = TclInterp()
        result = tcl_eval(interp, "info object isa class oo::class")
        assert result == "1"


# ---------------------------------------------------------------------------
# oo-1.x: Basic object creation and method dispatch
# ---------------------------------------------------------------------------


class TestOO1:
    def test_oo_1_1_create_object(self) -> None:
        """oo-1.1: create plain object and call instance method."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create foo")
        tcl_eval(interp, "oo::objdefine foo method bar {} { return ok }")
        assert tcl_eval(interp, "foo bar") == "ok"

    def test_oo_1_2_class_create(self) -> None:
        """oo-1.2: create class and instance."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Dog {
                method speak {} { return "woof" }
            }
        """,
        )
        tcl_eval(interp, "Dog create rex")
        assert tcl_eval(interp, "rex speak") == "woof"

    def test_oo_1_3_class_new(self) -> None:
        """oo-1.3: create instance with new."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method ping {} { return pong }
            }
        """,
        )
        name = tcl_eval(interp, "Cls new")
        assert name.startswith("::oo::Obj")
        assert tcl_eval(interp, f"{name} ping") == "pong"

    def test_oo_1_4_empty_name_error(self) -> None:
        """oo-1.4: empty object name rejected."""
        interp = TclInterp()
        with pytest.raises(TclError):
            interp.eval("oo::object create {}")


# ---------------------------------------------------------------------------
# oo-2.x: Constructors
# ---------------------------------------------------------------------------


class TestOO2:
    def test_oo_2_1_basic_constructor(self) -> None:
        """oo-2.1: constructor runs on creation."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable val
                constructor {v} { set val $v }
                method get {} { return $val }
            }
        """,
        )
        tcl_eval(interp, "Cls create obj hello")
        assert tcl_eval(interp, "obj get") == "hello"

    def test_oo_2_2_constructor_default(self) -> None:
        """oo-2.2: constructor with default args."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable val
                constructor {{v default}} { set val $v }
                method get {} { return $val }
            }
        """,
        )
        tcl_eval(interp, "Cls create obj")
        assert tcl_eval(interp, "obj get") == "default"

    def test_oo_2_3_constructor_chain(self) -> None:
        """oo-2.3: constructor next chain."""
        interp = TclInterp()
        tcl_eval(interp, "set ::trace {}")
        tcl_eval(
            interp,
            """
            oo::class create A {
                constructor {} { lappend ::trace A }
            }
            oo::class create B {
                superclass A
                constructor {} { lappend ::trace B; next }
            }
            B create obj
        """,
        )
        assert tcl_eval(interp, "set ::trace") == "B A"

    def test_oo_2_4_constructor_failure(self) -> None:
        """oo-2.4: constructor failure cleans up object."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                constructor {} { error "fail" }
            }
        """,
        )
        with pytest.raises(TclError, match="fail"):
            interp.eval("Cls create obj")
        with pytest.raises(TclError):
            interp.eval("obj ping")


# ---------------------------------------------------------------------------
# oo-3.x: Destructors
# ---------------------------------------------------------------------------


class TestOO3:
    def test_oo_3_1_basic_destructor(self) -> None:
        """oo-3.1: destructor runs on destroy."""
        interp = TclInterp()
        tcl_eval(interp, "set ::destroyed 0")
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                destructor { set ::destroyed 1 }
            }
            Cls create obj
        """,
        )
        tcl_eval(interp, "obj destroy")
        assert tcl_eval(interp, "set ::destroyed") == "1"

    def test_oo_3_2_destructor_chain(self) -> None:
        """oo-3.2: destructor next chain."""
        interp = TclInterp()
        tcl_eval(interp, "set ::trace {}")
        tcl_eval(
            interp,
            """
            oo::class create A {
                destructor { lappend ::trace A }
            }
            oo::class create B {
                superclass A
                destructor { lappend ::trace B; next }
            }
            B create obj
        """,
        )
        tcl_eval(interp, "obj destroy")
        assert tcl_eval(interp, "set ::trace") == "B A"

    def test_oo_3_3_destructor_reads_vars(self) -> None:
        """oo-3.3: destructor can read instance variables."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable name
                constructor {n} { set name $n }
                destructor { set ::last $name }
            }
            Cls create obj hello
        """,
        )
        tcl_eval(interp, "obj destroy")
        assert tcl_eval(interp, "set ::last") == "hello"


# ---------------------------------------------------------------------------
# oo-4.x: Method dispatch
# ---------------------------------------------------------------------------


class TestOO4:
    def test_oo_4_1_method_args(self) -> None:
        """oo-4.1: method with positional args."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method add {a b} { expr {$a + $b} }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj add 3 4") == "7"

    def test_oo_4_2_method_variadic(self) -> None:
        """oo-4.2: method with args parameter."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method gather {args} { return $args }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj gather a b c") == "a b c"

    def test_oo_4_3_method_wrong_args(self) -> None:
        """oo-4.3: wrong number of args."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method foo {a} { return $a }
            }
            Cls create obj
        """,
        )
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("obj foo")

    def test_oo_4_4_method_self(self) -> None:
        """oo-4.4: self returns current object."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method me {} { return [self] }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj me") == "::obj"


# ---------------------------------------------------------------------------
# oo-5.x: Method visibility
# ---------------------------------------------------------------------------


class TestOO5:
    def test_oo_5_1_unexport(self) -> None:
        """oo-5.1: unexported method blocked externally."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method secret {} { return s }
                unexport secret
            }
            Cls create obj
        """,
        )
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj secret")

    def test_oo_5_2_unexport_via_my(self) -> None:
        """oo-5.2: unexported accessible via my."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method secret {} { return s }
                method call {} { my secret }
                unexport secret
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj call") == "s"

    def test_oo_5_3_export_after_unexport(self) -> None:
        """oo-5.3: re-exporting method."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method m {} { return ok }
                unexport m
            }
            Cls create obj
        """,
        )
        with pytest.raises(TclError):
            interp.eval("obj m")
        tcl_eval(interp, "oo::define Cls export m")
        assert tcl_eval(interp, "obj m") == "ok"


# ---------------------------------------------------------------------------
# oo-7.x: Inheritance
# ---------------------------------------------------------------------------


class TestOO7:
    def test_oo_7_1_basic_inheritance(self) -> None:
        """oo-7.1: basic method inheritance."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method foo {} { return a }
            }
            oo::class create B {
                superclass A
            }
            B create obj
        """,
        )
        assert tcl_eval(interp, "obj foo") == "a"

    def test_oo_7_2_override(self) -> None:
        """oo-7.2: method override."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method foo {} { return a }
            }
            oo::class create B {
                superclass A
                method foo {} { return b }
            }
            B create obj
        """,
        )
        assert tcl_eval(interp, "obj foo") == "b"

    def test_oo_7_3_next(self) -> None:
        """oo-7.3: next calls parent method."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method foo {} { return a }
            }
            oo::class create B {
                superclass A
                method foo {} { return "b+[next]" }
            }
            B create obj
        """,
        )
        assert tcl_eval(interp, "obj foo") == "b+a"

    def test_oo_7_4_three_levels(self) -> None:
        """oo-7.4: three-level next chain."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method m {} { return A }
            }
            oo::class create B {
                superclass A
                method m {} { return "B+[next]" }
            }
            oo::class create C {
                superclass B
                method m {} { return "C+[next]" }
            }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "C+B+A"


# ---------------------------------------------------------------------------
# oo-8.x: Unknown method handler
# ---------------------------------------------------------------------------


class TestOO8:
    def test_oo_8_1_unknown(self) -> None:
        """oo-8.1: unknown catches missing methods."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method unknown {name args} { return "caught:$name" }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj missing") == "caught:missing"

    def test_oo_8_2_unknown_args(self) -> None:
        """oo-8.2: unknown receives args."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method unknown {name args} { return "$name:[llength $args]" }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj foo a b") == "foo:2"

    def test_oo_8_3_unknown_inherited(self) -> None:
        """oo-8.3: unknown handler inherited."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Base {
                method unknown {name args} { return "base:$name" }
            }
            oo::class create Sub {
                superclass Base
            }
            Sub create obj
        """,
        )
        assert tcl_eval(interp, "obj xyz") == "base:xyz"


# ---------------------------------------------------------------------------
# oo-9.x: Diamond inheritance / MRO
# ---------------------------------------------------------------------------


class TestOO9:
    def test_oo_9_1_diamond(self) -> None:
        """oo-9.1: diamond D(B,C)->A dispatch order."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method m {} { return A }
            }
            oo::class create B {
                superclass A
                method m {} { return "B+[next]" }
            }
            oo::class create C {
                superclass A
                method m {} { return "C+[next]" }
            }
            oo::class create D {
                superclass B C
                method m {} { return "D+[next]" }
            }
            D create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "D+B+C+A"


# ---------------------------------------------------------------------------
# oo-10.x: Variables
# ---------------------------------------------------------------------------


class TestOO10:
    def test_oo_10_1_variable_binding(self) -> None:
        """oo-10.1: class variable declaration."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable x
                constructor {} { set x 0 }
                method incr {} { incr x }
                method get {} { return $x }
            }
            Cls create c
        """,
        )
        tcl_eval(interp, "c incr")
        tcl_eval(interp, "c incr")
        assert tcl_eval(interp, "c get") == "2"

    def test_oo_10_2_variable_isolation(self) -> None:
        """oo-10.2: variables are per-instance."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable x
                constructor {} { set x 0 }
                method incr {} { incr x }
                method get {} { return $x }
            }
            Cls create a
            Cls create b
        """,
        )
        tcl_eval(interp, "a incr")
        tcl_eval(interp, "a incr")
        tcl_eval(interp, "b incr")
        assert tcl_eval(interp, "a get") == "2"
        assert tcl_eval(interp, "b get") == "1"


# ---------------------------------------------------------------------------
# oo-12.x: Forward methods
# ---------------------------------------------------------------------------


class TestOO12:
    def test_oo_12_1_forward_basic(self) -> None:
        """oo-12.1: forward method delegates to command."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                forward myLen string length
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj myLen hello") == "5"

    def test_oo_12_2_forward_with_args(self) -> None:
        """oo-12.2: forward prepends args."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                forward myRange string range
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj myRange hello 0 2") == "hel"

    def test_oo_12_3_forward_info(self) -> None:
        """oo-12.3: info class forward returns target."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                forward myLen string length
            }
        """,
        )
        assert tcl_eval(interp, "info class methodtype Cls myLen") == "forward"
        result = tcl_eval(interp, "info class forward Cls myLen")
        assert "string" in result and "length" in result


# ---------------------------------------------------------------------------
# oo-13.x: Filters
# ---------------------------------------------------------------------------


class TestOO13:
    def test_oo_13_1_filter_basic(self) -> None:
        """oo-13.1: filter intercepts method calls."""
        interp = TclInterp()
        tcl_eval(interp, "set ::log {}")
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method logF args {
                    lappend ::log "filtered"
                    next {*}$args
                }
                method foo {} { return ok }
                filter logF
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj foo") == "ok"
        assert tcl_eval(interp, "set ::log") == "filtered"

    def test_oo_13_2_filter_modify_return(self) -> None:
        """oo-13.2: filter can wrap return value."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method wrap args {
                    return "W([next {*}$args])"
                }
                method val {} { return 42 }
                filter wrap
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj val") == "W(42)"

    def test_oo_13_3_filter_chain(self) -> None:
        """oo-13.3: multiple filters chain correctly."""
        interp = TclInterp()
        tcl_eval(interp, "set ::order {}")
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method f1 args {
                    lappend ::order f1
                    next {*}$args
                }
                method f2 args {
                    lappend ::order f2
                    next {*}$args
                }
                method target {} {
                    lappend ::order target
                }
                filter f1
                filter f2
            }
            Cls create obj
        """,
        )
        tcl_eval(interp, "obj target")
        assert tcl_eval(interp, "set ::order") == "f1 f2 target"

    def test_oo_13_4_filter_inherited(self) -> None:
        """oo-13.4: filter inherited from parent class."""
        interp = TclInterp()
        tcl_eval(interp, "set ::filtered 0")
        tcl_eval(
            interp,
            """
            oo::class create Base {
                method f args {
                    set ::filtered 1
                    next {*}$args
                }
                filter f
            }
            oo::class create Sub {
                superclass Base
                method ping {} { return pong }
            }
            Sub create obj
        """,
        )
        assert tcl_eval(interp, "obj ping") == "pong"
        assert tcl_eval(interp, "set ::filtered") == "1"

    def test_oo_13_5_info_class_filters(self) -> None:
        """oo-13.5: info class filters."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method f1 args { next {*}$args }
                filter f1
            }
        """,
        )
        assert tcl_eval(interp, "info class filters Cls") == "f1"


# ---------------------------------------------------------------------------
# oo-14.x: Mixins
# ---------------------------------------------------------------------------


class TestOO14:
    def test_oo_14_1_mixin_override(self) -> None:
        """oo-14.1: mixin overrides class method."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Base {
                method m {} { return base }
            }
            oo::class create Mix {
                method m {} { return mix }
            }
            oo::class create C {
                superclass Base
                mixin Mix
            }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "mix"

    def test_oo_14_2_mixin_next(self) -> None:
        """oo-14.2: mixin calls next to class method."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Base {
                method m {} { return base }
            }
            oo::class create Mix {
                method m {} { return "mix+[next]" }
            }
            oo::class create C {
                superclass Base
                mixin Mix
            }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "mix+base"

    def test_oo_14_3_info_class_mixins(self) -> None:
        """oo-14.3: info class mixins returns mixin list."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Mix {}
            oo::class create Cls {
                mixin Mix
            }
        """,
        )
        result = tcl_eval(interp, "info class mixins Cls")
        assert "Mix" in result


# ---------------------------------------------------------------------------
# oo-15.x / oo-16.x: oo::objdefine
# ---------------------------------------------------------------------------


class TestOO15:
    def test_oo_15_1_objdefine_method(self) -> None:
        """oo-15.1: add method to instance."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        tcl_eval(interp, "oo::objdefine obj method foo {} { return bar }")
        assert tcl_eval(interp, "obj foo") == "bar"

    def test_oo_15_2_objdefine_forward(self) -> None:
        """oo-15.2: forward on instance."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        tcl_eval(interp, "oo::objdefine obj forward len string length")
        assert tcl_eval(interp, "obj len abc") == "3"

    def test_oo_15_3_objdefine_deletemethod(self) -> None:
        """oo-15.3: deletemethod on instance."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        tcl_eval(interp, "oo::objdefine obj method foo {} { return bar }")
        assert tcl_eval(interp, "obj foo") == "bar"
        tcl_eval(interp, "oo::objdefine obj deletemethod foo")
        with pytest.raises(TclError):
            interp.eval("obj foo")

    def test_oo_15_4_info_object_methods(self) -> None:
        """oo-15.4: info object methods lists instance methods."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        tcl_eval(interp, "oo::objdefine obj method foo {} { return bar }")
        tcl_eval(interp, "oo::objdefine obj method baz {} { return qux }")
        result = tcl_eval(interp, "info object methods obj")
        assert "baz" in result
        assert "foo" in result


# ---------------------------------------------------------------------------
# oo-17/18/19: info object/class
# ---------------------------------------------------------------------------


class TestOO17InfoObject:
    def test_oo_17_1_class(self) -> None:
        """oo-17.1: info object class."""
        interp = TclInterp()
        tcl_eval(interp, "oo::class create Cls {}")
        tcl_eval(interp, "Cls create obj")
        assert tcl_eval(interp, "info object class obj") == "::Cls"

    def test_oo_17_2_isa_object(self) -> None:
        """oo-17.2: info object isa object."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        assert tcl_eval(interp, "info object isa object obj") == "1"
        assert tcl_eval(interp, "info object isa object noexist") == "0"

    def test_oo_17_3_isa_typeof(self) -> None:
        """oo-17.3: info object isa typeof."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {}
            oo::class create B { superclass A }
            B create obj
        """,
        )
        assert tcl_eval(interp, "info object isa typeof obj B") == "1"
        assert tcl_eval(interp, "info object isa typeof obj A") == "1"

    def test_oo_17_4_namespace(self) -> None:
        """oo-17.4: info object namespace."""
        interp = TclInterp()
        tcl_eval(interp, "oo::object create obj")
        result = tcl_eval(interp, "info object namespace obj")
        assert result.startswith("::oo::Obj")

    def test_oo_17_5_vars(self) -> None:
        """oo-17.5: info object vars lists set variables."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable x y
                constructor {} { set x 1; set y 2 }
            }
            Cls create obj
        """,
        )
        result = tcl_eval(interp, "info object vars obj")
        assert "x" in result
        assert "y" in result


class TestOO18InfoClass:
    def test_oo_18_1_superclasses(self) -> None:
        """oo-18.1: info class superclasses."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {}
            oo::class create B { superclass A }
        """,
        )
        result = tcl_eval(interp, "info class superclasses B")
        assert "A" in result or "::A" in result

    def test_oo_18_2_methods(self) -> None:
        """oo-18.2: info class methods."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method foo {} {}
                method bar {} {}
            }
        """,
        )
        result = tcl_eval(interp, "info class methods Cls")
        assert "bar" in result
        assert "foo" in result

    def test_oo_18_3_instances(self) -> None:
        """oo-18.3: info class instances."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {}
            Cls create a
            Cls create b
        """,
        )
        result = tcl_eval(interp, "info class instances Cls")
        assert "::a" in result
        assert "::b" in result

    def test_oo_18_4_definition(self) -> None:
        """oo-18.4: info class definition."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method foo {x y} { return $x }
            }
        """,
        )
        result = tcl_eval(interp, "info class definition Cls foo")
        assert "x" in result
        assert "y" in result

    def test_oo_18_5_variables(self) -> None:
        """oo-18.5: info class variables."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable a b c
            }
        """,
        )
        result = tcl_eval(interp, "info class variables Cls")
        assert "a" in result
        assert "b" in result

    def test_oo_18_6_subclasses(self) -> None:
        """oo-18.6: info class subclasses."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {}
            oo::class create B { superclass A }
            oo::class create C { superclass A }
        """,
        )
        result = tcl_eval(interp, "info class subclasses A")
        assert "B" in result or "::B" in result
        assert "C" in result or "::C" in result


# ---------------------------------------------------------------------------
# oo-25.x: Destruction
# ---------------------------------------------------------------------------


class TestOO25:
    def test_oo_25_1_class_destroy_cascades(self) -> None:
        """oo-25.1: class destroy cascades to instances."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {}
            Cls create a
            Cls create b
        """,
        )
        tcl_eval(interp, "Cls destroy")
        with pytest.raises(TclError):
            interp.eval("a destroy")

    def test_oo_25_2_destroy_runs_dtor(self) -> None:
        """oo-25.2: instance destroy runs destructor."""
        interp = TclInterp()
        tcl_eval(interp, "set ::d 0")
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                destructor { set ::d 1 }
            }
            Cls create obj
        """,
        )
        tcl_eval(interp, "obj destroy")
        assert tcl_eval(interp, "set ::d") == "1"


# ---------------------------------------------------------------------------
# oo-26.x: oo::copy
# ---------------------------------------------------------------------------


class TestOO26:
    def test_oo_26_1_basic_copy(self) -> None:
        """oo-26.1: basic copy creates independent object."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable x
                constructor {v} { set x $v }
                method get {} { return $x }
            }
            Cls create orig 42
        """,
        )
        tcl_eval(interp, "oo::copy orig clone")
        assert tcl_eval(interp, "clone get") == "42"

    def test_oo_26_2_copy_independent(self) -> None:
        """oo-26.2: copy state is independent."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                variable x
                constructor {v} { set x $v }
                method get {} { return $x }
                method set {v} { set x $v }
            }
            Cls create orig 1
            oo::copy orig clone
        """,
        )
        tcl_eval(interp, "clone set 99")
        assert tcl_eval(interp, "orig get") == "1"
        assert tcl_eval(interp, "clone get") == "99"


# ---------------------------------------------------------------------------
# oo-37.x: TIP 500 Private methods
# ---------------------------------------------------------------------------


class TestOO37:
    def test_oo_37_1_private_via_my(self) -> None:
        """oo-37.1: private method accessible via my."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                private method secret {} { return s }
                method call {} { my secret }
            }
            Cls create obj
        """,
        )
        assert tcl_eval(interp, "obj call") == "s"

    def test_oo_37_2_private_blocked_external(self) -> None:
        """oo-37.2: private method blocked externally."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                private method secret {} { return s }
            }
            Cls create obj
        """,
        )
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj secret")

    def test_oo_37_3_private_not_in_subclass(self) -> None:
        """oo-37.3: private not accessible from subclass."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Base {
                private method secret {} { return s }
            }
            oo::class create Sub {
                superclass Base
                method try {} { my secret }
            }
            Sub create obj
        """,
        )
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj try")


# ---------------------------------------------------------------------------
# ooNext2.test: nextto
# ---------------------------------------------------------------------------


class TestOONext2:
    def test_nextto_1_specific_class(self) -> None:
        """nextto-1: jump to specific class."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method m {} { return A }
            }
            oo::class create B {
                superclass A
                method m {} { return B }
            }
            oo::class create C {
                superclass B
                method m {} { return "C+[nextto A]" }
            }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "C+A"

    def test_nextto_2_skip_intermediate(self) -> None:
        """nextto-2: skip intermediate class."""
        interp = TclInterp()
        tcl_eval(interp, "set ::trace {}")
        tcl_eval(
            interp,
            """
            oo::class create A {
                method m {} { lappend ::trace A }
            }
            oo::class create B {
                superclass A
                method m {} { lappend ::trace B; next }
            }
            oo::class create C {
                superclass B
                method m {} { lappend ::trace C; nextto A }
            }
            C create obj
        """,
        )
        tcl_eval(interp, "obj m")
        assert tcl_eval(interp, "set ::trace") == "C A"

    def test_nextto_3_unknown_class_error(self) -> None:
        """nextto-3: nextto unknown class raises error."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {
                method m {} { nextto NoSuchClass }
            }
            A create obj
        """,
        )
        with pytest.raises(TclError, match="is not a class"):
            interp.eval("obj m")

    def test_nextto_4_no_method_error(self) -> None:
        """nextto-4: nextto class without method raises error."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A {}
            oo::class create B {
                superclass A
                method m {} { nextto A }
            }
            B create obj
        """,
        )
        with pytest.raises(TclError, match="no non-filter implementation"):
            interp.eval("obj m")


# ---------------------------------------------------------------------------
# oo::define extensions
# ---------------------------------------------------------------------------


class TestOODefine:
    def test_define_method(self) -> None:
        """oo::define adds method."""
        interp = TclInterp()
        tcl_eval(interp, "oo::class create Cls {}")
        tcl_eval(interp, "oo::define Cls method foo {} { return bar }")
        tcl_eval(interp, "Cls create obj")
        assert tcl_eval(interp, "obj foo") == "bar"

    def test_define_superclass(self) -> None:
        """oo::define changes superclass."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create A { method m {} { return a } }
            oo::class create B {}
        """,
        )
        tcl_eval(interp, "oo::define B superclass A")
        tcl_eval(interp, "B create obj")
        assert tcl_eval(interp, "obj m") == "a"

    def test_define_mixin(self) -> None:
        """oo::define adds mixin."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create M { method m {} { return mixed } }
            oo::class create C {}
        """,
        )
        tcl_eval(interp, "oo::define C mixin M")
        tcl_eval(interp, "C create obj")
        assert tcl_eval(interp, "obj m") == "mixed"

    def test_define_deletemethod(self) -> None:
        """oo::define deletemethod removes method."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method foo {} { return f }
                method bar {} { return b }
            }
        """,
        )
        tcl_eval(interp, "oo::define Cls deletemethod foo")
        tcl_eval(interp, "Cls create obj")
        with pytest.raises(TclError):
            interp.eval("obj foo")
        assert tcl_eval(interp, "obj bar") == "b"

    def test_define_renamemethod(self) -> None:
        """oo::define renamemethod changes method name."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method foo {} { return result }
            }
        """,
        )
        tcl_eval(interp, "oo::define Cls renamemethod foo bar")
        tcl_eval(interp, "Cls create obj")
        with pytest.raises(TclError):
            interp.eval("obj foo")
        assert tcl_eval(interp, "obj bar") == "result"

    def test_define_forward(self) -> None:
        """oo::define forward adds forward method."""
        interp = TclInterp()
        tcl_eval(interp, "oo::class create Cls {}")
        tcl_eval(interp, "oo::define Cls forward len string length")
        tcl_eval(interp, "Cls create obj")
        assert tcl_eval(interp, "obj len abc") == "3"

    def test_define_variable(self) -> None:
        """oo::define variable adds variable."""
        interp = TclInterp()
        tcl_eval(interp, "oo::class create Cls {}")
        tcl_eval(interp, "oo::define Cls variable x")
        result = tcl_eval(interp, "info class variables Cls")
        assert "x" in result

    def test_define_filter(self) -> None:
        """oo::define filter adds filter."""
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create Cls {
                method f args { next {*}$args }
            }
        """,
        )
        tcl_eval(interp, "oo::define Cls filter f")
        assert tcl_eval(interp, "info class filters Cls") == "f"


# ---------------------------------------------------------------------------
# self subcommands
# ---------------------------------------------------------------------------


class TestSelf:
    def test_self_plain(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C { method m {} { self } }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "::obj"

    def test_self_object(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C { method m {} { self object } }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "::obj"

    def test_self_class(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C { method m {} { self class } }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj m") == "::C"

    def test_self_method(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C { method myM {} { self method } }
            C create obj
        """,
        )
        assert tcl_eval(interp, "obj myM") == "myM"

    def test_self_namespace(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C { method m {} { self namespace } }
            C create obj
        """,
        )
        result = tcl_eval(interp, "obj m")
        assert result.startswith("::oo::Obj")


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------


class TestOOErrors:
    def test_destroy_already_destroyed(self) -> None:
        interp = TclInterp()
        tcl_eval(interp, "oo::class create C {}")
        tcl_eval(interp, "C create obj")
        tcl_eval(interp, "obj destroy")
        with pytest.raises(TclError):
            interp.eval("obj destroy")

    def test_next_no_implementation(self) -> None:
        interp = TclInterp()
        tcl_eval(
            interp,
            """
            oo::class create C {
                method m {} { next }
            }
            C create obj
        """,
        )
        # oo::object doesn't have method m, so next should fail
        with pytest.raises(TclError, match="no next"):
            interp.eval("obj m")

    def test_self_outside(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="may only"):
            interp.eval("self")

    def test_my_outside(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="may only"):
            interp.eval("my foo")

    def test_info_class_unknown_subcmd(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="unknown or ambiguous"):
            interp.eval("info class nonexistent Cls")

    def test_info_object_unknown_subcmd(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="unknown or ambiguous"):
            interp.eval("info object nonexistent obj")
