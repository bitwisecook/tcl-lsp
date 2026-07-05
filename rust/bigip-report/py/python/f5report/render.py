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

"""Jinja rendering for f5report.

Kept deliberately small: the interesting work (parsing configs, projecting
objects, walking the reference graph) is done by the query engine in
:mod:`f5report.report`. This module only turns the resulting model into a single
self-contained HTML file — including the interactive Mermaid topology / flow /
listener views, all embedded (no external assets, no CDN).
"""

from __future__ import annotations

import base64
import datetime as _dt
import json
from importlib import resources
from typing import Any

import jinja2


def _jinja_env() -> jinja2.Environment:
    env = jinja2.Environment(
        loader=jinja2.PackageLoader("f5report", "templates"),
        autoescape=jinja2.select_autoescape(["html", "xml", "j2"]),
        trim_blocks=True,
        lstrip_blocks=True,
    )
    env.filters["tojson_attr"] = lambda v: json.dumps(v, separators=(",", ":"))
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

    These are emitted with ``| safe`` rather than ``{% include %}`` so Jinja
    never parses them — the topology JS uses Mermaid's ``{{"…"}}`` hexagon
    syntax, which would otherwise collide with Jinja's own delimiters.
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
    env = _jinja_env()
    template = env.get_template("report.html.j2")
    model = dict(model)
    model.setdefault(
        "generated_at",
        _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
    )
    model["report_id"] = report_id
    # The whole model is embedded as JSON so the client-side topology / flow /
    # listener views run with no server and no external assets.
    model["model_json"] = _script_safe_json(model)
    model["mermaid_js"] = _vendor_text("mermaid.min.js")
    model["report_css"] = _asset_text("report.css")
    model["topology_css"] = _asset_text("topology.css")
    model["certs_css"] = _asset_text("certs.css")
    model["secrets_css"] = _asset_text("secrets.css")
    model["forensics_css"] = _asset_text("forensics.css")
    model["report_js"] = _asset_text("report.js")
    model["topology_js"] = _asset_text("topology.js")
    model["certs_js"] = _asset_text("certs.js")
    model["secrets_js"] = _asset_text("secrets.js")
    model["forensics_js"] = _asset_text("forensics.js")
    model["irule_flow_js"] = _asset_text("irule-flow.js")

    # In-browser query console: the wasm build of the query engine, inlined.
    # Optional — a report still renders (minus the console) if the wasm artifacts
    # were not vendored (e.g. the toolchain was unavailable at build time).
    if embed_console is not False and _has_vendor("f5query_wasm_bg.wasm") and _has_vendor("f5query_wasm.js"):
        model["wasm_glue"] = _vendor_text("f5query_wasm.js")
        model["wasm_b64"] = base64.b64encode(_vendor_bytes("f5query_wasm_bg.wasm")).decode("ascii")
        model["console_js"] = _asset_text("console.js")
        model["has_console"] = True
    else:
        model["has_console"] = False
    return template.render(**model)
