"""Large cold builds (``_analyse_document_fresh`` in the subprocess pool) must
produce the same workspace-aware basic diagnostics as the in-thread phase1.

Regression for the cold path passing ``workspace_context=None`` and the default
``line_ending="\\n"``, which made a large file's W118 (line-ending) and
W123 (unknown-command, workspace-proc-filtered) diagnostics diverge from a small
file's in-thread build.  The fix forwards the line-ending config and the (small,
picklable) workspace diagnostic context into the subprocess worker.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.semantic_model import WorkspaceDiagnosticContext
from server.workspace.document_state import _analyse_document_fresh


def _codes(result: dict, code: str) -> list:
    return [d for d in result.get("basic_diags", []) if d.code == code]


# --- line_ending (W118) ---------------------------------------------------


def test_cold_build_honours_crlf_line_ending():
    # A CRLF file under a CRLF line-ending config must NOT fire W118 — the
    # endings match the configured style.  Before the fix the cold path always
    # used the default "\n" and wrongly flagged the file.
    src = "set a 1\r\nset b 2\r\n"
    result = _analyse_document_fresh(
        source=src,
        version=1,
        line_length=120,
        dialect="tcl8.6",
        uri="file:///crlf.tcl",
        line_ending="\r\n",
    )
    assert _codes(result, "W118") == []


def test_cold_build_default_lf_flags_crlf():
    # Control: the same CRLF file under the default "\n" config DOES fire W118 —
    # proves the check is active and the parameter is what suppresses it.
    src = "set a 1\r\nset b 2\r\n"
    result = _analyse_document_fresh(
        source=src,
        version=1,
        line_length=120,
        dialect="tcl8.6",
        uri="file:///crlf.tcl",
        line_ending="\n",
    )
    assert len(_codes(result, "W118")) == 1


# --- workspace_context (W123) --------------------------------------------


_UNKNOWN_CMD_SRC = "myWorkspaceProc foo bar\n"


def test_cold_build_suppresses_w123_for_workspace_proc():
    # An unknown command that IS a workspace proc must be suppressed when the
    # workspace context is forwarded (matches in-thread phase1).
    ctx = WorkspaceDiagnosticContext(
        workspace_proc_names=frozenset({"::myWorkspaceProc"}),
        workspace_package_names=frozenset(),
        package_names_by_uri={},
        source_graph={},
        alias_names_by_uri={},
    )
    result = _analyse_document_fresh(
        source=_UNKNOWN_CMD_SRC,
        version=1,
        line_length=120,
        dialect="tcl8.6",
        uri="file:///caller.tcl",
        disabled_diagnostics=set(),
        workspace_context=ctx,
    )
    w123 = [d for d in _codes(result, "W123") if "myWorkspaceProc" in (d.message or "")]
    assert w123 == [], "W123 for a known workspace proc must be suppressed"


def test_cold_build_without_context_still_flags_w123():
    # Control: with no workspace context (the old cold-path behaviour) the same
    # unknown command is flagged — proving the suppression comes from the
    # forwarded context, not from the command being intrinsically known.
    result = _analyse_document_fresh(
        source=_UNKNOWN_CMD_SRC,
        version=1,
        line_length=120,
        dialect="tcl8.6",
        uri="file:///caller.tcl",
        disabled_diagnostics=set(),
        workspace_context=None,
    )
    w123 = [d for d in _codes(result, "W123") if "myWorkspaceProc" in (d.message or "")]
    assert len(w123) == 1, "an unknown command with no workspace context must fire W123"
