from __future__ import annotations

_EXPORTS = frozenset(
    {"Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list", "analyse"}
)

__all__ = list(_EXPORTS)


def __getattr__(name: str):
    if name in _EXPORTS:
        from ._analyser import (  # noqa: PLC0415
            Analyser,
            AnalyserSnapshot,
            AnalysisResult,
            analyse,
            parse_param_list,
        )

        g = globals()
        g["Analyser"] = Analyser
        g["AnalyserSnapshot"] = AnalyserSnapshot
        g["AnalysisResult"] = AnalysisResult
        g["parse_param_list"] = parse_param_list
        g["analyse"] = analyse
        return g[name]
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
