"""``mermaid`` renderer — Mermaid flowchart of a query result.

Two modes pick themselves up automatically:

- when every value is an :class:`~dialects.f5.query.values.ObjectRef`,
  builds the BIG-IP object reference sub-graph reachable from those
  seeds and emits the same Mermaid that ``f5 graph --format mermaid``
  produces.  This lets a query like
  ``.ltm.virtual["~/web_"]`` scope a diagram down to a partition or
  name pattern without leaving the DSL.
- otherwise emits a generic ``graph LR`` flowchart: one node per value
  (rendered via :func:`~dialects.f5.query.output._cell_str`),
  consecutive values connected with an arrow.  Useful when the query
  produces an ordered chain (a sequence of stages, an event timeline,
  a list of pool members) and the user wants a quick picture.

Options:

- ``direction`` — ``LR`` (default), ``TB``, ``RL``, ``BT``.
- ``reverse`` — ``true`` to flip ObjectRef-mode edges so the diagram
  shows incoming references (what depends on the seeds) instead of
  outgoing (what the seeds depend on).
- ``max-depth`` — integer BFS depth limit for ObjectRef-mode.
"""

from __future__ import annotations

from typing import Any

from ..errors import RendererError
from ..output import _cell_str, _flat
from ..values import ObjectRef, PathRef
from . import renderer

_DIRECTIONS = ("LR", "RL", "TB", "BT")


@renderer(
    "mermaid",
    summary="Mermaid flowchart — BIG-IP object graph (for ObjectRef streams) or a generic chain.",
    accepts="stream of ObjectRef (for the BIG-IP graph mode) or any ordered stream (chain mode)",
    details=(
        "ObjectRef-mode reuses the same engine as ``f5 graph --format mermaid``: "
        "walks the reference graph from each ObjectRef seed in the query result, "
        "limited by --render-opt max-depth=N and optionally reversed via "
        "--render-opt reverse=true.  Generic mode renders the values as a left-to-right "
        "chain of nodes — handy for ad-hoc ``{stage, next}`` projections."
    ),
)
def _render_mermaid(values: list[Any], **opts: Any) -> str:
    direction = str(opts.get("direction", "LR")).upper()
    if direction not in _DIRECTIONS:
        valid = ", ".join(_DIRECTIONS)
        raise RendererError(f"mermaid: unknown direction {direction!r} (expected one of {valid})")
    flat = _flat(values)
    object_refs = [v for v in flat if isinstance(v, ObjectRef)]
    if object_refs and len(object_refs) == len(flat):
        return _render_object_graph(
            object_refs,
            direction=direction,
            reverse=_parse_bool(opts.get("reverse", False), name="reverse"),
            max_depth=_parse_optional_int(opts.get("max-depth", opts.get("max_depth"))),
        )
    return _render_chain(flat, direction=direction)


def _render_object_graph(
    seeds: list[ObjectRef],
    *,
    direction: str,
    reverse: bool,
    max_depth: int | None,
) -> str:
    """Re-use ``f5 graph`` machinery on the union of the seed configs.

    Falls back to chain mode when none of the seeds carry a
    ``config_uri`` (synthetic ObjectRefs, e.g. from tests) — the graph
    engine needs the source/config maps to walk references.
    """
    from dialects.f5.bigip.parser import parse_bigip_conf

    from ...graph_export import export_graph

    # Group seeds by config_uri and pull their full-paths.
    seed_paths_by_uri: dict[str, list[str]] = {}
    for ref in seeds:
        if not ref.config_uri:
            continue
        seed_paths_by_uri.setdefault(ref.config_uri, []).append(ref.full_path)
    if not seed_paths_by_uri:
        return _render_chain(seeds, direction=direction)
    # Re-parse each contributing source so the graph engine has the
    # same per-URI BigipConfig + source-text inputs it needs.  This is
    # the same data export_graph takes from `f5 graph`; we synthesise
    # it here so the query renderer doesn't require the caller to keep
    # the original sources around.
    sources: dict[str, str] = {}
    configs: dict[str, Any] = {}
    for ref in seeds:
        if not ref.config_uri or ref.config_uri in sources:
            continue
        if ref.stanza_slot is None:
            continue
        # ObjectRef back-references know their source URI but not the
        # full source text; fall back to chain mode when we can't
        # recover it cheaply.
        source_text = _recover_source_text(ref)
        if source_text is None:
            return _render_chain(seeds, direction=direction)
        sources[ref.config_uri] = source_text
        configs[ref.config_uri] = parse_bigip_conf(source_text)
    if not sources:
        return _render_chain(seeds, direction=direction)
    all_seed_paths = tuple(p for paths in seed_paths_by_uri.values() for p in paths)
    export = export_graph(
        sources=sources,
        configs=configs,
        fmt="mermaid",
        seeds=all_seed_paths,
        reverse=reverse,
        max_depth=max_depth,
    )
    # export_graph hard-codes ``graph LR``; rewrite the first line so
    # the user's --render-opt direction= choice wins.
    text = export.text
    if direction != "LR" and text.startswith("graph LR"):
        text = f"graph {direction}" + text[len("graph LR") :]
    return text


def _render_chain(values: list[Any], *, direction: str) -> str:
    if not values:
        return f"graph {direction}\n"
    lines = [f"graph {direction}"]
    for i, v in enumerate(values):
        lines.append(f'  n{i}["{_node_label(v)}"]')
    for i in range(len(values) - 1):
        lines.append(f"  n{i} --> n{i + 1}")
    return "\n".join(lines) + "\n"


def _node_label(v: Any) -> str:
    if isinstance(v, ObjectRef):
        text = v.full_path or v.kind or ""
    elif isinstance(v, PathRef):
        text = v.full_path
    elif isinstance(v, dict):
        title = v.get("title") or v.get("label") or v.get("name")
        if title is not None:
            text = str(title)
        else:
            text = _cell_str(v)
    else:
        text = _cell_str(v)
    return text.replace('"', "'").replace("\n", " ")


def _recover_source_text(ref: ObjectRef) -> str | None:
    """Try to fetch the originating source text for *ref*.

    Looks in two places:

    1. The render-time :data:`~dialects.f5.query.renderers.RENDER_SOURCES`
       contextvar — populated by :meth:`QueryRun.render` so a Python
       script that calls ``Query(...).run(paths=[...]).render("mermaid")``
       Just Works without threading the source dict through by hand.
    2. The runner's :func:`_lookup_active_root` map — populated for the
       duration of an in-flight ``run_query`` call (e.g. when a builtin
       invokes a renderer mid-evaluation), as a fallback for code paths
       that don't go through :class:`QueryRun`.

    Returns ``None`` when neither has it; the renderer falls back to
    chain mode in that case.
    """
    from . import RENDER_SOURCES

    sources = RENDER_SOURCES.get()
    if sources is not None and ref.config_uri in sources:
        return sources[ref.config_uri]
    from ..runner import _lookup_active_root

    root = _lookup_active_root(ref.config_uri)
    if root is None:
        return None
    return root.source


def _parse_bool(v: Any, *, name: str) -> bool:
    if isinstance(v, bool):
        return v
    if isinstance(v, str):
        if v.lower() in ("true", "yes", "1", "on"):
            return True
        if v.lower() in ("false", "no", "0", "off", ""):
            return False
    raise RendererError(f"mermaid: {name} must be a boolean (got {v!r})")


def _parse_optional_int(v: Any) -> int | None:
    if v is None or v == "":
        return None
    try:
        return int(v)
    except (TypeError, ValueError) as exc:
        raise RendererError(f"mermaid: max-depth must be an integer (got {v!r})") from exc
