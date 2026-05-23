"""Diagnostics provider -- converts analysis diagnostics to LSP diagnostics."""

from __future__ import annotations

import logging
import time

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import (
    _FILE_SUPPRESS_KEY,
    _NOQA_ALL,
    AnalysisResult,
    CodeFix,
    Diagnostic,
    Range,
    Severity,
    WorkspaceDiagnosticContext,
)
from analyser.tk_detection import has_tk_require
from compiler.compilation_unit import CompilationUnit, ensure_compilation_unit
from compiler.gvn import RedundantComputation, find_redundant_computations
from compiler.irules_flow import IrulesFlowWarning, find_irules_flow_warnings
from compiler.optimiser import Optimisation, find_optimisations
from compiler.registry import REGISTRY
from compiler.registry.dialect import active_dialect
from compiler.shimmer import ShimmerWarning, ThunkingWarning, find_shimmer_warnings
from compiler.taint import (
    TaintWarning,
    find_taint_warnings,
)
from server._lsp_conv import to_lsp_range
from shared.codes import diag
from shared.tokens import SourcePosition

log = logging.getLogger(__name__)

# Short names: d = Diagnostic.

_SEVERITY_MAP = {
    Severity.ERROR: types.DiagnosticSeverity.Error,
    Severity.WARNING: types.DiagnosticSeverity.Warning,
    Severity.INFO: types.DiagnosticSeverity.Information,
    Severity.HINT: types.DiagnosticSeverity.Hint,
}

_SHIMMER_SEVERITY = {
    "S100": types.DiagnosticSeverity.Information,
    "S101": types.DiagnosticSeverity.Warning,
    "S102": types.DiagnosticSeverity.Warning,
}

_TAINT_SEVERITY = {
    "T100": types.DiagnosticSeverity.Warning,
    "T101": types.DiagnosticSeverity.Warning,
    "T102": types.DiagnosticSeverity.Warning,
    "T103": types.DiagnosticSeverity.Warning,
    "T106": types.DiagnosticSeverity.Information,
    "IRULE3001": types.DiagnosticSeverity.Warning,
    "IRULE3002": types.DiagnosticSeverity.Warning,
    "IRULE3003": types.DiagnosticSeverity.Warning,
    "IRULE3004": types.DiagnosticSeverity.Warning,
    "IRULE3101": types.DiagnosticSeverity.Warning,
    "IRULE3103": types.DiagnosticSeverity.Information,
    "W201": types.DiagnosticSeverity.Hint,
}

_IRULES_FLOW_SEVERITY = {
    "IRULE1005": types.DiagnosticSeverity.Warning,
    "IRULE1006": types.DiagnosticSeverity.Warning,
    "IRULE1007": types.DiagnosticSeverity.Error,
    "IRULE1008": types.DiagnosticSeverity.Error,
    "IRULE1201": types.DiagnosticSeverity.Warning,
    "IRULE1202": types.DiagnosticSeverity.Warning,
    # IRULE2102 retired — subsumed by O105 (GVN/CSE).
    "IRULE4004": types.DiagnosticSeverity.Information,
    "IRULE5002": types.DiagnosticSeverity.Warning,
    "IRULE5004": types.DiagnosticSeverity.Warning,
}


def _is_suppressed(
    code: str,
    line: int,
    suppressed_lines: dict[int, frozenset[str]],
) -> bool:
    """Return True if *code* is suppressed by an inline ``# noqa`` or a
    top-of-file ``# tcl-lsp: disable=...`` directive."""
    file_codes = suppressed_lines.get(_FILE_SUPPRESS_KEY)
    if file_codes is not None and (
        file_codes is _NOQA_ALL or "*" in file_codes or code in file_codes
    ):
        return True
    codes = suppressed_lines.get(line)
    if codes is None:
        return False
    return codes is _NOQA_ALL or "*" in codes or code in codes


# Maps diagnostic codes to documentation URLs.
_DIAGNOSTIC_DOCS: dict[str, str] = {
    "W123": "https://tcl-lsp.dev/docs/kcs/unknown-command-resolution",
}


def _to_lsp_diagnostic(d: Diagnostic, *, uri: str | None = None) -> types.Diagnostic:
    code_desc = None
    if d.code and d.code in _DIAGNOSTIC_DOCS:
        code_desc = types.CodeDescription(href=_DIAGNOSTIC_DOCS[d.code])
    related: list[types.DiagnosticRelatedInformation] | None = None
    if uri and d.related_ranges:
        related = [
            types.DiagnosticRelatedInformation(
                location=types.Location(uri=uri, range=to_lsp_range(r)),
                message=msg,
            )
            for r, msg in d.related_ranges
        ]
    return types.Diagnostic(
        range=to_lsp_range(d.range),
        message=d.message,
        severity=_SEVERITY_MAP.get(d.severity, types.DiagnosticSeverity.Error),
        source="tcl-lsp",
        code=d.code or None,
        code_description=code_desc,
        related_information=related or None,
    )


