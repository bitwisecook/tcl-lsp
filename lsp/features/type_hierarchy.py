"""Type hierarchy provider -- browse class inheritance in the editor."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ClassDef
from core.common.lsp import to_lsp_range

from .symbol_resolution import find_word_at_position


def _class_to_item(uri: str, class_def: ClassDef) -> types.TypeHierarchyItem:
    """Convert a ClassDef to an LSP TypeHierarchyItem."""
    name_range = to_lsp_range(class_def.name_range)
    body_range = to_lsp_range(class_def.body_range)
    detail = class_def.metaclass if class_def.metaclass != "oo::class" else None
    return types.TypeHierarchyItem(
        name=class_def.name,
        kind=types.SymbolKind.Class,
        uri=uri,
        range=body_range,
        selection_range=name_range,
        detail=detail,
    )


def prepare_type_hierarchy(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.TypeHierarchyItem]:
    """Resolve the class at the cursor position for type hierarchy browsing."""
    if analysis is None:
        analysis = analyse(source)

    word = find_word_at_position(source, line, character)
    if not word:
        return []

    for _qname, class_def in analysis.all_classes.items():
        if (
            class_def.name == word
            or class_def.qualified_name == word
            or class_def.qualified_name == f"::{word}"
        ):
            return [_class_to_item(uri, class_def)]

    return []


def supertypes(
    item: types.TypeHierarchyItem,
    analysis: AnalysisResult | None = None,
    source: str = "",
) -> list[types.TypeHierarchyItem]:
    """Return the supertypes (superclasses + mixins) of the given class."""
    if analysis is None:
        analysis = analyse(source)

    class_name = item.name
    uri = item.uri

    # Find the class
    class_def: ClassDef | None = None
    for _qname, cd in analysis.all_classes.items():
        if cd.name == class_name or cd.qualified_name == class_name:
            class_def = cd
            break

    if class_def is None:
        return []

    results: list[types.TypeHierarchyItem] = []
    for parent_name in class_def.superclasses + class_def.mixins:
        for _qname, cd in analysis.all_classes.items():
            if (
                cd.name == parent_name
                or cd.qualified_name == parent_name
                or cd.qualified_name == f"::{parent_name}"
            ):
                results.append(_class_to_item(uri, cd))
                break

    return results


def subtypes(
    item: types.TypeHierarchyItem,
    analysis: AnalysisResult | None = None,
    source: str = "",
) -> list[types.TypeHierarchyItem]:
    """Return the direct subtypes (subclasses) of the given class."""
    if analysis is None:
        analysis = analyse(source)

    class_name = item.name
    uri = item.uri

    results: list[types.TypeHierarchyItem] = []
    for _qname, cd in analysis.all_classes.items():
        if class_name in cd.superclasses or class_name in cd.mixins:
            results.append(_class_to_item(uri, cd))
        # Also check qualified name
        for _qname2, target_cd in analysis.all_classes.items():
            if target_cd.name == class_name or target_cd.qualified_name == class_name:
                if (
                    target_cd.qualified_name in cd.superclasses
                    or target_cd.qualified_name in cd.mixins
                ):
                    if not any(r.name == cd.name for r in results):
                        results.append(_class_to_item(uri, cd))
                break

    return results
