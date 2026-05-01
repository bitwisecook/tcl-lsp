"""Tests for the Tcl semantic analyser."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis import analyse
from core.analysis.semantic_model import Severity


class TestProcAnalysis:
    def test_simple_proc(self):
        result = analyse("proc greet {name} { puts $name }")
        assert "greet" in result.global_scope.procs
        proc = result.global_scope.procs["greet"]
        assert proc.name == "greet"
        assert proc.qualified_name == "::greet"
        assert len(proc.params) == 1
        assert proc.params[0].name == "name"

    def test_proc_multiple_params(self):
        result = analyse("proc add {a b} { return [+ $a $b] }")
        proc = result.global_scope.procs["add"]
        assert len(proc.params) == 2
        assert proc.params[0].name == "a"
        assert proc.params[1].name == "b"

    def test_proc_default_param(self):
        result = analyse("proc greet {{name World}} { puts $name }")
        proc = result.global_scope.procs["greet"]
        assert proc.params[0].name == "name"
        assert proc.params[0].has_default is True
        assert proc.params[0].default_value == "World"

    def test_proc_no_params(self):
        result = analyse("proc noop {} { }")
        proc = result.global_scope.procs["noop"]
        assert len(proc.params) == 0

    def test_proc_in_all_procs(self):
        result = analyse("proc foo {} {}")
        assert "::foo" in result.all_procs

    def test_proc_doc_from_comment(self):
        source = textwrap.dedent("""\
            # Adds two numbers
            proc add {a b} { return [+ $a $b] }
        """)
        result = analyse(source)
        proc = result.global_scope.procs["add"]
        assert "Adds two numbers" in proc.doc

    def test_multiple_procs(self):
        source = textwrap.dedent("""\
            proc foo {} {}
            proc bar {x} { return $x }
        """)
        result = analyse(source)
        assert "foo" in result.global_scope.procs
        assert "bar" in result.global_scope.procs

    def test_proc_creates_child_scope(self):
        result = analyse("proc foo {x} { set y 1 }")
        assert len(result.global_scope.children) == 1
        proc_scope = result.global_scope.children[0]
        assert proc_scope.kind == "proc"
        assert proc_scope.name == "foo"
        # 'x' is a parameter, 'y' is set inside
        assert "x" in proc_scope.variables
        assert "y" in proc_scope.variables


class TestVariableAnalysis:
    def test_set_defines_var(self):
        result = analyse("set x 42")
        assert "x" in result.global_scope.variables

    def test_multiple_sets(self):
        source = "set x 1\nset y 2\nset z 3"
        result = analyse(source)
        assert "x" in result.global_scope.variables
        assert "y" in result.global_scope.variables
        assert "z" in result.global_scope.variables

    def test_incr_defines_var(self):
        result = analyse("incr counter")
        assert "counter" in result.global_scope.variables

    def test_array_var(self):
        result = analyse("set arr(key) value")
        # Should define 'arr' (base name)
        assert "arr" in result.global_scope.variables

    def test_var_in_proc_scope(self):
        result = analyse("proc foo {} { set local 1 }")
        # 'local' should not be in global scope
        assert "local" not in result.global_scope.variables
        proc_scope = result.global_scope.children[0]
        assert "local" in proc_scope.variables

    def test_set_one_arg_is_read_only(self):
        result = analyse("set x")
        assert "x" not in result.global_scope.variables

    def test_variable_command_skips_value_words(self):
        result = analyse("variable port 8080")
        assert "port" in result.global_scope.variables
        assert "8080" not in result.global_scope.variables


class TestNamespaceAnalysis:
    def test_namespace_eval(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return 1 }
            }
        """)
        result = analyse(source)
        assert len(result.global_scope.children) == 1
        ns = result.global_scope.children[0]
        assert ns.kind == "namespace"
        assert ns.name == "myns"
        assert "helper" in ns.procs

    def test_namespace_qualified_proc(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { return [+ $a $b] }
            }
        """)
        result = analyse(source)
        assert "::math::add" in result.all_procs


# Arity checking / diagnostics
class TestDiagnostics:
    def test_too_few_args_set(self):
        result = analyse("set")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "Too few" in errors[0].message

    def test_too_many_args_break(self):
        result = analyse("break extra")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "Too many" in errors[0].message

    def test_correct_arity_no_error(self):
        result = analyse("set x 42")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) == 0

    def test_puts_too_many_args(self):
        # puts -nonewline channel string is valid (3 raw, 2 positional)
        result = analyse("puts -nonewline stderr hello")
        errors = [d for d in result.diagnostics if d.code == "E003"]
        assert len(errors) == 0
        # puts a b c is invalid (3 positional args, max is 2)
        result = analyse("puts a b c")
        errors = [d for d in result.diagnostics if d.code == "E003"]
        assert len(errors) >= 1

    def test_while_too_few_args(self):
        result = analyse("while {1}")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1

    def test_unknown_subcommand_warning(self):
        result = analyse("string bogus hello")
        warnings = [d for d in result.diagnostics if d.severity == Severity.WARNING]
        assert len(warnings) >= 1
        assert "Unknown subcommand" in warnings[0].message

    def test_package_prefer_not_unknown(self):
        """Regression test for #109: package prefer should not produce W001."""
        result = analyse("package prefer")
        warnings = [d for d in result.diagnostics if d.severity == Severity.WARNING]
        assert not any("Unknown subcommand" in w.message for w in warnings)

    def test_package_files_not_unknown(self):
        """Regression test for #109: package files should not produce W001."""
        result = analyse("package files mypackage")
        warnings = [d for d in result.diagnostics if d.severity == Severity.WARNING]
        assert not any("Unknown subcommand" in w.message for w in warnings)

    def test_missing_subcommand(self):
        result = analyse("namespace")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "subcommand" in errors[0].message.lower()

    def test_proc_arity_not_checked_for_unknown(self):
        # Unknown commands should not produce arity errors
        result = analyse("mycommand a b c d e")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) == 0

    def test_error_has_range(self):
        result = analyse("set")
        assert len(result.diagnostics) >= 1
        d = result.diagnostics[0]
        assert d.range.start.line == 0
        assert d.range.start.offset == 0

    def test_for_too_few_args(self):
        result = analyse("for {set i 0} {$i < 10}")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1

    def test_proc_call_too_few_args(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            add 1
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002"]
        assert len(errors) >= 1
        assert "::add" in errors[0].message

    def test_proc_call_too_many_args(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            add 1 2 3
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003"]
        assert len(errors) >= 1
        assert "::add" in errors[0].message

    def test_proc_call_with_default_arg(self):
        source = textwrap.dedent("""\
            proc greet {name {title Mr}} { return "$title $name" }
            greet Bob
        """)
        result = analyse(source)
        proc_arity = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::greet" in d.message
        ]
        assert len(proc_arity) == 0

    def test_proc_call_variadic_args(self):
        source = textwrap.dedent("""\
            proc log {level args} { return $level }
            log info a b c
        """)
        result = analyse(source)
        proc_arity = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::log" in d.message
        ]
        assert len(proc_arity) == 0

    def test_namespace_proc_call_arity(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { return [expr {$a + $b}] }
                add 1
            }
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "::math::add" in d.message]
        assert len(errors) >= 1

    def test_proc_call_arg_expansion_suppresses_too_few(self):
        """Regression test for issue #129.

        ``{*}$var`` may expand to any number of arguments at runtime, so the
        static arity check must not treat it as a single argument when the
        required minimum is higher.
        """
        source = textwrap.dedent("""\
            proc rgbToLab {r g b} { return 1 }
            set rgb1 {255 255 255}
            rgbToLab {*}$rgb1
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "::rgbToLab" in d.message]
        assert errors == [], f"unexpected E002 diagnostics: {errors}"

    def test_proc_call_arg_expansion_only(self):
        """A call whose only argument is ``{*}$x`` should not fire E002/E003."""
        source = textwrap.dedent("""\
            proc myproc {x y z} { return 1 }
            set args {1 2 3}
            myproc {*}$args
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::myproc" in d.message
        ]
        assert errors == []

    def test_proc_call_arg_expansion_mixed_literal_and_expansion(self):
        """``foo a {*}$x`` where foo takes (a, b, c) is valid at runtime."""
        source = textwrap.dedent("""\
            proc myproc {a b c} { return 1 }
            set rest {2 3}
            myproc 1 {*}$rest
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::myproc" in d.message
        ]
        assert errors == []

    def test_proc_call_arg_expansion_still_detects_too_many(self):
        """Non-expansion args alone exceeding the max arity → E003 still fires."""
        source = textwrap.dedent("""\
            proc myproc {x} { return 1 }
            set args {}
            myproc a b {*}$args
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "::myproc" in d.message]
        assert len(errors) >= 1

    def test_builtin_call_arg_expansion_suppresses_too_many(self):
        """``set x {*}$dynamic`` should not fire E003 when ``$dynamic`` is
        unresolvable.

        The IR-level arity check does not yet resolve variable references
        back to constant values, so any ``{*}$var`` expansion is treated
        as contributing 0..∞ arguments and the static count cannot be
        used to decide that too many arguments are present.
        """
        # Use a dynamic value (command substitution) so neither layer can
        # statically resolve the element count.
        source = "set x y {*}[exec true]"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'set'" in d.message]
        assert errors == [], f"unexpected E003 diagnostics: {errors}"

    def test_proc_call_expansion_constant_resolves_to_exact_count(self):
        """``{*}$x`` with a known constant value should use the exact count.

        If the variable's value is statically known (from a prior
        constant ``set``), ``{*}$x`` contributes exactly that many
        arguments — the call should be treated the same as writing
        the elements inline.
        """
        # Exact count — clean
        source = textwrap.dedent("""\
            proc rgbToLab {r g b} { return 1 }
            set rgb1 {255 255 255}
            rgbToLab {*}$rgb1
        """)
        result = analyse(source)
        arity_errors = [
            d
            for d in result.diagnostics
            if d.code in ("E002", "E003") and "::rgbToLab" in d.message
        ]
        assert arity_errors == []

    def test_proc_call_expansion_constant_too_few_still_reports(self):
        """Constant expansion with fewer elements than required → E002."""
        source = textwrap.dedent("""\
            proc rgbToLab {r g b} { return 1 }
            set rgb1 {255 255}
            rgbToLab {*}$rgb1
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "::rgbToLab" in d.message]
        assert len(errors) == 1
        assert "got 2" in errors[0].message

    def test_proc_call_expansion_constant_too_many_still_reports(self):
        """Constant expansion with more elements than allowed → E003."""
        source = textwrap.dedent("""\
            proc greet {name} { return $name }
            set many {a b c}
            greet {*}$many
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "::greet" in d.message]
        assert len(errors) == 1
        assert "got 3" in errors[0].message

    def test_proc_call_expansion_literal_list(self):
        """``{*}{a b c}`` is a literal list with a statically known count."""
        source = textwrap.dedent("""\
            proc foo {a b c} { return 1 }
            foo {*}{1 2 3}
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::foo" in d.message
        ]
        assert errors == []

    def test_proc_call_expansion_literal_list_wrong_count(self):
        """Literal list expansion with wrong element count → E002/E003."""
        source = textwrap.dedent("""\
            proc foo {a b c} { return 1 }
            foo {*}{1 2}
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "::foo" in d.message]
        assert len(errors) == 1

    def test_proc_call_expansion_unknown_variable_is_barrier(self):
        """Unknown dynamic ``$x`` in ``{*}$x`` contributes an unknown count."""
        source = textwrap.dedent("""\
            proc foo {a b c} { return 1 }
            foo {*}$dynamic
        """)
        result = analyse(source)
        # No arity errors — the expansion count is unknown.
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::foo" in d.message
        ]
        assert errors == []

    def test_proc_call_arg_expansion_disabled_for_tcl84(self):
        """In Tcl 8.4 dialect ``{*}`` is not expansion — E002 should still fire."""
        from core.commands.registry.runtime import configure_signatures

        source = textwrap.dedent("""\
            proc rgbToLab {r g b} { return 1 }
            set rgb1 {255 255 255}
            rgbToLab {*}$rgb1
        """)
        try:
            configure_signatures(dialect="tcl8.4")
            result = analyse(source)
            errors = [
                d for d in result.diagnostics if d.code == "E002" and "::rgbToLab" in d.message
            ]
            assert len(errors) >= 1
        finally:
            configure_signatures(dialect="tcl8.6")

    def test_proc_call_arg_expansion_disabled_for_f5_irules(self):
        """iRules is 8.4-based — ``{*}`` is not recognised as expansion."""
        from core.commands.registry.runtime import configure_signatures

        source = textwrap.dedent("""\
            proc myproc {x y z} { return $x }
            when HTTP_REQUEST {
                set args {1 2 3}
                call myproc {*}$args
            }
        """)
        try:
            configure_signatures(dialect="f5-irules")
            result = analyse(source)
            errors = [d for d in result.diagnostics if d.code == "E002" and "::myproc" in d.message]
            assert len(errors) >= 1
        finally:
            configure_signatures(dialect="tcl8.6")

    def test_configure_signatures_sets_lexer_expand_flag(self):
        """``configure_signatures`` must toggle ``TclLexer.expand_syntax``
        based on the active dialect's base Tcl version."""
        from core.commands.registry.runtime import configure_signatures
        from core.parsing.lexer import TclLexer

        try:
            configure_signatures(dialect="tcl8.4")
            assert TclLexer.expand_syntax is False
            configure_signatures(dialect="f5-irules")
            assert TclLexer.expand_syntax is False
            configure_signatures(dialect="tcl8.5")
            assert TclLexer.expand_syntax is True
            configure_signatures(dialect="tcl8.6")
            assert TclLexer.expand_syntax is True
            configure_signatures(dialect="tcl9.0")
            assert TclLexer.expand_syntax is True
            configure_signatures(dialect="f5-iapps")
            assert TclLexer.expand_syntax is True
        finally:
            configure_signatures(dialect="tcl8.6")

    def test_command_name_expansion_skips_arity_check(self):
        """``{*}$cmd args`` expands the command name itself — we can't
        resolve the command, so no arity diagnostic should fire."""
        source = textwrap.dedent("""\
            proc dispatch {name args} { return 1 }
            set cmd dispatch
            set args {a b c}
            {*}$cmd $args
        """)
        result = analyse(source)
        arity = [d for d in result.diagnostics if d.code in ("E001", "E002", "E003", "W001")]
        assert arity == []

    def test_proc_call_expansion_variadic_proc(self):
        """``proc foo {level args}`` with ``{*}`` on trailing args is clean."""
        source = textwrap.dedent("""\
            proc logger {level args} { return $level }
            set rest {a b c}
            logger info {*}$rest
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::logger" in d.message
        ]
        assert errors == []

    def test_proc_call_expansion_proc_with_default_param(self):
        """A call that fills the required arg via ``{*}`` still satisfies
        defaults — no arity error for ``greet {*}$args`` when the value
        is an unknown single name."""
        source = textwrap.dedent("""\
            proc greet {name {title Mr}} { return 1 }
            greet {*}$dynamic
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::greet" in d.message
        ]
        assert errors == []

    def test_proc_call_expansion_constant_exactly_satisfies_variadic(self):
        """Variadic proc with enough constant elements in expansion passes."""
        source = textwrap.dedent("""\
            proc foo {a b args} { return 1 }
            set items {1 2}
            foo {*}$items
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::foo" in d.message
        ]
        assert errors == []

    def test_subcommand_position_expansion_skips_subcommand_check(self):
        """``string {*}$sub text`` — the subcommand position is expanded,
        so W001/E001 and arity checks must be skipped."""
        source = textwrap.dedent("""\
            set sub length
            string {*}$sub hello
        """)
        result = analyse(source)
        noise = [
            d
            for d in result.diagnostics
            if d.code in ("E001", "E002", "E003", "W001") and "string" in d.message
        ]
        assert noise == []

    def test_builtin_expansion_list_cmd_literal_refinement(self):
        """``{*}[list a b]`` resolves to exactly two elements — ``set x {*}[list a b]``
        is 1 + 2 = 3 positional args → E003 for ``set`` (max 2)."""
        source = "set x {*}[list a b]"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'set'" in d.message]
        assert len(errors) == 1
        assert "got 3" in errors[0].message

    def test_builtin_expansion_unknown_var_suppresses_set_arity(self):
        """``set x y {*}$rest`` — ``{*}$rest`` contributes 0..∞ args,
        so E003 must not fire based on the static count."""
        source = "set x y {*}$rest"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'set'" in d.message]
        assert errors == []

    def test_builtin_expansion_leading_options_through_literal(self):
        """``puts {*}{-nonewline} stdout hello`` is valid: the literal
        ``{*}{-nonewline}`` expansion contributes a leading option, not
        a positional argument."""
        source = "puts {*}{-nonewline} stdout hello"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'puts'" in d.message]
        assert errors == []

    def test_builtin_expansion_leading_options_too_many_positional(self):
        """Too many positional arguments after a literal-option expansion
        still produces E003."""
        source = "puts {*}{-nonewline} stdout a b c"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'puts'" in d.message]
        assert len(errors) == 1

    def test_builtin_expansion_literal_with_dollar_in_brace(self):
        """``pwd {*}{$x}`` — the braced literal contains the literal text
        ``$x`` (no substitution because braces).  The list has one
        element, and ``pwd`` takes none, so E003 must fire."""
        source = "pwd {*}{$x}"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003" and "'pwd'" in d.message]
        assert len(errors) == 1
        assert "got 1" in errors[0].message

    def test_builtin_expansion_empty_brace_resolves_to_zero(self):
        """``set {*}{}`` expands to zero elements → E002 fires (set
        requires at least one positional argument)."""
        source = "set {*}{}"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "'set'" in d.message]
        assert len(errors) == 1
        assert "got 0" in errors[0].message

    def test_builtin_expansion_empty_brace_combined_with_positional(self):
        """``set foo {*}{}`` is valid: 1 positional + 0 expanded = 1 arg."""
        source = "set foo {*}{}"
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "'set'" in d.message
        ]
        assert errors == []

    def test_proc_call_expansion_concatenated_word_unknown(self):
        """``foo {*}$y$z`` is a concatenated word (multi-token) — the
        analyser must NOT refine via the first token alone, otherwise
        ``y='a b'`` would falsely resolve to 2 elements when the actual
        runtime count depends on ``$z``."""
        source = textwrap.dedent("""\
            proc foo {x} { return 1 }
            set y "a b"
            set z xyz
            foo {*}$y$z
        """)
        result = analyse(source)
        errors = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::foo" in d.message
        ]
        assert errors == []

    def test_multiline_diagnostics(self):
        source = "set x 1\nbreak extra\nset y 2"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        # The error should be on line 1 (0-indexed)
        assert errors[0].range.start.line == 1

    def test_read_before_set_warning(self):
        result = analyse("puts $x")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 1
        assert warnings[0].severity == Severity.WARNING

    def test_read_before_set_warning_in_expr(self):
        result = analyse("if {$x > 0} { puts yes }")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 1

    def test_read_before_set_clears_after_assignment(self):
        result = analyse("set x 1\nputs $x")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 0

    def test_set_dual_shape_role_resolution(self):
        """``set`` resolver returns VAR_READ for 1-arg shape, VAR_WRITE for 2-arg."""
        from core.commands.registry.runtime import arg_indices_for_role
        from core.commands.registry.signatures import ArgRole

        # set x → read
        assert arg_indices_for_role("set", ["x"], ArgRole.VAR_WRITE) == set()
        assert arg_indices_for_role("set", ["x"], ArgRole.VAR_READ) == {0}
        # set x 1 → write
        assert arg_indices_for_role("set", ["x", "1"], ArgRole.VAR_WRITE) == {0}
        assert arg_indices_for_role("set", ["x", "1"], ArgRole.VAR_READ) == set()

    def test_array_read_uses_base_variable(self):
        result = analyse("set arr(key) 1\nputs $arr(key)")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 0

    def test_regexp_capture_vars_not_read_before_set(self):
        """regexp capture variables should not trigger W210."""
        result = analyse("regexp {^(\\w+)@(\\w+)$} $email -> user domain\nputs $user")
        warnings = [d for d in result.diagnostics if d.code == "W210" and "user" in d.message]
        assert len(warnings) == 0, f"unexpected W210 for regexp capture var: {warnings}"

    def test_regexp_match_var_not_read_before_set(self):
        """The matchVar (first capture) should not trigger W210."""
        result = analyse("regexp {\\d+} $text match\nputs $match")
        warnings = [d for d in result.diagnostics if d.code == "W210" and "match" in d.message]
        assert len(warnings) == 0

    def test_scan_capture_vars_not_read_before_set(self):
        """scan capture variables should not trigger W210."""
        result = analyse('scan "42 hello" "%d %s" num word\nputs $num')
        warnings = [d for d in result.diagnostics if d.code == "W210" and "num" in d.message]
        assert len(warnings) == 0

    def test_regsub_var_not_read_before_set(self):
        """regsub result variable should not trigger W210."""
        result = analyse("regsub {old} $text new result\nputs $result")
        warnings = [d for d in result.diagnostics if d.code == "W210" and "result" in d.message]
        assert len(warnings) == 0

    def test_lassign_vars_not_read_before_set(self):
        """lassign target variables should not trigger W210."""
        result = analyse('lassign {a b c} x y z\nputs "$x $y $z"')
        warnings = [d for d in result.diagnostics if d.code == "W210" and "x" in d.message]
        assert len(warnings) == 0

    def test_unused_assigned_variable_hint(self):
        result = analyse("proc foo {} { set x 1 }")
        hints = [d for d in result.diagnostics if d.code == "W211"]
        assert len(hints) == 1
        assert hints[0].severity == Severity.HINT

    def test_used_variable_no_unused_hint(self):
        result = analyse("proc foo {} { set x 1; puts $x }")
        hints = [d for d in result.diagnostics if d.code == "W211"]
        assert len(hints) == 0

    def test_dead_assignment_detected_with_ssa(self):
        result = analyse("proc foo {} { set x 1; set x 2; puts $x }")
        dead = [d for d in result.diagnostics if d.code == "W220"]
        assert len(dead) == 1
        assert "x" in dead[0].message

    def test_possible_paste_error_for_duplicate_static_assignment(self):
        result = analyse("proc foo {} { set x 0; set x 0; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 1
        assert possible[0].severity == Severity.HINT
        assert "x" in possible[0].message

    def test_possible_paste_error_not_emitted_when_value_changes(self):
        result = analyse("proc foo {} { set x 0; set x 1; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 0

    def test_possible_paste_error_not_emitted_for_dynamic_value(self):
        result = analyse("proc foo {} { set x $y; set x $y; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 0

    def test_constant_if_unreachable_branch(self):
        result = analyse("if {1} { set x 1 } else { set y 2 }")
        unreachable = [d for d in result.diagnostics if d.code == "I230"]
        assert len(unreachable) >= 1

    def test_constant_switch_unreachable_arm(self):
        source = "switch 1 {1 {set x 1} 2 {set y 2} default {set z 3}}"
        result = analyse(source)
        unreachable = [d for d in result.diagnostics if d.code == "I231"]
        assert len(unreachable) >= 1


class TestControlFlow:
    def test_if_body_analysed(self):
        source = "if {1} { set x 42 }"
        result = analyse(source)
        assert "x" in result.global_scope.variables

    def test_while_body_analysed(self):
        source = "while {1} { set x 42; break }"
        result = analyse(source)
        assert "x" in result.global_scope.variables

    def test_for_body_analysed(self):
        source = "for {set i 0} {$i < 10} {incr i} { set x $i }"
        result = analyse(source)
        assert "i" in result.global_scope.variables
        assert "x" in result.global_scope.variables

    def test_if_expr_records_var_reference(self):
        source = "set x 1\nif {$x > 0} { set y 1 }"
        result = analyse(source)
        assert "x" in result.global_scope.variables
        assert len(result.global_scope.variables["x"].references) >= 1

    def test_foreach_defines_var(self):
        source = "foreach item {a b c} { puts $item }"
        result = analyse(source)
        assert "item" in result.global_scope.variables

    def test_foreach_multi_var_individual_ranges(self):
        source = "foreach {a b c} {1 2 3} { puts $a }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables
        # Each variable should have its own distinct range, not all sharing
        # the range of the entire varList token.
        ra = result.global_scope.variables["a"].definition_range
        rb = result.global_scope.variables["b"].definition_range
        rc = result.global_scope.variables["c"].definition_range
        assert ra.start.character != rb.start.character
        assert rb.start.character != rc.start.character
        # Verify each range covers just the variable name (1 char each)
        assert ra.start.character == ra.end.character
        assert rb.start.character == rb.end.character
        assert rc.start.character == rc.end.character

    def test_nested_proc_and_if(self):
        source = textwrap.dedent("""\
            proc check {x} {
                if {== $x 0} {
                    set result zero
                } else {
                    set result nonzero
                }
                return $result
            }
        """)
        result = analyse(source)
        assert "check" in result.global_scope.procs

    def test_if_elseif_else_all_bodies_analysed(self):
        source = "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables

    def test_dict_for_body_analysed(self):
        source = "dict for {k v} $d { set seen 1 }"
        result = analyse(source)
        assert "seen" in result.global_scope.variables

    def test_command_subst_inside_expr_analysed(self):
        source = "set n [expr {[set y 1] + 2}]"
        result = analyse(source)
        assert "y" in result.global_scope.variables

    def test_tcloo_method_body_analysed(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} {
                    set message woof
                }
            }
        """)
        result = analyse(source)
        # Method body variables live in the method's own scope, not global
        assert "::Dog" in result.all_classes
        method_scope = result.global_scope.children[0]
        assert method_scope.name == "Dog::bark"
        assert "message" in method_scope.variables