def _optimisation_to_diagnostic(
    opt: Optimisation,
    *,
    group_edits: list[dict] | None = None,
    related: list[types.DiagnosticRelatedInformation] | None = None,
    message_suffix: str = "",
) -> types.Diagnostic:
    """Convert an optimiser suggestion to an LSP diagnostic with replacement data."""
    data: dict = {
        "replacement": opt.replacement,
        "startOffset": opt.range.start.offset,
        "endOffset": opt.range.end.offset,
    }
    if group_edits is not None:
        data["groupEdits"] = group_edits
    if opt.group is not None:
        data["group"] = opt.group
    if opt.hint_only:
        data["hintOnly"] = True
    return types.Diagnostic(
        range=to_lsp_range(opt.range),
        message=opt.message + message_suffix,
        severity=types.DiagnosticSeverity.Information,
        source="tcl-lsp",
        code=opt.code,
        data=data,
        related_information=related,
    )


def _shimmer_to_diagnostic(
    w: ShimmerWarning | ThunkingWarning,
    uri: str | None = None,
) -> types.Diagnostic:
    """Convert a shimmer/thunking warning to an LSP diagnostic."""
    related: list[types.DiagnosticRelatedInformation] | None = None
    if uri and w.related_ranges:
        related = [
            types.DiagnosticRelatedInformation(
                location=types.Location(uri=uri, range=to_lsp_range(r)),
                message=msg,
            )
            for r, msg in w.related_ranges
        ]
    return types.Diagnostic(
        range=to_lsp_range(w.range),
        message=w.message,
        severity=_SHIMMER_SEVERITY.get(w.code, types.DiagnosticSeverity.Information),
        source="tcl-lsp",
        code=w.code,
        related_information=related or None,
    )


def _taint_to_diagnostic(
    w: TaintWarning,
) -> types.Diagnostic:
    """Convert a taint warning to an LSP diagnostic."""
    return types.Diagnostic(
        range=to_lsp_range(w.range),
        message=w.message,
        severity=_TAINT_SEVERITY.get(w.code, types.DiagnosticSeverity.Warning),
        source="tcl-lsp",
        code=w.code,
    )


def _irules_flow_to_diagnostic(w: IrulesFlowWarning) -> types.Diagnostic:
    """Convert an iRules flow warning to an LSP diagnostic."""
    return types.Diagnostic(
        range=to_lsp_range(w.range),
        message=w.message,
        severity=_IRULES_FLOW_SEVERITY.get(w.code, types.DiagnosticSeverity.Warning),
        source="tcl-lsp",
        code=w.code,
    )


def _gvn_to_diagnostic(w: RedundantComputation) -> types.Diagnostic:
    """Convert a GVN redundancy warning to an LSP diagnostic."""
    return types.Diagnostic(
        range=to_lsp_range(w.range),
        message=w.message,
        severity=types.DiagnosticSeverity.Information,
        source="tcl-lsp",
        code=w.code,
    )


def _brace_expr_perf_hint(d: Diagnostic) -> types.Diagnostic:
    """Emit an optimisation info paired with W100 unbraced-expression warnings."""
    return types.Diagnostic(
        range=to_lsp_range(d.range),
        message=(
            "Brace expression text (for example, `expr {...}` / `if {...}`) to pass "
            "a single static argument, enabling bytecode compilation and avoiding "
            "per-evaluation substitution/parsing overhead."
        ),
        severity=types.DiagnosticSeverity.Information,
        source="tcl-lsp",
        code="O111",
    )


# Source-level style checks (W111, W112)


@diag("W111", "Line exceeds maximum length (see `tclLsp.style.lineLength`).", section="warning")
def _check_line_length(
    source: str, max_length: int, *, lines: list[str] | None = None
) -> list[Diagnostic]:
    """W111: Flag lines exceeding *max_length* characters."""
    if lines is None:
        lines = source.split("\n")
    diagnostics: list[Diagnostic] = []
    offset = 0
    for lineno, line in enumerate(lines):
        # Strip \r so CRLF line endings don't inflate the count.
        length = len(line.rstrip("\r"))
        if length > max_length:
            diagnostics.append(
                Diagnostic(
                    range=Range(
                        start=SourcePosition(line=lineno, character=0, offset=offset),
                        end=SourcePosition(
                            line=lineno, character=length - 1, offset=offset + length - 1
                        ),
                    ),
                    message=f"Line exceeds {max_length} characters ({length} characters)",
                    severity=Severity.WARNING,
                    code="W111",
                )
            )
        offset += length + 1  # +1 for the \n
    return diagnostics


