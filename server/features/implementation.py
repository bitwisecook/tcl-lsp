"""Go-to-implementation provider -- jump to TclOO method overrides.

For a class name, returns all direct subclasses.  For a method name, returns
every subclass that overrides the method plus the declaring class's definition.
"""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ClassDef
from shared.lsp import to_lsp_location

from .symbol_resolution import find_scope_at_line, find_word_at_position


def _class_matches(cd: ClassDef, name: str) -> bool:
    """True when ``cd`` is addressed by ``name`` (simple or qualified)."""
    return cd.name == name or cd.qualified_name == name or cd.qualified_name == f"::{name}"


def _is_parent_of(candidate: ClassDef, parent_qnames: set[str]) -> bool:
    """True when ``candidate`` lists any of ``parent_qnames`` as superclass or mixin."""
    for parent in candidate.superclasses + candidate.mixins:
        if parent in parent_qnames:
            return True
        if parent.lstrip(":") in {q.lstrip(":") for q in parent_qnames}:
            return True
    return False


def _collect_class_entries(
    analysis: AnalysisResult,
    uri: str,
    workspace_classes: list[tuple[str, ClassDef]] | None,
) -> list[tuple[str, ClassDef]]:
    """Merge the current file's classes with cross-file workspace classes.

    The current file's classes win on URI so the single-file tests still work
    without a workspace index.
    """
    entries: list[tuple[str, ClassDef]] = []
    seen_qnames: set[tuple[str, str]] = set()
    for _qname, cd in analysis.all_classes.items():
        key = (uri, cd.qualified_name)
        seen_qnames.add(key)
        entries.append((uri, cd))
    if workspace_classes:
        for w_uri, cd in workspace_classes:
            key = (w_uri, cd.qualified_name)
            if key in seen_qnames:
                continue
            entries.append((w_uri, cd))
    return entries


def get_implementations(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
    workspace_classes: list[tuple[str, ClassDef]] | None = None,
) -> list[types.Location]:
    """Return implementation locations for the symbol at ``(line, character)``.

    - **Cursor on a class name:** returns every subclass declaration.
    - **Cursor on a method name inside a class body:** returns each override
      in a subclass, plus any further overrides down the hierarchy.
    - **Cursor on a ``my method`` call inside a method body:** returns the
      method definitions in the enclosing class and every subclass override.
    """
    if analysis is None:
        analysis = analyse(source)

    word = find_word_at_position(source, line, character)
    if not word:
        return []

    entries = _collect_class_entries(analysis, uri, workspace_classes)

    # Case 1: cursor on a class name — return subclass declarations.
    target_class_qnames: set[str] = set()
    for _ent_uri, cd in entries:
        if _class_matches(cd, word):
            target_class_qnames.add(cd.qualified_name)

    if target_class_qnames:
        locations: list[types.Location] = []
        seen: set[tuple[str, int, int]] = set()
        for ent_uri, cd in entries:
            if cd.qualified_name in target_class_qnames:
                continue
            if _is_parent_of(cd, target_class_qnames):
                key = (ent_uri, cd.name_range.start.line, cd.name_range.start.character)
                if key in seen:
                    continue
                seen.add(key)
                locations.append(to_lsp_location(ent_uri, cd.name_range))
        return locations

    # Case 2/3: cursor on a method name.  Find the enclosing class (if any)
    # and treat ``word`` as a method name.
    enclosing_class_qname: str | None = None
    scope = find_scope_at_line(analysis.global_scope, line)
    cursor = scope
    while cursor is not None:
        if cursor.kind == "method":
            # Scope name is "Class::method"; strip to get the class name.
            class_name = cursor.name.rsplit("::", 1)[0] if "::" in cursor.name else cursor.name
            for _ent_uri, cd in entries:
                if _class_matches(cd, class_name):
                    enclosing_class_qname = cd.qualified_name
                    break
            if enclosing_class_qname:
                break
        # Also match when the cursor is directly inside a class body.
        if cursor.kind == "class":
            for _ent_uri, cd in entries:
                if _class_matches(cd, cursor.name):
                    enclosing_class_qname = cd.qualified_name
                    break
            if enclosing_class_qname:
                break
        cursor = cursor.parent

    # Collect classes that define ``word`` as a method.
    locations: list[types.Location] = []
    seen: set[tuple[str, int, int]] = set()

    def _add(ent_uri: str, method_def) -> None:  # noqa: ANN001 (MethodDef)
        key = (
            ent_uri,
            method_def.name_range.start.line,
            method_def.name_range.start.character,
        )
        if key in seen:
            return
        seen.add(key)
        locations.append(to_lsp_location(ent_uri, method_def.name_range))

    # The ancestor chain of the enclosing class (used to filter what counts
    # as an "override" when the cursor is inside a class).
    ancestors: set[str] = set()
    if enclosing_class_qname is not None:
        # Walk superclasses transitively.
        work = [enclosing_class_qname]
        while work:
            qn = work.pop()
            if qn in ancestors:
                continue
            ancestors.add(qn)
            for _ent_uri, cd in entries:
                if cd.qualified_name == qn:
                    work.extend(cd.superclasses)
                    work.extend(cd.mixins)

    found_any = False
    for ent_uri, cd in entries:
        method_def = cd.methods.get(word) or cd.class_methods.get(word)
        if method_def is None:
            continue
        found_any = True
        if enclosing_class_qname is None:
            # Cursor not in a class: return every class that declares the method.
            _add(ent_uri, method_def)
            continue
        # Cursor inside a class: return this class's own definition plus any
        # subclass override.  Skip ancestors' definitions (they are reachable
        # via go-to-definition).
        if cd.qualified_name == enclosing_class_qname:
            _add(ent_uri, method_def)
            continue
        if cd.qualified_name in ancestors:
            continue
        # Check whether this class is a descendant of the enclosing class.
        visiting = {cd.qualified_name}
        work = list(cd.superclasses) + list(cd.mixins)
        is_descendant = False
        while work:
            parent_name = work.pop()
            if parent_name in visiting:
                continue
            visiting.add(parent_name)
            for _e2_uri, pd in entries:
                if pd.qualified_name == parent_name or pd.name == parent_name:
                    if pd.qualified_name == enclosing_class_qname:
                        is_descendant = True
                        work.clear()
                        break
                    work.extend(pd.superclasses)
                    work.extend(pd.mixins)
        if is_descendant:
            _add(ent_uri, method_def)

    if not found_any:
        return []
    return locations
