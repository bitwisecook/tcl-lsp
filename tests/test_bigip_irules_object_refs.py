"""Unit tests for the declarative iRule object-reference table."""

from __future__ import annotations

from core.bigip.irules_object_refs import (
    DATA_GROUP_KINDS,
    IFILE_KINDS,
    PERSISTENCE_KINDS,
    SSL_PROFILE_KINDS,
    graph_commands_without_position_specs,
    kinds_for_placeholder,
    placeholders_for_command,
    pool_kinds_for_module,
    resolve_object_ref_args,
)


def _resolve(cmd: str, args: list[str], *, rule_module: str | None = "ltm"):
    return resolve_object_ref_args(cmd, args, rule_module=rule_module)


def test_pool_arg0_resolves_to_ltm_pool() -> None:
    assert _resolve("pool", ["/Common/p1"]) == [(0, ("ltm_pool",))]


def test_pool_member_subcommand_is_skipped() -> None:
    assert _resolve("pool", ["member", "/Common/p1"]) == []
    assert _resolve("pool", ["members"]) == []


def test_ssl_profile_resolves_both_clientssl_and_serverssl() -> None:
    assert _resolve("SSL::profile", ["/Common/cs1"]) == [(0, SSL_PROFILE_KINDS)]


def test_snat_pool_keyword_form() -> None:
    assert _resolve("snat", ["pool", "/Common/sp1"]) == [(1, ("ltm_snatpool",))]


def test_use_keyword_dispatch() -> None:
    assert _resolve("use", ["pool", "/Common/p1"]) == [(1, ("ltm_pool",))]
    assert _resolve("use", ["snatpool", "/Common/sp1"]) == [(1, ("ltm_snatpool",))]
    assert _resolve("use", ["virtual", "/Common/vs1"]) == [(1, ("ltm_virtual",))]
    assert _resolve("use", ["rateclass", "/Common/rc1"]) == [(1, ("net_rate_shaping_class",))]


def test_lb_status_extracts_pool_from_keyword_position() -> None:
    args = ["pool", "/Common/p1", "member", "1.2.3.4", "80"]
    assert _resolve("LB::status", args) == [(1, ("ltm_pool",))]


def test_findclass_resolves_data_group_at_index_1() -> None:
    assert _resolve("findclass", ["/admin", "/Common/dg1"]) == [(1, DATA_GROUP_KINDS)]


def test_class_match_skips_options_and_resolves_after_operator() -> None:
    args = ["match", "-all", "--", "[HTTP::host]", "equals", "/Common/dg1"]
    assert _resolve("class", args) == [(5, DATA_GROUP_KINDS)]


def test_class_lookup_skips_optional_separator() -> None:
    assert _resolve("class", ["lookup", "[HTTP::host]", "/Common/dg1"]) == [(2, DATA_GROUP_KINDS)]
    assert _resolve("class", ["lookup", "--", "k", "/Common/dg1"]) == [(3, DATA_GROUP_KINDS)]


def test_class_singleton_subcommands() -> None:
    for sub in ("exists", "size", "type", "get", "startsearch", "names", "element"):
        assert _resolve("class", [sub, "/Common/dg1"]) == [(1, DATA_GROUP_KINDS)], sub


def test_persist_full_path_form_resolves_persistence_profile() -> None:
    assert _resolve("persist", ["/Common/persist_profile"]) == [(0, PERSISTENCE_KINDS)]


def test_persist_type_keyword_form_does_not_resolve() -> None:
    # `persist cookie SESSION_KEY` — the second arg is an opaque session key
    # (not a profile path), so we must not emit a reference.
    assert _resolve("persist", ["cookie", "mykey"]) == []
    assert _resolve("persist", ["source_addr", "1.2.3.4"]) == []


def test_ifile_subcommands() -> None:
    for sub in ("get", "attributes", "size", "last_updated_by", "revision", "last_update_time"):
        assert _resolve("ifile", [sub, "/Common/x.html"]) == [(1, IFILE_KINDS)], sub
    # `ifile listall` takes no name and must not emit a reference.
    assert _resolve("ifile", ["listall"]) == []


def test_http_respond_ifile_keyword_and_flag_forms() -> None:
    assert _resolve("HTTP::respond", ["200", "ifile", "/Common/x.html"]) == [(2, IFILE_KINDS)]
    assert _resolve("HTTP::respond", ["200", "-ifile", "/Common/x.html"]) == [(2, IFILE_KINDS)]


def test_lsn_pool_and_subcommands() -> None:
    assert _resolve("LSN::pool", ["/Common/lsn1"]) == [(0, ("ltm_lsn_pool",))]
    assert _resolve("LSN::inbound-entry", ["create", "/Common/lsn1", "60"]) == [
        (1, ("ltm_lsn_pool",))
    ]
    assert _resolve("LSN::persistence-entry", ["create", "/Common/lsn1"]) == [
        (1, ("ltm_lsn_pool",))
    ]


