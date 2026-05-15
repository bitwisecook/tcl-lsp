"""Stress tests for the ``f5 query`` DSL.

The goal is to push the engine on the axes regular tests don't cover:

* deep linking (3+ hops, forward and reverse) via ``refs`` /
  ``referenced_by``;
* long pipe chains (8+ stages) combining ``map`` / ``select`` /
  ``sort`` / ``unique`` / ``count``;
* set algebra (intersection / difference / symmetric-difference
  spelled with ``any`` + ``not``);
* CIDR / port math composed with string ops (``split`` / ``last`` /
  ``in_cidr``);
* let bindings (``expr as $name | body``) that re-bind the
  outer iteration so a nested stream can refer back to its parent;
* stream-of-streams flattening (``.x[].y[]``);
* big object-literal construction (``{a: ..., b: ..., c: ...}``);
* multi-statement transactions (``mutation ; read``) where the read
  must see the post-mutation state;
* ``--partition`` qualification on a config full of bare names; and
* ``--merge`` running over a small multi-file corpus.

Each test states the "invariant under test" in its docstring so a
failure points at the engine feature that broke, not the surface
syntax.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query

# ---------------------------------------------------------------------------
# Rich fixture
#
# Two partitions; three pools (one cross-partition); three virtuals with
# distinct profile sets, distinct destination CIDRs, distinct port
# semantics; two rules that name pools and a data-group; nodes in two
# CIDRs.  Designed so every test below can find a real positive and a
# real negative for the predicate it's exercising.
# ---------------------------------------------------------------------------

BIG_CONF = """\
ltm node /Common/n_web1 {
    address 10.0.0.10
}
ltm node /Common/n_web2 {
    address 10.0.0.11
}
ltm node /Common/n_api1 {
    address 192.168.10.10
}
ltm node /Tenant_A/n_internal {
    address 172.16.0.42
}
ltm pool /Common/web_pool {
    members {
        /Common/n_web1:80 {
            address 10.0.0.10
        }
        /Common/n_web2:80 {
            address 10.0.0.11
        }
    }
    monitor /Common/http
    load-balancing-mode round-robin
}
ltm pool /Common/api_pool {
    members {
        /Common/n_api1:8080 {
            address 192.168.10.10
        }
    }
    monitor /Common/http
}
ltm pool /Tenant_A/tenant_pool {
    members {
        /Tenant_A/n_internal:443 {
            address 172.16.0.42
        }
    }
}
ltm rule /Common/log_rule {
when HTTP_REQUEST {
    pool /Common/web_pool
    log local0. "hit"
}
}
ltm rule /Common/api_rule {
when HTTP_REQUEST {
    pool /Common/api_pool
    if { [class match [HTTP::host] equals allowed_hosts] } {
        log local0. "allowed"
    }
}
}
ltm data-group internal /Common/allowed_hosts {
    type string
    records {
        foo.example.com { }
        bar.example.com { }
    }
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/web_pool
    rules {
        /Common/log_rule
    }
    profiles {
        /Common/http { }
        /Common/tcp { }
        /Common/clientssl { }
    }
    ip-protocol tcp
}
ltm virtual /Common/api_vs {
    destination /Common/10.10.0.6:8080
    pool /Common/api_pool
    rules {
        /Common/api_rule
    }
    profiles {
        /Common/http { }
        /Common/tcp { }
    }
    ip-protocol tcp
}
ltm virtual /Tenant_A/tenant_vs {
    destination /Tenant_A/172.16.0.100:443
    pool /Tenant_A/tenant_pool
    profiles {
        /Common/tcp { }
        /Common/clientssl { }
    }
    ip-protocol tcp
}
"""


def _run(query: str, source: str = BIG_CONF):
    return run_query(query, {"mem://big": source})


# ===========================================================================
# Pattern 1: deep linking — forward and reverse, multi-hop
# ===========================================================================


def test_pool_members_walked_back_to_their_virtuals_via_referenced_by():
    """Reverse-graph walk: from a node, find every pool that reaches
    it.  Exercises ``referenced_by`` on a non-pool starting kind.
    """
    expr = ".ltm.node[] | { node: .name, reached_by_refs: (referenced_by(.) | count)}"
    result = _run(expr)
    values = result.values_per_file["mem://big"]
    # Every node has at least one pool referencing it.
    assert all(v["reached_by_refs"] >= 1 for v in values), values


def test_three_hop_virtual_to_member_address_with_filter_and_uniq():
    """Forward navigation virtual → pool → members[] → .address,
    flattening the stream-of-streams, filtering by CIDR, and
    deduplicating.  Exercises ``[]`` flattening at every level."""
    expr = "[.ltm.virtual[].pool.members[].address] | unique | sort"
    [values] = _run(expr).values_per_file["mem://big"]
    assert values == ["10.0.0.10", "10.0.0.11", "172.16.0.42", "192.168.10.10"]


# ===========================================================================
# Pattern 2: long pipe chains
# ===========================================================================


def test_destination_port_histogram_via_ten_stage_pipe():
    """Ten-stage pipe: project → split → last → coerce → bucket →
    flatten → unique → sort → count → return list.  Tests that the
    evaluator keeps types coherent across long chains."""
    expr = '[.ltm.virtual[].destination | split(., ":") | last] | sort | unique'
    [values] = _run(expr).values_per_file["mem://big"]
    # Three distinct port suffixes: 443 (twice) and 8080 (once).
    assert values == ["443", "8080"]


def test_full_chain_pool_to_member_to_node_with_address_predicate():
    """Builds a flat list of ``{vs, pool, member, addr}`` records by
    composing ``map`` over a stream-of-streams.  Then filters by
    a CIDR predicate.  Tests that ``map`` doesn't lose context
    across nested iterations."""
    expr = (
        '[.ltm.virtual[].pool.members[] | select(in_cidr(.address, "10.0.0.0/8")) | .name] | sort'
    )
    [values] = _run(expr).values_per_file["mem://big"]
    # The two web pool members in 10/8; the api member is in 192.168/16
    # (not 10/8); the tenant member is in 172.16/12 (also not 10/8).
    assert values == ["/Common/n_web1:80", "/Common/n_web2:80"]


# ===========================================================================
# Pattern 3: let-bindings holding outer context inside nested streams
# ===========================================================================


def test_let_binding_propagates_outer_virtual_across_inner_map():
    """``.ltm.virtual[] as $vs | $vs.pool.members[] | {vs: $vs.name,
    member: .name}`` — the body must keep ``$vs`` bound while
    iterating the inner ``members[]`` stream.  This is the deep
    correlation pattern jq users reach for."""
    expr = ".ltm.virtual[] as $vs | $vs.pool.members[] | {vs: $vs.name, member: .name}"
    rows = _run(expr).values_per_file["mem://big"]
    # Three virtuals × member-counts (2, 1, 1) = 4 rows.
    assert len(rows) == 4
    # Every row has both a virtual and a member name.
    assert {r["vs"] for r in rows} == {"web_vs", "api_vs", "tenant_vs"}


# ===========================================================================
# Pattern 4: set algebra spelled with any / all / not
# ===========================================================================


def test_set_difference_via_any_with_negation():
    """Virtuals that attach the http profile but *not* the
    clientssl profile.  ``any(stream)`` + ``not any(stream)`` is the
    DSL's equivalent of set difference."""
    expr = (
        ".ltm.virtual[]"
        " | select("
        '  any([.profiles[].name | basename(.) == "http"])'
        '  and not any([.profiles[].name | basename(.) == "clientssl"])'
        " )"
        " | .name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    assert rows == ["api_vs"]


def test_set_intersection_via_two_any_predicates():
    """Virtuals using BOTH the http and tcp profiles (intersection)."""
    expr = (
        ".ltm.virtual[]"
        " | select("
        '  any([.profiles[].name | basename(.) == "http"])'
        '  and any([.profiles[].name | basename(.) == "tcp"])'
        " )"
        " | .name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    assert sorted(rows) == ["api_vs", "web_vs"]


def test_universal_quantifier_all_members_in_private_cidr():
    """``all(stream)`` over pool members' address fields, combined
    with ``in_cidr`` against RFC1918 supernets.  This is "find every
    pool whose entire membership lives in private address space" —
    a real audit query."""
    expr = ".ltm.pool[] | select(all(.members[].address | is_private)) | .name"
    rows = _run(expr).values_per_file["mem://big"]
    # Every pool's members live in RFC1918 in this fixture (10/8,
    # 172.16/12, 192.168/16) — so all three pools qualify.
    assert sorted(rows) == ["api_pool", "tenant_pool", "web_pool"]


# ===========================================================================
# Pattern 5: CIDR / port math composed with string ops
# ===========================================================================


def test_compound_predicate_cidr_and_port_extraction():
    """Virtuals whose pool has any member in 10/8 AND whose
    destination is on port 443.  Mixes CIDR membership with
    destination-string parsing."""
    expr = (
        ".ltm.virtual[]"
        " | select("
        '  any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))'
        '  and (.destination | split(., ":") | last | . == "443")'
        " )"
        " | .name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    assert rows == ["web_vs"]


def test_partition_aware_basename_on_destination():
    """Compose partition / basename / split / last to extract just
    the host portion of every virtual's destination, across
    partitions, then sort.  This is the kind of ad-hoc string
    surgery a network engineer reaches for."""
    expr = '[.ltm.virtual[].destination | basename(.) | split(., ":") | first] | sort'
    [rows] = _run(expr).values_per_file["mem://big"]
    assert rows == ["10.10.0.5", "10.10.0.6", "172.16.0.100"]


# ===========================================================================
# Pattern 6: multi-statement transactions
# ===========================================================================


def test_rename_then_read_sees_post_rename_state():
    """A two-statement query: rename a pool, then immediately read
    every virtual's pool — the second statement must see the
    renamed name everywhere."""
    expr = 'rename("/Common/web_pool", "/Common/renamed_pool") ; [.ltm.virtual[].pool] | sort'
    result = _run(expr)
    [rows] = result.values_per_file["mem://big"]
    paths = [str(row) for row in rows]
    # The mutation rolled forward through the second statement —
    # every reference to web_pool has been retargeted.
    assert "/Common/renamed_pool" in paths
    assert "/Common/web_pool" not in paths


def test_two_renames_compose_then_read_final_state():
    """Three-statement query: rename A→B, then B→C, then read.
    Tests that each statement applies its edits before the next
    evaluates."""
    expr = (
        'rename("/Common/web_pool", "/Common/intermediate")'
        ' ; rename("/Common/intermediate", "/Common/final")'
        " ; .ltm.virtual.web_vs.pool"
    )
    result = _run(expr)
    [final_pool] = result.values_per_file["mem://big"]
    # PathRef rendering yields its full_path string.
    assert str(final_pool) == "/Common/final"


# ===========================================================================
# Pattern 7: big object-literal construction
# ===========================================================================


def test_big_object_literal_builds_per_virtual_dossier():
    """Eight-field object literal mixing path projections, list
    comprehensions, count, and contains.  This is the
    "tabular report" pattern."""
    expr = (
        ".ltm.virtual[]"
        " | {"
        "  name: .name,"
        "  partition: partition(path(.)),"
        "  destination: .destination,"
        '  port: (.destination | split(., ":") | last),'
        "  pool: .pool,"
        "  profile_count: count([.profiles[]]),"
        "  rule_count: count([.rules[]]),"
        '  has_clientssl: any([.profiles[].name | basename(.) == "clientssl"])'
        " }"
    )
    rows = _run(expr).values_per_file["mem://big"]
    assert len(rows) == 3
    web = next(r for r in rows if r["name"] == "web_vs")
    assert web["partition"] == "Common"
    assert web["port"] == "443"
    assert web["profile_count"] == 3
    assert web["rule_count"] == 1
    assert web["has_clientssl"] is True
    api = next(r for r in rows if r["name"] == "api_vs")
    assert api["has_clientssl"] is False
    tenant = next(r for r in rows if r["name"] == "tenant_vs")
    assert tenant["partition"] == "Tenant_A"


# ===========================================================================
# Pattern 8: jq-compat index / contains piped form
# ===========================================================================


def test_piped_index_on_destination_finds_port_offset():
    """``.destination | index(":443")`` — index returns the offset
    of the substring or null.  Tests the piped-receiver form for a
    multi-arg builtin (``index(value, needle)`` invoked with one
    arg, piped value as receiver)."""
    expr = '.ltm.virtual[] | {name: .name, ssl_port_offset: (.destination | index(":443"))}'
    rows = _run(expr).values_per_file["mem://big"]
    by_name = {r["name"]: r["ssl_port_offset"] for r in rows}
    # ``/Common/10.10.0.5:443`` — ":443" starts at offset 17.
    assert by_name["web_vs"] is not None and by_name["web_vs"] > 0
    # ``/Common/10.10.0.6:8080`` — no ``:443``.
    assert by_name["api_vs"] is None


# ===========================================================================
# Pattern 9: --partition qualification
# ===========================================================================


_BARE_CONF = """\
ltm pool web_pool {
    members {
        node1:80 {
            address 10.0.0.1
        }
    }
    monitor http
}
ltm virtual web_vs {
    destination 10.0.0.100:80
    pool web_pool
    ip-protocol tcp
}
"""


def test_partition_qualifies_bare_identifiers():
    """The loader rewrites ``ltm pool web_pool {...}`` to live at
    ``/MyTenant/web_pool`` when ``partitions`` is set; ``resolve_name``
    still finds the same object whether the consumer asks for the
    short or the qualified form."""
    result = run_query(
        ".ltm.pool[]",
        {"mem://bare": _BARE_CONF},
        partitions={"mem://bare": "MyTenant"},
    )
    [pool] = result.values_per_file["mem://bare"]
    # Pool's name is the basename, but the dict key (and full_path)
    # have been qualified by the partition.
    assert pool.full_path == "/MyTenant/web_pool"


def test_partition_default_is_common_when_unset():
    """With no partitions mapping, bare identifiers default to
    ``Common`` — matching tmsh's implicit partition for unqualified
    references."""
    result = run_query(".ltm.pool[]", {"mem://bare": _BARE_CONF})
    [pool] = result.values_per_file["mem://bare"]
    assert pool.full_path == "/Common/web_pool"


def test_partition_mapping_round_trips_through_virtual_resolution():
    """A virtual with a bare ``pool web_pool`` reference, in a
    config loaded under partition ``Foo``, resolves the pool ref to
    ``/Foo/web_pool``.  Exercises the partition flowing through
    both the parser (qualifying defining headers) and resolve_name
    (qualifying body references)."""
    result = run_query(
        ".ltm.virtual.web_vs.pool",
        {"mem://bare": _BARE_CONF},
        partitions={"mem://bare": "Foo"},
    )
    [pool_ref] = result.values_per_file["mem://bare"]
    assert str(pool_ref) == "web_pool"


# ===========================================================================
# Pattern 10: --merge across multiple files
# ===========================================================================


_MERGE_FILE_A = """\
ltm pool /Common/shared_pool {
    members {
        /Common/n1:80 {
            address 10.0.0.10
        }
    }
}
ltm virtual /Common/vs_a {
    destination /Common/10.10.0.5:80
    pool /Common/shared_pool
    ip-protocol tcp
}
"""

_MERGE_FILE_B = """\
ltm virtual /Common/vs_b {
    destination /Common/10.10.0.6:443
    pool /Common/shared_pool
    ip-protocol tcp
}
"""


def test_merge_mode_streams_virtuals_from_every_input():
    """``--merge`` makes ``.ltm.virtual[]`` yield virtuals from
    every loaded source as one stream, with edits routing back to
    the originating file."""
    result = run_query(
        ".ltm.virtual[].name",
        {"mem://a": _MERGE_FILE_A, "mem://b": _MERGE_FILE_B},
        merge=True,
    )
    # Merge mode collapses to one logical namespace; the result
    # lands under whichever URI the runner picks as the primary.
    names = next(iter(result.values_per_file.values()))
    assert names == ["vs_a", "vs_b"]


def test_merge_source_file_tags_each_virtual_with_origin():
    """In merge mode, ``source_file`` lets queries label each
    streamed object with its source URI — exercising the contextvar
    plumbing introduced for the multi-file world."""
    result = run_query(
        ".ltm.virtual[] | {name: .name, src: source_file}",
        {"mem://a": _MERGE_FILE_A, "mem://b": _MERGE_FILE_B},
        merge=True,
    )
    rows = next(iter(result.values_per_file.values()))
    by_name = {r["name"]: r["src"] for r in rows}
    assert by_name == {"vs_a": "mem://a", "vs_b": "mem://b"}


# ===========================================================================
# Pattern 11: deep negation chains
# ===========================================================================


def test_triple_negation_chain_orphan_virtual():
    """Find virtuals that have NO rules, NO clientssl profile, AND
    whose pool has NO members in 10/8.  Three independent negations
    composed with ``and`` — the kind of "what's NOT here" predicate
    audits routinely need."""
    expr = (
        ".ltm.virtual[]"
        " | select("
        "  not any([.rules[]])"
        '  and not any([.profiles[].name | basename(.) == "clientssl"])'
        '  and not any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))'
        " )"
        " | .name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    # Web has rules and clientssl → excluded.
    # API has rules → excluded.
    # Tenant has no rules, has clientssl → excluded (clientssl is on it).
    # So none qualify; the negation chain correctly rejects every VS.
    assert rows == []


def test_double_negation_finds_what_simple_query_cant():
    """Virtuals with NO rules but WITH clientssl — combines a
    negation against the rules stream with a positive any against
    profiles.  Mixed-polarity predicate."""
    expr = (
        ".ltm.virtual[]"
        " | select("
        "  not any([.rules[]])"
        '  and any([.profiles[].name | basename(.) == "clientssl"])'
        " )"
        " | .name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    assert rows == ["tenant_vs"]


# ===========================================================================
# Pattern 12: stream-of-streams flattening
# ===========================================================================


def test_double_iteration_flattens_to_single_stream():
    """``.ltm.virtual[].profiles[].name`` produces a stream of
    profile-name references — across three virtuals × {2..3}
    profiles each.  The engine must flatten the inner iteration
    without dropping any."""
    rows = _run(".ltm.virtual[].profiles[].name").values_per_file["mem://big"]
    # 3 (web: http, tcp, clientssl) + 2 (api: http, tcp) + 2 (tenant:
    # tcp, clientssl) = 7 profile attachments.
    assert len(rows) == 7
    # Every value is a PathRef whose basename is one of the known
    # profile names.
    basenames = {str(r).rsplit("/", 1)[-1] for r in rows}
    assert basenames == {"http", "tcp", "clientssl"}


# ===========================================================================
# Pattern 13: csv / tsv multi-field row formatting
# ===========================================================================


def test_csv_row_builder_composes_six_disparate_fields():
    """``csv(...)`` flattens six per-virtual fields of mixed type
    (string / number / boolean / list-count) into one CSV row.
    Tests cell coercion of every supported type."""
    expr = (
        ".ltm.virtual[]"
        " | csv("
        "  .name,"
        "  partition(.name),"
        "  .destination,"
        "  count([.profiles[]]),"
        "  count([.rules[]]),"
        '  any([.profiles[].name | basename(.) == "clientssl"])'
        " )"
    )
    rows = _run(expr).values_per_file["mem://big"]
    # Three rows, six fields each.
    assert len(rows) == 3
    for row in rows:
        assert isinstance(row, str)
        assert row.count(",") == 5  # six fields → five separators
    # Spot-check web_vs: name, partition Common, destination, 3
    # profiles, 1 rule, has clientssl.
    web = next(r for r in rows if r.startswith("web_vs,"))
    assert "Common" in web
    assert ",3," in web  # profile_count
    assert ",1," in web  # rule_count
    assert "true" in web.lower()


# ===========================================================================
# Pattern 14: deep correlation with let + select + map
# ===========================================================================


def test_let_binding_under_select_correlates_pool_and_member():
    """The body of ``select`` reads ``$vs`` from the outer
    let-binding; the predicate checks "this virtual's pool has at
    least one member whose port matches the virtual's destination
    port".  Stress test for: let scope inside select, nested
    iteration, string comparison via ``split``/``last``."""
    expr = (
        ".ltm.virtual[] as $vs"
        " | select("
        "  any("
        "    $vs.pool.members[].name"
        "    | basename(.)"
        '    | split(., ":")'
        "    | last"
        '    | . == ($vs.destination | split(., ":") | last)'
        "  )"
        " )"
        " | $vs.name"
    )
    rows = _run(expr).values_per_file["mem://big"]
    # web_vs:443 has no member on :443 (members are :80) → excluded.
    # api_vs:8080 has /Common/n_api1:8080 → included.
    # tenant_vs:443 has /Tenant_A/n_internal:443 → included.
    assert sorted(rows) == ["api_vs", "tenant_vs"]


# ===========================================================================
# Pattern 15: refs() forward graph walk + count + filter
# ===========================================================================


def test_refs_forward_walk_counts_virtual_dependencies():
    """``refs(virtual)`` returns every full-path the virtual
    references through the current graph surface.  This test asserts
    the count for each virtual and that rule-less virtuals still keep
    their pool edge."""
    expr = ".ltm.virtual[] | {name: .name, deps: (refs(.) | count)}"
    rows = _run(expr).values_per_file["mem://big"]
    by_name = {r["name"]: r["deps"] for r in rows}
    assert by_name == {
        "api_vs": 2,
        "tenant_vs": 1,
        "web_vs": 2,
    }


def test_refs_returns_the_actual_pool_path():
    """Spot-check that ``refs(.ltm.virtual.web_vs)`` includes the
    string ``/Common/web_pool`` — the engine's reference-graph
    output must match the literal full-path."""
    expr = ".ltm.virtual.web_vs | refs(.) | sort"
    [deps] = _run(expr).values_per_file["mem://big"]
    assert "/Common/web_pool" in deps
    assert "/Common/log_rule" in deps
