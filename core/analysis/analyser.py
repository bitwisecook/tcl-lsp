"""Single-pass Tcl analyser — implementation split into _analyser/ package."""

from ._analyser import Analyser, AnalyserSnapshot, parse_param_list
from .semantic_model import AnalysisResult

__all__ = ["Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list"]


def analyse(source, cu=None):  # type: ignore[no-untyped-def]
    """Convenience wrapper: create an Analyser and run it on *source*."""
    return Analyser().analyse(source, cu=cu)