def test_stats_commands_all_use_first_arg_as_profile_name() -> None:
    for cmd in ("STATS::get", "STATS::incr", "STATS::set", "STATS::setmax", "STATS::setmin"):
        assert _resolve(cmd, ["/Common/sp", "field"]) == [(0, ("ltm_profile_statistics",))]


def test_flowtable_count_resolves_both_virtual_and_route_domain() -> None:
    assert _resolve("FLOWTABLE::count", ["virtual", "/Common/vs1"]) == [(1, ("ltm_virtual",))]
    assert _resolve("FLOWTABLE::count", ["route_domain", "/Common/rd1"]) == [
        (1, ("net_route_domain",))
    ]


def test_genericmessage_route_emits_one_ref_per_keyword() -> None:
    args = ["virtual", "/Common/vs1", "config", "/Common/transport1", "pool", "/Common/p1"]
    refs = _resolve("GENERICMESSAGE::route", args)
    arg_indices = {idx for idx, _kinds in refs}
    assert arg_indices == {1, 3, 5}


def test_aaa_auth_send_resolves_virtual_at_index_0() -> None:
    assert _resolve("AAA::auth_send", ["/Common/aaa_vs", "username"]) == [(0, ("ltm_virtual",))]


def test_active_members_handles_optional_list_flag() -> None:
    assert _resolve("active_members", ["/Common/p1"]) == [(0, ("ltm_pool",))]
    # `-list` is filtered as a falsey token; the resolver picks the last
    # non-flag arg, which here is the pool name.
    assert _resolve("active_members", ["-list", "/Common/p1"]) == [(1, ("ltm_pool",))]


def test_gtm_pool_kinds_replace_ltm_pool_for_gtm_rules() -> None:
    refs = _resolve("pool", ["/Common/p1"], rule_module="gtm")
    assert refs and refs[0][1] == pool_kinds_for_module("gtm")
    assert "gtm_pool_a" in refs[0][1]


def test_when_module_filter_blocks_ltm_only_specs_in_gtm_rules() -> None:
    # SSL::profile is LTM-only; GTM rules should not pick it up.
    assert _resolve("SSL::profile", ["/Common/cs1"], rule_module="gtm") == []


def test_unknown_command_returns_empty() -> None:
    assert _resolve("totally::made::up", ["/Common/x"]) == []


def test_empty_args_returns_empty() -> None:
    assert _resolve("pool", []) == []


# ── Graph-driven kind expansions ────────────────────────────────────────


def test_data_group_kinds_pulls_in_sys_file_data_group_from_graph() -> None:
    """The dependency graph adds ``sys_file_data_group`` (file-backed dg)."""
    assert "sys_file_data_group" in DATA_GROUP_KINDS
    assert "ltm_data_group_internal" in DATA_GROUP_KINDS
    assert "ltm_data_group_external" in DATA_GROUP_KINDS


def test_ifile_kinds_includes_sys_file_ifile_from_graph() -> None:
    """The dependency graph adds ``sys_file_ifile`` alongside ``ltm_ifile``."""
    assert "sys_file_ifile" in IFILE_KINDS
    assert "ltm_ifile" in IFILE_KINDS


def test_persistence_kinds_match_graph_profile_persist() -> None:
    """``PROFILE_PERSIST`` graph kinds back the ``persist`` resolver."""
    graph_kinds = set(kinds_for_placeholder("PROFILE_PERSIST"))
    assert graph_kinds.issubset(PERSISTENCE_KINDS)


def test_placeholders_for_command_round_trips_canonical_names() -> None:
    """The graph's ``AAA::acct::send`` form is normalised to ``aaa::acct_send``."""
    assert "VIRTUAL_SERVER" in placeholders_for_command("AAA::acct_send")
    assert "VIRTUAL_SERVER" in placeholders_for_command("AAA::auth_send")
    assert "LSN_POOL" in placeholders_for_command("XLAT::src_endpoint_reservation")
    assert "NET_RESOLVER_NAME" in placeholders_for_command("RESOLVER::name_lookup")


def test_graph_commands_without_position_specs_is_empty() -> None:
    """Every graph-listed command must have a position spec or be intentionally skipped.

    A non-empty result means the dependency graph references an iRule
    command whose argument position the table has not declared.  Either
    add an :class:`ObjectRefSpec` for it or list it in
    ``_GRAPH_COMMANDS_INTENTIONALLY_UNHANDLED`` with a comment.
    """
    assert graph_commands_without_position_specs() == ()


def test_resolver_name_lookup_resolves_to_net_dns_resolver() -> None:
    refs = _resolve("RESOLVER::name_lookup", ["/Common/dns1", "example.com", "A"])
    assert refs == [(0, ("net_dns_resolver",))]


def test_xlat_src_endpoint_reservation_resolves_via_pool_flag() -> None:
    args = ["create", "-pool", "/Common/lsn1", "-extra", "stuff"]
    refs = _resolve("XLAT::src_endpoint_reservation", args)
    assert refs == [(2, ("ltm_lsn_pool",))]
