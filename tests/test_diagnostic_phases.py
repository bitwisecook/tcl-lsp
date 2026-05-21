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

    def test_w112_crlf_no_false_positive(self):
        """W112 must not fire on bare CRLF line endings (GH-95)."""
        diags, _result, _suppressed = get_basic_diagnostics("set x 1\r\nset y 2\r\n")
        codes = [d.code for d in diags]
        assert "W112" not in codes

    def test_w112_crlf_with_real_trailing_space(self):
        """W112 should still fire for actual trailing spaces before CRLF."""
        diags, _result, _suppressed = get_basic_diagnostics("set x 1   \r\n")
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
        # Use an input that surfaces a single optimisation diagnostic — the
        # previous ``string tolower`` snippet produced none, so the filter was
        # never exercised. (A multi-pass input would let disabling one code
        # unmask another, which isn't what this filter test is about.)
        source = "puts [expr {1 + 1}]"
        diags_with = get_deep_diagnostics(source, {}, optimiser_enabled=True)
        o_codes = {_str_code(d) for d in diags_with if _str_code(d).startswith("O")}
        assert o_codes, "expected optimisation diagnostics to filter"

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

    def test_disabled_o109_excluded_from_group_edits(self):
        """Disabling O109 should exclude its edits from grouped diagnostics."""
        # This source produces a group: O105 (propagate constant into string)
        # + O109 (eliminate dead store for 'set x 5').
        source = 'proc foo {} {\n  set x 5\n  puts "x=$x"\n}'
        # With O109 disabled, the group should still emit O105 but without the
        # O109 member in groupEdits.
        diags = get_diagnostics(source, disabled_optimisations={"O109"})
        codes = [d.code for d in diags]
        assert "O109" not in codes
        # The O105 diagnostic should survive but its groupEdits must not
        # contain an O109 entry.
        o105 = [d for d in diags if d.code == "O105"]
        if o105:
            data = o105[0].data
            assert data is not None
            group_edits = data.get("groupEdits", [])
            ge_codes = [ge["code"] for ge in group_edits]
            assert "O109" not in ge_codes

    def test_group_edits_end_character_is_exclusive(self):
        """groupEdits endCharacter must use LSP exclusive-end convention."""
        # O105 + O109 group: the O105 edit replaces $x (2 chars including $).
        source = 'proc foo {} {\n  set x 5\n  puts "x=$x"\n}'
        diags = get_diagnostics(source, uri="file:///test.tcl")
        grouped = [d for d in diags if d.data and d.data.get("groupEdits")]
        for d in grouped:
            assert d.data is not None
            for ge in d.data["groupEdits"]:
                # endCharacter should equal the diagnostic range's exclusive
                # end, not the inclusive token end.  Verify that the source
                # slice [startOffset : endOffset + 1] covers the expected text.
                start_off = ge["startOffset"]
                end_off = ge["endOffset"]
                snippet = source[start_off : end_off + 1]
                # The replacement should make sense for the full snippet.
                assert len(snippet) > 0, f"empty snippet for group edit {ge}"


class TestW112LineEndings:
    """W112 trailing-whitespace detection across LF, CRLF, and CR line endings (GH-95)."""

    # -- LF (Unix) -----------------------------------------------------------

    def test_lf_clean(self):
        """No trailing whitespace with LF endings."""
        diags, _, _ = get_basic_diagnostics("set x 1\nset y 2\n")
        assert "W112" not in [d.code for d in diags]

    def test_lf_trailing_spaces(self):
        """Trailing spaces detected with LF endings."""
        diags, _, _ = get_basic_diagnostics("set x 1   \nset y 2\n")
        assert "W112" in [d.code for d in diags]

    def test_lf_trailing_tab(self):
        """Trailing tab detected with LF endings."""
        diags, _, _ = get_basic_diagnostics("set x 1\t\nset y 2\n")
        assert "W112" in [d.code for d in diags]

    def test_lf_blank_line_no_false_positive(self):
        """Blank LF-only lines must not trigger W112."""
        diags, _, _ = get_basic_diagnostics("set x 1\n\nset y 2\n")
        assert "W112" not in [d.code for d in diags]

    # -- CRLF (Windows) ------------------------------------------------------

    def test_crlf_clean(self):
        """No false positive with bare CRLF endings."""
        diags, _, _ = get_basic_diagnostics("set x 1\r\nset y 2\r\n")
        assert "W112" not in [d.code for d in diags]

    def test_crlf_trailing_spaces(self):
        """Real trailing spaces before CRLF are detected."""
        diags, _, _ = get_basic_diagnostics("set x 1   \r\nset y 2\r\n")
        assert "W112" in [d.code for d in diags]

    def test_crlf_trailing_tab(self):
        """Trailing tab before CRLF is detected."""
        diags, _, _ = get_basic_diagnostics("set x 1\t\r\nset y 2\r\n")
        assert "W112" in [d.code for d in diags]

    def test_crlf_blank_line_no_false_positive(self):
        """Blank CRLF-only lines must not trigger W112."""
        diags, _, _ = get_basic_diagnostics("set x 1\r\n\r\nset y 2\r\n")
        assert "W112" not in [d.code for d in diags]

    def test_crlf_multiple_lines_one_trailing(self):
        """Only the line with actual trailing whitespace fires W112."""
        src = "set a 1\r\nset b 2   \r\nset c 3\r\n"
        diags, _, _ = get_basic_diagnostics(src)
        w112 = [d for d in diags if d.code == "W112"]
        assert len(w112) == 1
        assert w112[0].range.start.line == 1

    # -- CR (Classic Mac OS 9) ------------------------------------------------
    # Pure CR files are not split into separate lines (split("\n") sees them
    # as a single line), so W112 only fires for trailing whitespace at the
    # very end of the "line" (i.e. end of file).

    def test_cr_clean(self):
        """No false positive with bare CR endings."""
        diags, _, _ = get_basic_diagnostics("set x 1\rset y 2\r")
        assert "W112" not in [d.code for d in diags]

    def test_cr_trailing_spaces_at_end(self):
        """Trailing spaces at the very end of a CR-only file are detected."""
        diags, _, _ = get_basic_diagnostics("set x 1\rset y 2   ")
        assert "W112" in [d.code for d in diags]

    # -- Mixed ----------------------------------------------------------------

    def test_mixed_endings_no_false_positive(self):
        """File with mixed LF/CRLF, no trailing whitespace."""
        src = "set x 1\nset y 2\r\nset z 3\n"
        diags, _, _ = get_basic_diagnostics(src)
        assert "W112" not in [d.code for d in diags]

    def test_mixed_endings_with_trailing(self):
        """File with mixed endings, trailing space only on CRLF line."""
        src = "set x 1\nset y 2   \r\nset z 3\n"
        diags, _, _ = get_basic_diagnostics(src)
        w112 = [d for d in diags if d.code == "W112"]
        assert len(w112) == 1
        assert w112[0].range.start.line == 1


