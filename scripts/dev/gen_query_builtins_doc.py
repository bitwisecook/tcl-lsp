"""Generate ``docs/design/f5-query-dsl-builtins.md`` from the builtin registry.

The :mod:`core.bigip.query.builtins` registry is the single source of
truth for every builtin's signature, summary, details, and examples.
This script renders that registry as a markdown reference so the
canonical per-function documentation can't drift from what the
evaluator actually dispatches against and what
``f5 query --help-builtins`` surfaces.

Run from the repo root::

    python scripts/dev/gen_query_builtins_doc.py

A CI check (``tests/test_f5_query.py::test_generated_builtins_doc_is_up_to_date``)
re-runs the generator against the live registry and asserts the
on-disk file is byte-identical, so a change to a builtin's spec
without regenerating the doc fails the gate before review.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT))

from core.bigip.query.builtins import (  # noqa: E402
    _CATEGORY_ORDER,
    BuiltinSpec,
    list_builtins,
)

OUTPUT_PATH = REPO_ROOT / "docs" / "design" / "f5-query-dsl-builtins.md"


_HEADER = """# F5 query DSL — builtin function reference

> **Audience:** Developer / Maintainer
> **Type:** Reference

**This page is generated from the builtin registry in
[`core/bigip/query/builtins.py`](../../core/bigip/query/builtins.py).
Edit that registry, not this file.**  The generator lives at
[`scripts/dev/gen_query_builtins_doc.py`](../../scripts/dev/gen_query_builtins_doc.py);
a CI check asserts the on-disk file is up to date.

This is the **canonical per-function reference** for every builtin the
`f5 query` DSL exposes.  For grammar, value-model, edit-pipeline, and
architectural context see [`f5-query-dsl.md`](f5-query-dsl.md); for
the user-facing feature overview and worked-example KCS notes start
from
[`../kcs/features/kcs-feature-bigip-query.md`](../kcs/features/kcs-feature-bigip-query.md).

The same per-function reference is available offline through the
verb's own help action — ``f5 query --help-builtins NAME`` prints
exactly the same content for one builtin.

## Categories

"""


_CATEGORY_BLURBS: dict[str, str] = {
    "stream": (
        "Sequence-shaped operations: filter (`select`), transform (`map`), "
        "aggregate (`any` / `all` / `count` / `unique` / `sort`), and the "
        "object-introspection helpers (`keys` / `values` / `first` / `last`)."
    ),
    "string": (
        "String predicates and rewrites: substring / prefix / suffix tests, "
        "regex `match` / `sub` / `gsub`, plain `split` / `join`, casing."
    ),
    "path": (
        "BIG-IP full-path string helpers — extract the partition or basename, "
        "swap a partition prefix.  These are *string* transforms; they don't "
        "move objects.  For object renames, reach for the **rename** category."
    ),
    "rename": (
        "Cascading rename operations — `rename` for one object, "
        "`rename_partition` for every object in a partition.  Both route "
        "through the same token-bounded engine `f5 rename` uses, so "
        "references inside iRule bodies and compound values (destination "
        "addresses, pool-member identifiers) are rewritten consistently."
    ),
    "net": (
        "IP-address arithmetic and route-domain helpers.  The `ip(net, src)` "
        "rebase is the workhorse of bulk readdressing; `with_route_domain` "
        "sets / replaces / strips the `%rd` suffix."
    ),
    "graph": (
        "Forward / reverse references across the same edge model "
        "`f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` "
        "for now."
    ),
    "value": (
        "Type / identity introspection: `kind` (TMSH kind), `path` "
        "(full-path), `length`, `defined`, `type`."
    ),
}


def _anchor(name: str) -> str:
    """Return the markdown anchor for a builtin name."""
    return name.replace("_", "-")


def _render_function(spec: BuiltinSpec) -> str:
    out: list[str] = []
    out.append(f"### `{spec.name}`")
    out.append("")
    out.append(spec.summary)
    out.append("")
    out.append("**Signatures**")
    out.append("")
    for sig in spec.signatures:
        out.append(f"- `{sig}`")
    out.append("")
    if spec.details:
        out.append("**Details**")
        out.append("")
        out.append(spec.details)
        out.append("")
    if spec.examples:
        out.append("**Examples**")
        out.append("")
        out.append("```")
        for ex in spec.examples:
            out.append(ex)
        out.append("```")
        out.append("")
    return "\n".join(out)


def render(specs: list[BuiltinSpec]) -> str:
    """Render the full reference document for *specs*."""
    by_cat: dict[str, list[BuiltinSpec]] = {}
    for spec in specs:
        by_cat.setdefault(spec.category, []).append(spec)

    out = [_HEADER]

    # Top-of-page navigation table.
    for cat in _CATEGORY_ORDER:
        names = sorted(s.name for s in by_cat.get(cat, ()))
        if not names:
            continue
        nav = ", ".join(f"[`{n}`](#{_anchor(n)})" for n in names)
        blurb = _CATEGORY_BLURBS.get(cat, "")
        out.append(f"- **[{cat}](#{cat})** — {blurb}")
        out.append(f"  - {nav}")
    out.append("")

    # One section per category, one subsection per function.
    for cat in _CATEGORY_ORDER:
        cat_specs = sorted(by_cat.get(cat, ()), key=lambda s: s.name)
        if not cat_specs:
            continue
        out.append(f"## {cat}")
        out.append("")
        if cat in _CATEGORY_BLURBS:
            out.append(_CATEGORY_BLURBS[cat])
            out.append("")
        for spec in cat_specs:
            out.append(_render_function(spec))

    # Footer.  Intentionally timestamp-free so the up-to-date check is
    # deterministic — a change to a builtin must regenerate this file
    # but a no-op regeneration on a different day must not.
    out.append("---")
    out.append("")
    out.append("*Generated by `scripts/dev/gen_query_builtins_doc.py`.*")
    out.append("")
    return "\n".join(out)


def main() -> int:
    specs = list_builtins()
    text = render(specs)
    OUTPUT_PATH.write_text(text, encoding="utf-8")
    print(f"wrote {OUTPUT_PATH.relative_to(REPO_ROOT)} ({len(specs)} builtins)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
