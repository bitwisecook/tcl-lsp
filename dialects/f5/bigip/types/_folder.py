"""Folder + ObjectPath value types — F5 admin folders and full object paths.

F5 object paths can be flat (``/Common/web_pool``) or nested through
admin folders (``/Common/Application_X/web_pool`` /
``/Common/iApps/Tenant.app/pool_1``).  The :class:`Folder` type
captures the partition + zero or more folder segments; the
:class:`ObjectPath` type pairs a folder with a leaf object name and
is what every BIG-IP ``full-path`` field actually means.

Distinguishing partition from folder matters because:

- the partition is the administrative boundary (RBAC, sync groups,
  route-domain default) — every object lives in exactly one;
- folders are organisational containers within a partition (no
  RBAC), commonly produced by iApps deployments;
- the rename / deep-link / cross-config resolver needs to know the
  partition prefix to scope its search, but treats folder segments
  as opaque path components.

These types compose with the rest of the value layer:
:class:`Partition` is the leading segment, :class:`Folder` is the
in-between segments, the leaf is just a string.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from ._partition import _PARTITION_VALID_CHARS, Partition

# Folder segments share the partition's valid character set —
# alphanumerics, underscores, hyphens, dots.  A folder segment may
# *not* contain slashes (those are the path separator).
_FOLDER_SEGMENT_VALID_CHARS = _PARTITION_VALID_CHARS


@dataclass(frozen=True, slots=True)
class Folder:
    """An F5 admin folder path: a partition + zero or more nested segments.

    Stored canonically with the leading slash and all segments joined
    with ``/``.  ``Folder("/Common")`` is equivalent to "the partition
    root with no folder nesting"; ``Folder("/Common/Application_X")``
    is the ``Application_X`` folder under ``/Common``.

    Only the partition is mandatory; pure-partition folders (no
    nested segments) are common.
    """

    partition: Partition
    segments: tuple[str, ...] = field(default_factory=tuple)

    @classmethod
    def parse(cls, text: str) -> "Folder":
        """Parse *text* as a folder path.

        Accepts ``"/Common"`` (partition only) and
        ``"/Common/Application_X"`` (partition + one folder segment)
        and arbitrarily-deep nesting (``"/Common/iApps/Tenant.app"``).
        Trailing slashes are tolerated.
        """
        text = text.strip()
        if not text:
            raise ValueError("Folder: empty input")
        if not text.startswith("/"):
            raise ValueError(f"Folder: must start with '/' (partition prefix), got {text!r}")
        # Strip the leading + any trailing slash, then split.
        parts = [p for p in text.strip("/").split("/") if p]
        if not parts:
            raise ValueError(f"Folder: only slashes ({text!r})")
        partition = Partition.parse(parts[0])
        segments: tuple[str, ...] = tuple(parts[1:])
        for seg in segments:
            if not all(c in _FOLDER_SEGMENT_VALID_CHARS for c in seg):
                raise ValueError(f"Folder: invalid character in folder segment {seg!r}")
        return cls(partition=partition, segments=segments)

    @classmethod
    def try_parse(cls, text: str) -> "Folder | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_root(self) -> bool:
        """``True`` when this folder is the partition root (no nesting)."""
        return not self.segments

    @property
    def depth(self) -> int:
        """Number of folder segments below the partition (0 for root)."""
        return len(self.segments)

    def child(self, segment: str) -> "Folder":
        """Return a new :class:`Folder` with *segment* appended."""
        if not all(c in _FOLDER_SEGMENT_VALID_CHARS for c in segment):
            raise ValueError(f"Folder.child: invalid segment {segment!r}")
        return Folder(partition=self.partition, segments=self.segments + (segment,))

    def __str__(self) -> str:
        if self.is_root:
            return str(self.partition)
        return str(self.partition) + "/" + "/".join(self.segments)


@dataclass(frozen=True, slots=True)
class ObjectPath:
    """A full BIG-IP object path: ``<folder>/<leaf>``.

    The leaf is the object's name (``web_pool``, ``vs1``, etc.) and
    can contain the same characters folder segments do.  Together
    the folder + leaf form what the rest of the codebase calls the
    object's ``full_path``.
    """

    folder: Folder
    name: str

    @classmethod
    def parse(cls, text: str) -> "ObjectPath":
        """Parse *text* as a full BIG-IP object path.

        ``"/Common/web_pool"`` → partition ``Common``, no folder
        segments, leaf ``web_pool``.  ``"/Common/Application_X/web_pool"``
        → partition ``Common``, folder ``Application_X``, leaf
        ``web_pool``.  ``"/Common/iApps/Tenant.app/pool_1"`` →
        partition ``Common``, folder ``iApps/Tenant.app``, leaf
        ``pool_1``.
        """
        text = text.strip()
        if not text:
            raise ValueError("ObjectPath: empty input")
        if not text.startswith("/"):
            raise ValueError(f"ObjectPath: must start with '/' (partition prefix), got {text!r}")
        parts = [p for p in text.strip("/").split("/") if p]
        if len(parts) < 2:
            raise ValueError(f"ObjectPath: must have at least partition + leaf, got {text!r}")
        leaf = parts[-1]
        if not all(c in _FOLDER_SEGMENT_VALID_CHARS for c in leaf):
            raise ValueError(f"ObjectPath: invalid character in leaf {leaf!r}")
        folder_text = "/" + "/".join(parts[:-1])
        return cls(folder=Folder.parse(folder_text), name=leaf)

    @classmethod
    def try_parse(cls, text: str) -> "ObjectPath | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def partition(self) -> Partition:
        return self.folder.partition

    @property
    def in_root_folder(self) -> bool:
        """``True`` when the object lives at the partition root (no folders)."""
        return self.folder.is_root

    def __str__(self) -> str:
        if self.folder.is_root:
            return f"{self.folder.partition}/{self.name}"
        return f"{self.folder}/{self.name}"
