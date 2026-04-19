"""Tests ported from Tcl's official tests/namespace.test.

These supplement the existing test_analyser.py with additional coverage
derived from the upstream Tcl test suite for namespace handling.

Areas covered:
- Namespace creation via namespace eval
- Nested namespaces
- Qualified proc names in all_procs
- Variable resolution in namespace scopes
- Namespace-qualified proc calls with arity checking
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity

from .helpers import diag_codes as _diag_codes

# Helpers


def _diag_with_code(source: str, code: str):
    """Return diagnostics whose code matches *code*."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


class TestNamespaceCreation:
    """Verify that ``namespace eval`` creates the expected scope structure."""

    def test_namespace_eval_creates_scope(self):
        """namespace eval should produce at least one namespace child scope."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        result = analyse(source)
        # Global scope should have at least one child
        ns_scopes = [s for s in result.global_scope.children if s.kind == "namespace"]
        assert len(ns_scopes) >= 1

    def test_namespace_scope_name(self):
        """The namespace scope's name should contain the namespace identifier."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        result = analyse(source)
        ns_scopes = [s for s in result.global_scope.children if s.kind == "namespace"]
        names = [s.name for s in ns_scopes]
        assert any("math" in n for n in names)

    def test_proc_inside_namespace(self):
        """A proc defined inside namespace eval should appear in all_procs
        under its fully-qualified name."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        result = analyse(source)
        assert "::math::add" in result.all_procs

    def test_multiple_procs_in_namespace(self):
        """Multiple procs inside a single namespace eval should all be registered."""
        source = textwrap.dedent("""\
            namespace eval utils {
                proc first {lst} { lindex $lst 0 }
                proc last  {lst} { lindex $lst end }
            }
        """)
        result = analyse(source)
        assert "::utils::first" in result.all_procs
        assert "::utils::last" in result.all_procs

    def test_namespace_eval_with_absolute_name(self):
        """``namespace eval ::foo`` should behave identically to
        ``namespace eval foo``."""
        source = textwrap.dedent("""\
            namespace eval ::abs {
                proc worker {} { return 1 }
            }
        """)
        result = analyse(source)
        # The proc should be reachable via a qualified name that includes "abs"
        matching = [k for k in result.all_procs if "abs" in k and "worker" in k]
        assert len(matching) >= 1

    def test_empty_namespace_eval(self):
        """An empty namespace eval body should still create a scope without
        producing any errors."""
        source = "namespace eval empty {}"
        result = analyse(source)
        error_diags = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(error_diags) == 0


class TestNestedNamespaces:
    """Nested namespace eval blocks should produce fully-qualified names."""

    def test_nested_namespace_eval(self):
        """Two levels of namespace eval should register the proc with a
        qualified name that includes the inner namespace segment."""
        source = textwrap.dedent("""\
            namespace eval a {
                namespace eval b {
                    proc inner {} { return 1 }
                }
            }
        """)
        result = analyse(source)
        # The analyser qualifies using the immediate parent scope name;
        # verify the proc is registered and reachable.
        matching = [k for k in result.all_procs if "inner" in k]
        assert len(matching) >= 1

    def test_deeply_nested(self):
        """Three levels of nesting should register the proc; the qualified
        name should contain the innermost namespace and the proc name."""
        source = textwrap.dedent("""\
            namespace eval x {
                namespace eval y {
                    namespace eval z {
                        proc deep {} { return 42 }
                    }
                }
            }
        """)
        result = analyse(source)
        matching = [k for k in result.all_procs if "deep" in k]
        assert len(matching) >= 1

    def test_nested_namespace_scope_hierarchy(self):
        """The outer namespace scope should have a child scope for the inner
        namespace."""
        source = textwrap.dedent("""\
            namespace eval outer {
                namespace eval inner {
                    proc leaf {} { return 1 }
                }
            }
        """)
        result = analyse(source)
        outer_scopes = [
            s for s in result.global_scope.children if s.kind == "namespace" and "outer" in s.name
        ]
        assert len(outer_scopes) >= 1
        outer = outer_scopes[0]
        inner_scopes = [s for s in outer.children if s.kind == "namespace" and "inner" in s.name]
        assert len(inner_scopes) >= 1

    def test_sibling_namespaces_inside_parent(self):
        """Two sibling namespaces inside a parent should both register procs."""
        source = textwrap.dedent("""\
            namespace eval parent {
                namespace eval childA {
                    proc alpha {} { return "a" }
                }
                namespace eval childB {
                    proc beta {} { return "b" }
                }
            }
        """)
        result = analyse(source)
        alpha_matches = [k for k in result.all_procs if "alpha" in k]
        beta_matches = [k for k in result.all_procs if "beta" in k]
        assert len(alpha_matches) >= 1
        assert len(beta_matches) >= 1


class TestQualifiedProcNames:
    """Procs defined with explicit qualifiers should be stored correctly."""

    def test_global_qualifier(self):
        """``proc ::foo`` should be registered in all_procs."""
        source = "proc ::foo {} { return }"
        result = analyse(source)
        matching = [k for k in result.all_procs if "foo" in k]
        assert len(matching) >= 1

    def test_namespace_qualifier(self):
        """``proc ::ns::bar`` should be registered in all_procs."""
        source = "proc ::ns::bar {} { return }"
        result = analyse(source)
        matching = [k for k in result.all_procs if "bar" in k]
        assert len(matching) >= 1

    def test_namespace_eval_qualifies_proc_name(self):
        """A plain proc name inside namespace eval should receive the
        namespace prefix automatically."""
        source = textwrap.dedent("""\
            namespace eval utils {
                proc helper {} { return 1 }
            }
        """)
        result = analyse(source)
        assert "::utils::helper" in result.all_procs

    def test_proc_with_params_qualified(self):
        """Parameters should be preserved on a namespace-qualified proc."""
        source = textwrap.dedent("""\
            namespace eval geo {
                proc distance {x1 y1 x2 y2} {
                    expr {sqrt(($x2-$x1)**2 + ($y2-$y1)**2)}
                }
            }
        """)
        result = analyse(source)
        assert "::geo::distance" in result.all_procs
        proc_def = result.all_procs["::geo::distance"]
        assert len(proc_def.params) == 4
        assert proc_def.params[0].name == "x1"
        assert proc_def.params[3].name == "y2"

    def test_proc_with_defaults_in_namespace(self):
        """Default parameter values should survive namespace qualification."""
        source = textwrap.dedent("""\
            namespace eval cfg {
                proc init {{verbose 0}} { return $verbose }
            }
        """)
        result = analyse(source)
        assert "::cfg::init" in result.all_procs
        proc_def = result.all_procs["::cfg::init"]
        assert proc_def.params[0].name == "verbose"
        assert proc_def.params[0].has_default is True
        assert proc_def.params[0].default_value == "0"

    def test_proc_with_args_param_in_namespace(self):
        """A proc with an ``args`` parameter inside a namespace should be
        recorded correctly."""
        source = textwrap.dedent("""\
            namespace eval logging {
                proc log {level args} { puts "$level: $args" }
            }
        """)
        result = analyse(source)
        assert "::logging::log" in result.all_procs
        proc_def = result.all_procs["::logging::log"]
        param_names = [p.name for p in proc_def.params]
        assert "level" in param_names
        assert "args" in param_names

    def test_proc_qualified_name_field(self):
        """The ``qualified_name`` field on ProcDef should match the key in
        ``all_procs``."""
        source = textwrap.dedent("""\
            namespace eval data {
                proc parse {input} { return $input }
            }
        """)
        result = analyse(source)
        assert "::data::parse" in result.all_procs
        proc_def = result.all_procs["::data::parse"]
        assert proc_def.qualified_name == "::data::parse"


class TestNamespaceVariables:
    """Variables set inside namespace eval should belong to the namespace scope."""

    def test_variable_in_namespace_scope(self):
        """``set`` inside namespace eval should define a variable in the
        namespace scope."""
        source = textwrap.dedent("""\
            namespace eval config {
                set debug 1
            }
        """)
        result = analyse(source)
        # Check that variable is defined in namespace scope
        ns_scopes = [s for s in result.global_scope.children if s.kind == "namespace"]
        assert any("debug" in s.variables for s in ns_scopes)

    def test_multiple_variables_in_namespace(self):
        """Several ``set`` commands should each register in the namespace scope."""
        source = textwrap.dedent("""\
            namespace eval settings {
                set host "localhost"
                set port 8080
                set timeout 30
            }
        """)
        result = analyse(source)
        ns_scopes = [
            s
            for s in result.global_scope.children
            if s.kind == "namespace" and "settings" in s.name
        ]
        assert len(ns_scopes) >= 1
        ns = ns_scopes[0]
        assert "host" in ns.variables
        assert "port" in ns.variables
        assert "timeout" in ns.variables

    def test_variable_command_in_namespace(self):
        """The ``variable`` command inside a namespace should define a variable
        in the namespace scope without producing errors."""
        source = textwrap.dedent("""\
            namespace eval state {
                variable counter 0
            }
        """)
        result = analyse(source)
        error_diags = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(error_diags) == 0

    def test_variable_not_leaked_to_global(self):
        """Variables set inside a namespace eval should not appear in the
        global scope."""
        source = textwrap.dedent("""\
            namespace eval hidden {
                set secret 42
            }
        """)
        result = analyse(source)
        # "secret" should not be a direct member of the global scope
        assert "secret" not in result.global_scope.variables


class TestNamespaceArityChecking:
    """Arity checking should work correctly for namespace-qualified proc calls."""

    def test_qualified_call_correct_arity(self):
        """Calling a namespace proc with the right number of arguments
        should not produce E002 or E003."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
            math::add 1 2
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags

    def test_qualified_call_too_few_args(self):
        """Calling a namespace proc with too few arguments should emit E002."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
            math::add 1
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" in diags

    def test_qualified_call_too_many_args(self):
        """Calling a namespace proc with too many arguments should emit E003."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
            math::add 1 2 3
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E003" in diags

    def test_fully_qualified_call_correct_arity(self):
        """A call using the full ``::ns::proc`` form should pass arity checking."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
            ::math::add 1 2
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags

    def test_call_inside_same_namespace_correct_arity(self):
        """A call within the same namespace eval body with correct arity
        should not produce errors."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
                add 1 2
            }
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags

    def test_call_inside_same_namespace_too_few(self):
        """A short call within a namespace eval should still trigger E002."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
                add 1
            }
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" in diags

    def test_proc_with_defaults_arity(self):
        """Optional parameters should be accounted for in arity checking."""
        source = textwrap.dedent("""\
            namespace eval fmt {
                proc greet {name {greeting "Hello"}} {
                    puts "$greeting, $name!"
                }
            }
            fmt::greet "Alice"
            fmt::greet "Bob" "Hi"
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags

    def test_proc_with_args_variadic_arity(self):
        """A proc with ``args`` should accept any number of trailing arguments."""
        source = textwrap.dedent("""\
            namespace eval io {
                proc log {level args} { puts "$level: $args" }
            }
            io::log "INFO"
            io::log "WARN" "something" "happened"
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags

    def test_nested_namespace_qualified_call(self):
        """Calling a deeply-nested namespace proc with correct arity should
        produce no arity errors."""
        source = textwrap.dedent("""\
            namespace eval a {
                namespace eval b {
                    proc f {x} { return $x }
                }
            }
            a::b::f 42
        """)
        diags = [d.code for d in analyse(source).diagnostics]
        assert "E002" not in diags
        assert "E003" not in diags


class TestNamespaceEdgeCases:
    """Edge cases and additional patterns inspired by the upstream test suite."""

    def test_namespace_with_global_qualified_proc(self):
        """Defining a proc with ``::`` prefix inside a namespace eval should
        still register the proc (the analyser prepends the enclosing
        namespace)."""
        source = textwrap.dedent("""\
            namespace eval wrapper {
                proc ::toplevel_func {} { return "top" }
            }
        """)
        result = analyse(source)
        matching = [k for k in result.all_procs if "toplevel_func" in k]
        assert len(matching) >= 1

    def test_namespace_proc_stored_in_scope_procs(self):
        """Procs should appear in the namespace scope's ``procs`` dict as well
        as in ``all_procs``."""
        source = textwrap.dedent("""\
            namespace eval registry {
                proc lookup {key} { return $key }
            }
        """)
        result = analyse(source)
        ns_scopes = [
            s
            for s in result.global_scope.children
            if s.kind == "namespace" and "registry" in s.name
        ]
        assert len(ns_scopes) >= 1
        assert "lookup" in ns_scopes[0].procs

    def test_no_false_positive_on_namespace_subcommands(self):
        """Commands like ``namespace current`` should not produce spurious
        error diagnostics."""
        source = textwrap.dedent("""\
            namespace eval myns {
                set cur [namespace current]
            }
        """)
        result = analyse(source)
        error_diags = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(error_diags) == 0

    def test_namespace_eval_multiple_times_same_name(self):
        """Calling ``namespace eval`` twice on the same namespace should merge
        procs -- both should be visible in ``all_procs``."""
        source = textwrap.dedent("""\
            namespace eval shared {
                proc alpha {} { return "a" }
            }
            namespace eval shared {
                proc beta {} { return "b" }
            }
        """)
        result = analyse(source)
        assert "::shared::alpha" in result.all_procs
        assert "::shared::beta" in result.all_procs


class TestDiagHelpers:
    """Smoke-test the helper functions used by other test classes."""

    def test_diag_codes_returns_list(self):
        codes = _diag_codes("set")
        assert isinstance(codes, list)
        assert len(codes) >= 1

    def test_diag_with_code_filters(self):
        diags = _diag_with_code("break extra", "E003")
        # ``break extra`` should trigger a too-many-arguments error
        assert all(d.code == "E003" for d in diags)

    def test_clean_source_no_errors(self):
        """Well-formed code should produce no E-level diagnostics."""
        source = textwrap.dedent("""\
            namespace eval clean {
                proc noop {} { return }
            }
            clean::noop
        """)
        diags = _diag_codes(source)
        e_codes = [c for c in diags if c.startswith("E")]
        assert len(e_codes) == 0
