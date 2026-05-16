"""Reverse index: per-module set of registered object-type strings.

The parser's :func:`_parse_generic_header` uses this to choose the
longest matching ``(module, object_type)`` prefix when a stanza header
has more than two tokens — so ``security protocol-inspection
compliance-objects /Common/x`` correctly resolves to module
``security`` + type ``protocol-inspection compliance-objects`` rather
than the whitespace-heuristic mis-segmentation.

The index is built lazily on first access and cached for the lifetime
of the process.  It depends only on the static registry catalogue, so
no invalidation is needed.
"""

from __future__ import annotations

from functools import lru_cache


@lru_cache(maxsize=1)
def header_object_types_by_module() -> dict[str, frozenset[str]]:
    """Return ``{module: {object_type, ...}}`` over every registered kind."""
    # Late import: ``core.bigip.registry`` pulls in a lot of state and
    # we don't want parser-helper import order to hinge on it.
    from .registry.data import HEADER_KIND_MAP

    out: dict[str, set[str]] = {}
    for module, object_type in HEADER_KIND_MAP:
        out.setdefault(module, set()).add(object_type)
    return {module: frozenset(types) for module, types in out.items()}
