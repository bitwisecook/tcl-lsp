"""Go-to-type-definition provider.

Tcl has no static typing, so this is intentionally narrow:

- For a ``$var`` where the variable was initialised via ``[ClassName new ...]``
  (or ``[ClassName create ...]``), return ``ClassName``'s declaration.
- For a ``my method`` call inside a class body, return the enclosing class's
  declaration.
- Otherwise return an empty list.
"""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ClassDef
from core.common.lsp import find_var_in_scopes, to_lsp_location

from .symbol_resolution import find_scope_at_line, find_var_at_position, find_word_at_position

# set|lassign|variable  name  [ClassName new|create ...]
# The class name is captured as group 1.
_CLASS_ASSIGN_RE = re.compile(
    r"""
    \b(?:set|variable|lassign)\s+
    [\w:]+\s+                  # variable name (possibly namespace-qualified)
    \[\s*
    ([\w:]+)                   # class name
    \s+(?:new|create)\b
    """,
    re.VERBOSE,
)


def _find_class(
    name: str,
    analysis: AnalysisResult,
    workspace_classes: list[tuple[str, ClassDef]] | None,
) -> tuple[str | None, ClassDef] | None:
    """Resolve ``name`` to ``(uri or None, ClassDef)`` from local/workspace scope."""
    for _qname, cd in analysis.all_classes.items():
        if (
            cd.name == name
            or cd.qualified_name == name
            or cd.qualified_name == f"::{name}"
        ):
            return (None, cd)
    if workspace_classes:
        for w_uri, cd in workspace_classes:
            if (
                cd.name == name
                or cd.qualified_name == name
                or cd.qualified_name == f"::{name}"
            ):
                return (w_uri, cd)
    return None


def _class_of_var(source: str, var_name: str) -> str | None:
    """Return the class name the variable is constructed with, if any.

    Looks for the first ``set|variable|lassign VAR [ClassName new|create ...]``
    assignment naming ``var_name``.
    """
    stripped = var_name.lstrip(":")
    for line in source.splitlines():
        m = _CLASS_ASSIGN_RE.search(line)
        if not m:
            continue
        # Extract the variable name from the line — recover the second word.
        parts = line.strip().split(None, 2)
        if len(parts) < 3:
            continue
        candidate_var = parts[1].lstrip(":")
        if candidate_var == stripped:
            return m.group(1)
    return None


def get_type_definition(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
    workspace_classes: list[tuple[str, ClassDef]] | None = None,
) -> list[types.Location]:
    """Return type-definition locations for the symbol at ``(line, character)``."""
    if analysis is None:
        analysis = analyse(source)

    # Variable receiver: look for [Class new ...] initialisation.
    var_name = find_var_at_position(source, line, character)
    if var_name:
        scope = find_scope_at_line(analysis.global_scope, line)
        var_def = find_var_in_scopes(var_name, scope)
        if var_def is None:
            return []
        class_name = _class_of_var(source, var_name)
        if class_name is None:
            return []
        resolved = _find_class(class_name, analysis, workspace_classes)
        if resolved is None:
            return []
        ent_uri, class_def = resolved
        return [to_lsp_location(ent_uri or uri, class_def.name_range)]

    # Method receiver: ``my foo`` inside a class body -> enclosing class.
    word = find_word_at_position(source, line, character)
    if not word:
        return []
    scope = find_scope_at_line(analysis.global_scope, line)
    cursor = scope
    while cursor is not None:
        if cursor.kind == "method":
            # Scope name is "Class::method" — strip the method to get the class.
            class_name = cursor.name.rsplit("::", 1)[0] if "::" in cursor.name else cursor.name
            resolved = _find_class(class_name, analysis, workspace_classes)
            if resolved:
                ent_uri, class_def = resolved
                return [to_lsp_location(ent_uri or uri, class_def.name_range)]
            break
        if cursor.kind == "class":
            resolved = _find_class(cursor.name, analysis, workspace_classes)
            if resolved:
                ent_uri, class_def = resolved
                return [to_lsp_location(ent_uri or uri, class_def.name_range)]
            break
        cursor = cursor.parent
    return []
