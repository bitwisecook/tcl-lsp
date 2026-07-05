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

"""Deterministic, process-stable hashing for cross-process cache identity.

Python's builtin ``hash()`` for ``str``/``bytes`` is salted per process by
``PYTHONHASHSEED``, so a value hashed in a ``ProcessPoolExecutor`` worker (which
under ``forkserver``/``spawn`` gets a fresh seed) does not match the same text
hashed in the main process.  Any hash that crosses the process-pool boundary —
e.g. ``TopLevelChunk.source_hash`` returned from subprocess analysis and then
compared against locally-segmented chunks — must therefore be deterministic.
"""

from __future__ import annotations

import hashlib

__all__ = ["stable_text_hash"]


def stable_text_hash(text: str) -> int:
    """Return a deterministic 64-bit hash of *text*.

    Stable across processes and runs (unlike builtin ``hash()``).  Uses BLAKE2b
    truncated to 8 bytes; the result is a non-negative 64-bit integer.
    """
    return int.from_bytes(hashlib.blake2b(text.encode("utf-8"), digest_size=8).digest(), "big")
