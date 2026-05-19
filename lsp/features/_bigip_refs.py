"""BIG-IP find-references and rename — parser/graph/registry-driven.

Three editor capabilities, one shared core:

- :func:`get_bigip_references` answers ``textDocument/references``
  for a BIG-IP path-shaped token by walking
  :data:`background_scanner.bigip_configs` and emitting one
  :class:`Location` per occurrence (definitions and references).
- :func:`prepare_bigip_rename` validates that the cursor is on a
  renameable BIG-IP path-shaped token and returns the token's
  range so the editor draws the rename input box correctly.
- :func:`get_bigip_rename_edits` builds a ``WorkspaceEdit`` from
  :func:`rename_object` (the same engine ``f5 rename`` uses), with
  per-file post-rename parse validation so a malformed rename
  fails loudly instead of writing broken SCF to disk.

All three reuse the parser + the deep-linking object-resolver +
the iRule scanner — no per-feature regex tables.  When a kind is
added to the model + projection + registry, every editor feature
that depends on these helpers picks it up automatically.
"""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis.semantic_model import Range, SourcePosition
from dialects.f5.bigip.model import BigipConfig
from dialects.f5.bigip.parser import parse_bigip_conf
from dialects.f5.bigip.rewrite import rename_object
from shared.lsp import to_lsp_location, to_lsp_range

# Identifier-shaped token at the cursor: a TMSH path
# (``/Partition/Name``) or a bare name.  Matches the same character
# class the renamer's pattern uses.
_BIGIP_TOKEN_CHARS = re.compile(r"[A-Za-z0-9_/.\-]+")


def get_bigip_references(
    source: str,
    *,
    uri: str,
    line: int,
    character: int,
    workspace_configs: dict[str, BigipConfig] | None,
    workspace_sources: dict[str, str] | None,
    include_declaration: bool = True,
) -> list[types.Location]:
    """Return every occurrence of the BIG-IP path-shaped token at the cursor.

    Walks every BIG-IP file the workspace scanner has indexed
    (passed in *workspace_configs* / *workspace_sources*) and emits
    one :class:`Location` per occurrence.  When *include_declaration*
    is False, the object's stanza-header occurrence is dropped from
    the result (matches LSP convention for "references only").
    """
    token = _token_at_cursor(source, line, character)
    if token is None or not token.startswith("/"):
        return []

    configs = dict(workspace_configs or {})
    sources = dict(workspace_sources or {})
    # Make sure the current document is in the search set even when
    # the scanner hasn't indexed it yet (e.g. a brand-new file).
    if uri not in sources:
        sources[uri] = source
    if uri not in configs:
        try:
            configs[uri] = parse_bigip_conf(source)
        except Exception:  # noqa: BLE001
            return []

    locations: list[types.Location] = []
    pattern = _build_token_pattern(token)
    for file_uri, file_source in sources.items():
        for match in pattern.finditer(file_source):
            absolute_range = _offset_to_range(file_source, match.start(), match.end())
            locations.append(to_lsp_location(file_uri, absolute_range))

    if not include_declaration:
        # Drop the location whose start equals the stanza header
        # range start of the matching object in any indexed config.
        # ``rename_object``'s pattern is identifier-bounded so we
        # already have only standalone occurrences; the declaration
        # is the one that lives inside a stanza-header range.
        decl_locations = _declaration_locations(token, configs)
        locations = [loc for loc in locations if not _is_inside_any(loc, decl_locations)]
    return locations


def prepare_bigip_rename(source: str, *, line: int, character: int) -> types.Range | None:
    """Validate that the cursor is on a renameable BIG-IP path token.

    Returns the token's range when it's a TMSH path
    (``/Partition/Name``).  Returns ``None`` when the cursor isn't on
    such a token — the editor then refuses the rename gesture
    rather than letting the user try and fail.
    """
    token = _token_at_cursor(source, line, character)
    if token is None or not token.startswith("/"):
        return None
    span = _token_span_at_cursor(source, line, character)
    if span is None:
        return None
    start_offset, end_offset = span
    rng = _offset_to_range(source, start_offset, end_offset)
    return to_lsp_range(rng)


