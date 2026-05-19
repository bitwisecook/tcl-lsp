"""Structured model for F5 iApp APL (Application Presentation Language).

Parses APL source into a model of sections, fields, tables, and defines
that can be used for cross-file validation between presentation and
implementation files.  Also resolves ``#include`` directives.

Variable naming convention
==========================

APL qualified names use dots (``section.field``).  In the iApp implementation
Tcl, these become global variables with double underscores:
``$::section__field``.  Table columns follow the same pattern:
``$::table__column``.
"""

from __future__ import annotations

import logging
import os
import re
from dataclasses import dataclass, field

from analyser.semantic_model import Range
from compiler.parsing.tokens import SourcePosition

from ._text_utils import find_brace_end, offset_to_line_char
from .apl_parser import _FIELD_TYPE_KEYWORDS

log = logging.getLogger(__name__)

# Data model


@dataclass(frozen=True, slots=True)
class AplField:
    """A single form field declared in APL."""

    name: str  # e.g. "addr"
    qualified_name: str  # e.g. "basic.addr"
    field_type: str  # e.g. "string", "choice", "password"
    is_required: bool
    range: Range  # source position of the field name


@dataclass
class AplSection:
    """A named ``section`` block."""

    name: str
    qualified_name: str  # e.g. "basic" or parent-qualified
    fields: dict[str, AplField] = field(default_factory=dict)
    range: Range | None = None


@dataclass
class AplTable:
    """A named ``table`` block with column fields."""

    name: str
    qualified_name: str  # e.g. "pool.members"
    columns: dict[str, AplField] = field(default_factory=dict)
    range: Range | None = None


@dataclass
class AplInclude:
    """An ``#include`` directive."""

    path: str  # the path string from the directive
    line: int  # source line number
    resolved: bool = False  # whether the file was found


@dataclass
class AplModel:
    """Complete parsed APL presentation model."""

    sections: dict[str, AplSection] = field(default_factory=dict)
    tables: dict[str, AplTable] = field(default_factory=dict)
    defines: dict[str, str] = field(default_factory=dict)  # name -> underlying type
    includes: list[AplInclude] = field(default_factory=list)
    # Flattened map: qualified_name -> AplField (for quick lookup)
    all_fields: dict[str, AplField] = field(default_factory=dict)

    def tcl_variable_names(self) -> set[str]:
        """Return the set of Tcl global variable names this presentation exports.

        Converts APL qualified names (``section.field``) to the iApp Tcl
        convention (``::section__field``).
        """
        result: set[str] = set()
        for qname in self.all_fields:
            tcl_name = "::" + qname.replace(".", "__")
            result.add(tcl_name)
        return result


# Regex patterns for structural parsing

# section <name> {
_SECTION_DECL_RE = re.compile(r"^\s*section\s+(\S+)\s*\{", re.MULTILINE)
# table <name> {
_TABLE_DECL_RE = re.compile(r"^\s*table\s+(\S+)\s*\{", re.MULTILINE)
# field-type <name> ...
_FIELD_DECL_RE = re.compile(
    r"^\s*(" + "|".join(sorted(_FIELD_TYPE_KEYWORDS)) + r")\s+(\S+)",
    re.MULTILINE,
)
# define <type> <name>
_DEFINE_DECL_RE = re.compile(r"^\s*define\s+(\S+)\s+(\S+)", re.MULTILINE)
# #include "path"
_INCLUDE_RE = re.compile(r'^\s*#include\s+"([^"]+)"', re.MULTILINE)
# required attribute
_REQUIRED_RE = re.compile(r"\brequired\b")


# APL parsing


def _parse_fields_in_block(
    block_source: str,
    prefix: str,
    base_offset: int,
    source: str,
) -> dict[str, AplField]:
    """Extract field declarations from a brace-delimited block."""
    fields: dict[str, AplField] = {}
    for m in _FIELD_DECL_RE.finditer(block_source):
        ftype = m.group(1)
        fname = m.group(2)
        # Skip if name looks like an attribute keyword
        if fname in ("default", "display", "required", "validator"):
            continue
        # Skip if name starts with brace (not a real field name)
        if fname.startswith("{") or fname.startswith('"'):
            continue
        qname = f"{prefix}.{fname}" if prefix else fname
        rest_of_line_start = m.end()
        rest_of_line_end = block_source.find("\n", rest_of_line_start)
        if rest_of_line_end == -1:
            rest_of_line_end = len(block_source)
        rest = block_source[rest_of_line_start:rest_of_line_end]
        is_req = bool(_REQUIRED_RE.search(m.group())) or bool(_REQUIRED_RE.search(rest))

        abs_offset = base_offset + m.start(2)
        line, char = offset_to_line_char(source, abs_offset)
        fields[fname] = AplField(
            name=fname,
            qualified_name=qname,
            field_type=ftype,
            is_required=is_req,
            range=Range(
                start=SourcePosition(line=line, character=char, offset=abs_offset),
                end=SourcePosition(
                    line=line, character=char + len(fname), offset=abs_offset + len(fname)
                ),
            ),
        )
    return fields


