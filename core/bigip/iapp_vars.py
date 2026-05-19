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
from ._text_utils import offset_to_line_char

# Match $::name or ${::name} (with optional ``(index)``).  We match the
# whole identifier in one greedy chunk and post-filter for the iApp
# ``__`` separator; the previous form had nested
# ``[a-zA-Z0-9_]+`` × ``(?:__...)+`` quantifiers which produced
# catastrophic backtracking on inputs like ``${::A__0__0__0__...}``
# (CodeQL ``py/redos``).
_IAPP_VAR_RE = re.compile(
    r"\$\{::"  # ${:: braced form
    r"([a-zA-Z_][a-zA-Z0-9_]*)"  # full identifier (post-filter for __)
    r"(?:\([^)]*\))?"  # optional (index)
    r"\}"  # closing }
    r"|\$::"  # $:: unbraced form
    r"([a-zA-Z_][a-zA-Z0-9_]*)"  # full identifier
    r"(?:\([^)]*\))?"  # optional (index)
)


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
        var_body = m.group(1) or m.group(2)  # group 1 = braced, group 2 = unbraced
        # Post-filter: real iApp refs contain at least one ``__`` separator.
        # The regex itself no longer enforces this (avoiding backtracking
        # on long underscore-heavy identifiers); we drop plain ``$::var``
        # references that don't follow the iApp naming convention.
        if "__" not in var_body:
            continue
        apl_name = var_body.replace("__", ".")
        tcl_name = "::" + var_body

        # Compute position
        line, char = offset_to_line_char(source, m.start())

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
