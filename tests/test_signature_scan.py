"""Tests for the lightweight signature-only scan."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.signature_scan import extract_signatures


class TestProcs:
    def test_top_level_proc(self):
        result = extract_signatures("proc greet {name} { puts $name }")
        assert "::greet" in result.all_procs
        pd = result.all_procs["::greet"]
        assert pd.name == "greet"
        assert [p.name for p in pd.params] == ["name"]

    def test_proc_in_namespace_eval(self):
        src = "namespace eval math { proc add {a b} { return [+ $a $b] } }"
        result = extract_signatures(src)
        assert "::math::add" in result.all_procs
        assert result.all_procs["::math::add"].name == "add"

    def test_nested_namespace_eval(self):
        src = "namespace eval a { namespace eval b { proc c {} {} } }"
        result = extract_signatures(src)
        assert "::a::b::c" in result.all_procs

    def test_absolute_proc_name_inside_namespace(self):
        src = "namespace eval a { proc ::foo::bar {} {} }"
        result = extract_signatures(src)
        # Absolute name (starts with ::) overrides namespace prefix
        assert "::foo::bar" in result.all_procs
        assert "::a::foo::bar" not in result.all_procs

    def test_proc_with_defaults_and_args(self):
        src = "proc f {a {b 2} args} {}"
        result = extract_signatures(src)
        pd = result.all_procs["::f"]
        assert [p.name for p in pd.params] == ["a", "b", "args"]
        assert pd.params[1].has_default
        assert pd.params[1].default_value == "2"


class TestPackageRequire:
    def test_simple_require(self):
        result = extract_signatures("package require Tcl 8.6")
        assert len(result.package_requires) == 1
        pr = result.package_requires[0]
        assert pr.name == "Tcl"
        assert pr.version == "8.6"

    def test_exact_flag(self):
        result = extract_signatures("package require -exact struct::list 1.8.5")
        assert len(result.package_requires) == 1
        pr = result.package_requires[0]
        assert pr.name == "struct::list"
        assert pr.version == "1.8.5"

    def test_require_without_version(self):
        result = extract_signatures("package require struct::list")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].version is None

    def test_other_package_subcommands_ignored(self):
        # package provide / package ifneeded must not produce require entries
        src = "package provide foo 1.0\npackage ifneeded foo 1.0 {source foo.tcl}"
        result = extract_signatures(src)
        assert result.package_requires == []


class TestSource:
    def test_literal_source(self):
        result = extract_signatures('source "lib/helpers.tcl"')
        assert len(result.source_targets) == 1
        assert result.source_targets[0].raw_path == "lib/helpers.tcl"
        assert result.source_targets[0].is_literal

    def test_substituted_source_is_not_literal(self):
        result = extract_signatures("source $dir/helpers.tcl")
        assert len(result.source_targets) == 1
        assert not result.source_targets[0].is_literal

    def test_source_with_encoding_flag(self):
        result = extract_signatures("source -encoding utf-8 foo.tcl")
        assert len(result.source_targets) == 1
        assert result.source_targets[0].raw_path == "foo.tcl"


class TestInterpAlias:
    def test_simple_alias(self):
        result = extract_signatures("interp alias {} myset {} set")
        assert "::myset" in result.command_aliases
        target, extras = result.command_aliases["::myset"]
        assert target == "set"
        assert extras == ()

    def test_alias_with_prepended_args(self):
        result = extract_signatures("interp alias {} = {} expr double")
        target, extras = result.command_aliases["::="]
        assert target == "expr"
        assert extras == ("double",)


class TestOOClass:
    def test_oo_class_create(self):
        result = extract_signatures("oo::class create Shape { method area {} {} }")
        assert "::Shape" in result.all_classes
        assert result.all_classes["::Shape"].name == "Shape"

    def test_oo_class_in_namespace(self):
        result = extract_signatures(
            "namespace eval geom { oo::class create Point { method x {} {} } }"
        )
        assert "::geom::Point" in result.all_classes


class TestItclClass:
    def test_itcl_class(self):
        result = extract_signatures("itcl::class Widget { method paint {} {} }")
        assert "::Widget" in result.all_classes


class TestConditionalGuards:
    """Procs defined under if / catch / try guards are still indexed."""

    def test_proc_in_if_then_branch(self):
        src = "if {$::tcl_version >= 9} { proc only9 {} {} }"
        result = extract_signatures(src)
        assert "::only9" in result.all_procs

    def test_proc_in_if_else_branch(self):
        src = "if {0} {} else { proc fallback {} {} }"
        result = extract_signatures(src)
        assert "::fallback" in result.all_procs

    def test_proc_in_both_branches(self):
        src = """
        if {$::tcl_platform(platform) eq "windows"} {
            proc path_sep {} { return \\; }
        } else {
            proc path_sep {} { return : }
        }
        """
        result = extract_signatures(src)
        # The second definition wins (dict overwrite), but it's still indexed.
        assert "::path_sep" in result.all_procs

    def test_proc_in_elseif_branch(self):
        src = """
        if {$a} { proc a_proc {} {} } elseif {$b} { proc b_proc {} {} } else { proc c_proc {} {} }
        """
        result = extract_signatures(src)
        for name in ("::a_proc", "::b_proc", "::c_proc"):
            assert name in result.all_procs

    def test_proc_in_explicit_then(self):
        src = "if {1} then { proc thenproc {} {} }"
        result = extract_signatures(src)
        assert "::thenproc" in result.all_procs

    def test_proc_in_catch_body(self):
        src = "catch { proc might_fail {} {} }"
        result = extract_signatures(src)
        assert "::might_fail" in result.all_procs

    def test_proc_in_try_body(self):
        src = "try { proc tried {} {} }"
        result = extract_signatures(src)
        assert "::tried" in result.all_procs

    def test_proc_in_try_handlers_and_finally(self):
        src = """
        try {
            proc main {} {}
        } on error {msg opts} {
            proc on_err {} {}
        } trap {POSIX EACCES} {msg opts} {
            proc on_eacces {} {}
        } finally {
            proc cleanup {} {}
        }
        """
        result = extract_signatures(src)
        for name in ("::main", "::on_err", "::on_eacces", "::cleanup"):
            assert name in result.all_procs

    def test_conditional_proc_respects_namespace(self):
        src = """
        namespace eval util {
            if {$::tcl_version >= 9} { proc helper {} {} }
        }
        """
        result = extract_signatures(src)
        assert "::util::helper" in result.all_procs

    def test_dynamic_body_is_skipped_not_crashed(self):
        # ``if`` with a substituted body must not make the scanner crash.
        src = "set body { proc x {} {} }\nif {1} $body"
        result = extract_signatures(src)
        # The body lived in a variable; we can't statically recurse into it.
        assert "::x" not in result.all_procs


class TestAbsoluteNamesInsideNamespace:
    def test_absolute_namespace_eval_rebases_prefix(self):
        # ``namespace eval ::foo`` must index under ``::foo``, not nest
        # under the enclosing namespace's prefix.
        src = "namespace eval a { namespace eval ::foo { proc bar {} {} } }"
        result = extract_signatures(src)
        assert "::foo::bar" in result.all_procs
        assert "::a::foo::bar" not in result.all_procs

    def test_absolute_oo_class_name_rebases(self):
        src = "namespace eval a { oo::class create ::Shape { method area {} {} } }"
        result = extract_signatures(src)
        assert "::Shape" in result.all_classes
        assert "::a::Shape" not in result.all_classes

    def test_absolute_itcl_class_name_rebases(self):
        src = "namespace eval a { itcl::class ::Widget { method paint {} {} } }"
        result = extract_signatures(src)
        assert "::Widget" in result.all_classes
        assert "::a::Widget" not in result.all_classes


class TestInterpAliasFiltering:
    def test_non_local_slave_alias_ignored(self):
        # ``interp alias slave foo {} puts`` installs ``foo`` in a child
        # interpreter and must not appear in the workspace alias table.
        result = extract_signatures("interp alias slave foo {} puts")
        assert result.command_aliases == {}

    def test_non_local_target_interp_ignored(self):
        # Target path other than ``{}`` means the alias points at a
        # command in a different interpreter; skip it for the same reason.
        result = extract_signatures("interp alias {} foo slave puts")
        assert result.command_aliases == {}

    def test_local_alias_still_recorded(self):
        result = extract_signatures("interp alias {} myset {} set")
        assert "::myset" in result.command_aliases


class TestConditionalPackageRequire:
    def test_if_body_require_marked_conditional(self):
        src = "if {$::tcl_version >= 9} { package require Tcl 9 }"
        result = extract_signatures(src)
        assert len(result.package_requires) == 1
        assert result.package_requires[0].conditional is True

    def test_catch_body_require_marked_conditional(self):
        src = "catch { package require Optional 1.0 }"
        result = extract_signatures(src)
        assert len(result.package_requires) == 1
        assert result.package_requires[0].conditional is True

    def test_try_body_and_handlers_marked_conditional(self):
        src = "try { package require A 1 } on error {msg opts} { package require B 1 }"
        result = extract_signatures(src)
        assert {pr.name: pr.conditional for pr in result.package_requires} == {
            "A": True,
            "B": True,
        }

    def test_top_level_require_not_conditional(self):
        result = extract_signatures("package require Tcl 8.6")
        assert result.package_requires[0].conditional is False


class TestCommandInvocations:
    def test_invocations_populated_top_level(self):
        result = extract_signatures("puts hello\nputs world\nset x 1\n")
        names = [inv.name for inv in result.command_invocations]
        assert names.count("puts") == 2
        assert names.count("set") == 1

    def test_invocations_populated_inside_namespace(self):
        result = extract_signatures("namespace eval util { puts hi; puts bye }")
        names = [inv.name for inv in result.command_invocations]
        assert names.count("puts") == 2
        # The ``namespace`` command itself is also an invocation.
        assert "namespace" in names

    def test_resolved_qualified_name_left_none(self):
        # Signature scan intentionally skips scope resolution; downstream
        # readers must tolerate ``resolved_qualified_name is None``.
        result = extract_signatures("puts hi")
        assert all(inv.resolved_qualified_name is None for inv in result.command_invocations)


class TestSegmenterRecovery:
    def test_unclosed_brace_does_not_swallow_later_procs(self):
        # A stray open brace in a ``proc`` body should not hide ``late``:
        # the segmenter's recovery heuristic finds the next top-level
        # command and resumes there.
        src = "proc early {} {\n    # intentionally missing close brace\n\nproc late {} {}\n"
        result = extract_signatures(src)
        assert "::late" in result.all_procs


class TestAbsenceOfHeavyFields:
    def test_no_diagnostics(self):
        # Even source that would normally produce diagnostics yields none.
        src = "proc {} {} {}\nregexp {} $x\nset"
        result = extract_signatures(src)
        assert result.diagnostics == []
        assert result.regex_patterns == []
        assert result.stub_commands == []
        assert result.stub_expr_defs == []
        assert result.all_variables == {}
        assert result.suppressed_lines == {}
