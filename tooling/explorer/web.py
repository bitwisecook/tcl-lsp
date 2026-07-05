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

"""Flask web application for the Tcl Compiler Explorer."""

from __future__ import annotations

from pathlib import Path

from flask import Flask, jsonify, request, send_from_directory

from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import run_pipeline

app = Flask(__name__, static_folder=None)

_STATIC_DIR = Path(__file__).resolve().parent / "static"


@app.route("/")
def index():
    return send_from_directory(_STATIC_DIR, "index.html")


@app.route("/<path:filename>")
def static_files(filename):
    return send_from_directory(_STATIC_DIR, filename)


@app.route("/api/compile", methods=["POST"])
def compile_source():
    data = request.get_json(silent=True) or {}
    source = data.get("source", "")
    if not source or not source.strip():
        return jsonify({"error": "source is required"}), 400

    result = run_pipeline(source)
    return jsonify(serialise_result(result))
