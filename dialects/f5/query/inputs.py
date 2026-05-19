"""Pluggable input-format registry for side-inputs.

Side-inputs (``--input-json`` / ``--input-jsonl`` / ``--input-csv`` /
``--input-f5log``, plus user-defined formats) bind a file's parsed
content to a DSL variable.  This module owns the per-format dispatch
table — what parser handles each ``InputSpec.kind`` — and exposes the
``@input_format`` decorator that lets external plugins add their own
formats.

The four built-in formats (json, jsonl, csv, f5log) register
themselves on import; user plugins add more via the same decorator:

.. code-block:: python

    # ~/.config/f5q/plugins/yaml_input.py
    from dialects.f5.query import input_format

    @input_format("yaml", summary="YAML side-input (single document or stream).")
    def _parse_yaml(source, *, uri, options=()):
        import yaml
        loaded = list(yaml.safe_load_all(source))
        return loaded[0] if len(loaded) == 1 else loaded

After the plugin loads, ``--input yaml routes=routes.yaml`` on the
CLI (or ``input_specs={"routes.yaml": InputSpec(kind="yaml")}`` from
Python) binds the parsed YAML tree to ``$routes`` exactly the way
built-in JSON inputs do.

Parser contract
---------------

A parser is a callable

.. code-block:: python

    def parse(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any

- *source* — the file contents as text.  Binary formats need the
  file pre-decoded; the loader currently requires UTF-8 sources.
- *uri* — the file path / URI of the input, for use in error
  messages.  Parsers SHOULD include it in any exception they raise.
- *options* — frozen tuple of ``(key, value)`` pairs from
  ``InputSpec.options``.  The format's own decoder reads what it
  understands and ignores the rest.
- Return value — a Python value the DSL can navigate naturally
  (dict, list of dicts, scalar).  Streams are not required; the
  evaluator wraps lists in :class:`~dialects.f5.query.values.Stream`
  when the user writes ``$name[]``.

Parsers raise :class:`~dialects.f5.query.errors.QueryError` on
malformed input (the runner re-wraps in a context-aware message);
other exceptions propagate as bugs.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable

ParserFn = Callable[..., Any]


@dataclass(frozen=True, slots=True)
class InputFormatSpec:
    """Metadata for one registered input format.

    ``name`` is the lookup key used by :class:`InputSpec.kind` and
    by the CLI's ``--input KIND NAME=PATH`` flag.
    """

    name: str
    summary: str
    impl: ParserFn
    details: str = ""


_REGISTRY: dict[str, InputFormatSpec] = {}


def input_format(
    name: str,
    *,
    summary: str = "",
    details: str = "",
) -> Callable[[ParserFn], ParserFn]:
    """Register *fn* as the parser for input format *name*.

    Duplicate registrations raise at import time so a typo in a
    plugin can't silently shadow a built-in format.  Plugin authors
    typically use a unique prefix (``mycorp-routes``) to avoid
    collisions with future built-ins.

    Example:

    .. code-block:: python

        from dialects.f5.query import input_format

        @input_format("xml", summary="XML side-input (root element to dict).")
        def _parse_xml(source, *, uri, options=()):
            import xml.etree.ElementTree as ET
            root = ET.fromstring(source)
            return _xml_to_dict(root)
    """

    def decorator(fn: ParserFn) -> ParserFn:
        if name in _REGISTRY:
            raise RuntimeError(f"duplicate input format registration: {name!r}")
        _REGISTRY[name] = InputFormatSpec(
            name=name,
            summary=summary,
            impl=fn,
            details=details,
        )
        return fn

    return decorator


def lookup(name: str) -> InputFormatSpec | None:
    """Return the parser spec for *name*, or ``None`` if not registered."""
    _ensure_loaded()
    return _REGISTRY.get(name)


def list_input_formats() -> list[InputFormatSpec]:
    """Return every registered input format, sorted by name."""
    _ensure_loaded()
    return sorted(_REGISTRY.values(), key=lambda s: s.name)


_LOADED = False


def _ensure_loaded() -> None:
    """Register the four built-in formats and auto-load user plugins.

    The built-in registrations live in a separate module
    (:mod:`._builtin_inputs`) so this module stays free of decorator
    side-effects at import time — the same pattern the renderer
    registry uses.
    """
    global _LOADED
    if _LOADED:
        return
    _LOADED = True
    from . import _builtin_inputs  # noqa: F401  (registration side-effect)
    from .plugins import load_user_plugins

    load_user_plugins()
