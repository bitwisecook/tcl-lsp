"""Apply-the-edit refactoring tests — verify semantic reality.

Unlike ``test_refactoring.py`` / ``test_code_actions.py``, which assert that
an action *exists* with the right title, these tests **apply** the edits the
refactoring produces and assert the exact resulting Tcl.  That is the only way
to catch off-by-one spans that drop characters, dropped structural braces, or
broken string interpolation — bugs that a title/existence check sails past.
"""

from __future__ import annotations

from collections.abc import Sequence

from lsprotocol import types

from core.refactoring._brace_expr import brace_expr
from core.refactoring._extract_variable import extract_variable
from core.refactoring._if_to_switch import if_to_switch
from core.refactoring._inline_variable import inline_variable
from core.refactoring._switch_to_dict import switch_to_dict
from lsp.features.code_actions import get_code_actions


def _apply_lsp_edits(source: str, edits: Sequence[types.TextEdit]) -> str:
    """Apply LSP TextEdits (0-based ranges) to *source* bottom-to-top."""
    lines = source.split("\n")
    starts = [0]
    for ln in lines:
        starts.append(starts[-1] + len(ln) + 1)

    def off(line: int, char: int) -> int:
        return starts[line] + char

    spans = [
        (
            off(e.range.start.line, e.range.start.character),
            off(e.range.end.line, e.range.end.character),
            e.new_text,
        )
        for e in edits
    ]
    # Apply bottom-to-top.  When two edits share a start offset (e.g. an
    # insert at (0,0) plus a replace of (0,0)-(1,0), as extract-proc emits),
    # the larger-end edit must be applied first — matching how an LSP client
    # orders same-position edits so the insert lands before the replacement.
    spans.sort(key=lambda x: (x[0], x[1]), reverse=True)
    out = source
    for s, en, txt in spans:
        out = out[:s] + txt + out[en:]
    return out


def _apply_action(source: str, action: types.CodeAction) -> str:
    assert action.edit is not None and action.edit.changes is not None
    edits = action.edit.changes["__current__"]
    return _apply_lsp_edits(source, edits)


def _find_action(actions: list[types.CodeAction], needle: str) -> types.CodeAction:
    for a in actions:
        if needle in a.title:
            return a
    titles = [a.title for a in actions]
    raise AssertionError(f"no action matching {needle!r}; got {titles}")


def _ctx(diags: list[types.Diagnostic] | None = None) -> types.CodeActionContext:
    return types.CodeActionContext(diagnostics=diags or [])


def _rng(sl: int, sc: int, el: int, ec: int) -> types.Range:
    return types.Range(
        start=types.Position(line=sl, character=sc),
        end=types.Position(line=el, character=ec),
    )


def _diag(code: str, message: str, sl: int, sc: int, el: int, ec: int) -> types.Diagnostic:
    return types.Diagnostic(
        range=_rng(sl, sc, el, ec),
        message=message,
        source="tcl-lsp",
        code=code,
    )


# ── extract variable ──────────────────────────────────────────────────


class TestExtractVariableApplied:
    def test_bare_operator_expr_is_wrapped_in_expr(self):
        # Selecting a bare arithmetic expression must produce a *valid* set
        # command — ``set result $a * $b`` is a 4-arg call and is broken.
        source = "set total [expr {$price * $qty + $tax}]\n"
        result = extract_variable(source, 0, 17, 0, 37)
        assert result is not None
        applied = result.apply(source)
        assert "set result [expr {$price * $qty + $tax}]" in applied
        assert "set total [expr {$result}]" in applied

    def test_command_substitution_selection_is_kept_verbatim(self):
        source = "set total [expr {$a + $b}]\n"
        result = extract_variable(source, 0, 10, 0, 26)
        assert result is not None
        applied = result.apply(source)
        # Already a [...] command substitution — must not be double-wrapped.
        assert "set result [expr {$a + $b}]" in applied
        assert "[expr {[expr" not in applied
        assert "set total $result" in applied

    def test_plain_word_selection_not_wrapped(self):
        source = 'set greeting "hello world"\n'
        result = extract_variable(source, 0, 13, 0, 26)
        assert result is not None
        applied = result.apply(source)
        assert 'set result "hello world"' in applied
        assert "expr" not in applied


# ── inline variable ───────────────────────────────────────────────────


class TestInlineVariableApplied:
    def test_standalone_use_keeps_quotes(self):
        source = 'set name "world"\nputs $name\n'
        result = inline_variable(source, 0, 4)
        assert result is not None
        assert result.apply(source) == 'puts "world"\n'

    def test_interpolation_into_quoted_string_dequotes(self):
        # The bug: keeping the value's own quotes produced ``"hello "world""``.
        source = 'set name "world"\nputs "hello $name"\n'
        result = inline_variable(source, 0, 4)
        assert result is not None
        assert result.apply(source) == 'puts "hello world"\n'

    def test_braced_value_interpolation_dequotes(self):
        source = 'set greet {hi there}\nputs "say $greet"\n'
        result = inline_variable(source, 0, 4)
        assert result is not None
        assert result.apply(source) == 'puts "say hi there"\n'

    def test_bare_concatenation_with_spaces_is_refused(self):
        # Splicing ``a b`` into ``foo$x`` would split the word — refuse.
        source = 'set x "a b"\nputs foo$x\n'
        assert inline_variable(source, 0, 4) is None


