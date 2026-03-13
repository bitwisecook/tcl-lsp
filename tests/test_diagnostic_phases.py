"""Tests for the phased diagnostic pipeline (basic vs deep splitting)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.diagnostics import get_basic_diagnostics, get_deep_diagnostics, get_diagnostics


def _str_code(d: types.Diagnostic) -> str:
    """Extract the diagnostic code as a string (safe for ``startswith``)."""
    return str(d.code) if d.code is not None else ""


class TestBasicDiagnostics:
    """get_basic_diagnostics returns analysis warnings + style checks."""

    def test_returns_tuple(self):
        diags, result, suppressed = get_basic_diagnostics("set x 42")
        assert isinstance(diags, list)
        assert result is not None
        assert isinstance(suppressed, dict)

    def test_clean_source_no_diagnostics(self):
        diags, _result, _suppressed = get_basic_diagnostics("set x [clock seconds]\nputs $x")
        assert len(diags) == 0

    def test_arity_error_in_basic(self):
        """Syntax/arity errors should appear in the basic phase."""
        diags, _result, _suppressed = get_basic_diagnostics("set")
        errors = [d for d in diags if d.severity == types.DiagnosticSeverity.Error]
        assert len(errors) >= 1

    def test_w100_in_basic(self):
        """W100 (unbraced expr) is an analysis diagnostic, so it's basic."""
        diags, _result, _suppressed = get_basic_diagnostics("expr $x + 1")
        codes = [d.code for d in diags]
        assert "W100" in codes

    def test_o111_brace_hint_in_basic(self):
        """O111 (brace expression hint) is paired with W100 in basic phase."""
        diags, _result, _suppressed = get_basic_diagnostics("expr $x + 1")
        codes = [d.code for d in diags]
        assert "O111" in codes

    def test_o111_disabled(self):
        """O111 can be suppressed via disabled_optimisations."""
        diags, _result, _suppressed = get_basic_diagnostics(
            "expr $x + 1",
            disabled_optimisations={"O111"},
        )
        codes = [d.code for d in diags]
        assert "W100" in codes
        assert "O111" not in codes

    def test_w111_line_length(self):
        """W111 (line length) is a style check in the basic phase."""
        long_line = "set x " + "a" * 200
        diags, _result, _suppressed = get_basic_diagnostics(long_line, line_length=120)
        codes = [d.code for d in diags]
        assert "W111" in codes

    def test_w112_trailing_whitespace(self):
        """W112 (trailing whitespace) is a style check in the basic phase."""
        diags, _result, _suppressed = get_basic_diagnostics("set x 1   \n")
        codes = [d.code for d in diags]
        assert "W112" in codes

    def test_w115_comment_continuation(self):
        """W115 (backslash-newline in comment) is a style check in the basic phase."""
        diags, _result, _suppressed = get_basic_diagnostics("# hello \\\nworld\nputs hi")
        codes = [d.code for d in diags]
        assert "W115" in codes

    def test_unused_variable_in_basic(self):
        """W211 (unused variable) comes from analysis, so it's basic."""
        diags, _result, _suppressed = get_basic_diagnostics("proc foo {} { set x 1 }")
        codes = [d.code for d in diags]
        assert "W211" in codes

    def test_no_optimiser_codes_in_basic(self):
        """Deep optimiser codes (O100-O110) should NOT appear in basic phase."""
        source = "proc foo {x} {\n  set y [expr {$x + 1}]\n  set y [expr {$x + 1}]\n  return $y\n}"
        diags, _result, _suppressed = get_basic_diagnostics(source)
        deep_codes = {
            "O100",
            "O101",
            "O102",
            "O103",
            "O104",
            "O105",
            "O106",
            "O107",
            "O108",
            "O109",
            "O110",
        }
        found = {d.code for d in diags} & deep_codes
        assert not found, f"Deep optimiser codes in basic phase: {found}"

    def test_no_shimmer_codes_in_basic(self):
        """Shimmer codes (S1xx) should NOT appear in basic phase."""
        source = "set x [llength $list]\nif {$x > 0} {\n  set y [lindex $list 0]\n}\n"
        diags, _result, _suppressed = get_basic_diagnostics(source)
        shimmer = [d for d in diags if _str_code(d).startswith("S")]
        assert len(shimmer) == 0

    def test_no_taint_codes_in_basic(self):
        """Taint codes (T1xx) should NOT appear in basic phase."""
        diags, _result, _suppressed = get_basic_diagnostics("set x 1")
        taint = [d for d in diags if _str_code(d).startswith("T")]
        assert len(taint) == 0

    def test_suppressed_lines_returned(self):
        """The suppressed_lines dict is returned for use by deep pass."""
        source = "set x 1  ;# noqa: W211"
        _diags, _result, suppressed = get_basic_diagnostics(source)
        # suppressed is the raw dict from analysis, may be empty or populated
        assert isinstance(suppressed, dict)

    def test_disabled_diagnostics(self):
        """Codes in disabled_diagnostics should be filtered out."""
        diags, _result, _suppressed = get_basic_diagnostics(
            "set",
            disabled_diagnostics={"E001"},
        )
        codes = [d.code for d in diags]
        assert "E001" not in codes