class TestFixtures:
    @pytest.fixture
    def fixtures_dir(self):
        return Path(__file__).parent / "fixtures"

    def test_simple_tcl(self, fixtures_dir):
        source = (fixtures_dir / "simple.tcl").read_text()
        result = analyse(source)
        assert "greeting" in result.global_scope.variables

    def test_procs_tcl(self, fixtures_dir):
        source = (fixtures_dir / "procs.tcl").read_text()
        result = analyse(source)
        assert "fib" in result.global_scope.procs
        assert "factorial" in result.global_scope.procs

    def test_namespaces_tcl(self, fixtures_dir):
        source = (fixtures_dir / "namespaces.tcl").read_text()
        result = analyse(source)
        # Should find the math namespace
        ns_children = [c for c in result.global_scope.children if c.kind == "namespace"]
        assert any("math" in c.name for c in ns_children)


class TestRegexPatterns:
    """Tests for regex_patterns on AnalysisResult."""

    def test_regexp_simple(self):
        result = analyse("regexp {^[a-z]+$} $str")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "^[a-z]+$"
        assert rp.command == "regexp"

    def test_regexp_with_options(self):
        result = analyse("regexp -nocase -expanded {\\d+} $str")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "\\d+"
        assert rp.command == "regexp"

    def test_regexp_with_option_terminator(self):
        result = analyse("regexp -nocase -- {^test} $str")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "^test"

    def test_regexp_with_start_option(self):
        result = analyse("regexp -start 5 {pattern} $str")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "pattern"

    def test_regsub_simple(self):
        result = analyse("regsub {\\d+} $str replacement result")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "\\d+"
        assert rp.command == "regsub"

    def test_regsub_with_options(self):
        result = analyse("regsub -all -nocase {foo} $str bar result")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "foo"

    def test_switch_regexp_braced(self):
        source = "switch -regexp $x { {^a} {puts a} {^b} {puts b} }"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert "^a" in patterns
        assert "^b" in patterns
        assert all(rp.command == "switch" for rp in result.regex_patterns)

    def test_switch_regexp_inline(self):
        source = "switch -regexp $x {^a} {puts a} {^b} {puts b}"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert "^a" in patterns
        assert "^b" in patterns

    def test_switch_regexp_default_excluded(self):
        source = "switch -regexp $x { {^a} {puts a} default {puts other} }"
        result = analyse(source)
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "^a"

    def test_switch_glob_no_regex(self):
        """switch -glob should NOT record regex patterns."""
        result = analyse("switch -glob $x { a* {puts a} b* {puts b} }")
        assert len(result.regex_patterns) == 0

    def test_switch_exact_no_regex(self):
        """switch -exact should NOT record regex patterns."""
        result = analyse("switch -exact $x { hello {puts hi} }")
        assert len(result.regex_patterns) == 0

    def test_switch_default_no_regex(self):
        """Plain switch (default glob mode) should NOT record regex patterns."""
        result = analyse("switch $x { hello {puts hi} }")
        assert len(result.regex_patterns) == 0

    def test_no_regex_in_other_commands(self):
        result = analyse("set x 1\nputs hello\nif {$x > 0} { puts yes }")
        assert len(result.regex_patterns) == 0

    def test_multiple_regexp_in_same_file(self):
        source = "regexp {^a} $x\nregsub {^b} $y c result"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        commands = {rp.command for rp in result.regex_patterns}
        assert commands == {"regexp", "regsub"}

    def test_regexp_inside_proc(self):
        source = "proc check {s} { regexp {^\\d+$} $s }"
        result = analyse(source)
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].command == "regexp"

    def test_range_points_at_pattern(self):
        source = "regexp {^hello} $str"
        result = analyse(source)
        rp = result.regex_patterns[0]
        # Pattern starts after "regexp " (col 7)
        assert rp.range.start.character == 7


