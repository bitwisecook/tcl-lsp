#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Lossless, human-readable blocks for Tcl execution evidence."""

from __future__ import annotations

import re
import unicodedata
from pathlib import Path

OPEN = "⟦"
CLOSE = "⟧"
LANGUAGES = {"tcl-visible", "text-visible"}


def _marker(name: str) -> str:
    return f"{OPEN}{name}{CLOSE}"


def _render_line(line: str) -> str:
    trailing_spaces = {
        index
        for match in re.finditer(r" +(?=\r|$)", line)
        for index in range(match.start(), match.end())
    }
    rendered: list[str] = []
    for index, char in enumerate(line):
        value = ord(char)
        if char == OPEN:
            rendered.append(_marker("LBRACKET"))
        elif char == CLOSE:
            rendered.append(_marker("RBRACKET"))
        elif char == "\u00a0":
            rendered.append(_marker("NBSP"))
        elif char == "\t":
            rendered.append(_marker("TAB"))
        elif char == "\r":
            rendered.append(_marker("CR"))
        elif char == " " and index in trailing_spaces:
            rendered.append(_marker("SPACE"))
        elif 0xDC80 <= value <= 0xDCFF:
            rendered.append(_marker(f"BYTE:{value - 0xDC00:02X}"))
        elif (
            value == 0x034F
            or 0x180B <= value <= 0x180D
            or 0xFE00 <= value <= 0xFE0F
            or 0xE0100 <= value <= 0xE01EF
        ):
            rendered.append(_marker(f"U+{value:04X}"))
        elif unicodedata.category(char)[0] == "C" or unicodedata.category(char) in {
            "Zl",
            "Zp",
            "Zs",
        }:
            # Normal interior ASCII spaces remain readable.
            rendered.append(char if char == " " else _marker(f"U+{value:04X}"))
        else:
            rendered.append(char)
    return "".join(rendered)


def visible_bytes(data: bytes) -> str:
    """Represent exact bytes with ordinary LF as Markdown line breaks."""
    text = data.decode("utf-8", errors="surrogateescape")
    lines = text.split("\n")
    rendered: list[str] = []
    for index, line in enumerate(lines):
        has_lf = index < len(lines) - 1
        if not has_lf and not line and data.endswith(b"\n"):
            break
        if has_lf and line.endswith("\r"):
            rendered.append(_render_line(line[:-1]) + _marker("CRLF") + "\n")
        elif has_lf:
            rendered.append(_render_line(line) + "\n")
        else:
            rendered.append(_render_line(line) + _marker("EOF") + "\n")
    return "".join(rendered)


def original_bytes(visible: str) -> bytes:
    """Decode a canonical visible block without normalizing its contents."""
    if not visible.endswith("\n"):
        raise ValueError("the visible block must end with a Markdown line break")
    output = bytearray()
    lines = visible.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if not line.endswith("\n"):
            raise ValueError("the visible block must use LF line breaks")
        text = line[:-1]
        ending = b"\n"
        if text.endswith(_marker("CRLF")):
            text = text[: -len(_marker("CRLF"))]
            ending = b"\r\n"
        elif text.endswith(_marker("EOF")):
            if index != len(lines) - 1:
                raise ValueError("EOF marker must appear on the last line")
            text = text[: -len(_marker("EOF"))]
            ending = b""
        position = 0
        while position < len(text):
            if text[position] != OPEN:
                output.extend(text[position].encode("utf-8"))
                position += 1
                continue
            close = text.find(CLOSE, position + 1)
            if close < 0:
                raise ValueError("unclosed evidence marker")
            token = text[position + 1 : close]
            simple = {
                "NBSP": "\u00a0".encode(),
                "SPACE": b" ",
                "TAB": b"\t",
                "CR": b"\r",
                "LBRACKET": OPEN.encode(),
                "RBRACKET": CLOSE.encode(),
            }
            if token in simple:
                output.extend(simple[token])
            elif re.fullmatch(r"BYTE:[0-9A-F]{2}", token):
                output.append(int(token[5:], 16))
            elif re.fullmatch(r"U\+[0-9A-F]{4,6}", token):
                value = int(token[2:], 16)  # noqa: FURB166 - U+ is not a Python hex prefix
                if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
                    raise ValueError("invalid Unicode scalar in evidence marker")
                output.extend(chr(value).encode("utf-8"))
            else:
                raise ValueError(f"unknown or misplaced evidence marker: {token}")
            position = close + 1
        output.extend(ending)
    result = bytes(output)
    if visible_bytes(result) != visible:
        raise ValueError("visible block is not in canonical form")
    return result


def markdown_block(data: bytes, language: str = "tcl-visible") -> str:
    """Return one fenced block, choosing a fence longer than any in the data."""
    if language not in LANGUAGES:
        raise ValueError("unsupported evidence block language")
    content = visible_bytes(data)
    longest = max((len(m.group()) for m in re.finditer(r"`+", content)), default=0)
    fence = "`" * max(3, longest + 1)
    return f"{fence}{language}\n{content}{fence}\n"


