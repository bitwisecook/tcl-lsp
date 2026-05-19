"""Shared package suggestion ranking for LSP features and commands."""

from __future__ import annotations


def rank_package_suggestions(symbol: str, package_names: list[str], limit: int) -> list[str]:
    """Rank package names against a symbol prefix heuristic."""
    query = (symbol or "").strip()
    if not query:
        return []

    prefix = query.split("::", 1)[0].lower()
    if len(prefix) < 2:
        return []

    ranked: list[tuple[int, str]] = []
    for package_name in package_names:
        lower = package_name.lower()
        if lower == prefix:
            score = 0
        elif lower.startswith(prefix):
            score = 1
        elif prefix in lower:
            score = 2
        else:
            continue
        ranked.append((score, package_name))

    ranked.sort(key=lambda item: (item[0], item[1]))
    return [name for _score, name in ranked[:limit]]
