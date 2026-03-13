"""Flask web application for the Tcl Compiler Explorer."""

from __future__ import annotations

from pathlib import Path

from flask import Flask, jsonify, request, send_from_directory

from explorer.pipeline import run_pipeline
from explorer.serialise import serialise_result

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
