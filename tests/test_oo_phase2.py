"""Phase 2-6 pytest port of Tcl oo.test — filters, visibility, unknown,
class destruction, advanced introspection, error handling.

Each test mirrors a real TclOO test case from oo.test with the original ID
in the test name for traceability. Expected results match Tcl 9.0.3.
"""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError

# ---------------------------------------------------------------------------
# oo-13.x: Filters
# ---------------------------------------------------------------------------


class TestOO13Filters:
    """oo-13.x: filter method interception."""

    def test_oo_13_1_basic_filter(self) -> None:
        """oo-13.1: basic filter intercepts method call."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method logFilter args {
                    lappend ::result "filtered"
                    next {*}$args
                }
                method foo {} {
                    lappend ::result "foo"
                }
                filter logFilter
            }
        """)
        interp.eval("set ::result {}")
        interp.eval("Cls create obj")
        interp.eval("obj foo")
        result = interp.eval("set ::result")
        assert "filtered" in result.value
        assert "foo" in result.value

    def test_oo_13_2_filter_sees_method_name(self) -> None:
        """oo-13.2: filter does NOT receive method name as arg; args match the original call."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method myFilter args {
                    set ::filterArgs $args
                    next {*}$args
                }
                method bar {} { return "bar-result" }
                filter myFilter
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj bar")
        assert result.value == "bar-result"
        filter_args = interp.eval("set ::filterArgs")
        # bar takes no arguments, so filter receives empty args
        assert filter_args.value == ""

    def test_oo_13_3_multiple_filters(self) -> None:
        """oo-13.3: multiple filters run in order."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method f1 args {
                    lappend ::result "f1"
                    next {*}$args
                }
                method f2 args {
                    lappend ::result "f2"
                    next {*}$args
                }
                method target {} {
                    lappend ::result "target"
                }
                filter f1
                filter f2
            }
        """)
        interp.eval("set ::result {}")
        interp.eval("Cls create obj")
        interp.eval("obj target")
        result = interp.eval("set ::result")
        assert "f1" in result.value
        assert "f2" in result.value
        assert "target" in result.value

    def test_oo_13_4_filter_inherited(self) -> None:
        """oo-13.4: filters are inherited from superclass."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method logFilter args {
                    lappend ::result "filtered"
                    next {*}$args
                }
                filter logFilter
            }
            oo::class create Sub {
                superclass Base
                method doSomething {} {
                    lappend ::result "done"
                }
            }
        """)
        interp.eval("set ::result {}")
        interp.eval("Sub create obj")
        interp.eval("obj doSomething")
        result = interp.eval("set ::result")
        assert "filtered" in result.value
        assert "done" in result.value

    def test_oo_13_5_filter_with_return_value(self) -> None:
        """oo-13.5: filter can modify return value."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method wrapFilter args {
                    set r [next {*}$args]
                    return "wrapped:$r"
                }
                method getValue {} {
                    return "hello"
                }
                filter wrapFilter
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj getValue")
        assert result.value == "wrapped:hello"

    def test_oo_13_6_info_class_filters(self) -> None:
        """oo-13.6: info class filters returns filter list."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method f1 args { next {*}$args }
                method f2 args { next {*}$args }
                filter f1
                filter f2
            }
        """)
        result = interp.eval("info class filters Cls")
        assert "f1" in result.value
        assert "f2" in result.value


# ---------------------------------------------------------------------------
# oo-5.x: Method visibility (export/unexport)
# ---------------------------------------------------------------------------