def get_bigip_rename_edits(
    source: str,
    *,
    uri: str,
    line: int,
    character: int,
    new_name: str,
    workspace_configs: dict[str, BigipConfig] | None,
    workspace_sources: dict[str, str] | None,
) -> types.WorkspaceEdit | None:
    """Build a :class:`WorkspaceEdit` renaming the object at the cursor.

    Calls :func:`rename_object` per file (same engine ``f5 rename``
    uses), so token boundaries, parse-validation refusal of broken
    SCF, and the existing kind-scoped header protection all kick in
    automatically.  When the rename produces no occurrences in a
    file, that file is omitted from the workspace edit.
    """
    old_name = _token_at_cursor(source, line, character)
    if old_name is None or not old_name.startswith("/"):
        return None
    if not new_name or not new_name.startswith("/"):
        return None

    sources = dict(workspace_sources or {})
    if uri not in sources:
        sources[uri] = source
    # ``workspace_configs`` is accepted for parity with the
    # references/definition signatures but the renamer drives off
    # the source text directly (``rename_object`` reparses to
    # validate), so the parsed configs aren't needed here.
    _ = workspace_configs

    changes: dict[str, list[types.TextEdit]] = {}
    for file_uri, file_source in sources.items():
        try:
            report = rename_object(file_source, old_name, new_name)
        except ValueError:
            # ``rename_object`` reparses to validate; a failing
            # rename in *this* file is a hard error and aborts the
            # whole workspace edit.
            return None
        if report.occurrences == 0:
            continue
        # WorkspaceEdit needs per-edit ranges; we have one rewritten
        # source.  Use a single full-document replacement edit per
        # file — simpler and matches what the editor will display
        # in its preview.
        full_range = _full_document_range(file_source)
        changes.setdefault(file_uri, []).append(
            types.TextEdit(range=full_range, new_text=report.new_source)
        )

    if not changes:
        return None
    return types.WorkspaceEdit(changes=changes)


def _token_at_cursor(source: str, line: int, character: int) -> str | None:
    span = _token_span_at_cursor(source, line, character)
    if span is None:
        return None
    return source[span[0] : span[1]]


def _token_span_at_cursor(source: str, line: int, character: int) -> tuple[int, int] | None:
    """Return the byte (start, end) of the path-shaped token at the cursor."""
    line_start = _line_start_offset(source, line)
    line_end = source.find("\n", line_start)
    if line_end == -1:
        line_end = len(source)
    line_text = source[line_start:line_end]
    # The cursor's character offset is into ``line_text``; find the
    # token that contains it.
    if character < 0 or character > len(line_text):
        return None
    for match in _BIGIP_TOKEN_CHARS.finditer(line_text):
        if match.start() <= character <= match.end():
            return (line_start + match.start(), line_start + match.end())
    return None


def _line_start_offset(source: str, line: int) -> int:
    """Return the byte offset of *line*'s first character."""
    if line <= 0:
        return 0
    seen = 0
    offset = 0
    for ch in source:
        if seen == line:
            return offset
        if ch == "\n":
            seen += 1
        offset += 1
    return offset


def _offset_to_range(source: str, start: int, end: int) -> Range:
    """Convert byte (start, end) into a :class:`Range`."""
    start_line, start_col = _offset_to_line_col(source, start)
    end_line, end_col = _offset_to_line_col(source, end)
    return Range(
        start=SourcePosition(line=start_line, character=start_col, offset=start),
        end=SourcePosition(line=end_line, character=end_col, offset=end),
    )


def _offset_to_line_col(source: str, offset: int) -> tuple[int, int]:
    """Return (line, column) for byte *offset*."""
    line = 0
    line_start = 0
    for i, ch in enumerate(source[: offset + 1]):
        if i == offset:
            break
        if ch == "\n":
            line += 1
            line_start = i + 1
    return line, offset - line_start


def _build_token_pattern(token: str) -> re.Pattern[str]:
    """Token-bounded pattern that matches *token* without firing on substrings.

    Same boundary rules :func:`rename_object` uses, so the search
    set lines up with the rewrite.
    """
    return re.compile(rf"(?<![A-Za-z0-9_/.\-]){re.escape(token)}(?![A-Za-z0-9_/.\-])")


def _declaration_locations(token: str, configs: dict[str, BigipConfig]) -> list[types.Location]:
    """Return one Location per stanza header that *declares* an object
    named *token* across every indexed config.

    Used to filter the declaration occurrence out of references-only
    results.  Walks every dataclass field of every config — picks up
    every kind without per-kind dispatch.
    """
    from dataclasses import fields

    decls: list[types.Location] = []
    for file_uri, cfg in configs.items():
        for fld in fields(cfg):
            if fld.name == "generic_objects":
                continue
            kind_dict = getattr(cfg, fld.name)
            if not isinstance(kind_dict, dict):
                continue
            obj = kind_dict.get(token)
            if obj is None:
                continue
            obj_range = getattr(obj, "range", None)
            if obj_range is None:
                continue
            decls.append(to_lsp_location(file_uri, obj_range))
    return decls


def _is_inside_any(location: types.Location, others: list[types.Location]) -> bool:
    """Return True when *location* falls inside any of *others*."""
    for other in others:
        if location.uri != other.uri:
            continue
        if other.range.start.line <= location.range.start.line <= other.range.end.line:
            return True
    return False


def _full_document_range(source: str) -> types.Range:
    """Return a ``Range`` covering the entire document."""
    if not source:
        return types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=0),
        )
    lines = source.split("\n")
    last_line = len(lines) - 1
    return types.Range(
        start=types.Position(line=0, character=0),
        end=types.Position(line=last_line, character=len(lines[-1])),
    )
