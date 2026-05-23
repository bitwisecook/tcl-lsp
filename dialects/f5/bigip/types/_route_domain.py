"""RouteDomain value type — the F5 ``%N`` route-domain suffix."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class RouteDomain:
    """An F5 route-domain identifier (the ``%N`` suffix on addresses).

    Route-domain 0 is the default (no segregation); higher numbers
    select specific routing tables on multi-tenant deployments.
    Stored as the integer ID; rendered as ``%N`` so it concatenates
    cleanly behind an address.
    """

    id: int

    @classmethod
    def parse(cls, text: str) -> "RouteDomain":
        """Parse *text* as a route-domain ID.

        Accepts ``"5"`` and ``"%5"`` and ``"   5  "``.  Negatives or
        non-integers raise :class:`ValueError`.
        """
        text = text.strip()
        if text.startswith("%"):
            text = text[1:]
        if not text:
            raise ValueError("RouteDomain: empty input")
        try:
            value = int(text, 10)
        except ValueError as exc:
            raise ValueError(f"RouteDomain: not numeric ({text!r})") from exc
        if value < 0:
            raise ValueError(f"RouteDomain: negative ({value})")
        return cls(id=value)

    @classmethod
    def try_parse(cls, text: str) -> "RouteDomain | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_default(self) -> bool:
        """``True`` when this is route-domain 0 (no segregation)."""
        return self.id == 0

    def __str__(self) -> str:
        return f"%{self.id}"
