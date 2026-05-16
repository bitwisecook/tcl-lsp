"""Fluent Python API on top of :func:`run_query`.

External scripts that want to leverage the query engine — to drive a
custom report, draw a diagram, or feed an integration test — go through
the classes in this module rather than calling :func:`run_query`
directly.  The lower-level entry point still works (and is what
:class:`Query` calls internally); the fluent shape just trims the
ceremony around source dicts, per-file result unpacking, and renderer
dispatch.

Typical script::

    from f5q import Query

    rows = (
        Query(".ltm.virtual[] | .name")
        .run(paths=["bigip.conf"])
        .values()
    )

    for name in rows:
        print(name)

Or, with a renderer::

    print(
        Query('''
            .ltm.virtual[] | { title: .name, rows: .pool.members[] }
        ''')
        .run(paths=["bigip.conf"])
        .render("ascii-blocks")
    )

The API is intentionally small.  Methods that don't fit cleanly here
(named-source binding, per-partition routing, structured side-inputs)
are exposed as keyword arguments on :meth:`Query.run` and map 1:1 to
:func:`run_query`'s parameters.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping

from ._inputs import InputSpec
from .errors import QueryError
from .runner import QueryResult, run_query
from .values import ObjectRef, PathRef, Stream


def _coerce_sources(
    sources: Mapping[str, str] | None,
    paths: Iterable[str | Path] | None,
) -> dict[str, str]:
    """Build the ``{uri: text}`` map :func:`run_query` expects.

    Accepts either *sources* (pre-loaded) or *paths* (read from disk)
    or both — when both are supplied, *sources* wins on conflict so a
    script can override one of several on-disk files with an
    in-memory edit without re-staging the rest.
    """
    out: dict[str, str] = {}
    if paths:
        for p in paths:
            path = Path(p)
            out[str(path)] = path.read_text(encoding="utf-8")
    if sources:
        out.update(dict(sources))
    return out


@dataclass(frozen=True, slots=True)
class QueryRow:
    """One value from a query result, tagged with its originating URI.

    Most callers iterate :meth:`QueryRun.values` and never see
    :class:`QueryRow` directly.  The tagged form (:meth:`QueryRun.rows`)
    is useful when a script loads two configs and needs to know which
    one each result came from — e.g. for per-file reports or for
    routing edits back to disk.
    """

    uri: str
    value: Any


class QueryRun:
    """The flattened result of one :meth:`Query.run`.

    Wraps the per-URI :class:`QueryResult` the runner returns and
    exposes the conveniences external scripts usually want — a single
    flat list of values, a "first or None" lookup, the path-only
    projection, and a one-call :meth:`render` that dispatches through
    the renderer registry.

    The underlying :attr:`result` is left untouched so a script can
    still reach into ``values_per_file`` / ``edits_per_file`` /
    ``has_mutation`` when it needs the full per-file shape (a writer
    routing edits to multiple files, an audit comparing two sources
    side by side, …).
    """

    __slots__ = ("result", "_sources")

    def __init__(self, result: QueryResult, sources: dict[str, str] | None = None) -> None:
        self.result = result
        self._sources = dict(sources) if sources else None

    def __repr__(self) -> str:  # pragma: no cover - debug aid
        return f"QueryRun(values={len(self.values())}, mutation={self.result.has_mutation})"

    def __len__(self) -> int:
        return sum(len(vs) for vs in self.result.values_per_file.values())

    def __bool__(self) -> bool:
        return any(self.result.values_per_file.values())

    def __iter__(self):
        return iter(self.values())

    def rows(self) -> list[QueryRow]:
        """Return every value tagged with its originating source URI."""
        out: list[QueryRow] = []
        for uri, vs in self.result.values_per_file.items():
            for v in _expand(vs):
                out.append(QueryRow(uri=uri, value=v))
        return out

    def values(self) -> list[Any]:
        """Return every value as a flat list, dropping URIs."""
        return [r.value for r in self.rows()]

    def first(self, default: Any = None) -> Any:
        """Return the first value, or *default* when the query produced none.

        Useful for "give me the one matching object" queries:
        ``Query('.ltm.pool["/Common/web"]').run(...).first()``.
        """
        for r in self.rows():
            return r.value
        return default

    def paths(self) -> list[str]:
        """Return the ``full_path`` of every :class:`ObjectRef` / :class:`PathRef`.

        Non-object values are skipped; mixed results don't raise.
        """
        out: list[str] = []
        for v in self.values():
            if isinstance(v, (ObjectRef, PathRef)):
                out.append(v.full_path)
        return out

    def objects(self) -> list[ObjectRef]:
        """Return every :class:`ObjectRef` from the result, in order."""
        return [v for v in self.values() if isinstance(v, ObjectRef)]

    def render(self, name: str, **opts: Any) -> str:
        """Render the values using the registered renderer *name*.

        Equivalent to ``render(name, run.values(), **opts)`` — but
        first binds the originating source text into the
        :data:`~core.bigip.query.renderers.RENDER_SOURCES` contextvar
        so renderers like ``mermaid`` can recover the BIG-IP source
        text the run consumed without the script passing it back in.
        """
        from .renderers import bind_render_sources
        from .renderers import render as _render

        with bind_render_sources(self._sources):
            return _render(name, self.values(), **opts)

    def edits(self, uri: str) -> str | None:
        """Return the post-edit source text for *uri*, or ``None``.

        Convenience for callers that ran a mutating query and want the
        rewritten config without poking ``edits_per_file`` themselves.
        Returns ``None`` when the query made no edit on *uri* — which
        for a no-op mutation means "nothing changed", not "unknown
        URI".
        """
        applied = self.result.edits_per_file.get(uri)
        if applied is None:
            return None
        return applied.new_source


class Query:
    """A parsed-and-ready query that can be run against multiple inputs.

    Holds only the query text — the parser is invoked inside
    :meth:`run` (via :func:`run_query`) so a stored :class:`Query` is
    cheap to pass around and safe to reuse across runs with different
    sources.

    The constructor accepts either a plain string (the DSL expression)
    or a :class:`~pathlib.Path` whose contents are read on construction
    — the latter mirrors the CLI's ``-f / --from-file`` shape.
    """

    __slots__ = ("text", "_origin")

    def __init__(self, text: str | Path) -> None:
        if isinstance(text, Path):
            self._origin = str(text)
            self.text = text.read_text(encoding="utf-8")
        else:
            self._origin = "<inline>"
            self.text = text

    def __repr__(self) -> str:  # pragma: no cover - debug aid
        snippet = self.text.strip().splitlines()[0] if self.text.strip() else ""
        return f"Query({snippet[:60]!r}, from={self._origin})"

    def run(
        self,
        *,
        sources: Mapping[str, str] | None = None,
        paths: Iterable[str | Path] | None = None,
        names: Mapping[str, str] | None = None,
        merge: bool = False,
        partitions: Mapping[str, str] | None = None,
        input_specs: Mapping[str, InputSpec] | None = None,
    ) -> QueryRun:
        """Parse, evaluate, and (if applicable) apply edits.

        At least one of *sources* / *paths* must be supplied.  Every
        other keyword is forwarded verbatim to :func:`run_query`; see
        its docstring for the full semantics around named-source
        binding, ``--merge`` mode, and structured side-inputs.

        Raises :class:`QueryError` on parse / evaluation failure
        (subclasses cover the specific stage); raises ``ValueError``
        when no sources are supplied.
        """
        source_map = _coerce_sources(sources, paths)
        # Auto-include any side-input URI the caller referenced via
        # ``input_specs`` but didn't pre-load via *sources* / *paths*.
        # The runner expects every URI in ``input_specs`` to also
        # appear in ``sources`` (that's what the CLI does with
        # ``read_path`` for each ``--input-<kind>`` binding); reading
        # them here keeps Python callers from having to do the same
        # double-bookkeeping by hand.
        if input_specs:
            for uri in input_specs:
                if uri in source_map:
                    continue
                p = Path(uri)
                if p.is_file():
                    source_map[uri] = p.read_text(encoding="utf-8")
        if not source_map:
            raise ValueError(
                "Query.run() needs at least one source — pass sources={URI: TEXT} "
                "and/or paths=[...]"
            )
        result = run_query(
            self.text,
            source_map,
            names=dict(names) if names else None,
            merge=merge,
            partitions=dict(partitions) if partitions else None,
            input_specs=dict(input_specs) if input_specs else None,
        )
        return QueryRun(result, sources=source_map)


def _expand(values: Iterable[Any]) -> Iterable[Any]:
    """Flatten :class:`Stream` wrappers the runner left in place.

    The runner's per-file accumulators sometimes hand back a
    :class:`Stream` directly when a top-level pipe stage produced one;
    the CLI's output renderers call ``_flat`` for the same reason.
    External callers expect a flat list of values out of ``.values()``,
    so we flatten the same way here.
    """
    for v in values:
        if isinstance(v, Stream):
            yield from v.items
        else:
            yield v


__all__ = (
    "Query",
    "QueryRow",
    "QueryRun",
    "QueryError",
)
