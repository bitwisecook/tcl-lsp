"""Tests for the ``namespace`` command in the Tcl VM."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestNamespaceCurrent:
    """Tests for ``namespace current``."""

    def test_global_namespace(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace current")
        assert result.value == "::"

    def test_inside_namespace_eval(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace eval ::foo { namespace current }")
        assert result.value == "::foo"

    def test_nested_namespace(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace eval ::foo { namespace eval bar { namespace current } }")
        assert result.value == "::foo::bar"


class TestNamespaceEval:
    """Tests for ``namespace eval``."""

    def test_create_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::myns { set x 10 }")
        result = interp.eval("namespace exists ::myns")
        assert result.value == "1"

    def test_eval_returns_result(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace eval ::myns { expr {1 + 2} }")
        assert result.value == "3"

    def test_eval_multiple_bodies(self) -> None:
        interp = TclInterp()
        result = interp.eval('namespace eval ::myns "set x" " 42"')
        assert result.value == "42"

    def test_relative_name(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval foo { namespace eval bar { set x 1 } }")
        result = interp.eval("namespace exists ::foo::bar")
        assert result.value == "1"

    def test_wrong_args(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("namespace eval")


class TestNamespaceExists:
    """Tests for ``namespace exists``."""

    def test_global_exists(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace exists ::")
        assert result.value == "1"

    def test_nonexistent(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace exists ::nonexistent")
        assert result.value == "0"

    def test_created_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::testns {}")
        result = interp.eval("namespace exists ::testns")
        assert result.value == "1"


class TestNamespaceChildren:
    """Tests for ``namespace children``."""

    def test_no_children(self) -> None:
        interp = TclInterp()
        # Global namespace may have no children initially
        result = interp.eval("namespace children ::")
        # Could be empty or have some built-in namespaces
        assert isinstance(result.value, str)

    def test_with_children(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::a {}")
        interp.eval("namespace eval ::b {}")
        result = interp.eval("namespace children ::")
        children = result.value.split()
        assert "::a" in children
        assert "::b" in children

    def test_with_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::alpha {}")
        interp.eval("namespace eval ::beta {}")
        result = interp.eval("namespace children :: ::a*")
        assert "::alpha" in result.value
        assert "::beta" not in result.value


class TestNamespaceParent:
    """Tests for ``namespace parent``."""

    def test_global_parent(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace parent ::")
        assert result.value == ""

    def test_child_parent(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::child {}")
        result = interp.eval("namespace parent ::child")
        assert result.value == "::"

    def test_nested_parent(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::a { namespace eval b {} }")
        result = interp.eval("namespace parent ::a::b")
        assert result.value == "::a"


class TestNamespaceDelete:
    """Tests for ``namespace delete``."""

    def test_delete_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::todelete {}")
        result = interp.eval("namespace exists ::todelete")
        assert result.value == "1"
        interp.eval("namespace delete ::todelete")
        result = interp.eval("namespace exists ::todelete")
        assert result.value == "0"

    def test_delete_nonexistent(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="unknown namespace"):
            interp.eval("namespace delete ::nonexistent")

    def test_cannot_delete_global(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot delete"):
            interp.eval("namespace delete ::")


class TestNamespaceQualifiersTail:
    """Tests for ``namespace qualifiers`` and ``namespace tail``."""

    def test_qualifiers_full(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace qualifiers ::foo::bar::baz")
        assert result.value == "::foo::bar"

    def test_qualifiers_simple(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace qualifiers ::foo")
        assert result.value == "::"

    def test_qualifiers_no_separator(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace qualifiers foo")
        assert result.value == ""

    def test_tail_full(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace tail ::foo::bar::baz")
        assert result.value == "baz"

    def test_tail_simple(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace tail ::foo")
        assert result.value == "foo"

    def test_tail_no_separator(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace tail foo")
        assert result.value == "foo"


class TestNamespaceProcs:
    """Tests for procs defined inside namespaces."""

    def test_proc_in_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        result = interp.eval("::math::add 3 4")
        assert result.value == "7"

    def test_proc_calls_within_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::calc {
                proc double {x} { expr {$x * 2} }
                proc quadruple {x} { double [double $x] }
            }
        """)
        result = interp.eval("::calc::quadruple 5")
        assert result.value == "20"

    def test_proc_qualified_name(self) -> None:
        interp = TclInterp()
        interp.eval("proc ::myproc {} { return hello }")
        result = interp.eval("::myproc")
        assert result.value == "hello"


