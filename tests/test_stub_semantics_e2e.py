# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""End-to-end semantic tests for inline dialect stubs.

These tests drive the full :func:`analyser.analyse` pipeline and
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

from analyser import analyse


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

    def test_subcommand_loop_stub_lowers_as_loop(self):
        # A loop declared on an ensemble subcommand (``coll foreach ...``)
        # is found via roles, past the leading subcommand word.
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub coll foreach {varName:var collection body:body} -loop
            # tcl-lsp: stubs-end
            set xs {1 2 3}
            coll foreach x $xs {
                set n $x
                puts $n
            }
        """
        assert _codes(src) == set()
        # And the body is still analysed: a genuine unset read fires.
        bad = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub coll foreach {varName:var collection body:body} -loop
            # tcl-lsp: stubs-end
            set xs {1 2 3}
            coll foreach x $xs {
                puts $never_set
            }
        """
        assert any("never_set" in m for m in _messages(bad, "W210"))

    def test_loop_stub_with_leading_option_lowers_as_loop(self):
        # The ``var collection body`` triple is located by role even when a
        # leading optional flag shifts the positions (even arg count).
        src = """\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub each_cell {?-filter? varName:var collection body:body} -loop
            # tcl-lsp: stubs-end
            set cells {a b c}
            each_cell -filter cell $cells {
                set n $cell
                puts $n
            }
        """
        assert _codes(src) == set()

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


class TestExternalStubFiles:
    """Stubs loaded from external ``.tcl.stubs`` files via the ambient scope."""

    _SRC = """\
        set cells [get_cells *]
        foreach_in_collection cell $cells {
            set name $cell
            puts $name
        }
        redirect -file out.rpt {
            set y 1
        }
    """

    @staticmethod
    def _write_stub_file(tmp_path):
        from compiler.registry.stub_comments import parse_stubs_file

        path = tmp_path / "eda.tcl.stubs"
        path.write_text(
            "stub foreach_in_collection {varName:var collection body:body} -loop\n"
            "stub get_cells {pattern:pattern} -pure\n"
            "stub redirect {?-file? target body:body}\n",
            encoding="utf-8",
        )
        cmd_stubs, _ = parse_stubs_file(path)
        return cmd_stubs

    def test_external_stubs_make_file_clean(self, tmp_path):
        from compiler.registry.stub_comments import ambient_stub_scope

        cmd_stubs = self._write_stub_file(tmp_path)
        with ambient_stub_scope(cmd_stubs):
            assert _codes(self._SRC) == set()

    def test_without_external_stubs_diagnostics_fire(self):
        # Negative control: same source, no ambient stubs in scope.
        codes = _codes(self._SRC)
        assert "W002" in codes  # redirect disabled
        assert "W210" in codes  # loop var / body not understood

    def test_external_stub_scope_does_not_leak(self, tmp_path):
        from compiler.registry.stub_comments import ambient_stub_scope

        cmd_stubs = self._write_stub_file(tmp_path)
        with ambient_stub_scope(cmd_stubs):
            assert _codes(self._SRC) == set()
        # Outside the scope the stubs are gone again.
        assert "W210" in _codes(self._SRC)

    def test_external_stub_does_not_emit_shadow_diagnostic(self, tmp_path):
        # An external stub's range points into the .tcl.stubs file, not the
        # document under analysis, so it must NOT raise W116/W117 against
        # the analysed source even when it shadows a built-in.
        from compiler.registry.stub_comments import ambient_stub_scope, parse_stubs_file

        path = tmp_path / "shadow.tcl.stubs"
        path.write_text("stub set {varName:var value}\n", encoding="utf-8")
        cmd_stubs, _ = parse_stubs_file(path)
        with ambient_stub_scope(cmd_stubs):
            codes = _codes("set x 1\nputs $x\n")
        assert "W116" not in codes


class TestExternalStubDiscovery:
    """Workspace init discovers and parses ``.tcl.stubs`` files."""

    def test_discovery_populates_workspace_stubs(self, tmp_path, monkeypatch):
        import server.state as state
        from server.workspace_init import _discover_workspace_stubs

        (tmp_path / "eda.tcl.stubs").write_text(
            "stub get_cells {pattern:pattern} -pure\n", encoding="utf-8"
        )
        nested = tmp_path / "lib"
        nested.mkdir()
        (nested / "more.tcl.stubs").write_text(
            "stub get_ports {pattern:pattern} -pure\n", encoding="utf-8"
        )

        monkeypatch.setattr(state, "workspace_stub_commands", [], raising=False)
        _discover_workspace_stubs([str(tmp_path)])

        names = {s.name for s in state.workspace_stub_commands}
        assert {"get_cells", "get_ports"} <= names

    def test_discovery_skips_vcs_dirs(self, tmp_path, monkeypatch):
        import server.state as state
        from server.workspace_init import _discover_workspace_stubs

        git = tmp_path / ".git"
        git.mkdir()
        (git / "ignored.tcl.stubs").write_text("stub should_not_load {x}\n", encoding="utf-8")

        monkeypatch.setattr(state, "workspace_stub_commands", [], raising=False)
        _discover_workspace_stubs([str(tmp_path)])

        names = {s.name for s in state.workspace_stub_commands}
        assert "should_not_load" not in names

    def test_discovery_spans_multiple_roots(self, tmp_path, monkeypatch):
        # Multi-root workspaces: every base directory is searched.
        import server.state as state
        from server.workspace_init import _discover_workspace_stubs

        root_a = tmp_path / "a"
        root_b = tmp_path / "b"
        root_a.mkdir()
        root_b.mkdir()
        (root_a / "a.tcl.stubs").write_text("stub cmd_a {x}\n", encoding="utf-8")
        (root_b / "b.tcl.stubs").write_text("stub cmd_b {x}\n", encoding="utf-8")

        monkeypatch.setattr(state, "workspace_stub_commands", [], raising=False)
        _discover_workspace_stubs([str(root_a), str(root_b)])

        names = {s.name for s in state.workspace_stub_commands}
        assert {"cmd_a", "cmd_b"} <= names


class TestShippedSampleIsClean:
    def test_inline_example_sample_has_no_diagnostics(self):
        sample = (
            Path(__file__).resolve().parent.parent / "samples" / "dialect_stubs_inline_example.tcl"
        )
        result = analyse(sample.read_text(encoding="utf-8"))
        assert result.diagnostics == [], [(d.code, d.message) for d in result.diagnostics]
