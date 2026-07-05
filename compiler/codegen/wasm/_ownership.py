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

"""Compile-side TclObj ownership tag (S2.1).

Every value the codegen leaves on the WASM operand stack carries a
compile-time tag describing its refcount relationship to the
caller:

- :data:`OWNED` — the value brought a +1 with it (it is the result
  of an ``obj_new_*`` call, a runtime command's return, an ``expr``
  computation that allocates, or a concat of literal pieces).  The
  caller can transfer that +1 directly into a slot without having
  to retain.

- :data:`BORROWED` — the value was read from another slot via
  ``local.get`` and the +1 stays in the source slot.  If the caller
  wants to put the value into a different owning slot it must
  ``tcl_obj_retain`` to claim its own +1.

This split is the keystone for S2: without it, the failed previous
attempt over-counted owned values (always retaining, even when the
value already came with a +1) and broke the rc==1 in-place fast
paths in the runtime.

The tag is consumed by :meth:`_emit_owned_local_write` (S2.2).
Currently only that one consumer is interested; if more sites need
the tag, expose helpers here rather than introducing new enums.
"""

from __future__ import annotations

from enum import Enum


class Ownership(Enum):
    """Refcount relationship of a stack value to its source."""

    #: The value brought a +1 with it.  Storing into a slot transfers
    #: that +1; the slot then owns it without an extra retain.
    OWNED = "owned"

    #: The value is borrowed from another owning slot.  Storing into
    #: a new slot must retain to claim a separate +1; the source slot
    #: keeps its original +1 so the borrowed handle stays valid.
    BORROWED = "borrowed"

    @property
    def is_owned(self) -> bool:
        """True when storing the value transfers a +1 to the slot."""
        return self is Ownership.OWNED

    @property
    def is_borrowed(self) -> bool:
        """True when storing the value requires the slot to retain."""
        return self is Ownership.BORROWED