@diag("W112", "Trailing whitespace.", section="warning")
def _check_trailing_whitespace(source: str, *, lines: list[str] | None = None) -> list[Diagnostic]:
    """W112: Flag trailing whitespace with an auto-fix to remove it."""
    if lines is None:
        lines = source.split("\n")
    diagnostics: list[Diagnostic] = []
    offset = 0
    for lineno, line in enumerate(lines):
        # Strip \r so CRLF line endings aren't flagged as trailing whitespace.
        line_no_cr = line.rstrip("\r")
        stripped = line_no_cr.rstrip()
        if len(stripped) < len(line_no_cr):
            ws_start = len(stripped)
            ws_end = len(line_no_cr)
            diagnostics.append(
                Diagnostic(
                    range=Range(
                        start=SourcePosition(
                            line=lineno, character=ws_start, offset=offset + ws_start
                        ),
                        end=SourcePosition(
                            line=lineno, character=ws_end - 1, offset=offset + ws_end - 1
                        ),
                    ),
                    message="Trailing whitespace",
                    severity=Severity.HINT,
                    code="W112",
                    fixes=(
                        CodeFix(
                            range=Range(
                                start=SourcePosition(
                                    line=lineno,
                                    character=ws_start,
                                    offset=offset + ws_start,
                                ),
                                end=SourcePosition(
                                    line=lineno,
                                    character=ws_end - 1,
                                    offset=offset + ws_end - 1,
                                ),
                            ),
                            new_text="",
                            description="Remove trailing whitespace",
                        ),
                    ),
                )
            )
        offset += len(line) + 1  # +1 for the \n
    return diagnostics


@diag("W118", "Inconsistent line endings.", section="warning")
def _check_line_endings(source: str, *, expected: str = "\n") -> list[Diagnostic]:
    """W118: Flag files with line endings that don't match the configured style.

    Scans the source for ``\\r\\n``, ``\\r``, and ``\\n`` occurrences and emits
    a single file-level diagnostic when the file contains line endings that
    differ from *expected* (``"\\n"``, ``"\\r\\n"``, or ``"\\r"``).
    """
    # Count each line-ending style.
    crlf = source.count("\r\n")
    # Lone \r = total \r minus those that are part of \r\n.
    cr = source.count("\r") - crlf
    lf = source.count("\n") - crlf

    # Build the set of endings actually present.
    present: dict[str, int] = {}
    if lf:
        present["\n"] = lf
    if crlf:
        present["\r\n"] = crlf
    if cr:
        present["\r"] = cr

    if not present:
        return []

    # Only fire when there are endings that differ from the expected style.
    unexpected = {k: v for k, v in present.items() if k != expected}
    if not unexpected:
        return []

    labels = {"\n": "LF", "\r\n": "CRLF", "\r": "CR"}
    expected_label = labels.get(expected, repr(expected))

    if len(present) > 1:
        parts = ", ".join(f"{labels.get(k, repr(k))} ({v})" for k, v in present.items())
        message = f"Mixed line endings: {parts}; expected {expected_label}"
    else:
        (actual_ending,) = unexpected
        actual_label = labels.get(actual_ending, repr(actual_ending))
        count = unexpected[actual_ending]
        message = f"File uses {actual_label} line endings ({count}); expected {expected_label}"

    # File-level diagnostic anchored at (0, 0).
    pos = SourcePosition(line=0, character=0, offset=0)
    return [
        Diagnostic(
            range=Range(start=pos, end=pos),
            message=message,
            severity=Severity.HINT,
            code="W118",
        )
    ]


