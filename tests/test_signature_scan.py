"""Tests for the lightweight signature-only scan."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.signature_scan import extract_signatures


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

    def test_no_command_invocations(self):
        # Signature scan intentionally skips invocation collection.
        result = extract_signatures("puts hello\nmyproc arg1 arg2")
        assert result.command_invocations == []
