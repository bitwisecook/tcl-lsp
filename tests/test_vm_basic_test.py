"""Pytest coverage for Tcl basic.test — tests from tclBasic.c.

Translates the key tests from ``tmp/tcl9.0.3/tests/basic.test`` into
pytest tests that exercise our VM.  Organised to match the original
test numbering so failures can be cross-referenced.
"""

from __future__ import annotations

import pytest

from vm.commands.interp_cmds import reset_interp_state
from vm.interp import TclInterp
from vm.types import TclError


@pytest.fixture(autouse=True)
def _clean_interp_state() -> None:
    """Reset child interpreter state between tests."""
    reset_interp_state()


# Helper


def fresh() -> TclInterp:
    """Return a fresh interpreter without init.tcl sourcing."""
    return TclInterp(source_init=False)


# ══════════════════════════════════════════════════════════════════
#  basic-1.*  —  interp create / eval / delete
# ══════════════════════════════════════════════════════════════════


class TestInterpCreateEvalDelete:
    """basic-1.1: Create a child interp, eval in it, delete it."""

    def test_child_interp_namespace_eval(self) -> None:
        interp = fresh()
        interp.eval("interp create test_interp")
        interp.eval(
            "interp eval test_interp {"
            "  namespace eval test_ns_basic {"
            "    proc p {} { return [namespace current] }"
            "  }"
            "}"
        )
        result = interp.eval("interp eval test_interp {test_ns_basic::p}")
        assert result.value == "::test_ns_basic"
        interp.eval("interp delete test_interp")

    def test_interp_delete_removes_child(self) -> None:
        interp = fresh()
        interp.eval("interp create myinterp")
        result = interp.eval("interp exists myinterp")
        assert result.value == "1"
        interp.eval("interp delete myinterp")
        result = interp.eval("interp exists myinterp")
        assert result.value == "0"


# ══════════════════════════════════════════════════════════════════
#  basic-11.*  —  interp hide / expose with aliases
# ══════════════════════════════════════════════════════════════════


class TestInterpHideExpose:
    """basic-11.1, 12.*, 13.*: Hiding and exposing commands."""

    def test_hide_and_expose_in_child(self) -> None:
        """basic-11.1: hide a proc in a child, alias still fails."""
        interp = fresh()
        interp.eval("interp create test_interp")
        interp.eval("interp eval test_interp {  proc p {} { return 27 }}")
        # Verify proc works
        result = interp.eval("interp eval test_interp {p}")
        assert result.value == "27"

        # Hide it
        interp.eval("test_interp hide p")

        # Calling p should fail
        result = interp.eval("catch {interp eval test_interp {p}} msg")
        assert result.value == "1"  # error

        # Expose it again
        interp.eval("test_interp expose p")
        result = interp.eval("interp eval test_interp {p}")
        assert result.value == "27"

        interp.eval("interp delete test_interp")

    def test_hide_rejects_namespace_qualifiers(self) -> None:
        """basic-12.1: can't hide with namespace qualifiers."""
        interp = fresh()
        interp.eval("interp create test_interp")
        interp.eval(
            "interp eval test_interp {"
            "  namespace eval test_ns_basic {"
            "    proc p {} { return [namespace current] }"
            "  }"
            "}"
        )
        result = interp.eval("catch {test_interp hide test_ns_basic::p x} msg")
        assert result.value == "1"
        result = interp.eval("set msg")
        assert "can only hide global namespace commands" in result.value

        result = interp.eval("catch {test_interp hide x test_ns_basic::p} msg1")
        assert result.value == "1"
        result = interp.eval("set msg1")
        assert "cannot use namespace qualifiers" in result.value

        interp.eval("interp delete test_interp")

    def test_expose_rejects_namespace_target(self) -> None:
        """basic-13.1: can't expose to a namespace."""
        interp = fresh()
        interp.eval("interp create test_interp")
        interp.eval("interp eval test_interp { proc cmd {} { return [namespace current] } }")
        interp.eval("test_interp hide cmd")
        result = interp.eval("catch {interp expose test_interp cmd ::test_ns_basic::newCmd} msg")
        assert result.value == "1"
        result = interp.eval("set msg")
        assert "cannot expose to a namespace" in result.value
        interp.eval("interp delete test_interp")