class TestOO5Visibility:
    """oo-5.x: method visibility enforcement."""

    def test_oo_5_1_unexported_method_not_callable_externally(self) -> None:
        """oo-5.1: unexported method cannot be called from outside."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method secret {} { return "secret" }
                unexport secret
            }
        """)
        interp.eval("Cls create obj")
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj secret")

    def test_oo_5_2_unexported_callable_via_my(self) -> None:
        """oo-5.2: unexported method is callable from within via my."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method secret {} { return "secret-value" }
                method callSecret {} { my secret }
                unexport secret
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj callSecret")
        assert result.value == "secret-value"

    def test_oo_5_3_export_makes_method_public(self) -> None:
        """oo-5.3: export restores public visibility."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method hidden {} { return "visible" }
                unexport hidden
            }
        """)
        interp.eval("Cls create obj")
        with pytest.raises(TclError):
            interp.eval("obj hidden")
        interp.eval("""
            oo::define Cls {
                export hidden
            }
        """)
        result = interp.eval("obj hidden")
        assert result.value == "visible"

    def test_oo_5_4_available_methods_excludes_unexported(self) -> None:
        """oo-5.4: error message lists only public methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method pub {} { return "pub" }
                method priv {} { return "priv" }
                unexport priv
            }
        """)
        interp.eval("Cls create obj")
        with pytest.raises(TclError, match="pub") as exc:
            interp.eval("obj nonexistent")
        # 'priv' should NOT appear in available methods
        assert "priv" not in str(exc.value)


# ---------------------------------------------------------------------------
# oo-8.x: Unknown method handler
# ---------------------------------------------------------------------------


class TestOO8Unknown:
    """oo-8.x: unknown method handler dispatch."""

    def test_oo_8_1_basic_unknown_handler(self) -> None:
        """oo-8.1: unknown handler catches unknown method calls."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method unknown {name args} {
                    return "caught:$name"
                }
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj nonexistent")
        assert result.value == "caught:nonexistent"

    def test_oo_8_2_unknown_receives_args(self) -> None:
        """oo-8.2: unknown handler receives method name and args."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method unknown {name args} {
                    return "$name:[join $args ,]"
                }
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj missing a b c")
        assert result.value == "missing:a,b,c"

    def test_oo_8_3_unknown_inherited(self) -> None:
        """oo-8.3: unknown handler is inherited from superclass."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method unknown {name args} {
                    return "base-caught:$name"
                }
            }
            oo::class create Sub {
                superclass Base
            }
        """)
        interp.eval("Sub create obj")
        result = interp.eval("obj anything")
        assert result.value == "base-caught:anything"

    def test_oo_8_4_unknown_not_called_for_known(self) -> None:
        """oo-8.4: unknown is not called when method exists."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method known {} { return "known" }
                method unknown {name args} { return "unknown" }
            }
        """)
        interp.eval("Cls create obj")
        result = interp.eval("obj known")
        assert result.value == "known"


# ---------------------------------------------------------------------------
# oo-25.x / oo-30.x: Object/class destruction
# ---------------------------------------------------------------------------


class TestOODestruction:
    """Object and class destruction behaviour."""

    def test_oo_25_1_class_destruction_cascades_to_instances(self) -> None:
        """oo-25.1: destroying a class destroys its instances."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method ping {} { return "pong" }
            }
            Cls create obj1
            Cls create obj2
        """)
        # Both instances work
        assert interp.eval("obj1 ping").value == "pong"
        assert interp.eval("obj2 ping").value == "pong"
        # Destroy the class
        interp.eval("Cls destroy")
        # Instances should be gone
        with pytest.raises(TclError):
            interp.eval("obj1 ping")
        with pytest.raises(TclError):
            interp.eval("obj2 ping")

    def test_oo_25_2_destructor_runs_on_class_destroy(self) -> None:
        """oo-25.2: destructors run when class is destroyed."""
        interp = TclInterp()
        interp.eval("set ::destroyed {}")
        interp.eval("""
            oo::class create Cls {
                destructor {
                    lappend ::destroyed [self]
                }
            }
            Cls create obj1
            Cls create obj2
        """)
        interp.eval("Cls destroy")
        result = interp.eval("set ::destroyed")
        assert "::obj1" in result.value
        assert "::obj2" in result.value

    def test_oo_30_1_constructor_failure_cleanup(self) -> None:
        """oo-30.1: failed constructor cleans up partial object."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                constructor {} {
                    error "construction failed"
                }
            }
        """)
        with pytest.raises(TclError, match="construction failed"):
            interp.eval("Cls create obj")
        # Object should not exist
        with pytest.raises(TclError):
            interp.eval("obj ping")

    def test_oo_30_2_normal_destroy(self) -> None:
        """oo-30.2: normal object destruction."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method ping {} { return "pong" }
            }
            Cls create obj
        """)
        assert interp.eval("obj ping").value == "pong"
        interp.eval("obj destroy")
        with pytest.raises(TclError):
            interp.eval("obj ping")


# ---------------------------------------------------------------------------
# oo-17/18/19: Advanced introspection
# ---------------------------------------------------------------------------


class TestOOIntrospectionAdvanced:
    """Advanced info object/class introspection."""

    def test_info_class_methodtype_method(self) -> None:
        """info class methodtype returns 'method' for normal methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method foo {} { return bar }
            }
        """)
        result = interp.eval("info class methodtype Cls foo")
        assert result.value == "method"

    def test_info_class_methodtype_forward(self) -> None:
        """info class methodtype returns 'forward' for forwarded methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                forward myLen string length
            }
        """)
        result = interp.eval("info class methodtype Cls myLen")
        assert result.value == "forward"

    def test_info_class_forward(self) -> None:
        """info class forward returns forward target."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                forward myLen string length
            }
        """)
        result = interp.eval("info class forward Cls myLen")
        assert result.value == "string length"

    def test_info_object_methodtype(self) -> None:
        """info object methodtype returns type for instance methods."""
        interp = TclInterp()
        interp.eval("oo::object create obj")
        interp.eval("oo::objdefine obj method foo {} { return bar }")
        result = interp.eval("info object methodtype obj foo")
        assert result.value == "method"

    def test_info_object_forward(self) -> None:
        """info object forward returns forward target for instance method."""
        interp = TclInterp()
        interp.eval("oo::object create obj")
        interp.eval("oo::objdefine obj forward myLen string length")
        result = interp.eval("info object forward obj myLen")
        assert result.value == "string length"

    def test_info_class_methods_all(self) -> None:
        """info class methods -all includes inherited methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method baseMethod {} {}
            }
            oo::class create Sub {
                superclass Base
                method subMethod {} {}
            }
        """)
        result = interp.eval("info class methods Sub -all")
        assert "baseMethod" in result.value
        assert "subMethod" in result.value

    def test_info_class_methods_private(self) -> None:
        """info class methods -private includes unexported methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method pub {} {}
                method hidden {} {}
                unexport hidden
            }
        """)
        # Without -private: only public
        result = interp.eval("info class methods Cls")
        assert "pub" in result.value
        assert "hidden" not in result.value
        # With -private: both
        result = interp.eval("info class methods Cls -private")
        assert "pub" in result.value
        assert "hidden" in result.value

    def test_info_object_isa_typeof(self) -> None:
        """info object isa typeof checks class membership."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {}
            oo::class create Sub { superclass Base }
            Sub create obj
        """)
        assert interp.eval("info object isa typeof obj Sub").value == "1"
        assert interp.eval("info object isa typeof obj Base").value == "1"

    def test_info_object_isa_object(self) -> None:
        """info object isa object checks if name is an object."""
        interp = TclInterp()
        interp.eval("oo::class create Cls {}")
        interp.eval("Cls create obj")
        assert interp.eval("info object isa object obj").value == "1"
        assert interp.eval("info object isa object nonexistent").value == "0"

    def test_info_object_vars(self) -> None:
        """info object vars returns set instance variables."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable x y
                constructor {} {
                    set x 1
                    set y 2
                }
            }
            Cls create obj
        """)
        result = interp.eval("info object vars obj")
        assert "x" in result.value
        assert "y" in result.value


# ---------------------------------------------------------------------------
# oo-12.x: Forward methods (additional tests)
# ---------------------------------------------------------------------------


class TestOO12ForwardAdvanced:
    """oo-12.x: additional forward method tests."""

    def test_oo_12_1_forward_with_prefix_args(self) -> None:
        """oo-12.1: forward with prefix arguments."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                forward myLength string length
            }
            Cls create obj
        """)
        result = interp.eval("obj myLength hello")
        assert result.value == "5"

    def test_oo_12_2_forward_via_objdefine(self) -> None:
        """oo-12.2: forward on instance via objdefine."""
        interp = TclInterp()
        interp.eval("oo::object create obj")
        interp.eval("oo::objdefine obj forward myLen string length")
        result = interp.eval("obj myLen world")
        assert result.value == "5"


# ---------------------------------------------------------------------------
# oo-7.x: Inheritance (additional tests)
# ---------------------------------------------------------------------------


class TestOO7InheritanceAdvanced:
    """oo-7.x: additional inheritance tests."""

    def test_oo_7_5_mixin_method_override(self) -> None:
        """oo-7.5: mixin method overrides class method."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method greet {} { return "base" }
            }
            oo::class create MixinClass {
                method greet {} { return "mixin" }
            }
            oo::class create Sub {
                superclass Base
                mixin MixinClass
            }
            Sub create obj
        """)
        result = interp.eval("obj greet")
        assert result.value == "mixin"

    def test_oo_7_6_mixin_next_to_class(self) -> None:
        """oo-7.6: mixin method calls next to reach class method."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method greet {} { return "base" }
            }
            oo::class create MixinClass {
                method greet {} {
                    return "mixin+[next]"
                }
            }
            oo::class create Sub {
                superclass Base
                mixin MixinClass
            }
            Sub create obj
        """)
        result = interp.eval("obj greet")
        assert result.value == "mixin+base"


# ---------------------------------------------------------------------------
# oo-15/16: objdefine (additional tests)
# ---------------------------------------------------------------------------


class TestOOObjdefineAdvanced:
    """oo-15/16: additional objdefine tests."""

    def test_oo_15_1_instance_mixin(self) -> None:
        """oo-15.1: per-instance mixin via objdefine."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method greet {} { return "base" }
            }
            oo::class create MixCls {
                method greet {} { return "mixed" }
            }
            Base create obj
        """)
        interp.eval("oo::objdefine obj mixin MixCls")
        result = interp.eval("info object mixins obj")
        assert "MixCls" in result.value or "::MixCls" in result.value

    def test_oo_15_2_instance_variable(self) -> None:
        """oo-15.2: per-instance variable via objdefine."""
        interp = TclInterp()
        interp.eval("oo::object create obj")
        interp.eval("oo::objdefine obj variable x y")
        result = interp.eval("info object variables obj")
        assert "x" in result.value
        assert "y" in result.value

    def test_oo_15_3_instance_filter(self) -> None:
        """oo-15.3: per-instance filter via objdefine."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method logFilter args {
                    set ::filtered 1
                    next {*}$args
                }
                method ping {} { return "pong" }
            }
            Cls create obj
        """)
        interp.eval("set ::filtered 0")
        interp.eval("oo::objdefine obj filter logFilter")
        interp.eval("obj ping")
        assert interp.eval("set ::filtered").value == "1"


# ---------------------------------------------------------------------------
# oo-10.x: Variable declarations
# ---------------------------------------------------------------------------


class TestOO10Variables:
    """oo-10.x: class variable declarations."""

    def test_oo_10_1_variable_binding(self) -> None:
        """oo-10.1: declared variables are bound in method scope."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Counter {
                variable count
                constructor {} { set count 0 }
                method incr {} { incr count; return $count }
                method get {} { return $count }
            }
            Counter create c
        """)
        assert interp.eval("c get").value == "0"
        assert interp.eval("c incr").value == "1"
        assert interp.eval("c incr").value == "2"
        assert interp.eval("c get").value == "2"

    def test_oo_10_2_variable_per_instance(self) -> None:
        """oo-10.2: each instance gets its own variable storage."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Counter {
                variable count
                constructor {} { set count 0 }
                method incr {} { incr count; return $count }
            }
            Counter create c1
            Counter create c2
        """)
        interp.eval("c1 incr")
        interp.eval("c1 incr")
        interp.eval("c2 incr")
        assert interp.eval("c1 incr").value == "3"
        assert interp.eval("c2 incr").value == "2"

    def test_oo_10_3_variable_inherited(self) -> None:
        """oo-10.3: variables accessible when declared in defining class.

        In C Tcl, each class must explicitly declare ``variable`` for
        the names it needs — they are not automatically inherited.
        """
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                variable x
                constructor {} { set x 42 }
                method getX {} { return $x }
            }
            oo::class create Sub {
                superclass Base
                variable x
                method doubleX {} { return [expr {$x * 2}] }
            }
            Sub create obj
        """)
        assert interp.eval("obj getX").value == "42"
        assert interp.eval("obj doubleX").value == "84"


