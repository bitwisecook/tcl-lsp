#!/usr/bin/env python3
"""Apply a split plan: move named items of one Rust file into child modules.

    apply_split.py PLAN.toml [--check] [--root DIR]

A plan names one file and the modules its items move to:

    file = "rust/tcl-lsp-core/src/semantic_tokens.rs"

    [[module]]
    name = "legend"
    doc = "The token legend and its wire encoding."
    items = ["const:TOKEN_TYPES", "fn:legend..fn:encode", "impl:Walker::emit"]

Item ids are those `outline.py` prints. `a..b` is every item from `a` to `b`
in file order, at one level: both top-level, or both methods of one impl.
`--check` resolves the plan and prints the resulting file sizes without
writing anything.

The move is textual. A moved private item that is named outside its new
module becomes `pub(super)` and the parent imports it; an item that was
visible further is re-exported at its old visibility, so every old path still
resolves. Each child starts with `use super::*;` for clippy's
`wildcard_imports` fix to narrow. What the text cannot settle (a private
field read across modules, a macro defined below its first user) is left for
the compiler to report.
"""

import argparse
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from rust_items import Item, parse_items

LIMIT = 2000
ROOT_STEMS = ("lib", "main", "mod")
WIDENABLE = ("fn", "struct", "enum", "union", "trait", "type", "const", "static")
IMMOVABLE = ("use", "mod", "macro_rules", "macro", "extern crate")
SUPER_PATH = re.compile(r"(?<![\w:])super::")
SELF_PATH = re.compile(r"(?<![\w:])self::")


class PlanError(Exception):
    pass


@dataclass
class Moved:
    item: Item
    owner: Item | None  # the impl block a method comes from
    start: int  # item start widened over the plain comment lines above it


@dataclass
class Child:
    name: str
    doc: str
    moved: list[Moved] = field(default_factory=list)


def child_dir(parent: Path) -> Path:
    return parent.parent if parent.stem in ROOT_STEMS else parent.with_suffix("")


def licence_header(text: str) -> str:
    lines = []
    for line in text.split("\n"):
        if not line.startswith("//") or line.startswith(("///", "//!")):
            break
        lines.append(line)
    return "\n".join(lines) + "\n\n" if lines else ""


def comment_lead(text: str, start: int) -> int:
    """`start` moved back over plain comment lines directly above the item."""
    line_start = text.rfind("\n", 0, start) + 1
    if text[line_start:start].strip():
        return start
    lead = line_start
    while lead > 0:
        previous = text.rfind("\n", 0, lead - 1) + 1
        if not text[previous:lead].strip().startswith("//"):
            break
        lead = previous
    return lead


def index_items(items: list[Item]) -> dict[str, tuple[Item, Item | None]]:
    index: dict[str, tuple[Item, Item | None]] = {}
    for item in items:
        index[item.ident] = (item, None)
    for item in items:
        for method in item.methods:
            index.setdefault(method.ident, (method, item))
    return index


def resolve(spec: str, items: list[Item], index: dict[str, tuple[Item, Item | None]]) -> list[tuple[Item, Item | None]]:
    if ".." not in spec:
        if spec not in index:
            raise PlanError(f"no item `{spec}`")
        return [index[spec]]
    first_id, last_id = spec.split("..", 1)
    for ident in (first_id, last_id):
        if ident not in index:
            raise PlanError(f"no item `{ident}` (in range `{spec}`)")
    (first, first_owner), (last, last_owner) = index[first_id], index[last_id]
    if first_owner is not last_owner:
        raise PlanError(f"range `{spec}` crosses an impl block")
    level = first_owner.methods if first_owner else items
    if first.start > last.start:
        raise PlanError(f"range `{spec}` runs backwards")
    return [(item, first_owner) for item in level if first.start <= item.start <= last.start and item.kind not in IMMOVABLE]


def is_trait_impl(owner: Item) -> bool:
    return re.search(r"\sfor\s", " " + owner.name) is not None