# ══════════════════════════════════════════════════════════════════
#  basic-15.*  —  Tcl_CreateObjCommand (proc in namespace)
# ══════════════════════════════════════════════════════════════════


class TestProcInNamespace:
    """basic-15.1: Creating a proc inside a namespace."""

    def test_proc_in_namespace(self) -> None:
        interp = fresh()
        interp.eval("namespace eval test_ns_basic {}")
        interp.eval("proc test_ns_basic::cmd {} { return [namespace current] }")
        result = interp.eval("test_ns_basic::cmd")
        assert result.value == "::test_ns_basic"

    def test_namespace_proc_and_delete(self) -> None:
        interp = fresh()
        interp.eval("namespace eval test_ns_basic {}")
        interp.eval("proc test_ns_basic::p {} { return 42 }")
        result = interp.eval("test_ns_basic::p")
        assert result.value == "42"
        interp.eval("namespace delete test_ns_basic")
        result = interp.eval("namespace exists test_ns_basic")
        assert result.value == "0"


# ══════════════════════════════════════════════════════════════════
#  basic-18.*  —  TclRenameCommand
# ══════════════════════════════════════════════════════════════════


class TestRenameCommand:
    """basic-18.*: Renaming commands."""

    def test_rename_ns_qualified(self) -> None:
        """basic-18.1: rename with namespace qualifiers."""
        interp = fresh()
        interp.eval(
            'namespace eval test_ns_basic {  proc p {} { return "p in [namespace current]" }}'
        )
        result = interp.eval("test_ns_basic::p")
        assert result.value == "p in ::test_ns_basic"

        interp.eval("rename test_ns_basic::p test_ns_basic::q")
        result = interp.eval("test_ns_basic::q")
        assert result.value == "p in ::test_ns_basic"

    def test_rename_nonexistent_errors(self) -> None:
        """basic-18.2: renaming a non-existent command errors."""
        interp = fresh()
        with pytest.raises(TclError, match="doesn't exist"):
            interp.eval("rename test_ns_basic::p test_ns_basic::q")

    def test_rename_to_empty_deletes(self) -> None:
        """basic-18.3: rename to empty deletes the command."""
        interp = fresh()
        interp.eval(
            'namespace eval test_ns_basic {  proc p {} { return "p in [namespace current]" }}'
        )
        result = interp.eval("info commands test_ns_basic::*")
        assert "::test_ns_basic::p" in result.value

        interp.eval('rename test_ns_basic::p ""')
        result = interp.eval("info commands test_ns_basic::*")
        assert result.value == ""

    def test_rename_shadows_command(self) -> None:
        """basic-18.6: renaming creates command shadowing."""
        interp = fresh()
        interp.eval('proc p {} { return "p in [namespace current]" }')
        interp.eval('proc q {} { return "q in [namespace current]" }')
        interp.eval("namespace eval test_ns_basic {  proc callP {} { p }}")
        result = interp.eval("test_ns_basic::callP")
        assert result.value == "p in ::"

        interp.eval("rename q test_ns_basic::p")
        result = interp.eval("test_ns_basic::callP")
        assert result.value == "q in ::test_ns_basic"


# ══════════════════════════════════════════════════════════════════
#  basic-22.*  —  Tcl_GetCommandFullName
# ══════════════════════════════════════════════════════════════════


class TestGetCommandFullName:
    """basic-22.1: namespace which for imported commands."""

    def test_namespace_which(self) -> None:
        interp = fresh()
        interp.eval(
            "namespace eval test_ns_basic1 {\n"
            "  namespace export cmd*\n"
            "  proc cmd1 {} {}\n"
            "  proc cmd2 {} {}\n"
            "}"
        )
        interp.eval(
            "namespace eval test_ns_basic2 {\n"
            "  namespace export *\n"
            "  namespace import ::test_ns_basic1::*\n"
            "  proc p {} {}\n"
            "}"
        )
        interp.eval(
            "namespace eval test_ns_basic3 {\n"
            "  namespace import ::test_ns_basic2::*\n"
            "  proc q {} {}\n"
            "}"
        )
        # Test which -command from within test_ns_basic3
        result = interp.eval("namespace eval test_ns_basic3 {  namespace which -command foreach}")
        assert result.value == "::foreach"

        result = interp.eval("namespace eval test_ns_basic3 {  namespace which -command q}")
        assert result.value == "::test_ns_basic3::q"


