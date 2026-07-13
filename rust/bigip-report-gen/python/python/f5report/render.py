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

"""MiniJinja rendering for f5report.

Kept deliberately small: the interesting work (parsing configs, projecting
objects, walking the reference graph) is done by the query engine in
:mod:`f5report.report`. This module only turns the resulting model into a single
self-contained HTML file — including the interactive elkjs topology / flow /
listener views, all embedded (no external assets, no CDN).
"""

from __future__ import annotations

import base64
import datetime as _dt
import json
from importlib import resources
from typing import Any

import minijinja

from . import _engine

# One template, one engine. The Python and Rust generators render the *same*
# MiniJinja template (`report.html.j2`) so they cannot drift: MiniJinja
# is Jinja2-compatible and ships a native Python binding backed by the same Rust
# engine the report generator embeds. See render.rs for the mirrored setup.

# The topology tab's type legend, mirroring `render.rs::topo_types`.
_TOPO_TYPES = [
    {"k": k, "label": label}
    for k, label in [
        ("vs", "Virtual"),
        ("pool", "Pool"),
        ("node", "Node"),
        ("mon", "Monitor"),
        ("rule", "iRule"),
        ("prof", "Profile"),
        ("persist", "Persist"),
        ("policy", "Policy"),
        ("snat", "SNAT"),
        ("dg", "DataGroup"),
    ]
]


def _leaf(value: Any) -> str:
    """`leaf` filter: the last `/`-separated segment of an object path.

    Mirrors the Rust `leaf` filter (render.rs) and replaces Jinja's
    `path.split('/')[-1]`, which MiniJinja does not support.
    """
    s = "" if value is None else str(value)
    return s.rsplit("/", 1)[-1]


def _build_env(template_src: str) -> minijinja.Environment:
    """A MiniJinja environment matching the Rust generator's configuration:
    HTML auto-escaping, the `leaf` filter, and lenient (chainable) undefined so a
    model missing an optional section (e.g. APM) renders empty rather than erroring.
    """
    env = minijinja.Environment(
        templates={"report": template_src},
        auto_escape_callback=lambda _name: "html",
        undefined_behavior="chainable",
    )
    env.add_filter("leaf", _leaf)
    return env


def _script_safe_json(obj: Any) -> str:
    """Serialise to JSON safe to embed inside a ``<script>`` element.

    Escapes ``<`` / ``>`` / ``&`` (and the line/para separators) so an iRule
    body containing e.g. ``</script>`` can never break out of the tag.
    """
    return (
        json.dumps(obj, separators=(",", ":"), default=str)
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
    )


def _vendor_text(name: str) -> str:
    return resources.files("f5report.vendor").joinpath(name).read_text("utf-8")


def _vendor_bytes(name: str) -> bytes:
    return resources.files("f5report.vendor").joinpath(name).read_bytes()


def _has_vendor(name: str) -> bool:
    return resources.files("f5report.vendor").joinpath(name).is_file()


def _asset_text(name: str) -> str:
    """Read a CSS/JS asset from the templates dir verbatim.

    These are emitted with ``| safe`` rather than ``{% include %}`` so MiniJinja
    never parses them — the embedded JS/CSS can contain ``{{`` / ``}}`` / ``{%``
    sequences that would otherwise collide with MiniJinja's own delimiters.
    """
    return resources.files("f5report.templates").joinpath(name).read_text("utf-8")


def render_report(
    model: dict[str, Any], *, embed_console: bool | None = None, report_id: str = ""
) -> str:
    """Render the report ``model`` to a standalone HTML document.

    ``embed_console``: ``None`` (default) embeds the in-browser WASM query console
    when its artifacts are vendored; ``False`` forces it off (a much smaller page,
    e.g. for hosting where a strict CSP would block WebAssembly instantiation).

    ``report_id``: a stable per-report id embedded as ``<html data-report-id>`` so
    the in-report architecture editor keys its localStorage per report.
    """
    model = dict(model)
    model.setdefault(
        "generated_at",
        _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
    )
    model["report_id"] = report_id
    # The full f5-query manual, embedded as a reference panel by the console.
    model["f5q_manual"] = _engine.manual()
    # The whole model is embedded as JSON so the client-side topology / flow /
    # listener views run with no server and no external assets. Computed before
    # the (large) asset strings are added so they are not duplicated into it.
    model["model_json"] = _script_safe_json(model)
    # elkjs + the orthogonal renderer, which draws every report diagram.
    model["elk_js"] = _vendor_text("elk.bundled.js")
    model["elk_graph_js"] = _asset_text("elk-graph.js")
    model["apm_css"] = _asset_text("apm.css")
    model["apm_js"] = _asset_text("apm.js")
    model["report_css"] = _asset_text("report.css")
    model["topology_css"] = _asset_text("topology.css")
    model["print_css"] = _asset_text("print.css")
    model["certs_css"] = _asset_text("certs.css")
    model["secrets_css"] = _asset_text("secrets.css")
    model["forensics_css"] = _asset_text("forensics.css")
    model["report_js"] = _asset_text("report.js")
    model["topology_js"] = _asset_text("topology.js")
    model["certs_js"] = _asset_text("certs.js")
    model["secrets_js"] = _asset_text("secrets.js")
    model["forensics_js"] = _asset_text("forensics.js")
    model["irule_flow_js"] = _asset_text("irule-flow.js")
    model["print_js"] = _asset_text("print.js")
    # Project marks, inlined as <svg> (mirrors render.rs). tcl-lsp has a light
    # and a dark variant; the report shows whichever matches the active theme.
    model["logo_f5q_svg"] = _vendor_text("logo-f5q.svg")
    model["logo_tcl_lsp_svg"] = _vendor_text("logo-tcl-lsp.svg")
    model["logo_tcl_lsp_dark_svg"] = _vendor_text("logo-tcl-lsp-dark.svg")
    # Topology tab type legend (mirrors render.rs).
    model["topo_types"] = _TOPO_TYPES

    # In-browser query console: the wasm build of the query engine, inlined.
    # Optional — a report still renders (minus the console) if the wasm artifacts
    # were not vendored (e.g. the toolchain was unavailable at build time).
    if (
        embed_console is not False
        and _has_vendor("f5query_wasm_bg.wasm")
        and _has_vendor("f5query_wasm.js")
    ):
        model["wasm_glue"] = _vendor_text("f5query_wasm.js")
        model["wasm_b64"] = base64.b64encode(
            _vendor_bytes("f5query_wasm_bg.wasm")
        ).decode("ascii")
        model["console_js"] = _asset_text("console.js")
        # The iRule Format button reuses the same wasm engine (format_irule).
        model["irule_format_js"] = _asset_text("irule-format.js")
        model["has_console"] = True
    else:
        model["has_console"] = False

    template_src = _asset_text("report.html.j2")
    return _build_env(template_src).render_template("report", **model)