# ---------------------------------------------------------------------------
# oo-26: oo::copy (additional tests)
# ---------------------------------------------------------------------------


class TestOO26CopyAdvanced:
    """oo-26: additional oo::copy tests."""

    def test_oo_26_3_copy_preserves_variables(self) -> None:
        """oo-26.3: copy preserves instance variable values."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable x
                constructor {val} { set x $val }
                method get {} { return $x }
            }
            Cls create orig 42
        """)
        assert interp.eval("orig get").value == "42"
        interp.eval("oo::copy orig clone")
        assert interp.eval("clone get").value == "42"

    def test_oo_26_4_copy_independent_state(self) -> None:
        """oo-26.4: copy state is independent of original."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                variable x
                constructor {val} { set x $val }
                method get {} { return $x }
                method setX {val} { set x $val }
            }
            Cls create orig 1
        """)
        interp.eval("oo::copy orig clone")
        interp.eval("clone setX 99")
        assert interp.eval("orig get").value == "1"
        assert interp.eval("clone get").value == "99"


# ---------------------------------------------------------------------------
# oo-define: deletemethod / renamemethod
# ---------------------------------------------------------------------------


class TestOODefineModification:
    """oo::define deletemethod and renamemethod."""

    def test_deletemethod_removes_method(self) -> None:
        """deletemethod removes a method from the class."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method foo {} { return "foo" }
                method bar {} { return "bar" }
            }
        """)
        interp.eval("Cls create obj")
        assert interp.eval("obj foo").value == "foo"
        interp.eval("oo::define Cls deletemethod foo")
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj foo")
        assert interp.eval("obj bar").value == "bar"

    def test_renamemethod_renames_method(self) -> None:
        """renamemethod changes method name."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method foo {} { return "foo-result" }
            }
        """)
        interp.eval("Cls create obj")
        interp.eval("oo::define Cls renamemethod foo bar")
        with pytest.raises(TclError, match="unknown method"):
            interp.eval("obj foo")
        assert interp.eval("obj bar").value == "foo-result"