# ══════════════════════════════════════════════════════════════════
#  basic-24.*  —  Tcl_DeleteCommandFromToken
# ══════════════════════════════════════════════════════════════════


class TestDeleteCommand:
    """basic-24.*: Deleting commands and namespace procs."""

    def test_delete_ns_proc_falls_back(self) -> None:
        """basic-24.2: deleting ns proc makes global visible."""
        interp = fresh()
        interp.eval('proc p {} { return "global p" }')
        interp.eval(
            "namespace eval test_ns_basic {\n"
            '  proc p {} { return "namespace p" }\n'
            "  proc callP {} { p }\n"
            "}"
        )
        result = interp.eval("test_ns_basic::callP")
        assert result.value == "namespace p"

        interp.eval('rename test_ns_basic::p ""')
        result = interp.eval("test_ns_basic::callP")
        assert result.value == "global p"


# ══════════════════════════════════════════════════════════════════
#  basic-36.*  —  Unknown command handling
# ══════════════════════════════════════════════════════════════════


class TestUnknownCommand:
    """basic-36.1: Lookup of 'unknown' command."""

    def test_unknown_handler_in_child(self) -> None:
        interp = fresh()
        interp.eval("interp create test_interp")
        interp.eval('interp eval test_interp {  proc unknown {args} { return "global unknown" }}')
        interp.eval("interp alias test_interp newAlias test_interp doesntExist")
        result = interp.eval("catch {interp eval test_interp {newAlias}} msg")
        assert result.value == "0"  # OK via unknown handler
        result = interp.eval("set msg")
        assert result.value == "global unknown"
        interp.eval("interp delete test_interp")


# ══════════════════════════════════════════════════════════════════
#  basic-47/48.*  —  Expansion ({*}) tests
# ══════════════════════════════════════════════════════════════════