def build_children(plan: dict, text: str, items: list[Item]) -> list[Child]:
    index = index_items(items)
    taken: dict[int, str] = {}
    children = []
    for module in plan.get("module", []):
        child = Child(module["name"], module.get("doc", ""))
        if not re.fullmatch(r"[a-z][a-z0-9_]*", child.name):
            raise PlanError(f"`{child.name}` is not a module name")
        for spec in module["items"]:
            for item, owner in resolve(spec, items, index):
                if item.kind in IMMOVABLE:
                    raise PlanError(f"`{item.ident}` stays in the parent: a {item.kind} item cannot move")
                if owner and is_trait_impl(owner):
                    raise PlanError(f"`{item.ident}` belongs to a trait impl: move the whole impl")
                if item.start in taken:
                    raise PlanError(f"`{item.ident}` is in both `{taken[item.start]}` and `{child.name}`")
                taken[item.start] = child.name
                child.moved.append(Moved(item, owner, comment_lead(text, item.start)))
        child.moved.sort(key=lambda moved: moved.start)
        children.append(child)
    for child in children:
        for moved in child.moved:
            if moved.owner and moved.owner.start in taken:
                raise PlanError(f"`{moved.item.ident}` and its whole impl are both listed")
    return children


def moved_text(text: str, moved: Moved) -> str:
    return text[moved.start : moved.item.end]


def widen(body: str, moved: Moved, text: str, named_outside: bool) -> str:
    """The item's text with its visibility adjusted for one level deeper."""
    item = moved.item
    head = item.head - moved.start
    before, after = body[:head], body[head:]
    visibility = item.visibility
    if not visibility:
        if item.kind in WIDENABLE and named_outside:
            after = "pub(super) " + after
    elif visibility in ("pub(super)", "pub(in super)"):
        after = "pub(in super::super)" + after[len(visibility) :]
    elif visibility == "pub(self)":
        after = "pub(super)" + after[len(visibility) :]
    elif visibility.startswith("pub(in super"):
        after = "pub(in super::" + after[len("pub(in ") :]
    return before + after


def deepen_paths(body: str) -> str:
    body = SUPER_PATH.sub("super::super::", body)
    return SELF_PATH.sub("super::", body)


def impl_opening(text: str, owner: Item) -> str:
    attributes = [line for line in text[owner.start : owner.head].split("\n") if line.strip().startswith("#[")]
    assert owner.body is not None
    header = text[owner.head : owner.body[0] - 1]
    return "".join(line + "\n" for line in attributes) + header


def names_word(name: str, haystack: str) -> bool:
    return re.search(rf"(?<![\w]){re.escape(name)}(?![\w])", haystack) is not None