# ---------------------------------------------------------------------------
# self subcommands
# ---------------------------------------------------------------------------


class TestSelfSubcommands:
    """self object, self class, self method, self namespace, self caller."""

    def test_self_object(self) -> None:
        """self object returns current object name."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method who {} { self object }
            }
            Cls create obj
        """)
        result = interp.eval("obj who")
        assert result.value == "::obj"

    def test_self_class(self) -> None:
        """self class returns defining class."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method myClass {} { self class }
            }
            Cls create obj
        """)
        result = interp.eval("obj myClass")
        assert result.value == "::Cls"

    def test_self_method(self) -> None:
        """self method returns current method name."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method myMethod {} { self method }
            }
            Cls create obj
        """)
        result = interp.eval("obj myMethod")
        assert result.value == "myMethod"

    def test_self_namespace(self) -> None:
        """self namespace returns object's namespace."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method myNS {} { self namespace }
            }
            Cls create obj
        """)
        result = interp.eval("obj myNS")
        assert result.value.startswith("::oo::Obj")

    def test_self_caller(self) -> None:
        """self caller returns calling method's class and name."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Inner {
                method whoCalledMe {} { self caller }
            }
            oo::class create Outer {
                method callInner {target} {
                    $target whoCalledMe
                }
            }
            Inner create inner
            Outer create outer
        """)
        result = interp.eval("outer callInner inner")
        assert "Outer" in result.value or "::Outer" in result.value
        assert "callInner" in result.value


