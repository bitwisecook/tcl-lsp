"""Shared expression evaluation helpers for the compiler pipeline."""

from __future__ import annotations

import re

DECIMAL_INT_RE = re.compile(r"[+-]?[0-9]+\Z")
