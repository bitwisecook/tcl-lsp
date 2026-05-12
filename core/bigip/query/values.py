"""Runtime value model for the query DSL.

Most values flow as plain Python objects — strings, ints, floats, bools,
``None``, lists, and dicts — which keeps the evaluator small.  Three
specialised wrappers carry extra information the rest of the pipeline
needs:

- :class:`ObjectRef` — a parsed BIG-IP object (virtual, pool, …).  The
  evaluator builds these lazily as the user navigates the tree; the
  edit planner pulls field byte-ranges off them when an assignment
  lands.
- :class:`PathRef` — a string-valued reference to another BIG-IP object
  by full-path.  Field access on a :class:`PathRef` transparently
  dereferences into the target :class:`ObjectRef`, so
  ``.ltm.virtual[].pool.members`` walks VS → pool → members in one
  step.  Arithmetic and string predicates treat it as a string.
- :class:`Stream` — a flat sequence produced by the ``[]`` operator or
  any builtin that returns multiple values.  Distinct from
  :class:`list` so the output formatter knows when to emit one-per-line
  vs a JSON array.

The address/network helpers in :mod:`.builtins` use plain
``ipaddress`` objects; we do not wrap them.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ..model import BigipConfig
from .source_map import SourceMap


@dataclass(frozen=True, slots=True)
class FieldSlot:
    """Where a single property value lives in the source text.

    ``range`` covers just the value half of ``key value`` — assigning a
    new value rewrites this span and leaves the key, indentation, and
    surrounding stanza untouched.
    """

    source_uri: str
    start: int
    end: int
    raw_text: str


@dataclass
class ObjectRef:
    """A BIG-IP object exposed to the query DSL.

    *kind* is the TMSH module + object type (``"ltm virtual"``,
    ``"ltm pool"``).  *fields* are the user-visible property names with
    their current Python values (resolved via the model dataclass).
    *field_slots* maps the same keys to byte ranges in the source so
    the edit planner can rewrite a single property in place.

    The whole-stanza ``range`` is captured separately for SCF stanza
    output and for identity-field writes (which need to know which
    object owns a header).
    """

    kind: str
    full_path: str
    fields: dict[str, Any] = field(default_factory=dict)
    field_slots: dict[str, FieldSlot] = field(default_factory=dict)
    stanza_slot: FieldSlot | None = None
    # Back-pointer used by builtins like `refs()` / `referenced_by()`.
    config_uri: str = ""

    @property
    def name(self) -> str:
        return self.full_path.rsplit("/", 1)[-1]

    @property
    def partition(self) -> str:
        if self.full_path.startswith("/"):
            parts = self.full_path.split("/", 2)
            if len(parts) >= 2:
                return parts[1]
        return ""


@dataclass(frozen=True, slots=True)
class PathRef:
    """A string reference to another BIG-IP object.

    String-like in every context that expects a scalar (the underlying
    full-path), but the evaluator follows ``.field`` access through to
    the target object when one resolves.  Lookups happen against the
    :class:`BigipConfig` carried by *root*.
    """

    full_path: str
    root: "Root | None" = None
    # The kind of object we expect this path to resolve to ("ltm pool",
    # …).  Empty means "any matching kind".
    expected_kind: str = ""

    def __str__(self) -> str:  # pragma: no cover - used by formatters
        return self.full_path

    def __bool__(self) -> bool:
        return bool(self.full_path)


@dataclass
class Stream:
    """A flat sequence of values produced by ``[]`` or stream builtins.

    Lists in the DSL are explicit (literal arrays or builtin results
    like ``keys``); streams are what ``.[]`` / ``map(...)`` /
    ``select(...)`` produce.  The distinction matters for output:
    streams flatten to one-value-per-line in ``--raw`` mode; lists do
    not.
    """

    items: list[Any] = field(default_factory=list)

    def __iter__(self):
        return iter(self.items)

    def __len__(self) -> int:
        return len(self.items)

    def __bool__(self) -> bool:
        return bool(self.items)


@dataclass
class Root:
    """The per-file evaluation root.

    Holds the parsed :class:`BigipConfig`, the source text, and the
    :class:`SourceMap` used for line/column reporting.  Builtins that
    walk references (``refs``, ``referenced_by``) consult this back-
    pointer rather than threading the config through every call.
    """

    uri: str
    source: str
    config: BigipConfig
    source_map: SourceMap

    # Lazily built: full-path → ObjectRef.  Populated as the user
    # navigates so we don't pay for objects no query touches.
    _object_cache: dict[str, ObjectRef] = field(default_factory=dict)
