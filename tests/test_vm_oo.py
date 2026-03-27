"""Tests for the TclOO VM runtime: class creation, method dispatch, MRO."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestOOClassCreate:
    """Tests for oo::class create."""

    def test_create_class(self) -> None:
        interp = TclInterp()
        result = interp.eval('oo::class create Dog {}')
        assert result.value == "::Dog"

    def test_create_class_with_method(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                method bark {} { return "woof" }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} bark")
        assert result.value == "woof"

    def test_create_class_with_constructor(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                variable name
                constructor {n} { set name $n }
                method get_name {} { return $name }
            }
        ''')
        result = interp.eval("Dog new Rex")
        obj_name = result.value
        result = interp.eval(f"{obj_name} get_name")
        assert result.value == "Rex"


class TestOOMethodDispatch:
    """Tests for method invocation."""

    def test_method_with_params(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Calc {
                method add {a b} { expr {$a + $b} }
            }
        ''')
        result = interp.eval("Calc new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} add 3 4")
        assert result.value == "7"

    def test_method_wrong_args(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Calc {
                method add {a b} { expr {$a + $b} }
            }
        ''')
        result = interp.eval("Calc new")
        obj_name = result.value
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval(f"{obj_name} add 3")

    def test_unknown_method(self) -> None:
        interp = TclInterp()
        interp.eval('oo::class create Dog {}')
        result = interp.eval("Dog new")
        obj_name = result.value
        with pytest.raises(TclError, match="unknown method"):
            interp.eval(f"{obj_name} nonexistent")


class TestOOInheritance:
    """Tests for class inheritance and MRO."""

    def test_single_inheritance(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Animal {
                method speak {} { return "..." }
            }
            oo::class create Dog {
                superclass Animal
                method bark {} { return "woof" }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        # Inherited method
        result = interp.eval(f"{obj_name} speak")
        assert result.value == "..."
        # Own method
        result = interp.eval(f"{obj_name} bark")
        assert result.value == "woof"

    def test_method_override(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Animal {
                method speak {} { return "..." }
            }
            oo::class create Dog {
                superclass Animal
                method speak {} { return "woof" }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} speak")
        assert result.value == "woof"


class TestOOSelfAndMy:
    """Tests for self and my commands."""

    def test_self_returns_object_name(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                method who {} { return [self] }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} who")
        assert result.value == obj_name

    def test_my_dispatches_method(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                method bark {} { return "woof" }
                method speak {} { return [my bark] }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} speak")
        assert result.value == "woof"

    def test_self_class(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                method my_class {} { return [self class] }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} my_class")
        assert result.value == "::Dog"


class TestOODestroy:
    """Tests for object destruction."""

    def test_destroy(self) -> None:
        interp = TclInterp()
        interp.eval('oo::class create Dog {}')
        result = interp.eval("Dog new")
        obj_name = result.value
        interp.eval(f"{obj_name} destroy")
        with pytest.raises(TclError):
            interp.eval(f"{obj_name} destroy")

    def test_destructor_runs(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                variable destroyed
                constructor {} { set destroyed 0 }
                destructor { set ::was_destroyed 1 }
            }
        ''')
        interp.eval("set ::was_destroyed 0")
        result = interp.eval("Dog new")
        obj_name = result.value
        interp.eval(f"{obj_name} destroy")
        result = interp.eval("set ::was_destroyed")
        assert result.value == "1"


class TestOODefine:
    """Tests for oo::define."""

    def test_define_adds_method(self) -> None:
        interp = TclInterp()
        interp.eval('oo::class create Dog {}')
        interp.eval('''
            oo::define Dog {
                method bark {} { return "woof" }
            }
        ''')
        result = interp.eval("Dog new")
        obj_name = result.value
        result = interp.eval(f"{obj_name} bark")
        assert result.value == "woof"


class TestOOInstanceVars:
    """Tests for instance variable storage."""

    def test_instance_vars_persist(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Counter {
                variable count
                constructor {} { set count 0 }
                method incr {} { set count [expr {$count + 1}]; return $count }
                method get {} { return $count }
            }
        ''')
        result = interp.eval("Counter new")
        obj_name = result.value
        interp.eval(f"{obj_name} incr")
        interp.eval(f"{obj_name} incr")
        result = interp.eval(f"{obj_name} get")
        assert result.value == "2"

    def test_separate_instances(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Counter {
                variable count
                constructor {} { set count 0 }
                method incr {} { set count [expr {$count + 1}]; return $count }
                method get {} { return $count }
            }
        ''')
        r1 = interp.eval("Counter new")
        r2 = interp.eval("Counter new")
        obj1 = r1.value
        obj2 = r2.value
        interp.eval(f"{obj1} incr")
        interp.eval(f"{obj1} incr")
        interp.eval(f"{obj2} incr")
        result1 = interp.eval(f"{obj1} get")
        result2 = interp.eval(f"{obj2} get")
        assert result1.value == "2"
        assert result2.value == "1"


class TestOONamedCreate:
    """Tests for named object creation."""

    def test_create_named_object(self) -> None:
        interp = TclInterp()
        interp.eval('''
            oo::class create Dog {
                method bark {} { return "woof" }
            }
        ''')
        result = interp.eval("Dog create rex")
        assert result.value == "rex"
        result = interp.eval("rex bark")
        assert result.value == "woof"
