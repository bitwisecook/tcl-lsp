"""Tests for namespace-import rewriting and extraction."""

from __future__ import annotations

from analyser import analyse
from analyser.auto_path_eval import evaluate_auto_path_expr
from analyser.namespace_imports import rewrite_via_imports
from analyser.semantic_model import NamespaceImport, Range
from analyser.signature_scan import extract_signatures
from shared.tokens import SourcePosition


def _zero_range() -> Range:
    p = SourcePosition(line=0, character=0, offset=0)
    return Range(start=p, end=p)


# rewrite_via_imports


class TestRewriteViaImports:
    def test_glob_import_rewrites_qualified_ref(self):
        imports = [
            NamespaceImport(
                ns="::vt",
                pattern="::term::ansi::send::*",
                range=_zero_range(),
            )
        ]
        assert rewrite_via_imports("vt::showat", imports) == ["::term::ansi::send::showat"]
        # Explicit ``::`` prefix on the source is accepted too.
        assert rewrite_via_imports("::vt::showat", imports) == ["::term::ansi::send::showat"]

    def test_global_import_rewrites_bare_name(self):
        imports = [
            NamespaceImport(
                ns="::",
                pattern="::foo::bar::*",
                range=_zero_range(),
            )
        ]
        assert rewrite_via_imports("baz", imports) == ["::foo::bar::baz"]

    def test_specific_import_matches_only_its_tail(self):
        imports = [
            NamespaceImport(
                ns="::my",
                pattern="::other::baz",
                range=_zero_range(),
            )
        ]
        assert rewrite_via_imports("my::baz", imports) == ["::other::baz"]
        assert rewrite_via_imports("my::quux", imports) == []

    def test_mismatched_namespace_returns_nothing(self):
        imports = [
            NamespaceImport(
                ns="::vt",
                pattern="::term::ansi::send::*",
                range=_zero_range(),
            )
        ]
        # Importing namespace is ``::vt``, reference is ``other::showat``.
        assert rewrite_via_imports("other::showat", imports) == []

    def test_conjectured_sorts_after_explicit(self):
        imports = [
            NamespaceImport(
                ns="::vt",
                pattern="::tcllib::send::*",
                range=_zero_range(),
                conjectured=True,
            ),
            NamespaceImport(
                ns="::vt",
                pattern="::explicit::send::*",
                range=_zero_range(),
            ),
        ]
        assert rewrite_via_imports("vt::showat", imports) == [
            "::explicit::send::showat",
            "::tcllib::send::showat",
        ]


# evaluate_auto_path_expr


class TestEvaluateAutoPathExpr:
    def test_literal_path(self):
        assert evaluate_auto_path_expr("/abs/lib", "/tmp/foo.tcl") == "/abs/lib"

    def test_file_dirname_info_script(self):
        assert (
            evaluate_auto_path_expr("[file dirname [info script]]", "/tmp/foo/bar.tcl")
            == "/tmp/foo"
        )

    def test_tcllib_idiom(self):
        raw = "[file join [file dirname [file dirname [file dirname [info script]]]] modules]"
        got = evaluate_auto_path_expr(raw, "/tmp/tcllib-2.0/examples/term/menu")
        assert got == "/tmp/tcllib-2.0/modules"

    def test_variable_substitution_is_refused(self):
        assert evaluate_auto_path_expr("$env(PATH)", "/tmp/foo.tcl") is None

    def test_unknown_command_is_refused(self):
        assert evaluate_auto_path_expr("[glob *.tcl]", "/tmp/foo.tcl") is None


# namespace import extraction