@diag("W115", "Backslash-newline in comment silently swallows the next line.", section="warning")
def _check_comment_continuation(source: str, *, lines: list[str] | None = None) -> list[Diagnostic]:
    """W115: Flag backslash-newline continuation in comments.

    In Tcl, a ``\\`` immediately before a newline inside a comment causes the
    next line to be silently swallowed into the comment.  This is almost always
    unintentional and can hide live code.  The quick-fix converts the continued
    comment into separate per-line ``#`` comments.
    """
    diagnostics: list[Diagnostic] = []
    if lines is None:
        lines = source.split("\n")
    offsets: list[int] = []
    running = 0
    for line in lines:
        offsets.append(running)
        running += len(line) + 1

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.lstrip()
        if not (stripped.startswith("#") and line.rstrip("\r").endswith("\\")):
            i += 1
            continue

        # Found a comment with backslash continuation.
        indent = line[: len(line) - len(stripped)]
        block_start = i
        block_lines: list[str] = [line]

        # Gather continuation lines.
        j = i + 1
        while j < len(lines):
            block_lines.append(lines[j])
            if lines[j].rstrip("\r").endswith("\\"):
                j += 1
            else:
                break
        else:
            # Ran off the end of the file -- last continuation is empty/EOF.
            pass

        block_end = min(j, len(lines) - 1)

        # Build the replacement text: per-line comments.
        fixed: list[str] = []
        for k, orig in enumerate(block_lines):
            raw = orig.rstrip("\r")
            if k == 0:
                # First line: remove trailing backslash.
                fixed.append(raw[:-1].rstrip())
            else:
                content = raw.lstrip()
                if raw.endswith("\\") and k < len(block_lines) - 1:
                    content = content[:-1].rstrip()
                if content.startswith("#"):
                    # Already looks like a comment -- keep indent only.
                    fixed.append(f"{indent}{content}")
                else:
                    fixed.append(f"{indent}# {content}")
        new_text = "\n".join(fixed)

        # Diagnostic range: from start of first line to end of last line.
        end_line_text = lines[block_end]
        diagnostics.append(
            Diagnostic(
                range=Range(
                    start=SourcePosition(
                        line=block_start, character=0, offset=offsets[block_start]
                    ),
                    end=SourcePosition(
                        line=block_end,
                        character=max(0, len(end_line_text) - 1),
                        offset=offsets[block_end] + max(0, len(end_line_text) - 1),
                    ),
                ),
                message=("Backslash-newline in comment silently swallows the next line"),
                severity=Severity.WARNING,
                code="W115",
                fixes=(
                    CodeFix(
                        range=Range(
                            start=SourcePosition(
                                line=block_start, character=0, offset=offsets[block_start]
                            ),
                            end=SourcePosition(
                                line=block_end,
                                character=max(0, len(end_line_text) - 1),
                                offset=offsets[block_end] + max(0, len(end_line_text) - 1),
                            ),
                        ),
                        new_text=new_text,
                        description="Convert to per-line comments",
                    ),
                ),
            )
        )

        i = block_end + 1
    return diagnostics


def compute_style_diagnostics_for_range(
    source: str,
    start_line: int,
    end_line: int,
    *,
    disabled_diagnostics: set[str] | None = None,
    line_length: int = 120,
) -> list[types.Diagnostic]:
    """Compute W111/W112/W115 style diagnostics for a line range.

    Returns LSP-format diagnostics for lines ``start_line..end_line``
    (inclusive).  No suppression filtering is applied — the caller is
    responsible for that.
    """
    diags: list[types.Diagnostic] = []
    if not (disabled_diagnostics and "W111" in disabled_diagnostics):
        for d in _check_line_length(source, line_length):
            if start_line <= d.range.start.line <= end_line:
                diags.append(_to_lsp_diagnostic(d))
    if not (disabled_diagnostics and "W112" in disabled_diagnostics):
        for d in _check_trailing_whitespace(source):
            if start_line <= d.range.start.line <= end_line:
                diags.append(_to_lsp_diagnostic(d))
    if not (disabled_diagnostics and "W115" in disabled_diagnostics):
        for d in _check_comment_continuation(source):
            if start_line <= d.range.start.line <= end_line:
                diags.append(_to_lsp_diagnostic(d))
    return diags


def compute_all_style_diagnostics(
    source: str,
    *,
    disabled_diagnostics: set[str] | None = None,
    line_length: int = 120,
) -> list[types.Diagnostic]:
    """Compute W111/W112/W115 style diagnostics for the entire file.

    Returns LSP-format diagnostics sorted by start line.  Callers that
    need per-chunk partitioning should use :func:`bisect` on the result
    rather than calling :func:`compute_style_diagnostics_for_range`
    once per chunk (which re-scans the full source each time).

    W118 (line endings) is a file-level diagnostic emitted by
    :func:`get_basic_diagnostics` instead, to avoid duplication with
    the per-chunk cache path.
    """
    diags: list[types.Diagnostic] = []
    lines = source.split("\n")
    if not (disabled_diagnostics and "W111" in disabled_diagnostics):
        for d in _check_line_length(source, line_length, lines=lines):
            diags.append(_to_lsp_diagnostic(d))
    if not (disabled_diagnostics and "W112" in disabled_diagnostics):
        for d in _check_trailing_whitespace(source, lines=lines):
            diags.append(_to_lsp_diagnostic(d))
    if not (disabled_diagnostics and "W115" in disabled_diagnostics):
        for d in _check_comment_continuation(source, lines=lines):
            diags.append(_to_lsp_diagnostic(d))
    # Already sorted by start line because _check_* iterate lines in order.
    return diags


# W120: command without package require