class TestRegexVariablePropagation:
    """Tests for tracking regex patterns through variable assignments."""

    def test_set_then_regexp(self):
        """set pat {regex}; regexp $pat $str — should find 2 regex_patterns."""
        source = "set pat {^\\d+$}\nregexp $pat $str"
        result = analyse(source)
        # One for the $pat usage in regexp, one for the set definition site
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert all(p == "^\\d+$" for p in patterns)

    def test_set_then_regsub(self):
        source = "set pat {foo}\nregsub $pat $str bar result"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        assert all(rp.pattern == "foo" for rp in result.regex_patterns)

    def test_set_then_switch_regexp(self):
        source = "set pat {^hello}\nswitch -regexp $x $pat {puts matched}"
        result = analyse(source)
        assert any(rp.pattern == "^hello" for rp in result.regex_patterns)

    def test_variable_not_constant_no_regex(self):
        """When the variable's value is dynamic, don't record regex."""
        source = "set pat $dynamic_value\nregexp $pat $str"
        result = analyse(source)
        # Only direct patterns are recorded, not dynamic ones
        assert len(result.regex_patterns) == 0

    def test_variable_with_interpolation_no_regex(self):
        """set pat "^$prefix"; regexp $pat $str — not constant."""
        source = 'set pat "^$prefix"\nregexp $pat $str'
        result = analyse(source)
        assert len(result.regex_patterns) == 0

    def test_variable_reassigned_dynamic(self):
        """Variable reassigned to non-constant loses const tracking."""
        source = "set pat {^\\d+}\nset pat $other\nregexp $pat $str"
        result = analyse(source)
        # pat was reassigned dynamically, so $pat in regexp is not a regex
        assert len(result.regex_patterns) == 0

    def test_variable_in_proc_scope(self):
        source = "proc check {s} { set pat {^\\w+$}; regexp $pat $s }"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        assert all(rp.pattern == "^\\w+$" for rp in result.regex_patterns)

    def test_literal_and_variable_patterns_mixed(self):
        """File with both literal and variable patterns."""
        source = "set pat {^a}\nregexp $pat $str\nregexp {^b} $str2"
        result = analyse(source)
        patterns = sorted(rp.pattern for rp in result.regex_patterns)
        assert "^a" in patterns
        assert "^b" in patterns

    def test_definition_site_marked_as_regex(self):
        """The 'set pat {regex}' definition site should itself be a regex_pattern."""
        source = "set pat {^test}\nregexp $pat $str"
        result = analyse(source)
        # Find the regex pattern that points at the definition site (col 0 for "set pat")
        # The definition range should point at where the var was defined (the var name token)
        def_patterns = [rp for rp in result.regex_patterns if rp.range.start.line == 0]
        assert len(def_patterns) == 1
        assert def_patterns[0].pattern == "^test"

    def test_use_site_marked_as_regex(self):
        """The '$pat' usage site should be a regex_pattern."""
        source = "set pat {^test}\nregexp $pat $str"
        result = analyse(source)
        use_patterns = [rp for rp in result.regex_patterns if rp.range.start.line == 1]
        assert len(use_patterns) == 1
        assert use_patterns[0].pattern == "^test"
        assert use_patterns[0].command == "regexp"


