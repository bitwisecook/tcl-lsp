"""Generate XC-series diagnostics for inline editor feedback.

Walks the same IR as the translator but produces :class:`Diagnostic`
objects (from ``semantic_model``) for the LSP diagnostics pipeline.
"""

from __future__ import annotations

from ..analysis.semantic_model import Diagnostic, Severity
from .translator import translate_irule
from .xc_model import TranslationItem

# Severity mapping by diagnostic code prefix
_SEVERITY_MAP: dict[str, Severity] = {
    "XC1": Severity.HINT,  # XC100-series: translatable
    "XC2": Severity.INFO,  # XC200-series: partial / manual review
    "XC3": Severity.INFO,  # XC300-series: untranslatable
}


def _item_to_diagnostic(item: TranslationItem) -> Diagnostic | None:
    """Convert a TranslationItem to an LSP Diagnostic."""
    if item.irule_range is None:
        return None

    code = item.diagnostic_code
    if not code:
        return None

    # Determine severity from code prefix
    severity = Severity.INFO
    for prefix, sev in _SEVERITY_MAP.items():
        if code.startswith(prefix):
            severity = sev
            break

    message = item.xc_description
    if item.note:
        message += f" — {item.note}"

    return Diagnostic(
        range=item.irule_range,
        message=message,
        severity=severity,
        code=code,
    )


def get_xc_diagnostics(source: str) -> list[Diagnostic]:
    """Analyse an iRule and return XC translatability diagnostics.

    Returns a list of :class:`Diagnostic` objects with XC-series codes
    suitable for the LSP diagnostics pipeline.
    """
    result = translate_irule(source)

    diagnostics: list[Diagnostic] = []
    for item in result.items:
        diag = _item_to_diagnostic(item)
        if diag is not None:
            diagnostics.append(diag)

    return diagnostics