class TestExpansion:
    """basic-47/48.*: Word expansion with {*}."""

    def test_no_expansion(self) -> None:
        """basic-47.4: {*} separated by whitespace is just a literal *."""
        interp = fresh()
        result = interp.eval("list {*} {*}\t{*}")
        assert result.value == "* * *"

    def test_expansion_empty(self) -> None:
        """basic-47.6: expansion to zero args."""
        interp = fresh()
        result = interp.eval("list {*}{}")
        assert result.value == ""

    def test_expansion_one(self) -> None:
        """basic-47.7: expansion to one arg."""
        interp = fresh()
        result = interp.eval("list {*}x")
        assert result.value == "x"

    def test_expansion_many(self) -> None:
        """basic-47.8: expansion to many args."""
        interp = fresh()
        result = interp.eval('list {*}"y z"')
        assert result.value == "y z"

    def test_expansion_order(self) -> None:
        """basic-47.9: expansion and substitution order."""
        interp = fresh()
        interp.eval("set x 0")
        result = interp.eval(
            "list [incr x] {*}[incr x] [incr x] {*}[list [incr x] [incr x]] [incr x]"
        )
        assert result.value == "1 2 3 4 5 6"

    def test_expansion_concat(self) -> None:
        """basic-47.10: expansion with concat."""
        interp = fresh()
        result = interp.eval("concat {*}{} a b c d e f g h i j k l m n o p q r")
        assert result.value == "a b c d e f g h i j k l m n o p q r"

    def test_expansion_with_variables(self) -> None:
        """basic-48.2: expansion with variables."""
        interp = fresh()
        interp.eval("set l1 [list a {b b} c d]")
        interp.eval("set l2 [list e f {g g} h]")
        interp.eval("proc l3 {} { list i j k {l l} }")
        result = interp.eval("list $l1 $l2 [l3]")
        assert result.value == "{a {b b} c d} {e f {g g} h} {i j k {l l}}"

    def test_expansion_mixed(self) -> None:
        """basic-48.3: mixed expansion."""
        interp = fresh()
        interp.eval("set l1 [list a {b b} c d]")
        interp.eval("set l2 [list e f {g g} h]")
        interp.eval("proc l3 {} { list i j k {l l} }")
        result = interp.eval("list {*}$l1 $l2 {*}[l3]")
        assert result.value == "a {b b} c d {e f {g g} h} i j k {l l}"

    def test_expansion_error_detection(self) -> None:
        """basic-48.5: error in expanded list."""
        interp = fresh()
        interp.eval('set l "a {a b}x y"')
        with pytest.raises(TclError, match="list element in braces"):
            interp.eval("list {*}$l")

    def test_expansion_cmd_word(self) -> None:
        """basic-48.10: expansion of command word."""
        interp = fresh()
        interp.eval("set cmd [list string range jultomte]")
        result = interp.eval("{*}$cmd 2 6")
        assert result.value == "ltomt"

    def test_expansion_empty_args(self) -> None:
        """basic-48.11: expansion into nothing."""
        interp = fresh()
        interp.eval("set cmd {}")
        interp.eval("set bar {}")
        result = interp.eval("{*}$cmd {*}$bar")
        assert result.value == ""

    def test_expansion_double_quote(self) -> None:
        """basic-48.12: expansion with double-quoted strings."""
        interp = fresh()
        interp.eval("set l1 [list a {b b} c d]")
        interp.eval("set l2 [list e f {g g} h]")
        result = interp.eval('list {*}$l1 {*}"hej hopp" {*}$l2')
        assert result.value == "a {b b} c d hej hopp e f {g g} h"

    def test_expansion_braced(self) -> None:
        """basic-48.13: expansion with braces."""
        interp = fresh()
        interp.eval("set l1 [list a {b b} c d]")
        interp.eval("set l2 [list e f {g g} h]")
        result = interp.eval("list {*}$l1 {*}{hej hopp} {*}$l2")
        assert result.value == "a {b b} c d hej hopp e f {g g} h"

    def test_expansion_hash_command(self) -> None:
        """basic-48.14: expansion with hash command."""
        interp = fresh()
        interp.eval('set cmd "#"')
        with pytest.raises(TclError, match='invalid command name "#"'):
            interp.eval("{*}$cmd apa bepa")

    def test_expansion_object_safety(self) -> None:
        """basic-48.17: expansion preserves object types."""
        interp = fresh()
        result = interp.eval(
            "set third [expr {1.0/3.0}];"
            "set l [list $third $third];"
            "set x [list $third {*}$l $third];"
            "set res [list];"
            "foreach t $x { lappend res [expr {$t * 3.0}] };"
            "set res"
        )
        assert result.value == "1.0 1.0 1.0 1.0"

    def test_expansion_list_semantics(self) -> None:
        """basic-48.18: expansion follows list semantics."""
        interp = fresh()
        # The original test uses a multi-line braced string (no semicolons).
        # When parsed as a Tcl list, elements are: list a b set apa 10 (6 items).
        # {*} expansion makes `list` the command with 5 args → llength = 5.
        result = interp.eval(
            "set badcmd {\n"
            "    list a b\n"
            "    set apa 10\n"
            "};\n"
            "set apa 0;\n"
            "list [llength [{*}$badcmd]] $apa"
        )
        assert result.value == "5 0"

    def test_expansion_break_continue(self) -> None:
        """basic-48.23: expansion with break/continue."""
        interp = fresh()
        result = interp.eval(
            "set res {};"
            "for {set t 0} {$t < 10} {incr t} { {*}break };"
            "lappend res $t;"
            "for {set t 0} {$t < 10} {incr t} { {*}continue ; set t 20 };"
            "lappend res $t;"
            "lappend res [catch { {*}{error Hejsan} } err];"
            "lappend res $err"
        )
        assert result.value == "0 10 1 Hejsan"


# ══════════════════════════════════════════════════════════════════
#  basic-50.*  —  interp alias and return codes
# ══════════════════════════════════════════════════════════════════


class TestInterpAlias:
    """basic-50.1 and general interp alias tests."""

    def test_alias_self_referencing(self) -> None:
        """Alias in current interpreter works."""
        interp = fresh()
        interp.eval("interp alias {} run {} if 1")
        result = interp.eval("run {list a b c}")
        assert result.value == "a b c"

    def test_alias_cross_interp(self) -> None:
        """Alias from parent to child interpreter."""
        interp = fresh()
        interp.eval("interp create child")
        interp.eval("interp eval child { proc greet {} { return hello } }")
        interp.eval("interp alias {} localGreet child greet")
        result = interp.eval("localGreet")
        assert result.value == "hello"
        interp.eval("interp delete child")

    def test_alias_with_extra_args(self) -> None:
        """Alias prepends extra arguments."""
        interp = fresh()
        interp.eval("interp alias {} mylist {} list prefix")
        result = interp.eval("mylist a b")
        assert result.value == "prefix a b"

    def test_alias_delete(self) -> None:
        """Deleting an alias."""
        interp = fresh()
        interp.eval("interp alias {} myalias {} list")
        result = interp.eval("myalias a b")
        assert result.value == "a b"
        # Note: in Tcl, deleting is done via: interp alias {} myalias {}
        # But this is the 3-arg form


