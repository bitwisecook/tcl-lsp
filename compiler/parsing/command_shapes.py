"""Shared command text shape matchers.

``extract_single_expr_argument`` now lives canonically in
:mod:`compiler.parsing.token_scanning` (routed through the green-tree tokeniser
memo); re-exported here so existing import paths stay stable.
"""

from __future__ import annotations

from .token_scanning import extract_single_expr_argument  # noqa: F401  (re-export)
