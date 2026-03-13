"""Tests for iRules ``call`` command support.

In iRules, procs defined with ``proc`` can only be invoked via ``call``,
not by direct name invocation as in standard Tcl.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity
from core.commands.registry.runtime import configure_signatures
from lsp.features.call_hierarchy import (
    incoming_calls,
    outgoing_calls,
    prepare_call_hierarchy,
)
from lsp.features.completion import get_completions
from lsp.features.definition import get_definition
from lsp.features.references import get_references
from lsp.features.rename import get_rename_edits

TEST_URI = "file:///test.irul"


def _irules(fn):
    """Decorator that sets the f5-irules dialect for a test."""

    def wrapper(*args, **kwargs):
        configure_signatures(dialect="f5-irules")
        return fn(*args, **kwargs)

    wrapper.__name__ = fn.__name__
    return wrapper


def _diag_with_code(source: str, code: str):
    configure_signatures(dialect="f5-irules")
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


# IRULE5005: direct proc invocation without ``call``


class TestIrule5005:
    """IRULE5005: direct proc invocation without ``call`` in iRules."""

    def test_direct_invocation_in_when_block(self):
        """myproc inside a when block should trigger IRULE5005."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                myproc hello
            }
        """)
        diags = _diag_with_code(src, "IRULE5005")
        assert len(diags) == 1
        assert "call myproc" in diags[0].message
        assert diags[0].severity == Severity.ERROR

    def test_call_invocation_no_warning(self):
        """call myproc should NOT trigger IRULE5005."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        diags = _diag_with_code(src, "IRULE5005")
        assert len(diags) == 0

    def test_direct_invocation_outside_when_no_warning(self):
        """Direct invocation outside when block (top-level) should not warn.

        This is the standard Tcl context (proc definitions etc.) where
        direct invocation is fine.
        """
        src = textwrap.dedent("""\
            proc helper {} { return 1 }
            helper
        """)
        diags = _diag_with_code(src, "IRULE5005")
        assert len(diags) == 0

    def test_fix_suggests_call_prefix(self):
        """The code fix should insert ``call`` before the proc name."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                myproc hello
            }
        """)
        diags = _diag_with_code(src, "IRULE5005")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text == "call myproc"

    def test_no_warning_for_builtin_commands(self):
        """Built-in commands should not trigger IRULE5005."""
        src = textwrap.dedent("""\
            when HTTP_REQUEST {
                set x 1
                log local0. "hi"
            }
        """)
        diags = _diag_with_code(src, "IRULE5005")
        assert len(diags) == 0

    def test_not_in_plain_tcl_dialect(self):
        """In plain Tcl dialect, direct proc invocation is fine."""
        configure_signatures(dialect="tcl")
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            myproc hello
        """)
        result = analyse(src)
        irule5005 = [d for d in result.diagnostics if d.code == "IRULE5005"]
        assert len(irule5005) == 0


# Analyser: ``call`` resolves the target proc


class TestCallResolution:
    """The analyser records a CommandInvocation for the proc target of ``call``."""

    @_irules
    def test_call_records_proc_invocation(self):
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        result = analyse(src)
        proc_invocations = [inv for inv in result.command_invocations if inv.name == "myproc"]
        assert len(proc_invocations) == 1
        assert proc_invocations[0].resolved_qualified_name == "::myproc"

    @_irules
    def test_call_arity_check_too_few(self):
        """call myproc (no args) when myproc needs 1 → arity error."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc
            }
        """)
        result = analyse(src)
        arity_diags = [
            d
            for d in result.diagnostics
            if "Too few arguments" in d.message and "myproc" in d.message
        ]
        assert len(arity_diags) == 1

    @_irules
    def test_call_arity_check_too_many(self):
        """call myproc a b c when myproc takes 1 → arity error."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc a b c
            }
        """)
        result = analyse(src)
        arity_diags = [
            d
            for d in result.diagnostics
            if "Too many arguments" in d.message and "myproc" in d.message
        ]
        assert len(arity_diags) == 1

    @_irules
    def test_call_arity_check_correct(self):
        """call myproc a when myproc takes 1 → no error."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        result = analyse(src)
        arity_diags = [
            d
            for d in result.diagnostics
            if "arguments" in d.message.lower() and "myproc" in d.message
        ]
        assert len(arity_diags) == 0


