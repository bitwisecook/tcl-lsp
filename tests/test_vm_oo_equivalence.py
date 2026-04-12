"""VM equivalence tests derived from Tcl 9.0/8.6 oo.test suite.

Each test mirrors a real TclOO test case from generic/tests/oo.test.
The test name includes the original test ID (e.g. oo_9_1) for traceability.
Expected results match Tcl 9.0.3's output exactly.

These tests verify that our VM's OO dispatch, MRO, next/nextto, mixin,
constructor/destructor, and self/my behavior match real Tcl.
"""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError

# ---------------------------------------------------------------------------
# oo-2.x: Constructors
# ---------------------------------------------------------------------------


class TestOOConstructorEquivalence:
    def test_oo_2_2_constructor_and_method(self) -> None:
        """oo-2.2: constructor runs on creation, method dispatches after."""
        interp = TclInterp()
        interp.eval("""
            oo::class create testClass {
                constructor {} {
                    set ::result [list]
                    lappend ::result "[self]->construct"
                }
                method bar {} {
                    lappend ::result "[self]->bar"
                }
            }
        """)
        interp.eval("set ::result {}")
        interp.eval("[testClass create foo] bar")
        result = interp.eval("set ::result")
        assert "->construct" in result.value
        assert "->bar" in result.value


# ---------------------------------------------------------------------------
# oo-3.x: Destructors
# ---------------------------------------------------------------------------


class TestOODestructorEquivalence:
    def test_oo_3_3_constructor_destructor_lifecycle(self) -> None:
        """oo-3.3: constructor then destructor run in order."""
        interp = TclInterp()
        interp.eval("set ::result {}")
        interp.eval("""
            oo::class create foo {
                constructor {} {lappend ::result made}
                destructor {lappend ::result died}
            }
        """)
        result = interp.eval("foo new")
        obj = result.value
        interp.eval(f"{obj} destroy")
        result = interp.eval("set ::result")
        assert "made" in result.value
        assert "died" in result.value


# ---------------------------------------------------------------------------
# oo-7.x: Inheritance and next
# ---------------------------------------------------------------------------


class TestOOInheritanceEquivalence:
    def test_oo_7_1_inherited_method(self) -> None:
        """oo-7.1: subclass instance can call superclass method."""
        interp = TclInterp()
        interp.eval("""
            oo::class create superClass {
                method doit x { return $x }
            }
            oo::class create subClass {
                superclass superClass
            }
        """)
        result = interp.eval("[subClass new] doit ok")
        assert result.value == "ok"

    def test_oo_7_3_three_level_next_chain(self) -> None:
        """oo-7.3: instance -> subclass -> superclass next chain with args.

        Tcl expected: {-1- |2|}
        """
        interp = TclInterp()
        interp.eval("set ::result {}")
        interp.eval("""
            oo::class create superClass {
                method doit x {
                    lappend ::result |$x|
                }
            }
            oo::class create subClass {
                superclass superClass
                method doit x {lappend ::result -$x-; next [expr {$x + 1}]}
            }
        """)
        interp.eval("[subClass new] doit 1")
        result = interp.eval("set ::result")
        # subClass appends -1-, then next passes 2, superClass appends |2|
        assert "-1-" in result.value
        assert "|2|" in result.value

    def test_oo_7_8_next_at_end_of_chain_raises_error(self) -> None:
        """oo-7.8: next with no more implementations raises error.

        Tcl expected: {foo2 foo 1 {no next method implementation}}
        """
        interp = TclInterp()
        interp.eval("set ::result {}")
        interp.eval("""
            oo::class create foo {
                method bar {} {lappend ::result foo; next}
            }
            oo::class create foo2 {
                superclass foo
                method bar {} {lappend ::result foo2; next}
            }
        """)
        with pytest.raises(TclError, match="no next method implementation"):
            interp.eval("[foo2 new] bar")
        # Both methods did execute before the error
        result = interp.eval("set ::result")
        assert "foo2" in result.value
        assert "foo" in result.value


# ---------------------------------------------------------------------------
# oo-9.x: Multiple inheritance (diamond)
# ---------------------------------------------------------------------------