def fenced_blocks(section: str, language: str) -> list[str]:
    if language not in LANGUAGES:
        raise ValueError("unsupported evidence block language")
    pattern = rf"(?ms)^(?P<fence>`{{3,}}){re.escape(language)}[ \t]*\n(?P<body>.*?)^(?P=fence)[ \t]*$"
    return [match.group("body") for match in re.finditer(pattern, section)]


def sections(document: str, level: int = 2) -> dict[str, list[str]]:
    """Read sections without mistaking headings in code for Markdown."""
    result: dict[str, list[str]] = {}
    title: str | None = None
    content: list[str] = []
    fence: str | None = None
    for line in document.splitlines(keepends=True):
        if fence is not None:
            if re.fullmatch(
                re.escape(fence[0]) + "{" + str(len(fence)) + r",}[ \t]*\n?", line
            ):
                fence = None
        else:
            heading = re.fullmatch(
                r"#{" + str(level) + r"} ([^#\n][^\n]*?)[ \t]*\n?", line
            )
            if heading:
                if title is not None:
                    result.setdefault(title, []).append("".join(content))
                title = heading.group(1)
                content = []
                continue
            opening = re.match(r"^(`{3,}|~{3,})[^\n]*\n?$", line)
            if opening:
                fence = opening.group(1)
        if title is not None:
            content.append(line)
    if title is not None:
        result.setdefault(title, []).append("".join(content))
    return result


def outside_fences(document: str) -> str:
    """Mask fenced blocks while retaining offsets for their surrounding text."""
    result: list[str] = []
    fence: str | None = None
    for line in document.splitlines(keepends=True):
        masked = fence is not None
        if fence is not None:
            if re.fullmatch(
                re.escape(fence[0]) + "{" + str(len(fence)) + r",}[ \t]*\n?", line
            ):
                fence = None
        else:
            opening = re.match(r"^(`{3,}|~{3,})[^\n]*\n?$", line)
            if opening:
                fence = opening.group(1)
                masked = True
        if masked:
            result.append("".join("\n" if c == "\n" else " " for c in line))
        else:
            result.append(line)
    return "".join(result)


FIXTURE_LINK = re.compile(r"(?m)^Fixture: \[[^\]\n]+\]\(([^)\s]+)\)[ \t]*$")
SLOT = re.compile(r"⟦(HEX|PARAM):([A-Za-z0-9_.-]+)⟧")
INLINE_PARAM = re.compile(r"(?m)^- ([A-Za-z0-9_.-]+): `([A-Za-z0-9_.:/-]+)`[ \t]*$")
_SLOT_OPEN = ""
_SLOT_CLOSE = ""


def named_blocks(section: str) -> dict[str, bytes]:
    """Read `### name` headings that each hold one tcl-visible value."""
    values: dict[str, bytes] = {}
    for name, entries in sections(section, level=3).items():
        if len(entries) != 1:
            raise ValueError(f"duplicate value heading: {name}")
        blocks = fenced_blocks(entries[0], "tcl-visible")
        if len(blocks) != 1:
            raise ValueError(f"value {name} must contain exactly one tcl-visible block")
        values[name] = original_bytes(blocks[0])
    return values


def parse_fixture(document: str) -> tuple[str, dict[str, bytes]]:
    """Return a fixture's template block and its shared constant values."""
    found = sections(document)
    for name in ("Setup", "Template"):
        if len(found.get(name, [])) != 1 or not found[name][0].strip():
            raise ValueError(f"fixture must contain one nonempty ## {name} section")
    blocks = fenced_blocks(found["Template"][0], "tcl-visible")
    if len(blocks) != 1:
        raise ValueError("Template section must contain exactly one tcl-visible block")
    constants = named_blocks(found["Constants"][0]) if "Constants" in found else {}
    return blocks[0], constants


def compose(template: str, values: dict[str, bytes]) -> bytes:
    """Fill a template's ⟦HEX:name⟧ and ⟦PARAM:name⟧ slots with exact bytes."""
    kinds: dict[str, str] = {}

    def to_sentinel(match: re.Match[str]) -> str:
        kind, name = match.groups()
        if kinds.setdefault(name, kind) != kind:
            raise ValueError(f"slot {name} is used with two kinds")
        return f"⟦U+E000⟧{kind[0]}:{name}⟦U+E001⟧"

    raw = original_bytes(SLOT.sub(to_sentinel, template))
    missing = sorted(set(kinds) - set(values))
    unused = sorted(set(values) - set(kinds))
    if missing or unused:
        raise ValueError(f"fixture slots missing {missing} or unused {unused}")
    pattern = re.compile(
        re.escape(_SLOT_OPEN.encode())
        + rb"([HP]):([A-Za-z0-9_.-]+)"
        + re.escape(_SLOT_CLOSE.encode())
    )

    def fill(match: re.Match[bytes]) -> bytes:
        value = values[match.group(2).decode()]
        return value.hex().encode() if match.group(1) == b"H" else value

    return pattern.sub(fill, raw)


