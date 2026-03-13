"""Tests for context-aware snippet template completions."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.formatting.config import BraceStyle, FormatterConfig
from lsp.features.completion import get_completions
from lsp.features.snippet_templates import (
    SnippetContext,
    get_snippet_completions,
)

# Helpers


def _default_ctx(**overrides) -> SnippetContext:
    defaults = dict(
        dialect="tcl",
        brace_style=BraceStyle.K_AND_R,
        indent_unit="    ",
        current_event=None,
        file_events=frozenset(),
        scope_vars=[],
        partial="",
    )
    defaults.update(overrides)
    return SnippetContext(**defaults)  # type: ignore[arg-type]


def _labels(items: list[types.CompletionItem]) -> list[str]:
    return [i.label for i in items]


# Filtering tests


class TestSnippetFiltering:
    def test_all_tcl_templates_returned_by_default(self):
        items = get_snippet_completions(_default_ctx())
        labels = _labels(items)
        assert "Tcl Proc" in labels
        assert "Foreach" in labels
        assert "If Else" in labels
        assert "Try Trap" in labels

    def test_irules_templates_hidden_in_tcl_dialect(self):
        items = get_snippet_completions(_default_ctx(dialect="tcl"))
        labels = _labels(items)
        assert "iRule HTTP_REQUEST" not in labels
        assert "iRule RULE_INIT" not in labels
        assert "iRule Collect/Release" not in labels

    def test_irules_templates_shown_in_irules_dialect(self):
        items = get_snippet_completions(_default_ctx(dialect="f5-irules"))
        labels = _labels(items)
        assert "iRule HTTP_REQUEST" in labels
        assert "iRule RULE_INIT" in labels

    def test_top_level_templates_hidden_inside_event(self):
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                current_event="HTTP_REQUEST",
            )
        )
        labels = _labels(items)
        assert "iRule HTTP_REQUEST" not in labels
        assert "iRule RULE_INIT" not in labels
        # Tcl core templates should still appear
        assert "Tcl Proc" in labels

    def test_existing_event_suppresses_template(self):
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                file_events=frozenset({"HTTP_REQUEST"}),
            )
        )
        labels = _labels(items)
        assert "iRule HTTP_REQUEST" not in labels
        assert "iRule Redirect HTTPS" not in labels
        assert "iRule Data Group Lookup" not in labels
        # RULE_INIT is not suppressed
        assert "iRule RULE_INIT" in labels

    def test_rule_init_suppressed_when_exists(self):
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                file_events=frozenset({"RULE_INIT"}),
            )
        )
        labels = _labels(items)
        assert "iRule RULE_INIT" not in labels

    def test_partial_prefix_filter(self):
        items = get_snippet_completions(_default_ctx(partial="tcl-f"))
        labels = _labels(items)
        assert "Foreach" in labels
        assert "For Loop" in labels
        assert "Tcl Proc" not in labels
        assert "If Else" not in labels

    def test_partial_irule_prefix(self):
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                partial="irule-",
            )
        )
        labels = _labels(items)
        assert "iRule HTTP_REQUEST" in labels
        assert "iRule RULE_INIT" in labels
        # Tcl core templates don't match prefix
        assert "Tcl Proc" not in labels

    def test_collect_release_partial_suppression(self):
        """When only HTTP_REQUEST exists, snippet only contains HTTP_REQUEST_DATA."""
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                file_events=frozenset({"HTTP_REQUEST"}),
            )
        )
        collect = [i for i in items if i.label == "iRule Collect/Release"]
        assert len(collect) == 1
        body = collect[0].insert_text
        assert body is not None
        assert "when HTTP_REQUEST_DATA" in body
        assert "when HTTP_REQUEST {" not in body

    def test_collect_release_fully_suppressed(self):
        items = get_snippet_completions(
            _default_ctx(
                dialect="f5-irules",
                file_events=frozenset({"HTTP_REQUEST", "HTTP_REQUEST_DATA"}),
            )
        )
        labels = _labels(items)
        assert "iRule Collect/Release" not in labels


# Brace style tests


class TestBraceStyle:
    def test_kandr_brace_on_same_line(self):
        items = get_snippet_completions(
            _default_ctx(
                brace_style=BraceStyle.K_AND_R,
                partial="tcl-proc",
            )
        )
        proc = items[0]
        # Opening brace on same line as proc
        assert proc.insert_text is not None
        assert "} {" in proc.insert_text or "{" in proc.insert_text.split("\n")[0]


# Indent unit tests


class TestIndentUnit:
    def test_spaces_indent(self):
        items = get_snippet_completions(
            _default_ctx(
                indent_unit="    ",
                partial="tcl-proc",
            )
        )
        body = items[0].insert_text
        assert body is not None
        lines = body.split("\n")
        # Inner line should use 4 spaces
        inner_lines = [
            ln for ln in lines if ln and not ln.startswith("proc") and ln.strip() not in ("{", "}")
        ]
        assert any(ln.startswith("    ") for ln in inner_lines)

    def test_tab_indent(self):
        items = get_snippet_completions(
            _default_ctx(
                indent_unit="\t",
                partial="tcl-proc",
            )
        )
        body = items[0].insert_text
        assert body is not None
        lines = body.split("\n")
        inner_lines = [
            ln for ln in lines if ln and not ln.startswith("proc") and ln.strip() not in ("{", "}")
        ]
        assert any(ln.startswith("\t") for ln in inner_lines)

    def test_two_space_indent(self):
        items = get_snippet_completions(
            _default_ctx(
                indent_unit="  ",
                partial="tcl-class",
            )
        )
        body = items[0].insert_text
        assert body is not None
        # Should use 2-space indent for inner level
        assert "\n  " in body


# Variable choices tests


class TestVariableChoices:
    def test_foreach_with_scope_vars(self):
        items = get_snippet_completions(
            _default_ctx(
                scope_vars=["myList", "items", "data"],
                partial="tcl-foreach",
            )
        )
        body = items[0].insert_text
        assert body is not None
        # Should contain choice syntax with variable names
        assert "\\$myList" in body
        assert "\\$items" in body
        assert "\\$data" in body

    def test_foreach_without_scope_vars(self):
        items = get_snippet_completions(
            _default_ctx(
                scope_vars=[],
                partial="tcl-foreach",
            )
        )
        body = items[0].insert_text
        assert body is not None
        # Should use plain default
        assert "listVar" in body

    def test_dict_for_with_scope_vars(self):
        items = get_snippet_completions(
            _default_ctx(
                scope_vars=["myDict"],
                partial="tcl-dict-for",
            )
        )
        body = items[0].insert_text
        assert body is not None
        assert "\\$myDict" in body

    def test_var_choices_limited_to_10(self):
        """Ensure we don't overflow the choice list with too many variables."""
        many_vars = [f"var{i}" for i in range(20)]
        items = get_snippet_completions(
            _default_ctx(
                scope_vars=many_vars,
                partial="tcl-foreach",
            )
        )
        body = items[0].insert_text
        assert body is not None
        assert "\\$var0" in body
        assert "\\$var9" in body
        # var10+ should not appear in choices
        assert "\\$var10" not in body


