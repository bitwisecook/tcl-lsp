# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Type hierarchy provider -- browse class inheritance in the editor."""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ClassDef
from server._lsp_conv import to_lsp_range

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

    # Resolve class_name to its qualified name once (O(n) instead of O(n^2))
    qualified_names: set[str] = {class_name}
    for _qname, target_cd in analysis.all_classes.items():
        if target_cd.name == class_name or target_cd.qualified_name == class_name:
            qualified_names.add(target_cd.qualified_name)

    results: list[types.TypeHierarchyItem] = []
    seen: set[str] = set()
    for _qname, cd in analysis.all_classes.items():
        if cd.name in seen:
            continue
        for qn in qualified_names:
            if qn in cd.superclasses or qn in cd.mixins:
                results.append(_class_to_item(uri, cd))
                seen.add(cd.name)
                break

    return results
