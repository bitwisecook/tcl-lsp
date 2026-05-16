"""Fluent Python API on top of :func:`run_query`.

External scripts that want to leverage the query engine — to drive a
custom report, draw a diagram, or feed an integration test — go
through this module rather than calling :func:`run_query` directly.
The lower-level entry point still works (and is what the wrappers
here call internally); the fluent shape just trims the ceremony
around source dicts, per-file result unpacking, and renderer
dispatch.

Three call shapes cover essentially every script's needs:

1. **One-shot.**  ``f5q.q(".ltm.virtual[]", "bigip.conf")`` loads the
   config and runs the query in one call.  The first string that is
   not an existing file path is the expression; everything else is
   an input.
2. **Progressive.**  Feed a prior result back in to refine:
   ``f5q.q("select(.pool != null)", prior)`` runs the new
   expression against ``prior``'s values (which become a JSON list
   the new expression's ``.`` reads).  Method form
   ``prior.q("...")`` is identical.
3. **Pre-staged.**  ``f5q.load("a.conf", "b.conf")`` returns a
   :class:`Sources` object; downstream calls take it the same way
   they take a file path.  Useful when the same corpus feeds many
   queries.

Each step is **immutable** — every ``q()`` returns a fresh
:class:`QueryRun`.  The string DSL is the only place ``|`` /
``select`` / ``map`` etc. happen; the Python verbs (``q``,
``render``, ``out``) compose whole queries rather than per-stage
transforms.  Inline callables are accepted everywhere a registered
plugin name is — pass a function to :meth:`QueryRun.render` or to
:func:`load` / :func:`q` (via ``input=``) when a one-off doesn't
deserve a named XDG plugin.

Typical scripts::

    import f5q

    # One-shot.
    for name in f5q.q(".ltm.virtual[] | .name", "bigip.conf"):
        print(name)

    # Progressive — file then filter then render.
    print(
        f5q.q(".ltm.virtual[]", "bigip.conf")
            .q("select(.pool != null)")
            .render("ascii-blocks")
    )

    # In-memory data + inline renderer.
    text = f5q.q(".[] | select(.x > 5)", [{"x": 1}, {"x": 10}]).render(
        lambda values, **opts: ", ".join(str(v) for v in values) + "\\n"
    )

    # Custom file format — one-off, no XDG plugin needed.
    routes = f5q.load("routes.xml", parser=my_xml_parser)
    print(f5q.q(".items[].name", routes).out())
"""

from __future__ import annotations

import json as _json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Sequence, Union

from ._inputs import InputSpec
from .errors import QueryError
from .runner import QueryResult, run_query
from .values import ObjectRef, PathRef, Stream, resolve_lazy_field

# Type alias used by render() / load() / q() to accept either a
# registered plugin name (looked up in the registry) or an inline
# callable matching the same contract.  Renderers take
# ``(values, **opts) -> str``; input parsers take
# ``(source, *, uri, options=()) -> Any``.
RendererArg = Union[str, Callable[..., str]]
ParserArg = Callable[..., Any]


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


@dataclass
class Sources:
    """Pre-loaded inputs ready to feed into :func:`q`.

    A :class:`Sources` carries the source text for one or more files
    (plus, for non-BIG-IP formats, the parser to apply) without
    running a query against them.  Useful when one corpus feeds
    multiple queries — load once, query many times.

    Construct with :func:`load`; pass to :func:`q` exactly the way
    you'd pass a file path:

    .. code-block:: python

        corpus = f5q.load("ltm.conf", "gtm.conf")
        virtuals = f5q.q(".ltm.virtual[]", corpus)
        pools = f5q.q(".ltm.pool[]", corpus)

    The default ``parser=None`` treats each file as a BIG-IP config;
    pass ``parser=<callable>`` to handle a custom file format inline,
    or ``parser="<format-name>"`` to use a registered
    ``@input_format`` plugin.
    """

    text_by_uri: dict[str, str] = field(default_factory=dict)
    input_specs: dict[str, InputSpec] = field(default_factory=dict)
    # Parser callables passed inline via ``load(..., parser=fn)``.
    # Wired in alongside ``input_specs`` by registering the parser as
    # a one-shot input format under a synthesised name.
    inline_parsers: dict[str, ParserArg] = field(default_factory=dict)

    def merged(self, other: "Sources") -> "Sources":
        """Return a new ``Sources`` that combines this and *other*.

        URIs in *other* shadow ones in self on conflict; callers that
        need a strict merge should detect collisions before calling.
        """
        out = Sources()
        out.text_by_uri.update(self.text_by_uri)
        out.input_specs.update(self.input_specs)
        out.inline_parsers.update(self.inline_parsers)
        out.text_by_uri.update(other.text_by_uri)
        out.input_specs.update(other.input_specs)
        out.inline_parsers.update(other.inline_parsers)
        return out


