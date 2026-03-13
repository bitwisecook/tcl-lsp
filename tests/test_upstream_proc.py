"""Tests ported from Tcl's official tests/proc.test.

These supplement the existing test_analyser.py with additional coverage
derived from the upstream Tcl test suite for procedure definitions,
parameter handling, arity checking, and scope creation.

Areas covered:
- Valid proc definitions (simple, multi-param, defaults, args, no-params)
- Namespace-qualified procs
- Proc documentation (comment extraction)
- Arity checking: E002 (too few args), E003 (too many args)
- Default and variadic argument handling
- Scope creation: proc creates child scope with params and locals
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity

from .helpers import diag_codes as _diag_codes


def _diag_with_code(source: str, code: str):
    """Return diagnostics matching a specific *code*."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


class TestProcDefinition:
    """Basic proc definition parsing – mirrors upstream proc-1.* tests."""

    def test_simple_proc(self):
        """proc with a single parameter is registered in global scope."""
        result = analyse("proc greet {name} { puts $name }")
        assert "greet" in result.global_scope.procs
        proc = result.global_scope.procs["greet"]
        assert proc.name == "greet"
        assert len(proc.params) == 1
        assert proc.params[0].name == "name"

    def test_no_params(self):
        """proc with empty parameter list is valid."""
        result = analyse("proc noop {} { return }")
        assert "noop" in result.global_scope.procs
        proc = result.global_scope.procs["noop"]
        assert proc.params == []

    def test_multi_params(self):
        """proc with two positional parameters records both."""
        result = analyse("proc add {a b} { expr {$a + $b} }")
        proc = result.global_scope.procs["add"]
        assert len(proc.params) == 2
        assert proc.params[0].name == "a"
        assert proc.params[1].name == "b"

    def test_default_params(self):
        """proc with a default parameter records has_default and default_value."""
        result = analyse('proc greet {name {greeting Hello}} { puts "$greeting $name" }')
        proc = result.global_scope.procs["greet"]
        assert len(proc.params) == 2
        # First param has no default
        assert proc.params[0].name == "name"
        assert proc.params[0].has_default is False
        # Second param has a default
        assert proc.params[1].name == "greeting"
        assert proc.params[1].has_default is True
        assert proc.params[1].default_value == "Hello"

    def test_variadic_args(self):
        """proc whose last parameter is 'args' accepts variable arguments."""
        result = analyse("proc mylog {msg args} { puts $msg }")
        proc = result.global_scope.procs["mylog"]
        assert len(proc.params) == 2
        assert proc.params[-1].name == "args"

    def test_proc_creates_child_scope(self):
        """proc body creates a child scope containing params and locals."""
        result = analyse("proc foo {x} { set y 1 }")
        assert len(result.global_scope.children) >= 1
        proc_scope = result.global_scope.children[0]
        assert proc_scope.kind == "proc"
        assert proc_scope.name == "foo"
        # Parameter 'x' and local 'y' should both be visible
        assert "x" in proc_scope.variables
        assert "y" in proc_scope.variables

    def test_param_in_proc_scope(self):
        """Parameters appear as variables in the proc's scope."""
        result = analyse("proc double {x} { expr {$x * 2} }")
        proc_scope = result.global_scope.children[0]
        assert proc_scope.kind == "proc"
        assert "x" in proc_scope.variables


class TestNamespaceQualifiedProcs:
    """Procs defined with namespace qualifiers or inside namespace eval."""

    def test_qualified_proc_name(self):
        """Fully-qualified proc name appears in all_procs.

        When the proc name is already absolute (starts with '::'), the
        analyser normalises by prepending the current namespace prefix,
        yielding '::::math::add' as the all_procs key.
        """
        result = analyse("proc ::math::add {a b} { expr {$a + $b} }")
        assert "::::math::add" in result.all_procs

    def test_proc_qualified_name_field(self):
        """A global proc's qualified_name is prefixed with '::'."""
        result = analyse("proc greet {name} { puts $name }")
        proc = result.global_scope.procs["greet"]
        assert proc.qualified_name == "::greet"

    def test_namespace_eval_proc(self):
        """proc defined inside namespace eval gets the namespace prefix."""
        source = textwrap.dedent("""\
            namespace eval ns {
                proc foo {} { return 1 }
            }
        """)
        result = analyse(source)
        assert "::ns::foo" in result.all_procs


