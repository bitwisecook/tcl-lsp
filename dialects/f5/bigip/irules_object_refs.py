"""Canonical iRule command → BIG-IP object reference table.

A single source of truth for "which iRule command arguments name which
BIG-IP objects".  Consumers (the cleanup analyser, the workspace link
extractor, future security / lifetime analysers) read this table
through :func:`resolve_object_ref_args` instead of hard-coding command
lists.

The table has two layers:

- ``data/irules_object_refs_graph.json`` — a generated dependency
  graph (placeholders ↔ object kinds, plus iRule commands ↔
  placeholders).  This is the source of truth for which placeholder
  type each iRule command argument expects, and which BIG-IP object
  kinds each placeholder can resolve to.  Loaded at import time and
  filtered to kinds that exist in
  :data:`dialects.f5.bigip.registry.data.OBJECT_KIND_SPECS`.
- The per-command :class:`ObjectRefSpec` table below — declares
  argument *position* (fixed index, after-keyword, or last-arg) for
  each command.  Position is not in the graph and is encoded here.

Commands whose argument shape is too irregular for the declarative
form (``class`` and ``persist``) have custom resolvers.

Object kinds use the names from
:data:`dialects.f5.bigip.registry.data.OBJECT_KIND_SPECS` so the resolved
references slot directly into the BIG-IP object resolver in
:mod:`dialects.f5.bigip.object_registry`.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .registry.data import OBJECT_KIND_SPECS

# ---------------------------------------------------------------------------
# Generated dependency graph — placeholders → kinds, commands → placeholders.
# ---------------------------------------------------------------------------

_GRAPH_PATH = Path(__file__).with_name("data") / "irules_object_refs_graph.json"

# A few commands appear in the graph in fully-colon-separated form
# (``AAA::acct::send``) but the actual iRule command name uses a single
# colon-pair plus underscores (``AAA::acct_send``).  Map them so the
# graph data drives off the same names the parser sees.
_GRAPH_COMMAND_RENAMES: dict[str, str] = {
    "AAA::acct::send": "AAA::acct_send",
    "AAA::auth::send": "AAA::auth_send",
    "MR::equivalent::transport": "MR::equivalent_transport",
    "RESOLVER::name::lookup": "RESOLVER::name_lookup",
    "XLAT::src::endpoint::reservation": "XLAT::src_endpoint_reservation",
}


def _load_graph() -> tuple[
    dict[str, tuple[str, ...]],  # placeholder -> (kind, …) (filtered to known kinds)
    dict[str, tuple[str, ...]],  # command_lower -> (placeholder, …)
]:
    """Load the generated dependency graph and filter to kinds we know about."""
    if not _GRAPH_PATH.is_file():
        return {}, {}

    raw = json.loads(_GRAPH_PATH.read_text(encoding="utf-8"))

    known_kinds = set(OBJECT_KIND_SPECS.keys())
    ph_to_kinds: dict[str, list[str]] = {}
    for edge in raw.get("edges", ()):
        if edge.get("type") != "resolves_to":
            continue
        src = edge.get("from", "")
        dst = edge.get("to", "")
        if not src.startswith("placeholder:") or not dst.startswith("kind:"):
            continue
        ph = src[len("placeholder:") :]
        kind = dst[len("kind:") :]
        if kind in known_kinds:
            ph_to_kinds.setdefault(ph, []).append(kind)

    cmd_to_phs: dict[str, list[str]] = {}
    for edge in raw.get("edges", ()):
        if edge.get("type") != "uses_placeholder":
            continue
        src = edge.get("from", "")
        dst = edge.get("to", "")
        if not src.startswith("usage:") or not dst.startswith("placeholder:"):
            continue
        usage = src[len("usage:") :]
        # ``usage:CMD:PLACEHOLDER`` — split off the trailing placeholder name.
        if ":" not in usage:
            continue
        cmd_raw, _ph_in_id = usage.rsplit(":", 1)
        cmd = _GRAPH_COMMAND_RENAMES.get(cmd_raw, cmd_raw).lower()
        ph = dst[len("placeholder:") :]
        cmd_to_phs.setdefault(cmd, []).append(ph)

    return (
        {ph: tuple(sorted(set(kinds))) for ph, kinds in ph_to_kinds.items()},
        {cmd: tuple(sorted(set(phs))) for cmd, phs in cmd_to_phs.items()},
    )


_PLACEHOLDER_KINDS, _COMMAND_PLACEHOLDERS = _load_graph()


def kinds_for_placeholder(name: str) -> tuple[str, ...]:
    """Return BIG-IP kinds for an iRule placeholder (e.g. ``POOL_OBJ``).

    Empty tuple when the placeholder is not in the graph or when none
    of its kinds are present in :data:`OBJECT_KIND_SPECS`.
    """
    return _PLACEHOLDER_KINDS.get(name, ())


def placeholders_for_command(command: str) -> tuple[str, ...]:
    """Return the placeholders an iRule command names anywhere in its argv.

    The graph does not record argument *position* — only that the
    command takes an argument of a given placeholder type somewhere.
    Position resolution is the responsibility of :func:`resolve_object_ref_args`.
    """
    return _COMMAND_PLACEHOLDERS.get(command.lower(), ())


# The kind sets below are derived from the graph at import time; the
# fallback tuples encode the same defaults the project relied on before
# the graph existed, so analysis never silently degrades to "no kinds".


def _graph_or(default: tuple[str, ...], placeholder: str) -> tuple[str, ...]:
    """Return graph-derived kinds for *placeholder*, or *default* if the graph is empty."""
    graph_kinds = kinds_for_placeholder(placeholder)
    return graph_kinds or default


# Persistence-profile kinds — used by every command that accepts a persistence profile.
PERSISTENCE_KINDS: tuple[str, ...] = _graph_or(
    (
        "ltm_persistence_cookie",
        "ltm_persistence_dest_addr",
        "ltm_persistence_global_settings",
        "ltm_persistence_hash",
        "ltm_persistence_host",
        "ltm_persistence_msrdp",
        "ltm_persistence_persist_records",
        "ltm_persistence_sip",
        "ltm_persistence_source_addr",
        "ltm_persistence_ssl",
        "ltm_persistence_universal",
    ),
    "PROFILE_PERSIST",
)

# Data-group kinds.  The graph adds ``sys_file_data_group`` (the on-disk
# file backing an external data-group), which the legacy table missed.
DATA_GROUP_KINDS: tuple[str, ...] = _graph_or(
    (
        "ltm_data_group_internal",
        "ltm_data_group_external",
    ),
    "CLASS_OBJ",
)

# SSL profile kinds.  The graph's ``PROFILE_OBJ`` placeholder spans every
# ``ltm_profile_*`` kind in the registry (``SSL::profile`` is generic over
# the side ssl-profile context, but the wider set is still the upper
# bound on what a user could reference).  We narrow back to the
# clientssl/serverssl pair since those are the only profiles the F5
# documentation lists for the ``SSL::profile`` switch.
SSL_PROFILE_KINDS: tuple[str, ...] = (
    "ltm_profile_client_ssl",
    "ltm_profile_server_ssl",
)

# Message-routing transport-config kinds — used by GENERICMESSAGE / MR commands.
TRANSPORT_CONFIG_KINDS: tuple[str, ...] = _graph_or(
    (
        "ltm_message_routing_diameter_transport_config",
        "ltm_message_routing_generic_transport_config",
        "ltm_message_routing_mqtt_transport_config",
        "ltm_message_routing_sip_transport_config",
    ),
    "TRANSPORT_CONFIG",
)

# iFile kinds.  The graph adds ``sys_file_ifile`` (the on-disk file
# behind an ``ltm_ifile`` object).
IFILE_KINDS: tuple[str, ...] = _graph_or(("ltm_ifile",), "IFILE_OBJ")

# Net DNS resolver — for ``RESOLVER::name_lookup``.
NET_DNS_RESOLVER_KINDS: tuple[str, ...] = _graph_or(("net_dns_resolver",), "NET_RESOLVER_NAME")

# Pool kinds depend on whether the rule is attached to LTM or GTM; the
# helper :func:`pool_kinds_for_module` picks the right set.

# Tokens that should never be treated as object names when they appear
# at a reference position (sub-command keywords, modifier flags …).
_FALSEY_REF_TOKENS: frozenset[str] = frozenset(
    {
        "none",
        "add",
        "delete",
        "modify",
        "replace-all-with",
        "enabled",
        "disabled",
        "default",
        "all",
        "and",
        "or",
        "context",
        "clientside",
        "serverside",
        "true",
        "false",
        "-list",
        "--",
    }
)


@dataclass(frozen=True, slots=True)
class ObjectRefArg:
    """Where in argv the object-name argument is located.

    Exactly one of ``index`` or ``after_keyword`` must be set.  When
    ``after_keyword`` is set, the resolved argument is the token
    immediately following a (case-insensitive) match against the
    keyword anywhere in ``args``; if the keyword is not present the
    spec does not match.  When ``last`` is True, the resolved argument
    is the final element of ``args``.
    """

    index: int | None = None
    after_keyword: str | None = None
    last: bool = False


@dataclass(frozen=True, slots=True)
class ObjectRefSpec:
    """One way an iRule command argument names a BIG-IP object."""

    command: str  # canonical lowercase, e.g. "ssl::profile"
    position: ObjectRefArg
    kinds: tuple[str, ...]
    sub_command: str | None = None  # require args[0] == this (case-insensitive)
    when_module: str | None = None  # restrict to "ltm" or "gtm"
    description: str = ""


def pool_kinds_for_module(rule_module: str | None) -> tuple[str, ...]:
    """Return pool kinds for *rule_module* — GTM rules use GTM pool kinds."""
    if rule_module == "gtm":
        # GTM has per-record-type pool kinds (a / aaaa / cname / mx / naptr / srv).
        return (
            "gtm_pool_a",
            "gtm_pool_aaaa",
            "gtm_pool_cname",
            "gtm_pool_mx",
            "gtm_pool_naptr",
            "gtm_pool_srv",
        )
    return ("ltm_pool",)


# The canonical table.  Each entry describes one (command, arg-position)
# rule for resolving an object reference.  Multiple specs may target the
# same command — e.g. ``LB::reselect`` has separate entries for pool /
# virtual / snatpool / vlan / rateclass.
#
# Pool kinds are filled in dynamically by :func:`_pool_specs_for_module`
# because they depend on rule_module.  Every other kind is module-agnostic.
_BASE_SPECS: tuple[ObjectRefSpec, ...] = (
    # ── SSL profile switch ─────────────────────────────────────────────
    ObjectRefSpec(
        command="ssl::profile",
        position=ObjectRefArg(index=0),
        kinds=SSL_PROFILE_KINDS,
        when_module="ltm",
        description="SSL::profile PROFILE_OBJ",
    ),
    # ── Snat pool ──────────────────────────────────────────────────────
    ObjectRefSpec(
        command="snatpool",
        position=ObjectRefArg(index=0),
        kinds=("ltm_snatpool",),
        when_module="ltm",
        description="snatpool SNAT_POOL_OBJ",
    ),
    ObjectRefSpec(
        command="snat",
        position=ObjectRefArg(after_keyword="pool"),
        kinds=("ltm_snatpool",),
        when_module="ltm",
        description="snat pool SNAT_POOL_OBJ",
    ),
    # ── Virtual ────────────────────────────────────────────────────────
    ObjectRefSpec(
        command="virtual",
        position=ObjectRefArg(index=0),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="aaa::auth_send",
        position=ObjectRefArg(index=0),
        kinds=("ltm_virtual",),
        description="AAA::auth_send VIRTUAL_SERVER USERNAME ...",
    ),
    ObjectRefSpec(
        command="aaa::acct_send",
        position=ObjectRefArg(index=0),
        kinds=("ltm_virtual",),
        description="AAA::acct_send VIRTUAL_SERVER ...",
    ),
    # ── Node ───────────────────────────────────────────────────────────
    ObjectRefSpec(
        command="node",
        position=ObjectRefArg(index=0),
        kinds=("ltm_node",),
        when_module="ltm",
        description="node NODE_OBJ ?PORT?",
    ),
    # ── ``use`` family ─────────────────────────────────────────────────
    ObjectRefSpec(
        command="use",
        position=ObjectRefArg(after_keyword="snatpool"),
        kinds=("ltm_snatpool",),
        when_module="ltm",
        description="use snatpool SNAT_POOL_OBJ",
    ),
    ObjectRefSpec(
        command="use",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="use virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="use",
        position=ObjectRefArg(after_keyword="rateclass"),
        kinds=("net_rate_shaping_class",),
        description="use rateclass RATE_CLASS",
    ),
    # ── ``LB::reselect`` family ────────────────────────────────────────
    ObjectRefSpec(
        command="lb::reselect",
        position=ObjectRefArg(after_keyword="snatpool"),
        kinds=("ltm_snatpool",),
        when_module="ltm",
        description="LB::reselect snatpool SNAT_POOL_OBJ",
    ),
    ObjectRefSpec(
        command="lb::reselect",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="LB::reselect virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="lb::reselect",
        position=ObjectRefArg(after_keyword="rateclass"),
        kinds=("net_rate_shaping_class",),
        description="LB::reselect rateclass RATE_CLASS",
    ),
    # ── Rate class ─────────────────────────────────────────────────────
    ObjectRefSpec(
        command="rateclass",
        position=ObjectRefArg(index=0),
        kinds=("net_rate_shaping_class",),
        description="rateclass RATE_CLASS",
    ),
    # ── iFile ──────────────────────────────────────────────────────────
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="get",
        description="ifile get IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="attributes",
        description="ifile attributes IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="size",
        description="ifile size IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="last_updated_by",
        description="ifile last_updated_by IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="revision",
        description="ifile revision IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="ifile",
        position=ObjectRefArg(index=1),
        kinds=IFILE_KINDS,
        sub_command="last_update_time",
        description="ifile last_update_time IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="http::respond",
        position=ObjectRefArg(after_keyword="ifile"),
        kinds=IFILE_KINDS,
        description="HTTP::respond ... ifile IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="http::respond",
        position=ObjectRefArg(after_keyword="-ifile"),
        kinds=IFILE_KINDS,
        description="HTTP::respond ... -ifile IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="http2::push",
        position=ObjectRefArg(after_keyword="-ifile"),
        kinds=IFILE_KINDS,
        description="HTTP2::push ... -ifile IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="access::respond",
        position=ObjectRefArg(after_keyword="ifile"),
        kinds=IFILE_KINDS,
        description="ACCESS::respond ... ifile IFILE_OBJ",
    ),
    ObjectRefSpec(
        command="access::respond",
        position=ObjectRefArg(after_keyword="-ifile"),
        kinds=IFILE_KINDS,
        description="ACCESS::respond ... -ifile IFILE_OBJ",
    ),
    # ── LSN ────────────────────────────────────────────────────────────
    ObjectRefSpec(
        command="lsn::pool",
        position=ObjectRefArg(index=0),
        kinds=("ltm_lsn_pool",),
        when_module="ltm",
        description="LSN::pool LSN_POOL",
    ),
    ObjectRefSpec(
        command="lsn::inbound-entry",
        position=ObjectRefArg(index=1),
        kinds=("ltm_lsn_pool",),
        sub_command="create",
        when_module="ltm",
        description="LSN::inbound-entry create LSN_POOL TIMEOUT ...",
    ),
    ObjectRefSpec(
        command="lsn::persistence-entry",
        position=ObjectRefArg(index=1),
        kinds=("ltm_lsn_pool",),
        sub_command="create",
        when_module="ltm",
        description="LSN::persistence-entry create LSN_POOL ... ",
    ),
    # ── STATS ──────────────────────────────────────────────────────────
    ObjectRefSpec(
        command="stats::get",
        position=ObjectRefArg(index=0),
        kinds=("ltm_profile_statistics",),
        description="STATS::get PROFILE_NAME FIELD_NAME",
    ),
    ObjectRefSpec(
        command="stats::incr",
        position=ObjectRefArg(index=0),
        kinds=("ltm_profile_statistics",),
        description="STATS::incr PROFILE_NAME FIELD_NAME ?VALUE?",
    ),
    ObjectRefSpec(
        command="stats::set",
        position=ObjectRefArg(index=0),
        kinds=("ltm_profile_statistics",),
        description="STATS::set PROFILE_NAME FIELD_NAME ?VALUE?",
    ),
    ObjectRefSpec(
        command="stats::setmax",
        position=ObjectRefArg(index=0),
        kinds=("ltm_profile_statistics",),
        description="STATS::setmax PROFILE_NAME FIELD_NAME ?VALUE?",
    ),
    ObjectRefSpec(
        command="stats::setmin",
        position=ObjectRefArg(index=0),
        kinds=("ltm_profile_statistics",),
        description="STATS::setmin PROFILE_NAME FIELD_NAME ?VALUE?",
    ),
    # ── Message routing / generic message ──────────────────────────────
    ObjectRefSpec(
        command="genericmessage::route",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="GENERICMESSAGE::route ... virtual VIRTUAL_SERVER_OBJ ...",
    ),
    ObjectRefSpec(
        command="genericmessage::route",
        position=ObjectRefArg(after_keyword="config"),
        kinds=TRANSPORT_CONFIG_KINDS,
        description="GENERICMESSAGE::route ... config TRANSPORT_CONFIG ...",
    ),
    ObjectRefSpec(
        command="mr::peer",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="MR::peer ... virtual VIRTUAL_SERVER_OBJ ...",
    ),
    ObjectRefSpec(
        command="mr::peer",
        position=ObjectRefArg(after_keyword="config"),
        kinds=TRANSPORT_CONFIG_KINDS,
        description="MR::peer ... config TRANSPORT_CONFIG ...",
    ),
    ObjectRefSpec(
        command="mr::message",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="MR::message ... virtual VIRTUAL_SERVER_OBJ ...",
    ),
    ObjectRefSpec(
        command="mr::message",
        position=ObjectRefArg(after_keyword="config"),
        kinds=TRANSPORT_CONFIG_KINDS,
        description="MR::message ... config TRANSPORT_CONFIG ...",
    ),
    ObjectRefSpec(
        command="mr::prime",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="MR::prime ... virtual VIRTUAL_SERVER_OBJ ...",
    ),
    ObjectRefSpec(
        command="mr::prime",
        position=ObjectRefArg(after_keyword="config"),
        kinds=TRANSPORT_CONFIG_KINDS,
        description="MR::prime ... config TRANSPORT_CONFIG ...",
    ),
    ObjectRefSpec(
        command="mr::equivalent_transport",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="MR::equivalent_transport virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="mr::equivalent_transport",
        position=ObjectRefArg(after_keyword="config"),
        kinds=TRANSPORT_CONFIG_KINDS,
        description="MR::equivalent_transport config TRANSPORT_CONFIG",
    ),
    # ── Flow table — virtual / route_domain ────────────────────────────
    ObjectRefSpec(
        command="flowtable::count",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="FLOWTABLE::count virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="flowtable::count",
        position=ObjectRefArg(after_keyword="route_domain"),
        kinds=("net_route_domain",),
        description="FLOWTABLE::count route_domain ROUTE_DOMAIN_NAME",
    ),
    ObjectRefSpec(
        command="flowtable::limit",
        position=ObjectRefArg(after_keyword="virtual"),
        kinds=("ltm_virtual",),
        when_module="ltm",
        description="FLOWTABLE::limit virtual VIRTUAL_SERVER_OBJ",
    ),
    ObjectRefSpec(
        command="flowtable::limit",
        position=ObjectRefArg(after_keyword="route_domain"),
        kinds=("net_route_domain",),
        description="FLOWTABLE::limit route_domain ROUTE_DOMAIN_NAME",
    ),
    # ── Class (data-group) family ──────────────────────────────────────
    # `class` is handled by a custom resolver (see :func:`_resolve_class_refs`)
    # because of the variable-leading-flag positions in `match` / `search`.
    ObjectRefSpec(
        command="matchclass",
        position=ObjectRefArg(last=True),
        kinds=DATA_GROUP_KINDS,
        description="matchclass <var> <op> CLASS_OBJ",
    ),
    ObjectRefSpec(
        command="findclass",
        position=ObjectRefArg(index=1),
        kinds=DATA_GROUP_KINDS,
        description="findclass STRING CLASS_OBJ ?SEPARATOR?",
    ),
    # ── DNS / resolver ─────────────────────────────────────────────────
    ObjectRefSpec(
        command="resolver::name_lookup",
        position=ObjectRefArg(index=0),
        kinds=NET_DNS_RESOLVER_KINDS,
        description="RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE",
    ),
    # ── XLAT — translation pool reservation (LSN family) ───────────────
    # ``XLAT::src_endpoint_reservation`` accepts an LSN pool via
    # ``-pool`` flag in the create form and as a positional in the
    # update_lifetime / get forms.  Use the ``-pool`` flag form for
    # declarative resolution; missing the positional forms is acceptable
    # given how rarely they appear in production iRules and that the
    # variable-tracking scope still recovers literal `set var /Common/X`
    # bindings for them.
    ObjectRefSpec(
        command="xlat::src_endpoint_reservation",
        position=ObjectRefArg(after_keyword="-pool"),
        kinds=("ltm_lsn_pool",),
        description="XLAT::src_endpoint_reservation create -pool LSN_POOL ...",
    ),
)

# Specs whose pool kind depends on rule_module — built dynamically.
_POOL_REF_TEMPLATES: tuple[tuple[str, ObjectRefArg, str], ...] = (
    ("pool", ObjectRefArg(index=0), "pool POOL_OBJ"),
    ("active_members", ObjectRefArg(last=True), "active_members ?-list? POOL_OBJ"),
    ("active_nodes", ObjectRefArg(last=True), "active_nodes ?-list? POOL_OBJ"),
    ("members", ObjectRefArg(last=True), "members ?-list? POOL_OBJ"),
    ("nodes", ObjectRefArg(last=True), "nodes ?-list? POOL_OBJ"),
    ("clone", ObjectRefArg(after_keyword="pool"), "clone pool POOL_OBJ"),
    ("lb::reselect", ObjectRefArg(after_keyword="pool"), "LB::reselect pool POOL_OBJ"),
    ("lb::status", ObjectRefArg(after_keyword="pool"), "LB::status pool POOL_OBJ ..."),
    ("lb::down", ObjectRefArg(after_keyword="pool"), "LB::down pool POOL_OBJ ..."),
    ("lb::up", ObjectRefArg(after_keyword="pool"), "LB::up pool POOL_OBJ ..."),
    ("lb::persist", ObjectRefArg(after_keyword="cookie"), "LB::persist cookie POOL_OBJ ..."),
    # ``LB::queue`` may take a pool but not behind a fixed keyword (the
    # syntax is ``LB::queue on connlimit POOL`` / ``LB::queue limit ... POOL``)
    # so we skip declarative resolution and rely on lattice-aware variable
    # resolution to catch obvious cases.
    ("use", ObjectRefArg(after_keyword="pool"), "use pool POOL_OBJ"),
    ("hsl::open", ObjectRefArg(after_keyword="-pool"), "HSL::open ... -pool POOL_OBJ"),
    (
        "genericmessage::route",
        ObjectRefArg(after_keyword="pool"),
        "GENERICMESSAGE::route ... pool POOL_OBJ ...",
    ),
    ("mr::peer", ObjectRefArg(after_keyword="pool"), "MR::peer ... pool POOL_OBJ ..."),
    ("mr::message", ObjectRefArg(after_keyword="pool"), "MR::message ... pool POOL_OBJ ..."),
    ("mr::prime", ObjectRefArg(after_keyword="pool"), "MR::prime ... pool POOL_OBJ ..."),
)


def _materialise_specs(rule_module: str | None) -> tuple[ObjectRefSpec, ...]:
    """Return the full spec list for *rule_module*, with pool kinds filled in.

    Pool kinds are the only ones that vary by ``rule_module`` (LTM vs GTM
    have different per-record-type pool kinds), so the pool-bearing
    commands live in ``_POOL_REF_TEMPLATES`` and are inlined per call.
    """
    pool_kinds = pool_kinds_for_module(rule_module)
    materialised = list(_BASE_SPECS)
    for command, position, description in _POOL_REF_TEMPLATES:
        materialised.append(
            ObjectRefSpec(
                command=command,
                position=position,
                kinds=pool_kinds,
                description=description,
            )
        )
    return tuple(materialised)


# ---------------------------------------------------------------------------
# Custom resolvers for commands too irregular for the declarative form.
# ---------------------------------------------------------------------------


_CLASS_OPERATORS: frozenset[str] = frozenset(
    {"equals", "starts_with", "ends_with", "contains", "matches_glob", "matches_regex"}
)


def _resolve_class_refs(args: list[str]) -> list[tuple[int, tuple[str, ...]]]:
    """Return ``[(arg_index, kinds), ...]`` for the ``class`` command family.

    Handles every ``class`` subcommand that takes a data-group name:
    ``match`` / ``search`` / ``lookup`` / ``element`` / ``type`` /
    ``exists`` / ``size`` / ``names`` / ``get`` / ``startsearch``.
    """
    if not args:
        return []
    sub = args[0].lower()

    if sub in {"match", "search"}:
        idx = 1
        while idx < len(args) and args[idx].startswith("-"):
            idx += 1
        if idx < len(args) and args[idx] == "--":
            idx += 1
        idx += 1  # skip the ITEM argument
        if idx >= len(args):
            return []
        if args[idx].lower() not in _CLASS_OPERATORS:
            return []
        idx += 1  # skip the operator
        if idx < len(args):
            return [(idx, DATA_GROUP_KINDS)]
        return []

    if sub == "lookup":
        idx = 1
        if idx < len(args) and args[idx] == "--":
            idx += 1
        idx += 1  # skip the key
        if idx < len(args):
            return [(idx, DATA_GROUP_KINDS)]
        return []

    if sub in {"exists", "size", "type", "get", "startsearch", "names", "element"}:
        if len(args) >= 2:
            return [(1, DATA_GROUP_KINDS)]

    return []


_PERSIST_NON_REFERENCE_KEYWORDS: frozenset[str] = frozenset(
    {
        "add",
        "delete",
        "lookup",
        "replace",
        "none",
        "simple",
        "sticky",
        "source_addr",
        "dest_addr",
        "hash",
        "cookie",
        "ssl",
        "msrdp",
        "sip",
        "uie",
        "carp",
        "universal",
    }
)


def _resolve_persist_refs(args: list[str]) -> list[tuple[int, tuple[str, ...]]]:
    """Return refs for ``persist <profile>`` while skipping the type-keyword form.

    ``persist /Common/my_persist`` references a persistence-profile object
    by full path; ``persist cookie ...`` / ``persist source_addr ...`` /
    etc. are type-keyword forms whose subsequent args are session keys, not
    object references.  We match the first form when ``args[0]`` is a full
    path or otherwise not a persistence-type keyword.

    Some configurations use the ``persist <kind> insert <value>`` form
    with a TMSH-shaped path as the session key (``persist cookie insert
    /Common/my_persist``) — treat the path-shaped value as a
    persistence-profile reference too so cross-config audits catch it.
    """
    if not args:
        return []
    head = args[0].lower()
    if head not in _PERSIST_NON_REFERENCE_KEYWORDS or args[0].startswith("/"):
        return [(0, PERSISTENCE_KINDS)]
    # ``persist <kind> {insert|lookup} <path>`` — the third arg is the
    # session-key value, but real-world iRules sometimes use a TMSH
    # path there.  Pick up the ref when args[2] is path-shaped.
    if len(args) >= 3 and args[1].lower() in ("insert", "lookup") and args[2].startswith("/"):
        return [(2, PERSISTENCE_KINDS)]
    return []


_CUSTOM_RESOLVERS: dict[str, Callable[[list[str]], list[tuple[int, tuple[str, ...]]]]] = {
    "class": _resolve_class_refs,
    "persist": _resolve_persist_refs,
}


# ---------------------------------------------------------------------------
# Public lookup
# ---------------------------------------------------------------------------


def _arg_index_for_position(position: ObjectRefArg, args: list[str]) -> int | None:
    if position.last:
        if not args:
            return None
        return len(args) - 1
    if position.index is not None:
        if 0 <= position.index < len(args):
            return position.index
        return None
    if position.after_keyword is not None:
        keyword = position.after_keyword.lower()
        for i, arg in enumerate(args):
            if arg.lower() == keyword and i + 1 < len(args):
                return i + 1
        return None
    return None


def resolve_object_ref_args(
    command: str,
    args: list[str],
    *,
    rule_module: str | None = None,
) -> list[tuple[int, tuple[str, ...]]]:
    """Return the list of ``(arg_index, kinds)`` pairs for *command*.

    The result describes which argument positions of an iRule
    ``command args[0] args[1] ...`` invocation name BIG-IP objects.
    Multiple specs may match (e.g. ``MR::peer ... virtual VS ... pool
    POOL`` returns both).  Persistence-type keywords (``persist
    cookie``) and modifier flags (``-list``) that occupy a reference
    position are filtered out to avoid false positives.

    *rule_module* should be ``"ltm"`` or ``"gtm"`` so the pool kinds
    resolve to the appropriate side of the registry.
    """
    lower = command.lower()
    if not args:
        return []

    if lower in _CUSTOM_RESOLVERS:
        return _CUSTOM_RESOLVERS[lower](args)

    matches: list[tuple[int, tuple[str, ...]]] = []
    seen: set[int] = set()
    for spec in _materialise_specs(rule_module):
        if spec.command != lower:
            continue
        if spec.when_module is not None and rule_module not in (None, spec.when_module):
            continue
        if spec.sub_command is not None:
            if not args or args[0].lower() != spec.sub_command.lower():
                continue
        idx = _arg_index_for_position(spec.position, args)
        if idx is None or idx in seen:
            continue
        # Skip if the positional value itself is a falsey keyword (flag like "-list",
        # subcommand keyword like "none", separator like "--").
        token = args[idx].strip("{}\"'[](),").lower()
        if token in _FALSEY_REF_TOKENS:
            continue
        # ``pool`` skips ``pool member`` and ``pool members`` — those denote
        # the pool's member subcommand, not a pool-name reference.
        if lower == "pool" and idx == 0 and token in {"member", "members"}:
            continue
        seen.add(idx)
        matches.append((idx, spec.kinds))
    return matches


# Commands the dependency graph mentions but that we deliberately do not
# resolve declaratively — typically because their argument shape requires
# a custom resolver, or because they reference a kind family we do not
# track (VLAN objects have no kind in the registry).  Listed here so the
# coverage helper does not flag them as missing.
_GRAPH_COMMANDS_INTENTIONALLY_UNHANDLED: frozenset[str] = frozenset(
    {
        # `class` and `persist` ship custom resolvers above.
        "class",
        "persist",
        # VLAN-only commands — no VLAN object kind in the registry today.
        "lasthop",
        "listen",
        "nexthop",
        # Security / APM / WebSSO policies — kinds aren't in OBJECT_KIND_SPECS
        # for cleanup analysis.  Re-enable when the registry gains them.
        "access::policy",
        "antifraud::enable",
        "asm::enable",
        "bwc::color",
        "bwc::policy",
        "dosl7::enable",
        "policy::rules",
        "psc::policy",
        "websso::select",
        "profile::persist",
        # ``HSL::open`` references a ``PUBLISHER`` (sys log-config publisher)
        # whose kind isn't in OBJECT_KIND_SPECS today.
        "hsl::open",
        # ``LB::queue`` accepts a pool but at no fixed keyword position
        # (``LB::queue on connlimit POOL`` / ``LB::queue limit X POOL``).
        # Variable-tracking still recovers ``set p /Common/foo; LB::queue
        # ... $p`` cases for it.
        "lb::queue",
    }
)


def graph_commands_without_position_specs() -> tuple[str, ...]:
    """Return graph-listed commands that lack any declarative position spec.

    Returns commands that the dependency graph says reference a BIG-IP
    object but for which neither :data:`_BASE_SPECS` /
    :data:`_POOL_REF_TEMPLATES` nor :data:`_CUSTOM_RESOLVERS` nor
    :data:`_GRAPH_COMMANDS_INTENTIONALLY_UNHANDLED` declares a
    resolution rule.  A non-empty result indicates the table has fallen
    behind the graph.
    """
    declared: set[str] = set()
    for spec in _BASE_SPECS:
        declared.add(spec.command.lower())
    for command, _position, _description in _POOL_REF_TEMPLATES:
        declared.add(command.lower())
    declared.update(_CUSTOM_RESOLVERS.keys())
    declared.update(_GRAPH_COMMANDS_INTENTIONALLY_UNHANDLED)
    missing = sorted(set(_COMMAND_PLACEHOLDERS.keys()) - declared)
    return tuple(missing)


__all__ = [
    "DATA_GROUP_KINDS",
    "IFILE_KINDS",
    "NET_DNS_RESOLVER_KINDS",
    "ObjectRefArg",
    "ObjectRefSpec",
    "PERSISTENCE_KINDS",
    "SSL_PROFILE_KINDS",
    "TRANSPORT_CONFIG_KINDS",
    "graph_commands_without_position_specs",
    "kinds_for_placeholder",
    "placeholders_for_command",
    "pool_kinds_for_module",
    "resolve_object_ref_args",
]
