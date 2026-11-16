#!/usr/bin/env python3
"""Make a split-off module import names from where they are defined.

    direct_imports.py CHILD.rs [CHILD.rs ...]

After a split, clippy narrows each child's `use super::*;` to
`use super::{A, B, ...};`. Many of those names are only imports of the parent
(`use std::collections::HashMap;`), so the child would depend on the parent's
import list. This rewrites each such name to its real path and keeps
`use super::Name` for what the parent itself declares.
"""

import re
import sys
from pathlib import Path

from apply_split import ROOT_STEMS
from rust_items import Item, parse_items

KEYWORD_ROOTS = ("crate", "self", "super")


def split_top(text: str, separator: str) -> list[str]:
    parts, depth, start = [], 0, 0
    index = 0
    while index < len(text):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        elif depth == 0 and text.startswith(separator, index):
            parts.append(text[start:index])
            index += len(separator)
            start = index
            continue
        index += 1
    parts.append(text[start:])
    return [part.strip() for part in parts if part.strip()]


def top_level_separator(tree: str) -> int:
    """Offset of the first `::` outside braces, or -1."""
    depth = 0
    for index, char in enumerate(tree):
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        elif depth == 0 and tree.startswith("::", index):
            return index
    return -1


def leaves(tree: str, prefix: tuple[str, ...] = ()) -> list[tuple[tuple[str, ...], str]]:
    """Every `(path, bound name)` a use tree imports; a glob binds `*`."""
    tree = tree.strip()
    if tree.startswith("{") and tree.endswith("}"):
        return [leaf for part in split_top(tree[1:-1], ",") for leaf in leaves(part, prefix)]
    cut = top_level_separator(tree)
    if cut >= 0:
        return leaves(tree[cut + 2 :], prefix + (tree[:cut].strip(),))
    head = tree
    name, _, alias = head.partition(" as ")
    name, alias = name.strip(), alias.strip()
    if name == "self":
        return [(prefix, alias or prefix[-1])]
    return [(prefix + (name,), alias or name)]


def use_items(text: str) -> list[Item]:
    return [item for item in parse_items(text) if item.kind == "use"]


def use_tree(text: str, item: Item) -> str:
    statement = " ".join(text[item.head : item.end].split())
    return re.sub(r"^(?:pub(?:\([^)]*\))? )?use ", "", statement).removesuffix(";").strip()


def gated(text: str, item: Item) -> bool:
    return re.search(r"^[ \t]*#\[", text[item.start : item.head], re.M) is not None


def parent_of(child: Path) -> Path:
    owner = child.parent.with_suffix(".rs")
    if owner.exists():
        return owner
    for stem in ROOT_STEMS:
        candidate = child.parent / f"{stem}.rs"
        if candidate.exists():
            return candidate
    raise SystemExit(f"{child}: no parent module file")


def parent_scope(parent: Path) -> tuple[dict[str, tuple[str, ...]], set[str]]:
    """What each name imported by the parent stands for, and the parent's own modules."""
    text = parent.read_text(encoding="utf-8")
    modules = {item.name for item in parse_items(text) if item.kind == "mod"}
    imported: dict[str, tuple[str, ...]] = {}
    for item in use_items(text):
        if gated(text, item):
            continue
        for path, bound in leaves(use_tree(text, item)):
            if bound != "*" and path[-1] != "*":
                imported[bound] = path
    return imported, modules


def from_child(path: tuple[str, ...], imported: dict[str, tuple[str, ...]], modules: set[str], depth: int = 0) -> tuple[str, ...]:
    """A parent-relative path as the child writes it."""
    head = path[0]
    if head == "crate" or head.startswith("::") or head == "":
        return path
    if head == "self":
        return ("super", *path[1:])
    if head == "super":
        return ("super", *path)
    if head in modules:
        return ("super", *path)
    if head in imported and depth < 8 and imported[head] != path:
        return from_child((*imported[head], *path[1:]), imported, modules, depth + 1)
    return path


def render(groups: dict[tuple[str, ...], list[str]], attributes: str) -> str:
    lines = []
    for prefix, names in sorted(groups.items()):
        names = sorted(set(names))
        joined = names[0] if len(names) == 1 else "{" + ", ".join(names) + "}"
        lines.append(f"{attributes}use {'::'.join((*prefix, joined))};\n")
    return "".join(lines)


def rewrite(child: Path) -> int:
    imported, modules = parent_scope(parent_of(child))
    text = child.read_text(encoding="utf-8")
    changed = 0
    for item in sorted(use_items(text), key=lambda item: item.start, reverse=True):
        tree = use_tree(text, item)
        if not re.match(r"super::(?!super\b)", tree):
            continue
        # The item's own attributes; module docs above the first item are not part of it.
        leading = text[item.start : item.head]
        first_attribute = re.search(r"^[ \t]*#\[", leading, re.M)
        start = item.start + first_attribute.start() if first_attribute else item.head
        attributes = text[start : item.head]
        kept: list[str] = []
        direct: dict[tuple[str, ...], list[str]] = {}
        for path, bound in leaves(tree):
            name = path[-1]
            spelled = name if bound == name else f"{name} as {bound}"
            if len(path) != 2 or name == "*" or bound not in imported or bound != name:
                kept.append("::".join(path[1:-1] + (spelled,)))
                continue
            target = from_child(imported[name], imported, modules)
            leaf = target[-1] if target[-1] == name else f"{target[-1]} as {name}"
            direct.setdefault(target[:-1], []).append(leaf)
            changed += 1
        if not direct:
            continue
        replacement = render(direct, attributes)
        if kept:
            replacement += render({("super",): kept}, attributes)
        end = item.end + 1 if text[item.end : item.end + 1] == "\n" else item.end
        text = text[:start] + replacement + text[end:]
    if changed:
        child.write_text(text, encoding="utf-8")
    return changed


def main() -> int:
    # A parent first, so its own children see where its names really come from.
    for name in sorted(sys.argv[1:], key=lambda name: (len(Path(name).parts), name)):
        count = rewrite(Path(name))
        print(f"{count:4}  {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