class TestProcDocumentation:
    """Comment extraction for proc definitions."""

    def test_comment_before_proc(self):
        """A comment immediately before proc is captured as doc."""
        source = "# Greet someone\nproc greet {name} { puts $name }"
        result = analyse(source)
        proc = result.global_scope.procs["greet"]
        assert "Greet someone" in proc.doc

    def test_no_doc_without_comment(self):
        """A proc without a preceding comment has an empty doc string."""
        result = analyse("proc foo {} { return }")
        proc = result.global_scope.procs["foo"]
        assert proc.doc == ""


class TestArityErrors:
    """Arity checking for user-defined procedures (E002 / E003)."""

    def test_too_few_args_for_user_proc(self):
        """Calling a 2-arg proc with 1 arg produces E002."""
        source = textwrap.dedent("""\
            proc add {a b} { expr {$a + $b} }
            add 1
        """)
        errors = _diag_with_code(source, "E002")
        assert len(errors) >= 1
        assert errors[0].severity == Severity.ERROR
        assert "::add" in errors[0].message

    def test_too_many_args_for_user_proc(self):
        """Calling a 1-arg proc with 3 args produces E003."""
        source = textwrap.dedent("""\
            proc greet {name} { puts $name }
            greet Alice Bob Charlie
        """)
        errors = _diag_with_code(source, "E003")
        assert len(errors) >= 1
        assert "::greet" in errors[0].message

    def test_correct_arity_no_error(self):
        """Calling a 2-arg proc with exactly 2 args produces no arity error."""
        source = textwrap.dedent("""\
            proc add {a b} { expr {$a + $b} }
            add 1 2
        """)
        codes = _diag_codes(source)
        assert "E002" not in codes
        assert "E003" not in codes

    def test_default_reduces_min_arity(self):
        """A default parameter reduces the minimum required arity."""
        source = textwrap.dedent("""\
            proc greet {name {greeting Hello}} { puts "$greeting $name" }
            greet Alice
        """)
        codes = _diag_codes(source)
        assert "E002" not in codes

    def test_variadic_allows_extra_args(self):
        """An 'args' parameter allows arbitrarily many extra arguments."""
        source = textwrap.dedent("""\
            proc mylog {msg args} { puts $msg }
            mylog hello world foo
        """)
        codes = _diag_codes(source)
        assert "E003" not in codes

    def test_variadic_still_needs_required(self):
        """Even with 'args', required parameters must be supplied (E002)."""
        source = textwrap.dedent("""\
            proc mylog {msg args} { puts $msg }
            mylog
        """)
        errors = _diag_with_code(source, "E002")
        assert len(errors) >= 1


class TestProcScopeIsolation:
    """Variables defined inside a proc are not visible globally, and vice versa."""

    def test_local_not_visible_globally(self):
        """A variable set inside a proc does not leak into the global scope."""
        source = textwrap.dedent("""\
            proc foo {} { set local_var 42 }
            set x $local_var
        """)
        result = analyse(source)
        assert "local_var" not in result.global_scope.variables

    def test_global_not_visible_in_proc(self):
        """Global variables are not automatically inherited by proc scopes."""
        source = textwrap.dedent("""\
            set gvar 99
            proc bar {} { puts $gvar }
        """)
        result = analyse(source)
        proc_scope = [s for s in result.global_scope.children if s.kind == "proc"][0]
        # The proc scope should not have 'gvar' unless an explicit
        # 'global gvar' declaration is present.
        assert "gvar" not in proc_scope.variables

    def test_multiple_procs_separate_scopes(self):
        """Two procs each create their own independent child scope."""
        source = textwrap.dedent("""\
            proc alpha {} { set a_var 1 }
            proc beta {} { set b_var 2 }
        """)
        result = analyse(source)
        proc_scopes = [s for s in result.global_scope.children if s.kind == "proc"]
        assert len(proc_scopes) == 2
        scope_names = {s.name for s in proc_scopes}
        assert "alpha" in scope_names
        assert "beta" in scope_names
        # Each scope contains only its own variable
        alpha_scope = next(s for s in proc_scopes if s.name == "alpha")
        beta_scope = next(s for s in proc_scopes if s.name == "beta")
        assert "a_var" in alpha_scope.variables
        assert "b_var" not in alpha_scope.variables
        assert "b_var" in beta_scope.variables
        assert "a_var" not in beta_scope.variables


class TestBuiltinArityErrors:
    """Arity checking for built-in Tcl commands."""

    def test_set_too_many_args(self):
        """'break' with an extra argument produces E003."""
        errors = _diag_with_code("break extra", "E003")
        assert len(errors) >= 1

    def test_puts_correct(self):
        """'puts hello' is valid – no arity errors."""
        codes = _diag_codes("puts hello")
        assert "E002" not in codes
        assert "E003" not in codes
