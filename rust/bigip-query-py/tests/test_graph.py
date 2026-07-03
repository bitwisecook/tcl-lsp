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
