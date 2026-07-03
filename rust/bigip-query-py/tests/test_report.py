"""Tests for the report model + HTML rendering."""
from __future__ import annotations

import pathlib

import f5report
from f5report.report import build_report, collect_model

DATA = pathlib.Path(__file__).parent / "data"
UCS1 = str(DATA / "lab-device-01.ucs")
UCS2 = str(DATA / "lab-device-02.ucs")


def _model():
    return collect_model(f5report.load_paths([UCS1, UCS2]), title="Test Estate")


def test_model_shape():
    m = _model()
    assert m["title"] == "Test Estate"
    assert len(m["devices"]) == 2
    names = {d["name"] for d in m["devices"]}
    assert names == {"bigip-lab-01.example.net", "bigip-edge-02.example.net"}


def test_device_has_all_sections():
    d = _model()["devices"][0]
    for key in ("virtuals", "pools", "nodes", "monitors", "rules", "dataGroups", "profiles"):
        assert key in d and isinstance(d[key], list)
    assert d["counts"]["virtuals"] == len(d["virtuals"])


def test_pool_members_and_usedby():
    d = _model()["devices"][0]
    pool = next(p for p in d["pools"] if p["name"] == "app1_t80_pool")
    assert pool["memberCount"] == len(pool["members"]) >= 1
    assert pool["members"][0]["address"]
    assert any("app1_t443_vs" in u for u in pool["usedBy"])


def test_orphan_detection():
    d = _model()["devices"][0]
    # Orphans come straight from the engine's referenced_by graph.
    assert isinstance(d["orphans"]["pools"], list)
    all_orphans = d["counts"]["orphans"]
    assert all_orphans == sum(len(v) for v in d["orphans"].values())


def test_irule_events_extracted():
    d = _model()["devices"][0]
    assert any(r["events"] for r in d["rules"])


def test_totals_sum_devices():
    m = _model()
    assert m["totals"]["virtuals"] == sum(dv["counts"]["virtuals"] for dv in m["devices"])


def test_build_report_html_self_contained():
    html = build_report(f5report.load_paths([UCS1]), title="Solo")
    assert html.startswith("<!doctype html>")
    assert "Solo" in html
    # no unrendered Jinja (our template uses spaced delimiters like `{{ x }}`).
    # (A substring `{{` check is unusable here: the vendored Mermaid bundle
    # embeds KaTeX strings that legitimately contain `{{`.)
    assert "{{ " not in html and " }}" not in html and "{% " not in html
    # no remote asset references anywhere (fully self-contained)
    assert 'src="http' not in html and 'href="http' not in html
    assert "cdn." not in html.split("<script id=\"f5-model\"")[0]


def test_json_model_serialisable():
    import json

    m = collect_model(f5report.load_paths([UCS1]))
    json.dumps(m)  # must not raise
