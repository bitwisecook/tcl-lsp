"""Go-to-type-definition provider.

Uses the compiler's type lattice (``compiler.core_analyses``) to resolve
a variable's inferred class, then jumps to that ``ClassDef``.  No regex or
source-text scanning -- the compiler already tracks TclOO instance types via
``TypeLattice.object_of(class_name)`` when it sees ``[Class new ...]`` or
``[Class create ...]`` assignments.

For cursor-on-``my``-method-call, we fall back to the enclosing class scope
(looked up via the analyser's lexical scope chain).
"""

from __future__ import annotations

from lsprotocol import types

from compiler.core_analyses import analyse_source
from compiler.types import TclType, TypeKind
from core.analysis import analyse
from core.analysis.semantic_model import AnalysisResult, ClassDef
from shared.lsp import to_lsp_location

from .symbol_resolution import find_scope_at_line, find_var_at_position, find_word_at_position


def _find_class(
    name: str,
    analysis: AnalysisResult,
    workspace_classes: list[tuple[str, ClassDef]] | None,
) -> tuple[str | None, ClassDef] | None:
    """Resolve ``name`` to ``(uri or None, ClassDef)`` from local/workspace scope."""
    for _qname, cd in analysis.all_classes.items():
        if cd.name == name or cd.qualified_name == name or cd.qualified_name == f"::{name}":
            return (None, cd)
    if workspace_classes:
        for w_uri, cd in workspace_classes:
            if cd.name == name or cd.qualified_name == name or cd.qualified_name == f"::{name}":
                return (w_uri, cd)
    return None


def _infer_var_class(source: str, var_name: str) -> str | None:
    """Look up the inferred class for ``var_name`` in the compiler type lattice.

    Returns the first ``class_name`` for any SSA version of ``var_name``
    whose type is ``KNOWN OBJECT`` with a concrete ``class_name``.  Searches
    both the top-level function and every analysed procedure.
    """
    try:
        module = analyse_source(source)
    except Exception:
        return None

    for function in [module.top_level, *module.procedures.values()]:
        for (name, _version), type_lattice in function.types.items():
            if name != var_name:
                continue
            if type_lattice.kind is not TypeKind.KNOWN:
                continue
            if type_lattice.tcl_type is not TclType.OBJECT:
                continue
            if type_lattice.class_name:
                return type_lattice.class_name
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

    # Variable receiver: ask the compiler for the inferred class.
    var_name = find_var_at_position(source, line, character)
    if var_name:
        class_name = _infer_var_class(source, var_name)
        if class_name is None:
            return []
        resolved = _find_class(class_name, analysis, workspace_classes)
        if resolved is None:
            return []
        ent_uri, class_def = resolved
        return [to_lsp_location(ent_uri or uri, class_def.name_range)]

    # Method receiver: only ``my`` call sites (or the method declaration name)
    # inside a class body should resolve to the enclosing class.  Other
    # identifiers fall through to the empty list so we do not over-report.
    word = find_word_at_position(source, line, character)
    if not word:
        return []
    scope = find_scope_at_line(analysis.global_scope, line)
    if scope.kind != "method":
        return []
    class_name = scope.name.rsplit("::", 1)[0] if "::" in scope.name else scope.name
    if class_name in (None, "", "::"):
        return []
    # Require the cursor to actually be on a method name that exists on the
    # enclosing class (either a declaration or a ``my <name>`` call).
    resolved = _find_class(class_name, analysis, workspace_classes)
    if resolved is None:
        return []
    ent_uri, class_def = resolved
    if word not in class_def.methods and word not in class_def.class_methods:
        return []
    return [to_lsp_location(ent_uri or uri, class_def.name_range)]