class TestExtractNamespaceImports:
    def test_direct_namespace_import_at_top_level(self):
        src = "namespace import ::foo::bar::*\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        imp = result.namespace_imports[0]
        assert imp.ns == "::"
        assert imp.pattern == "::foo::bar::*"
        assert not imp.conjectured

    def test_namespace_import_inside_eval_block(self):
        src = "namespace eval my { namespace import ::other::baz }\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        imp = result.namespace_imports[0]
        assert imp.ns == "::my"
        assert imp.pattern == "::other::baz"

    def test_tcllib_import_wrapper_is_conjectured(self):
        src = "term::ansi::send::import vt\n"
        result = extract_signatures(src)
        imports = [i for i in result.namespace_imports if i.conjectured]
        assert len(imports) == 1
        imp = imports[0]
        assert imp.ns == "::vt"
        assert imp.pattern == "::term::ansi::send::*"

    def test_unqualified_pattern_is_ignored(self):
        # ``namespace import foo`` without leading ``::`` can't be resolved.
        src = "namespace import foo\n"
        result = extract_signatures(src)
        assert result.namespace_imports == []

    def test_relative_pattern_resolves_against_current_namespace(self):
        # Tcl resolves ``bar::*`` inside ``namespace eval foo`` as
        # ``::foo::bar::*``, not ``::bar::*``.  The LSP must track the
        # same target, otherwise cross-file lookup points at the wrong
        # source namespace.
        src = "namespace eval foo { namespace import bar::baz }\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        imp = result.namespace_imports[0]
        assert imp.ns == "::foo"
        assert imp.pattern == "::foo::bar::baz"

    def test_relative_glob_pattern_resolves_against_current_namespace(self):
        src = "namespace eval outer { namespace import inner::* }\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        assert result.namespace_imports[0].pattern == "::outer::inner::*"

    def test_relative_pattern_at_global_still_absolute(self):
        # At the global level ``foo::bar`` resolves to ``::foo::bar``.
        src = "namespace import foo::bar\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        assert result.namespace_imports[0].pattern == "::foo::bar"

    def test_substituted_pattern_is_ignored(self):
        # ``namespace import $var::*`` can't be statically resolved.
        src = "namespace import $mod::*\n"
        result = extract_signatures(src)
        assert result.namespace_imports == []

    def test_nested_namespace_eval_uses_full_qualified_path(self):
        # ``namespace eval outer { namespace eval inner { namespace
        # import … } }`` records the import under ``::outer::inner``,
        # not ``::inner``.
        src = "namespace eval outer { namespace eval inner { namespace import ::foo::* } }\n"
        result = extract_signatures(src)
        assert len(result.namespace_imports) == 1
        assert result.namespace_imports[0].ns == "::outer::inner"

    def test_import_wrapper_alias_relative_to_current_namespace(self):
        # tcllib ``X::import alias`` inside ``namespace eval outer``
        # creates ``::outer::alias``, not ``::alias``.
        src = "namespace eval outer { term::ansi::send::import vt }\n"
        result = extract_signatures(src)
        conj = [i for i in result.namespace_imports if i.conjectured]
        assert len(conj) == 1
        assert conj[0].ns == "::outer::vt"
        assert conj[0].pattern == "::term::ansi::send::*"

    def test_auto_path_entry_captured(self):
        src = "lappend auto_path [file dirname [info script]] /abs/lib\n"
        result = extract_signatures(src)
        assert len(result.auto_path_entries) == 2
        # Raw text is preserved verbatim so callers can static-evaluate.
        raws = [e.raw for e in result.auto_path_entries]
        assert "[file dirname [info script]]" in raws
        assert "/abs/lib" in raws


# factory-wrapper detection (DEFC-style)


