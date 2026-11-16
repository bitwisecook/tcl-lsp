#!/usr/bin/env python3
"""Follow moved declarations in the owner-resolution contract.

    update_contract.py [--root DIR] [--base REV] [--check]

`docs/design/contracts/shared-utility-contracts-rust.md` names, for every
owner, the file that declares each entry point and the file and module of
each internal route. After a split an entry lives in a child module of the
file the contract names. This rewrites those rows from the tree as it
stands: an owner row gains the child file; a route row gets the child's
path, module and the declaration's visibility. Only files the split created
(untracked, or added since `--base`) are considered, and rows whose entries
are still where the contract says are left alone, so it is safe to re-run.
"""

import argparse
import re
import subprocess
import sys
from functools import cache
from pathlib import Path

from apply_split import ROOT_STEMS
from rust_items import Item, parse_items

CONTRACT = "docs/design/contracts/shared-utility-contracts-rust.md"
OWNER = ("<!-- owner-resolution-manifest -->", "<!-- end-owner-resolution-manifest -->")
ROUTES = ("<!-- owner-resolution-source-manifest -->", "<!-- end-owner-resolution-source-manifest -->")
DECLARING = ("fn", "struct", "enum", "union", "trait", "type", "const", "static", "mod")
CODE = re.compile(r"`([^`]+)`")
FIELD = re.compile(r"^[ \t]*(pub(?:\([^)]*\))?[ \t]+)?(?:r#)?(\w+)[ \t]*:(?!:)", re.M)


@cache
def items_of(path: Path) -> tuple[str, tuple[Item, ...]]:
    text = path.read_text(encoding="utf-8", errors="replace")
    try:
        return text, tuple(parse_items(text))
    except (ValueError, IndexError):
        return text, ()


def visibility_name(visibility: str) -> str:
    return visibility or "private"


def declared(path: Path, entry: str) -> str | None:
    """The visibility `entry` is declared with in `path`, as the gate spells it."""
    text, items = items_of(path)
    owner, _, ident = entry.rpartition("::")
    owner = owner.rsplit("::", 1)[-1]
    if owner == path.stem:
        owner = ""
    for item in items:
        if not owner and item.kind in DECLARING and item.name == ident:
            return visibility_name(item.visibility)
        if item.kind == "impl" and (not owner or re.search(rf"(?<!\w){re.escape(owner)}(?!\w)", item.name)):
            for method in item.methods:
                if method.name == ident:
                    return visibility_name(method.visibility)
        if owner and item.name == owner and item.body:
            body = text[item.body[0] : item.body[1]]
            if item.kind == "struct":
                for match in FIELD.finditer(body):
                    if match.group(2) == ident:
                        return visibility_name((match.group(1) or "").strip())
            if item.kind in ("enum", "trait") and re.search(rf"(?<!\w){re.escape(ident)}(?!\w)", body):
                return "pub" if item.visibility == "pub" else visibility_name(item.visibility)
    return None


CREATED: set[Path] = set()


def created_files(root: Path, base: str) -> set[Path]:
    """Rust files the split created: untracked, or added since `base`."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "--others", "--exclude-standard", "--", "*.rs"],
        check=True, capture_output=True, text=True,
    ).stdout.split("\n")
    listed += subprocess.run(
        ["git", "-C", str(root), "diff", "--name-only", "--diff-filter=A", base, "--", "*.rs"],
        check=True, capture_output=True, text=True,
    ).stdout.split("\n")
    return {root / name for name in listed if name}


def module_files(root: Path, source: str) -> list[Path]:
    """The module files the split created beneath `source`, nearest first."""
    path = root / source
    directory = path.parent if path.stem in ROOT_STEMS else path.with_suffix("")
    if not directory.is_dir():
        return []
    files = [candidate for candidate in directory.rglob("*.rs") if candidate != path and candidate in CREATED]
    return sorted(files, key=lambda candidate: (len(candidate.parts), candidate.as_posix()))


def child_module(root: Path, source: str, module: str, child: Path) -> str:
    path = root / source
    directory = path.parent if path.stem in ROOT_STEMS else path.with_suffix("")
    parts = list(child.relative_to(directory).with_suffix("").parts)
    if parts and parts[-1] == "mod":
        parts.pop()
    return "::".join([module, *parts])


def find_child(root: Path, source: str, entry: str, exported: bool) -> tuple[Path, str] | None:
    for child in module_files(root, source):
        found = declared(child, entry)
        if found is not None and (found == "pub" or not exported):
            return child, found
    return None


def cells_of(line: str) -> list[str]:
    return line.split("|")


def update_owner_row(root: Path, line: str) -> str:
    cells = cells_of(line)
    if len(cells) != 7:
        return line
    sources = [source for source in CODE.findall(cells[2]) if source.endswith(".rs")]
    added: list[str] = []
    for entry in CODE.findall(cells[3]):
        if any((root / source).exists() and declared(root / source, entry) == "pub" for source in sources + added):
            continue
        for source in sources:
            found = find_child(root, source, entry, exported=True) or find_child(root, source, entry, exported=False)
            if found:
                added.append(found[0].relative_to(root).as_posix())
                break
    if not added:
        return line
    cells[2] = cells[2].rstrip() + "".join(f"; `{path}`" for path in dict.fromkeys(added)) + " "
    return "|".join(cells)


def update_route_row(root: Path, line: str) -> str:
    cells = cells_of(line)
    if len(cells) != 12:
        return line
    value = [CODE.findall(cell) for cell in cells]

    def one(index: int) -> str | None:
        return value[index][0] if len(value[index]) == 1 else None

    source, module, entry = one(2), one(4), one(5)
    if source and module and entry and (root / source).exists() and declared(root / source, entry) is None:
        found = find_child(root, source, entry, exported=False)
        if found:
            child, visibility = found
            cells[2] = f" `{child.relative_to(root).as_posix()}` "
            cells[4] = f" `{child_module(root, source, module, child)}` "
            cells[6] = f" {visibility} "
    consumer_source, consumer_module, consumer = one(7), one(8), one(9)
    if consumer_source and consumer_module and consumer and (root / consumer_source).exists():
        if declared(root / consumer_source, consumer) is None:
            found = find_child(root, consumer_source, consumer, exported=False)
            if found:
                child, _ = found
                cells[7] = f" `{child.relative_to(root).as_posix()}` "
                cells[8] = f" `{child_module(root, consumer_source, consumer_module, child)}` "
    return "|".join(cells)


def update(root: Path, markdown: str) -> str:
    lines = markdown.split("\n")
    section = None
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped == OWNER[0]:
            section = "owner"
        elif stripped == ROUTES[0]:
            section = "routes"
        elif stripped in (OWNER[1], ROUTES[1]):
            section = None
        elif section and line.startswith("| ") and not line.startswith("| ---"):
            lines[index] = update_owner_row(root, line) if section == "owner" else update_route_row(root, line)
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--check", action="store_true", help="report how many rows would change; write nothing")
    parser.add_argument("--base", default="HEAD", help="the commit before the split (default HEAD)")
    arguments = parser.parse_args()
    CREATED.update(created_files(arguments.root, arguments.base))
    path = arguments.root / CONTRACT
    before = path.read_text(encoding="utf-8")
    after = update(arguments.root, before)
    changed = sum(1 for old, new in zip(before.split("\n"), after.split("\n")) if old != new)
    print(f"{changed} contract row(s) follow a moved declaration")
    if changed and not arguments.check:
        path.write_text(after, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