# CompletionItem metadata tests


class TestCompletionItemMetadata:
    def test_snippet_kind(self):
        items = get_snippet_completions(_default_ctx())
        for item in items:
            assert item.kind == types.CompletionItemKind.Snippet

    def test_insert_text_format(self):
        items = get_snippet_completions(_default_ctx())
        for item in items:
            assert item.insert_text_format == types.InsertTextFormat.Snippet

    def test_sort_text_prefix(self):
        items = get_snippet_completions(_default_ctx())
        for item in items:
            assert item.sort_text is not None
            assert item.sort_text.startswith("Z0_")

    def test_filter_text_is_prefix(self):
        items = get_snippet_completions(_default_ctx())
        prefixes = {i.filter_text for i in items}
        assert "tcl-proc" in prefixes
        assert "tcl-if" in prefixes
        assert "tcl-foreach" in prefixes


# Integration with get_completions()


class TestSnippetIntegration:
    def test_snippets_appear_with_formatter_config(self):
        config = FormatterConfig()
        items = get_completions("", 0, 0, formatter_config=config)
        labels = [i.label for i in items]
        assert "Tcl Proc" in labels
        assert "Foreach" in labels

    def test_snippets_absent_without_formatter_config(self):
        items = get_completions("", 0, 0)
        snippet_items = [i for i in items if i.kind == types.CompletionItemKind.Snippet]
        assert len(snippet_items) == 0

    def test_snippets_sort_after_commands(self):
        config = FormatterConfig()
        items = get_completions("", 0, 0, formatter_config=config)
        cmd_sort = [
            i.sort_text
            for i in items
            if i.kind == types.CompletionItemKind.Function and i.sort_text
        ]
        snip_sort = [i.sort_text for i in items if i.kind == types.CompletionItemKind.Snippet]
        if cmd_sort and snip_sort:
            assert max(cmd_sort) < min(snip_sort)
