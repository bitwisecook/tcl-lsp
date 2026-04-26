from __future__ import annotations

import logging

from ..semantic_model import (
    AnalysisResult,
    ClassDef,
    Diagnostic,
    ParamDef,
    ProcDef,
    Scope,
    Severity,
    VarDef,
)
from ._commands import _AnalyserCommandsMixin
from ._core import _AnalyserBase
from ._diag_branches import _AnalyserDiagBranchesMixin
from ._diag_channel import _AnalyserDiagChannelMixin
from ._diag_commands import _AnalyserDiagCommandsMixin
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

log = logging.getLogger(__name__)

# Optional Rust binding — imported best-effort so the Python path
# still loads cleanly when the wheel isn't installed.
try:  # pragma: no cover - import-time fallback
    from tcl_lsp_rust import analyser_analyse as _rust_analyse  # type: ignore[import-not-found]
except ImportError:  # pragma: no cover
    _rust_analyse = None  # type: ignore[assignment]


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
    _AnalyserDiagBranchesMixin,
    _AnalyserDiagChannelMixin,
    _AnalyserDiagIPMixin,
    _AnalyserDiagRacyMixin,
    _AnalyserDiagVarLifecycleMixin,
    _AnalyserBase,  # last — must follow all mixins that inherit from it (TYPE_CHECKING)
):
    "Single-pass Tcl analyser assembled from mixin groups."


_SEVERITY_MAP: dict[str, Severity] = {
    "hint": Severity.HINT,
    "suggestion": Severity.HINT,  # Python has no Suggestion variant; map to Hint.
    "warning": Severity.WARNING,
    "error": Severity.ERROR,
}


def _materialise_rust_analysis(source: str, raw: dict) -> AnalysisResult:
    """Convert the dict returned by ``tcl_lsp_rust.analyser_analyse``
    into an :class:`AnalysisResult`.

    Spans on the Rust side are ``(start, end)`` ``u32`` tuples; we
    resolve them to LSP :class:`Range` values via
    :func:`core.compiler.rust_spans.build_position_resolver`.

    The Rust analyser is partial — fields it doesn't populate
    (``stub_commands``, ``unknown_proc_info``, ``auto_path_entries``,
    ``regex_patterns``, …) default to empty.  Differential testing
    in C41f5 surfaces parity gaps over time; failures cleanly
    fall back to the Python path via the ``analyse`` shim.
    """
    from core.compiler.rust_spans import build_position_resolver

    _, range_at = build_position_resolver(source)
    result = AnalysisResult()

    def _params(params_raw: list[dict]) -> list[ParamDef]:
        return [
            ParamDef(
                name=p["name"],
                has_default=p["has_default"],
                default_value=p.get("default_value") or "",
            )
            for p in params_raw
        ]

    def _proc(p: dict) -> ProcDef:
        return ProcDef(
            name=p["name"],
            qualified_name=p["qualified_name"],
            params=_params(p["params"]),
            name_range=range_at(*p["name_range"]),
            body_range=range_at(*p["body_range"]),
            doc=p.get("doc") or "",
        )

    def _class(c: dict) -> ClassDef:
        return ClassDef(
            name=c["name"],
            qualified_name=c["qualified_name"],
            name_range=range_at(*c["name_range"]),
            body_range=range_at(*c["body_range"]),
        )

    def _var(v: dict) -> VarDef:
        return VarDef(
            name=v["name"],
            definition_range=range_at(*v["definition_range"]),
            warn_if_unused=v["warn_if_unused"],
        )

    def _scope(s: dict) -> Scope:
        scope = Scope(
            kind=s["kind"],
            name=s["name"],
            body_range=range_at(*s["body_range"]) if s.get("body_range") else None,
        )
        for name, v in s.get("variables", {}).items():
            scope.variables[name] = _var(v)
        for name, p in s.get("procs", {}).items():
            scope.procs[name] = _proc(p)
        for name, c in s.get("classes", {}).items():
            scope.classes[name] = _class(c)
        for child in s.get("children", []):
            scope.children.append(_scope(child))
        return scope

    result.global_scope = _scope(raw["global_scope"])
    for qname, p in raw.get("all_procs", {}).items():
        result.all_procs[qname] = _proc(p)
    for qname, c in raw.get("all_classes", {}).items():
        result.all_classes[qname] = _class(c)
    for qname, v in raw.get("all_variables", {}).items():
        result.all_variables[qname] = _var(v)
    for d in raw.get("diagnostics", []):
        result.diagnostics.append(
            Diagnostic(
                range=range_at(*d["range"]),
                message=d["message"],
                severity=_SEVERITY_MAP.get(d["severity"], Severity.WARNING),
                code=d["code"],
            )
        )
    return result


def analyse(source: str, cu=None) -> AnalysisResult:
    """Analyse `source` for the active dialect.

    Dispatches to the Rust port when ``TCL_LSP_RUST_ANALYSER`` is
    set to a truthy value AND no caller-provided
    :class:`CompilationUnit` is supplied (the Rust path builds its
    own CU internally; reusing a pre-built one isn't supported
    yet).  Default polarity is **OFF** — the Python implementation
    runs unless the env var is explicitly set.  Any exception
    raised by the Rust path is logged at DEBUG and the Python
    path runs as a safety net.
    """
    from core.common.dialect import active_dialect
    from core.compiler.rust_spans import rust_shim_enabled

    if (
        cu is None
        and _rust_analyse is not None
        and rust_shim_enabled("TCL_LSP_RUST_ANALYSER", default=False)
    ):
        try:
            return _materialise_rust_analysis(source, _rust_analyse(source, active_dialect()))
        except Exception:  # pragma: no cover - safety net
            log.debug("rust analyser failed, falling back to python", exc_info=True)
    return Analyser().analyse(source, cu=cu)


__all__ = ["Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list", "analyse"]
