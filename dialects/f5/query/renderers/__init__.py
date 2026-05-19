"""Pluggable output renderers for the query DSL.

Renderers turn a list of evaluator output values (``ObjectRef`` /
``PathRef`` / dicts / scalars / ``Stream`` instances) into a
human-readable string.  They sit alongside the built-in output modes in
:mod:`dialects.f5.query.output` and are reachable from the CLI via
``f5 q --render NAME`` and from Python via
:func:`dialects.f5.query.render`.

Registering a renderer
----------------------

Renderers self-register on import with the :func:`renderer` decorator,
the same shape used by ``@verb`` in :mod:`explorer.verbs.f5._registry`
and ``@_register`` in :mod:`dialects.f5.query.builtins`:

.. code-block:: python

    from dialects.f5.query.renderers import renderer

    @renderer(
        "json-lines",
        summary="One JSON value per line (NDJSON).",
        accepts="any",
    )
    def _render_jsonl(values, **opts):
        import json
        return "\\n".join(json.dumps(v) for v in values) + "\\n"

The first import of :mod:`dialects.f5.query` triggers the renderer
sub-modules so the built-ins (``mermaid`` / ``gantt`` / ``ascii-blocks``)
are always available without an explicit import on the caller's side.

Renderer contract
-----------------

A renderer is a callable ``(values: list[Any], **opts: str) -> str``.

- *values* — the same flattened list the rest of
  :mod:`~dialects.f5.query.output` sees.  Renderers SHOULD treat unknown
  shapes by falling back to a reasonable default rather than raising
  ``BuiltinError`` so the user can swap renderers without rewriting the
  query.
- *opts* — string key/value pairs supplied via ``--render-opt KEY=VALUE``
  on the CLI, or passed as keyword arguments from Python.  Renderers
  parse and validate their own options; type coercion is the renderer's
  responsibility.
- Return value — a single string, ready to write to stdout.  Renderers
  MUST NOT print directly.

Renderers raise :class:`~dialects.f5.query.errors.RendererError` for
caller-visible problems (unknown option, malformed input shape, missing
required argument).  Any other exception is treated as a bug and
propagates with the original traceback.
"""

from __future__ import annotations

import contextlib
from contextvars import ContextVar
from dataclasses import dataclass
from typing import Any, Callable

# Per-render context: the originating source text for each URI the
# values came from.  ``QueryRun.render`` populates this for the
# duration of a renderer dispatch so renderers like ``mermaid`` can
# recover the BIG-IP source text without the script having to pass it
# back in.  Default ``None`` means "no sources known" — renderers that
# care must handle that gracefully (e.g. by falling back to a generic
# rendering mode).
RENDER_SOURCES: ContextVar[dict[str, str] | None] = ContextVar(
    "f5_query_render_sources", default=None
)


@contextlib.contextmanager
def bind_render_sources(sources: dict[str, str] | None):
    """Expose *sources* to the renderer running inside the ``with`` block.

    Used by :meth:`dialects.f5.query.api.QueryRun.render` to surface
    the source text the run consumed; renderers read it via
    :data:`RENDER_SOURCES`.
    """
    token = RENDER_SOURCES.set(sources)
    try:
        yield
    finally:
        RENDER_SOURCES.reset(token)


@dataclass(frozen=True, slots=True)
class RendererSpec:
    """Metadata for one registered renderer.

    *accepts* is a short free-text shape hint shown by
    ``--help-renderers`` (``"stream of ObjectRef"``,
    ``"rows of (ts, label, state)"``) — the registry does not
    enforce it.
    """

    name: str
    summary: str
    accepts: str
    impl: Callable[..., str]
    details: str = ""


_REGISTRY: dict[str, RendererSpec] = {}


def renderer(
    name: str,
    *,
    summary: str,
    accepts: str = "any",
    details: str = "",
) -> Callable[[Callable[..., str]], Callable[..., str]]:
    """Register *fn* as the implementation of renderer *name*.

    Duplicate registrations are rejected at import time so a typo in a
    plugin module can't silently shadow a built-in.
    """

    def decorator(fn: Callable[..., str]) -> Callable[..., str]:
        if name in _REGISTRY:
            raise RuntimeError(f"duplicate renderer registration: {name!r}")
        _REGISTRY[name] = RendererSpec(
            name=name,
            summary=summary,
            accepts=accepts,
            impl=fn,
            details=details,
        )
        return fn

    return decorator


def lookup(name: str) -> RendererSpec | None:
    """Return the spec for *name*, or ``None`` if no such renderer."""
    _ensure_builtins_loaded()
    return _REGISTRY.get(name)


def list_renderers() -> list[RendererSpec]:
    """Return every registered renderer, sorted by name."""
    _ensure_builtins_loaded()
    return sorted(_REGISTRY.values(), key=lambda s: s.name)


def render(name: str, values: list[Any], **opts: Any) -> str:
    """Look up *name* and dispatch the renderer with *values* and *opts*.

    Raises :class:`~dialects.f5.query.errors.RendererError` if *name* is
    not registered.  Other errors from the renderer itself propagate so
    the CLI's error path can surface them.
    """
    from ..errors import RendererError

    spec = lookup(name)
    if spec is None:
        names = ", ".join(s.name for s in list_renderers())
        raise RendererError(f"unknown renderer {name!r} (registered: {names})")
    return spec.impl(values, **opts)


_BUILTINS_LOADED = False


def _ensure_builtins_loaded() -> None:
    """Import the built-in renderer modules and any user plugins on first use.

    Keeping the in-tree imports lazy here means ``import dialects.f5.query``
    doesn't drag in the mermaid / gantt / ascii-blocks modules for
    callers that never ask for them, but ``--render gantt`` and
    ``lookup("gantt")`` Just Work without the caller needing to know
    where the renderer lives.

    The same call also triggers
    :func:`~dialects.f5.query.plugins.load_user_plugins` so renderers
    dropped into ``$XDG_CONFIG_HOME/f5q/plugins/`` are visible to the
    CLI and the Python API alike.
    """
    global _BUILTINS_LOADED
    if _BUILTINS_LOADED:
        return
    _BUILTINS_LOADED = True
    from ..plugins import load_user_plugins
    from . import ascii_blocks, gantt, mermaid  # noqa: F401  (registration side-effect)

    load_user_plugins()
