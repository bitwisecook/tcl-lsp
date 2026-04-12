"""Refactoring engine — mechanical code transformations for Tcl and iRules.

Each sub-module exposes pure functions that accept source text (and
optionally an analysis result) and return :class:`RefactoringEdit`
objects describing the changes.  The LSP code-actions layer, MCP
server, and Claude AI skills all consume these functions identically.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class RefactoringEdit:
    """A single text replacement in a source document."""

    start_line: int
    start_character: int
    end_line: int
    end_character: int
    new_text: str


def _apply_edits(edits: tuple[RefactoringEdit, ...], source: str) -> str:
    """Apply *edits* to *source* bottom-to-top and return the result."""
    lines = source.split("\n")

    def _offset(line: int, char: int) -> int:
        off = 0
        for i in range(min(line, len(lines))):
            off += len(lines[i]) + 1
        return off + char

    sorted_edits = sorted(
        edits,
        key=lambda e: _offset(e.start_line, e.start_character),
        reverse=True,
    )
    result = source
    for edit in sorted_edits:
        start = _offset(edit.start_line, edit.start_character)
        end = _offset(edit.end_line, edit.end_character)
        result = result[:start] + edit.new_text + result[end:]
    return result


@dataclass(frozen=True, slots=True)
class RefactoringResult:
    """The outcome of a refactoring operation."""

    title: str
    edits: tuple[RefactoringEdit, ...]
    kind: str = "refactor.rewrite"

    def apply(self, source: str) -> str:
        """Apply all edits to *source* and return the rewritten text."""
        return _apply_edits(self.edits, source)


@dataclass(frozen=True, slots=True)
class DataGroupDefinition:
    """A generated data-group artefact (separate from the iRule edits)."""

    name: str
    value_type: str  # "string", "ip", "integer"
    records: tuple[tuple[str, str], ...]  # (key, value) pairs


@dataclass(frozen=True, slots=True)
class DataGroupExtractionResult:
    """Result of an extract-to-datagroup refactoring."""

    title: str
    edits: tuple[RefactoringEdit, ...]
    data_group: DataGroupDefinition
    kind: str = "refactor.extract"

    def apply(self, source: str) -> str:
        """Apply iRule edits to *source*."""
        return _apply_edits(self.edits, source)

    def data_group_tcl(self) -> str:
        """Render the data-group as a BIG-IP tmsh definition."""
        dg = self.data_group
        lines = [f"ltm data-group internal {dg.name} {{"]
        if dg.records:
            lines.append("    records {")
            for key, value in dg.records:
                if value:
                    lines.append(f"        {key} {{")
                    lines.append(f"            data {value}")
                    lines.append("        }")
                else:
                    lines.append(f"        {key} {{ }}")
            lines.append("    }")
        lines.append(f"    type {dg.value_type}")
        lines.append("}")
        return "\n".join(lines)
