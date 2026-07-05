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

"""Regression test for GitHub issue #32 ("Superfluous type hints are inserted").

The user reported confusing ``: int`` / ``: str`` type annotations being
injected after ``set`` statements.  Inlay hints are an *opt-in* feature
(``features.inlayTypeHints`` → ``FeatureConfig.inlay_type_hints_enabled``,
default ``False``), so the fix is that the server must produce *no* inlay
hints unless the user explicitly enables them.

The existing coverage only asserts that the flag parses
(``tests/test_user_config.py``) and that ``get_inlay_hints`` renders hints when
called directly (``tests/test_inlay_hints.py``).  Neither exercises the actual
gate.  The toggle is honoured at the *handler* level: ``on_inlay_hint`` in
``server/server.py`` checks ``config_for_uri(uri).inlay_type_hints_enabled``
(and the parameter-hint sibling) and short-circuits to ``[]`` before ever
calling ``get_inlay_hints``.  This test drives that handler directly so a
regression that stops honouring the toggle (e.g. dropping the guard, or
defaulting the flag on) fails here.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest
from lsprotocol import types

import server.state as _state
from server.server import on_inlay_hint

# A snippet that mirrors the issue report: ``set`` statements whose RHS the
# analyser can type-infer, so the ``: int`` / ``: str`` annotations the user
# complained about are produced when the feature is enabled.
SOURCE = "set keep [expr {1 ? 1 : 0}]\nset name [file rootname foo]\n"

_URI = "file:///tmp/tcl-lsp-issue-32-inlay.tcl"

_RANGE = types.Range(
    start=types.Position(line=0, character=0),
    end=types.Position(line=10, character=0),
)


def _params() -> types.InlayHintParams:
    return types.InlayHintParams(
        text_document=types.TextDocumentIdentifier(uri=_URI),
        range=_RANGE,
    )


@pytest.fixture
def opened_doc():
    """Register the document with the workspace and restore global state after."""
    _state.workspace_state.open(_URI, SOURCE)
    original_type = _state.feature_config.inlay_type_hints_enabled
    original_param = _state.feature_config.inlay_parameter_hints_enabled
    try:
        yield
    finally:
        _state.feature_config.inlay_type_hints_enabled = original_type
        _state.feature_config.inlay_parameter_hints_enabled = original_param
        _state.workspace_state.close(_URI)


def test_inlay_hints_produced_when_enabled(opened_doc):
    """Positive control: with type hints on, the type hints from #32 appear."""
    _state.feature_config.inlay_type_hints_enabled = True
    hints = on_inlay_hint(_params())
    assert hints, "expected inlay hints when features.inlayTypeHints is enabled"
    # The specific ``: <type>`` annotations the issue complained about.
    type_labels = [h.label for h in hints if h.kind == types.InlayHintKind.Type]
    assert ": int" in type_labels
    assert ": str" in type_labels


def test_no_inlay_hints_when_disabled(opened_doc):
    """Regression for #32: with the feature off, the handler returns zero hints.

    This is the assertion the audit found missing.  Must fail if the toggle
    stops being honoured at the handler gate.
    """
    _state.feature_config.inlay_type_hints_enabled = False
    _state.feature_config.inlay_parameter_hints_enabled = False
    hints = on_inlay_hint(_params())
    assert hints == [], (
        "inlay hints must be suppressed when both inlay toggles are disabled; "
        f"got {[h.label for h in hints]}"
    )


def test_disabled_is_the_default(opened_doc):
    """Both families ship off by default, so an untouched config produces none.

    Guards against a regression that flips a default flag on (which would
    reintroduce the unwanted annotations for every user).
    """
    from server.feature_config import FeatureConfig

    assert FeatureConfig().inlay_type_hints_enabled is False
    assert FeatureConfig().inlay_parameter_hints_enabled is False
    # And the live process config the handler reads honours that default too,
    # unless a test explicitly opted in.
    _state.feature_config.inlay_type_hints_enabled = FeatureConfig().inlay_type_hints_enabled
    _state.feature_config.inlay_parameter_hints_enabled = (
        FeatureConfig().inlay_parameter_hints_enabled
    )
    assert on_inlay_hint(_params()) == []
