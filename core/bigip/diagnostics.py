"""BIG-IP configuration diagnostics for the LSP pipeline.

Converts :class:`BigipConfig` validation results into LSP-compatible
diagnostics that can be surfaced in the editor.
"""

from __future__ import annotations

from lsprotocol import types

from ..analysis.semantic_model import Severity
from ..common.lsp import to_lsp_range
from .model import BigipConfig
from .validator import validate_bigip_config

_SEVERITY_MAP = {
    Severity.ERROR: types.DiagnosticSeverity.Error,
    Severity.WARNING: types.DiagnosticSeverity.Warning,
    Severity.INFO: types.DiagnosticSeverity.Information,
    Severity.HINT: types.DiagnosticSeverity.Hint,
}


def get_bigip_diagnostics(
    config: BigipConfig,
    *,
    disabled_codes: set[str] | None = None,
) -> list[types.Diagnostic]:
    """Run BIG-IP validation and return LSP diagnostics.

    Parameters
    ----------
    config:
        Parsed BIG-IP configuration.
    disabled_codes:
        Diagnostic codes to suppress (e.g. ``{"BIGIP6006", "BIGIP6008"}``).
    """
    raw = validate_bigip_config(config)
    results: list[types.Diagnostic] = []
    for d in raw:
        if disabled_codes and d.code in disabled_codes:
            continue
        results.append(
            types.Diagnostic(
                range=to_lsp_range(d.range),
                message=d.message,
                severity=_SEVERITY_MAP.get(d.severity, types.DiagnosticSeverity.Warning),
                source="tcl-lsp",
                code=d.code or None,
            )
        )
    return results
