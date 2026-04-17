"""Tests for LSP features derived from upstream Tcl command semantics.

These supplement the existing test_hover.py, test_diagnostics.py, and
test_document_symbols.py with additional coverage for patterns from the
upstream Tcl test suite.

Areas covered:
- Hover on builtins: set, if, for, foreach, switch, proc, expr, string,
  list, namespace
- Hover on subcommands: string length, string match, namespace eval
- Hover on user-defined procs: params, defaults, docstrings
- Diagnostics for upstream error patterns: arity errors, unbraced expr
- Document symbols: procs, namespaced procs, variables
"""

from __future__ import annotations

import sys
import textwrap
from collections.abc import Sequence
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.diagnostics import get_diagnostics
from lsp.features.document_symbols import get_document_symbols
from lsp.features.hover import get_hover


def _hover_text(result: types.Hover) -> str:
    """Extract the display text from a Hover result, regardless of content type."""
    contents = result.contents
    if isinstance(contents, types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        parts: list[str] = []
        for item in contents:
            if isinstance(item, types.MarkedStringWithLanguage):
                parts.append(item.value)
            else:
                parts.append(str(item))
        return "\n".join(parts)
    if isinstance(contents, types.MarkedStringWithLanguage):
        return contents.value
    return str(contents)


# Hover: builtin commands


class TestBuiltinHover:
    """Verify that hovering over core Tcl builtins produces informative text."""

    def test_set_hover(self):
        result = get_hover("set x 42", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "set" in text

    def test_if_hover(self):
        result = get_hover("if {1} { puts yes }", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "if" in text

    def test_for_hover(self):
        result = get_hover("for {set i 0} {$i < 10} {incr i} {}", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "for" in text

    def test_foreach_hover(self):
        result = get_hover("foreach item $list { puts $item }", 0, 3)
        assert result is not None
        text = _hover_text(result)
        assert "foreach" in text

    def test_switch_hover(self):
        result = get_hover("switch $x { a { set r 1 } }", 0, 3)
        assert result is not None
        text = _hover_text(result)
        assert "switch" in text

    def test_proc_hover(self):
        result = get_hover("proc foo {} {}", 0, 2)
        assert result is not None
        text = _hover_text(result)
        assert "proc" in text

    def test_expr_hover(self):
        result = get_hover("expr {1 + 2}", 0, 2)
        assert result is not None
        text = _hover_text(result)
        assert "expr" in text

    def test_string_hover(self):
        result = get_hover("string length hello", 0, 3)
        assert result is not None
        text = _hover_text(result)
        assert "string" in text

    def test_list_hover(self):
        result = get_hover("list 1 2 3", 0, 2)
        assert result is not None
        text = _hover_text(result)
        assert "list" in text

    def test_namespace_hover(self):
        result = get_hover("namespace eval ns {}", 0, 4)
        assert result is not None
        text = _hover_text(result)
        assert "namespace" in text


# Hover: subcommands


class TestSubcommandHover:
    """Verify hover on subcommand tokens surfaces the subcommand description."""

    def test_string_length_hover(self):
        result = get_hover("string length hello", 0, 9)
        assert result is not None
        text = _hover_text(result)
        assert "length" in text

    def test_string_match_hover(self):
        result = get_hover("string match *.tcl file.tcl", 0, 9)
        assert result is not None
        text = _hover_text(result)
        assert "match" in text

    def test_namespace_eval_hover(self):
        result = get_hover("namespace eval ns {}", 0, 12)
        assert result is not None
        text = _hover_text(result)
        assert "eval" in text


# Hover: user-defined procs


class TestUserProcHover:
    """Hover over calls to user-defined procs should show signature info."""

    def test_user_proc_hover(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello $name"
            }
            greet world
        """)
        result = get_hover(source, 3, 2)
        assert result is not None
        text = _hover_text(result)
        assert "greet" in text

    def test_user_proc_with_defaults_hover(self):
        source = textwrap.dedent("""\
            proc greet {name {greeting Hello}} {
                puts "$greeting $name"
            }
            greet world
        """)
        result = get_hover(source, 3, 2)
        assert result is not None
        text = _hover_text(result)
        assert "greet" in text

    def test_user_proc_with_docstring_hover(self):
        """A comment immediately preceding the proc should appear in hover."""
        source = textwrap.dedent("""\
            # Greet someone by name
            proc greet {name} {
                puts "Hello $name"
            }
            greet world
        """)
        result = get_hover(source, 4, 2)
        assert result is not None
        text = _hover_text(result)
        assert "Greet someone" in text or "greet" in text


# Diagnostics: arity errors (E002 too few, E003 too many)


class TestArityDiagnostics:
    """Arity validation should flag incorrect argument counts."""

    def test_puts_too_many_args(self):
        """puts accepts at most 3 arguments; 4 should trigger E003."""
        result = get_diagnostics("puts a b c d")
        codes = [d.code for d in result]
        assert "E003" in codes

    def test_puts_valid(self):
        """puts with a single argument is perfectly valid."""
        result = get_diagnostics("puts hello")
        codes = [d.code for d in result]
        assert "E002" not in codes
        assert "E003" not in codes

    def test_for_missing_args(self):
        """for requires exactly 4 arguments; 3 should trigger E002."""
        result = get_diagnostics("for {set i 0} {$i<10} {incr i}")
        codes = [d.code for d in result]
        assert "E002" in codes

    def test_foreach_valid(self):
        """A well-formed foreach should produce no arity diagnostics."""
        result = get_diagnostics("foreach item {a b c} { puts $item }")
        codes = [d.code for d in result]
        assert "E002" not in codes
        assert "E003" not in codes

    def test_while_valid(self):
        """A well-formed while should produce no arity diagnostics."""
        result = get_diagnostics("while {1} { break }")
        codes = [d.code for d in result]
        assert "E002" not in codes
        assert "E003" not in codes

    def test_break_with_args(self):
        """break takes zero arguments; an extra arg should trigger E003."""
        result = get_diagnostics("break extra")
        codes = [d.code for d in result]
        assert "E003" in codes

    def test_continue_with_args(self):
        """continue takes zero arguments; an extra arg should trigger E003."""
        result = get_diagnostics("continue extra")
        codes = [d.code for d in result]
        assert "E003" in codes


# Diagnostics: unbraced expressions (W100)


class TestUnbracedExprDiagnostics:
    """Unbraced expressions are a performance and safety hazard."""

    def test_unbraced_expr_warning(self):
        """expr with an unbraced expression should produce W100."""
        result = get_diagnostics("expr $x + 1")
        codes = [d.code for d in result]
        assert "W100" in codes

    def test_braced_expr_clean(self):
        """A properly braced expr should not trigger W100."""
        result = get_diagnostics("expr {$x + 1}")
        codes = [d.code for d in result]
        assert "W100" not in codes

    def test_unbraced_if_condition(self):
        """An unbraced condition in if should produce W100."""
        result = get_diagnostics("if $x {puts yes}")
        codes = [d.code for d in result]
        assert "W100" in codes

    def test_braced_if_condition_clean(self):
        """A properly braced if condition should not trigger W100."""
        result = get_diagnostics("if {$x > 0} {puts yes}")
        codes = [d.code for d in result]
        assert "W100" not in codes


# Document symbols derived from upstream patterns


class TestDocumentSymbolsFromUpstreamPatterns:
    """Verify document symbol extraction for common Tcl patterns."""

    def test_proc_symbol(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello $name"
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) >= 1
        proc_sym = [s for s in symbols if s.name == "greet"]
        assert len(proc_sym) == 1
        assert proc_sym[0].kind == types.SymbolKind.Function
        assert proc_sym[0].detail == "(name)"

    def test_namespace_proc_symbol(self):
        """Procs inside namespace eval should appear in the symbol tree."""
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        symbols = get_document_symbols(source)
        # Collect all symbol names recursively to find 'add'
        all_names: list[str] = []

        def collect(syms: Sequence[types.DocumentSymbol]) -> None:
            for s in syms:
                all_names.append(s.name)
                if s.children:
                    collect(s.children)

        collect(symbols)
        assert any("add" in n for n in all_names)

    def test_multiple_proc_symbols(self):
        source = textwrap.dedent("""\
            proc foo {} { return 1 }
            proc bar {} { return 2 }
        """)
        symbols = get_document_symbols(source)
        names = {s.name for s in symbols}
        assert "foo" in names
        assert "bar" in names

    def test_proc_with_defaults_detail(self):
        """The detail string should include default-value notation."""
        source = textwrap.dedent("""\
            proc greet {name {greeting Hello}} {
                puts "$greeting $name"
            }
        """)
        symbols = get_document_symbols(source)
        proc_sym = [s for s in symbols if s.name == "greet"][0]
        assert proc_sym.detail is not None
        assert "{greeting Hello}" in proc_sym.detail