def parse_apl(source: str) -> AplModel:
    """Parse APL source into a structured model."""
    model = AplModel()

    # Extract #include directives
    for m in _INCLUDE_RE.finditer(source):
        line = source.count("\n", 0, m.start())
        model.includes.append(AplInclude(path=m.group(1), line=line))

    # Extract define blocks
    for m in _DEFINE_DECL_RE.finditer(source):
        underlying_type = m.group(1)
        define_name = m.group(2)
        model.defines[define_name] = underlying_type

    # Extract sections (tracking their byte ranges for nested-table scoping)
    section_ranges: list[tuple[int, int, str]] = []  # (start, end, name)
    for m in _SECTION_DECL_RE.finditer(source):
        sec_name = m.group(1)
        brace_pos = m.end() - 1
        end_pos = find_brace_end(source, brace_pos)
        block = source[brace_pos + 1 : end_pos - 1]
        section_ranges.append((brace_pos, end_pos, sec_name))

        abs_offset = m.start(1)
        line, char = offset_to_line_char(source, abs_offset)

        section = AplSection(
            name=sec_name,
            qualified_name=sec_name,
            range=Range(
                start=SourcePosition(line=line, character=char, offset=abs_offset),
                end=SourcePosition(
                    line=line, character=char + len(sec_name), offset=abs_offset + len(sec_name)
                ),
            ),
        )
        section.fields = _parse_fields_in_block(block, sec_name, brace_pos + 1, source)
        model.sections[sec_name] = section
        for field_obj in section.fields.values():
            model.all_fields[field_obj.qualified_name] = field_obj

    # Extract tables (qualify name with parent section if nested)
    for m in _TABLE_DECL_RE.finditer(source):
        tbl_name = m.group(1)
        brace_pos = m.end() - 1
        end_pos = find_brace_end(source, brace_pos)
        block = source[brace_pos + 1 : end_pos - 1]

        # Determine parent section by checking if this table falls inside one
        parent_prefix = ""
        for sec_start, sec_end, sec_name in section_ranges:
            if sec_start < m.start() < sec_end:
                parent_prefix = sec_name
                break

        qualified_tbl = f"{parent_prefix}.{tbl_name}" if parent_prefix else tbl_name

        abs_offset = m.start(1)
        line, char = offset_to_line_char(source, abs_offset)

        table = AplTable(
            name=tbl_name,
            qualified_name=qualified_tbl,
            range=Range(
                start=SourcePosition(line=line, character=char, offset=abs_offset),
                end=SourcePosition(
                    line=line, character=char + len(tbl_name), offset=abs_offset + len(tbl_name)
                ),
            ),
        )
        table.columns = _parse_fields_in_block(block, qualified_tbl, brace_pos + 1, source)
        model.tables[qualified_tbl] = table
        for field_obj in table.columns.values():
            model.all_fields[field_obj.qualified_name] = field_obj

    return model


# #include resolution


def resolve_apl_includes(
    source: str,
    base_dir: str | None = None,
    *,
    _seen: frozenset[str] | None = None,
) -> AplModel:
    """Parse APL source and recursively resolve ``#include`` directives.

    Parameters
    ----------
    source:
        APL source text.
    base_dir:
        Directory to resolve relative ``#include`` paths against.
        If ``None``, includes are not resolved.
    _seen:
        Internal set of already-visited paths to prevent cycles.
    """
    model = parse_apl(source)
    if base_dir is None:
        return model

    if _seen is None:
        _seen = frozenset()

    for inc in model.includes:
        inc_path = os.path.join(base_dir, inc.path)
        inc_path = os.path.normpath(inc_path)

        if inc_path in _seen:
            log.debug("Circular #include detected: %s", inc_path)
            continue

        if not os.path.isfile(inc_path):
            log.debug("#include not found: %s", inc_path)
            continue

        inc.resolved = True
        try:
            with open(inc_path, encoding="utf-8", errors="replace") as f:
                inc_source = f.read()
        except OSError:
            log.debug("Failed to read #include: %s", inc_path, exc_info=True)
            continue

        inc_model = resolve_apl_includes(
            inc_source,
            os.path.dirname(inc_path),
            _seen=_seen | {inc_path},
        )
        # Merge included model (included definitions are available but
        # don't override local ones).
        for k, v in inc_model.defines.items():
            model.defines.setdefault(k, v)
        for k, v in inc_model.sections.items():
            model.sections.setdefault(k, v)
            for fk, fv in v.fields.items():
                model.all_fields.setdefault(fv.qualified_name, fv)
        for k, v in inc_model.tables.items():
            model.tables.setdefault(k, v)
            for fk, fv in v.columns.items():
                model.all_fields.setdefault(fv.qualified_name, fv)

    return model


# Tcl variable mapping


def tcl_var_to_apl_name(tcl_var: str) -> str | None:
    """Convert a Tcl iApp variable name to APL qualified name.

    ``::basic__addr`` → ``basic.addr``
    ``basic__addr``   → ``basic.addr``

    Returns ``None`` if the name doesn't match the iApp convention.
    """
    name = tcl_var.lstrip(":")
    if "__" not in name:
        return None
    return name.replace("__", ".")


def apl_name_to_tcl_var(apl_name: str) -> str:
    """Convert an APL qualified name to the Tcl global variable name.

    ``basic.addr`` → ``::basic__addr``
    """
    return "::" + apl_name.replace(".", "__")