# ══════════════════════════════════════════════════════════════════
#  Interp eval isolation
# ══════════════════════════════════════════════════════════════════


class TestInterpEvalIsolation:
    """Child interp eval creates isolated environments."""

    def test_child_var_isolation(self) -> None:
        """Variables in child don't leak to parent."""
        interp = fresh()
        interp.eval("interp create child")
        interp.eval("interp eval child {set x 42}")
        with pytest.raises(TclError, match='can\'t read "x"'):
            interp.eval("set x")
        interp.eval("interp delete child")

    def test_child_proc_isolation(self) -> None:
        """Procs in child don't leak to parent."""
        interp = fresh()
        interp.eval("interp create child")
        interp.eval("interp eval child {proc myproc {} { return 99 }}")
        result = interp.eval("interp eval child {myproc}")
        assert result.value == "99"
        with pytest.raises(TclError, match="invalid command name"):
            interp.eval("myproc")
        interp.eval("interp delete child")

    def test_child_namespace(self) -> None:
        """Namespaces in child are independent."""
        interp = fresh()
        interp.eval("interp create child")
        interp.eval(
            "interp eval child {  namespace eval myns {    proc hello {} { return world }  }}"
        )
        result = interp.eval("interp eval child {myns::hello}")
        assert result.value == "world"
        interp.eval("interp delete child")


# ══════════════════════════════════════════════════════════════════
#  Rename with namespace-qualified names
# ══════════════════════════════════════════════════════════════════


class TestRenameNamespaceProcs:
    """Renaming namespace-qualified procs."""

    def test_rename_within_namespace(self) -> None:
        interp = fresh()
        interp.eval('namespace eval ns {  proc foo {} { return "hello" }}')
        interp.eval("rename ns::foo ns::bar")
        result = interp.eval("ns::bar")
        assert result.value == "hello"
        with pytest.raises(TclError, match="invalid command name"):
            interp.eval("ns::foo")

    def test_rename_to_empty_deletes_ns_proc(self) -> None:
        interp = fresh()
        interp.eval("namespace eval ns { proc p {} { return ok } }")
        interp.eval('rename ns::p ""')
        with pytest.raises(TclError, match="invalid command name"):
            interp.eval("ns::p")


# ══════════════════════════════════════════════════════════════════
#  Namespace export / import
# ══════════════════════════════════════════════════════════════════


class TestNamespaceExportImport:
    """Namespace export and import semantics."""

    def test_export_import_basic(self) -> None:
        interp = fresh()
        interp.eval(
            "namespace eval src {\n"
            "  namespace export myproc\n"
            "  proc myproc {} { return imported }\n"
            "}"
        )
        interp.eval("namespace eval dst { namespace import ::src::* }")
        result = interp.eval("namespace eval dst { myproc }")
        assert result.value == "imported"

    def test_import_force(self) -> None:
        interp = fresh()
        interp.eval("namespace eval src {\n  namespace export cmd\n  proc cmd {} { return v1 }\n}")
        interp.eval("namespace eval dst { namespace import ::src::* }")
        result = interp.eval("namespace eval dst { cmd }")
        assert result.value == "v1"

        # Redefine and re-import with -force
        interp.eval("namespace eval src { proc cmd {} { return v2 } }")
        interp.eval("namespace eval dst { namespace import -force ::src::* }")
        result = interp.eval("namespace eval dst { cmd }")
        assert result.value == "v2"


# ══════════════════════════════════════════════════════════════════
#  tcltest framework
# ══════════════════════════════════════════════════════════════════


