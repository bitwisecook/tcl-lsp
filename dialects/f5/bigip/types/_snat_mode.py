"""Typed BIG-IP source-address-translation mode value.

The ``source-address-translation`` property on an ltm virtual (and
the related ``snatpool`` field) is a sum type with four forms:

    source-address-translation { type none }
    source-address-translation { type automap }
    source-address-translation { type snat pool /Common/snatpool_a }
    snatpool /Common/snatpool_a

This module models the sum type so consumers can ask
``is this automap?``, ``which snat pool does this use?``, ``is
SNAT off here?`` without re-parsing the body, and the rewrite
layer can swap modes safely (an automap → pool conversion should
rewrite the body, not just splice a new string in).

The matching :class:`dialects.f5.bigip.registry.value_specs.SnatModeSpec`
wires the value into the registry.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

SnatModeKind = Literal["none", "automap", "snat", "address-list"]


@dataclass(frozen=True, slots=True)
class SnatMode:
    """A parsed source-address-translation mode.

    ``kind`` picks the variant:

    - ``"none"`` — SNAT is disabled (``type none``).
    - ``"automap"`` — BIG-IP auto-selects a self-IP per pool member
      (``type automap``).
    - ``"snat"`` — A named SNAT pool is used (``type snat`` + ``pool
      <full-path>``); ``pool_path`` holds the reference.
    - ``"address-list"`` — Reserved for the address-list variant
      some classifier objects use; ``addresses`` carries the list.

    ``raw`` preserves the original brace-body text so a no-op round
    trip through the rewrite layer doesn't change spacing.
    """

    kind: SnatModeKind = "none"
    pool_path: str = ""
    addresses: tuple[str, ...] = ()
    raw: str = ""

    @classmethod
    def try_parse(cls, text: str) -> "SnatMode | None":
        """Parse a ``source-address-translation`` body.

        Accepts both shapes the parser layer might hand in:

        - The full property body ``{ type snat pool /Common/p }`` (with
          braces).
        - The inner content ``type snat pool /Common/p`` (without
          braces, which is how :func:`_parse_properties_with_spans`
          reports it after stripping the outer ``{`` / ``}``).
        """
        if text is None:
            return None
        body = text.strip()
        if not body:
            return None
        if body.startswith("{"):
            body = body[1:]
        if body.endswith("}"):
            body = body[:-1]
        tokens = body.split()
        if not tokens:
            return None
        kind: SnatModeKind = "none"
        pool_path = ""
        idx = 0
        while idx < len(tokens):
            tok = tokens[idx]
            if tok == "type" and idx + 1 < len(tokens):
                t = tokens[idx + 1]
                if t == "none":
                    kind = "none"
                elif t == "automap":
                    kind = "automap"
                elif t == "snat":
                    kind = "snat"
                else:
                    return None
                idx += 2
                continue
            if tok == "pool" and idx + 1 < len(tokens):
                pool_path = tokens[idx + 1]
                idx += 2
                continue
            idx += 1
        if kind == "snat" and not pool_path:
            # ``type snat`` without a pool reference is malformed.
            return None
        return cls(kind=kind, pool_path=pool_path, raw=text.strip())

    @property
    def is_none(self) -> bool:
        return self.kind == "none"

    @property
    def is_automap(self) -> bool:
        return self.kind == "automap"

    @property
    def is_snat(self) -> bool:
        return self.kind == "snat"

    def references(self) -> tuple[str, ...]:
        """Return the full-paths this mode references (the SNAT pool,
        when the mode is ``snat``).  Empty for none / automap."""
        if self.kind == "snat" and self.pool_path:
            return (self.pool_path,)
        return ()

    def __str__(self) -> str:
        if self.raw:
            return self.raw
        if self.kind == "none":
            return "{ type none }"
        if self.kind == "automap":
            return "{ type automap }"
        if self.kind == "snat":
            return f"{{ type snat pool {self.pool_path} }}"
        return ""


__all__ = ["SnatMode", "SnatModeKind"]
