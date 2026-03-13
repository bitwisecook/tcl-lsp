"""Tests for code action quick fixes."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.commands.registry.runtime import configure_signatures
from lsp.features.code_actions import get_code_actions


def _diag(
    code: str,
    message: str,
    *,
    start_line: int,
    start_char: int,
    end_line: int,
    end_char: int,
) -> types.Diagnostic:
    return types.Diagnostic(
        range=types.Range(
            start=types.Position(line=start_line, character=start_char),
            end=types.Position(line=end_line, character=end_char),
        ),
        message=message,
        source="tcl-lsp",
        code=code,
    )


def _action_snippets(actions: list[types.CodeAction]) -> list[str]:
    snippets: list[str] = []
    for action in actions:
        if not action.edit or not action.edit.changes:
            continue
        edits = action.edit.changes.get("__current__")
        if not edits:
            continue
        snippets.extend(edit.new_text for edit in edits)
    return snippets


class TestIrulesCollectCodeActions:
    def test_irule1005_adds_collect_bootstrap_options(self):
        source = "when CLIENT_DATA {\n    set payload [TCP::payload]\n}\n"
        diagnostic = _diag(
            "IRULE1005",
            "'CLIENT_DATA' will never fire without a TCP::collect or UDP::collect call in another event.",
            start_line=0,
            start_char=5,
            end_line=0,
            end_char=16,
        )
        context = types.CodeActionContext(diagnostics=[diagnostic])

        actions = get_code_actions(source, diagnostic.range, context, package_names=[])

        snippets = _action_snippets(actions)
        assert any(
            "when CLIENT_ACCEPTED" in snippet and "TCP::collect" in snippet for snippet in snippets
        )
        assert any(
            "when CLIENT_ACCEPTED" in snippet and "UDP::collect" in snippet for snippet in snippets
        )

    def test_irule1006_prefers_server_ssl_handshake_bootstrap(self):
        source = "when SERVERSSL_DATA {\n    SSL::payload\n}\n"
        diagnostic = _diag(
            "IRULE1006",
            "'SSL::payload' without a SSL::collect call. The payload buffer will be empty.",
            start_line=1,
            start_char=4,
            end_line=1,
            end_char=16,
        )
        context = types.CodeActionContext(diagnostics=[diagnostic])

        actions = get_code_actions(source, diagnostic.range, context, package_names=[])

        snippets = _action_snippets(actions)
        assert any(
            "when SERVERSSL_HANDSHAKE" in snippet and "SSL::collect" in snippet
            for snippet in snippets
        )

    def test_irule1006_bootstrap_action_is_deduplicated(self):
        source = "when CLIENT_DATA {\n    set a [TCP::payload]\n    set b [TCP::payload]\n}\n"
        first = _diag(
            "IRULE1006",
            "'TCP::payload' without a TCP::collect call. The payload buffer will be empty.",
            start_line=1,
            start_char=11,
            end_line=1,
            end_char=23,
        )
        second = _diag(
            "IRULE1006",
            "'TCP::payload' without a TCP::collect call. The payload buffer will be empty.",
            start_line=2,
            start_char=11,
            end_line=2,
            end_char=23,
        )
        context = types.CodeActionContext(diagnostics=[first, second])

        actions = get_code_actions(source, first.range, context, package_names=[])

        collect_actions = [
            action for action in actions if action.title and "TCP::collect" in action.title
        ]
        assert len(collect_actions) == 1


class TestCatchCodeActions:
    def test_w302_adds_result_capture_actions(self):
        source = "catch {error oops}\n"
        diagnostic = _diag(
            "W302",
            "catch without a result variable silently swallows errors.",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=4,
        )
        context = types.CodeActionContext(
            diagnostics=[diagnostic],
            only=[types.CodeActionKind.QuickFix],
        )

        actions = get_code_actions(source, diagnostic.range, context, package_names=[])
        snippets = _action_snippets(actions)

        assert " result" in snippets
        assert " result opts" in snippets

    def test_w302_does_not_offer_fix_when_result_is_present(self):
        source = "catch {error oops} result\n"
        diagnostic = _diag(
            "W302",
            "catch without a result variable silently swallows errors.",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=4,
        )
        context = types.CodeActionContext(
            diagnostics=[diagnostic],
            only=[types.CodeActionKind.QuickFix],
        )

        actions = get_code_actions(source, diagnostic.range, context, package_names=[])
        assert actions == []


class TestRefactorCodeActions:
    def test_extract_proc_available_without_diagnostics(self):
        source = "set value 42\nputs $value\nputs done\n"
        selected = types.Range(
            start=types.Position(line=1, character=0),
            end=types.Position(line=2, character=0),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorExtract],
        )

        actions = get_code_actions(source, selected, context, package_names=[])
        extract = next(
            (a for a in actions if a.kind == types.CodeActionKind.RefactorExtract),
            None,
        )
        assert extract is not None
        snippets = _action_snippets([extract])
        assert any("proc extracted_proc {value}" in text for text in snippets)
        assert any("extracted_proc $value" in text for text in snippets)

    def test_extract_proc_attaches_rename_command(self):
        source = "set value 42\nputs $value\nputs done\n"
        selected = types.Range(
            start=types.Position(line=1, character=0),
            end=types.Position(line=2, character=0),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorExtract],
        )

        actions = get_code_actions(source, selected, context, package_names=[])
        extract = next(
            (a for a in actions if a.kind == types.CodeActionKind.RefactorExtract),
            None,
        )
        assert extract is not None
        assert extract.command is not None
        assert extract.command.command == "tclLsp.renameSymbolAtPosition"
        # Arguments: [line, name_start, name_end]
        assert extract.command.arguments is not None
        line, start, end = extract.command.arguments
        assert line == 0  # inserted at top
        assert start == len("proc ")
        assert end == start + len("extracted_proc")

    def test_inline_proc_available_without_diagnostics(self):
        source = "proc greet {name} { puts $name }\ngreet Bob\n"
        caret = types.Range(
            start=types.Position(line=1, character=1),
            end=types.Position(line=1, character=1),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorInline],
        )

        actions = get_code_actions(source, caret, context, package_names=[])
        inline = next(
            (a for a in actions if a.kind == types.CodeActionKind.RefactorInline),
            None,
        )
        assert inline is not None
        snippets = _action_snippets([inline])
        assert any("puts Bob" in text for text in snippets)

    def test_inline_proc_skips_returning_proc(self):
        source = "proc wrap {x} { return $x }\nwrap value\n"
        caret = types.Range(
            start=types.Position(line=1, character=1),
            end=types.Position(line=1, character=1),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorInline],
        )

        actions = get_code_actions(source, caret, context, package_names=[])
        inline = [a for a in actions if a.kind == types.CodeActionKind.RefactorInline]
        assert len(inline) == 0


class TestExprRefactorCodeActions:
    def test_demorgan_forward_action(self):
        source = "if {!($a && $b)} { puts yes }\n"
        selected = types.Range(
            start=types.Position(line=0, character=4),
            end=types.Position(line=0, character=15),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, selected, context, package_names=[])
        dm = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite and "De Morgan" in (a.title or "")
        ]
        assert len(dm) == 1
        snippets = _action_snippets(dm)
        assert any("!$a || !$b" in s for s in snippets)

    def test_demorgan_reverse_action(self):
        source = "if {!$a || !$b} { puts yes }\n"
        selected = types.Range(
            start=types.Position(line=0, character=4),
            end=types.Position(line=0, character=14),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, selected, context, package_names=[])
        dm = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite and "De Morgan" in (a.title or "")
        ]
        assert len(dm) == 1
        snippets = _action_snippets(dm)
        assert any("!($a && $b)" in s for s in snippets)

    def test_invert_expression_action(self):
        source = "if {$a == $b} { puts yes }\n"
        selected = types.Range(
            start=types.Position(line=0, character=4),
            end=types.Position(line=0, character=12),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, selected, context, package_names=[])
        inv = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite and "Invert" in (a.title or "")
        ]
        assert len(inv) == 1
        snippets = _action_snippets(inv)
        assert any("$a != $b" in s for s in snippets)

    def test_no_actions_for_non_expression_selection(self):
        source = "puts hello\n"
        selected = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=4),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, selected, context, package_names=[])
        expr_actions = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite
            and ("De Morgan" in (a.title or "") or "Invert" in (a.title or ""))
        ]
        assert len(expr_actions) == 0

    def test_no_actions_for_empty_selection(self):
        source = "if {$a == $b} { puts yes }\n"
        caret = types.Range(
            start=types.Position(line=0, character=5),
            end=types.Position(line=0, character=5),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, caret, context, package_names=[])
        expr_actions = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite
            and ("De Morgan" in (a.title or "") or "Invert" in (a.title or ""))
        ]
        assert len(expr_actions) == 0

    def test_invert_subexpression_in_compound(self):
        """Select just $c < $d from a larger expression."""
        source = "if {$a == $b && $c < $d} { puts yes }\n"
        selected = types.Range(
            start=types.Position(line=0, character=16),
            end=types.Position(line=0, character=23),
        )
        context = types.CodeActionContext(
            diagnostics=[],
            only=[types.CodeActionKind.RefactorRewrite],
        )
        actions = get_code_actions(source, selected, context, package_names=[])
        inv = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorRewrite and "Invert" in (a.title or "")
        ]
        assert len(inv) == 1
        snippets = _action_snippets(inv)
        assert any("$c >= $d" in s for s in snippets)


# Taint quick-fix code actions


class TestTaintQuickFixes:
    """Code actions for taint diagnostics — wrapping and '--' insertion."""

    def _fix_actions(
        self,
        source: str,
        code: str,
        message: str,
        start_line: int,
        start_char: int,
        end_line: int,
        end_char: int,
    ) -> list[types.CodeAction]:
        d = _diag(
            code,
            message,
            start_line=start_line,
            start_char=start_char,
            end_line=end_line,
            end_char=end_char,
        )
        context = types.CodeActionContext(
            diagnostics=[d],
            only=[types.CodeActionKind.QuickFix],
        )
        selected = types.Range(
            start=types.Position(line=start_line, character=start_char),
            end=types.Position(line=end_line, character=end_char),
        )
        return get_code_actions(source, selected, context)

    def test_irule3001_wrap_html_encode(self):
        """IRULE3001 quick fix wraps variable with [html_encode]."""
        source = "HTTP::respond 200 content $raw\n"
        actions = self._fix_actions(
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=29,
        )
        fixes = [a for a in actions if "html_encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[html_encode $raw]" in s for s in snippets)

    def test_irule3002_wrap_uri_encode(self):
        """IRULE3002 quick fix wraps variable with [URI::encode]."""
        source = "HTTP::header insert X-Fwd $raw\n"
        actions = self._fix_actions(
            source,
            "IRULE3002",
            "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=29,
        )
        fixes = [a for a in actions if "URI::encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[URI::encode $raw]" in s for s in snippets)

    def test_t103_wrap_regex_quote(self):
        """T103 quick fix wraps variable with [regex::quote]."""
        source = "regexp $pat $data\n"
        actions = self._fix_actions(
            source,
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=16,
        )
        fixes = [a for a in actions if "regex::quote" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[regex::quote $pat]" in s for s in snippets)

    def test_t102_insert_double_dash(self):
        """T102 quick fix inserts '--' before the tainted variable."""
        source = "regexp $pat $data\n"
        actions = self._fix_actions(
            source,
            "T102",
            "Tainted variable $pat in option position of 'regexp' without '--' terminator; risk of option injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=16,
        )
        fixes = [a for a in actions if "--" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("-- " in s for s in snippets)

    def test_braced_variable_wrapped(self):
        """Wrapping works for ${var} syntax too."""
        source = "HTTP::respond 200 content ${raw}\n"
        actions = self._fix_actions(
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=30,
        )
        fixes = [a for a in actions if "html_encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[html_encode ${raw}]" in s for s in snippets)

    def test_no_fix_for_unknown_code(self):
        """Unknown diagnostic code produces no taint fix."""
        source = "puts $raw\n"
        actions = self._fix_actions(
            source,
            "T101",
            "Tainted variable $raw flows into puts; output may contain injected content",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=8,
        )
        taint_fixes = [
            a
            for a in actions
            if any(
                kw in (a.title or "")
                for kw in ("HTML::encode", "URI::encode", "regex::quote", "--")
            )
        ]
        assert len(taint_fixes) == 0

    def test_t106_remove_redundant_encode(self):
        """T106 quick fix removes the redundant encoding wrapper."""
        source = "set double [HTML::encode $safe]\n"
        actions = self._fix_actions(
            source,
            "T106",
            "Variable $safe is already HTML-escaped; passing through HTML::encode double-encodes the value",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=30,
        )
        fixes = [a for a in actions if "Remove redundant" in (a.title or "")]
        assert len(fixes) == 1
        assert "HTML::encode" in fixes[0].title
        snippets = _action_snippets(fixes)
        assert any("$safe" == s for s in snippets)

    def test_irule3004_no_autofix(self):
        """IRULE3004 (open redirect) has no automatic fix."""
        source = "HTTP::redirect $url\n"
        actions = self._fix_actions(
            source,
            "IRULE3004",
            "Tainted variable $url in redirect URL (HTTP::redirect); risk of open redirect",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=18,
        )
        # Should find no automatic wrapper fix.
        taint_fixes = [
            a
            for a in actions
            if any(kw in (a.title or "") for kw in ("html_encode", "URI::encode", "regex::quote"))
        ]
        assert len(taint_fixes) == 0


class TestTaintProcInsertion:
    """Code actions that insert helper proc definitions."""

    def _fix_actions(
        self,
        source: str,
        code: str,
        message: str,
        start_line: int,
        start_char: int,
        end_line: int,
        end_char: int,
    ) -> list[types.CodeAction]:
        d = _diag(
            code,
            message,
            start_line=start_line,
            start_char=start_char,
            end_line=end_line,
            end_char=end_char,
        )
        context = types.CodeActionContext(
            diagnostics=[d],
            only=[types.CodeActionKind.QuickFix],
        )
        selected = types.Range(
            start=types.Position(line=start_line, character=start_char),
            end=types.Position(line=end_line, character=end_char),
        )
        return get_code_actions(source, selected, context)

    def test_t103_inserts_regex_quote_proc(self):
        """T103 fix inserts regex::quote proc when not defined."""
        source = "regexp $pat $data\n"
        actions = self._fix_actions(
            source,
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=16,
        )
        fixes = [a for a in actions if "regex::quote" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        # Should contain both the wrapping edit and the proc definition.
        assert any("[regex::quote $pat]" in s for s in snippets)
        assert any("proc regex::quote" in s for s in snippets)

    def test_t103_no_proc_insert_when_already_defined(self):
        """T103 fix does NOT insert proc when regex::quote already exists."""
        source = (
            "proc regex::quote {str} { regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&} }\n"
            "regexp $pat $data\n"
        )
        actions = self._fix_actions(
            source,
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            start_line=1,
            start_char=0,
            end_line=1,
            end_char=16,
        )
        fixes = [a for a in actions if "regex::quote" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        # Should have the wrapping edit but NOT the proc definition.
        assert any("[regex::quote $pat]" in s for s in snippets)
        assert not any("proc regex::quote" in s for s in snippets)

    def test_irule3001_inserts_html_encode_proc(self):
        """IRULE3001 fix inserts html_encode proc when not defined."""
        source = "HTTP::respond 200 content $raw\n"
        actions = self._fix_actions(
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=29,
        )
        fixes = [a for a in actions if "html_encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[html_encode $raw]" in s for s in snippets)
        assert any("proc html_encode" in s for s in snippets)

    def test_irule3001_no_proc_insert_when_already_defined(self):
        """IRULE3001 fix does NOT insert proc when html_encode exists."""
        source = (
            "proc html_encode {str} { string map {& &amp; < &lt;} $str }\n"
            "HTTP::respond 200 content $raw\n"
        )
        actions = self._fix_actions(
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            start_line=1,
            start_char=0,
            end_line=1,
            end_char=29,
        )
        fixes = [a for a in actions if "html_encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        assert any("[html_encode $raw]" in s for s in snippets)
        assert not any("proc html_encode" in s for s in snippets)

    def test_irule3002_no_proc_insert(self):
        """IRULE3002 (URI::encode) has no template — no proc insertion."""
        source = "HTTP::header insert X-Fwd $raw\n"
        actions = self._fix_actions(
            source,
            "IRULE3002",
            "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=29,
        )
        fixes = [a for a in actions if "URI::encode" in (a.title or "")]
        assert len(fixes) == 1
        snippets = _action_snippets(fixes)
        # URI::encode is a built-in, no proc insertion.
        assert not any("proc " in s for s in snippets)

    def test_proc_template_has_regsub(self):
        """regex::quote template uses regsub."""
        source = "regexp $pat $data\n"
        actions = self._fix_actions(
            source,
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=16,
        )
        fixes = [a for a in actions if "regex::quote" in (a.title or "")]
        snippets = _action_snippets(fixes)
        assert any("regsub" in s for s in snippets)

    def test_proc_template_has_string_map(self):
        """html_encode template uses string map."""
        source = "HTTP::respond 200 content $raw\n"
        actions = self._fix_actions(
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            start_line=0,
            start_char=0,
            end_line=0,
            end_char=29,
        )
        fixes = [a for a in actions if "html_encode" in (a.title or "")]
        snippets = _action_snippets(fixes)
        assert any("string map" in s for s in snippets)


class TestW115CommentContinuationCodeAction:
    """Code actions for W115: convert backslash-continued comment to per-line comments."""

    def test_simple_continuation_fix(self):
        source = "# hello \\\nworld"
        diagnostic = _diag(
            "W115",
            "Backslash-newline in comment silently swallows the next line",
            start_line=0,
            start_char=0,
            end_line=1,
            end_char=4,
        )
        context = types.CodeActionContext(diagnostics=[diagnostic])
        actions = get_code_actions(source, diagnostic.range, context, package_names=[])

        fix_actions = [a for a in actions if a.title and "per-line" in a.title]
        assert len(fix_actions) == 1
        snippets = _action_snippets(fix_actions)
        assert len(snippets) == 1
        assert "# hello" in snippets[0]
        assert "# world" in snippets[0]
        assert "\\" not in snippets[0]

    def test_chained_continuation_fix(self):
        source = "# line1 \\\nline2 \\\nline3"
        diagnostic = _diag(
            "W115",
            "Backslash-newline in comment silently swallows the next line",
            start_line=0,
            start_char=0,
            end_line=2,
            end_char=4,
        )
        context = types.CodeActionContext(diagnostics=[diagnostic])
        actions = get_code_actions(source, diagnostic.range, context, package_names=[])

        fix_actions = [a for a in actions if a.title and "per-line" in a.title]
        assert len(fix_actions) == 1
        snippets = _action_snippets(fix_actions)
        assert "# line1" in snippets[0]
        assert "# line2" in snippets[0]
        assert "# line3" in snippets[0]

    def test_already_commented_continuation_not_doubled(self):
        """When the continuation line already starts with #, don't add another."""
        source = "# line1 \\\n# line2"
        diagnostic = _diag(
            "W115",
            "Backslash-newline in comment silently swallows the next line",
            start_line=0,
            start_char=0,
            end_line=1,
            end_char=6,
        )
        context = types.CodeActionContext(diagnostics=[diagnostic])
        actions = get_code_actions(source, diagnostic.range, context, package_names=[])

        fix_actions = [a for a in actions if a.title and "per-line" in a.title]
        snippets = _action_snippets(fix_actions)
        assert len(snippets) == 1
        # Should have exactly two '#' prefixed lines, not '# # line2'.
        result_lines = snippets[0].split("\n")
        assert result_lines[0] == "# line1"
        assert result_lines[1] == "# line2"


