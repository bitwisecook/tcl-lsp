#!/usr/bin/env python3
"""File the hand-maintained BIG-IP object specs by tmsh component.

    split_bigip_data.py [--root DIR] [--check]

`rust/tcl-registry/src/bigip/data/<module>.rs` holds one `SPECS` table per
tmsh module. A module over the line budget becomes a facade plus one file
per component (the first word after the module in the kind's header:
`ltm profile http` is component `profile`); a component still over the
budget becomes a directory with one file per following word. Each facade
lists its tables in `tables()`, and `data/mod.rs` reads them.

The specs themselves are not edited: every `BigipObjectSpec { ... }` moves
whole, in its original order within its file.
"""

import argparse
import re
import sys
from pathlib import Path

LIMIT = 2000
DATA = "rust/tcl-registry/src/bigip/data"
TABLE_OPEN = "pub static SPECS: &[BigipObjectSpec] = &[\n"
SPEC_OPEN = "    BigipObjectSpec {\n"
SPEC_CLOSE = "    },\n"
HEADER = re.compile(r'header_types: &\[\(\s*"([^"]+)",\s*"([^"]+)"')
KIND =re.compile(r'kind: "([^"]+)"')
TYPES = ("BigipObjectKindSpec", "BigipObjectSpec", "BigipPropertySpec", "ValueKind")
LONG_NUMBER = re.compile(r"(?<![\w.])\d{6,}")
KEYWORDS = frozenset(
    "abstract as async await become box break const continue do dyn else enum extern false final fn for gen if impl in "
    "let loop macro match mod move mut override priv pub ref return static struct trait true try type typeof unsafe "
    "unsized use virtual where while yield".split()
)


class Spec:
    def __init__(self, text: str) -> None:
        self.text = text
        header = HEADER.search(text)
        kind = KIND.search(text)
        if header is None or kind is None:
            raise SystemExit(f"a spec without a header or kind:\n{text[:200]}")
        self.module = header.group(1)
        self.words = header.group(2).split()
        self.kind = kind.group(1)
        self.lines = text.count("\n")


def stem(word: str) -> str:
    name = word.replace("-", "_").replace(".", "_")
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", name):
        raise SystemExit(f"`{word}` does not make a module name")
    # These keywords cannot be written as raw identifiers.
    return f"{name}_" if name in ("self", "super", "crate") else name


def declared(name: str) -> str:
    return f"r#{name}" if name in KEYWORDS else name


def licence(text: str) -> str:
    lines = []
    for line in text.split("\n"):
        if not line.startswith("//") or line.startswith("//!"):
            break
        lines.append(line)
    return "\n".join(lines) + "\n\n"


def parse(text: str) -> tuple[str, list[Spec]]:
    start = text.index(TABLE_OPEN)
    body = text[start + len(TABLE_OPEN) :]
    if not body.rstrip("\n").endswith("];"):
        raise SystemExit("the table does not end the file")
    body = body.rstrip("\n")[: -len("];")]
    specs = []
    cursor = 0
    while cursor < len(body):
        if not body.startswith(SPEC_OPEN, cursor):
            raise SystemExit(f"unexpected text between specs: {body[cursor:cursor + 80]!r}")
        close = body.index("\n" + SPEC_CLOSE, cursor) + 1 + len(SPEC_CLOSE)
        specs.append(Spec(body[cursor:close]))
        cursor = close
    return text[:start], specs


def leaf(licence_text: str, title: str, specs: list[Spec], visibility: str) -> str:
    body = "".join(spec.text for spec in specs)
    used = [name for name in TYPES if re.search(rf"\b{name}\b", body)]
    code_lines = [line for line in body.split("\n") if '"' not in line]
    allow = "// Large tmsh bounds appear as bare f64 literals.\n#![allow(clippy::unreadable_literal)]\n" if any(LONG_NUMBER.search(line) for line in code_lines) else ""
    return (
        f"{licence_text}//! BIG-IP object specs for `{title}`. Hand-maintained: edit them directly.\n\n"
        f"{allow}use crate::bigip::{{{', '.join(used)}}};\n\n"
        f"{visibility}static SPECS: &[BigipObjectSpec] = &[\n{body}];\n"
    )


PROPERTIES_OPEN = "        properties: &[\n"
PROPERTIES_CLOSE = "        ],\n"
PROPERTY_OPEN = "            BigipPropertySpec {\n"
PROPERTY_CLOSE = "            },\n"
IN_SECTIONS = re.compile(r"^                in_sections: &\[\s*\S", re.M)
PROPERTY_NAME = re.compile(r'^                name: "([^"]+)"', re.M)
PART_LINES = 1400


SECTIONS = re.compile(r"^                in_sections: &\[(.*?)\]", re.M | re.S)


def section_depth(text: str) -> int:
    """How many sections deep a property sits: 0 for a top-level property."""
    found = SECTIONS.search(text)
    return found.group(1).count('"') // 2 if found else 0


