from __future__ import annotations

from ._commands import _AnalyserCommandsMixin
from ._core import _AnalyserBase
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
    _AnalyserBase,
):
    "Single-pass Tcl analyser assembled from mixin groups."


__all__ = ["Analyser", "AnalyserSnapshot", "parse_param_list"]