class TestFactoryWrapperDetection:
    def test_defc_factory_emits_synthetic_proc(self):
        src = """
namespace eval ::x {}
proc ::x::DEFC {name arguments script} {
    proc $name $arguments $script
}
proc ::x::INIT {} {
    DEFC showat {r c text} {
        return "$r $c $text"
    }
}
"""
        result = extract_signatures(src)
        assert "::x::showat" in result.all_procs
        # Non-factory procs still indexed.
        assert "::x::DEFC" in result.all_procs
        assert "::x::INIT" in result.all_procs

    def test_non_factory_wrapper_does_not_emit(self):
        # A wrapper whose body doesn't call ``proc $p1 $p2 $p3`` must not
        # turn every 3-arg call into a synthetic proc.
        src = """
namespace eval ::x {}
proc ::x::frob {a b c} {
    return "$a-$b-$c"
}
proc ::x::INIT {} {
    frob showat {r c text} {
        return "noop"
    }
}
"""
        result = extract_signatures(src)
        assert "::x::showat" not in result.all_procs

    def test_factory_call_with_dollar_braces_in_body(self):
        # tcllib's ``DEFC`` body uses ``proc $name $arguments $script``.
        src = """
proc ::ns::DEFC {name arguments script} {
    variable ctrl
    proc ${name} ${arguments} ${script}
    return
}
proc ::ns::INIT {} {
    DEFC foo {a b} { return "$a-$b" }
}
"""
        result = extract_signatures(src)
        assert "::ns::foo" in result.all_procs

    def test_factory_requires_three_params(self):
        # A wrapper with only 2 params can't map to ``proc NAME ARGS BODY``.
        src = """
proc ::ns::two_arg {a b} {
    proc $a $b ""
}
proc ::ns::INIT {} {
    two_arg foo {r c}
}
"""
        result = extract_signatures(src)
        assert "::ns::foo" not in result.all_procs

    def test_two_namespaces_with_identical_factory_do_not_cross(self):
        # Two unrelated namespaces each define their own ``DEFC``
        # wrapper.  An unqualified ``DEFC`` call inside ``::b`` must
        # bind to ``::b::DEFC``, emitting the synthetic proc in
        # ``::b`` — never to ``::a::DEFC``.  The call inside ``::a``
        # stays in ``::a``.
        src = """
proc ::a::DEFC {name arguments script} {
    proc $name $arguments $script
}
proc ::b::DEFC {name arguments script} {
    proc $name $arguments $script
}
proc ::a::INIT {} {
    DEFC from_a {r} { return $r }
}
proc ::b::INIT {} {
    DEFC from_b {r} { return $r }
}
"""
        result = extract_signatures(src)
        assert "::a::from_a" in result.all_procs
        assert "::b::from_b" in result.all_procs
        # Cross-namespace leakage is the bug — assert both ways.
        assert "::a::from_b" not in result.all_procs
        assert "::b::from_a" not in result.all_procs

    def test_factory_without_matching_namespace_is_skipped(self):
        # A factory in ``::a`` and a call in ``::b`` (where ``::b``
        # has no factory) must NOT bind — better to emit nothing than
        # to bind into the wrong namespace.
        src = """
proc ::a::DEFC {name arguments script} {
    proc $name $arguments $script
}
proc ::b::INIT {} {
    DEFC stray {r} { return $r }
}
"""
        result = extract_signatures(src)
        assert "::a::stray" not in result.all_procs
        assert "::b::stray" not in result.all_procs

    def test_global_factory_matches_global_call(self):
        # A factory defined at the global level matches bare calls at
        # the global level.
        src = """
proc ::DEFC {name arguments script} {
    proc $name $arguments $script
}
DEFC global_proc {r} { return $r }
"""
        result = extract_signatures(src)
        assert "::global_proc" in result.all_procs


# full analyser picks up namespace imports


class TestAnalyserNamespaceImports:
    def test_direct_import_tracked(self):
        src = "namespace eval my { namespace import ::foo::bar::* }\n"
        result = analyse(src)
        patterns = {(i.ns, i.pattern) for i in result.namespace_imports}
        assert ("::my", "::foo::bar::*") in patterns

    def test_tcllib_import_wrapper_conjectured(self):
        src = "term::ansi::send::import vt\n"
        result = analyse(src)
        conj = [i for i in result.namespace_imports if i.conjectured]
        assert any(i.ns == "::vt" and i.pattern == "::term::ansi::send::*" for i in conj)

    def test_auto_path_entries_captured(self):
        src = "lappend auto_path /tmp/lib\n"
        result = analyse(src)
        assert any(e.raw == "/tmp/lib" for e in result.auto_path_entries)