def _inherited_packages(
    uri: str | None,
    ctx: WorkspaceDiagnosticContext,
) -> frozenset[str]:
    """Return packages inherited from files that ``source`` this file.

    Walks the reverse source graph: for each file that includes *uri*
    via ``source``, collect its ``package require`` names.  Falls back
    to all workspace package names when the source graph is empty or
    *uri* is unknown.
    """
    if not uri or not ctx.source_graph:
        return ctx.workspace_package_names

    # Reverse lookup: find all files whose source_graph includes uri.
    inherited: set[str] = set()
    for src_uri, targets in ctx.source_graph.items():
        if uri in targets:
            inherited.update(ctx.package_names_by_uri.get(src_uri, frozenset()))
    # If nothing found in the graph, fall back to all workspace packages
    # to avoid false positives for files not yet in the graph.
    if not inherited:
        return ctx.workspace_package_names
    return frozenset(inherited)


def _inherited_aliases(
    uri: str | None,
    ctx: WorkspaceDiagnosticContext,
) -> frozenset[str]:
    """Return alias tail names inherited via the source graph.

    For file F, collect aliases from all files transitively sourced by
    any file that ``source``-includes F.  This handles the common pattern
    where a package init file sources both an aliases file and the
    library files that use those aliases.
    """
    if not uri or not ctx.source_graph or not ctx.alias_names_by_uri:
        # Fallback: return all workspace aliases.
        all_aliases: set[str] = set()
        for tails in ctx.alias_names_by_uri.values():
            all_aliases.update(tails)
        return frozenset(all_aliases)

    # Reverse lookup: find parent files that source this URI.
    parents: set[str] = set()
    for src_uri, targets in ctx.source_graph.items():
        if uri in targets:
            parents.add(src_uri)

    if not parents:
        # Not in the source graph — fall back to all workspace aliases.
        all_aliases_fb: set[str] = set()
        for tails in ctx.alias_names_by_uri.values():
            all_aliases_fb.update(tails)
        return frozenset(all_aliases_fb)

    # Collect aliases from parents and all files they transitively source.
    inherited: set[str] = set()
    for parent in parents:
        # Aliases defined in the parent itself.
        inherited.update(ctx.alias_names_by_uri.get(parent, frozenset()))
        # Aliases from files the parent sources (transitive closure).
        visited: set[str] = set()
        stack = list(ctx.source_graph.get(parent, frozenset()))
        while stack:
            dep = stack.pop()
            if dep in visited or dep == uri:
                continue
            visited.add(dep)
            inherited.update(ctx.alias_names_by_uri.get(dep, frozenset()))
            stack.extend(ctx.source_graph.get(dep, frozenset()))

    return frozenset(inherited)


@diag("W120", "Command used without a corresponding `package require`.", section="warning")
def _check_missing_package_require(
    result: AnalysisResult,
    suppressed: dict[int, frozenset[str]],
    workspace_context: WorkspaceDiagnosticContext | None = None,
    uri: str | None = None,
) -> list[Diagnostic]:
    """W120: Flag package-gated commands used without a ``package require``."""
    # Skip entirely when the dialect does not support ``package require``
    # (e.g. iRules).  The availability is encoded on the ``package``
    # command spec via ``dialects``.
    dialect = active_dialect()
    if REGISTRY.get("package", dialect) is None:
        return []
    # When packages are loaded dynamically (variable/command substitution in
    # ``package require``, ``load``, ``auto_path``, etc.) we cannot know which
    # commands are available — suppress W120 entirely.
    if result.has_dynamic_providers:
        return []
    imported = result.active_package_names()
    # Expand with packages from files that ``source`` this file (parent
    # files in the dependency graph).  If no source graph is available,
    # fall back to all workspace package names.
    if workspace_context:
        imported = imported | _inherited_packages(uri, workspace_context)
    seen: set[str] = set()
    diagnostics: list[Diagnostic] = []
    for inv in result.command_invocations:
        pkg = REGISTRY.required_package_for(inv.name)
        if pkg is None:
            continue
        if pkg in imported:
            continue
        # Honour the per-spec warn_missing_import flag (e.g. Tk).
        spec = REGISTRY.get(inv.name)
        if spec is not None and not spec.warn_missing_import:
            continue
        if _is_suppressed("W120", inv.range.start.line, suppressed):
            continue
        # Only emit once per command name to avoid flooding.
        if inv.name in seen:
            continue
        seen.add(inv.name)
        # Build code fix: insert `package require <pkg>` at the right place.
        fix = _package_require_fix(pkg, result)
        diagnostics.append(
            Diagnostic(
                range=inv.range,
                message=f'"{inv.name}" requires `package require {pkg}`',
                severity=Severity.WARNING,
                code="W120",
                fixes=(fix,) if fix else (),
            )
        )
    return diagnostics


