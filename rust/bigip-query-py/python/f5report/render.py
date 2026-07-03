"""Jinja rendering for f5report.

Kept deliberately small: the interesting work (parsing configs, projecting
objects, walking the reference graph) is done by the query engine in
:mod:`f5report.report`. This module only turns the resulting model into a single
self-contained HTML file.
"""

from __future__ import annotations

import datetime as _dt
import json
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


def render_report(model: dict[str, Any]) -> str:
    """Render the report ``model`` to a standalone HTML document."""
    env = _jinja_env()
    template = env.get_template("report.html.j2")
    # The whole model is also embedded as JSON so the page's client-side
    # search/filter works with no server and no external assets.
    model = dict(model)
    model.setdefault(
        "generated_at",
        _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
    )
    model["model_json"] = json.dumps(model, separators=(",", ":"), default=str)
    return template.render(**model)