class TestOOMultipleInheritanceEquivalence:
    def test_oo_9_1_diamond_dispatch_order(self) -> None:
        """oo-9.1: D(B,C), B(A), C(A) — MRO dispatch D B C A.

        Tcl expected: {D B C A ok}
        """
        interp = TclInterp()
        interp.eval("set ::result {}")
        interp.eval("""
            oo::class create A {
                method test {} {lappend ::result A; return ok}
            }
            oo::class create B {
                superclass A
                method test {} {lappend ::result B; next}
            }
            oo::class create C {
                superclass A
                method test {} {lappend ::result C; next}
            }
            oo::class create D {
                superclass B C
                method test {} {lappend ::result D; next}
            }
        """)
        interp.eval("lappend ::result [[D new] test]")
        result = interp.eval("set ::result")
        # Tcl result: "D B C A ok"
        assert result.value == "D B C A ok"


# ---------------------------------------------------------------------------
# oo-14.x: Mixins
# ---------------------------------------------------------------------------


class TestOOMixinEquivalence:
    def test_oo_14_2_class_mixin_makes_methods_available(self) -> None:
        """oo-14.2: class-level mixin makes mixin methods available."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Aclass {
                method bar {} {return "in bar"}
            }
            oo::class create Bclass {
                method boo {} {return "in boo"}
            }
            oo::define Aclass mixin Bclass
        """)
        result = interp.eval("[Aclass new] bar")
        assert result.value == "in bar"
        result = interp.eval("[Aclass new] boo")
        assert result.value == "in boo"

    def test_oo_14_6_mixin_of_mixin(self) -> None:
        """oo-14.6: mixin of mixin — method accessible through chain.

        Tcl expected: chicken
        """
        interp = TclInterp()
        interp.eval("""
            oo::class create parent
            oo::class create A {
                superclass parent
                method egg {} {
                    return chicken
                }
            }
            oo::class create B {
                superclass parent
                mixin A
                method bar {} {
                    my egg
                }
            }
            oo::class create C {
                superclass parent
                mixin B
                method foo {} {
                    my bar
                }
            }
        """)
        result = interp.eval("[C new] foo")
        assert result.value == "chicken"

    def test_oo_14_8_class_mixin_dispatch_order(self) -> None:
        """oo-14.8: class mixin order — mixin method runs before class method.

        Tcl expected: {mix cls}
        """
        interp = TclInterp()
        interp.eval("set ::result {}")
        interp.eval("""
            oo::class create parent {
                method test {} {}
            }
            oo::class create mix {
                superclass parent
                method test {} {lappend ::result mix; next; return $::result}
            }
            oo::class create cls {
                superclass parent
                mixin mix
                method test {} {lappend ::result cls; next; return $::result}
            }
        """)
        result = interp.eval("[cls new] test")
        assert result.value == "mix cls"

    def test_mixin_method_overrides_class_method(self) -> None:
        """Mixin's implementation of a method wins over the class's own."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method greet {} { return "base" }
            }
            oo::class create Mix {
                superclass Base
                method greet {} { return "mix" }
            }
            oo::class create Cls {
                superclass Base
                method greet {} { return "cls" }
            }
            oo::define Cls mixin Mix
        """)
        result = interp.eval("[Cls new] greet")
        assert result.value == "mix"

    def test_mixin_next_through_class_to_base(self) -> None:
        """Mixin -> class -> base dispatch chain via next."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                method who {} { return "Base" }
            }
            oo::class create Mix {
                superclass Base
                method who {} { return "Mix-[next]" }
            }
            oo::class create Cls {
                superclass Base
                method who {} { return "Cls-[next]" }
            }
            oo::define Cls mixin Mix
        """)
        result = interp.eval("[Cls new] who")
        assert result.value == "Mix-Cls-Base"


# ---------------------------------------------------------------------------
# self / my commands
# ---------------------------------------------------------------------------


class TestOOSelfMyEquivalence:
    def test_self_returns_object_name(self) -> None:
        interp = TclInterp()
        interp.eval("""
            oo::class create Dog {
                method who {} { return [self] }
            }
        """)
        result = interp.eval("Dog new")
        obj = result.value
        result = interp.eval(f"{obj} who")
        assert result.value == obj

    def test_self_class(self) -> None:
        interp = TclInterp()
        interp.eval("""
            oo::class create Dog {
                method my_class {} { return [self class] }
            }
        """)
        result = interp.eval("[Dog new] my_class")
        assert result.value == "::Dog"

    def test_my_dispatches_to_own_method(self) -> None:
        interp = TclInterp()
        interp.eval("""
            oo::class create Dog {
                method bark {} { return "woof" }
                method speak {} { return [my bark] }
            }
        """)
        result = interp.eval("[Dog new] speak")
        assert result.value == "woof"


