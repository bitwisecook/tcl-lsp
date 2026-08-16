# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
    for key in (
        "virtuals",
        "pools",
        "nodes",
        "monitors",
        "rules",
        "dataGroups",
        "profiles",
    ):
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


def test_virtual_profiles_in_traffic_order():
    # app1_t80_vs lists `http` then `tcp` in the config; a BIG-IP processes the
    # stack transport-first, so the report must show TCP ahead of HTTP.
    m = _model()
    vs = next(
        v for d in m["devices"] for v in d["virtuals"] if v["name"] == "app1_t80_vs"
    )
    profs = vs["profiles"]
    tcp = next(i for i, p in enumerate(profs) if p.endswith("/tcp"))
    http = next(i for i, p in enumerate(profs) if p.endswith("/http"))
    assert tcp < http, profs


def test_rule_events_in_firing_order():
    # Events are ordered by canonical firing order, not alphabetically:
    # CLIENT_ACCEPTED precedes CLIENTSSL_HANDSHAKE wherever both appear.
    for d in _model()["devices"]:
        for r in d["rules"]:
            evs = r["events"]
            if "CLIENT_ACCEPTED" in evs and "CLIENTSSL_HANDSHAKE" in evs:
                assert evs.index("CLIENT_ACCEPTED") < evs.index(
                    "CLIENTSSL_HANDSHAKE"
                ), evs


def test_relative_custom_profile_names_ordered_by_traffic():
    # Partition-relative custom profile names still resolve (by leaf) against
    # the full-path profile inventory, so transport leads application.
    scf = (
        "ltm virtual /Common/relvs {\n"
        "    destination /Common/1.2.3.4:80\n"
        "    profiles {\n        my_http { }\n        my_tcp { }\n    }\n}\n"
        "ltm profile http /Common/my_http { }\n"
        "ltm profile tcp /Common/my_tcp { }\n"
    )
    m = collect_model([("mem://x", scf)], title="T")
    vs = next(v for d in m["devices"] for v in d["virtuals"] if v["name"] == "relvs")
    assert vs["profiles"] == ["my_tcp", "my_http"], vs["profiles"]


def test_totals_sum_devices():
    m = _model()
    assert m["totals"]["virtuals"] == sum(
        dv["counts"]["virtuals"] for dv in m["devices"]
    )


def test_build_report_html_self_contained():
    html = build_report(f5report.load_paths([UCS1]), title="Solo")
    assert html.startswith("<!doctype html>")
    assert "Solo" in html
    # no unrendered Jinja (our template uses spaced delimiters like `{{ x }}`).
    # (A substring `{{` check is unusable here: the vendored Mermaid bundle
    # embeds KaTeX strings that legitimately contain `{{`.)
    assert "{{ " not in html and " }}" not in html and "{% " not in html
    # no auto-loaded remote assets (scripts/images via src=, external
    # stylesheets via <link>). Plain <a href> attribution links are fine —
    # they are user-initiated navigation, not loaded to run the report.
    assert 'src="http' not in html
    assert "<link " not in html
    assert "cdn." not in html.split('<script id="f5-model"')[0]
    # …and enforced in the browser: a CSP that forbids ALL network egress
    # (connect-src 'none'), so the config a report carries can never be phoned
    # home even if a future change or injected data tried to.
    assert "Content-Security-Policy" in html
    assert "connect-src 'none'" in html


def test_wasm_console_embedded_when_vendored():
    from importlib import resources

    have_wasm = (
        resources.files("f5report.vendor").joinpath("f5query_wasm_bg.wasm").is_file()
    )
    html = build_report(f5report.load_paths([UCS1]), title="Console")
    if have_wasm:
        assert 'id="f5-wasm"' in html  # base64 wasm blob embedded
        assert 'data-panel="console"' in html
        assert "wasm_bindgen" in html
        # the device config is embedded so the console can query it
        assert '"configText"' in html
    else:  # pragma: no cover - depends on build environment
        assert 'data-panel="console"' not in html


def test_json_model_serialisable():
    import json

    m = collect_model(f5report.load_paths([UCS1]))
    json.dumps(m)  # must not raise


def test_iapp_diagnostics_are_global_and_routed_to_apps():
    sources = [
        (
            "memory://iapp/presentation.apl",
            '#include "missing.inc"\nsection basic {\n  string addr\n  string port\n}\n',
        ),
        (
            "memory://iapp/implementation.impl",
            "set ok $::basic__addr\nset missing $::basic__missing\n",
        ),
    ]
    model = collect_model(sources, title="iApp")
    presentation = next(
        d for d in model["devices"] if d["uri"] == "memory://iapp/presentation.apl"
    )
    implementation = next(
        d for d in model["devices"] if d["uri"] == "memory://iapp/implementation.impl"
    )
    assert {d["code"] for d in presentation["configDiagnostics"]} >= {
        "IAPP7002",
        "IAPP7003",
    }
    assert {d["code"] for d in implementation["configDiagnostics"]} >= {"IAPP7001"}
    assert all(d["tab"] == "apps" for d in presentation["appDiagnostics"])
    assert presentation["iappDiagnosticEvidence"]["state"] == "complete"
