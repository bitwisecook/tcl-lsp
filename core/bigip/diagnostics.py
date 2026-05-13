"""BIG-IP configuration diagnostics for the LSP pipeline.

Converts :class:`BigipConfig` validation results into LSP-compatible
diagnostics that can be surfaced in the editor.

Two layers:

- :func:`get_bigip_diagnostics` calls :func:`validate_bigip_config`
  for stanza-shape / value-format checks (BIGIP6xxx codes).  These
  always carry a :class:`Range` from the parser.
- :func:`get_bigip_lint_diagnostics` calls :func:`run_lint` for the
  cross-file rules (orphaned monitors, iRules referencing missing
  objects, partition mismatches, deprecated commands, …).  Lint
  ``Finding`` objects carry only ``full_path``; we map back to a
  range by looking the object up in the config and reading its
  stanza ``range``.
"""

from __future__ import annotations

from dataclasses import fields

from lsprotocol import types

from ..analysis.semantic_model import Range, Severity
from ..common.lsp import to_lsp_range
from .lint import Finding, run_lint
from .model import BigipConfig
from .validator import validate_bigip_config

_SEVERITY_MAP = {
    Severity.ERROR: types.DiagnosticSeverity.Error,
    Severity.WARNING: types.DiagnosticSeverity.Warning,
    Severity.INFO: types.DiagnosticSeverity.Information,
    Severity.HINT: types.DiagnosticSeverity.Hint,
}

# Lint :class:`Finding.severity` is a string; map to LSP enum so the
# editor renders the right squiggle colour.  Default to ``Warning`` for
# anything we don't recognise.
_LINT_SEVERITY_MAP = {
    "error": types.DiagnosticSeverity.Error,
    "warning": types.DiagnosticSeverity.Warning,
    "info": types.DiagnosticSeverity.Information,
    "hint": types.DiagnosticSeverity.Hint,
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


def get_bigip_lint_diagnostics(
    *,
    uri: str,
    source: str,
    config: BigipConfig,
    workspace_sources: dict[str, str] | None = None,
    workspace_configs: dict[str, BigipConfig] | None = None,
    disabled_codes: set[str] | None = None,
) -> list[types.Diagnostic]:
    """Run cross-file lint rules and return diagnostics anchored in *uri*.

    Pulls in every rule registered with :func:`core.bigip.lint.register`
    (orphaned monitors, missing iRule references, partition mismatches,
    deprecated commands, etc.).  Findings produced by walking the
    union of *workspace_configs* are filtered to objects defined in
    *config* — diagnostics need source-anchored ranges and we have a
    range only for objects whose stanza lives in *uri*.

    Cross-file findings (rule says "VS in fileA references missing pool
    that lives in fileB") show up on the file that owns the *referrer*
    object, which is the natural place for an editor to surface the
    warning.
    """
    sources = dict(workspace_sources or {})
    configs = dict(workspace_configs or {})
    sources.setdefault(uri, source)
    configs.setdefault(uri, config)

    findings = run_lint(sources=sources, configs=configs)
    return _findings_to_diagnostics(findings, config, disabled_codes=disabled_codes)


def _findings_to_diagnostics(
    findings: list[Finding],
    self_config: BigipConfig,
    *,
    disabled_codes: set[str] | None,
) -> list[types.Diagnostic]:
    """Convert lint findings into LSP diagnostics anchored on *self_config*.

    Findings whose ``full_path`` matches an object in *self_config*
    inherit that object's stanza ``range`` as the diagnostic range.
    Findings whose target lives in some other file are dropped — the
    editor will surface them when *that* file gets its own diagnostic
    pass.
    """
    paths_to_range = _index_object_ranges(self_config)
    results: list[types.Diagnostic] = []
    for finding in findings:
        if disabled_codes and finding.rule_id in disabled_codes:
            continue
        rng = paths_to_range.get(finding.full_path)
        if rng is None:
            continue
        results.append(
            types.Diagnostic(
                range=to_lsp_range(rng),
                message=finding.message,
                severity=_LINT_SEVERITY_MAP.get(finding.severity, types.DiagnosticSeverity.Warning),
                source="tcl-lsp",
                code=finding.rule_id,
            )
        )
    return results


def _index_object_ranges(config: BigipConfig) -> dict[str, Range]:
    """Return ``{full_path: Range}`` for every parsed object in *config*.

    Walks every dict-valued field by introspection so future kinds
    added to :class:`BigipConfig` are picked up automatically — same
    pattern :meth:`BigipConfig.merge` and the document-symbols
    feature use.  ``generic_objects`` is skipped because its keys
    aren't object full-paths (they're ``module::type::path`` join
    strings).
    """
    out: dict[str, Range] = {}
    for fld in fields(config):
        if fld.name == "generic_objects":
            continue
        kind_dict = getattr(config, fld.name)
        if not isinstance(kind_dict, dict):
            continue
        for full_path, obj in kind_dict.items():
            rng = getattr(obj, "range", None)
            if isinstance(rng, Range):
                out[full_path] = rng
    return out