class TestPackageRequire:
    def test_simple_require(self):
        result = analyse("package require http")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version is None

    def test_require_with_version(self):
        result = analyse("package require http 2.9")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version == "2.9"

    def test_require_exact(self):
        result = analyse("package require -exact http 2.9")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version == "2.9"

    def test_multiple_requires(self):
        source = "package require http\npackage require tls"
        result = analyse(source)
        assert len(result.package_requires) == 2
        names = {pr.name for pr in result.package_requires}
        assert names == {"http", "tls"}

    def test_package_provide_not_recorded(self):
        result = analyse("package provide mylib 1.0")
        assert len(result.package_requires) == 0

    def test_no_package_commands(self):
        result = analyse("set x 42")
        assert len(result.package_requires) == 0


# W214
class TestUnusedProcParameters:
    def test_unused_param_detected(self):
        result = analyse("proc foo {x y} { puts $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "y" in w214[0].message
        assert w214[0].severity == Severity.HINT

    def test_all_params_used_no_warning(self):
        result = analyse("proc foo {a b} { puts $a; puts $b }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_args_param_not_flagged(self):
        result = analyse("proc foo {args} { puts hello }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_underscore_prefix_not_flagged(self):
        result = analyse("proc foo {_unused x} { puts $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_multiple_unused_params(self):
        result = analyse("proc foo {a b c} { puts hello }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 3

    def test_param_used_in_return(self):
        result = analyse("proc foo {x} { return $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_param_used_in_branch_condition(self):
        result = analyse("proc foo {x} { if {$x > 0} { puts yes } }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_param_used_in_expr_alias_no_warning(self):
        """interp alias for expr — variable references in the aliased
        expression argument must count as reads (issue #42)."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc foo {x y} {
                set result [= {$x + $y}]
                return $result
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_param_used_in_expr_alias_complex(self):
        """Full example from issue #42 — relPos should not be flagged."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc calc {index relPos} {
                set newIndex [= {$index+$relPos}]
                return $newIndex
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_param_used_in_dict_for_body_issue_236(self):
        """Issue #236 — parameter referenced inside a ``dict for`` body
        (which is lowered as an opaque ``::tcl::dict::for`` barrier) must
        be detected as used."""
        source = textwrap.dedent("""\
            proc allSpecsUpdate {{count 0}} {
                dict for {k v} $things {
                    if {$count>0} {
                        incr count
                    }
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_param_used_inside_nested_braced_body(self):
        """A parameter referenced only inside a nested ``if`` body that
        sits inside a ``dict for`` body must not be flagged."""
        source = textwrap.dedent("""\
            proc f {but} {
                dict for {k v} $d {
                    if {$but ne ""} { puts $but }
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_param_used_only_in_dict_map_body(self):
        """``dict map`` follows the same opaque-body lowering as ``dict for``."""
        source = textwrap.dedent("""\
            proc f {scale} {
                dict map {k v} $d { expr {$v * $scale} }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_unused_param_in_dict_for_still_flagged(self):
        """Make sure the broader scan does not silence true unused
        parameters that appear alongside used ones."""
        source = textwrap.dedent("""\
            proc f {{count 0} other} {
                dict for {k v} $d {
                    if {$count>0} { incr count }
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "other" in w214[0].message

    def test_param_read_inside_catch_body(self):
        """A parameter referenced via ``$``-substitution inside a
        ``catch { ... }`` body must be observed as used."""
        source = textwrap.dedent("""\
            proc f {x} {
                catch { puts $x }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_param_used_only_by_dict_with_issue_307(self):
        """Issue #307 — a parameter passed to ``dict with`` as the dict
        variable must count as used.  The barrier marks the variable as
        both VAR_READ and VAR_WRITE; the read must survive the
        defs-filter in ``_uses``."""
        source = textwrap.dedent("""\
            proc f {pdata} {
                dict with pdata {}
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_param_used_only_by_dict_update_issue_307(self):
        """``dict update`` shares the same VAR_READ+VAR_WRITE shape as
        ``dict with``; a parameter that is only the dict variable must
        not be flagged as unused."""
        source = textwrap.dedent("""\
            proc f {d} {
                dict update d k v {}
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert w214 == []

    def test_pure_write_to_param_inside_body_still_flags(self):
        """A parameter that is only *written* (never read) inside an
        opaque body is still unused — the deep body scan must not count
        ``set x 1`` etc. as a read."""
        source = textwrap.dedent("""\
            proc f {x} {
                dict for {k v} $d { set x 1 }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "x" in w214[0].message

    def test_namespaced_for_command_does_not_get_dict_body_fallback(self):
        """Only ``::tcl::dict::for|map`` get the synthetic body-arg
        fallback — an unrelated ``::my::for`` must not have its third
        arg scanned as a script (which would silently suppress W214)."""
        source = textwrap.dedent("""\
            proc f {x} {
                ::my::for a b { set x 1 }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "x" in w214[0].message

    def test_trace_add_variable_callback_unused_args(self):
        """Issue #239: ``trace add variable`` callbacks must accept the
        ``name1 name2 op`` signature; the body legitimately may not use
        any of those arguments, so W214 should not fire."""
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} {
                puts hello
            }
            trace add variable x write watcher
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0
        assert result.all_procs["::watcher"].is_trace_callback

    def test_trace_add_command_callback_unused_args(self):
        source = textwrap.dedent("""\
            proc onCmd {oldName newName op} { puts hi }
            trace add command mycmd rename onCmd
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_trace_add_execution_callback_unused_args(self):
        source = textwrap.dedent("""\
            proc onExec {cmd code result op} { puts hi }
            trace add execution mycmd enter onExec
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_legacy_trace_variable_callback_unused_args(self):
        """The deprecated ``trace variable name ops command`` form is
        still in use and must also suppress W214."""
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} { puts hello }
            trace variable x w watcher
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_trace_callback_braced_list_prefix(self):
        """When the command prefix is a literal list, the first element
        is the proc and W214 is suppressed for it."""
        source = textwrap.dedent("""\
            proc watcher {prefix name1 name2 op} { puts $prefix }
            trace add variable x write {watcher hello}
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_trace_add_before_proc_definition(self):
        """The proc may be defined after the ``trace add`` site."""
        source = textwrap.dedent("""\
            trace add variable x write watcher
            proc watcher {name1 name2 op} { puts hello }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_trace_callback_in_namespace(self):
        source = textwrap.dedent("""\
            namespace eval ns {
                proc watcher {name1 name2 op} { puts hello }
                trace add variable x write watcher
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_unrelated_proc_unused_param_still_flagged(self):
        """Suppression must be scoped to trace-registered procs only."""
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} { puts hello }
            proc other {a b} { puts $a }
            trace add variable x write watcher
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "'b'" in w214[0].message and "'other'" in w214[0].message

    def test_trace_with_qualified_command_name(self):
        """``::trace`` (qualified built-in) is recognised the same as ``trace``."""
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} { puts hello }
            ::trace add variable x write watcher
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_trace_callback_marks_all_namespace_candidates(self):
        """Tcl resolves the trace ``commandPrefix`` at fire time using the
        namespace active at the variable-write site.  When a same-named
        proc exists in both the registration namespace and the global
        namespace, either could be the actual callback target — we mark
        both to avoid false-positive W214 on the real callback."""
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} { puts global }
            namespace eval ns {
                proc watcher {name1 name2 op} { puts ns }
                trace add variable ::x write watcher
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0
        assert result.all_procs["::watcher"].is_trace_callback
        assert result.all_procs["::ns::watcher"].is_trace_callback


# interp alias
class TestInterpAlias:
    def test_alias_recorded(self):
        result = analyse("interp alias {} = {} expr")
        assert "::=" in result.command_aliases
        assert result.command_aliases["::="] == ("expr", ())

    def test_alias_with_prepended_args(self):
        result = analyse("interp alias {} myput {} puts stdout")
        assert "::myput" in result.command_aliases
        assert result.command_aliases["::myput"] == ("puts", ("stdout",))

    def test_alias_non_current_interp_not_recorded(self):
        result = analyse("interp alias child = {} expr")
        assert "::=" not in result.command_aliases

    def test_alias_expr_analysis(self):
        """Standalone expr alias call should have its argument analysed as expr."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            = {1 + 2}
        """)
        result = analyse(source)
        # No errors — the expression is valid
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) == 0

    def test_alias_body_analysis(self):
        """Alias for a body-taking command should recurse into the body."""
        source = textwrap.dedent("""\
            interp alias {} myeval {} eval
            proc foo {x} {
                myeval { set y 1 }
                return $x
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_alias_qualified_name(self):
        """Alias with ``::`` prefix is normalised and resolved."""
        result = analyse("interp alias {} ::myexpr {} expr")
        assert "::myexpr" in result.command_aliases

    def test_alias_resolved_from_namespace(self):
        """Alias created as ``::math::=`` resolves inside ``namespace eval math``."""
        source = textwrap.dedent("""\
            interp alias {} ::math::= {} expr
            namespace eval math {
                proc calc {x y} {
                    set result [= {$x + $y}]
                    return $result
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_alias_global_fallback(self):
        """Unqualified alias resolves from any namespace via global fallback."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            namespace eval utils {
                proc calc {x y} {
                    set result [= {$x + $y}]
                    return $result
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_alias_explicit_global_call(self):
        """Calling ``::=`` resolves the alias registered as ``=``."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc calc {x y} {
                set result [::= {$x + $y}]
                return $result
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_alias_chain_not_resolved(self):
        """Alias-of-alias should NOT resolve transitively."""
        source = textwrap.dedent("""\
            interp alias {} a {} b
            interp alias {} b {} expr
        """)
        result = analyse(source)
        assert result.command_aliases["::a"] == ("b", ())
        assert result.command_aliases["::b"] == ("expr", ())

    def test_alias_dynamic_name_not_recorded(self):
        """Variable in alias name is not resolved statically."""
        source = 'set n "="\ninterp alias {} $n {} expr'
        result = analyse(source)
        # $n is dynamic — stored literally, not as "="
        assert "::=" not in result.command_aliases

    def test_alias_redefinition_overwrites(self):
        """Redefining an alias updates the target."""
        source = textwrap.dedent("""\
            interp alias {} myop {} expr
            interp alias {} myop {} puts
        """)
        result = analyse(source)
        assert result.command_aliases["::myop"] == ("puts", ())

    def test_alias_too_few_args_no_crash(self):
        """interp alias {} = {} (missing target) should not crash."""
        result = analyse("interp alias {} = {}")
        assert "::=" not in result.command_aliases

    def test_alias_target_in_child_interp(self):
        """interp alias {} name child target — not tracked (target in child)."""
        result = analyse("interp alias {} myexpr child expr")
        assert "::myexpr" not in result.command_aliases

    def test_alias_query_form_no_crash(self):
        """interp alias {} name (query form, 3 args) — should not crash."""
        result = analyse("interp alias {} myexpr")
        assert "::myexpr" not in result.command_aliases

    def test_alias_for_foreach(self):
        """Alias for foreach — body and var_name roles work through alias."""
        source = textwrap.dedent("""\
            interp alias {} myforeach {} foreach
            proc foo {items} {
                myforeach item $items { puts $item }
                return $item
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    # Real-world patterns
    def test_real_world_math_operators(self):
        """Common Tcl pattern: operator aliases for expr (= + - * /)."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            interp alias {} + {} expr
            proc calculate {base rate discount} {
                set subtotal [= {$base * $rate}]
                set total [= {$subtotal - $discount}]
                return $total
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_real_world_logging_alias_with_prepended(self):
        """Common pattern: logging alias with channel prepended."""
        source = textwrap.dedent("""\
            interp alias {} log {} puts stderr
            proc process {data} {
                log "Processing: $data"
                return $data
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_real_world_namespace_dsl(self):
        """DSL-style namespace with aliased commands."""
        source = textwrap.dedent("""\
            interp alias {} ::dsl::calculate {} expr
            namespace eval dsl {
                proc run {x y} {
                    set result [calculate {$x + $y}]
                    return $result
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_real_world_safe_interp_not_tracked(self):
        """Safe interpreter aliases should not be tracked (real-world pattern)."""
        source = textwrap.dedent("""\
            set i [interp create -safe]
            interp alias $i add {} ::api::add
            proc ::api::add {a b} {
                return [expr {$a + $b}]
            }
        """)
        result = analyse(source)
        # $i is not the current interp — should not be tracked
        assert "::add" not in result.command_aliases

    def test_real_world_multi_arg_expr(self):
        """Real issue #42 pattern: proc params used in aliased expr."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc relativePosition {index relPos} {
                set newIndex [= {$index + $relPos}]
                if {$newIndex < 0} {
                    set newIndex 0
                }
                return $newIndex
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_real_world_file_shortcuts(self):
        """Unix-like file command shortcuts (common interactive Tcl pattern)."""
        source = textwrap.dedent("""\
            interp alias {} cp {} file copy -force
            interp alias {} mkdir {} file mkdir
            interp alias {} rm {} file delete -force
        """)
        result = analyse(source)
        assert "::cp" in result.command_aliases
        assert result.command_aliases["::cp"] == ("file", ("copy", "-force"))
        assert result.command_aliases["::mkdir"] == ("file", ("mkdir",))
        assert result.command_aliases["::rm"] == ("file", ("delete", "-force"))

    def test_real_world_dynamic_html_tag_aliases(self):
        """Dynamic alias creation via foreach (common metaprogramming pattern).

        Since the alias name is ``$tag`` (a variable), the analyser cannot
        resolve it statically — this tests that it does not crash.
        """
        source = textwrap.dedent("""\
            proc html_tag {tag args} { return "<$tag>$args</$tag>" }
            foreach tag {h1 h2 h3 p div span} {
                interp alias {} $tag {} html_tag $tag
            }
        """)
        result = analyse(source)
        # Dynamic names — none should be tracked as literal aliases
        assert "::h1" not in result.command_aliases

    def test_real_world_cross_interp_alias(self):
        """Cross-interpreter alias (from test_vm_basic_test.py pattern).

        ``interp alias {} localGreet child greet`` has target path "child"
        (not empty), so the alias target is in a child interp.  We only
        track aliases where both source and target paths are empty.
        """
        source = textwrap.dedent("""\
            interp create child
            interp eval child { proc greet {} { return hello } }
            interp alias {} localGreet child greet
            localGreet
            interp delete child
        """)
        result = analyse(source)
        # Target path is "child" (not empty) — not tracked.
        assert "::localGreet" not in result.command_aliases

    def test_real_world_alias_deletion_form(self):
        """Alias deletion: ``interp alias {} name {}`` (empty target path, no target).

        This 4-arg form should not be recorded (it deletes the alias).
        """
        source = textwrap.dedent("""\
            interp alias {} myalias {} list
            interp alias {} myalias {}
        """)
        result = analyse(source)
        # The second call has only 4 args after "interp" — should not be
        # detected as a new alias (gated by len >= 5).  The first call's
        # alias should still be present since we don't track deletions.
        assert "::myalias" in result.command_aliases
        assert result.command_aliases["::myalias"] == ("list", ())

    # return [expr/alias {...}] — issue #42 reopened
    def test_return_expr_braced_no_w214(self):
        """return [expr {$x + 1}] should not produce W214."""
        source = "proc foo {x} { return [expr {$x + 1}] }"
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_return_expr_alias_no_w214(self):
        """Issue #42 reopened: return [= {2*$width+2*$length}] with alias."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc pdPsCalc {width length} {
                return [= {2*$width+2*$length}]
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_return_cmd_subst_no_w214(self):
        """return [string length $x] should not produce W214."""
        source = "proc foo {x} { return [string length $x] }"
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_issue_42_original_example(self):
        """Full original example from issue #42 — moveKeyInDict."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc moveKeyInDict {dict key relPos} {
                set keys [dkeys $dict]
                set index [lsearch -exact $keys $key]
                set newIndex [= {$index+$relPos}]
                set prevKey [@ $keys $newIndex]
                set newKeysOrder [lreplace [lreplace $keys $index $index $prevKey] $newIndex $newIndex $key]
                foreach key $newKeysOrder {
                    dict append newDict $key [dget $dict $key]
                }
                return $newDict
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_issue_42_reopened_example(self):
        """Reopened example from issue #42 — pdPsCalc."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            set scriptPath [file dirname [file normalize [info script]]]
            proc pdPsCalc {width length} {
                return [= {2*$width+2*$length}]
            }
            set vSupply 2.0
            set inpFreq 850e6
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_return_unused_param_still_warns(self):
        """Truly unused param should still produce W214 with return."""
        source = "proc foo {x y} { return [expr {$x + 1}] }"
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "y" in w214[0].message

    def test_dict_for_body_uses_no_w214(self):
        """Issue #234: params used inside ``dict for`` body must not trigger W214."""
        source = textwrap.dedent("""\
            proc foo {targetKey} {
                set d [dict create]
                dict for {k v} $d {
                    if {[dict exists $v $targetKey]} { puts hi }
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_issue_234_dict_prune(self):
        """Full ``dictPrune`` example from issue #234 — no W214 false positives."""
        source = textwrap.dedent("""\
            proc dictPrune {tree targetKey maxDepth retainRules skipRules {cutBelow 0} {level 0}} {
                set result [dict create]
                dict for {k v} $tree {
                    if {[dict exists $retainRules $level]
                        && [lsearch -exact [dict get $retainRules $level] $k] < 0} {
                        continue
                    }
                    if {[dict exists $skipRules $level]
                        && [lsearch -exact [dict get $skipRules $level] $k] >= 0} {
                        continue
                    }
                    if {$level == $maxDepth} {
                        if {[dict exists $v $targetKey]} {
                            if {$cutBelow} {
                                dict set result $k [dict create]
                            } else {
                                dict set result $k $v
                            }
                        }
                    } else {
                        set sub [dictPrune $v $targetKey $maxDepth $retainRules $skipRules $cutBelow [expr {$level+1}]]
                        if {[dict size $sub] > 0} {
                            dict set result $k $sub
                        }
                    }
                }
                return $result
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_dict_map_body_uses_no_w214(self):
        """``dict map`` body usage tracked the same as ``dict for``."""
        source = textwrap.dedent("""\
            proc foo {threshold} {
                set d [dict create]
                dict map {k v} $d {
                    if {$v > $threshold} { set v $threshold }
                    set v
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_dict_for_body_truly_unused_still_warns(self):
        """Param not referenced in ``dict for`` body still triggers W214."""
        source = textwrap.dedent("""\
            proc foo {used unused} {
                set d [dict create]
                dict for {k v} $d {
                    puts $used
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "unused" in w214[0].message

    def test_dict_for_braced_data_word_not_a_use(self):
        """Braced data words inside ``dict for`` bodies must not count as uses.

        ``{$unused}`` is a literal — Tcl braces inhibit substitution.  The
        deep body scanner only descends into argument positions registered
        as ``ArgRole.BODY``/``EXPR`` so plain data words are left alone.
        """
        source = textwrap.dedent("""\
            proc foo {used unused} {
                set d [dict create]
                dict for {k v} $d {
                    set msg {$unused}
                    puts $used
                }
            }
        """)
        result = analyse(source)
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "unused" in w214[0].message


class TestW123UnresolvedCommand:
    """W123: Unresolved command detection and unknown proc analysis."""

    @staticmethod
    def _w123(source: str) -> list:
        result = analyse(source)
        return [d for d in result.diagnostics if d.code == "W123"]

    def test_unknown_command_emits_w123(self):
        diags = self._w123("mycommand a b c")
        assert len(diags) == 1
        assert "mycommand" in diags[0].message

    def test_known_builtin_no_w123(self):
        diags = self._w123("set x 1")
        assert len(diags) == 0

    def test_user_proc_no_w123(self):
        diags = self._w123("proc mycommand {x} { puts $x }\nmycommand hello")
        assert len(diags) == 0

    def test_forward_defined_proc_no_w123(self):
        """Proc defined after usage should still suppress W123."""
        diags = self._w123("mycommand hello\nproc mycommand {x} { puts $x }")
        assert len(diags) == 0

    def test_namespace_qualified_skipped(self):
        diags = self._w123("::myns::cmd arg1")
        assert len(diags) == 0

    def test_variable_command_skipped(self):
        diags = self._w123("set cmd puts\n$cmd hello")
        # $cmd starts with $, should be skipped
        assert all("$cmd" not in d.message for d in diags)

    def test_dynamic_providers_suppress(self):
        diags = self._w123("load mylib.so\nmycommand arg1")
        assert len(diags) == 0

    def test_stub_command_no_w123(self):
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub mycommand {arg1 arg2}
            # tcl-lsp: stubs-end
            mycommand x y
        """)
        diags = self._w123(source)
        assert len(diags) == 0

    # --- unknown proc analysis ---

    def test_unknown_proc_switch_dispatch(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                    bar { puts bar }
                }
            }
            foo x
            bar y
        """)
        diags = self._w123(source)
        assert len(diags) == 0

    def test_unknown_proc_switch_dispatch_unhandled(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                }
            }
            foo x
            baz y
        """)
        diags = self._w123(source)
        assert len(diags) == 1
        assert "baz" in diags[0].message

    def test_unknown_proc_empty_stub(self):
        source = textwrap.dedent("""\
            proc unknown {args} {}
            mycommand arg1
        """)
        diags = self._w123(source)
        assert len(diags) == 1
        assert "mycommand" in diags[0].message

    def test_unknown_proc_chains_original_suppresses(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                _original_unknown $cmd {*}$args
            }
            mycommand arg1
        """)
        diags = self._w123(source)
        assert len(diags) == 0

    def test_unknown_proc_auto_load_suppresses(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                auto_load $cmd
            }
            mycommand arg1
        """)
        diags = self._w123(source)
        assert len(diags) == 0

    def test_unknown_proc_exec_suppresses(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                exec $cmd {*}$args
            }
            mycommand arg1
        """)
        diags = self._w123(source)
        assert len(diags) == 0

    # --- did you mean? ---

    def test_did_you_mean_suggestion(self):
        diags = self._w123("putz hello")
        assert len(diags) == 1
        assert "puts" in diags[0].message
        assert diags[0].fixes
        assert diags[0].fixes[0].new_text == "puts"

    def test_no_suggestion_for_distant_name(self):
        diags = self._w123("xyzzyplugh arg1")
        assert len(diags) == 1
        assert "did you mean" not in diags[0].message

    # --- unknown_proc_info on result ---

    def test_unknown_proc_info_populated(self):
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                    bar { puts bar }
                }
            }
        """)
        result = analyse(source)
        upi = result.unknown_proc_info
        assert upi is not None
        assert "foo" in upi.dispatch_targets
        assert "bar" in upi.dispatch_targets
        assert not upi.empty_stub
        assert not upi.chains_original

    def test_unknown_proc_info_empty_stub(self):
        result = analyse("proc unknown {args} {}")
        upi = result.unknown_proc_info
        assert upi is not None
        assert upi.empty_stub

    def test_no_unknown_proc_info_by_default(self):
        result = analyse("set x 1")
        assert result.unknown_proc_info is None

    # --- new suppression patterns ---

    def test_package_require_suppresses(self):
        diags = self._w123("package require Tk\nbutton .b -text hi")
        assert len(diags) == 0

    def test_rename_suppresses(self):
        diags = self._w123("rename puts myputs\nmyputs hello")
        assert len(diags) == 0

    def test_namespace_import_suppresses(self):
        diags = self._w123("namespace import ::foo::*\nbar arg")
        assert len(diags) == 0

    def test_lappend_auto_path_suppresses(self):
        diags = self._w123("lappend auto_path /opt/mylib\nmycmd arg")
        assert len(diags) == 0

    # --- unknown proc edge cases ---

    def test_unknown_if_else_chains_original(self):
        """Chain detection inside if/else body (not just top-level)."""
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                if {$cmd eq "foo"} {
                    puts foo
                } else {
                    _original_unknown $cmd {*}$args
                }
            }
            baz x
        """)
        diags = self._w123(source)
        assert len(diags) == 0
        result = analyse(source)
        assert result.unknown_proc_info is not None
        assert result.unknown_proc_info.chains_original

    def test_multiple_unknown_procs_last_wins(self):
        """Second ``proc unknown`` definition overwrites the first."""
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                _original_unknown $cmd {*}$args
            }
            proc unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                }
            }
            foo x
            baz y
        """)
        result = analyse(source)
        upi = result.unknown_proc_info
        assert upi is not None
        # Second definition does not chain — it only dispatches foo.
        assert not upi.chains_original
        assert "foo" in upi.dispatch_targets
        diags = [d for d in result.diagnostics if d.code == "W123"]
        assert any("baz" in d.message for d in diags)

    def test_unknown_switch_glob_suppresses(self):
        """switch -glob in unknown handler suppresses W123 entirely."""
        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                switch -glob $cmd {
                    fo* { puts foo }
                }
            }
            foobar x
        """)
        result = analyse(source)
        upi = result.unknown_proc_info
        assert upi is not None
        assert upi.has_pattern_dispatch
        diags = [d for d in result.diagnostics if d.code == "W123"]
        assert len(diags) == 0

    # --- alias integration ---

    def test_alias_command_no_w123(self):
        """Commands defined via interp alias should not trigger W123."""
        diags = self._w123("interp alias {} = {} expr\n= {1 + 2}")
        assert len(diags) == 0

    def test_alias_in_did_you_mean(self):
        """Alias names should appear in 'did you mean?' suggestions."""
        source = textwrap.dedent("""\
            interp alias {} myput {} puts stdout
            myptu hello
        """)
        diags = self._w123(source)
        assert len(diags) == 1
        assert "myput" in diags[0].message

    def test_unknown_proc_lowering_failure_suppresses(self):
        """When IR lowering of the unknown proc body fails, treat as opaque."""
        from unittest.mock import patch

        source = textwrap.dedent("""\
            proc unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                }
            }
            baz x
        """)
        with patch(
            "core.compiler.lowering.lower_to_ir",
            side_effect=RuntimeError("lowering failed"),
        ):
            result = analyse(source)
        upi = result.unknown_proc_info
        assert upi is not None
        assert upi.chains_original  # opaque — suppresses W123
        diags = [d for d in result.diagnostics if d.code == "W123"]
        assert len(diags) == 0

    def test_tcl_unknown_qualified(self):
        """``proc ::tcl::unknown`` is recognised as the unknown handler."""
        source = textwrap.dedent("""\
            proc ::tcl::unknown {cmd args} {
                switch $cmd {
                    foo { puts foo }
                }
            }
        """)
        result = analyse(source)
        upi = result.unknown_proc_info
        assert upi is not None
        assert "foo" in upi.dispatch_targets


class TestSourceTargets:
    """Verify that the analyser extracts ``source`` command targets."""

    def test_literal_source(self):
        result = analyse("source lib/utils.tcl")
        assert len(result.source_targets) == 1
        st = result.source_targets[0]
        assert st.raw_path == "lib/utils.tcl"
        assert st.is_literal is True

    def test_variable_source_not_literal(self):
        result = analyse("source $dir/utils.tcl")
        assert len(result.source_targets) == 1
        assert result.source_targets[0].is_literal is False

    def test_cmd_subst_source_not_literal(self):
        result = analyse("source [file join [file dirname [info script]] helper.tcl]")
        assert len(result.source_targets) == 1
        assert result.source_targets[0].is_literal is False

    def test_source_with_encoding(self):
        result = analyse("source -encoding utf-8 myfile.tcl")
        assert len(result.source_targets) == 1
        st = result.source_targets[0]
        assert st.raw_path == "myfile.tcl"
        assert st.is_literal is True

    def test_multiple_sources(self):
        source = "source a.tcl\nsource b.tcl\nsource $c"
        result = analyse(source)
        assert len(result.source_targets) == 3
        assert result.source_targets[0].is_literal is True
        assert result.source_targets[1].is_literal is True
        assert result.source_targets[2].is_literal is False


class TestTclOOClassExtraction:
    """Tests for TclOO class definition extraction."""

    def test_class_create_basic(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} { return "woof" }
            }
        """)
        result = analyse(source)
        assert "::Dog" in result.all_classes
        cd = result.all_classes["::Dog"]
        assert cd.name == "Dog"
        assert cd.metaclass == "oo::class"
        assert "bark" in cd.methods
        assert cd.methods["bark"].kind == "method"

    def test_class_superclass(self):
        source = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} { error "abstract" }
            }
            oo::class create Dog {
                superclass Animal
                method bark {} { return "woof" }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert cd.superclasses == ["Animal"]

    def test_class_variables(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                variable name breed
                constructor {n b} { set name $n; set breed $b }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert cd.variables == ["name", "breed"]
        assert len(cd.constructors) == 1
        assert cd.constructors[0].kind == "constructor"
        assert [p.name for p in cd.constructors[0].params] == ["n", "b"]

    def test_class_method_params(self):
        source = textwrap.dedent("""\
            oo::class create Calc {
                method add {a b} { expr {$a + $b} }
            }
        """)
        result = analyse(source)
        md = result.all_classes["::Calc"].methods["add"]
        assert [p.name for p in md.params] == ["a", "b"]

    def test_class_destructor(self):
        source = textwrap.dedent("""\
            oo::class create Conn {
                destructor { close $fd }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Conn"]
        assert cd.destructor is not None
        assert cd.destructor.kind == "destructor"

    def test_class_mixins(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                mixin Serializable Comparable
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert cd.mixins == ["Serializable", "Comparable"]

    def test_class_forward(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                forward run ::dog::run_impl
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert "run" in cd.methods
        assert cd.methods["run"].kind == "forward"

    def test_class_filter_export_unexport(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} {}
                method internal {} {}
                filter myfilter
                export bark
                unexport internal
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert cd.filters == ["myfilter"]
        assert "bark" in cd.exports
        assert "internal" in cd.unexports

    def test_oo_define_body_merges(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} { return "woof" }
            }
            oo::define Dog {
                method fetch {item} { return $item }
                classmethod count {} { return 0 }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert "bark" in cd.methods
        assert "fetch" in cd.methods
        assert "count" in cd.class_methods

    def test_oo_define_inline(self):
        source = 'oo::define Dog method sit {} { return "sitting" }'
        result = analyse(source)
        cd = result.all_classes["::Dog"]
        assert "sit" in cd.methods

    def test_oo_define_creates_partial_class(self):
        """oo::define for an unseen class creates a partial ClassDef."""
        source = textwrap.dedent("""\
            oo::define MyClass {
                method foo {} {}
            }
        """)
        result = analyse(source)
        assert "::MyClass" in result.all_classes
        assert "foo" in result.all_classes["::MyClass"].methods

    def test_configurable_metaclass(self):
        source = textwrap.dedent("""\
            oo::configurable create Point {
                property x y
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Point"]
        assert cd.metaclass == "oo::configurable"
        assert "x" in cd.properties
        assert "y" in cd.properties

    def test_abstract_metaclass(self):
        source = textwrap.dedent("""\
            oo::abstract create Shape {
                method area {} { error "abstract" }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Shape"]
        assert cd.metaclass == "oo::abstract"

    def test_singleton_metaclass(self):
        source = textwrap.dedent("""\
            oo::singleton create Logger {
                method log {msg} { puts $msg }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Logger"]
        assert cd.metaclass == "oo::singleton"

    def test_method_body_variables_in_scope(self):
        """Instance variables declared via class-level 'variable' are available in method bodies."""
        source = textwrap.dedent("""\
            oo::class create Dog {
                variable name
                constructor {n} { set name $n }
                method bark {} { return $name }
            }
        """)
        result = analyse(source)
        # The method scope should have 'name' as a variable
        bark_scope = [s for s in result.global_scope.children if s.name == "Dog::bark"]
        assert len(bark_scope) == 1
        assert "name" in bark_scope[0].variables

    def test_classmethod_extraction(self):
        source = textwrap.dedent("""\
            oo::class create Counter {
                classmethod instances {} { return 0 }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Counter"]
        assert "instances" in cd.class_methods
        assert cd.class_methods["instances"].kind == "classmethod"

    def test_private_method(self):
        source = textwrap.dedent("""\
            oo::class create Foo {
                private method helper {} { return 1 }
            }
        """)
        result = analyse(source)
        cd = result.all_classes["::Foo"]
        assert "helper" in cd.methods
        assert cd.methods["helper"].visibility == "private"

    def test_class_in_namespace(self):
        source = textwrap.dedent("""\
            namespace eval shapes {
                oo::class create Circle {
                    variable radius
                    method area {} { expr {3.14 * $radius * $radius} }
                }
            }
        """)
        result = analyse(source)
        assert "::shapes::Circle" in result.all_classes
        cd = result.all_classes["::shapes::Circle"]
        assert cd.variables == ["radius"]
        assert "area" in cd.methods

    def test_scope_on_class(self):
        """ClassDef is registered on the enclosing scope."""
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} {}
            }
        """)
        result = analyse(source)
        assert "Dog" in result.global_scope.classes
