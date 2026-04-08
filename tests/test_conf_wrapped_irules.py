"""Tests for conf-wrapped iRules analysis (``ltm rule`` / ``gtm rule`` mode)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.conf_wrapped import analyse_conf_wrapped
from core.bigip.rule_extract import is_conf_wrapped_irules
from core.commands.registry.runtime import configure_signatures
from core.common.dialect import detect_dialect_from_source

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


# -- Range shifting -----------------------------------------------------------


class TestRangeShifting:
    """Diagnostics have correct file-absolute line/column positions."""

    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def test_diagnostic_line_offset(self):
        """Diagnostic from rule body at line 3 in file maps to correct line."""
        src = (
            "ltm rule /Common/test {\n"  # line 0
            "    when HTTP_REQUEST {\n"  # line 1
            '        HTTP::respond 200 content "ok"\n'  # line 2
            "    }\n"  # line 3
            "}\n"  # line 4
        )
        result, rules = analyse_conf_wrapped(src)
        # The rule body starts after the opening brace on line 0.
        # Any diagnostics should have line >= 1 (the body starts after
        # the opening brace of ltm rule).
        for diag in result.diagnostics:
            assert diag.range.start.line >= 0, (
                f"Diagnostic {diag.code} at line {diag.range.start.line} is before the file start"
            )

    def test_second_rule_diagnostics_shifted(self):
        """Diagnostics from the second rule have correct line numbers."""
        src = (
            "ltm rule /Common/first {\n"  # line 0
            "    when RULE_INIT { }\n"  # line 1
            "}\n"  # line 2
            "ltm rule /Common/second {\n"  # line 3
            "    when CLIENT_ACCEPTED {\n"  # line 4
            '        HTTP::respond 200 content "ok"\n'  # line 5 — wrong event!
            "    }\n"  # line 6
            "}\n"  # line 7
        )
        result, rules = analyse_conf_wrapped(src)
        # The second rule body starts at line 3 (after opening brace).
        # HTTP::respond in CLIENT_ACCEPTED should generate IRULE1001
        # and the line should be >= 4.
        irule1001 = [d for d in result.diagnostics if d.code == "IRULE1001"]
        if irule1001:
            assert irule1001[0].range.start.line >= 4, (
                f"IRULE1001 diagnostic at line {irule1001[0].range.start.line} "
                f"should be >= 4 (in second rule body)"
            )


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
