#!/usr/bin/env python3
"""Apply a split plan: move named items of one Rust file into child modules.

    apply_split.py PLAN.toml [--check] [--root DIR]

A plan names one file and the modules its items move to:

    file = "rust/tcl-lsp-core/src/semantic_tokens.rs"

    [[module]]
    name = "legend"
    doc = "The token legend and its wire encoding."
    items = ["const:TOKEN_TYPES", "fn:legend..fn:encode", "impl:Walker::emit"]

`move_from = "rust/x/tests/name.rs"` beside `file = "rust/x/tests/name/main.rs"`
moves a crate-root integration test into its own directory first.

A plan may also start with `hoist = ["name"]`: each inline `mod name { ... }`
becomes `mod name;` with its body in `name.rs`, before the modules are cut.

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
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

from hoist_inline_tests import INCLUDE, path_loaded_files
from rust_items import Item, parse_items

LIMIT = 2000
ROOT_STEMS = ("lib", "main", "mod")
WIDENABLE = ("fn", "struct", "enum", "union", "trait", "type", "const", "static", "mod")
IMMOVABLE = ("use", "mod", "macro_rules", "macro", "extern crate")
SUPER_PATH = re.compile(r"(?<![\w:])super::(?=(\w+|\*|\{))")
SELF_PATH = re.compile(r"(?<![\w:])self::")
CFG_ATTRIBUTE = re.compile(r"^[ \t]*(#\[cfg\(.*\)\])[ \t]*$", re.M)
TEST_CFG = r"#\[cfg\((?:test|all\(test\b[^\]]*)\)\]"
TEST_MODULE = re.compile(TEST_CFG + r"\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod (\w+);")


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


def is_test_file(path: Path) -> bool:
    """Whether `path` is only compiled for tests: an integration test, or a
    module its parent declares under `#[cfg(test)]`."""
    if "tests" in path.parts[:-1] or "benches" in path.parts[:-1]:
        return True
    if path.stem in ROOT_STEMS:
        return False
    owners = [path.parent.with_suffix(".rs"), *(path.parent / f"{stem}.rs" for stem in ROOT_STEMS)]
    for owner in owners:
        if owner.exists():
            if path.stem in TEST_MODULE.findall(owner.read_text(encoding="utf-8", errors="replace")):
                return True
            return is_test_file(owner)
    return False


def test_only(text: str, moved: Moved) -> bool:
    leading = text[moved.start : moved.item.head]
    return re.search(r"#\[test\]|" + TEST_CFG, leading) is not None


def licence_header(text: str) -> str:
    lines = []
    for line in text.split("\n"):
        if not line.startswith("//") or line.startswith(("///", "//!")):
            break
        lines.append(line)
    return "\n".join(lines) + "\n\n" if lines else ""


def previous_line(text: str, at: int) -> tuple[int, str]:
    """Start and stripped content of the line that ends just before `at`."""
    start = text.rfind("\n", 0, at - 1) + 1 if at > 0 else 0
    return start, text[start:at].strip()


def is_plain_comment(line: str) -> bool:
    return line.startswith("//") and not line.startswith(("///", "//!"))


def comment_lead(text: str, start: int) -> int:
    """`start` moved back over the plain comments that belong to the item:
    the lines directly above it, and a section heading set off by blank lines
    above those (the heading goes with the first item of its section)."""
    line_start = text.rfind("\n", 0, start) + 1
    if text[line_start:start].strip():
        return start
    lead = line_start
    while lead > 0:
        previous, line = previous_line(text, lead)
        if not is_plain_comment(line):
            break
        lead = previous
    if lead == 0:
        return lead
    blank, line = previous_line(text, lead)
    if line or blank == 0:
        return lead
    heading = blank
    while heading > 0:
        previous, line = previous_line(text, heading)
        if not is_plain_comment(line):
            break
        heading = previous
    if heading == blank or heading == 0:
        return lead
    _, above = previous_line(text, heading)
    return heading if above == "" or above.endswith("{") else lead


def prologue_end(text: str) -> int:
    """Offset just past the licence header, the inner docs and the inner attributes."""
    end = 0
    cursor = 0
    in_licence = True
    open_brackets = 0
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        cursor += len(line)
        if open_brackets or stripped.startswith("#!["):
            open_brackets += stripped.count("[") - stripped.count("]")
            in_licence = False
        elif stripped.startswith("//!"):
            in_licence = False
        elif not stripped:
            in_licence = False
            continue
        elif not (in_licence and is_plain_comment(stripped)):
            break
        end = cursor - 1 if line.endswith("\n") else cursor
    return end


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
    return [(item, first_owner) for item in level if first.start <= item.start <= last.start and not immovable(item)]


def immovable(item: Item) -> bool:
    """`use`, macro and `mod name;` items stay; a module written inline moves like any item."""
    return item.kind in IMMOVABLE and not (item.kind == "mod" and item.body is not None)


def hoist_modules(text: str, names: list[str], deeper: bool) -> tuple[str, list[tuple[str, str]]]:
    """Turn each inline `mod name { ... }` into `mod name;`, returning the bodies."""
    bodies = []
    for name in names:
        found = [item for item in parse_items(text) if item.kind == "mod" and item.name == name and item.body is not None]
        if len(found) != 1:
            raise PlanError(f"hoist: no single inline `mod {name}`")
        item = found[0]
        assert item.body is not None
        body = text[item.body[0] : item.body[1]].strip("\n")
        if deeper:
            body = INCLUDE.sub(lambda match: match.group(0) + "../", body)
        text = text[: item.body[0] - 1].rstrip() + ";" + text[item.end :]
        bodies.append((name, body + "\n"))
    return text, bodies


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
                if immovable(item):
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


PUB_SUPER = "\x00S\x00"
PUB_GRAND = "\x00G\x00"
FIELD = re.compile(r"^[ \t]*(?:r#)?(\w+)\s*:(?!:)", re.M)


WORD = re.compile(r"\w+")
FIELD_READ = re.compile(r"\.\s*(\w+)(?!\w)(?!\s*\()")
BUILT = re.compile(r"\b(?<!impl )(?<!for )(?<!struct )(?<!enum )(?<!union )(?<!trait )(?<!mod )(\w+)\s*\{")
IMPL_HEADER = re.compile(r"\bimpl\b([^{;]*)")


@dataclass
class Census:
    """How often a text names a word, reads a field, builds a struct or opens an impl for a type."""

    words: Counter
    reads: Counter
    built: Counter
    impls: Counter

    @classmethod
    def of(cls, text: str) -> "Census":
        impls: Counter = Counter()
        for header in IMPL_HEADER.findall(text):
            impls.update(WORD.findall(header))
        return cls(Counter(WORD.findall(text)), Counter(FIELD_READ.findall(text)), Counter(BUILT.findall(text)), impls)

    def __add__(self, other: "Census") -> "Census":
        return Census(self.words + other.words, self.reads + other.reads, self.built + other.built, self.impls + other.impls)

    def __sub__(self, other: "Census") -> "Census":
        return Census(self.words - other.words, self.reads - other.reads, self.built - other.built, self.impls - other.impls)


@dataclass
class Reach:
    """What code outside one child module names."""

    outside: Census
    exposed: set[str] = field(default_factory=set)  # words of the child's own signatures that outsiders see

    def names(self, name: str) -> bool:
        return self.outside.words[name] > 0 or name in self.exposed

    def reads(self, field_name: str) -> bool:
        return self.outside.reads[field_name] > 0

    def builds(self, struct_name: str) -> bool:
        if self.outside.built[struct_name] > 0:
            return True
        return self.outside.impls[struct_name] > 0 and self.outside.built["Self"] > 0


def visibility_edit(item: Item, reach: Reach) -> tuple[int, int, str] | None:
    """The edit that keeps `item` as visible one level deeper as it was."""
    visibility = item.visibility
    if not visibility:
        if item.kind in WIDENABLE and reach.names(item.name):
            return (item.head, 0, PUB_SUPER + " ")
        return None
    if visibility == "pub(self)":
        return (item.head, len(visibility), PUB_SUPER)
    return None


def field_edits(item: Item, text: str, reach: Reach) -> list[tuple[int, int, str]]:
    """Private fields of a moved struct that code outside its module reads or builds."""
    if item.kind != "struct" or item.body is None:
        return []
    built_outside = reach.builds(item.name)
    edits = []
    body = text[item.body[0] : item.body[1]]
    for match in FIELD.finditer(body):
        field_name = match.group(1)
        if field_name == "pub" or body[match.start() : match.end()].lstrip().startswith("pub"):
            continue
        if built_outside or reach.reads(field_name):
            edits.append((item.body[0] + match.start(1), 0, PUB_SUPER + " "))
    return edits


def signature(text: str, item: Item) -> str:
    end = item.body[0] if item.kind == "fn" and item.body else item.end
    return text[item.head : end]


def expose_signatures(text: str, child: Child, reach: Reach) -> None:
    """Record the signatures of the child's items that code outside it can name.

    A private type named in one of them has to become as visible as the item.
    """
    seen: set[int] = set()
    changed = True
    while changed:
        changed = False
        for moved in child.moved:
            whole_impl = moved.owner is None and moved.item.kind == "impl"
            if whole_impl and is_trait_impl(moved.item):
                continue
            for item in moved.item.methods if whole_impl else [moved.item]:
                if item.start in seen or item.kind not in WIDENABLE:
                    continue
                if item.visibility or reach.names(item.name):
                    seen.add(item.start)
                    reach.exposed.update(WORD.findall(signature(text, item)))
                    changed = True


def widened(text: str, moved: Moved, reach: Reach, deeper: bool, stays: frozenset[str]) -> str:
    """The moved text, one module deeper: visibility kept, relative paths lengthened."""
    item = moved.item
    edits = []
    if moved.owner is not None or item.kind != "impl":
        edit = visibility_edit(item, reach)
        if edit:
            edits.append(edit)
        edits += field_edits(item, text, reach)
    elif not is_trait_impl(item):
        for method in item.methods:
            edit = visibility_edit(method, reach)
            if edit:
                edits.append(edit)
    body = text[moved.start : item.end]
    for position, length, replacement in sorted(edits, reverse=True):
        at = position - moved.start
        body = body[:at] + replacement + body[at + length :]
    if deeper:
        body = INCLUDE.sub(lambda match: match.group(0) + "../", body)
    # Whatever was visible to the old parent's parent still is.
    body = re.sub(r"\bpub\((?:in )?super\)", PUB_GRAND, body)
    return deepen_paths(body, stays if item.kind == "mod" else frozenset()).replace(PUB_SUPER, "pub(super)").replace(PUB_GRAND, "pub(in super::super)")


def deepen_paths(body: str, stays: frozenset[str] = frozenset()) -> str:
    """Relative paths one module deeper. Inside a moved inline module, `super::Name`
    keeps naming an item that moved with it (`stays`)."""
    body = SUPER_PATH.sub(lambda match: "super::" if match.group(1) in stays else "super::super::", body)
    return SELF_PATH.sub("super::", body)


def impl_opening(text: str, owner: Item) -> str:
    attributes = [line for line in text[owner.start : owner.head].split("\n") if line.strip().startswith("#[")]
    assert owner.body is not None
    header = text[owner.head : owner.body[0] - 1]
    return "".join(line + "\n" for line in attributes) + header


def plan_file(root: Path, plan: dict, write: bool) -> list[tuple[str, int]]:
    parent = root / plan["file"]
    # A crate-root integration test (`tests/name.rs`) becomes `tests/name/main.rs` first.
    origin = root / plan["move_from"] if "move_from" in plan and not parent.exists() else parent
    text = origin.read_text(encoding="utf-8")
    if origin != parent:
        # One directory deeper: every relative `#[path]` and include climbs one more level.
        text = INCLUDE.sub(lambda match: match.group(0) + "../", re.sub(r'#\[path = "(?!/)', '#[path = "../', text))
    directory = child_dir(parent)
    header = licence_header(text)
    text, hoisted = hoist_modules(text, plan.get("hoist", []), deeper=parent.stem not in ROOT_STEMS)
    for name, _ in hoisted:
        if (directory / f"{name}.rs").exists():
            raise PlanError(f"hoist: {(directory / name).relative_to(root)}.rs already exists")
    items = parse_items(text)
    children = build_children(plan, text, items)
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

    production_neighbours = Census.of("")
    test_neighbours = Census.of("")
    test_modules = set(TEST_MODULE.findall(text))
    is_root = parent.stem in ROOT_STEMS
    if directory.is_dir():
        for path in sorted(directory.rglob("*.rs")):
            if path == parent or "target" in path.parts:
                continue
            top = path.relative_to(directory).parts[0].removesuffix(".rs")
            census = Census.of(path.read_text(encoding="utf-8", errors="replace"))
            if top in test_modules or (is_root and is_test_file(path)):
                test_neighbours += census
            else:
                production_neighbours += census
    whole = Census.of(text)

    parent_is_test = is_test_file(parent)
    # A file loaded through `#[path]` looks for its child modules beside itself.
    path_loaded = not is_root and parent.resolve() in path_loaded_files(root)
    declarations = []
    outputs = []
    for child in children:
        own = Census.of("".join(moved_text(text, moved) for moved in child.moved))
        production_outside = whole - own + production_neighbours
        parts = [header, f"//! {child.doc}\n\n" if child.doc else "", "use super::*;\n"]
        open_impl: Item | None = None
        imports: dict[str, list[str]] = {}
        reach = Reach(production_outside + test_neighbours)
        stays = frozenset(moved.item.name for moved in child.moved if moved.owner is None and moved.item.kind in WIDENABLE)
        expose_signatures(text, child, reach)
        for moved in child.moved:
            item = moved.item
            in_production = production_outside.words[item.name] > 0
            in_tests = test_neighbours.words[item.name] > 0
            body = widened(text, moved, reach, deeper=not is_root, stays=stays)
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
                # An import is compiled under the same conditions as the item it names.
                gate = "".join(line + "\n" for line in CFG_ATTRIBUTE.findall(text[moved.start : item.head]))
                if visibility:
                    imports.setdefault(f"{gate}{visibility} ", []).append(item.name)
                elif in_production:
                    imports.setdefault(gate, []).append(item.name)
                elif in_tests:
                    imports.setdefault(gate or "#[cfg(test)]\n", []).append(item.name)
        if open_impl is not None:
            parts.append("}\n")
        outputs.append((directory / f"{child.name}.rs", "".join(parts)))
        attribute = "#[cfg(test)]\n" if not parent_is_test and all(test_only(text, moved) for moved in child.moved) else ""
        if path_loaded:
            attribute += f'#[path = "{parent.stem}/{child.name}.rs"]\n'
        declarations.append((child.name, attribute, imports))

    # Declarations go after the leading `use`/`mod` run, and after any macro the children may use.
    anchor = 0
    for item in items:
        if item.kind in ("use", "mod", "extern crate"):
            anchor = item.end
        else:
            break
    macros = [item.end for item in items if item.kind in ("macro_rules", "macro")]
    anchor = max([anchor, *macros])
    if anchor == 0:
        anchor = prologue_end(text)
    offset = anchor - max([removed for at, removed in removed_before if at <= anchor], default=0)
    block = "\n\n" + "".join(f"{attribute}mod {name};\n" for name, attribute, _ in declarations)
    uses = []
    for name, _, imports in declarations:
        for prefix, names in sorted(imports.items()):
            listed = ", ".join(sorted(set(names)))
            uses.append(f"{prefix}use {name}::{{{listed}}};" if len(set(names)) > 1 else f"{prefix}use {name}::{listed};")
    if uses:
        block += "\n" + "".join(line + "\n" for line in uses)
    new_parent = remaining[:offset] + block + remaining[offset:]

    outputs += [(directory / f"{name}.rs", header + body) for name, body in hoisted]
    sizes = [(parent.relative_to(root).as_posix(), new_parent.count("\n") + 1)]
    sizes += [(path.relative_to(root).as_posix(), content.count("\n") + 1) for path, content in outputs]
    if write:
        directory.mkdir(parents=True, exist_ok=True)
        for path, content in outputs:
            path.write_text(content, encoding="utf-8")
        parent.write_text(new_parent, encoding="utf-8")
        if origin != parent:
            origin.unlink()
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
