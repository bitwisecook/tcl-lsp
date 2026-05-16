"""Typed BIG-IP GTM region-member row value.

GTM ``region`` objects carry a ``region-members`` block whose
items are row expressions, not refs:

    region-members {
        subnet 10.10.1.0/24 { }
        not subnet 10.10.2.0/24 { }
        country US { }
        not country JP { }
    }

Each row has an optional ``not`` negation, a clause type (``subnet``,
``country``, ``state``, ``isp``, ``continent``, ...), and a value.
The legacy parser captured the row as raw text; this typed value
exposes the structure so the DSL can ask

    .gtm.region[].region-members[]
      | select(.negated and .kind == "subnet")
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class GtmRegionMember:
    """One row inside a GTM ``region-members`` block.

    ``kind`` is the row's clause type (``subnet`` / ``country`` /
    ``state`` / ``isp`` / ``continent`` / ``region`` /
    ``geoip-isp`` — the documented topology clause types).
    ``value`` is the matched literal — a CIDR for ``subnet`` rows,
    a country code for ``country`` rows, etc.  ``negated`` is the
    ``not`` prefix.  ``raw`` preserves the original spelling so a
    no-op round trip keeps spacing intact.
    """

    kind: str = ""
    value: str = ""
    negated: bool = False
    raw: str = ""

    @classmethod
    def try_parse(cls, text: str) -> "GtmRegionMember | None":
        """Parse one row.  The input may include the trailing
        ``{ }`` body or omit it; we strip whichever shape we
        find.  Returns ``None`` for empty or all-whitespace input.
        """
        if text is None:
            return None
        stripped = text.strip()
        if not stripped:
            return None
        # Drop the trailing ``{ ... }`` body — it never carries
        # structured data on a region member, just an empty marker.
        brace = stripped.find("{")
        if brace >= 0:
            stripped = stripped[:brace].strip()
        tokens = stripped.split()
        if not tokens:
            return None
        negated = False
        if tokens[0] == "not":
            negated = True
            tokens = tokens[1:]
        if len(tokens) < 2:
            return None
        kind = tokens[0]
        value = " ".join(tokens[1:])
        return cls(kind=kind, value=value, negated=negated, raw=text.strip())

    def __str__(self) -> str:
        if self.raw:
            return self.raw
        prefix = "not " if self.negated else ""
        return f"{prefix}{self.kind} {self.value} {{ }}"


__all__ = ["GtmRegionMember"]