# References: find-references sees through ``call``


class TestCallReferences:
    @_irules
    def test_references_from_definition(self):
        """Find-references on the proc definition finds ``call`` site."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        # Cursor on "myproc" in the proc definition (line 0, col 5)
        refs = get_references(src, TEST_URI, 0, 5)
        ref_lines = {loc.range.start.line for loc in refs}
        assert 0 in ref_lines, "definition should be in references"
        assert 2 in ref_lines, "call site should be in references"

    @_irules
    def test_references_from_call_site(self):
        """Find-references from the proc name in ``call myproc``."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        # Cursor on "myproc" in "call myproc" (line 2, ~col 9)
        refs = get_references(src, TEST_URI, 2, 9)
        ref_lines = {loc.range.start.line for loc in refs}
        assert 0 in ref_lines, "definition should be found"
        assert 2 in ref_lines, "call site should be found"


# Go-to-definition: from ``call myproc`` jumps to proc def


class TestCallDefinition:
    @_irules
    def test_goto_definition_from_call_target(self):
        """Go-to-definition on the proc name in ``call myproc``."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        # Cursor on "myproc" in "call myproc" (line 2, ~col 9)
        locs = get_definition(src, TEST_URI, 2, 9)
        assert len(locs) >= 1
        assert locs[0].range.start.line == 0


# Completion: ``call `` offers proc names


class TestCallCompletion:
    @_irules
    def test_proc_names_after_call(self):
        """Typing ``call `` should complete with proc names."""
        # Use a literal space after "call " so the cursor is in argument position.
        src = (
            "proc myhelper {x} { return $x }\n"
            "proc another {} { return 1 }\n"
            "when HTTP_REQUEST {\n"
            "    call \n"
            "}\n"
        )
        # Cursor right after "call " on line 3, col 9
        items = get_completions(src, 3, 9)
        labels = {item.label for item in items}
        assert "myhelper" in labels
        assert "another" in labels

    @_irules
    def test_proc_names_filtered_by_partial(self):
        """Typing ``call my`` should filter to matching procs."""
        src = (
            "proc myhelper {x} { return $x }\n"
            "proc another {} { return 1 }\n"
            "when HTTP_REQUEST {\n"
            "    call my\n"
            "}\n"
        )
        # Cursor at end of "my" on line 3, col 11
        items = get_completions(src, 3, 11)
        labels = {item.label for item in items}
        assert "myhelper" in labels
        # "another" should be filtered out
        assert "another" not in labels


# Call hierarchy: sees through ``call``


class TestCallHierarchy:
    @_irules
    def test_incoming_calls_from_call_site(self):
        """Incoming calls for a proc should include ``call`` sites."""
        src = textwrap.dedent("""\
            proc outer {} {
                call inner
            }
            proc inner {} { return 1 }
            when HTTP_REQUEST {
                call outer
            }
        """)
        analysis = analyse(src)
        items = prepare_call_hierarchy(src, TEST_URI, 3, 6, analysis)
        if items:
            calls = incoming_calls(items[0], src, TEST_URI, analysis)
            caller_names = {c.from_.name for c in calls}
            assert "outer" in caller_names

    @_irules
    def test_outgoing_calls_from_proc(self):
        """Outgoing calls from a proc should include ``call`` targets."""
        src = textwrap.dedent("""\
            proc outer {} {
                call inner
            }
            proc inner {} { return 1 }
        """)
        analysis = analyse(src)
        items = prepare_call_hierarchy(src, TEST_URI, 0, 6, analysis)
        if items:
            calls = outgoing_calls(items[0], src, TEST_URI, analysis)
            callee_names = {c.to.name for c in calls}
            assert "inner" in callee_names


# Rename: rename works through ``call`` sites


class TestCallRename:
    @_irules
    def test_rename_proc_updates_call_site(self):
        """Renaming a proc should also update ``call myproc`` sites."""
        src = textwrap.dedent("""\
            proc myproc {x} { return $x }
            when HTTP_REQUEST {
                call myproc hello
            }
        """)
        edits = get_rename_edits(src, TEST_URI, 0, 5, "newname")
        assert edits is not None
        assert edits.changes is not None
        all_edits = edits.changes[TEST_URI]
        new_texts = {e.new_text for e in all_edits}
        assert "newname" in new_texts
        # Should have at least 2 edits: definition + call site
        assert len(all_edits) >= 2


# Command registry: ``call`` spec is correct


class TestCallSpec:
    def test_call_available_in_all_events(self):
        """call should not be restricted to RULE_INIT."""
        from core.commands.registry import REGISTRY

        configure_signatures(dialect="f5-irules")
        spec = REGISTRY.get("call", "f5-irules")
        assert spec is not None
        assert spec.event_requires is not None
        assert spec.event_requires.init_only is False

    def test_call_has_role_hints(self):
        """call should have arg_roles marking arg[0] as NAME."""
        from core.commands.registry.irules.call import CallCommand
        from core.commands.registry.signatures import ArgRole

        spec = CallCommand.spec()
        assert spec.arg_roles.get(0) == ArgRole.NAME

    def test_call_minimum_arity(self):
        """call requires at least 1 argument (the proc name)."""
        from core.commands.registry import REGISTRY

        configure_signatures(dialect="f5-irules")
        spec = REGISTRY.get("call", "f5-irules")
        assert spec is not None
        assert spec.validation is not None
        assert spec.validation.arity.min == 1


# Interprocedural: optimiser folds ``call`` the same as direct invocation


class TestCallInterprocedural:
    @_irules
    def test_call_pure_proc_is_folded(self):
        """[call one] should fold to 1, same as [one] in plain Tcl."""
        from core.compiler.optimiser import optimise_source

        src = textwrap.dedent("""\
            proc one {} { return 1 }
            when HTTP_REQUEST {
                set v [call one]
            }
        """)
        optimised, rewrites = optimise_source(src)
        # O103 folds [call one] → 1.  When v is unused, O126 (higher
        # priority) subsumes the entire statement, so O103 may not
        # appear in the final selected rewrites.
        assert any(r.code in ("O103", "O126") for r in rewrites)
        assert "set v [call one]" not in optimised

    @_irules
    def test_call_passthrough_folds_static_arg(self):
        """[call id $a] should fold when $a is constant."""
        from core.compiler.optimiser import optimise_source

        src = textwrap.dedent("""\
            proc id {x} { return $x }
            when HTTP_REQUEST {
                set a 7
                set v [call id $a]
            }
        """)
        optimised, rewrites = optimise_source(src)
        # O103 folds [call id $a] → 7.  O126 may subsume the
        # statement if v is unused, removing it entirely.
        assert any(r.code in ("O103", "O126") for r in rewrites)
        assert "set v [call id $a]" not in optimised

    @_irules
    def test_call_impure_proc_not_folded(self):
        """[call noisy] should NOT fold when the proc has side effects."""
        from core.compiler.optimiser import optimise_source

        src = textwrap.dedent("""\
            proc noisy {} { log local0. "hi"; return 1 }
            when HTTP_REQUEST {
                set v [call noisy]
            }
        """)
        _optimised, rewrites = optimise_source(src)
        assert not any(r.code == "O103" for r in rewrites)

    @_irules
    def test_call_in_call_graph(self):
        """call myproc should appear in the interprocedural call graph."""
        from core.compiler.interprocedural import analyse_interprocedural_source

        src = textwrap.dedent("""\
            proc helper {} { return 1 }
            proc outer {} {
                call helper
            }
        """)
        interproc = analyse_interprocedural_source(src)
        outer_summary = interproc.procedures.get("::outer")
        assert outer_summary is not None
        assert "::helper" in outer_summary.calls
