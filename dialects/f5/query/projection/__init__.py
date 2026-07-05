# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
``from dialects.f5.query.projection import MODULE_KINDS`` /
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