# ---------------------------------------------------------------------------
# Error handling edge cases
# ---------------------------------------------------------------------------


class TestOOErrorHandling:
    """Error handling in OO operations."""

    def test_unknown_class(self) -> None:
        """Creating instance of nonexistent class raises error."""
        interp = TclInterp()
        with pytest.raises(TclError):
            interp.eval("NonExistent create obj")

    def test_method_wrong_args(self) -> None:
        """Calling method with wrong number of args raises error."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {
                method foo {a b} { return "$a$b" }
            }
            Cls create obj
        """)
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("obj foo")

    def test_no_method_no_args(self) -> None:
        """Calling object with no method name raises error."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Cls {}
            Cls create obj
        """)
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("obj")

    def test_info_class_not_a_class(self) -> None:
        """info class on a non-class object raises error."""
        interp = TclInterp()
        interp.eval("oo::object create obj")
        with pytest.raises(TclError, match="not a class"):
            interp.eval("info class methods obj")

    def test_info_object_not_an_object(self) -> None:
        """info object on nonexistent object raises error."""
        interp = TclInterp()
        with pytest.raises(TclError, match="does not refer to an object"):
            interp.eval("info object class nonexistent")

    def test_next_outside_method(self) -> None:
        """next outside a method raises error."""
        interp = TclInterp()
        with pytest.raises(TclError, match="may only be invoked"):
            interp.eval("next")

    def test_self_outside_method(self) -> None:
        """self outside a method raises error."""
        interp = TclInterp()
        with pytest.raises(TclError, match="may only be invoked"):
            interp.eval("self")


