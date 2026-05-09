"""Format conversion helpers for ``f5 convert``.

Currently supports:

- UCS↔SCF (delegates to :mod:`explorer.f5_remote.ucs`)
- SCF→AS3 declaration (best-effort, common LTM objects)

The AS3 emitter only covers the common LTM-side objects (virtuals,
pools, nodes, monitors, iRules); anything else is reported in the
``unmapped`` list so callers know what was skipped.
"""

from __future__ import annotations

from .as3 import scf_to_as3, AS3ConversionReport  # noqa: F401  (re-export)
