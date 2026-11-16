#!/usr/bin/env python3
"""Move top-level `#[cfg(test)] mod name { ... }` blocks of large Rust files
into their own files, leaving `#[cfg(test)] mod name;` behind.

The module path of every test is unchanged. Bodies are written as they are
and left to `cargo fmt` to re-indent, so string literals keep their content.
"""

import re
import sys
from pathlib import Path

LIMIT = 2000
ROOTS = ("rust", "runtime/rust")
MODULE_OPEN = re.compile(r"^mod (\w+) \{$")
INCLUDE = re.compile(r'\b(include_str|include_bytes|include)!\(\s*"(?!/)')
ROOT_FILES = ("mod.rs", "lib.rs", "main.rs")


def matching_brace(text: str, open_at: int) -> int:
    """Offset of the `}` closing the `{` at `open_at`, skipping literals and comments."""
    depth = 0
    index = open_at
    size = len(text)
    while index < size:
        char = text[index]
        nxt = text[index + 1] if index + 1 < size else ""
        if char == "/" and nxt == "/":
            index = text.find("\n", index)
            if index < 0:
                break
            continue
        if char == "/" and nxt == "*":
            nesting = 1
            index += 2
            while index < size and nesting:
                if text.startswith("/*", index):
                    nesting += 1
                    index += 2
                elif text.startswith("*/", index):
                    nesting -= 1
                    index += 2
                else:
                    index += 1
            continue
        if char == "r" and nxt in '#"' and raw_prefix_starts_here(text, index):
            hashes = 0
            cursor = index + 1
            while cursor < size and text[cursor] == "#":
                hashes += 1
                cursor += 1
            if cursor < size and text[cursor] == '"':
                end = text.find('"' + "#" * hashes, cursor + 1)
                if end < 0:
                    break
                index = end + 1 + hashes
                continue
        if char == '"':
            index += 1
            while index < size and text[index] != '"':
                index += 2 if text[index] == "\\" else 1
            index += 1
            continue
        if char == "'":
            if nxt == "\\":
                index = text.find("'", index + 3) + 1
                continue
            if index + 2 < size and text[index + 2] == "'":
                index += 3
                continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    raise ValueError("unbalanced braces")


def raw_prefix_starts_here(text: str, index: int) -> bool:
    """Whether the `r` at `index` begins a raw literal (`r"`, `br"`, `cr#"`), not an identifier."""
    before = text[index - 1] if index else ""
    if before in "bc":
        before = text[index - 2] if index > 1 else ""
    return not (before.isalnum() or before == "_")


PATH_ATTRIBUTE = re.compile(r'#\[path\s*=\s*"([^"]+)"\]')


def path_loaded_files(root: Path) -> set[Path]:
    """Files some module loads through `#[path]`; their child modules sit beside them."""
    loaded = set()
    for tree in ROOTS:
        for path in (root / tree).rglob("*.rs"):
            if "target" in path.parts:
                continue
            for match in PATH_ATTRIBUTE.finditer(path.read_text(encoding="utf-8", errors="replace")):
                for base in (path.parent, path.parent / path.stem):
                    candidate = (base / match.group(1)).resolve()
                    if candidate.exists():
                        loaded.add(candidate)
    return loaded


def licence_header(text: str) -> str:
    lines = []
    for line in text.split("\n"):
        if not line.startswith("//") or line.startswith("//!") or line.startswith("///"):
            break
        lines.append(line)
    header = "\n".join(lines)
    return header + "\n\n" if "SPDX" in header or "Copyright" in header else ""


def hoist(path: Path, beside: bool = False) -> list[str]:
    text = path.read_text(encoding="utf-8")
    if text.count("\n") + 1 <= LIMIT:
        return []
    beside = beside or path.name in ROOT_FILES
    child_dir = path.parent if beside else path.parent / path.stem
    deeper = not beside
    header = licence_header(text)
    moved = []
    search = 0
    while True:
        marker = text.find("\n#[cfg(test)]\n", search)
        if marker < 0:
            break
        cursor = marker + len("\n#[cfg(test)]\n")
        while text.startswith("#[", cursor) or text.startswith("///", cursor):
            cursor = text.index("\n", cursor) + 1
        line_end = text.find("\n", cursor)
        match = MODULE_OPEN.match(text[cursor:line_end]) if line_end > 0 else None
        if not match:
            search = cursor
            continue
        name = match.group(1)
        target = child_dir / f"{name}.rs"
        if target.exists():
            search = cursor
            continue
        open_at = line_end - 1
        close_at = matching_brace(text, open_at)
        body = text[open_at + 1 : close_at].strip("\n")
        if deeper:
            body = INCLUDE.sub(lambda m: m.group(0) + "../", body)
        text = text[:cursor] + f"mod {name};" + text[close_at + 1 :]
        moved.append((target, header + body + "\n"))
        search = cursor
    for target, content in moved:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    if moved:
        path.write_text(text, encoding="utf-8")
    return [target.as_posix() for target, _ in moved]


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    only = sys.argv[2:]
    loaded = path_loaded_files(root)
    for tree in ROOTS:
        for path in sorted((root / tree).rglob("*.rs")):
            if "target" in path.parts:
                continue
            relative = path.relative_to(root).as_posix()
            if only and not any(relative.startswith(prefix) for prefix in only):
                continue
            try:
                for target in hoist(path, beside=path.resolve() in loaded):
                    print(f"{relative} -> {Path(target).relative_to(root).as_posix()}")
            except ValueError as error:
                print(f"SKIPPED {relative}: {error}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