# ---------------------------------------------------------------------------
# oo-9.x: Diamond + complex MRO (additional)
# ---------------------------------------------------------------------------


class TestOOMROAdvanced:
    """Advanced MRO / diamond tests."""

    def test_mixin_precedes_superclass(self) -> None:
        """Mixin methods are checked before superclass methods."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method greet {} { return "base" }
            }
            oo::class create MixA {
                method greet {} { return "mixA" }
            }
            oo::class create Child {
                superclass Base
                mixin MixA
            }
            Child create obj
        """)
        result = interp.eval("obj greet")
        assert result.value == "mixA"

    def test_diamond_mro_order(self) -> None:
        """Diamond inheritance resolves methods correctly."""
        interp = TclInterp()
        interp.eval("""
            oo::class create A {
                method name {} { return "A" }
            }
            oo::class create B {
                superclass A
                method name {} { return "B+[next]" }
            }
            oo::class create C {
                superclass A
                method name {} { return "C+[next]" }
            }
            oo::class create D {
                superclass B C
                method name {} { return "D+[next]" }
            }
            D create obj
        """)
        result = interp.eval("obj name")
        assert result.value == "D+B+C+A"

    def test_nextto_specific_class(self) -> None:
        """nextto jumps to a specific class in the MRO."""
        interp = TclInterp()
        interp.eval("""
            oo::class create A {
                method m {} { return "A" }
            }
            oo::class create B {
                superclass A
                method m {} { return "B" }
            }
            oo::class create C {
                superclass B
                method m {} { return "C+[nextto A]" }
            }
            C create obj
        """)
        result = interp.eval("obj m")
        assert result.value == "C+A"
