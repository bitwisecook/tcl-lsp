"""PortSet value type — F5 firewall-rule port specs.

Parses ``80-82,8081``-style comma-separated ranges.  Each comma-
separated segment is either a single port (``8081``) or a
``low-high`` inclusive range (``80-82``).  The ``any`` /
``*`` / ``0`` spellings are recognised as the wildcard.

Distinct from :class:`Port` (single port) and the older
:class:`PortRange` (single ``low-high``) in this package — those
two model the values F5 emits inside a single rule field, while
:class:`PortSet` models the comma-separated *spec* a security
firewall rule's ``port`` field can carry.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterator


@dataclass(frozen=True, slots=True)
class PortSegment:
    """One contiguous ``low..high`` port range (inclusive)."""

    low: int
    high: int

    def __post_init__(self) -> None:
        if self.low < 0 or self.high > 65535:
            raise ValueError(f"PortSegment: out of range ({self.low}-{self.high})")
        if self.low > self.high:
            raise ValueError(f"PortSegment: low > high ({self.low}-{self.high})")

    @property
    def is_single(self) -> bool:
        return self.low == self.high

    def contains(self, port: int) -> bool:
        return self.low <= port <= self.high

    def overlaps(self, other: "PortSegment") -> bool:
        return not (self.high < other.low or other.high < self.low)

    def __iter__(self) -> Iterator[int]:
        return iter(range(self.low, self.high + 1))

    def __str__(self) -> str:
        return str(self.low) if self.is_single else f"{self.low}-{self.high}"


@dataclass(frozen=True, slots=True)
class PortSet:
    """A set of port ranges expressed as ``80-82,8081``.

    The internal representation is a tuple of disjoint
    :class:`PortSegment` items sorted by their low end so equality
    and intersection are well-defined.  Use :meth:`parse` to read
    the textual form; the constructor accepts an already-normalised
    segment tuple if you've built one programmatically.
    """

    segments: tuple[PortSegment, ...] = field(default_factory=tuple)
    is_wildcard: bool = False

    @classmethod
    def parse(cls, text: str) -> "PortSet":
        """Parse a comma-separated port spec into a normalised range.

        ``"80-82,8081"`` → two segments (80-82 and 8081-8081).
        ``"any"`` / ``"*"`` / ``"0"`` produce a wildcard range.
        Overlapping / adjacent segments are collapsed so the
        representation is canonical.

        Raises :class:`ValueError` for malformed input.
        """
        text = (text or "").strip()
        if not text:
            raise ValueError("PortSet: empty input")
        if text.lower() in ("any", "*", "0"):
            return cls(segments=(PortSegment(0, 65535),), is_wildcard=True)
        raw: list[PortSegment] = []
        for chunk in text.split(","):
            piece = chunk.strip()
            if not piece:
                continue
            if "-" in piece:
                lo_text, _, hi_text = piece.partition("-")
                try:
                    lo, hi = int(lo_text), int(hi_text)
                except ValueError as exc:
                    raise ValueError(f"PortSet: non-integer in {piece!r}") from exc
                raw.append(PortSegment(lo, hi))
            else:
                try:
                    n = int(piece)
                except ValueError as exc:
                    raise ValueError(f"PortSet: non-integer port {piece!r}") from exc
                raw.append(PortSegment(n, n))
        if not raw:
            raise ValueError("PortSet: no usable segments")
        normalised = _collapse_segments(raw)
        return cls(segments=tuple(normalised), is_wildcard=False)

    @classmethod
    def try_parse(cls, text: str) -> "PortSet | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    def contains(self, port: int) -> bool:
        """``True`` when *port* is inside any segment."""
        return any(seg.contains(port) for seg in self.segments)

    def overlaps(self, other: "PortSet") -> bool:
        """``True`` when *self* and *other* share at least one port."""
        for s in self.segments:
            for o in other.segments:
                if s.overlaps(o):
                    return True
        return False

    @property
    def count(self) -> int:
        """Total number of ports across every segment."""
        return sum(seg.high - seg.low + 1 for seg in self.segments)

    def __iter__(self) -> Iterator[int]:
        for seg in self.segments:
            yield from seg

    def __str__(self) -> str:
        if self.is_wildcard:
            return "0-65535"
        return ",".join(str(seg) for seg in self.segments)


def _collapse_segments(raw: list[PortSegment]) -> list[PortSegment]:
    """Merge overlapping / adjacent segments into a canonical list.

    Sorted by ``low``; runs that touch (``[1,5]`` + ``[6,10]``) are
    fused; runs that overlap (``[1,7]`` + ``[5,10]``) are fused.
    """
    if not raw:
        return []
    sorted_segs = sorted(raw, key=lambda s: (s.low, s.high))
    out: list[PortSegment] = [sorted_segs[0]]
    for seg in sorted_segs[1:]:
        last = out[-1]
        # ``+1`` covers the adjacency case (``[1,5]`` then ``[6,10]``
        # merges into ``[1,10]``).
        if seg.low <= last.high + 1:
            out[-1] = PortSegment(last.low, max(last.high, seg.high))
        else:
            out.append(seg)
    return out
