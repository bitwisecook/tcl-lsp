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

from __future__ import annotations

from ..semantic_model import AnalysisResult
from ._commands import _AnalyserCommandsMixin
from ._core import _AnalyserBase
from ._diag_brace_then_paren import _AnalyserDiagBraceThenParenMixin
from ._diag_branches import _AnalyserDiagBranchesMixin
from ._diag_channel import _AnalyserDiagChannelMixin
from ._diag_command_binding import _AnalyserDiagCommandBindingMixin
from ._diag_commands import _AnalyserDiagCommandsMixin
from ._diag_interval_bounds import _AnalyserDiagIntervalBoundsMixin
from ._diag_ip import _AnalyserDiagIPMixin
from ._diag_racy import _AnalyserDiagRacyMixin
from ._diag_var_command import _AnalyserDiagVarCommandMixin
from ._diag_var_lifecycle import _AnalyserDiagVarLifecycleMixin
from ._diagnostics import _AnalyserDiagsMixin
from ._handlers import _AnalyserHandlersMixin
from ._oo import _AnalyserOOMixin
from ._proc import _AnalyserProcMixin
from ._recovery import _AnalyserRecoveryMixin
from ._scope import _AnalyserScopeMixin
from ._snapshot import AnalyserSnapshot
from ._utils import parse_param_list


class Analyser(
    _AnalyserRecoveryMixin,
    _AnalyserScopeMixin,
    _AnalyserCommandsMixin,
    _AnalyserHandlersMixin,
    _AnalyserProcMixin,
    _AnalyserOOMixin,
    _AnalyserDiagsMixin,
    _AnalyserDiagCommandsMixin,
    _AnalyserDiagVarCommandMixin,
    _AnalyserDiagBraceThenParenMixin,
    _AnalyserDiagBranchesMixin,
    _AnalyserDiagCommandBindingMixin,
    _AnalyserDiagChannelMixin,
    _AnalyserDiagIntervalBoundsMixin,
    _AnalyserDiagIPMixin,
    _AnalyserDiagRacyMixin,
    _AnalyserDiagVarLifecycleMixin,
    _AnalyserBase,  # last — must follow all mixins that inherit from it (TYPE_CHECKING)
):
    "Single-pass Tcl analyser assembled from mixin groups."


def analyse(source: str, cu=None) -> AnalysisResult:
    return Analyser().analyse(source, cu=cu)


__all__ = ["Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list", "analyse"]
