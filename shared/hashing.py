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
