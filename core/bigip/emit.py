"""SCF emitter / round-trip helpers.

The simplest round-trip is text-preserving: we already have the
original source plus block offsets from :func:`_extract_blocks`, so
we slice rather than reformat.  This keeps comments, ordering, and
whitespace intact — important for ``f5 split`` / ``merge`` and for
``f5 rename`` / ``redact`` which want minimal-diff output.

Public API:

- :func:`iter_blocks_with_source` — yield ``(block, full_text_with_header)``
  tuples covering each top-level stanza, including the leading newline.
- :func:`partition_of` — extract the partition prefix from a full-path.
- :func:`emit_split_by_partition` — slice a source into ``{partition: text}``.
- :func:`emit_merged` — concatenate per-partition (or per-file) sources.
"""

from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass

from .parser import _Block, _extract_blocks, _parse_generic_header


@dataclass(frozen=True, slots=True)
class BlockSlice:
    """A top-level stanza with its full surrounding source text."""

    block: _Block
    module: str
    object_type: str
    identifier: str
    text: str  # full stanza text including header and trailing newline


def partition_of(identifier: str) -> str:
    """Return the ``/Common/`` etc. partition prefix, or ``""`` if none."""
    if not identifier or not identifier.startswith("/"):
        return ""
    parts = identifier.split("/", 2)
    if len(parts) >= 2:
        return f"/{parts[1]}/"
    return ""


def _slice_for_block(source: str, block: _Block) -> str:
    """Return the full text of *block* including its header."""
    # Walk back from start_offset (the `{`) to the start of the line.
    header_start = block.start_offset
    while header_start > 0 and source[header_start - 1] != "\n":
        header_start -= 1
    return source[header_start : block.end_offset]


def iter_blocks_with_source(source: str) -> list[BlockSlice]:
    """Yield :class:`BlockSlice` instances for every top-level stanza."""
    out: list[BlockSlice] = []
    for block in _extract_blocks(source):
        generic = _parse_generic_header(block.header)
        if generic is None:
            continue
        module, object_type, identifier = generic
        text = _slice_for_block(source, block)
        out.append(
            BlockSlice(
                block=block,
                module=module,
                object_type=object_type,
                identifier=identifier,
                text=text,
            )
        )
    return out


def emit_split_by_partition(
    source: str, *, default_partition: str = "_no_partition"
) -> "OrderedDict[str, str]":
    """Slice *source* into partitions: ``{partition_dir: concatenated_blocks}``.

    Stanzas without an identifier-style partition (``auth partition``,
    singleton sys objects, …) are gathered under *default_partition*.
    """
    out: OrderedDict[str, list[str]] = OrderedDict()
    for slice_ in iter_blocks_with_source(source):
        partition = partition_of(slice_.identifier).strip("/") or default_partition
        out.setdefault(partition, []).append(slice_.text.rstrip() + "\n")
    return OrderedDict((part, "\n".join(blocks)) for part, blocks in out.items())


def emit_merged(parts: "OrderedDict[str, str] | dict[str, str] | list[str]") -> str:
    """Concatenate split sources back into a single SCF text."""
    if isinstance(parts, dict):
        ordered = list(parts.values())
    else:
        ordered = list(parts)
    chunks = [p.rstrip() + "\n" for p in ordered if p.strip()]
    return "\n".join(chunks)
