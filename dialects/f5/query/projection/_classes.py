"""Container and FieldSpec dataclasses used by the projection engine.

The :class:`Container` namespace abstraction is what ``.ltm``,
``.ltm.virtual``, etc. resolve to inside the DSL evaluator.
:class:`FieldSpec` declares the projection rule for one field of
one kind (attribute name, ref-kind, list-ref flag) and is consumed
by the dispatch tables in :mod:`._data`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from ..errors import EvalError
from ..values import Root

# ---------------------------------------------------------------------------
# Container abstraction
# ---------------------------------------------------------------------------


@dataclass
class Container:
    """A navigable mapping projected from a :class:`BigipConfig`.

    Containers are returned for the namespace nodes ``.ltm``, ``.gtm``,
    and for kind nodes like ``.ltm.virtual``.  Leaf entries inside a
    kind container are :class:`ObjectRef` instances; entries inside a
    namespace container are themselves :class:`Container` instances.

    The ``kind`` field carries either the module name (``"ltm"``) or
    the full TMSH module+type (``"ltm virtual"``).  Builtins and
    error messages use it to describe the level being navigated.
    """

    kind: str
    root: Root
    # Lazily filled.  Keys are the user-visible identifiers — full-paths
    # for object kinds, plain TMSH type names for module namespaces.
    _entries: dict[str, Any] | None = None
    _entry_source: str = ""  # "ltm.virtual", "ltm", ""

    def entries(self) -> dict[str, Any]:
        if self._entries is None:
            # Lazy import: ``_build_entries`` lives in :mod:`._engine`,
            # which imports ``Container`` from this module — a
            # top-level import would form a cycle.
            from ._engine import _build_entries

            self._entries = _build_entries(self)
        return self._entries

    def lookup(self, key: str) -> Any:
        ents = self.entries()
        if key in ents:
            return ents[key]
        # Partition shorthand: bare name resolves to ``/Common/<name>``
        # when that key exists and is unambiguous.  Any other matching
        # full-path with a different partition makes the lookup
        # ambiguous and we raise rather than guess.
        if self._is_object_kind() and not key.startswith("/"):
            full = f"/Common/{key}"
            matches = [k for k in ents if k.endswith(f"/{key}")]
            if full in ents:
                return ents[full]
            if len(matches) == 1:
                return ents[matches[0]]
            if len(matches) > 1:
                raise EvalError(
                    f"{self.kind}: name {key!r} is ambiguous "
                    f"({len(matches)} matches; use a full path)"
                )
        raise EvalError(f"{self.kind}: no entry {key!r}")

    def regex_keys(self, pattern: str) -> list[str]:
        # Route through the DSL's central regex chokepoint
        # (length + nested-quantifier guards) so the container regex
        # subscript can't be the soft underbelly when the rest of the
        # surface is hardened.  Wrap ``BuiltinError`` as ``EvalError``
        # to keep the existing error type for navigation failures.
        from ..builtins import _safe_regex_compile
        from ..errors import BuiltinError

        try:
            rx = _safe_regex_compile(pattern, name="regex subscript")
        except BuiltinError as exc:
            raise EvalError(str(exc)) from exc
        return [k for k in self.entries() if rx.search(k)]

    def _is_object_kind(self) -> bool:
        # Object kinds are the leaf containers (``ltm virtual`` etc.);
        # the bare module containers ("ltm", "gtm") hold sub-containers.
        # Lazy import: ``_OBJECT_KIND_ALIASES`` lives in :mod:`._data`,
        # which imports ``FieldSpec`` from this module — top-level import
        # would form a cycle.
        from ._data import _OBJECT_KIND_ALIASES

        return " " in self.kind or self.kind in _OBJECT_KIND_ALIASES


# ---------------------------------------------------------------------------
# Field maps — TMSH-spelt user names mapping to dataclass attribute names.
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FieldSpec:
    """How a single TMSH-spelt field projects from a dataclass."""

    attr: str  # dataclass attribute
    # ``ref_kind`` non-empty signals "this is a PathRef into <kind>".
    # ``list_ref`` flags list-of-PathRef fields.
    ref_kind: str = ""
    list_ref: bool = False
    # ``typed`` signals the dataclass field is a typed value object
    # (``Network`` / ``IPAddress`` / ``Destination`` / ``FQDN`` / …)
    # rather than a string.  The projection layer wraps ``str(value)``
    # around the attribute access so DSL users continue to see strings
    # (``.ltm.virtual[].destination`` still yields ``"/Common/1.1.1.1:80"``,
    # not a ``Destination`` repr).  ``None``-typed values render as the
    # empty string so the DSL truthiness rules match the prior
    # string-field behaviour.
    typed: bool = False
