"""F5 BIG-IP query projection layer (package facade).

The projection maps a parsed :class:`BigipConfig` into the DSL's
:class:`Container` / :class:`ObjectRef` / :class:`FieldSpec` value
space.  The implementation is split by concern:

- :mod:`._classes` — :class:`Container` namespace abstraction and
  :class:`FieldSpec` descriptor.
- :mod:`._data` — pure static dispatch tables: per-kind field
  maps, ``_KIND_FIELD_MAPS``, ``_MODULE_KINDS``.
- :mod:`._engine` — the lazy projection engine that reads
  :mod:`._data` and emits :class:`Container` / :class:`ObjectRef`
  instances on demand (``root_container`` is the public entry).

This ``__init__`` is the stable import point.  Downstream code uses
``from core.bigip.query.projection import MODULE_KINDS`` /
``Container`` / ``FieldSpec`` / ``root_container``.
"""

from __future__ import annotations

from ._classes import Container, FieldSpec
from ._data import LTM_KINDS, MODULE_KINDS
from ._engine import root_container

__all__ = [
    "Container",
    "FieldSpec",
    "LTM_KINDS",
    "MODULE_KINDS",
    "root_container",
]
