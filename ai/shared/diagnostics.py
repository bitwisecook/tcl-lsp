"""Canonical diagnostic categorisation loaded from diagnostics.json.

Single source of truth for diagnostic code-to-category mappings, category
ordering, conversion maps, and helper patterns.  Both ``ai.claude.tcl_ai``
and ``ai.mcp.tcl_mcp_server`` import from here instead of maintaining their
own copies.
"""

from __future__ import annotations

import json
import re
from importlib.resources import files
from typing import Any

# Load canonical data (importlib.resources works inside zipapps)

_DATA: dict[str, Any] = json.loads(
    files("ai.shared").joinpath("diagnostics.json").read_text(encoding="utf-8")
)

# Category ordering (list order in JSON = canonical display order)

CATEGORY_ORDER: list[tuple[str, str]] = [(cat["key"], cat["label"]) for cat in _DATA["categories"]]

# Named code sets — built from the canonical categories

_CATEGORY_CODES: dict[str, frozenset[str]] = {
    cat["key"]: frozenset(cat["codes"]) for cat in _DATA["categories"]
}

SECURITY_CODES: frozenset[str] = _CATEGORY_CODES.get("security", frozenset())
TAINT_CODES: frozenset[str] = _CATEGORY_CODES.get("taint", frozenset())
THREAD_CODES: frozenset[str] = _CATEGORY_CODES.get("thread_safety", frozenset())
PERF_CODES: frozenset[str] = _CATEGORY_CODES.get("performance", frozenset())
STYLE_CODES: frozenset[str] = _CATEGORY_CODES.get("style", frozenset())
CONTROL_FLOW_CODES: frozenset[str] = _CATEGORY_CODES.get("control_flow", frozenset())

# Derived sets

CONVERTIBLE_CODES: frozenset[str] = frozenset(_DATA["convertible_codes"])

REVIEW_CODES: frozenset[str] = frozenset().union(
    *(_CATEGORY_CODES.get(cat, frozenset()) for cat in _DATA["review_categories"])
)

# Conversion map (diagnostic code -> human-readable description)

CONVERSION_MAP: dict[str, str] = dict(_DATA["conversion_map"])

# iRules event detection pattern

IRULES_EVENT_PATTERN: re.Pattern[str] = re.compile(_DATA["irules_event_pattern"], re.MULTILINE)

# Categorisation function

# Build a flat lookup: code -> category key
_CODE_TO_CATEGORY: dict[str, str] = {}
for _cat in _DATA["categories"]:
    for _code in _cat["codes"]:
        _CODE_TO_CATEGORY[_code] = _cat["key"]

_PREFIX_RULES: list[tuple[str, str]] = [
    (rule["prefix"], rule["category"]) for rule in _DATA["prefix_rules"]
]

_DEFAULT_CATEGORY: str = _DATA.get("default_category", "style")


def categorise(code: str) -> str:
    """Map a diagnostic code to its category key.

    Looks up the code in the explicit mapping first, then falls back to
    prefix rules, and finally returns the default category.
    """
    cat = _CODE_TO_CATEGORY.get(code)
    if cat is not None:
        return cat
    for prefix, category in _PREFIX_RULES:
        if code.startswith(prefix):
            return category
    return _DEFAULT_CATEGORY