def template_block(skeleton: bytes) -> str:
    """Render bytes holding ⟦U+E000⟧H:name⟦U+E001⟧ sentinels as template slots."""
    visible = visible_bytes(skeleton)
    return re.sub(
        r"⟦U\+E000⟧([HP]):([A-Za-z0-9_.-]+)⟦U\+E001⟧",
        lambda m: f"⟦{'HEX' if m.group(1) == 'H' else 'PARAM'}:{m.group(2)}⟧",
        visible,
    )


def script_from_markdown(document: str, path: Path | None = None) -> bytes:
    """Extract the exact executed script, composing it from a fixture if linked."""
    script_sections = sections(document).get("Script", [])
    if len(script_sections) != 1:
        raise ValueError("evidence must have exactly one ## Script section")
    section = script_sections[0]
    link = FIXTURE_LINK.findall(outside_fences(section))
    if not link:
        blocks = fenced_blocks(section, "tcl-visible")
        if len(blocks) != 1:
            raise ValueError(
                "Script section must contain exactly one tcl-visible block"
            )
        return original_bytes(blocks[0])
    if len(link) != 1:
        raise ValueError("Script section must link exactly one fixture")
    if path is None:
        raise ValueError("a fixture-based script needs its evidence file path")
    template, values = parse_fixture(
        (path.parent / link[0]).read_text(encoding="utf-8")
    )
    own = named_blocks(section)
    own.update(
        (k, v.encode()) for k, v in INLINE_PARAM.findall(outside_fences(section))
    )
    overlap = sorted(set(own) & set(values))
    if overlap:
        raise ValueError(f"case values repeat fixture constants: {overlap}")
    values.update(own)
    return compose(template, values)


def validate_document(document: str, path: Path | None = None) -> list[str]:
    """Check the shape and reversible blocks of one execution evidence file."""
    errors: list[str] = []
    if "\r" in document:
        errors.append("Markdown must use LF line endings")
    if not re.search(r"(?m)^# [^#\n].+", outside_fences(document)):
        errors.append("missing evidence title")
    all_sections = sections(document)
    for name in ("Question", "Assumptions", "Script", "Results"):
        items = all_sections.get(name, [])
        if len(items) != 1 or not items[0].strip():
            errors.append(f"evidence must contain one nonempty ## {name} section")
    try:
        script_from_markdown(document, path)
    except (OSError, ValueError) as error:
        errors.append(str(error))
    results = all_sections.get("Results", [])
    if len(results) != 1:
        return errors
    environments = sections(results[0], level=3)
    if not environments:
        errors.append("Results must contain at least one ### environment")
    for name, entries in environments.items():
        if len(entries) != 1:
            errors.append(f"duplicate environment heading: {name}")
            continue
        body = entries[0]
        structure = outside_fences(body)
        status = list(re.finditer(r"(?m)^- Exit status:[ \t]*(.+)$", structure))
        if len(status) != 1 or not status[0].group(1).strip():
            errors.append(f"{name}: record one exit status or Not recorded")
        for field in ("Standard output", "Standard error", "Recorded result"):
            matches = list(
                re.finditer(rf"(?m)^- {re.escape(field)}:[ \t]*(.*)$", structure)
            )
            if field == "Recorded result" and not matches:
                continue
            if len(matches) != 1:
                errors.append(f"{name}: record {field.lower()} exactly once")
                continue
            match = matches[0]
            inline = match.group(1).strip().rstrip(".").casefold()
            if inline in {"empty", "not recorded"}:
                continue
            if inline == "decoded below" and field != "Standard error":
                continue
            if inline:
                errors.append(
                    f"{name}: {field.lower()} must be a visible block, empty, decoded below or Not recorded"
                )
                continue
            following = body[match.end() :]
            block = re.match(
                r"(?ms)\A[ \t]*\n(?:[ \t]*\n)*(?P<fence>`{3,})text-visible[ \t]*\n(?P<body>.*?)^(?P=fence)[ \t]*$",
                following,
            )
            if not block:
                errors.append(f"{name}: missing {field.lower()} text-visible block")
                continue
            try:
                original_bytes(block.group("body"))
            except ValueError as error:
                errors.append(f"{name}: invalid {field.lower()}: {error}")
    return errors


def validate_fixture(document: str) -> list[str]:
    """Check a shared fixture's sections, template and constants."""
    try:
        template, constants = parse_fixture(document)
        slots = {name for _, name in SLOT.findall(template)}
        unused = sorted(set(constants) - slots)
        if unused:
            return [f"constants not used by the template: {unused}"]
        compose(template, dict.fromkeys(slots, b"") | constants)
    except ValueError as error:
        return [str(error)]
    if not re.search(r"(?m)^# [^#\n].+", outside_fences(document)):
        return ["missing fixture title"]
    return []
