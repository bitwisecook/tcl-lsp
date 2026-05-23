"""``ascii-blocks`` renderer — Unicode line-art block diagrams.

Walks a nested ``{title, rows}`` (or ``{label, children}``) tree and
lays it out as a stack of boxes with ``┌─┬─┐`` borders.  Designed to
feed the future APM-policy view — and useful today for any flow that
already has a tree shape (an iRule with nested ``when`` blocks, a
pool's monitor chain, an HA fail-over tree).

Accepted shapes
---------------

A single dict::

    {"title": "Access Policy: /Common/portal",
     "rows": [
       {"title": "Logon Page", "rows": [{"title": "OK"}]},
       {"title": "AD Auth", "rows": [{"title": "Success"},
                                      {"title": "Fall-back"}]},
     ]}

Or a list of such dicts — rendered as separate top-level boxes
stacked vertically.

Keys (synonyms):

- ``title`` / ``label`` / ``name`` — the box header text.
- ``rows`` / ``children`` / ``items`` — list of nested boxes.
- ``note`` / ``detail`` — optional second line in the box header.

Options:

- ``min-width`` — minimum interior width of a box, default 12.
- ``style`` — ``"rounded"`` (``╭─╮``, default) or ``"square"`` (``┌─┐``)
  or ``"ascii"`` (``+-+`` / ``|``) for terminals that mangle Unicode.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ..errors import RendererError
from ..values import ObjectRef, Stream
from . import renderer

_STYLES: dict[str, dict[str, str]] = {
    "rounded": {
        "tl": "╭",
        "tr": "╮",
        "bl": "╰",
        "br": "╯",
        "h": "─",
        "v": "│",
        "stem": "│",
        "branch": "├",
        "last": "╰",
        "elbow": "─",
    },
    "square": {
        "tl": "┌",
        "tr": "┐",
        "bl": "└",
        "br": "┘",
        "h": "─",
        "v": "│",
        "stem": "│",
        "branch": "├",
        "last": "└",
        "elbow": "─",
    },
    "ascii": {
        "tl": "+",
        "tr": "+",
        "bl": "+",
        "br": "+",
        "h": "-",
        "v": "|",
        "stem": "|",
        "branch": "+",
        "last": "+",
        "elbow": "-",
    },
}


@dataclass
class _Box:
    title: str
    note: str = ""
    children: list["_Box"] = field(default_factory=list)


@renderer(
    "ascii-blocks",
    summary="Unicode line-art block diagram of a nested {title, rows} tree.",
    accepts="dict {title, rows} or list of such dicts",
    details=(
        "Each dict becomes a bordered box; nested ``rows`` (synonyms: "
        "``children``, ``items``) become indented child boxes joined to "
        "the parent with a tree connector.  Picks the box-drawing style "
        "with --render-opt style=rounded|square|ascii (default rounded)."
    ),
)
def _render_ascii_blocks(values: list[Any], **opts: Any) -> str:
    style_name = str(opts.get("style", "rounded"))
    if style_name not in _STYLES:
        valid = ", ".join(sorted(_STYLES))
        raise RendererError(f"ascii-blocks: unknown style {style_name!r} (expected one of {valid})")
    glyphs = _STYLES[style_name]
    min_width = _parse_int(opts.get("min-width", opts.get("min_width", 12)), name="min-width")

    boxes = [_coerce_box(v) for v in _flatten(values)]
    if not boxes:
        return "(no blocks)\n"
    parts: list[str] = []
    for box in boxes:
        parts.append(_render_box(box, glyphs=glyphs, min_width=min_width))
    return "\n".join(parts)


def render_ascii_blocks(
    tree: Any,
    *,
    style: str = "rounded",
    min_width: int = 12,
) -> str:
    """Public-API entry — render *tree* directly without the registry."""
    return _render_ascii_blocks([tree], style=style, **{"min-width": min_width})


def _flatten(values: list[Any]) -> list[Any]:
    out: list[Any] = []
    for v in values:
        if isinstance(v, Stream):
            out.extend(v.items)
        else:
            out.append(v)
    return out


def _coerce_box(v: Any) -> _Box:
    if isinstance(v, _Box):
        return v
    if isinstance(v, ObjectRef):
        return _Box(title=v.full_path or v.kind, note=v.kind)
    if isinstance(v, dict):
        title = v.get("title") or v.get("label") or v.get("name")
        if title is None:
            raise RendererError(
                "ascii-blocks: dict block must carry 'title' (or 'label' / 'name'); "
                f"got {sorted(v.keys())!r}"
            )
        note = v.get("note") or v.get("detail") or ""
        kids_raw = v.get("rows") or v.get("children") or v.get("items") or []
        if not isinstance(kids_raw, (list, tuple)):
            raise RendererError(
                "ascii-blocks: 'rows' / 'children' / 'items' must be a list; "
                f"got {type(kids_raw).__name__}"
            )
        return _Box(title=str(title), note=str(note), children=[_coerce_box(k) for k in kids_raw])
    if isinstance(v, str):
        return _Box(title=v)
    raise RendererError(f"ascii-blocks: unsupported block shape {type(v).__name__}")


def _render_box(box: _Box, *, glyphs: dict[str, str], min_width: int) -> str:
    interior = _box_interior_lines(box)
    width = max(min_width, max((len(line) for line in interior), default=min_width))
    h = glyphs["h"]
    v = glyphs["v"]
    top = glyphs["tl"] + h * (width + 2) + glyphs["tr"]
    bot = glyphs["bl"] + h * (width + 2) + glyphs["br"]
    body_lines = [f"{v} {line:<{width}} {v}" for line in interior]
    lines = [top, *body_lines, bot]
    if not box.children:
        return "\n".join(lines) + "\n"
    # Render children indented under the parent with a connector line.
    out: list[str] = list(lines)
    for i, child in enumerate(box.children):
        is_last = i == len(box.children) - 1
        out.append(glyphs["stem"])
        child_text = _render_box(child, glyphs=glyphs, min_width=min_width)
        child_lines = child_text.splitlines()
        if not child_lines:
            continue
        elbow = glyphs["last"] if is_last else glyphs["branch"]
        out.append(elbow + glyphs["elbow"] + " " + child_lines[0])
        cont = " " if is_last else glyphs["stem"]
        for line in child_lines[1:]:
            out.append(cont + "  " + line)
    return "\n".join(out) + "\n"


def _box_interior_lines(box: _Box) -> list[str]:
    lines = [box.title]
    if box.note:
        lines.append(box.note)
    return lines


def _parse_int(value: Any, *, name: str) -> int:
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise RendererError(f"ascii-blocks: {name} must be an integer (got {value!r})") from exc
