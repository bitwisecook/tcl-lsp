"""Dialect-aware system prompt composition for Python consumers.

Reads ``ai/prompts/manifest.json`` and assembles a composite prompt from
all markdown files whose dialect set includes the requested dialect.
"""

from __future__ import annotations

import json
from pathlib import Path

_PROMPTS_DIR = Path(__file__).resolve().parent.parent / "prompts"
_MANIFEST = json.loads((_PROMPTS_DIR / "manifest.json").read_text())


def build_prompt(dialect: str) -> str:
    """Return the composite system prompt for *dialect*.

    Concatenates all prompt fragments whose manifest entry lists *dialect*,
    separated by double newlines.
    """
    parts: list[str] = []
    for entry in _MANIFEST["prompts"]:
        if dialect in entry["dialects"]:
            path = _PROMPTS_DIR / entry["file"]
            parts.append(path.read_text())
    return "\n\n".join(parts)