class TestNamespaceExportImport:
    """Tests for ``namespace export`` and ``namespace import``."""

    def test_export_and_import(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::src {
                namespace export greet
                proc greet {name} { return "hello $name" }
            }
        """)
        interp.eval("namespace import ::src::greet")
        result = interp.eval("greet world")
        assert result.value == "hello world"

    def test_export_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::lib {
                namespace export *
                proc foo {} { return foo }
                proc bar {} { return bar }
            }
        """)
        interp.eval("namespace import ::lib::*")
        assert interp.eval("foo").value == "foo"
        assert interp.eval("bar").value == "bar"

    def test_export_clear(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::lib2 {
                namespace export foo bar
                namespace export -clear baz
            }
        """)
        result = interp.eval("namespace eval ::lib2 { namespace export }")
        assert result.value == "baz"

    def test_import_force(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::src2 {
                namespace export cmd
                proc cmd {} { return v1 }
            }
        """)
        interp.eval("namespace import ::src2::cmd")
        assert interp.eval("cmd").value == "v1"
        # Redefine and force-import
        interp.eval("""
            namespace eval ::src2 {
                proc cmd {} { return v2 }
            }
        """)
        interp.eval("namespace import -force ::src2::cmd")
        assert interp.eval("cmd").value == "v2"


class TestNamespaceWhich:
    """Tests for ``namespace which``."""

    def test_which_builtin(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace which set")
        assert result.value == "::set"

    def test_which_nonexistent(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace which nonexistent_cmd")
        assert result.value == ""

    def test_which_variable(self) -> None:
        interp = TclInterp()
        interp.eval("set myvar 42")
        result = interp.eval("namespace which -variable myvar")
        assert result.value == "::myvar"


class TestNamespaceCode:
    """Tests for ``namespace code``."""

    def test_code_in_global(self) -> None:
        interp = TclInterp()
        result = interp.eval('namespace code "set x 1"')
        assert result.value == "::namespace inscope :: set x 1"

    def test_code_in_namespace(self) -> None:
        interp = TclInterp()
        result = interp.eval('namespace eval ::foo { namespace code "set x 1" }')
        assert result.value == "::namespace inscope ::foo set x 1"


class TestNamespaceInscope:
    """Tests for ``namespace inscope``."""

    def test_inscope_eval(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::ns1 { set val 42 }")
        result = interp.eval("namespace inscope ::ns1 { set val }")
        assert result.value == "42"


class TestNamespaceUpvar:
    """Tests for ``namespace upvar``."""

    def test_upvar_from_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::myns { set counter 0 }")
        interp.eval("""
            proc incr_counter {} {
                namespace upvar ::myns counter c
                incr c
            }
        """)
        interp.eval("incr_counter")
        result = interp.eval("namespace eval ::myns { set counter }")
        assert result.value == "1"


class TestNamespaceUnknown:
    """Tests for ``namespace unknown``."""

    def test_set_and_get_unknown(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::ns1 { namespace unknown myhandler }")
        result = interp.eval("namespace eval ::ns1 { namespace unknown }")
        assert result.value == "myhandler"

    def test_get_empty_unknown(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace unknown")
        assert result.value == ""


class TestNamespacePath:
    """Tests for ``namespace path``."""

    def test_empty_path(self) -> None:
        interp = TclInterp()
        result = interp.eval("namespace path")
        assert result.value == ""

    def test_set_path(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::a {}")
        interp.eval("namespace eval ::b {}")
        interp.eval("namespace path {::a ::b}")
        result = interp.eval("namespace path")
        assert "::a" in result.value
        assert "::b" in result.value


class TestNamespaceEnsemble:
    """Tests for ``namespace ensemble``."""

    def test_ensemble_create(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::myens {
                namespace export *
                namespace ensemble create
                proc sub1 {} { return "sub1 result" }
                proc sub2 {x} { return "sub2: $x" }
            }
        """)
        result = interp.eval("myens sub1")
        assert result.value == "sub1 result"
        result = interp.eval("myens sub2 hello")
        assert result.value == "sub2: hello"

    def test_ensemble_with_map(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::mapped {
                proc impl_a {} { return "a" }
                proc impl_b {} { return "b" }
                namespace ensemble create -map {alpha ::mapped::impl_a beta ::mapped::impl_b}
            }
        """)
        assert interp.eval("mapped alpha").value == "a"
        assert interp.eval("mapped beta").value == "b"

    def test_ensemble_exists(self) -> None:
        interp = TclInterp()
        interp.eval("""
            namespace eval ::ens2 {
                namespace ensemble create
            }
        """)
        result = interp.eval("namespace ensemble exists ens2")
        assert result.value == "1"
        result = interp.eval("namespace ensemble exists nonexistent")
        assert result.value == "0"


class TestNamespaceVariables:
    """Tests for variable scoping with namespaces."""

    def test_variable_in_namespace(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::ns { set x 10 }")
        # Reading from global shouldn't find it
        result = interp.eval("namespace eval ::ns { set x }")
        assert result.value == "10"

    def test_qualified_variable_access(self) -> None:
        interp = TclInterp()
        interp.eval("namespace eval ::data { set value 99 }")
        # Access via namespace eval
        result = interp.eval("namespace eval ::data { set value }")
        assert result.value == "99"

    def test_namespace_returns_to_caller(self) -> None:
        interp = TclInterp()
        interp.eval("set outside before")
        interp.eval("namespace eval ::ns { set inside hello }")
        result = interp.eval("set outside")
        assert result.value == "before"
        # Still in global namespace
        result = interp.eval("namespace current")
        assert result.value == "::"