# Profile requirements header generation

_NO_DIAG_CONTEXT = types.CodeActionContext(diagnostics=[])
_FULL_DOC_RANGE = types.Range(
    start=types.Position(line=0, character=0),
    end=types.Position(line=0, character=0),
)


def _source_actions(actions: list[types.CodeAction]) -> list[types.CodeAction]:
    return [a for a in actions if a.kind == types.CodeActionKind.Source]


class TestGenerateProfilesHeader:
    def setup_method(self):
        configure_signatures(dialect="f5-irules")

    def teardown_method(self):
        configure_signatures(dialect="tcl8.6")

    def test_http_event_generates_http_profile(self):
        source = "when HTTP_REQUEST {\n    HTTP::respond 200\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        assert snippets[0] == "# Profiles: HTTP\n"

    def test_http_event_plus_ssl_command(self):
        # SSL::extensions requires CLIENTSSL or SERVERSSL profiles.
        source = "when HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        # SSL::extensions adds CLIENTSSL + SERVERSSL; event adds HTTP.
        assert "CLIENTSSL" in snippets[0]
        assert "HTTP" in snippets[0]

    def test_existing_matching_directive_no_action(self):
        source = "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    HTTP::respond 200\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 0

    def test_existing_matching_directive_comma_format(self):
        source = (
            "# Profiles: CLIENTSSL, HTTP, SERVERSSL\n"
            "when HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n"
        )
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 0

    def test_existing_outdated_directive_offers_update(self):
        source = (
            "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n"
        )
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        assert "Update" in sa[0].title
        snippets = _action_snippets(sa)
        assert "CLIENTSSL" in snippets[0]
        assert "HTTP" in snippets[0]
        assert "Update" in sa[0].title

    def test_fasthttp_normalised_to_http(self):
        # HTTP_REQUEST implies {HTTP, FASTHTTP} — we normalise to HTTP only.
        source = "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        assert "FASTHTTP" not in snippets[0]
        assert "HTTP" in snippets[0]

    def test_no_action_for_tcl_dialect(self):
        configure_signatures(dialect="tcl8.6")
        source = "proc hello {} { puts hi }\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 0

    def test_dns_event_generates_dns_profile(self):
        source = "when DNS_REQUEST {\n    set q [DNS::question name]\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        assert snippets[0] == "# Profiles: DNS\n"

    def test_clientssl_event_generates_clientssl_profile(self):
        source = 'when CLIENTSSL_HANDSHAKE {\n    log local0. "TLS done"\n}\n'
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        assert snippets[0] == "# Profiles: CLIENTSSL\n"

    def test_multiple_events_combines_profiles(self):
        source = (
            "when HTTP_REQUEST {\n    HTTP::uri\n}\n"
            'when CLIENTSSL_HANDSHAKE {\n    log local0. "done"\n}\n'
        )
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 1
        snippets = _action_snippets(sa)
        assert snippets[0] == "# Profiles: CLIENTSSL, HTTP\n"

    def test_rule_init_only_no_action(self):
        source = "when RULE_INIT {\n    set ::counter 0\n}\n"
        actions = get_code_actions(source, _FULL_DOC_RANGE, _NO_DIAG_CONTEXT)
        sa = _source_actions(actions)
        assert len(sa) == 0


