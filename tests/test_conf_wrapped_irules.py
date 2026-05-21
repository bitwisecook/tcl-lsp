"""Tests for conf-wrapped iRules analysis (``ltm rule`` / ``gtm rule`` mode)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.conf_wrapped import (
    _shift_position,
    _shift_range,
    analyse_conf_wrapped,
)
from core.analysis.semantic_model import Range
from core.bigip.rule_extract import find_embedded_rules, is_conf_wrapped_irules
from core.commands.registry.runtime import configure_signatures
from core.common.dialect import detect_dialect_from_source
from core.parsing.tokens import SourcePosition
from lsp.features.document_symbols import get_document_symbols
from lsp.features.irules_context import find_enclosing_when_event

# -- Detection ----------------------------------------------------------------


class TestDetection:
    """is_conf_wrapped_irules and detect_dialect_from_source."""

    def test_standalone_irule_not_detected(self):
        src = 'when HTTP_REQUEST {\n    log local0. "hi"\n}'
        assert not is_conf_wrapped_irules(src)

    def test_ltm_rule_detected(self):
        src = 'ltm rule /Common/foo {\n    when HTTP_REQUEST {\n        log local0. "hi"\n    }\n}'
        assert is_conf_wrapped_irules(src)

    def test_gtm_rule_detected(self):
        src = 'gtm rule /Common/bar {\n    when DNS_REQUEST {\n        log local0. "hi"\n    }\n}'
        assert is_conf_wrapped_irules(src)

    def test_multiple_rules_detected(self):
        src = (
            "ltm rule /Common/a {\n    when RULE_INIT { }\n}\n"
            "ltm rule /Common/b {\n    when HTTP_REQUEST { }\n}\n"
        )
        assert is_conf_wrapped_irules(src)

    def test_detect_dialect_returns_irules(self):
        src = "ltm rule /Common/foo {\n    when HTTP_REQUEST { }\n}"
        assert detect_dialect_from_source(src) == "f5-irules"

    def test_dialect_directive_takes_priority(self):
        """Explicit ``# tcl-dialect:`` overrides conf-wrapped detection."""
        src = "# tcl-dialect: tcl8.6\nltm rule /Common/foo {\n    when HTTP_REQUEST { }\n}"
        assert detect_dialect_from_source(src) == "tcl8.6"


# -- Analysis -----------------------------------------------------------------