class TestDeepDiagnostics:
    """get_deep_diagnostics returns optimiser, shimmer, taint, GVN, iRules flow."""

    def test_returns_list(self):
        diags = get_deep_diagnostics("set x 42", {})
        assert isinstance(diags, list)

    def test_optimiser_in_deep(self):
        """Optimiser suggestions (O1xx) appear in the deep phase."""
        source = "set x [expr {1 + 2}]"
        diags = get_deep_diagnostics(source, {}, optimiser_enabled=True)
        # O102 is constant expression folding
        optimiser = [d for d in diags if _str_code(d).startswith("O")]
        assert len(optimiser) >= 1

    def test_optimiser_disabled(self):
        """When optimiser_enabled=False, no O1xx codes should appear."""
        source = 'set x [string tolower "HELLO"]'
        diags = get_deep_diagnostics(source, {}, optimiser_enabled=False)
        optimiser = [d for d in diags if _str_code(d).startswith("O")]
        assert len(optimiser) == 0

    def test_shimmer_disabled(self):
        """When shimmer_enabled=False, no S1xx codes should appear."""
        diags = get_deep_diagnostics("set x 1", {}, shimmer_enabled=False)
        shimmer = [d for d in diags if _str_code(d).startswith("S")]
        assert len(shimmer) == 0

    def test_taint_disabled(self):
        """When taint_enabled=False, no T1xx codes should appear."""
        diags = get_deep_diagnostics("set x 1", {}, taint_enabled=False)
        taint = [d for d in diags if _str_code(d).startswith("T")]
        assert len(taint) == 0

    def test_no_analysis_codes_in_deep(self):
        """Analysis-level codes (W1xx, E0xx, H3xx) should NOT appear in deep phase."""
        diags = get_deep_diagnostics("set", {})
        analysis_codes = [d for d in diags if _str_code(d).startswith(("E0", "W1", "H3"))]
        assert len(analysis_codes) == 0

    def test_suppressed_lines_respected(self):
        """Diagnostics suppressed by noqa should not appear even in deep pass."""
        source = 'set x [string tolower "HELLO"]'
        # Suppress line 0 for all codes
        suppressed: dict[int, frozenset[str]] = {0: frozenset({"*"})}
        diags = get_deep_diagnostics(source, suppressed, optimiser_enabled=True)
        assert len(diags) == 0

    def test_disabled_optimisations(self):
        """Per-code optimisation filters work in deep phase."""
        source = 'set x [string tolower "HELLO"]'
        diags_with = get_deep_diagnostics(source, {}, optimiser_enabled=True)
        o_codes = {_str_code(d) for d in diags_with if _str_code(d).startswith("O")}

        if o_codes:
            # Disable all found codes and verify they disappear
            diags_without = get_deep_diagnostics(
                source, {}, optimiser_enabled=True, disabled_optimisations=o_codes
            )
            remaining = {_str_code(d) for d in diags_without if _str_code(d).startswith("O")}
            assert remaining == set()


class TestCombinedGetDiagnostics:
    """get_diagnostics (the combined wrapper) should equal basic + deep."""

    def test_combined_equals_basic_plus_deep(self):
        """get_diagnostics should return the same diagnostics as basic + deep."""
        source = "expr $x + 1"
        combined = get_diagnostics(source)

        basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(source, suppressed)

        combined_codes = sorted(d.code or "" for d in combined)
        split_codes = sorted(d.code or "" for d in basic + deep)
        assert combined_codes == split_codes

    def test_combined_clean_source(self):
        source = "set x [clock seconds]\nputs $x"
        combined = get_diagnostics(source)
        basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(source, suppressed)
        assert len(combined) == len(basic) + len(deep)

    def test_combined_with_errors(self):
        source = "set"
        combined = get_diagnostics(source)
        basic, _result, suppressed = get_basic_diagnostics(source)
        deep = get_deep_diagnostics(source, suppressed)
        assert len(combined) == len(basic) + len(deep)

    def test_combined_passes_uri_to_deep(self):
        """URI should propagate to deep diagnostics for related-info locations."""
        source = 'set x [string tolower "HELLO"]'
        combined_with_uri = get_diagnostics(source, uri="file:///test.tcl")
        combined_without = get_diagnostics(source)
        # Both should produce the same set of codes
        codes_with = sorted(d.code or "" for d in combined_with_uri)
        codes_without = sorted(d.code or "" for d in combined_without)
        assert codes_with == codes_without

    def test_combined_with_disabled_codes(self):
        """Disabled diagnostics should be filtered from both phases."""
        source = "expr $x + 1"
        combined = get_diagnostics(
            source,
            disabled_diagnostics={"W100"},
            disabled_optimisations={"O111"},
        )
        codes = [d.code for d in combined]
        assert "W100" not in codes
        assert "O111" not in codes
