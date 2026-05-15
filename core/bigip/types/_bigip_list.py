"""Generic typed list value for BIG-IP list-shaped properties.

The legacy projection stored list-shaped properties as flat
``tuple[str, ...]`` of names.  That dropped every piece of per-item
metadata — the ``context`` on a ``profiles`` attachment, the
``default`` on a ``persist`` attachment, the ``cert`` / ``key`` /
``chain`` on a ``cert-key-chain`` entry, the structured body of a
firewall rule, etc.

:class:`BigipList` carries the items as :class:`ListItem` records,
each pairing the typed item value with its lexical metadata
(``key`` / ``body`` / ``raw`` / source ranges).  The query DSL can
then ask

    .ltm.virtual[].profiles[] | select(.context == "clientside") | .name
    .security.firewall-rule-list[].rules[] | select(.action == "accept")

without the consumer having to know that ``profiles`` is a
keyed-block list and ``cert-key-chain`` is a different keyed-block
list.  Each item still exposes its ``raw`` body for round-trip
preservation, and ``range`` / ``key_range`` / ``body_range`` carry
the byte spans LSP features (document links, rename, semantic
tokens) need for exact navigation.

The matching :class:`core.bigip.registry.value_specs.ListSpec` owns
``parse`` / ``project`` / ``render`` / ``references`` so consumers
(the parser, the projection, the edit planner, the graph builder,
the LSP) never reach into BigipList internals; they ask the spec.

Four list shapes live behind the single :class:`BigipList` type, all
discriminated by :class:`ListSyntax`:

- ``space-separated`` — unbraced scalar lists (``ip-protocol icmp
  tcp``, the bareword side of compact stanzas).
- ``braced-space-separated`` — ``{ /Common/a /Common/b }``
  (``rules``, ``policies``, ``vlans``).
- ``keyed-block`` — ``{ /Common/clientssl { context clientside }
  /Common/http { } }`` (``profiles``, ``persist``,
  ``cert-key-chain``, ``records``, pool ``members``).
- ``named-rule-block`` — ``{ name { action accept ... } }``
  (firewall rules, policy rules — the body is a structured
  classifier rather than scalar metadata).

- ``operator-prefixed`` — ``{ replace-all-with { /Common/a } }`` /
  ``{ add { /Common/b } }`` — the tmsh edit verbs nest INSIDE the
  property's braced body.  The ``operator`` field on
  :class:`BigipList` carries the verb so the renderer can re-emit
  the same shape.

One further shape is intentionally *not* in :class:`ListSyntax`
today:

- *monitor-expression* — handled by
  :class:`core.bigip.types.MonitorExpression` as a dedicated typed
  value (``mode`` / ``monitors`` / ``minimum`` plus per-monitor
  source spans).  Lives outside :class:`BigipList` because the
  ``min N of`` + ``and``-chain grammar isn't a flat list.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator, MutableSequence
from dataclasses import dataclass, field
from typing import Any, Literal, Protocol, overload, runtime_checkable

ListSyntax = Literal[
    "space-separated",
    "braced-space-separated",
    "keyed-block",
    "named-rule-block",
    "operator-prefixed",
]


@dataclass(frozen=True, slots=True)
class SourceSpan:
    """Half-open byte span paired with absolute offsets.

    Mirrors :class:`core.bigip.registry.value_specs.SourceRange` but
    lives in the types module so :class:`ListItem` can carry ranges
    without the types layer importing the registry layer.  Consumers
    bridge the two via ``SourceRange(span.start, span.end)``.
    """

    start: int
    end: int


@dataclass(frozen=True, slots=True)
class ListItem:
    """One element of a :class:`BigipList`.

    ``value`` is the typed item value (``str`` for plain
    space-separated lists, :class:`ProfileAttachment` /
    :class:`PersistenceAttachment` / :class:`CertKeyChain` /
    :class:`DataGroupRecord` / :class:`GtmRegionMember` /
    :class:`FirewallRule` / :class:`MonitorExpression` for the
    structured shapes).

    ``key`` and ``body`` carry the lexical halves of a keyed-block
    item — ``key`` is the path or rule-name preceding ``{ ... }``,
    ``body`` is the brace body content (without the braces).  Both
    are empty for non-keyed lists.

    ``raw`` is the per-item source text (including the brace body
    for keyed forms) so rewrite layers can preserve original
    spelling when the structured fields are unchanged.

    The three range fields locate the item span (``range``), the
    key token (``key_range``), and the body span (``body_range``)
    inside the original source text.  Each is ``None`` when not
    available (e.g. typed values built programmatically, not parsed
    from source).
    """

    value: object
    key: str = ""
    body: str = ""
    raw: str = ""
    range: SourceSpan | None = None
    key_range: SourceSpan | None = None
    body_range: SourceSpan | None = None

    @property
    def name(self) -> str:
        """Convenience: the leaf name of ``key`` when the key is a
        TMSH full-path (``/Common/clientssl`` → ``clientssl``) or
        the key verbatim otherwise (so plain rule names round-trip
        without a ``rsplit`` everywhere).
        """
        if not self.key:
            return ""
        if "/" in self.key:
            return self.key.rsplit("/", 1)[-1]
        return self.key


@runtime_checkable
class _HasFullPath(Protocol):
    full_path: str


@runtime_checkable
class _HasPath(Protocol):
    path: str


@dataclass(slots=True)
class BigipList(MutableSequence[Any]):
    """A list-shaped BIG-IP property value.

    ``items`` is the typed per-element view.  ``syntax`` discriminates
    the lexical shape so the renderer knows how to emit braces /
    operators / context tokens.  ``operator`` carries the tmsh edit
    verb (``add`` / ``delete`` / ``modify`` / ``replace-all-with``)
    when present; the standalone list value has ``operator=None``.

    ``raw`` preserves the original brace body (without the
    surrounding braces) for no-op round trips; ``range`` is the
    absolute span of the list value in the source text.

    For ``monitor-expression`` syntax, ``mode`` and ``minimum``
    carry the evaluation-rule metadata (``"all"`` / ``"min-of"``
    / etc.).  Other syntaxes leave these fields at their defaults.
    """

    items: list[ListItem] = field(default_factory=list)
    syntax: str = "braced-space-separated"  # values from :data:`ListSyntax`
    operator: str | None = None
    raw: str = ""
    range: SourceSpan | None = None
    # Diagnostics raised during parsing — surfaces unknown enum
    # tokens, malformed keyed-block entries, etc.  Empty for
    # well-formed inputs.
    diagnostics: tuple[object, ...] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        self.items = [_as_list_item(item) for item in self.items]

    def __iter__(self) -> Iterator[Any]:
        """Iterating the list yields the typed item values so query
        DSL ``.profiles[]`` flows directly through the list without
        the consumer reaching into ``items``."""
        for item in self.items:
            yield item.value

    def __len__(self) -> int:
        return len(self.items)

    def __bool__(self) -> bool:
        return bool(self.items)

    @overload
    def __getitem__(self, index: int) -> Any: ...

    @overload
    def __getitem__(self, index: slice) -> list[Any]: ...

    def __getitem__(self, index: int | slice) -> Any | list[Any]:
        if isinstance(index, slice):
            return [item.value for item in self.items[index]]
        return self.items[index].value

    @overload
    def __setitem__(self, index: int, value: Any) -> None: ...

    @overload
    def __setitem__(self, index: slice, value: Iterable[Any]) -> None: ...

    def __setitem__(self, index: int | slice, value: Any | Iterable[Any]) -> None:
        if isinstance(index, slice):
            if isinstance(value, (str, bytes)) or not isinstance(value, Iterable):
                replacement = [value]
            else:
                replacement = list(value)
            self.items[index] = [_as_list_item(item) for item in replacement]
            return
        self.items[index] = _as_list_item(value)

    def __delitem__(self, index: int | slice) -> None:
        del self.items[index]

    def insert(self, index: int, value: Any) -> None:
        self.items.insert(index, _as_list_item(value))

    def __contains__(self, needle: object) -> bool:
        """Membership against either a typed item value OR its
        string path — so legacy callers checking ``"/Common/x" in
        vs.profiles`` keep working without unwrapping items."""
        if isinstance(needle, str):
            return needle in self.paths
        return any(item.value == needle for item in self.items)

    @property
    def values(self) -> tuple[Any, ...]:
        """Tuple of just the typed item values — for callers that
        want the legacy ``tuple[T, ...]`` shape without iterating."""
        return tuple(item.value for item in self.items)

    @property
    def paths(self) -> tuple[str, ...]:
        """Tuple of string paths the items reference.

        For typed item values with a ``full_path`` / ``path``
        attribute (``ProfileAttachment``, ``PersistenceAttachment``,
        ``CertKeyChain``, ``FirewallRule``, ...) returns the
        ``full_path``; for plain string items (``ObjectRefSpec``-
        wrapped paths in ``rules`` / ``policies`` / ``vlans``)
        returns the string verbatim.  This is the back-compat
        surface the legacy ``tuple[str, ...]`` consumers used —
        ``for p in vs.profiles.paths`` reads identically to the
        pre-collapse ``for p in vs.profiles`` shape."""
        out: list[str] = []
        for item in self.items:
            v = item.value
            if isinstance(v, str):
                out.append(v)
                continue
            if isinstance(v, _HasFullPath) and v.full_path:
                out.append(v.full_path)
                continue
            if isinstance(v, _HasPath) and v.path:
                out.append(v.path)
                continue
            if item.key:
                out.append(item.key)
                continue
            out.append(str(v) if v is not None else "")
        return tuple(out)


def _as_list_item(value: object) -> ListItem:
    if isinstance(value, ListItem):
        return value
    return ListItem(value=value)


__all__ = ["BigipList", "ListItem", "ListSyntax", "SourceSpan"]
