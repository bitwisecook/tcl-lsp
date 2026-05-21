"""End-to-end semantic tests for inline dialect stubs.

These tests drive the full :func:`core.analysis.analyse` pipeline and
assert on the *meaning* the stubs give a file — not just that a stub is
parsed.  Each behaviour is paired with a negative control (the same
source without the stub) so the assertions prove the stub is what does
the work, and that genuine problems are still reported when a stub
declares the command's shape.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis import analyse


def _codes(source: str) -> set[str]:
    return {d.code for d in analyse(textwrap.dedent(source)).diagnostics}


def _messages(source: str, code: str) -> list[str]:
    return [d.message for d in analyse(textwrap.dedent(source)).diagnostics if d.code == code]


class TestLoopStubSemantics:
    """A ``-loop`` stub turns a custom command into a real, analysed loop."""

    _LOOP_BODY = """\
        # tcl-lsp: stubs-begin
        # tcl-lsp: stub each_cell {varName:var collection body:body} -loop
        # tcl-lsp: stubs-end
        set cells {a b c}
        each_cell cell $cells {
            set name $cell
            puts $name
        }
    """

    def test_loop_body_is_analysed_in_iteration_order(self):
        # `name` is set before it is read inside the body, so once the
        # body is understood as a script there is no read-before-set.
        assert "W210" not in _codes(self._LOOP_BODY)

    def test_loop_variable_is_defined_by_the_stub(self):
        # `$cell` is the loop variable; the loop lowering defines it, so
        # reading it inside the body is not a read-before-set.
        rbs = _messages(self._LOOP_BODY, "W210")
        assert not any("cell" in m for m in rbs)

    def test_without_stub_loop_var_and_body_are_unknown(self):
        # Negative control: drop the stub block and the same call is an
        # unknown command whose "body" is opaque, so both the loop
        # variable and the body-local variable look read-before-set.
        no_stub = """\
            set cells {a b c}
            each_cell cell $cells {
                set name $cell
                puts $name
            }
        """
        codes = _codes(no_stub)
        assert "W123" in codes  # unknown command
        rbs = _messages(no_stub, "W210")
        assert any("cell" in m for m in rbs)
        assert any("name" in m for m in rbs)

    def test_body_is_really_analysed_not_merely_suppressed(self):
        # A genuine read-before-set inside the loop body must still fire —
        # the body is analysed as a script, not treated as an opaque
        # region where diagnostics are silenced.
        bad = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub each_cell {varName:var collection body:body} -loop
            # tcl-lsp: stubs-end
            set cells {a b c}
            each_cell cell $cells {
                puts $never_set
            }
        """
        assert any("never_set" in m for m in _messages(bad, "W210"))

    def test_foreach_in_collection_stub_lowers_as_loop(self):
        # The canonical EDA example: the documented foreach_in_collection
        # stub must behave like a loop regardless of the active dialect.
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
            # tcl-lsp: stub get_cells {pattern:pattern} -pure
            # tcl-lsp: stub get_attribute {object attrName:name} -pure
            # tcl-lsp: stubs-end
            set all_cells [get_cells *]
            foreach_in_collection cell $all_cells {
                set name [get_attribute $cell full_name]
                puts "Cell: $name"
            }
        """
        assert _codes(src) == set()


class TestDisabledCommandStubSemantics:
    """A stub re-enables a command disabled in the active dialect (W002)."""

    def test_stub_suppresses_w002_for_disabled_command(self):
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub redirect {?-variable? ?-file? target body:body}
            # tcl-lsp: stubs-end
            redirect -file timing.rpt {
                set y 1
            }
        """
        assert "W002" not in _codes(src)

    def test_without_stub_disabled_command_is_flagged(self):
        # Negative control: `redirect` is a known command that is not in
        # the default tcl dialect, so without a stub it is reported as
        # disabled.
        no_stub = """\
            redirect -file timing.rpt {
                set y 1
            }
        """
        assert "W002" in _codes(no_stub)

    def test_stub_suppression_does_not_leak_to_other_files(self):
        # The overlay is scoped to the analysis pass that declared it;
        # analysing a stubbed file must not silence W002 in a later,
        # unrelated file that calls the same command without a stub.
        stubbed = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub redirect {?-file? target body:body}
            # tcl-lsp: stubs-end
            redirect -file out.rpt {set y 1}
        """
        analyse(textwrap.dedent(stubbed))
        assert "W002" in _codes("redirect -file out.rpt {set y 1}\n")


class TestBodyStubRecursion:
    """Body args of a stub are descended into for command-level checks."""

    def test_inner_command_of_non_loop_body_is_checked(self):
        # A plain ``body:body`` stub (no -loop) is not a control-flow loop,
        # but its body is still walked: an unknown command inside it is
        # reported, proving the body is analysed rather than ignored.
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub redirect {?-file? target body:body}
            # tcl-lsp: stubs-end
            redirect -file out.rpt {
                totally_unknown_command foo bar
            }
        """
        assert any("totally_unknown_command" in m for m in _messages(src, "W123"))

    def test_barrier_stub_body_is_not_silenced(self):
        # A ``-barrier`` body executes in a dynamic scope, so a read of a
        # variable that is never set is still surfaced — the body is not
        # treated as an opaque region where diagnostics are dropped.
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub my_eval {script:body} -barrier
            # tcl-lsp: stubs-end
            my_eval {
                puts $never_set_anywhere
            }
        """
        assert any("never_set_anywhere" in m for m in _messages(src, "W210"))


class TestShippedSampleIsClean:
    def test_inline_example_sample_has_no_diagnostics(self):
        sample = (
            Path(__file__).resolve().parent.parent
            / "samples"
            / "dialect_stubs_inline_example.tcl"
        )
        result = analyse(sample.read_text(encoding="utf-8"))
        assert result.diagnostics == [], [
            (d.code, d.message) for d in result.diagnostics
        ]
