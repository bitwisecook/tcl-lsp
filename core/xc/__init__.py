"""iRule to F5 Distributed Cloud (XC) translation.

Translates F5 BIG-IP iRules into XC-native constructs: L7 routes,
service policies, header processing, and origin pool references.

Public API
----------
translate_irule(source)
    Analyse an iRule and return an :class:`XCTranslationResult`.
"""

from __future__ import annotations

from .translator import translate_irule
from .xc_model import XCTranslationResult

__all__ = ["translate_irule", "XCTranslationResult"]
