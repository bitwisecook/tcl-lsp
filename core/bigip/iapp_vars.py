"""Extract iApp presentation variable references from implementation Tcl.

In iApp templates, presentation fields declared as ``section.field`` in APL
become ``$::section__field`` in the implementation Tcl code.  This module
extracts those references so they can be cross-validated against the
presentation model.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition

# Match $::name__name or ${::name__name} or ${::name__name(index)}
_IAPP_VAR_RE = re.compile(
    r"\$\{?::"  # $:: or ${::
    r"([a-zA-Z_][a-zA-Z0-9_]*(?:__[a-zA-Z0-9_]+)+)"  # name with __ separator
    r"(?:\([^)]*\))?"  # optional (index)
    r"\}?"  # optional closing }
)

# Match table row iteration variables: [tmsh::get_field_value $row <column>]
# where column matches the table column names.
_TABLE_FIELD_RE = re.compile(r"\btmsh::get_field_value\s+\S+\s+(\S+)")


@dataclass(frozen=True, slots=True)
class IappVarRef:
    """A reference to an iApp presentation variable in implementation Tcl."""

    tcl_name: str  # e.g. "::basic__addr"
    apl_name: str  # e.g. "basic.addr"
    range: Range  # source position of the reference


def extract_iapp_var_refs(source: str) -> list[IappVarRef]:
    """Extract iApp presentation variable references from Tcl source.

    Returns a list of variable references that follow the iApp naming
    convention (``::section__field``).
    """
    refs: list[IappVarRef] = []
    for m in _IAPP_VAR_RE.finditer(source):
        var_body = m.group(1)
        apl_name = var_body.replace("__", ".")
        tcl_name = "::" + var_body

        # Compute position
        line = source.count("\n", 0, m.start())
        line_start = source.rfind("\n", 0, m.start()) + 1
        char = m.start() - line_start

        refs.append(
            IappVarRef(
                tcl_name=tcl_name,
                apl_name=apl_name,
                range=Range(
                    start=SourcePosition(line=line, character=char, offset=m.start()),
                    end=SourcePosition(
                        line=line,
                        character=char + len(m.group()),
                        offset=m.end(),
                    ),
                ),
            )
        )
    return refs


def extract_iapp_table_refs(source: str) -> list[IappVarRef]:
    """Extract tmsh::get_field_value column references from Tcl source."""
    refs: list[IappVarRef] = []
    for m in _TABLE_FIELD_RE.finditer(source):
        col_name = m.group(1).strip("\"'{}")
        if not col_name or "__" in col_name:
            continue

        line = source.count("\n", 0, m.start(1))
        line_start = source.rfind("\n", 0, m.start(1)) + 1
        char = m.start(1) - line_start

        refs.append(
            IappVarRef(
                tcl_name=col_name,
                apl_name=col_name,
                range=Range(
                    start=SourcePosition(line=line, character=char, offset=m.start(1)),
                    end=SourcePosition(
                        line=line,
                        character=char + len(m.group(1)),
                        offset=m.end(1),
                    ),
                ),
            )
        )
    return refs
