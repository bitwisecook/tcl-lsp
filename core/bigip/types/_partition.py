"""Partition value type — F5 administrative partitions."""

from __future__ import annotations

from dataclasses import dataclass

# F5 partition names: alphanumerics, underscores, hyphens, and dots.
# Must NOT contain slashes (slashes are the path separator).
# Reserved partition: ``Common`` is the system-default and is always present.
_PARTITION_VALID_CHARS = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-."
)


@dataclass(frozen=True, slots=True)
class Partition:
    """An F5 administrative partition (``/Common``, ``/Tenant_A``, ...).

    Stored canonically with the leading ``/`` so :meth:`__str__`
    matches the F5 spelling everywhere.  Parsing accepts both
    leading-slash (``/Common``) and bare (``Common``) input.
    """

    name: str  # canonical: leading slash + the partition name

    @classmethod
    def parse(cls, text: str) -> "Partition":
        """Parse *text* as a partition name.

        Accepts ``"/Common"``, ``"Common"``, ``"/Tenant_A"``.  Rejects
        empty / multi-segment / invalid-character input.  Raises
        :class:`ValueError` on failure.
        """
        text = text.strip()
        if not text:
            raise ValueError("Partition: empty input")
        bare = text.lstrip("/")
        if not bare:
            raise ValueError(f"Partition: only slashes ({text!r})")
        if "/" in bare:
            raise ValueError(f"Partition: nested path not allowed ({text!r}); partitions are flat")
        if not all(c in _PARTITION_VALID_CHARS for c in bare):
            raise ValueError(f"Partition: invalid character in {text!r}")
        return cls(name=f"/{bare}")

    @classmethod
    def try_parse(cls, text: str) -> "Partition | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def short_name(self) -> str:
        """The partition name without the leading slash."""
        return self.name.lstrip("/")

    @property
    def is_common(self) -> bool:
        """``True`` for the system-default ``/Common`` partition."""
        return self.short_name == "Common"

    def __str__(self) -> str:
        return self.name