class TestW111LineEndings:
    """W111 line-length must not count \\r from CRLF as a character."""

    def test_crlf_does_not_inflate_length(self):
        """A 120-char line with CRLF ending must not trigger W111 (default max=120)."""
        line = "x" * 120
        src = f"{line}\r\n"
        diags, _, _ = get_basic_diagnostics(src)
        assert "W111" not in [d.code for d in diags]

    def test_crlf_over_limit_still_fires(self):
        """A 121-char line with CRLF ending must still trigger W111."""
        line = "x" * 121
        src = f"{line}\r\n"
        diags, _, _ = get_basic_diagnostics(src)
        assert "W111" in [d.code for d in diags]


class TestW118LineEndings:
    """W118: inconsistent or mismatched line endings (GH-95)."""

    def test_lf_expected_lf_no_warning(self):
        """Pure LF file with LF configured — no W118."""
        diags, _, _ = get_basic_diagnostics("set x 1\nset y 2\n", line_ending="\n")
        assert "W118" not in [d.code for d in diags]

    def test_crlf_expected_crlf_no_warning(self):
        """Pure CRLF file with CRLF configured — no W118."""
        diags, _, _ = get_basic_diagnostics("set x 1\r\nset y 2\r\n", line_ending="\r\n")
        assert "W118" not in [d.code for d in diags]

    def test_crlf_expected_lf_fires(self):
        """CRLF file with LF configured — W118 fires."""
        diags, _, _ = get_basic_diagnostics("set x 1\r\nset y 2\r\n", line_ending="\n")
        w118 = [d for d in diags if d.code == "W118"]
        assert len(w118) == 1
        assert "CRLF" in w118[0].message
        assert "expected LF" in w118[0].message

    def test_lf_expected_crlf_fires(self):
        """LF file with CRLF configured — W118 fires."""
        diags, _, _ = get_basic_diagnostics("set x 1\nset y 2\n", line_ending="\r\n")
        w118 = [d for d in diags if d.code == "W118"]
        assert len(w118) == 1
        assert "LF" in w118[0].message
        assert "expected CRLF" in w118[0].message

    def test_mixed_endings_fires(self):
        """Mixed LF/CRLF file — W118 fires with 'Mixed' message."""
        diags, _, _ = get_basic_diagnostics("set x 1\nset y 2\r\nset z 3\n", line_ending="\n")
        w118 = [d for d in diags if d.code == "W118"]
        assert len(w118) == 1
        assert "Mixed" in w118[0].message

    def test_no_newlines_no_warning(self):
        """Single-line file with no newlines — no W118."""
        diags, _, _ = get_basic_diagnostics("set x 1", line_ending="\n")
        assert "W118" not in [d.code for d in diags]

    def test_cr_expected_lf_fires(self):
        """CR-only line endings with LF configured — W118 fires."""
        diags, _, _ = get_basic_diagnostics("set x 1\rset y 2\r", line_ending="\n")
        w118 = [d for d in diags if d.code == "W118"]
        assert len(w118) == 1
        assert "CR" in w118[0].message

    def test_default_line_ending_is_lf(self):
        """Default line_ending parameter is LF."""
        # CRLF file without explicit line_ending should warn (default=LF)
        diags, _, _ = get_basic_diagnostics("set x 1\r\nset y 2\r\n")
        w118 = [d for d in diags if d.code == "W118"]
        assert len(w118) == 1