class QueryRun:
    """The flattened result of one :meth:`Query.run` / :func:`q`.

    Wraps the per-URI :class:`QueryResult` the runner returns and
    exposes the conveniences external scripts usually want — a single
    flat list of values, a "first or None" lookup, the path-only
    projection, a progressive :meth:`q` for chaining, and a one-call
    :meth:`render` that dispatches through the renderer registry or
    an inline callable.

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

    def __getitem__(self, idx: int) -> Any:
        return self.values()[idx]

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

    def q(self, expression: str, *extra_inputs: Any) -> "QueryRun":
        """Run *expression* against this result's values.

        The current values are surfaced as the new primary input
        (``.`` in the chained expression reads the synthesised JSON
        list).  Standard jq idioms apply — write ``.[]`` to iterate
        per-value, or operate on the list as a whole (``length``,
        ``count``, ``[.[] | .name]``, ...).

        Additional *extra_inputs* are merged in alongside the prior
        values — useful for "join my prior result against another
        config" patterns.  See :func:`q` for the supported input
        shapes.
        """
        return q(expression, self, *extra_inputs)

    def render(self, renderer: RendererArg, **opts: Any) -> str:
        """Render the values via a registered renderer or an inline callable.

        *renderer* is either:

        - a **string** — looked up in the renderer registry
          (``f5 q --help-renderers`` for the catalogue); or
        - a **callable** with signature ``(values, **opts) -> str``
          — used directly without registration.  Pass a function
          when the renderer is one-off and doesn't deserve a named
          XDG plugin.

        Either form binds the originating source text into the
        :data:`~core.bigip.query.renderers.RENDER_SOURCES` contextvar
        so renderers like ``mermaid`` can recover the BIG-IP source
        text the run consumed without the script passing it back in.
        """
        from .renderers import bind_render_sources
        from .renderers import render as _render

        with bind_render_sources(self._sources):
            if isinstance(renderer, str):
                return _render(renderer, self.values(), **opts)
            return renderer(self.values(), **opts)

    def out(self) -> list[Any]:
        """Return the values as plain JSON-compatible Python.

        Walks every value and coerces engine-typed wrappers:

        - :class:`ObjectRef` →
          ``{"kind": ..., "full-path": ..., "fields": {...}}``
        - :class:`PathRef` → ``str`` (the full-path)
        - :class:`Stream` → ``list``
        - dict / list — recursed
        - scalars (str / int / float / bool / None) — left as-is

        Use this when the script needs to serialise results to JSON,
        feed another library that expects plain dicts, or just print
        a human-readable dump.  Iterating ``self`` directly keeps the
        typed handles (``.full_path`` / ``.fields`` / ``.config_uri``)
        if you want to walk back into the originating config.
        """
        return [_to_plain(v) for v in self.values()]

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


# ---------------------------------------------------------------------------
# Public functional API: load() and q()
# ---------------------------------------------------------------------------


# Counter for synthesised URIs (in-memory data + inline parsers).
# Keeping it module-level rather than per-call so each input gets a
# unique key even when one ``q()`` mixes several anonymous inputs.
_synth_counter = 0


def _next_synth_uri(prefix: str) -> str:
    global _synth_counter
    _synth_counter += 1
    return f"<{prefix}:{_synth_counter}>"


def load(
    *inputs: Any,
    parser: str | ParserArg | None = None,
) -> Sources:
    """Stage one or more inputs without running a query against them.

    Each *input* is one of:

    - a **string** that is an existing file path — read from disk;
    - a :class:`~pathlib.Path` — read from disk;
    - a **dict / list** — bound as in-memory JSON data;
    - a :class:`Sources` — merged in.

    Returns a :class:`Sources` object you can pass to :func:`q` the
    same way you pass a file path.  Useful when one corpus feeds
    multiple queries.

    *parser* applies to file inputs only and selects how to parse
    each file:

    - ``None`` (default) — treat as a BIG-IP config;
    - a **string** — name of a registered ``@input_format`` plugin
      (``"json"`` / ``"jsonl"`` / ``"csv"`` / ``"f5log"`` built-in,
      or a user-registered name);
    - a **callable** matching the
      ``(source, *, uri, options=()) -> Any`` contract — used
      directly without registration.  The escape hatch for one-off
      custom file formats that don't deserve a named XDG plugin.

    Examples:

    .. code-block:: python

        # BIG-IP configs (default).
        corpus = f5q.load("ltm.conf", "gtm.conf")

        # JSON sidecar via a registered format name.
        cmdb = f5q.load("cmdb.json", parser="json")

        # In-memory data — no file involved.
        rows = f5q.load([{"x": 1}, {"x": 10}])

        # Custom file format with an inline parser.
        routes = f5q.load("routes.xml", parser=parse_xml)
    """
    out = Sources()
    for item in inputs:
        if isinstance(item, Sources):
            out = out.merged(item)
        elif isinstance(item, QueryRun):
            # Treat the prior result as a chunk of in-memory JSON
            # data — same shape ``q(expr, prev)`` produces below.
            uri = _next_synth_uri("chain")
            out.text_by_uri[uri] = _json.dumps([_to_plain(v) for v in item.values()])
            out.input_specs[uri] = InputSpec(kind="json")
        elif isinstance(item, (str, Path)) and Path(item).is_file():
            uri = str(Path(item))
            out.text_by_uri[uri] = Path(item).read_text(encoding="utf-8")
            if parser is not None:
                out.input_specs[uri] = _spec_for_parser(parser, out, uri)
        elif isinstance(item, str):
            raise ValueError(
                f"load(): {item!r} is neither an existing file path nor "
                "a Sources / QueryRun / dict / list — did you mean to "
                "pass a Python value directly?"
            )
        elif isinstance(item, (dict, list, tuple)):
            uri = _next_synth_uri("inline")
            out.text_by_uri[uri] = _json.dumps(item)
            out.input_specs[uri] = InputSpec(kind="json")
        else:
            raise TypeError(
                f"load(): unsupported input type {type(item).__name__} "
                f"(expected file path, Sources, QueryRun, dict, or list)"
            )
    return out


def _spec_for_parser(
    parser: str | ParserArg,
    sources: Sources,
    uri: str,
) -> InputSpec:
    """Resolve a *parser* argument into an :class:`InputSpec`.

    A string parser names a registered format; a callable is wired
    in as a one-shot format under a synthesised name registered on
    the *sources* object for the duration of the call.
    """
    if isinstance(parser, str):
        return InputSpec(kind=parser)
    if callable(parser):
        sources.inline_parsers[uri] = parser
        # The synthesised kind name is unique per uri so two inline
        # parsers can coexist in one ``q()`` call.
        return InputSpec(kind=f"<inline-parser:{uri}>")
    raise TypeError(
        f"parser= must be a registered format name (str) or a callable "
        f"(source, *, uri, options=()) -> Any (got {type(parser).__name__})"
    )


def q(*args: Any, parser: str | ParserArg | None = None) -> QueryRun:
    """Run a query end-to-end in one call.

    *args* combines an optional expression with one or more inputs.
    Argument disambiguation is positional and unambiguous:

    - The **first string that is not an existing file path** is the
      expression.  At most one expression is permitted.
    - Strings that **are** file paths, :class:`Path` objects, and
      :class:`Sources` instances are inputs.
    - :class:`QueryRun` instances are also inputs; multiple priors
      combine into a single flat list of values so the chained
      expression sees ``.`` as the union of every prior.
    - **dict / list / tuple** are inputs too — bound as in-memory
      JSON data the expression can navigate naturally.
    - If no expression is supplied, the identity query ``.`` runs
      against the combined inputs (so ``q(file)`` is equivalent to
      ``q(".", file)``).

    *parser* applies to file inputs the same way as on :func:`load`:
    pass a string format name or a callable to override the default
    BIG-IP parser for any file path in *args*.

    Chain semantics (one or more :class:`QueryRun` inputs):

    - **Only priors** — every prior's values are concatenated into
      one list and bound as the primary input.  The chained
      expression's ``.`` reads that list, so ``.[] | <expr>`` is
      the standard jq idiom for "do this per prior value".
    - **Priors + other inputs** — the priors' combined values are
      bound as ``$_chain`` (still one list across all priors); the
      file / in-memory inputs act as primary.  Useful when joining
      a prior list against a config:
      ``q(".ltm.virtual[] | select(contains($_chain, .name))",
      virtuals_to_keep, "bigip.conf")``.

    Examples:

    .. code-block:: python

        f5q.q(".ltm.virtual[] | .name", "bigip.conf")
        f5q.q(".ltm.virtual[] | .name", corpus)               # Sources
        f5q.q(".[] | select(.x > 5)", prior_result)           # progressive
        f5q.q(".[]", run_a, run_b, run_c)                     # merge priors
        f5q.q(".[] | select(.x > 5)", [{"x": 1}, {"x": 10}])  # in-memory
        f5q.q(".items[].name", "routes.xml", parser=parse_xml)

    Raises :class:`ValueError` when *args* is empty or when two
    expression strings are supplied; :class:`QueryError` on parse /
    evaluation failure.
    """
    expression, inputs = _split_q_args(args)

    # Split priors from everything else so we can flatten them into
    # one combined JSON source — matches the user-facing promise that
    # ``q(expr, run_a, run_b, run_c)`` sees the union of values, not
    # three separate per-file iterations.
    priors: list[QueryRun] = [i for i in inputs if isinstance(i, QueryRun)]
    other_inputs: list[Any] = [i for i in inputs if not isinstance(i, QueryRun)]

    staged = Sources()
    for item in other_inputs:
        staged = staged.merged(_stage_one(item, parser))

    names: dict[str, str] | None = None
    if priors:
        combined = [_to_plain(v) for run in priors for v in run.values()]
        chain_uri = _next_synth_uri("chain")
        staged.text_by_uri[chain_uri] = _json.dumps(combined)
        staged.input_specs[chain_uri] = InputSpec(kind="json")
        # When the priors are combined with other inputs (files /
        # in-memory data), the file is the primary input and the
        # priors are reachable as ``$_chain``.  When priors are the
        # *only* input, the synthesised JSON source acts as primary
        # automatically — the runner's all-JSON fallback handles it.
        if other_inputs:
            names = {"_chain": chain_uri}

    if not staged.text_by_uri:
        raise ValueError(
            "q() needs at least one input — pass a file path, a Sources "
            "loaded via load(), a prior QueryRun, or a Python value"
        )

    # Register inline parsers as one-shot @input_format plugins.  The
    # registration is scoped to this call: we add the synthesised
    # name, run the query, then remove it.  Doing this here (rather
    # than in load()) keeps the registry side-effect tightly scoped
    # to the q() invocation that uses the parser.
    inline_kinds_added: list[str] = []
    if staged.inline_parsers:
        from .inputs import _REGISTRY as _INPUT_REGISTRY
        from .inputs import InputFormatSpec
        from .inputs import _ensure_loaded as _ensure_inputs_loaded

        _ensure_inputs_loaded()
        for uri, fn in staged.inline_parsers.items():
            kind = f"<inline-parser:{uri}>"
            _INPUT_REGISTRY[kind] = InputFormatSpec(
                name=kind,
                summary="inline parser",
                impl=fn,
            )
            inline_kinds_added.append(kind)

    try:
        run = Query(expression).run(
            sources=staged.text_by_uri,
            input_specs=staged.input_specs or None,
            names=names,
        )
    finally:
        if inline_kinds_added:
            from .inputs import _REGISTRY as _INPUT_REGISTRY

            for kind in inline_kinds_added:
                _INPUT_REGISTRY.pop(kind, None)
    return run


def _split_q_args(args: Sequence[Any]) -> tuple[str, list[Any]]:
    """Split positional args into (expression, inputs).

    Returns ``("."` , inputs)`` when no expression was given so the
    runner evaluates the identity query against the combined inputs
    — equivalent to ``load()`` plus an implicit identity run.

    Raises :class:`ValueError` for empty args, for two expression
    strings, or for a string that is neither an expression nor an
    existing file (so a typo doesn't silently become a no-input
    query).
    """
    if not args:
        raise ValueError(
            "q() needs at least one argument — pass an expression, a file path, or both"
        )
    expression: str | None = None
    inputs: list[Any] = []
    for arg in args:
        if isinstance(arg, str):
            if Path(arg).is_file():
                inputs.append(arg)
                continue
            if expression is not None:
                raise ValueError(
                    f"q(): two expression strings supplied "
                    f"({expression!r} and {arg!r}) — only one is allowed; "
                    "did you forget a comma or mean to pass an input?"
                )
            expression = arg
        else:
            inputs.append(arg)
    if expression is None:
        expression = "."
    return expression, inputs


def _stage_one(item: Any, parser: str | ParserArg | None) -> Sources:
    """Stage one positional *non-QueryRun* input arg into a fresh :class:`Sources`.

    Routes by type:

    - :class:`Sources` — returned as-is (merged in).
    - file path (str / Path) — loaded with the given *parser*.
    - in-memory dict / list / tuple — wrapped as a JSON source.

    :class:`QueryRun` priors are handled in :func:`q` directly so
    multiple priors combine into a single source rather than turning
    into separate per-file iterations.
    """
    out = Sources()
    if isinstance(item, Sources):
        return out.merged(item)
    if isinstance(item, Path) or (isinstance(item, str) and Path(item).is_file()):
        path = Path(item)
        uri = str(path)
        out.text_by_uri[uri] = path.read_text(encoding="utf-8")
        if parser is not None:
            out.input_specs[uri] = _spec_for_parser(parser, out, uri)
        return out
    if isinstance(item, (dict, list, tuple)):
        uri = _next_synth_uri("inline")
        out.text_by_uri[uri] = _json.dumps(item)
        out.input_specs[uri] = InputSpec(kind="json")
        return out
    if isinstance(item, str):
        raise ValueError(
            f"q(): {item!r} is neither an existing file path nor a known "
            "input type — pass a file path, Sources, QueryRun, dict, or list"
        )
    raise TypeError(
        f"q(): unsupported input type {type(item).__name__} "
        f"(expected file path, Sources, QueryRun, dict, or list)"
    )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


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


def _to_plain(v: Any) -> Any:
    """Coerce one engine value to a JSON-compatible Python primitive.

    Mirrors :func:`core.bigip.query.output._to_json` but lives here so
    :meth:`QueryRun.out` and the inline-JSON synthesis path (for
    progressive chaining) share one definition.
    """
    if isinstance(v, ObjectRef):
        return {
            "kind": v.kind,
            "full-path": v.full_path,
            "fields": {k: _to_plain(resolve_lazy_field(v, k, val)) for k, val in v.fields.items()},
        }
    if isinstance(v, PathRef):
        return v.full_path
    if isinstance(v, Stream):
        return [_to_plain(x) for x in v.items]
    if isinstance(v, dict):
        return {k: _to_plain(val) for k, val in v.items()}
    if isinstance(v, (list, tuple)):
        return [_to_plain(x) for x in v]
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)


__all__ = (
    "Query",
    "QueryError",
    "QueryRow",
    "QueryRun",
    "Sources",
    "load",
    "q",
)