def properties_of(body: str) -> list[tuple[str, int, str]]:
    """`(name, section depth, text)` of each property, in order."""
    properties = []
    cursor = 0
    while cursor < len(body):
        if not body.startswith(PROPERTY_OPEN, cursor):
            raise SystemExit(f"unexpected text between properties: {body[cursor:cursor + 80]!r}")
        close = body.index("\n" + PROPERTY_CLOSE, cursor) + 1 + len(PROPERTY_CLOSE)
        text = body[cursor:close]
        name = PROPERTY_NAME.search(text)
        if name is None:
            raise SystemExit("a property without a name")
        properties.append((name.group(1), section_depth(text), text))
        cursor = close
    return properties


def property_groups(properties: list[tuple[str, int, str]], depth: int) -> list[tuple[str, list[tuple[str, int, str]]]]:
    """Groups led by a property at most `depth` sections deep; the deeper
    properties that follow it belong to its sections."""
    groups: list[tuple[str, list[tuple[str, int, str]]]] = []
    for name, own_depth, text in properties:
        if own_depth <= depth or not groups:
            groups.append((name, []))
        groups[-1][1].append((name, own_depth, text))
    return groups


def lines_in(properties: list[tuple[str, int, str]]) -> int:
    return sum(text.count("\n") for _, _, text in properties)


def property_parts(properties: list[tuple[str, int, str]], depth: int, prefix: str) -> list[tuple[str, str, list[tuple[str, int, str]]]]:
    """`(module name, description, properties)` parts of at most `PART_LINES`
    lines, cut between groups; a group too large on its own is cut one
    section deeper."""
    parts: list[tuple[str, str, list[tuple[str, int, str]]]] = []
    pending: list[tuple[str, list[tuple[str, int, str]]]] = []

    def flush() -> None:
        if not pending:
            return
        first, last = pending[0][0], pending[-1][0]
        name = stem(first) if first == last else f"{stem(first)}_through_{stem(last)}"
        if prefix == f"{stem(first)}_":
            # The part that starts with the section's own property is named for the section.
            name = name.removeprefix(prefix) if first != last else ""
            name = (prefix + name).rstrip("_") if name else prefix.rstrip("_")
        else:
            name = prefix + name
        span = f"`{first}`" if first == last else f"`{first}` to `{last}`"
        parts.append((name, span, [row for _, rows in pending for row in rows]))
        pending.clear()

    for group in property_groups(properties, depth):
        if lines_in(group[1]) > PART_LINES and len(group[1]) > 1 and depth < 4:
            flush()
            inner = property_parts(group[1], depth + 1, f"{prefix}{stem(group[0])}_")
            # The first inner part starts with the group's own property: name it for the group.
            parts.extend(inner)
            continue
        if pending and sum(lines_in(rows) for _, rows in pending) + lines_in(group[1]) > PART_LINES:
            flush()
        pending.append(group)
    flush()
    return parts


def kind_in_parts(path: Path, licence_text: str, title: str, spec: Spec, visibility: str, out: dict[Path, str]) -> None:
    """One kind too large for a file: its property table is filed by ranges of
    top-level properties and joined by `join_property_tables`."""
    start = spec.text.index(PROPERTIES_OPEN)
    end = spec.text.rindex(PROPERTIES_CLOSE)
    parts = property_parts(properties_of(spec.text[start + len(PROPERTIES_OPEN) : end]), 0, "")
    names = []
    for name, span, rows in parts:
        names.append(name)
        body = "".join(text for _, _, text in rows)
        used = [type_name for type_name in ("BigipPropertySpec", "ValueKind") if re.search(rf"\b{type_name}\b", body)]
        allow = "// Large tmsh bounds appear as bare f64 literals.\n#![allow(clippy::unreadable_literal)]\n" if any(LONG_NUMBER.search(line) for line in body.split("\n") if '"' not in line) else ""
        out[path.with_suffix("") / f"{name}.rs"] = (
            f"{licence_text}//! Properties {span} of `{title}`, each with the properties of its sections.\n\n"
            f"{allow}use crate::bigip::{{{', '.join(used)}}};\n\n"
            f"pub(super) const PROPERTIES: &[BigipPropertySpec] = &[\n{body}];\n"
        )
    mods = "".join(f"mod {declared(name)};\n" for name in sorted(names))
    listed = ", ".join(f"{declared(name)}::PROPERTIES" for name in names)
    count = " + ".join(f"{declared(name)}::PROPERTIES.len()" for name in names)
    head = spec.text[:start] + "        properties: &PROPERTIES,\n" + spec.text[end + len(PROPERTIES_CLOSE) :]
    out[path] = (
        f"{licence_text}//! BIG-IP object spec for `{title}`. Hand-maintained: edit it directly.\n//!\n"
        "//! The kind is too large for one file, so its property table is filed by\n"
        "//! ranges of top-level properties and joined here.\n\n"
        "use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, join_property_tables};\n\n"
        f"{mods}\nconst PROPERTY_COUNT: usize = {count};\n\n"
        "/// The kind's properties, in filing order.\n"
        f"static PROPERTIES: [BigipPropertySpec; PROPERTY_COUNT] = join_property_tables(&[{listed}]);\n\n"
        f"{visibility}static SPECS: &[BigipObjectSpec] = &[\n{head}];\n"
    )