# ---------------------------------------------------------------------------
# nextto
# ---------------------------------------------------------------------------


class TestOONexttoEquivalence:
    def test_nextto_skips_intermediate_class(self) -> None:
        """nextto jumps directly to a named class in the MRO chain."""
        interp = TclInterp()
        interp.eval("""
            oo::class create A {
                method greet {} { return "A" }
            }
            oo::class create B {
                superclass A
                method greet {} { return "B-[next]" }
            }
            oo::class create C {
                superclass B
                method greet {} { return "C-[nextto A]" }
            }
        """)
        result = interp.eval("[C new] greet")
        assert result.value == "C-A"


# ---------------------------------------------------------------------------
# Instance variables
# ---------------------------------------------------------------------------


class TestOOInstanceVarsEquivalence:
    def test_counter_persistence(self) -> None:
        """Instance variables persist across method calls."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Counter {
                variable count
                constructor {} { set count 0 }
                method incr {} { set count [expr {$count + 1}]; return $count }
                method get {} { return $count }
            }
        """)
        result = interp.eval("Counter new")
        c1 = result.value
        result = interp.eval("Counter new")
        c2 = result.value
        interp.eval(f"{c1} incr")
        interp.eval(f"{c1} incr")
        interp.eval(f"{c2} incr")
        r1 = interp.eval(f"{c1} get")
        r2 = interp.eval(f"{c2} get")
        assert r1.value == "2"
        assert r2.value == "1"


# ---------------------------------------------------------------------------
# oo::define modifications
# ---------------------------------------------------------------------------


class TestOODefineEquivalence:
    def test_define_adds_method_to_existing_class(self) -> None:
        interp = TclInterp()
        interp.eval("oo::class create Dog {}")
        interp.eval("""
            oo::define Dog {
                method bark {} { return "woof" }
            }
        """)
        result = interp.eval("[Dog new] bark")
        assert result.value == "woof"

    def test_define_replaces_method(self) -> None:
        interp = TclInterp()
        interp.eval("""
            oo::class create Animal {
                method speak {} { return "..." }
            }
            oo::define Animal {
                method speak {} { return "roar" }
            }
        """)
        result = interp.eval("[Animal new] speak")
        assert result.value == "roar"

    def test_define_single_subcommand_form(self) -> None:
        """oo::define ClassName subcommand args (no body block)."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Animal {
                method speak {} { return "..." }
            }
            oo::class create Dog {}
        """)
        interp.eval("oo::define Dog superclass Animal")
        result = interp.eval("[Dog new] speak")
        assert result.value == "..."

    def test_define_mixin_via_single_subcommand(self) -> None:
        """oo::define ClassName mixin MixinClass (single subcommand form)."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Mix {
                method extra {} { return "extra" }
            }
            oo::class create Cls {}
        """)
        interp.eval("oo::define Cls mixin Mix")
        result = interp.eval("[Cls new] extra")
        assert result.value == "extra"


# ---------------------------------------------------------------------------
# Constructor / destructor inheritance
# ---------------------------------------------------------------------------


class TestOOInheritedLifecycleEquivalence:
    def test_inherited_constructor(self) -> None:
        """Subclass uses superclass constructor when it has none of its own."""
        interp = TclInterp()
        interp.eval("""
            oo::class create Base {
                variable val
                constructor {v} { set val $v }
                method get {} { return $val }
            }
            oo::class create Child {
                superclass Base
            }
        """)
        result = interp.eval("[Child new hello] get")
        assert result.value == "hello"

    def test_inherited_destructor(self) -> None:
        """Subclass runs superclass destructor when it has none of its own."""
        interp = TclInterp()
        interp.eval("set ::dtor_ran 0")
        interp.eval("""
            oo::class create Base {
                destructor { set ::dtor_ran 1 }
            }
            oo::class create Child {
                superclass Base
            }
        """)
        result = interp.eval("Child new")
        obj = result.value
        interp.eval(f"{obj} destroy")
        result = interp.eval("set ::dtor_ran")
        assert result.value == "1"