def _package_require_fix(pkg: str, result: AnalysisResult) -> CodeFix | None:
    """Build a ``CodeFix`` that inserts ``package require <pkg>``."""
    # Insert after the last existing package require, or at line 0.
    if result.package_requires:
        last = max(result.package_requires, key=lambda pr: pr.range.end.line)
        insert_line = last.range.end.line + 1
        insert_offset = last.range.end.offset + 1
    else:
        insert_line = 0
        insert_offset = 0
    pos = SourcePosition(line=insert_line, character=0, offset=insert_offset)
    return CodeFix(
        range=Range(start=pos, end=pos),
        new_text=f"package require {pkg}\n",
        description=f"Add 'package require {pkg}'",
    )


# Main entry point


def get_basic_diagnostics(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    cu: CompilationUnit | None = None,
    optimiser_enabled: bool = True,
    disabled_diagnostics: set[str] | None = None,
    disabled_optimisations: set[str] | None = None,
    line_length: int = 120,
    line_ending: str = "\n",
    cached_style_diagnostics: list[types.Diagnostic] | None = None,
    workspace_context: WorkspaceDiagnosticContext | None = None,
    uri: str | None = None,
) -> tuple[list[types.Diagnostic], AnalysisResult, dict[int, frozenset[str]]]:
    """Return fast diagnostics: analysis warnings + style checks.

    Returns ``(diagnostics, analysis_result, suppressed_lines)`` so the
    caller can pass the analysis context to deep passes without
    recomputing.

    When *cached_style_diagnostics* is provided, W111/W112/W115 style
    checks are skipped and the cached diagnostics are used instead.
    """
    cu = ensure_compilation_unit(source, cu, logger=log, context="diagnostics")

    result = analysis if analysis is not None else analyse(source, cu=cu)
    suppressed = result.suppressed_lines

    # Pre-compute workspace proc tail names and alias names for W123 filtering.
    _ws_proc_tails: frozenset[str] | None = None
    _ws_alias_tails: frozenset[str] | None = None
    if workspace_context and workspace_context.workspace_proc_names:
        _ws_proc_tails = frozenset(
            qn.rsplit("::", 1)[-1] for qn in workspace_context.workspace_proc_names
        )
    if workspace_context and workspace_context.alias_names_by_uri:
        _ws_alias_tails = _inherited_aliases(uri, workspace_context)

    diags: list[types.Diagnostic] = []
    for d in result.diagnostics:
        if disabled_diagnostics and d.code in disabled_diagnostics:
            continue
        if suppressed and _is_suppressed(d.code, d.range.start.line, suppressed):
            continue
        # W123: suppress if the unknown command is a workspace proc or alias.
        if d.code == "W123":
            cmd = d.message.split("'")[1] if "'" in d.message else ""
            if _ws_proc_tails is not None and cmd in _ws_proc_tails:
                continue
            if _ws_alias_tails is not None and cmd in _ws_alias_tails:
                continue
        diags.append(_to_lsp_diagnostic(d, uri=uri))
        if (
            optimiser_enabled
            and d.code == "W100"
            and not (disabled_optimisations and "O111" in disabled_optimisations)
        ):
            diags.append(_brace_expr_perf_hint(d))

    if cached_style_diagnostics is not None:
        # Use pre-computed style diagnostics (from per-chunk cache).
        # Still apply disabled-diagnostic and # noqa suppression filtering.
        for d in cached_style_diagnostics:
            if disabled_diagnostics and isinstance(d.code, str) and d.code in disabled_diagnostics:
                continue
            if (
                suppressed
                and isinstance(d.code, str)
                and _is_suppressed(d.code, d.range.start.line, suppressed)
            ):
                continue
            diags.append(d)
    else:
        # W111: line length
        if not (disabled_diagnostics and "W111" in disabled_diagnostics):
            for d in _check_line_length(source, line_length):
                if suppressed and _is_suppressed(d.code, d.range.start.line, suppressed):
                    continue
                diags.append(_to_lsp_diagnostic(d))

        # W112: trailing whitespace
        if not (disabled_diagnostics and "W112" in disabled_diagnostics):
            for d in _check_trailing_whitespace(source):
                if suppressed and _is_suppressed(d.code, d.range.start.line, suppressed):
                    continue
                diags.append(_to_lsp_diagnostic(d))

        # W115: backslash-newline continuation in comments
        if not (disabled_diagnostics and "W115" in disabled_diagnostics):
            for d in _check_comment_continuation(source):
                if suppressed and _is_suppressed(d.code, d.range.start.line, suppressed):
                    continue
                diags.append(_to_lsp_diagnostic(d))

    # W118: inconsistent line endings
    if not (disabled_diagnostics and "W118" in disabled_diagnostics):
        for d in _check_line_endings(source, expected=line_ending):
            diags.append(_to_lsp_diagnostic(d))

    # W120: command without package require
    if not (disabled_diagnostics and "W120" in disabled_diagnostics):
        for d in _check_missing_package_require(result, suppressed, workspace_context, uri):
            diags.append(_to_lsp_diagnostic(d))

    return diags, result, suppressed


