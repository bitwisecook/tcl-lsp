"""Tests for the graph / listener / iRule-dynamic-action analysis layer."""
from __future__ import annotations

import pathlib

import f5report
from f5report import graph
from f5report.report import collect_model

DATA = pathlib.Path(__file__).parent / "data"
UCS1 = str(DATA / "lab-device-01.ucs")


def _device():
    return collect_model(f5report.load_paths([UCS1]))["devices"][0]


def test_graph_nodes_and_edges():
    g = _device()["graph"]
    assert len(g["nodes"]) > 0 and len(g["edges"]) > 0
    kinds = {e["kind"] for e in g["edges"]}
    assert "pool" in kinds and "member" in kinds


def test_irule_pool_edges_present():
    # app4_pool_rule references css_pool / js.io_t80_pool *inside* the iRule;
    # those must appear as pool-irule edges (the engine's .refs graph).
    g = _device()["graph"]
    irule_edges = [e for e in g["edges"] if e["kind"] == "pool-irule"]
    assert irule_edges, "expected pools referenced inside iRules"
    targets = {e["to"] for e in irule_edges}
    assert any("css_pool" in t for t in targets)


def test_member_to_node_edges():
    g = _device()["graph"]
    members = [e for e in g["edges"] if e["kind"] == "member"]
    assert members
    # every member edge must land on a real node in the graph
    node_oids = {n["oid"] for n in g["nodes"]}
    assert all(e["to"] in node_oids for e in members)


def test_listener_fields():
    d = _device()
    v = next(x for x in d["virtuals"] if x["name"] == "app3_t8443_vs")
    L = v["listener"]
    assert L["address"] == "192.168.1.51"
    assert L["port"] == 8443
    assert L["prefix"] == 32
    assert L["protocol"] == "tcp"
    assert L["routeDomain"] == 0


def test_wildcard_listener_is_slash_zero():
    d = _device()
    fwd = next((x for x in d["virtuals"] if "forwarder" in x["name"]), None)
    assert fwd is not None
    assert fwd["listener"]["prefix"] == 0
    assert fwd["listener"]["port"] == 0  # any port


def test_route_domain_parsing():
    assert graph._split_rd("10.0.0.1%2") == ("10.0.0.1", 2)
    assert graph._split_rd("10.0.0.1%3:80") == ("10.0.0.1", 3)
    assert graph._split_rd("10.0.0.1") == ("10.0.0.1", 0)


def test_mask_to_prefix():
    assert graph._mask_to_prefix("1.2.3.4", "255.255.255.0") == 24
    assert graph._mask_to_prefix("1.2.3.4", "24") == 24
    assert graph._mask_to_prefix("1.2.3.4", "") is None


def test_irule_dynamic_actions_extraction():
    body = """when HTTP_REQUEST {
        if { [HTTP::uri] starts_with "/api" } { pool api_pool }
        SSL::disable serverside
        HTTP::disable
        persist uie [HTTP::header Host]
    }"""
    acts = graph.irule_dynamic_actions(body)
    effects = {a["effect"] for a in acts}
    assert "SSL::disable" in effects
    assert "HTTP::disable" in effects
    assert "persist" in effects


def test_ipv6_dot_port_destination_split():
    from f5report.report import _split_dest
    # IPv6 uses a dot before the port; the colons belong to the address.
    assert _split_dest("/Common/2001:db8::10.443") == ("2001:db8::10", "443")
    assert _split_dest("/Common/2001:db8::10") == ("2001:db8::10", "")
    # IPv4 / names still split on the colon.
    assert _split_dest("/Common/192.168.1.21:443") == ("192.168.1.21", "443")


def test_snatpools_carry_usedby_and_orphan_correctly():
    d = _device()
    # A SNAT pool referenced by a virtual must not be counted as an orphan.
    used = {s["name"]: s.get("usedBy") for s in d["snatpools"]}
    assert "usedBy" in d["snatpools"][0]
    referenced = [n for n, u in used.items() if u]
    assert referenced, "expected at least one referenced SNAT pool"
    for name in referenced:
        assert name not in d["orphans"]["snatpools"]


def test_policies_shaped_with_rules():
    d = _device()
    assert d["policies"], "expected LTM policies"
    pol = d["policies"][0]
    assert "rules" in pol and isinstance(pol["rules"], list)
    # every rule exposes conditions/actions lists the simulator evaluates
    for r in pol["rules"]:
        assert "conditions" in r and "actions" in r


def test_pool_members_carry_addressing():
    d = _device()
    pool = next(p for p in d["pools"] if p["members"])
    m = pool["members"][0]
    assert "ratio" in m and "priorityGroup" in m and "state" in m


def test_dynamic_profiles_propagate_to_virtuals():
    d = _device()
    # app4_pool_rule issues a `node` override; the virtual attaching it lists it.
    v = next((x for x in d["virtuals"] if x.get("dynamicProfiles")), None)
    assert v is not None
    assert all("rule" in a for a in v["dynamicProfiles"])


# --- Deep orphan analysis: dynamic-attach name-pattern reconstruction --------

SCF_ORPHAN_PREFIX = """
ltm pool /Common/web_backend { members { /Common/10.0.0.9:80 { address 10.0.0.9 } } }
ltm pool /Common/db_backend { members { /Common/10.0.0.7:80 { address 10.0.0.7 } } }
ltm pool /Common/used_pool { members { /Common/10.0.0.8:80 { address 10.0.0.8 } } }
ltm rule /Common/dyn { when HTTP_REQUEST { pool web_[HTTP::host] } }
ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 pool /Common/used_pool rules { /Common/dyn } }
"""


def test_attach_reach_patterns():
    from f5report import _engine
    import json
    reach = json.loads(_engine.irule_attach_patterns(
        'when HTTP_REQUEST { set p "web_[HTTP::host]"; pool $p }'))
    assert reach["pools"][0]["glob"] == "web_*"
    assert reach["pools"][0]["prefix"] == "web_"
    assert reach["pools"][0]["unconstrained"] is False


def test_pattern_matches_helper():
    web = {"prefix": "web_", "contains": [], "suffix": "", "exact": False, "unconstrained": False}
    assert graph._pattern_matches(web, "web_backend")
    assert not graph._pattern_matches(web, "db_backend")


def test_constrained_pattern_filters_orphans_by_name():
    d = collect_model([("o.scf", SCF_ORPHAN_PREFIX)])["devices"][0]
    confirmed = d["orphans"]["pools"]
    possible = d["possibleOrphans"]["pools"]
    # db_backend can't be built by web_[HTTP::host] -> provably orphaned.
    assert "db_backend" in confirmed, (confirmed, possible)
    # web_backend could be web_<host> -> only a possible orphan.
    assert "web_backend" in possible
    assert "web_backend" not in confirmed
    # Reconstructed pattern surfaced for the report UI.
    assert any(p["glob"] == "web_*" for p in d["attachPatterns"]["pools"])
    web = next(p for p in d["pools"] if p["name"] == "web_backend")
    assert web["orphanStatus"] == "possible"
    assert web["orphanMatches"][0]["pattern"] == "web_*"
