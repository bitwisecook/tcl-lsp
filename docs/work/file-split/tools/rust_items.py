#!/usr/bin/env python3
"""Top-level items of a Rust source file, and the methods of its impl blocks.

Shared by `outline.py` (what is in a file) and `apply_split.py` (move it).
An item's id is `kind:name`; a method's is `impl:<header>::name`. Repeated
ids get `#2`, `#3` in file order.
"""

import re
from dataclasses import dataclass, field

from hoist_inline_tests import matching_brace

ITEM = re.compile(
    r"^(?P<vis>pub(?:\([^)]*\))?\s+)?(?:(?:default|async|const|unsafe|extern\s+\"[^\"]*\")\s+)*"
    r"(?P<kind>fn|struct|enum|union|trait|type|const|static|mod|impl|use|macro_rules!|extern crate)\b\s*(?P<rest>.*)",
    re.S,
)
NAME = re.compile(r"(?:mut\s+)?(r#)?(\w+)")


@dataclass
class Item:
    kind: str
    name: str
    start: int  # offset of the first leading attribute or doc line
    head: int  # offset of the item keyword line
    end: int  # offset just past the item
    visibility: str = ""
    body: tuple[int, int] | None = None  # offsets inside the braces
    methods: list["Item"] = field(default_factory=list)
    ident: str = ""


def _skip_trivia(text: str, index: int) -> int:
    while index < len(text):
        if text[index] in " \t\r\n":
            index += 1
        elif text.startswith("//", index) and not text.startswith(("///", "//!"), index):
            index = text.find("\n", index)
            index = len(text) if index < 0 else index
        else:
            break
    return index


def _item_end(text: str, head: int, to_semicolon: bool = False) -> tuple[int, tuple[int, int] | None]:
    """End of the item starting at `head`: a `;` or a balanced `{...}` at depth 0.

    A `const`, `static` or `type` item always ends at its `;`, whatever
    braces its value holds.
    """
    depth = 0
    index = head
    while index < len(text):
        char = text[index]
        if text.startswith("//", index):
            index = text.find("\n", index)
            continue
        if char in '"\'' or (char == "r" and text[index + 1 : index + 2] in '#"'):
            probe = "{" + text[index:]
            # Reuse the literal skipping of the brace scan on a one-token probe.
            end = _literal_end(text, index)
            if end > index:
                index = end
                continue
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        elif char == "{" and depth == 0 and to_semicolon:
            index = matching_brace(text, index)
        elif char == "{" and depth == 0:
            close = matching_brace(text, index)
            tail = close + 1
            if text[tail : tail + 1] == ";":
                tail += 1
            return tail, (index + 1, close)
        elif char == ";" and depth == 0:
            return index + 1, None
        index += 1
    raise ValueError("unterminated item")


def _literal_end(text: str, index: int) -> int:
    char = text[index]
    if char == "r":
        cursor = index + 1
        hashes = 0
        while text[cursor : cursor + 1] == "#":
            hashes += 1
            cursor += 1
        if text[cursor : cursor + 1] != '"' or (index and (text[index - 1].isalnum() or text[index - 1] == "_") and text[index - 1] not in "bc"):
            return index
        end = text.find('"' + "#" * hashes, cursor + 1)
        return end + 1 + hashes
    if char == '"':
        cursor = index + 1
        while text[cursor] != '"':
            cursor += 2 if text[cursor] == "\\" else 1
        return cursor + 1
    if text[index + 1 : index + 2] == "\\":
        return text.find("'", index + 3) + 1
    if text[index + 2 : index + 3] == "'":
        return index + 3
    return index


def parse_items(text: str, start: int = 0, stop: int | None = None, owner: str = "") -> list[Item]:
    stop = len(text) if stop is None else stop
    items: list[Item] = []
    index = start
    while True:
        index = _skip_trivia(text, index)
        if index >= stop:
            break
        first = index
        # Leading doc comments and attributes belong to the item.
        while True:
            if text.startswith(("///", "//!"), index):
                index = text.find("\n", index) + 1
            elif text.startswith("#[", index) or text.startswith("#![", index):
                bracket = text.index("[", index)
                depth = 0
                cursor = bracket
                while True:
                    if text[cursor] == "[":
                        depth += 1
                    elif text[cursor] == "]":
                        depth -= 1
                        if depth == 0:
                            break
                    elif text[cursor] == '"':
                        cursor = _literal_end(text, cursor) - 1
                    cursor += 1
                index = cursor + 1
            else:
                break
            index = _skip_trivia(text, index)
        head = index
        line_end = text.find("{", head)
        semicolon = text.find(";", head)
        header_stop = min(x for x in (line_end, semicolon, stop) if x >= 0)
        header = text[head:header_stop]
        match = ITEM.match(header)
        end, body = _item_end(text, head, bool(match) and match.group("kind") in ("const", "static", "type"))
        if match:
            kind = match.group("kind").rstrip("!")
            rest = match.group("rest").strip()
            if kind == "impl":
                name = " ".join(rest.split())
            elif kind == "use":
                name = " ".join(rest.split())
            else:
                found = NAME.match(rest)
                name = found.group(2) if found else rest[:40]
            visibility = (match.group("vis") or "").strip()
        else:
            kind, name, visibility = "macro", " ".join(header.split())[:60], ""
        item = Item(kind, name, first, head, end, visibility, body)
        if kind == "impl" and body and not owner:
            item.methods = parse_items(text, body[0], body[1], owner=name)
        items.append(item)
        index = end
    counts: dict[str, int] = {}
    for item in items:
        base = f"impl:{owner}::{item.name}" if owner else f"{item.kind}:{item.name}"
        counts[base] = counts.get(base, 0) + 1
        item.ident = base if counts[base] == 1 else f"{base}#{counts[base]}"
    return items


def line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1