class TestAnalysis:
    """analyse_conf_wrapped produces correct merged results."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_single_rule_diagnostics(self):
        src = (
            "ltm rule /Common/test {\n"
            "    when HTTP_REQUEST {\n"
            '        log local0. "hello"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 1
        assert rules[0].name == "test"
        # Should produce no IRULE5006/5007 errors — when is at top-level
        # within the rule body.
        depth_diags = [d for d in result.diagnostics if d.code in ("IRULE5006", "IRULE5007")]
        assert len(depth_diags) == 0

    def test_multiple_rules_all_analysed(self):
        src = (
            "ltm rule /Common/rule_a {\n"
            "    when RULE_INIT {\n"
            "        set static::app_debug 1\n"
            "    }\n"
            "}\n"
            "ltm rule /Common/rule_b {\n"
            "    when HTTP_REQUEST {\n"
            '        log local0. "request"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 2
        assert rules[0].name == "rule_a"
        assert rules[1].name == "rule_b"

    def test_gtm_rule_analysed(self):
        src = (
            "gtm rule /Common/dns_handler {\n"
            "    when DNS_REQUEST {\n"
            '        log local0. "dns"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 1
        assert rules[0].name == "dns_handler"

    def test_when_not_flagged_as_nested(self):
        """``when`` at rule body top-level should NOT trigger IRULE5006."""
        src = (
            "ltm rule /Common/test {\n"
            "    when HTTP_REQUEST {\n"
            '        HTTP::respond 200 content "ok"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        irule5006 = [d for d in result.diagnostics if d.code == "IRULE5006"]
        assert len(irule5006) == 0

    def test_irule5007_fires_for_top_level_event_command(self):
        """Event-context command outside ``when`` triggers IRULE5007."""
        src = 'ltm rule /Common/bad {\n    HTTP::respond 200 content "ok"\n}\n'
        result, rules = analyse_conf_wrapped(src)
        irule5007 = [d for d in result.diagnostics if d.code == "IRULE5007"]
        assert len(irule5007) == 1
        assert "HTTP::respond" in irule5007[0].message

    def test_multiple_when_blocks_in_one_rule(self):
        """Multiple ``when`` blocks in a single rule are all analysed."""
        src = (
            "ltm rule /Common/multi {\n"
            "    when RULE_INIT {\n"
            "        set static::app_debug 1\n"
            "    }\n"
            "    when HTTP_REQUEST {\n"
            '        log local0. "request"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 1
        depth_diags = [d for d in result.diagnostics if d.code in ("IRULE5006", "IRULE5007")]
        assert len(depth_diags) == 0


# -- Range shifting -----------------------------------------------------------


class TestRangeShifting:
    """Diagnostics have correct file-absolute line/column/offset positions."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_shift_position_line_zero(self):
        """Line 0 positions shift both line and character."""
        pos = SourcePosition(line=0, character=4, offset=4)
        shifted = _shift_position(pos, base_line=3, base_char=8, base_offset=100)
        assert shifted.line == 3
        assert shifted.character == 12  # 4 + 8
        assert shifted.offset == 104  # 4 + 100

    def test_shift_position_later_line(self):
        """Lines > 0 shift only line (character is relative to line start)."""
        pos = SourcePosition(line=2, character=6, offset=30)
        shifted = _shift_position(pos, base_line=5, base_char=8, base_offset=100)
        assert shifted.line == 7  # 2 + 5
        assert shifted.character == 6  # unchanged
        assert shifted.offset == 130  # 30 + 100

    def test_shift_range_spans_lines(self):
        """Range spanning multiple lines shifts both endpoints correctly."""
        rng = Range(
            start=SourcePosition(line=0, character=2, offset=2),
            end=SourcePosition(line=3, character=5, offset=40),
        )
        shifted = _shift_range(rng, base_line=10, base_char=4, base_offset=200)
        assert shifted.start.line == 10
        assert shifted.start.character == 6  # 2 + 4
        assert shifted.start.offset == 202  # 2 + 200
        assert shifted.end.line == 13  # 3 + 10
        assert shifted.end.character == 5  # unchanged
        assert shifted.end.offset == 240  # 40 + 200

    def test_diagnostic_offsets_are_absolute(self):
        """Diagnostic offsets point to the correct position in the full file."""
        src = (
            "ltm rule /Common/test {\n"  # line 0, 24 chars
            "    when HTTP_REQUEST {\n"  # line 1
            '        HTTP::respond 200 content "ok"\n'  # line 2
            "    }\n"  # line 3
            "}\n"  # line 4
        )
        result, rules = analyse_conf_wrapped(src)
        # Every diagnostic offset should be within the source bounds.
        for diag in result.diagnostics:
            assert 0 <= diag.range.start.offset <= len(src), (
                f"Diagnostic {diag.code} offset {diag.range.start.offset} "
                f"is outside source bounds [0, {len(src)}]"
            )
            assert 0 <= diag.range.end.offset <= len(src), (
                f"Diagnostic {diag.code} end offset {diag.range.end.offset} "
                f"is outside source bounds [0, {len(src)}]"
            )

    def test_second_rule_diagnostic_exact_line(self):
        """IRULE1001 in the second rule maps to the exact file line."""
        src = (
            "ltm rule /Common/first {\n"  # line 0
            "    when RULE_INIT { }\n"  # line 1
            "}\n"  # line 2
            "ltm rule /Common/second {\n"  # line 3
            "    when CLIENT_ACCEPTED {\n"  # line 4
            '        HTTP::respond 200 content "ok"\n'  # line 5
            "    }\n"  # line 6
            "}\n"  # line 7
        )
        result, rules = analyse_conf_wrapped(src)
        irule1001 = [d for d in result.diagnostics if d.code == "IRULE1001"]
        assert len(irule1001) >= 1
        # HTTP::respond is on line 5 in the file.
        assert irule1001[0].range.start.line == 5

    def test_second_rule_diagnostic_offset_in_range(self):
        """Offsets from the second rule body are within the rule's range."""
        src = (
            "ltm rule /Common/first {\n"
            "    when RULE_INIT { }\n"
            "}\n"
            "ltm rule /Common/second {\n"
            "    when CLIENT_ACCEPTED {\n"
            '        HTTP::respond 200 content "ok"\n'
            "    }\n"
            "}\n"
        )
        # IRULE1001 (HTTP command in a non-HTTP event) is gated on the active
        # dialect — configure_signatures alone doesn't set it, so scope the
        # iRules dialect explicitly; otherwise the diagnostic never fires and
        # the offset check is skipped.
        from core.common.dialect import dialect_scope

        with dialect_scope("f5-irules"):
            result, rules = analyse_conf_wrapped(src)
        irule1001 = [d for d in result.diagnostics if d.code == "IRULE1001"]
        assert irule1001, [d.code for d in result.diagnostics]
        # The diagnostic offset must be within the second rule body.
        assert irule1001[0].range.start.offset >= rules[1].body_start_offset
        assert irule1001[0].range.start.offset < rules[1].body_end_offset


