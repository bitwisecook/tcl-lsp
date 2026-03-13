"""Tests for the diagnostics feature (LSP-level output)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.diagnostics import get_diagnostics


class TestLSPDiagnostics:
    def test_returns_list(self):
        result = get_diagnostics("set x 42")
        assert isinstance(result, list)

    def test_clean_file_no_diagnostics(self):
        result = get_diagnostics("set x [clock seconds]\nputs $x")
        assert len(result) == 0

    def test_arity_error_produces_diagnostic(self):
        result = get_diagnostics("set")
        assert len(result) >= 1
        d = result[0]
        assert isinstance(d, types.Diagnostic)
        assert d.severity == types.DiagnosticSeverity.Error
        assert d.source == "tcl-lsp"

    def test_diagnostic_has_range(self):
        result = get_diagnostics("break extra")
        assert len(result) >= 1
        d = result[0]
        assert d.range.start.line == 0
        assert d.range.start.character == 0

    def test_warning_severity(self):
        result = get_diagnostics("string bogus hello")
        warnings = [d for d in result if d.severity == types.DiagnosticSeverity.Warning]
        assert len(warnings) >= 1

    def test_hint_severity(self):
        result = get_diagnostics("proc foo {} { set x 1 }")
        hints = [d for d in result if d.severity == types.DiagnosticSeverity.Hint]
        assert len(hints) >= 1

    def test_multiple_errors(self):
        source = "set\nbreak extra\nwhile {1}"
        result = get_diagnostics(source)
        assert len(result) >= 3

    def test_unknown_command_no_diagnostic(self):
        result = get_diagnostics("mycommand arg1 arg2")
        assert len(result) == 0

    def test_w100_is_paired_with_optimisation_hint(self):
        result = get_diagnostics("expr $x + 1")
        codes = [d.code for d in result]
        assert "W100" in codes
        assert "O111" in codes
        o111 = [d for d in result if d.code == "O111"]
        assert len(o111) == 1
        assert o111[0].severity == types.DiagnosticSeverity.Information

    def test_o111_can_be_disabled(self):
        result = get_diagnostics(
            "expr $x + 1",
            disabled_optimisations={"O111"},
        )
        codes = [d.code for d in result]
        assert "W100" in codes
        assert "O111" not in codes

    def test_o111_range_matches_full_w100_expression_span(self):
        source = "set n [expr $n << 5]"
        result = get_diagnostics(source)
        w100 = [d for d in result if d.code == "W100"]
        o111 = [d for d in result if d.code == "O111"]
        assert len(w100) == 1
        assert len(o111) == 1
        assert w100[0].range == o111[0].range
        span = source[w100[0].range.start.character : w100[0].range.end.character]
        assert span == "$n << 5"


class TestW115CommentContinuation:
    """W115: Backslash-newline continuation in comments."""

    def test_simple_continuation_flagged(self):
        source = "# hello \\\nworld\nputs hi"
        result = get_diagnostics(source)
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 1
        assert w115[0].severity == types.DiagnosticSeverity.Warning
        assert w115[0].range.start.line == 0
        assert w115[0].range.end.line == 1

    def test_chained_continuation_single_diagnostic(self):
        source = "# line1 \\\nline2 \\\nline3\nputs hi"
        result = get_diagnostics(source)
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 1
        assert w115[0].range.start.line == 0
        assert w115[0].range.end.line == 2

    def test_no_continuation_no_diagnostic(self):
        source = "# normal comment\nputs hi"
        result = get_diagnostics(source)
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 0

    def test_w115_can_be_disabled(self):
        source = "# hello \\\nworld"
        result = get_diagnostics(source, disabled_diagnostics={"W115"})
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 0

    def test_continuation_with_already_commented_line(self):
        """Continuation line that already has # should keep it."""
        source = "# line1 \\\n# line2\nputs hi"
        result = get_diagnostics(source)
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 1

    def test_indented_comment_continuation(self):
        source = "    # hello \\\n    world\nputs hi"
        result = get_diagnostics(source)
        w115 = [d for d in result if d.code == "W115"]
        assert len(w115) == 1


_FIXTURE_DIR = Path(__file__).parent / "fixtures"


def _load_fixture(relative_path: str) -> str:
    return (_FIXTURE_DIR / relative_path).read_text(encoding="utf-8")


class TestVariableShapeDiagnostics:
    """Variable shape diagnostics should preserve expected names."""

    def test_namespaced_scalar_reports_qualified_name(self):
        source = _load_fixture("shimmer/namespace_scalar_vs_list.tcl")
        result = get_diagnostics(source)
        s100 = [d for d in result if d.code == "S100"]
        assert len(s100) == 1
        assert "$::demo::payload" in s100[0].message

    def test_namespaced_array_reports_base_array_name(self):
        source = _load_fixture("shimmer/namespace_array_vs_list.tcl")
        result = get_diagnostics(source)
        s100 = [d for d in result if d.code == "S100"]
        assert len(s100) == 1
        assert "$::demo::arr" in s100[0].message

    def test_braced_scalar_like_array_name_treated_as_scalar_name(self):
        source = _load_fixture("variable_shapes/braced_scalar_like_array_name.tcl")
        result = get_diagnostics(source)
        s100 = [d for d in result if d.code == "S100"]
        assert len(s100) == 1
        assert "$a" in s100[0].message

    def test_unbraced_array_ref_treated_as_base_array_name(self):
        source = _load_fixture("variable_shapes/unbraced_array_ref.tcl")
        result = get_diagnostics(source)
        s100 = [d for d in result if d.code == "S100"]
        assert len(s100) == 1
        assert "$a" in s100[0].message

    def test_namespace_var_commands_do_not_emit_shape_diagnostics(self):
        source = _load_fixture("variable_shapes/namespace_var_commands.tcl")
        result = get_diagnostics(source)
        shape_codes = {"S100", "S101", "S102", "W200", "W201", "W210", "W211"}
        assert not any(d.code in shape_codes for d in result)


class TestMalformedNestedSubstitutionDiagnostics:
    """Malformed nested substitutions should produce stable parser diagnostics."""

    def test_missing_close_bracket_with_array_ref_in_quotes_reports_e201(self):
        result = get_diagnostics('puts "value [set a(1)"')
        e201 = [d for d in result if d.code == "E201"]
        assert len(e201) == 1
        assert "missing close-bracket" in e201[0].message

    def test_missing_close_bracket_with_namespaced_array_ref_reports_e201(self):
        result = get_diagnostics('puts "value [set ::ns::arr(k)"')
        e201 = [d for d in result if d.code == "E201"]
        assert len(e201) == 1
        assert "missing close-bracket" in e201[0].message

    def test_braced_scalar_like_array_name_in_braces_has_no_parse_error(self):
        result = get_diagnostics("puts {${a(1)} [set x}")
        assert not any(d.code and str(d.code).startswith("E") for d in result)

    def test_unbraced_array_ref_in_braces_has_no_parse_error(self):
        result = get_diagnostics("puts {$a(1) [set x}")
        assert not any(d.code and str(d.code).startswith("E") for d in result)
