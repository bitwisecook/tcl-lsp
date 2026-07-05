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

"""Coverage for the Tcl 9.1 dialect.

Tcl 9.1 is wired as a strict superset of Tcl 9.0: every 9.0 command,
subcommand, and option is available, plus the 9.1 additions (``unicode``
and ``timer`` ensembles, ``subst``'s positive option forms).  The
superset behaviour is provided by dialect inheritance in
``compiler.registry.dialects`` so individual command specs do not have
to re-tag themselves with ``tcl9.1``.
"""

from __future__ import annotations

from compiler.registry.command_registry import REGISTRY
from compiler.registry.dialect import Dialect, detect_dialect_from_source
from compiler.registry.dialects import (
    KNOWN_DIALECTS,
    dialect_membership,
    dialect_supports,
    dialects_since,
)
from compiler.registry.operators import operator_supports_dialect


def test_tcl91_is_a_known_dialect():
    assert "tcl9.1" in KNOWN_DIALECTS
    assert Dialect.TCL_9_1.value == "tcl9.1"


def test_tcl91_inherits_tcl90_membership():
    assert dialect_membership("tcl9.1") == frozenset({"tcl9.1", "tcl9.0"})
    assert dialect_membership("tcl9.0") == frozenset({"tcl9.0"})
    # A spec tagged only with tcl9.0 is available under tcl9.1, not vice versa.
    only_90 = frozenset({"tcl9.0"})
    assert dialect_supports("tcl9.1", only_90) is True
    assert dialect_supports("tcl9.0", frozenset({"tcl9.1"})) is False


def test_tcl91_command_set_is_superset_of_tcl90():
    cmds_90 = set(REGISTRY.command_names("tcl9.0"))
    cmds_91 = set(REGISTRY.command_names("tcl9.1"))
    assert cmds_90 <= cmds_91
    # The 9.1-only additions.
    assert cmds_91 - cmds_90 == {"timer", "unicode"}


def test_tcl91_new_commands_absent_in_tcl90():
    assert REGISTRY.get("unicode", "tcl9.0") is None
    assert REGISTRY.get("timer", "tcl9.0") is None
    assert REGISTRY.get("unicode", "tcl9.1") is not None
    assert REGISTRY.get("timer", "tcl9.1") is not None


def test_unicode_normalization_subcommands():
    assert REGISTRY.subcommand_names("unicode", "tcl9.1") == (
        "tonfc",
        "tonfd",
        "tonfkc",
        "tonfkd",
    )


def test_timer_ensemble_subcommands():
    assert REGISTRY.subcommand_names("timer", "tcl9.1") == (
        "at",
        "cancel",
        "idle",
        "in",
        "info",
        "sleep",
    )


def test_timer_in_at_require_script():
    # Tcl 9.1: `timer in delay unit script` / `timer at timepoint unit script`
    # require the script; the blocking form is `timer sleep`.
    spec = REGISTRY.get("timer", "tcl9.1")
    assert spec is not None
    for name in ("in", "at"):
        sub = spec.subcommands[name]
        assert sub.arity.min == 3
        assert sub.arity.max == 3


def test_subst_positive_options_gated_to_tcl91():
    spec = REGISTRY.get("subst", "tcl9.1")
    assert spec is not None
    options = spec.forms[0].options
    opts_91 = {o.name for o in options if o.supports_dialect("tcl9.1", spec.dialects)}
    opts_90 = {o.name for o in options if o.supports_dialect("tcl9.0", spec.dialects)}
    assert {"-backslashes", "-commands", "-variables"} <= opts_91
    assert opts_90.isdisjoint({"-backslashes", "-commands", "-variables"})


def test_string_comparison_operators_available_in_tcl91():
    for op in ("lt", "le", "gt", "ge"):
        assert operator_supports_dialect(op, "tcl9.1") is True
        assert operator_supports_dialect(op, "tcl8.6") is False


def test_dialects_since_includes_tcl91():
    assert "tcl9.1" in dialects_since("tcl9.0")
    assert "tcl9.1" in dialects_since("tcl8.5")
    assert "tcl9.0" not in dialects_since("tcl9.1")


def test_detect_tcl91_from_source():
    assert detect_dialect_from_source("# tcl-dialect: tcl9.1\n") == "tcl9.1"
    assert detect_dialect_from_source("#!/usr/bin/tclsh9.1\n") == "tcl9.1"
    assert detect_dialect_from_source("package require Tcl 9.1\n") == "tcl9.1"
