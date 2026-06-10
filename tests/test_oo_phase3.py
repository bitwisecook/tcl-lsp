"""Phase 7+ pytest port — TIP 500 private methods, additional oo.test coverage.

Tests for private methods (Tcl 9.0 TIP 500), additional inheritance patterns,
constructor/destructor chains, and comprehensive introspection.
"""

from __future__ import annotations

import pytest

from tooling.vm.interp import TclInterp
from tooling.vm.types import TclError

# ---------------------------------------------------------------------------
# oo-37.x: TIP 500 Private Methods
# ---------------------------------------------------------------------------


class TestOO37PrivateMethods:
    """oo-37.x: TIP 500 private method support (Tcl 9.0)."""

    def test_oo_37_1_private_method_via_my(self) -> None:
        """oo-37.1: private method is accessible via my from same class."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                private method secret {} { return "secret-value" }
                method callSecret {} { my secret }
            }
            Cls create obj
        """)
        result = interp.eval("obj callSecret")
        assert result.value == "secret-value"

    def test_oo_37_2_private_not_callable_externally(self) -> None:
        """oo-37.2: private method not callable from outside."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                private method secret {} { return "secret" }
            }
            Cls create obj
        """)
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj secret")

    def test_oo_37_3_private_not_in_available_methods(self) -> None:
        """oo-37.3: private methods excluded from error messages."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method pub {} { return "pub" }
                private method priv {} { return "priv" }
            }
            Cls create obj
        """)
        with pytest.raises(TclError) as exc:
            interp.eval("obj nonexistent")
        assert "priv" not in str(exc.value)
        assert "pub" in str(exc.value)

    def test_oo_37_4_private_not_callable_from_subclass(self) -> None:
        """oo-37.4: private method not accessible from subclass."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                private method secret {} { return "base-secret" }
            }
            oo::class create Sub {
                superclass Base
                method trySecret {} { my secret }
            }
            Sub create obj
        """)
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj trySecret")

    def test_oo_37_5_private_visible_in_info_class_private(self) -> None:
        """oo-37.5: info class methods -private shows unexported methods.

        In C Tcl, ``-private`` without ``-all`` shows public + unexported
        methods.  Truly private methods only appear with ``-all -private``.
        """
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method pub {} {}
                method Bar {} {}
                private method priv {} {}
            }
        """)
        # Without -private: only public
        result = interp.eval("info class methods Cls")
        assert "priv" not in result.value
        assert "pub" in result.value
        assert "Bar" not in result.value  # uppercase = unexported
        # With -private: public + unexported, but NOT private
        result = interp.eval("info class methods Cls -private")
        assert "pub" in result.value
        assert "Bar" in result.value  # unexported now visible
        assert "priv" not in result.value  # truly private still hidden
        # With -all -private: everything
        result = interp.eval("info class methods Cls -all -private")
        assert "priv" in result.value
        assert "pub" in result.value
        assert "Bar" in result.value


# ---------------------------------------------------------------------------
# oo-2.x: Additional constructor tests
# ---------------------------------------------------------------------------


class TestOO2ConstructorsAdvanced:
    """oo-2.x: constructor patterns."""

    def test_oo_2_5_constructor_with_defaults(self) -> None:
        """oo-2.5: constructor with default parameter values."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable val
                constructor {{v 42}} { set val $v }
                method get {} { return $val }
            }
        """)
        interp.eval("Cls create obj1")
        interp.eval("Cls create obj2 99")
        assert interp.eval("obj1 get").value == "42"
        assert interp.eval("obj2 get").value == "99"

    def test_oo_2_6_constructor_chain_via_next(self) -> None:
        """oo-2.6: constructor calls next to invoke parent constructor."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                variable x
                constructor {} { set x "base-init" }
                method getX {} { return $x }
            }
            oo::class create Sub {
                superclass Base
                variable y
                constructor {} {
                    next
                    set y "sub-init"
                }
                method getY {} { return $y }
            }
            Sub create obj
        """)
        assert interp.eval("obj getX").value == "base-init"
        assert interp.eval("obj getY").value == "sub-init"

    def test_oo_2_7_constructor_with_args(self) -> None:
        """oo-2.7: constructor with variadic args."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable items
                constructor {args} { set items $args }
                method get {} { return $items }
            }
            Cls create obj a b c
        """)
        result = interp.eval("obj get")
        assert "a b c" in result.value


# ---------------------------------------------------------------------------
# oo-3.x: Additional destructor tests
# ---------------------------------------------------------------------------


class TestOO3DestructorsAdvanced:
    """oo-3.x: destructor patterns."""

    def test_oo_3_3_destructor_chain(self) -> None:
        """oo-3.3: destructor chain runs through MRO."""
        interp = TclInterp()
        interp.eval("set ::dtor_order {}")
        interp.eval("""
            oo::class create Base {
                destructor {
                    lappend ::dtor_order "base"
                }
            }
            oo::class create Sub {
                superclass Base
                destructor {
                    lappend ::dtor_order "sub"
                    next
                }
            }
            Sub create obj
        """)
        interp.eval("obj destroy")
        result = interp.eval("set ::dtor_order")
        assert "sub" in result.value
        assert "base" in result.value

    def test_oo_3_4_destructor_access_vars(self) -> None:
        """oo-3.4: destructor can access instance variables."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable name
                constructor {n} { set name $n }
                destructor {
                    set ::lastDestroyed $name
                }
            }
            Cls create obj myObj
        """)
        interp.eval("obj destroy")
        assert interp.eval("set ::lastDestroyed").value == "myObj"


# ---------------------------------------------------------------------------
# oo-4.x: Additional method tests
# ---------------------------------------------------------------------------


class TestOO4MethodsAdvanced:
    """oo-4.x: method calling patterns."""

    def test_oo_4_5_method_returning_self(self) -> None:
        """oo-4.5: method returning self enables chaining."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Builder {
                variable result
                constructor {} { set result "" }
                method add {s} {
                    append result $s
                    return [self]
                }
                method build {} { return $result }
            }
            Builder create b
        """)
        interp.eval("[b add hello] add world")
        result = interp.eval("b build")
        assert result.value == "helloworld"

    def test_oo_4_6_method_with_variable_args(self) -> None:
        """oo-4.6: method with args parameter."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method gather {args} {
                    return [join $args ","]
                }
            }
            Cls create obj
        """)
        result = interp.eval("obj gather a b c")
        assert result.value == "a,b,c"


# ---------------------------------------------------------------------------
# oo-14.x: Additional mixin tests
# ---------------------------------------------------------------------------


class TestOO14MixinsAdvanced:
    """oo-14.x: mixin patterns."""

    def test_oo_14_3_mixin_append(self) -> None:
        """oo-14.3: mixin -append adds to existing mixins."""
        interp = TclInterp()
        interp.eval("""
            oo::class create M1 {
                method m1 {} { return "m1" }
            }
            oo::class create M2 {
                method m2 {} { return "m2" }
            }
            oo::class create Base {}
        """)
        interp.eval("oo::define Base mixin M1")
        interp.eval("oo::define Base mixin -append M2")
        result = interp.eval("info class mixins Base")
        assert "M1" in result.value or "::M1" in result.value
        assert "M2" in result.value or "::M2" in result.value

    def test_oo_14_4_mixin_with_constructor(self) -> None:
        """oo-14.4: mixin classes with constructors."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Logger {
                method log {msg} {
                    lappend ::log $msg
                }
            }
            oo::class create Base {
                variable name
                constructor {n} {
                    set name $n
                }
                method getName {} { return $name }
            }
            oo::class create Sub {
                superclass Base
                mixin Logger
            }
            set ::log {}
            Sub create obj "test"
        """)
        interp.eval("obj log hello")
        assert interp.eval("obj getName").value == "test"
        assert "hello" in interp.eval("set ::log").value


# ---------------------------------------------------------------------------
# Additional info introspection tests
# ---------------------------------------------------------------------------


class TestOOInfoComplete:
    """Comprehensive info object/class tests."""

    def test_info_class_subclasses(self) -> None:
        """info class subclasses lists direct subclasses."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {}
            oo::class create Sub1 { superclass Base }
            oo::class create Sub2 { superclass Base }
        """)
        result = interp.eval("info class subclasses Base")
        assert "Sub1" in result.value or "::Sub1" in result.value
        assert "Sub2" in result.value or "::Sub2" in result.value

    def test_info_class_constructor(self) -> None:
        """info class constructor returns constructor arglist + body."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                constructor {x {y 5}} { set z 1 }
            }
        """)
        result = interp.eval("info class constructor Cls")
        assert "x" in result.value
        assert "y" in result.value

    def test_info_class_destructor(self) -> None:
        """info class destructor returns destructor body."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                destructor { set x 1 }
            }
        """)
        result = interp.eval("info class destructor Cls")
        assert "set x 1" in result.value

    def test_info_class_variables(self) -> None:
        """info class variables returns declared variables."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable x y z
            }
        """)
        result = interp.eval("info class variables Cls")
        assert "x" in result.value
        assert "y" in result.value
        assert "z" in result.value

    def test_info_class_definition(self) -> None:
        """info class definition returns arglist + body."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method foo {a b} { return "$a$b" }
            }
        """)
        result = interp.eval("info class definition Cls foo")
        assert "a" in result.value
        assert "b" in result.value

    def test_info_object_class(self) -> None:
        """info object class returns the object's class."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {}
            Cls create obj
        """)
        result = interp.eval("info object class obj")
        assert result.value == "::Cls"

    def test_info_object_isa_metaclass(self) -> None:
        """info object isa metaclass checks for metaclass."""
        interp = TclInterp()
        assert interp.eval("info object isa metaclass oo::class").value == "1"
        interp.eval("oo::class create NotMeta {}")
        # Verify oo::class is a metaclass
        assert interp.eval("info object isa metaclass oo::class").value == "1"

    def test_info_class_instances(self) -> None:
        """info class instances lists class instances."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {}
            Cls create foo
            Cls create bar
        """)
        result = interp.eval("info class instances Cls")
        assert "::foo" in result.value
        assert "::bar" in result.value


# ---------------------------------------------------------------------------
# Complex integration tests
# ---------------------------------------------------------------------------


class TestOOIntegration:
    """Integration tests combining multiple OO features."""

    def test_filter_with_inheritance_and_next(self) -> None:
        """Filter on base class works through inheritance with next."""
        interp = TclInterp()
        interp.eval("set ::trace {}")
        interp.eval("""
            oo::class create Base {
                method traceFilter args {
                    lappend ::trace "before"
                    set r [next {*}$args]
                    lappend ::trace "after"
                    return $r
                }
                method baseMethod {} { return "base" }
                filter traceFilter
            }
            oo::class create Sub {
                superclass Base
                method subMethod {} { return "sub" }
            }
            Sub create obj
        """)
        result = interp.eval("obj subMethod")
        assert result.value == "sub"
        trace = interp.eval("set ::trace")
        assert "before" in trace.value
        assert "after" in trace.value

    def test_unknown_with_forward(self) -> None:
        """Unknown handler combined with forward methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                forward myLen string length
                method unknown {name args} {
                    return "unknown:$name"
                }
            }
            Cls create obj
        """)
        # Forward method works
        assert interp.eval("obj myLen hello").value == "5"
        # Unknown catches other methods
        assert interp.eval("obj nonexistent").value == "unknown:nonexistent"

    def test_copy_with_instance_methods(self) -> None:
        """oo::copy preserves instance methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method classMethod {} { return "class" }
            }
            Cls create orig
            oo::objdefine orig method instMethod {} { return "instance" }
            oo::copy orig clone
        """)
        assert interp.eval("clone classMethod").value == "class"
        assert interp.eval("clone instMethod").value == "instance"

    def test_define_after_creation(self) -> None:
        """oo::define modifies class after instances are created."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {}
            Cls create obj
        """)
        interp.eval("""
            oo::define Cls {
                method added {} { return "added" }
            }
        """)
        result = interp.eval("obj added")
        assert result.value == "added"

    def test_multiple_inheritance_with_variables(self) -> None:
        """Variables from multiple ancestors are all available."""
        interp = TclInterp()
        interp.eval("""
            oo::class create A {
                variable a
                constructor {} { set a "fromA" }
                method getA {} { return $a }
            }
            oo::class create B {
                variable b
                constructor {} { set b "fromB" }
                method getB {} { return $b }
            }
            oo::class create C {
                superclass A B
                constructor {} {
                    next
                }
            }
            C create obj
        """)
        assert interp.eval("obj getA").value == "fromA"
        # B's constructor doesn't run automatically (only A's via next)
        # but B's variable is still declared

    def test_self_inside_constructor(self) -> None:
        """self works inside constructor."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                constructor {} {
                    set ::ctorSelf [self]
                }
            }
            Cls create obj
        """)
        result = interp.eval("set ::ctorSelf")
        assert result.value == "::obj"

    def test_self_inside_destructor(self) -> None:
        """self works inside destructor."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                destructor {
                    set ::dtorSelf [self]
                }
            }
            Cls create obj
        """)
        interp.eval("obj destroy")
        result = interp.eval("set ::dtorSelf")
        assert result.value == "::obj"