def plan_file(root: Path, plan: dict, write: bool) -> list[tuple[str, int]]:
    parent = root / plan["file"]
    text = parent.read_text(encoding="utf-8")
    items = parse_items(text)
    children = build_children(plan, text, items)
    directory = child_dir(parent)
    for child in children:
        target = directory / f"{child.name}.rs"
        if target.exists() or (directory / child.name).is_dir():
            raise PlanError(f"{target.relative_to(root)} already exists")
        if any(item.kind == "mod" and item.name == child.name for item in items):
            raise PlanError(f"the parent already declares `mod {child.name}`")

    # Text that stays in the parent, and the text beside it that can name a moved item.
    cuts = sorted((moved.start, moved.item.end) for child in children for moved in child.moved)
    emptied = []
    for item in items:
        if item.kind == "impl" and item.methods:
            gone = sum(1 for child in children for moved in child.moved if moved.owner is item)
            if gone == len(item.methods):
                emptied.append((comment_lead(text, item.start), item.end))
    cuts = sorted(cuts + emptied)
    remaining = ""
    removed_before: list[tuple[int, int]] = []  # (offset in `text`, characters removed before it)
    cursor = 0
    for start, end in cuts:
        if start < cursor:
            continue
        remaining += text[cursor:start]
        cursor = end
        while text[cursor : cursor + 1] == "\n" and remaining.endswith(("\n\n", "{\n")):
            cursor += 1
        removed_before.append((cursor, cursor - len(remaining)))
    remaining += text[cursor:]

    production_neighbours = ""
    test_neighbours = ""
    test_modules = set(re.findall(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod (\w+);", text))
    if directory.is_dir() and parent.stem not in ROOT_STEMS:
        for path in sorted(directory.rglob("*.rs")):
            top = path.relative_to(directory).parts[0].removesuffix(".rs")
            content = path.read_text(encoding="utf-8", errors="replace")
            if top in test_modules:
                test_neighbours += content
            else:
                production_neighbours += content

    header = licence_header(text)
    declarations = []
    outputs = []
    for child in children:
        own = "".join(moved_text(text, moved) for moved in child.moved)
        siblings = "".join(moved_text(text, moved) for other in children if other is not child for moved in other.moved)
        production_outside = remaining + siblings + production_neighbours
        parts = [header, f"//! {child.doc}\n\n" if child.doc else "", "use super::*;\n"]
        open_impl: Item | None = None
        imports: list[str] = []
        for moved in child.moved:
            item = moved.item
            in_production = names_word(item.name, production_outside) if item.kind in WIDENABLE else False
            in_tests = names_word(item.name, test_neighbours) if item.kind in WIDENABLE else False
            body = widen(deepen_paths(moved_text(text, moved)), moved, text, in_production or in_tests)
            if moved.owner is not open_impl:
                if open_impl is not None:
                    parts.append("}\n")
                open_impl = moved.owner
                if open_impl is not None:
                    parts.append("\n" + deepen_paths(impl_opening(text, open_impl)) + "{\n")
            elif open_impl is not None:
                parts.append("\n")
            if open_impl is None:
                parts.append("\n")
            parts.append(body.rstrip("\n") + "\n")
            if moved.owner is None and item.kind in WIDENABLE:
                visibility = item.visibility
                if visibility == "pub(self)":
                    visibility = ""
                if visibility:
                    imports.append(f"{visibility} use {child.name}::{item.name};")
                elif in_production:
                    imports.append(f"use {child.name}::{item.name};")
                elif in_tests:
                    imports.append(f"#[cfg(test)]\nuse {child.name}::{item.name};")
        if open_impl is not None:
            parts.append("}\n")
        outputs.append((directory / f"{child.name}.rs", "".join(parts)))
        declarations.append((child.name, imports))

    # Declarations go after the leading `use`/`mod` run, and after any macro the children may use.
    anchor = 0
    for item in items:
        if item.kind in ("use", "mod", "extern crate"):
            anchor = item.end
        else:
            break
    macros = [item.end for item in items if item.kind in ("macro_rules", "macro")]
    anchor = max([anchor, *macros])
    if anchor == 0 and items:
        anchor = comment_lead(text, items[0].start)
    offset = anchor - max([removed for at, removed in removed_before if at <= anchor], default=0)
    block = "\n\n" + "".join(f"mod {name};\n" for name, _ in declarations)
    uses = [line for _, imports in declarations for line in sorted(set(imports))]
    if uses:
        block += "\n" + "".join(line + "\n" for line in uses)
    new_parent = remaining[:offset] + block + remaining[offset:]

    sizes = [(parent.relative_to(root).as_posix(), new_parent.count("\n") + 1)]
    sizes += [(path.relative_to(root).as_posix(), content.count("\n") + 1) for path, content in outputs]
    if write:
        directory.mkdir(parents=True, exist_ok=True)
        for path, content in outputs:
            path.write_text(content, encoding="utf-8")
        parent.write_text(new_parent, encoding="utf-8")
    return sizes


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("plan", type=Path)
    parser.add_argument("--check", action="store_true", help="resolve the plan and print sizes; write nothing")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    arguments = parser.parse_args()
    plan = tomllib.loads(arguments.plan.read_text(encoding="utf-8"))
    try:
        sizes = plan_file(arguments.root, plan, write=not arguments.check)
    except PlanError as error:
        print(f"{arguments.plan}: {error}", file=sys.stderr)
        return 1
    over = False
    for path, lines in sizes:
        mark = "  OVER" if lines > LIMIT else ""
        over = over or lines > LIMIT
        print(f"{lines:6}  {path}{mark}")
    return 2 if over and arguments.check else 0


if __name__ == "__main__":
    raise SystemExit(main())