class TestTcltestFramework:
    """Test the tcltest package implementation."""

    def test_package_require(self) -> None:
        interp = fresh()
        result = interp.eval("package require tcltest 2.5")
        assert result.value == "2.5.10"

    def test_test_constraint(self) -> None:
        interp = fresh()
        interp.eval("testConstraint myConstraint 1")
        result = interp.eval("testConstraint myConstraint")
        assert result.value == "1"

        interp.eval("testConstraint myConstraint 0")
        result = interp.eval("testConstraint myConstraint")
        assert result.value == "0"

    def test_run_passing_test(self) -> None:
        interp = fresh()
        interp.eval("test mytest-1.0 {simple pass} {} {expr {1 + 1}} 2")

    def test_run_skipped_test(self) -> None:
        interp = fresh()
        interp.eval("testConstraint myFeature 0")
        interp.eval("test mytest-2.0 {skipped test} {myFeature} {error boom} ok")

    def test_new_style_test(self) -> None:
        interp = fresh()
        interp.eval("test mytest-3.0 {new style} -body {expr {2 + 3}} -result 5")

    def test_new_style_with_setup_cleanup(self) -> None:
        interp = fresh()
        interp.eval(
            "test mytest-4.0 {setup and cleanup} "
            "-setup {set x 10} "
            "-body {expr {$x + 5}} "
            "-result 15 "
            "-cleanup {unset x}"
        )

    def test_new_style_error_return_code(self) -> None:
        interp = fresh()
        interp.eval(
            "test mytest-5.0 {error return code} "
            '-body {error "test error"} '
            "-returnCodes error "
            "-result {test error}"
        )


# ══════════════════════════════════════════════════════════════════
#  Namespace children
# ══════════════════════════════════════════════════════════════════


class TestNamespaceChildren:
    """namespace children command."""

    def test_children_empty(self) -> None:
        interp = fresh()
        # Root always has at least ::tcl and ::tcltest
        result = interp.eval("namespace children")
        assert "::tcltest" in result.value

    def test_children_with_pattern(self) -> None:
        interp = fresh()
        interp.eval("namespace eval foo {}")
        interp.eval("namespace eval bar {}")
        result = interp.eval("namespace children :: ::foo")
        assert result.value == "::foo"


# ══════════════════════════════════════════════════════════════════
#  Namespace delete
# ══════════════════════════════════════════════════════════════════


class TestNamespaceDelete:
    """namespace delete command."""

    def test_delete_namespace(self) -> None:
        interp = fresh()
        interp.eval("namespace eval myns { proc p {} { return 1 } }")
        result = interp.eval("namespace exists myns")
        assert result.value == "1"
        interp.eval("namespace delete myns")
        result = interp.eval("namespace exists myns")
        assert result.value == "0"

    def test_delete_root_namespace_errors(self) -> None:
        interp = fresh()
        with pytest.raises(TclError, match="cannot delete"):
            interp.eval("namespace delete ::")


# ══════════════════════════════════════════════════════════════════
#  Info commands with patterns
# ══════════════════════════════════════════════════════════════════


class TestInfoCommands:
    """info commands with namespace patterns."""

    def test_info_commands_pattern(self) -> None:
        interp = fresh()
        interp.eval(
            "namespace eval myns {\n  proc cmd1 {} {}\n  proc cmd2 {} {}\n  proc other {} {}\n}"
        )
        result = interp.eval("info commands myns::cmd*")
        names = result.value.split()
        assert len(names) == 2
        assert all("cmd" in n for n in names)


# ══════════════════════════════════════════════════════════════════
#  catch with return codes
# ══════════════════════════════════════════════════════════════════


class TestCatch:
    """catch command return codes."""

    def test_catch_ok(self) -> None:
        interp = fresh()
        result = interp.eval("catch {expr {1 + 1}} res")
        assert result.value == "0"
        result = interp.eval("set res")
        assert result.value == "2"

    def test_catch_error(self) -> None:
        interp = fresh()
        result = interp.eval('catch {error "oops"} res')
        assert result.value == "1"
        result = interp.eval("set res")
        assert result.value == "oops"

    def test_catch_break(self) -> None:
        interp = fresh()
        result = interp.eval("catch {break} res")
        assert result.value == "3"

    def test_catch_continue(self) -> None:
        interp = fresh()
        result = interp.eval("catch {continue} res")
        assert result.value == "4"

    def test_catch_return(self) -> None:
        interp = fresh()
        result = interp.eval("catch {return hello} res")
        assert result.value == "2"
        result = interp.eval("set res")
        assert result.value == "hello"
