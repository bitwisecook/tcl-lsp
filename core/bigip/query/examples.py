"""Worked examples rendered by ``f5 query --help-examples``.

Each example is a tuple of (title, query, comment).  The same list
feeds the design doc generator and the test that asserts every example
parses cleanly — so a broken example fails CI before it ever lands in
the help text.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Example:
    title: str
    query: str
    comment: str


_EXAMPLES: tuple[Example, ...] = (
    Example(
        title="List every virtual server's default pool",
        query=".ltm.virtual[] | .pool",
        comment="Streams every VS, projects its `pool` field as a path-ref.",
    ),
    Example(
        title="Names of VSes whose name starts with ``vs_prod_``",
        query='.ltm.virtual["~^vs_prod_"] | .name',
        comment="Regex subscript filters keys; the dot-chain projects the name.",
    ),
    Example(
        title="VSes attached to a specific iRule",
        query='.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name',
        comment="`contains` works on both strings and lists.",
    ),
    Example(
        title="VSes whose pool member is in 10.0.0.0/8",
        query=(
            ".ltm.virtual[] "
            "| select(any(.pool.members[].address | map(in_cidr(., \"10.0.0.0/8\")))) "
            "| .name"
        ),
        comment=(
            "PathRefs dereference transparently: `.pool.members[]` walks "
            "VS -> pool -> members in one chain."
        ),
    ),
    Example(
        title="Readdress every VS into 192.168.9.0/24, keeping host bits",
        query='.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)',
        comment=(
            "`|=` re-binds `.` to the current destination so the rebase "
            "helper sees the value it must transform.  `f5 query` shows "
            "a unified diff by default; pass --write to apply."
        ),
    ),
    Example(
        title="Rename a pool everywhere it appears",
        query='.ltm.pool["/Common/old_pool"].name = "/Common/new_pool"',
        comment=(
            "Identity-field writes auto-route through the same engine "
            "`f5 rename` uses, so every reference (VS, iRule, data-group) "
            "moves with the pool."
        ),
    ),
    Example(
        title="Add ``/Common/log_rule`` to every VS that does not already have it",
        query=(
            '.ltm.virtual[] '
            '| select(not contains(.rules, "/Common/log_rule")) '
            '| .rules += "/Common/log_rule"'
        ),
        comment=(
            "`not` is a prefix operator on the predicate; `+=` appends a "
            "scalar (or another list) to a list field."
        ),
    ),
    Example(
        title="Find every iRule that mentions a removed pool",
        query=(
            '.ltm.rule[] '
            '| select(contains(.refs.pools, "/Common/old_pool")) '
            '| .name'
        ),
        comment="The iRule sub-tree exposes parsed ref slots without sub-parsing every command.",
    ),
    Example(
        title="Strip the ``/Common/`` partition from every default pool",
        query='.ltm.virtual[].pool |= basename(.)',
        comment="One-line projection-then-rewrite using `|=` plus the path helper.",
    ),
    Example(
        title="Count VSes grouped by partition",
        query=".ltm.virtual[].name | map(partition(.)) | sort",
        comment=(
            "Combine builtins for ad-hoc reporting.  `--raw` emits one "
            "value per line, easy to pipe through `uniq -c`."
        ),
    ),
    Example(
        title="Park every dev VS on port 0 (a common maintenance trick)",
        query=(
            '.ltm.virtual[] '
            '| select(contains(.name, "_dev_")) '
            '| .destination |= sub(., ":[0-9]+$", ":0")'
        ),
        comment=(
            "Arbitrary string rewriting via `sub` / `gsub` lands as a "
            "normal field edit.  Pair with `--in-place` to persist."
        ),
    ),
)


def list_examples() -> tuple[Example, ...]:
    return _EXAMPLES


def format_examples() -> str:
    out: list[str] = ["F5 QUERY DSL — COOKBOOK", ""]
    for index, ex in enumerate(_EXAMPLES, start=1):
        out.append(f"  {index}. {ex.title}")
        out.append(f"     $ f5 query '{ex.query}' bigip.conf")
        out.append(f"     -- {ex.comment}")
        out.append("")
    out.append("More: see docs/kcs/features/kcs-feature-bigip-query.md")
    out.append("      and docs/design/f5-query-dsl.md")
    return "\n".join(out) + "\n"