def _run_deep_diagnostics(
    source: str,
    suppressed: dict[int, frozenset[str]],
    dialect: str,
    optimiser_enabled: bool = True,
    shimmer_enabled: bool = True,
    taint_enabled: bool = True,
    xc_diagnostics_enabled: bool = False,
    disabled_diagnostics: set[str] | None = None,
    disabled_optimisations: set[str] | None = None,
    uri: str | None = None,
    generic_variable_patterns: list[str] | None = None,
) -> list[types.Diagnostic]:
    """Subprocess-safe wrapper for deep diagnostics.

    Module-level function suitable for ``ProcessPoolExecutor``.
    Rebuilds the CompilationUnit from source (no pre-built CU or
    analysis needed) so the only data crossing the process boundary
    is the source string and config flags.
    """
    import server._codes_init  # noqa: F401
    from compiler.registry.runtime import configure_signatures

    configure_signatures(dialect=dialect)
    return get_deep_diagnostics(
        source,
        suppressed,
        cu=None,
        analysis=None,
        optimiser_enabled=optimiser_enabled,
        shimmer_enabled=shimmer_enabled,
        taint_enabled=taint_enabled,
        xc_diagnostics_enabled=xc_diagnostics_enabled,
        disabled_diagnostics=disabled_diagnostics,
        disabled_optimisations=disabled_optimisations,
        uri=uri,
        generic_variable_patterns=generic_variable_patterns,
    )


def get_deep_diagnostics(
    source: str,
    suppressed: dict[int, frozenset[str]],
    *,
    cu: CompilationUnit | None = None,
    analysis: AnalysisResult | None = None,
    optimiser_enabled: bool = True,
    shimmer_enabled: bool = True,
    taint_enabled: bool = True,
    xc_diagnostics_enabled: bool = False,
    disabled_diagnostics: set[str] | None = None,
    disabled_optimisations: set[str] | None = None,
    uri: str | None = None,
    generic_variable_patterns: list[str] | None = None,
) -> list[types.Diagnostic]:
    """Return deep diagnostics: optimiser, shimmer, taint, iRules flow, GVN.

    These are the expensive passes that can run in a background thread
    without blocking interactive features.
    """
    diags: list[types.Diagnostic] = []
    cu = ensure_compilation_unit(source, cu, logger=log, context="diagnostics")

    if optimiser_enabled:
        all_opts = find_optimisations(source, cu=cu)
        # Collect groups: group_id → list of optimisations.
        groups: dict[int, list[Optimisation]] = {}
        emitted_groups: set[int] = set()
        for opt in all_opts:
            if opt.group is not None:
                groups.setdefault(opt.group, []).append(opt)

        for opt in all_opts:
            if disabled_optimisations and opt.code in disabled_optimisations:
                continue
            if suppressed and _is_suppressed(opt.code, opt.range.start.line, suppressed):
                continue

            if opt.group is not None:
                # Emit one diagnostic per group, at the primary (non-elimination) member.
                if opt.group in emitted_groups:
                    continue
                # Filter disabled and suppressed members out of the group
                # so their edits are never included in the code action.
                _ELIM_CODES = frozenset(("O107", "O108", "O109"))
                raw_members = groups.get(opt.group, [opt])
                members = [
                    m
                    for m in raw_members
                    if not (disabled_optimisations and m.code in disabled_optimisations)
                    and not (suppressed and _is_suppressed(m.code, m.range.start.line, suppressed))
                ]
                if not members:
                    emitted_groups.add(opt.group)
                    continue
                # Primary = first non-elimination member; fallback to first.
                primary = next((m for m in members if m.code not in _ELIM_CODES), members[0])
                others = [m for m in members if m is not primary]
                # Build grouped edits for all members.  End positions use
                # character + 1 to convert from inclusive to LSP exclusive end.
                group_edits = [
                    {
                        "startLine": m.range.start.line,
                        "startCharacter": m.range.start.character,
                        "endLine": m.range.end.line,
                        "endCharacter": m.range.end.character + 1,
                        "startOffset": m.range.start.offset,
                        "endOffset": m.range.end.offset,
                        "replacement": m.replacement,
                        "code": m.code,
                        "message": m.message,
                    }
                    for m in members
                ]
                related = (
                    [
                        types.DiagnosticRelatedInformation(
                            location=types.Location(
                                uri=uri or "",
                                range=to_lsp_range(m.range),
                            ),
                            message=f"{m.code}: {m.message}",
                        )
                        for m in others
                    ]
                    if uri and others
                    else None
                )
                suffix = ""
                elim_count = sum(1 for m in others if m.code in _ELIM_CODES)
                if elim_count:
                    suffix = (
                        f" (+{elim_count} dead store{'s' if elim_count > 1 else ''} eliminated)"
                    )
                diags.append(
                    _optimisation_to_diagnostic(
                        primary,
                        group_edits=group_edits,
                        related=related,
                        message_suffix=suffix,
                    )
                )
                emitted_groups.add(opt.group)
            else:
                diags.append(_optimisation_to_diagnostic(opt))
    time.sleep(0)  # Yield GIL between deep diagnostic passes
    if shimmer_enabled:
        for w in find_shimmer_warnings(source, cu=cu):
            if disabled_diagnostics and w.code in disabled_diagnostics:
                continue
            if suppressed and _is_suppressed(w.code, w.range.start.line, suppressed):
                continue
            diags.append(_shimmer_to_diagnostic(w, uri=uri))
    time.sleep(0)  # Yield GIL between deep diagnostic passes
    if taint_enabled:
        for w in find_taint_warnings(source, cu=cu):
            if disabled_diagnostics and w.code in disabled_diagnostics:
                continue
            if suppressed and _is_suppressed(w.code, w.range.start.line, suppressed):
                continue
            diags.append(_taint_to_diagnostic(w))
    time.sleep(0)  # Yield GIL between deep diagnostic passes
    for w in find_irules_flow_warnings(
        source,
        cu=cu,
        generic_variable_patterns=generic_variable_patterns,
    ):
        if disabled_diagnostics and w.code in disabled_diagnostics:
            continue
        if suppressed and _is_suppressed(w.code, w.range.start.line, suppressed):
            continue
        diags.append(_irules_flow_to_diagnostic(w))
    time.sleep(0)  # Yield GIL between deep diagnostic passes
    if optimiser_enabled:
        for w in find_redundant_computations(source, cu=cu):
            if disabled_optimisations and w.code in disabled_optimisations:
                continue
            if suppressed and _is_suppressed(w.code, w.range.start.line, suppressed):
                continue
            diags.append(_gvn_to_diagnostic(w))

    # XC translatability diagnostics (opt-in)
    if xc_diagnostics_enabled:
        try:
            from dialects.f5.xc.diagnostics import get_xc_diagnostics

            for xc_diag in get_xc_diagnostics(source):
                if disabled_diagnostics and xc_diag.code in disabled_diagnostics:
                    continue
                if suppressed and _is_suppressed(
                    xc_diag.code, xc_diag.range.start.line, suppressed
                ):
                    continue
                diags.append(_to_lsp_diagnostic(xc_diag))
        except Exception:
            pass  # XC diagnostics are advisory — don't break normal diagnostics

    # Tk-specific diagnostics (only when package require Tk is present)
    if analysis is None:
        analysis = analyse(source, cu=cu)
    if has_tk_require(analysis):
        try:
            from analyser.checks.tk import check_tk_diagnostics

            for tk_diag in check_tk_diagnostics(source, analysis):
                if disabled_diagnostics and tk_diag.code in disabled_diagnostics:
                    continue
                if suppressed and _is_suppressed(
                    tk_diag.code, tk_diag.range.start.line, suppressed
                ):
                    continue
                diags.append(_to_lsp_diagnostic(tk_diag))
        except Exception:
            pass  # Tk diagnostics are advisory — don't break normal diagnostics

    return diags