# IP conversion code actions


def _refactor_actions(actions: list[types.CodeAction]) -> list[types.CodeAction]:
    return [a for a in actions if a.kind == types.CodeActionKind.RefactorRewrite]


class TestIPConversionCodeActions:
    def test_ipv4_offers_ipv6_mapped(self):
        source = "set addr 10.0.0.1"
        cursor = types.Range(
            start=types.Position(line=0, character=10),
            end=types.Position(line=0, character=10),
        )
        actions = get_code_actions(source, cursor, _NO_DIAG_CONTEXT)
        ra = _refactor_actions(actions)
        assert any("IPv6-mapped" in a.title for a in ra)
        snippets = _action_snippets(ra)
        assert any("::ffff:" in s for s in snippets)

    def test_ipv6_mapped_offers_ipv4(self):
        source = "set addr ::ffff:192.168.1.1"
        cursor = types.Range(
            start=types.Position(line=0, character=12),
            end=types.Position(line=0, character=12),
        )
        actions = get_code_actions(source, cursor, _NO_DIAG_CONTEXT)
        ra = _refactor_actions(actions)
        assert any("IPv4" in a.title for a in ra)
        snippets = _action_snippets(ra)
        assert any("192.168.1.1" in s for s in snippets)

    def test_non_mapped_ipv6_no_ipv4_action(self):
        source = "set addr ::1"
        cursor = types.Range(
            start=types.Position(line=0, character=10),
            end=types.Position(line=0, character=10),
        )
        actions = get_code_actions(source, cursor, _NO_DIAG_CONTEXT)
        ra = _refactor_actions(actions)
        assert not any("IPv4" in a.title for a in ra)

    def test_ipv4_with_prefix_preserves_suffix(self):
        source = "set net 10.0.0.0/8"
        cursor = types.Range(
            start=types.Position(line=0, character=10),
            end=types.Position(line=0, character=10),
        )
        actions = get_code_actions(source, cursor, _NO_DIAG_CONTEXT)
        ra = _refactor_actions(actions)
        snippets = _action_snippets(ra)
        assert any("/8" in s for s in snippets)

    def test_non_ip_word_no_action(self):
        source = "set name hello"
        cursor = types.Range(
            start=types.Position(line=0, character=10),
            end=types.Position(line=0, character=10),
        )
        actions = get_code_actions(source, cursor, _NO_DIAG_CONTEXT)
        ra = _refactor_actions(actions)
        assert len(ra) == 0
