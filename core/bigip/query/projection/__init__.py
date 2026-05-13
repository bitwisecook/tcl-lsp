"""F5 BIG-IP query projection layer (package facade).

The projection layer maps parsed :class:`BigipConfig` containers
into the DSL's :class:`Container` / :class:`ObjectRef` /
:class:`FieldSpec` value space.  The full implementation —
``_KIND_FIELD_MAPS``, ``_MODULE_KINDS``, per-kind projector
helpers, and the lazy entry-builder — lives in :mod:`._impl`.

This ``__init__`` is the stable import point: existing call sites
(``from core.bigip.query.projection import MODULE_KINDS``,
``from .projection import Container, root_container``, ...) keep
working after the implementation gets split into per-module files.
"""

from __future__ import annotations

from ._impl import (
    LTM_KINDS,
    MODULE_KINDS,
    Container,
    FieldSpec,
    root_container,
)

__all__ = [
    "Container",
    "FieldSpec",
    "LTM_KINDS",
    "MODULE_KINDS",
    "root_container",
]