def lines_of(specs: list[Spec]) -> int:
    return sum(spec.lines for spec in specs) + 30


def layout(path: Path, title: str, specs: list[Spec], depth: int, licence_text: str, docs: str, out: dict[Path, str], top: bool) -> str:
    """Write `path` (a leaf or a facade with children); return the expression for its tables."""
    groups: dict[str, list[Spec]] = {}
    for spec in specs:
        word = spec.words[depth] if depth < len(spec.words) else spec.words[-1]
        groups.setdefault(word, []).append(spec)
    visibility = "pub " if top else "pub(super) "
    if len(groups) == 1 and lines_of(specs) > LIMIT and any(depth + 1 < len(spec.words) for spec in specs):
        # Every kind here shares this word: file by the next one instead.
        return layout(path, title, specs, depth + 1, licence_text, docs, out, top)
    if len(specs) == 1 and lines_of(specs) > LIMIT:
        kind_in_parts(path, licence_text, title, specs[0], visibility, out)
        return "leaf"
    if lines_of(specs) <= LIMIT or len(groups) == 1:
        out[path] = leaf(licence_text, title, specs, visibility)
        return "leaf"
    names = {word: stem(word) for word in groups}
    if len(set(names.values())) != len(names):
        raise SystemExit(f"{path}: two components share a module name")
    rows = []
    for word, members in groups.items():
        child = path.with_suffix("") / f"{names[word]}.rs"
        kind = layout(child, f"{title} {word}", members, depth + 1, licence_text, "", out, top=False)
        name = declared(names[word])
        rows.append((name, f"vec![{name}::SPECS]" if kind == "leaf" else f"{name}::tables()"))
    mods = "".join(f"mod {name};\n" for name, _ in sorted(rows))
    # Runs of plain tables, and the lists of the components that are themselves filed by word.
    segments: list[str] = []
    run: list[str] = []
    for name, expression in rows:
        if expression.startswith("vec!"):
            run.append(f"{name}::SPECS")
            continue
        if run:
            segments.append("vec![" + ", ".join(run) + "]")
            run = []
        segments.append(f"{name}::tables()")
    if run:
        segments.append("vec![" + ", ".join(run) + "]")
    body = segments[0] if len(segments) == 1 else "[" + ", ".join(segments) + "].concat()"
    heading = docs or f"//! BIG-IP object specs for `{title}`, one file per following tmsh word.\n"
    out[path] = (
        f"{licence_text}{heading}\nuse crate::bigip::BigipObjectSpec;\n\n{mods}\n"
        f"/// The spec tables of `{title}`, in the order they are filed.\n#[must_use]\n"
        f"{visibility}fn tables() -> Vec<&'static [BigipObjectSpec]> {{\n    {body}\n}}\n"
    )
    return "facade"


def module_docs(prefix: str) -> str:
    docs = [line for line in prefix.split("\n") if line.startswith("//!")]
    return "\n".join(docs) + "\n//!\n//! One file per tmsh component (the word after the module); a large\n//! component has one file per following word.\n"


def plan(root: Path) -> tuple[dict[Path, str], list[str]]:
    directory = root / DATA
    out: dict[Path, str] = {}
    split = []
    kinds: list[str] = []
    for path in sorted(directory.glob("*.rs")):
        if path.name == "mod.rs":
            continue
        text = path.read_text(encoding="utf-8")
        kinds += re.findall(r'kind_spec:\s*BigipObjectKindSpec\s*\{\s*kind: "([^"]+)"', text)
        if text.count("\n") + 1 <= LIMIT:
            continue
        prefix, specs = parse(text)
        module = specs[0].module
        if layout(path, module, specs, 0, licence(text), module_docs(prefix), out, top=True) == "facade":
            split.append(path.stem)
    if kinds != sorted(kinds):
        raise SystemExit("the tables are not in kind order today; all_specs() cannot simply sort")
    mod_rs = directory / "mod.rs"
    text = mod_rs.read_text(encoding="utf-8")
    for name in split:
        old = f"    v.extend({name}::SPECS.iter());\n"
        if old not in text:
            raise SystemExit(f"mod.rs does not read {name}::SPECS")
        text = text.replace(old, f"    v.extend({name}::tables().into_iter().flatten());\n")
    if split and "v.sort_by_key" not in text:
        text = text.replace("    v\n}", "    // A module filed by component lists its tables in filing order.\n    v.sort_by_key(|spec| spec.kind_spec.kind);\n    v\n}")
    out[mod_rs] = text
    return out, split


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--check", action="store_true", help="print the resulting files; write nothing")
    arguments = parser.parse_args()
    out, split = plan(arguments.root)
    over = 0
    for path, content in sorted(out.items()):
        count = content.count("\n") + 1
        over += count > LIMIT
        if arguments.check or count > LIMIT:
            print(f"{count:6}  {path.relative_to(arguments.root)}{'  OVER' if count > LIMIT else ''}")
        if not arguments.check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
    print(f"{len(split)} modules filed by component, {len(out) - 1} files, {over} over the budget")
    return 0


if __name__ == "__main__":
    sys.exit(main())
