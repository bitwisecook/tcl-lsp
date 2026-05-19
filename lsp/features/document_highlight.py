"""Document highlight provider -- highlight all occurrences of the symbol at cursor.

For variables, uses the analyser's scope tree to distinguish local/global
shadows (e.g. a local ``x`` inside ``proc p`` is distinct from a global
``x`` at file scope) and marks the definition site as
:attr:`DocumentHighlightKind.Write` and reference sites as
:attr:`DocumentHighlightKind.Read`.  The Read/Write distinction lets
editors render write-sites differently (VS Code uses a brighter
background).

For procs, classes, and other non-variable identifiers the provider
delegates to :func:`get_references` and marks every occurrence as
:attr:`DocumentHighlightKind.Text` -- there is no semantically
meaningful read/write distinction for command invocations.
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis import analyse
from core.analysis.semantic_model import AnalysisResult
from shared.lsp import find_var_in_scopes, to_lsp_range

from .references import get_references
from .symbol_resolution import find_scope_at_line, find_var_at_position


def _variable_highlights(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult,
) -> list[types.DocumentHighlight] | None:
    """Return Read/Write highlights for the variable at ``(line, character)``.

    Returns ``None`` when the cursor is not on a variable; callers then
    fall back to the generic reference-based path.
    """
    var_name = find_var_at_position(source, line, character)
    if not var_name:
        return None
    scope = find_scope_at_line(analysis.global_scope, line)
    var_def = find_var_in_scopes(var_name, scope)
    if var_def is None:
        return None

    highlights: list[types.DocumentHighlight] = []
    seen: set[tuple[int, int, int, int]] = set()

    def _add(r, kind: types.DocumentHighlightKind) -> None:  # noqa: ANN001
        key = (r.start.line, r.start.character, r.end.line, r.end.character)
        if key in seen:
            return
        seen.add(key)
        highlights.append(types.DocumentHighlight(range=to_lsp_range(r), kind=kind))

    # Definition site: Write.  The analyser uses a zero-width range for
    # the definition when the variable was introduced by a command that
    # has no single-token name (e.g. ``foreach x`` argv); treat those as
    # Write too since they are the binding site.
    _add(var_def.definition_range, types.DocumentHighlightKind.Write)
    # Every recorded reference is a Read.
    for ref in var_def.references:
        _add(ref, types.DocumentHighlightKind.Read)
    return highlights


def get_document_highlights(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.DocumentHighlight]:
    """Return highlight ranges for the symbol at ``(line, character)``.

    Returns an empty list when no symbol is under the cursor.  Variable
    highlights carry Read/Write kinds; non-variable highlights (procs,
    classes, methods) are marked as Text.
    """
    # Variable path — use the scope tree so shadowed names stay separate.
    # Compute analysis on demand when state.analysis has not been populated
    # yet (did_open runs analysis as a fire-and-forget task, so callers may
    # hit us before state.analysis is ready on slow CI runners).
    if analysis is None:
        analysis = analyse(source)
    var_highlights = _variable_highlights(source, line, character, analysis)
    if var_highlights is not None:
        return var_highlights

    # Non-variable path — delegate to the reference provider and mark
    # each occurrence as Text (procs, classes, method names have no
    # read/write distinction on a per-invocation basis).
    locations = get_references(
        source,
        uri,
        line,
        character,
        analysis=analysis,
        include_declaration=True,
    )
    highlights: list[types.DocumentHighlight] = []
    seen: set[tuple[int, int, int, int]] = set()
    for loc in locations:
        if loc.uri != uri:
            continue
        key = (
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        )
        if key in seen:
            continue
        seen.add(key)
        highlights.append(
            types.DocumentHighlight(
                range=loc.range,
                kind=types.DocumentHighlightKind.Text,
            )
        )
    return highlights
