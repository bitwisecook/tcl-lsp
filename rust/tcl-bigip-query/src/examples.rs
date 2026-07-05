// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Worked examples rendered by `f5 query --help-examples`.
//!
//! The `_EXAMPLES` cookbook tuple plus `format_examples()`. Each entry is a
//! `(title, query, comment)` triple; the formatter lays them out as a
//! numbered list with the invocation and a one-line gloss.

/// One cookbook entry: a worked query with its title and explanation.
struct Example {
    title: &'static str,
    query: &'static str,
    comment: &'static str,
}

/// The cookbook, in display order — verbatim from `_EXAMPLES`.
const EXAMPLES: &[Example] = &[
    Example {
        title: "List every virtual server's default pool",
        query: ".ltm.virtual[] | .pool",
        comment: "Streams every VS, projects its `pool` field as a path-ref.",
    },
    Example {
        title: "Names of VSes whose name starts with ``vs_prod_``",
        query: ".ltm.virtual[\"~/vs_prod_\"] | .name",
        comment: "Regex subscript filters keys; the dot-chain projects the name.",
    },
    Example {
        title: "VSes attached to a specific iRule",
        query: ".ltm.virtual[] | select(contains(.rules, \"/Common/log_rule\")) | .name",
        comment: "`contains` works on both strings and lists.",
    },
    Example {
        title: "VSes whose pool member is in 10.0.0.0/8",
        query: ".ltm.virtual[] | select(any(.pool.members[].address | in_cidr(., \"10.0.0.0/8\"))) | .name",
        comment: "PathRefs dereference transparently: `.pool.members[]` walks VS -> pool -> members in one chain.  The pipe iterates the stream of addresses, `in_cidr` runs per item producing booleans, `any` collapses.",
    },
    Example {
        title: "Readdress every VS into 192.168.9.0/24, keeping host bits",
        query: ".ltm.virtual[] | .destination |= ip(\"192.168.9.0/24\", .)",
        comment: "`|=` re-binds `.` to the current destination so the rebase helper sees the value it must transform.  `f5 query` shows a unified diff by default; pass --write to apply.",
    },
    Example {
        title: "Rename a pool everywhere it appears",
        query: ".ltm.pool[\"/Common/old_pool\"].name = \"/Common/new_pool\"",
        comment: "Identity-field writes auto-route through the same engine `f5 rename` uses, so every reference (VS, iRule, data-group) moves with the pool.",
    },
    Example {
        title: "Add ``/Common/log_rule`` to every VS that does not already have it",
        query: ".ltm.virtual[] | select(not contains(.rules, \"/Common/log_rule\")) | .rules += \"/Common/log_rule\"",
        comment: "`not` is a prefix operator on the predicate; `+=` appends a scalar (or another list) to a list field.",
    },
    Example {
        title: "Find every iRule that mentions a removed pool",
        query: ".ltm.rule[] | select(contains(.refs.pools, \"/Common/old_pool\")) | .name",
        comment: "The iRule sub-tree exposes parsed ref slots without sub-parsing every command.",
    },
    Example {
        title: "Strip the ``/Common/`` partition from every default pool",
        query: ".ltm.virtual[].pool |= basename(.)",
        comment: "One-line projection-then-rewrite using `|=` plus the path helper.",
    },
    Example {
        title: "Rename a single object everywhere (the engine `f5 rename` uses)",
        query: "rename(\"/Common/old_pool\", \"/Common/new_pool\")",
        comment: "Same token-bounded rewrite the `f5 rename` verb runs; tolerant of zero-match (returns 0) so the CLI can surface it as a warning rather than an error.",
    },
    Example {
        title: "Migrate every object from /Tenant_A/ into /Tenant_B/",
        query: "rename_partition(\"Tenant_A\", \"Tenant_B\")",
        comment: "Token-bounded prefix rewrite — every object header and every reference (including destination addresses, pool members, and iRule body literals) moves together.  Renames the `auth partition Tenant_A` stanza too.  The visibility model refuses renames involving `/Common` (every tenant sees `/Common` one-way) so examples stay tenant→tenant.",
    },
    Example {
        title: "Move every pool out of /Common/ but leave other kinds alone",
        query: ".ltm.pool[\"~^/Common/\"] | .name |= with_partition(., \"Tenant_A\")",
        comment: "Identity-field `|=` routes through the rename engine; only the pools (and references *to* them) move, virtuals and iRules keep their partition.",
    },
    Example {
        title: "Set the route domain on every destination",
        query: ".ltm.virtual[] | .destination |= with_route_domain(., 7)",
        comment: "Route domain is part of the routable identity; `with_route_domain` sets, replaces, or strips it while preserving partition prefix and port.",
    },
    Example {
        title: "Readdress with a route domain preserved through the rebase",
        query: ".ltm.virtual[] | .destination |= ip(\"192.168.9.0/24\", .)",
        comment: "`ip(net, src)` keeps the route domain and port from `src` and only rebases the address bits — `%5` survives the subnet move.",
    },
    Example {
        title: "Count VSes grouped by partition",
        query: "[.ltm.virtual[]] | group_by(partition(.\"full-path\")) | map({partition: (.[0].\"full-path\" | partition(.)), count: length})",
        comment: "`group_by(partition(.\"full-path\"))` partitions the stream of VSes by their partition string; `map` then projects each group to `{partition, count}` using `length` to count and `.[0].\"full-path\" | partition(.)` to recover the partition label.  For the flat-list variant, ``[.ltm.virtual[].\"full-path\" | partition(.)] | unique`` yields the distinct partition names sorted.",
    },
    Example {
        title: "Park every dev VS on port 0 (a common maintenance trick)",
        query: ".ltm.virtual[] | select(contains(.name, \"_dev_\")) | .destination |= sub(., \":[0-9]+$\", \":0\")",
        comment: "Arbitrary string rewriting via `sub` / `gsub` lands as a normal field edit.  Pair with `--in-place` to persist.",
    },
    Example {
        title: "Cross-reference GTM and LTM via named sources",
        query: "$ltm.ltm.virtual[].name",
        comment: "Load several configs together (`f5 query ... gtm.conf ltm.conf`).  Every input is auto-bound under its filename stem so `$ltm` and `$gtm` work without ceremony; override with `--name N=PATH` when the stem would collide or read poorly.",
    },
    Example {
        title: "Edit one source via $name from a multi-config invocation",
        query: "$ltm.ltm.virtual[\"/Common/vs_app\"].destination = \"/Common/192.168.1.1:443\"",
        comment: "The assignment routes back to the source the named root came from, so only `ltm.conf` is modified even though `gtm.conf` was loaded alongside it.  Pair with `--in-place` to persist edits to each originating file.",
    },
    Example {
        title: "Walk references across files with --merge",
        query: ".ltm.pool[] | referenced_by(.)",
        comment: "`--merge` treats every loaded source as one namespace, so `refs` / `referenced_by` cross files (a GTM pool pointing into an LTM virtual resolves transparently).  Refuses to merge when two sources define the same (kind, full-path) — namespace or redact the inputs first.",
    },
    Example {
        title: "Pool names attached to more than one VS",
        query: "[.ltm.virtual[].pool] | dupes",
        comment: "``dupes`` is the inverse of ``unique`` — returns the values that occur **more than once** in the list, sorted.  Useful for surfacing intentional sharing (or copy-paste bugs).",
    },
    Example {
        title: "VSes sorted ascending by attached pool member count",
        query: "[.ltm.virtual[]] | sort_by(.pool.members | length) | map(.name)",
        comment: "``sort_by(body)`` orders a list by the value of *body* evaluated against each item.  Pair with ``map`` to project the field you care about.",
    },
    Example {
        title: "Largest pool by member count, in one pass",
        query: "[.ltm.pool[]] | max_by(.members | length)",
        comment: "``max_by(body)`` picks the item whose *body* value is largest under jq's cross-type ordering.  Use ``min_by`` for the opposite extreme and ``min_max(body)`` to get both in one array.",
    },
    Example {
        title: "Lowercase every string field anywhere in a VS, recursively",
        query: ".ltm.virtual.web_vs | walk(if type == \"string\" then ascii_downcase else . end)",
        comment: "``walk(body)`` is jq's recursive transform — for every value in the tree (bottom-up), it rebinds ``.`` to that value and evaluates *body*.  Pair with ``type`` and ``ascii_downcase`` / ``ascii_upcase`` for case normalisation across an entire object.",
    },
    Example {
        title: "Index VSes by name for O(1) lookup in downstream pipelines",
        query: "[.ltm.virtual[]] | INDEX(.name)",
        comment: "``INDEX`` builds an object keyed by the result of *body* evaluated against each item — here the VS name.  jq's two-arg form ``INDEX(source; key)`` uses ``;`` as the argument separator; in this DSL function arguments are comma-separated, so collect the stream first and pipe.",
    },
];

/// Render the cookbook as a single string.
#[must_use]
pub fn format_examples() -> String {
    use std::fmt::Write as _;

    let mut out = String::from("F5 QUERY DSL — COOKBOOK\n\n");
    for (index, ex) in EXAMPLES.iter().enumerate() {
        let n = index + 1;
        let _ = writeln!(out, "  {n}. {}", ex.title);
        let _ = writeln!(out, "     $ f5 query '{}' bigip.conf", ex.query);
        let _ = writeln!(out, "     -- {}", ex.comment);
        out.push('\n');
    }
    out.push_str("More: see docs/kcs/features/kcs-feature-bigip-query.md\n");
    out.push_str("      and docs/references/f5_query/dsl.md\n");
    out
}

#[cfg(test)]
mod tests {
    use super::format_examples;

    #[test]
    fn examples_are_non_empty_with_expected_header() {
        let e = format_examples();
        assert!(!e.is_empty(), "cookbook must not be empty");
        assert!(
            e.starts_with("F5 QUERY DSL — COOKBOOK"),
            "cookbook must start with the expected header, got: {:?}",
            &e[..e.len().min(40)]
        );
        assert!(e.ends_with('\n'), "cookbook must end with a newline");
    }
}
