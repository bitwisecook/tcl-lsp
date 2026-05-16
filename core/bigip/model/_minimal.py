"""Shared minimal data-class for the long-tail kinds.

Every "minimal" projection carries only ``name`` / ``full_path``
/ ``description`` / ``kind`` / ``range``.  Rather than mint a
separate dataclass per F5 module, every minimal kind shares
:class:`BigipMinimalObject` — the per-module aliases below are
kept so existing imports and ``isinstance`` checks continue to
work without an attribute-rename sweep.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range

# Bundles 17-20 — shared minimal shape for the long-tail ltm.*
# kinds (CGNAT / LSN, global-settings singletons, classification /
# URL-DB, tacdb).  Each kind keeps its own ``BigipConfig``
# attribute; the ``kind`` field preserves the full TMSH label.


@dataclass(frozen=True, slots=True)
class BigipMinimalObject:
    """Shared shape for every "minimal" projection.

    The DSL has hundreds of kinds — most carry only the identity
    tuple (``name``, ``full_path``) plus a ``description`` and the
    TMSH kind label (``kind``).  Rather than mint a separate
    dataclass per module, every minimal kind shares this one.  The
    discriminator is ``kind`` (e.g. ``"net routing as-path"``,
    ``"sys icall script"``), which the projection already exposes
    via ``.kind``.
    """

    name: str
    full_path: str
    kind: str = ""
    description: str = ""
    range: Range | None = None


# Per-module aliases — kept so existing imports and isinstance
# checks across the parser / projection / tests continue to work
# without an attribute renaming sweep.  All resolve to the same
# class, so ``isinstance(x, BigipNetMinimalObject)`` and
# ``isinstance(x, BigipSecurityMinimalObject)`` are the same
# runtime check.
BigipLtmMinimalObject = BigipMinimalObject
BigipNetMinimalObject = BigipMinimalObject
BigipApmMinimalObject = BigipMinimalObject
BigipPemMinimalObject = BigipMinimalObject
BigipSysMinimalObject = BigipMinimalObject
BigipVcmpMinimalObject = BigipMinimalObject
BigipCmMinimalObject = BigipMinimalObject
BigipCliMinimalObject = BigipMinimalObject
BigipApiProtectionMinimalObject = BigipMinimalObject
BigipAsmMinimalObject = BigipMinimalObject
BigipIlxMinimalObject = BigipMinimalObject
BigipWomMinimalObject = BigipMinimalObject
BigipAnalyticsMinimalObject = BigipMinimalObject

# into the shared :class:`BigipMinimalObject` since the shape is
# identical.  ``kind`` carries the TMSH module + sub-type
# (e.g. ``"security dos virtual"``).
BigipSecurityMinimalObject = BigipMinimalObject


@dataclass(frozen=True, slots=True)
class BigipGenericObject:
    """A generic BIG-IP stanza retained when no specialised model exists."""

    module: str  # e.g. "net", "auth", "sys"
    object_type: str  # e.g. "route-domain", "partition", "user"
    identifier: str  # e.g. "/Common/0", "admin", or "" for singleton stanzas
    header: str
    range: Range | None = None