def get_diagnostics(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    cu: CompilationUnit | None = None,
    optimiser_enabled: bool = True,
    shimmer_enabled: bool = True,
    taint_enabled: bool = True,
    xc_diagnostics_enabled: bool = False,
    disabled_diagnostics: set[str] | None = None,
    disabled_optimisations: set[str] | None = None,
    uri: str | None = None,
    line_length: int = 120,
    workspace_context: WorkspaceDiagnosticContext | None = None,
) -> list[types.Diagnostic]:
    """Analyse source and return all LSP diagnostics (basic + deep).

    This is the synchronous entry point kept for backward compatibility
    and for callers that need all diagnostics in a single list (e.g. tests).
    """
    cu = ensure_compilation_unit(source, cu, logger=log, context="diagnostics")

    basic, analysis_result, suppressed = get_basic_diagnostics(
        source,
        analysis,
        cu=cu,
        optimiser_enabled=optimiser_enabled,
        disabled_diagnostics=disabled_diagnostics,
        disabled_optimisations=disabled_optimisations,
        line_length=line_length,
        workspace_context=workspace_context,
        uri=uri,
    )
    deep = get_deep_diagnostics(
        source,
        suppressed,
        cu=cu,
        analysis=analysis_result,
        optimiser_enabled=optimiser_enabled,
        shimmer_enabled=shimmer_enabled,
        taint_enabled=taint_enabled,
        xc_diagnostics_enabled=xc_diagnostics_enabled,
        disabled_diagnostics=disabled_diagnostics,
        disabled_optimisations=disabled_optimisations,
        uri=uri,
    )
    return basic + deep