# ── inline proc (LSP code-actions layer) ──────────────────────────────


class TestInlineProcApplied:
    def test_preserves_body_braces(self):
        # The bug: joining word texts dropped the expr braces, turning
        # ``expr {$x * 2}`` into ``expr $x * 2``.
        source = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 5\n"
        actions = get_code_actions(source, _rng(3, 0, 3, 0), _ctx())
        action = _find_action(actions, "Inline proc")
        applied = _apply_action(source, action)
        assert "expr {5 * 2}" in applied
        assert "expr 5 * 2" not in applied

    def test_substitutes_argument(self):
        source = "proc inc {n} {\n    expr {$n + 1}\n}\ninc 41\n"
        actions = get_code_actions(source, _rng(3, 0, 3, 0), _ctx())
        action = _find_action(actions, "Inline proc")
        applied = _apply_action(source, action)
        assert "expr {41 + 1}" in applied


# ── extract proc (LSP code-actions layer) ─────────────────────────────


class TestExtractProcApplied:
    def test_extracted_body_is_complete(self):
        source = "puts hello\nputs world\n"
        actions = get_code_actions(source, _rng(0, 0, 1, 0), _ctx())
        action = _find_action(actions, "Extract selection into proc")
        applied = _apply_action(source, action)
        # The extracted proc body must contain the full selected line, and the
        # call site must invoke the new proc on its own line.
        assert "proc extracted_proc {} {" in applied
        assert "    puts hello" in applied
        assert "\nextracted_proc\n" in applied
        assert "puts world" in applied


# ── if → switch ───────────────────────────────────────────────────────


class TestIfToSwitchApplied:
    def test_chain_becomes_switch(self):
        source = (
            "if {$x == 1} {\n"
            "    puts one\n"
            "} elseif {$x == 2} {\n"
            "    puts two\n"
            "} else {\n"
            "    puts other\n"
            "}\n"
        )
        result = if_to_switch(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "switch -exact -- $x {" in applied
        assert "1 {" in applied
        assert "puts one" in applied
        assert "2 {" in applied
        assert "puts two" in applied
        assert "default {" in applied
        assert "puts other" in applied
        # No fragments of the original ``if`` should survive.
        assert "elseif" not in applied


# ── switch → dict ─────────────────────────────────────────────────────


class TestSwitchToDictApplied:
    def test_switch_becomes_dict_lookup(self):
        source = (
            "switch -exact -- $color {\n"
            '    red { set hex "#ff0000" }\n'
            '    green { set hex "#00ff00" }\n'
            '    blue { set hex "#0000ff" }\n'
            "}\n"
        )
        result = switch_to_dict(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "dict create" in applied
        assert '"#ff0000"' in applied
        assert '"#00ff00"' in applied
        assert '"#0000ff"' in applied
        assert "dict get" in applied
        assert "switch" not in applied


# ── brace expr ────────────────────────────────────────────────────────


class TestBraceExprApplied:
    def test_unbraced_expr_gets_braced(self):
        source = "expr $a + $b\n"
        result = brace_expr(source, 0, 0)
        assert result is not None
        assert result.apply(source) == "expr {$a + $b}\n"


# ── expression rewrites (De Morgan / invert) ──────────────────────────


class TestExprRewriteApplied:
    def test_invert_expression(self):
        source = "if {$a == $b} {}\n"
        actions = get_code_actions(source, _rng(0, 4, 0, 12), _ctx())
        action = _find_action(actions, "Invert expression")
        applied = _apply_action(source, action)
        assert "$a != $b" in applied

    def test_demorgan(self):
        # De Morgan applies to a negated grouping: !(a || b) -> !a && !b.
        source = "if {!($a || $b)} {}\n"
        actions = get_code_actions(source, _rng(0, 4, 0, 15), _ctx())
        action = _find_action(actions, "De Morgan")
        applied = _apply_action(source, action)
        assert "!$a && !$b" in applied


# ── quick-fixes (diagnostic-driven) ───────────────────────────────────


class TestQuickFixesApplied:
    def test_w302_add_catch_result(self):
        source = "catch {error oops}\n"
        d = _diag("W302", "catch without a result variable", 0, 0, 0, 5)
        actions = get_code_actions(source, d.range, _ctx([d]))
        action = _find_action(actions, "Add catch result variable")
        applied = _apply_action(source, action)
        assert applied == "catch {error oops} result\n"

    def test_w120_add_package_require(self):
        source = "json::write foo\n"
        d = _diag(
            "W120",
            "Command 'json::write' requires `package require json`.",
            0,
            0,
            0,
            11,
        )
        actions = get_code_actions(source, d.range, _ctx([d]))
        action = _find_action(actions, "package require json")
        applied = _apply_action(source, action)
        assert applied.splitlines()[0] == "package require json"
        assert "json::write foo" in applied
