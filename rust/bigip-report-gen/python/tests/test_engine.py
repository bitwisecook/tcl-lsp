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

"""Tests for the native query-engine binding (`f5report._engine`)."""

from __future__ import annotations

import pathlib

import f5report
import pytest
from f5report import _engine

DATA = pathlib.Path(__file__).parent / "data"
UCS1 = str(DATA / "lab-device-01.ucs")
CONF1 = str(DATA / "device-01.bigip.conf")


def test_version_exposed():
    assert isinstance(_engine.__version__, str)
    assert f5report.engine_version() == _engine.__version__


def test_load_plain_conf():
    sources = f5report.load_paths([CONF1])
    assert len(sources) == 1
    uri, text = sources[0]
    assert uri == CONF1
    assert "ltm virtual" in text


def test_load_ucs_extracts_scf():
    sources = f5report.load_paths([UCS1])
    uri, text = sources[0]
    assert uri == UCS1
    # UCS is gzip-tar of /config; the reader flattens it to SCF text.
    assert "ltm virtual" in text
    assert "bigip-lab-01" in text  # hostname from the base-config member


def test_query_returns_native_objects():
    sources = f5report.load_paths([UCS1])
    rows = f5report.query(".ltm.virtual[] | {name:.name, dest:.destination}", sources)
    assert isinstance(rows, list)
    assert rows and isinstance(rows[0], dict)
    assert set(rows[0]) == {"name", "dest"}
    assert isinstance(rows[0]["name"], str)


def test_query_objectref_shape():
    sources = f5report.load_paths([UCS1])
    rows = f5report.query(".ltm.pool[]", sources)
    obj = rows[0]
    assert obj["kind"] == "ltm pool"
    assert obj["full-path"].startswith("/")
    assert isinstance(obj["fields"], dict)


def test_referenced_by_graph():
    sources = f5report.load_paths([UCS1])
    # Every pool -> the objects that reference it (the engine's graph builtin).
    rows = f5report.query(".ltm.pool[] | {n:.name, by:referenced_by(.)}", sources)
    by_name = {r["n"]: r["by"] for r in rows}
    # app1_t80_pool is used by the 443 virtual in this config.
    assert any("app1_t443_vs" in v for v in by_name.get("app1_t80_pool", []))


def test_per_file_grouping():
    sources = f5report.load_paths([UCS1])
    grouped = f5report.query(".ltm.node[]", sources, per_file=True)
    assert isinstance(grouped, list)
    uri, values = grouped[0]
    assert uri == UCS1
    assert isinstance(values, list)


def test_merge_mode_crosses_files():
    # Two small, non-colliding configs merged into one namespace: `.ltm.pool[]`
    # then enumerates pools from both sources together.
    a = (
        "a.conf",
        "ltm pool /Common/pool_a {\n    members { /Common/n1:80 { address 10.0.0.1 } }\n}\n",
    )
    b = (
        "b.conf",
        "ltm pool /Common/pool_b {\n    members { /Common/n2:80 { address 10.0.0.2 } }\n}\n",
    )
    names = f5report.query(".ltm.pool[] | .name", [a, b], merge=True)
    assert sorted(names) == ["pool_a", "pool_b"]


def test_merge_refuses_colliding_identities():
    # Two configs defining the same object are refused (a safety property of the
    # engine, surfaced verbatim as a QueryError).
    a = ("a.conf", "ltm pool /Common/dup { }\n")
    b = ("b.conf", "ltm pool /Common/dup { }\n")
    with pytest.raises(_engine.QueryError):
        f5report.query(".ltm.pool[]", [a, b], merge=True)


def test_bad_query_raises_queryerror():
    sources = f5report.load_paths([UCS1])
    with pytest.raises(_engine.QueryError):
        f5report.query(".ltm.virtual[", sources)


def test_mutating_query_rejected():
    sources = f5report.load_paths([UCS1])
    with pytest.raises(_engine.QueryError):
        f5report.query('.ltm.pool[] | .name = "x"', sources)


def test_sources_accept_dict():
    sources = dict(f5report.load_paths([CONF1]))
    rows = f5report.query("[.ltm.pool[]] | length", sources)
    assert rows[0] > 0
