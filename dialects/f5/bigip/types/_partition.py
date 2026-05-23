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
        """``True`` for the system-default ``/Common`` partition.

        ``/Common`` is the system-default and has *one-way* visibility:
        every partition can reference ``/Common``, but ``/Common``
        cannot reference any other partition.  Use :meth:`can_see` to
        check directional visibility.
        """
        return self.short_name == "Common"

    def can_see(self, other: "Partition") -> bool:
        """``True`` when an object in ``self`` may reference an object
        in ``other``.

        F5 partition visibility rules:

        - same partition → always visible;
        - ``other == /Common`` → visible to every partition;
        - any other cross-partition pairing → **not** visible.

        ``/Common/web_pool`` may be referenced from ``/Tenant_A/vs1``
        (Tenant_A can see /Common); the reverse is forbidden
        (``/Common/vs1`` cannot reference ``/Tenant_A/web_pool``).
        :meth:`Partition.can_see` returns the correct answer for both
        directions so rename / validate logic can reason about
        visibility without re-implementing the rules.

        Partition-to-partition renames (``rename_partition("Tenant_A",
        "Tenant_B")``) are valid — the partition itself can be
        renamed regardless of visibility — but trying to *move* an
        object from ``/Common`` into a tenant partition (where other
        ``/Common`` objects reference it) breaks visibility for
        those referrers.  Callers should check
        :meth:`Partition.can_see` against the referring object's
        partition before authorising the move.
        """
        if self == other:
            return True
        return other.is_common

    def __str__(self) -> str:
        return self.name