# -- Scope tree structure -----------------------------------------------------


class TestScopeTree:
    """Merged analysis has correct scope tree structure."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_scope_children_per_rule(self):
        """Merged global scope has one child per rule."""
        src = (
            "ltm rule /Common/a {\n"
            "    when RULE_INIT { }\n"
            "}\n"
            "ltm rule /Common/b {\n"
            "    when HTTP_REQUEST { }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(result.global_scope.children) == 2

    def test_proc_in_rule_has_shifted_range(self):
        """Proc defined in rule body has ranges in file coordinates."""
        src = (
            "ltm rule /Common/with_proc {\n"  # line 0, 29 chars
            "    proc helper { x } { return $x }\n"  # line 1
            "    when HTTP_REQUEST { }\n"  # line 2
            "}\n"  # line 3
        )
        result, rules = analyse_conf_wrapped(src)
        proc = result.all_procs.get("helper") or result.all_procs.get("::helper")
        assert proc is not None
        # The proc is on line 1 in the file.
        assert proc.name_range.start.line == 1
        # The proc offset should be within the rule body.
        assert proc.name_range.start.offset >= rules[0].body_start_offset


# -- Cross-rule proc sharing --------------------------------------------------


class TestCrossRuleSharing:
    """Procs defined in one rule are visible in merged result."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_procs_from_all_rules_in_merged(self):
        src = (
            "ltm rule /Common/utils {\n"
            "    when RULE_INIT {\n"
            "        proc helper { x } { return $x }\n"
            "    }\n"
            "}\n"
            "ltm rule /Common/main {\n"
            "    when HTTP_REQUEST {\n"
            '        log local0. "req"\n'
            "    }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        # The helper proc from rule_a should be in all_procs.
        assert "helper" in result.all_procs or "::helper" in result.all_procs


# -- find_enclosing_when_event with embedded_rules ----------------------------


class TestEnclosingWhenEvent:
    """find_enclosing_when_event respects conf-wrapped rule body boundaries."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def _rules(self, src: str):
        return find_embedded_rules(src)

    def test_cursor_in_first_rule(self):
        src = (
            "ltm rule /Common/a {\n"  # line 0
            "    when HTTP_REQUEST {\n"  # line 1
            '        log local0. "hi"\n'  # line 2
            "    }\n"  # line 3
            "}\n"  # line 4
            "ltm rule /Common/b {\n"  # line 5
            "    when DNS_REQUEST {\n"  # line 6
            '        log local0. "dns"\n'  # line 7
            "    }\n"  # line 8
            "}\n"  # line 9
        )
        rules = self._rules(src)
        event, _ = find_enclosing_when_event(src, 2, embedded_rules=rules)
        assert event == "HTTP_REQUEST"

    def test_cursor_in_second_rule(self):
        src = (
            "ltm rule /Common/a {\n"
            "    when HTTP_REQUEST {\n"
            '        log local0. "hi"\n'
            "    }\n"
            "}\n"
            "ltm rule /Common/b {\n"
            "    when DNS_REQUEST {\n"
            '        log local0. "dns"\n'
            "    }\n"
            "}\n"
        )
        rules = self._rules(src)
        event, _ = find_enclosing_when_event(src, 7, embedded_rules=rules)
        assert event == "DNS_REQUEST"

    def test_cursor_outside_all_rules(self):
        src = "ltm rule /Common/a {\n    when HTTP_REQUEST { }\n}\n"
        rules = self._rules(src)
        # Line 2 is the closing brace + newline (outside rule body).
        event, _ = find_enclosing_when_event(src, 2, embedded_rules=rules)
        assert event is None

    def test_cursor_in_rule_without_when(self):
        src = "ltm rule /Common/empty {\n    set x 1\n}\n"
        rules = self._rules(src)
        event, _ = find_enclosing_when_event(src, 1, embedded_rules=rules)
        assert event is None


# -- Document symbols ---------------------------------------------------------


class TestDocumentSymbols:
    """get_document_symbols with embedded_rules produces rule containers."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_rule_symbols_are_modules(self):
        from lsprotocol import types

        src = "ltm rule /Common/my_rule {\n    when HTTP_REQUEST { }\n}\n"
        result, rules = analyse_conf_wrapped(src)
        symbols = get_document_symbols(src, analysis=result, embedded_rules=rules)
        assert len(symbols) == 1
        assert symbols[0].kind == types.SymbolKind.Module
        assert symbols[0].name == "/Common/my_rule"

    def test_multiple_rule_symbols(self):
        src = (
            "ltm rule /Common/a {\n"
            "    when RULE_INIT { }\n"
            "}\n"
            "ltm rule /Common/b {\n"
            "    when HTTP_REQUEST { }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        symbols = get_document_symbols(src, analysis=result, embedded_rules=rules)
        assert len(symbols) == 2
        names = [s.name for s in symbols]
        assert "/Common/a" in names
        assert "/Common/b" in names


# -- Brace scanning robustness -----------------------------------------------


class TestBraceScanning:
    """find_embedded_rules handles braces inside quoted strings."""

    def test_braces_in_quoted_string(self):
        """Braces inside double-quoted strings don't terminate the rule body."""
        src = (
            "ltm rule /Common/quoted {\n"
            "    when HTTP_REQUEST {\n"
            '        set x "value with { braces }"\n'
            "        log local0. $x\n"
            "    }\n"
            "}\n"
        )
        rules = find_embedded_rules(src)
        assert len(rules) == 1
        assert rules[0].name == "quoted"
        # Body should contain the full when block including the quoted string.
        assert "braces" in rules[0].body

    def test_escaped_quote_in_string(self):
        """Escaped quotes inside strings are handled correctly."""
        src = (
            "ltm rule /Common/escaped {\n"
            "    when HTTP_REQUEST {\n"
            '        set x "value \\"with\\" braces { }"\n'
            "    }\n"
            "}\n"
        )
        rules = find_embedded_rules(src)
        assert len(rules) == 1
        assert rules[0].name == "escaped"


# -- Empty / edge cases -------------------------------------------------------


class TestEdgeCases:
    """Edge cases for conf-wrapped analysis."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_empty_rule_body(self):
        src = "ltm rule /Common/empty {\n}\n"
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 1
        assert rules[0].name == "empty"

    def test_no_rules_returns_empty(self):
        """Source with no rules should return empty results."""
        src = "# just a comment\n"
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 0
        assert len(result.diagnostics) == 0

    def test_mixed_ltm_gtm_rules(self):
        src = (
            "ltm rule /Common/http_handler {\n"
            "    when HTTP_REQUEST { }\n"
            "}\n"
            "gtm rule /Common/dns_handler {\n"
            "    when DNS_REQUEST { }\n"
            "}\n"
        )
        result, rules = analyse_conf_wrapped(src)
        assert len(rules) == 2
        names = {r.name for r in rules}
        assert "http_handler" in names
        assert "dns_handler" in names
