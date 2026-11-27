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

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import split_baseline as tool


def test_fixture_workspaces_have_stable_sizes_and_real_source_edges(
    tmp_path: Path,
) -> None:
    roots = tool.stage_workspaces(tmp_path)
    assert set(roots) == {"small", "large", "source"}
    assert len((roots["small"] / "main.tcl").read_text().splitlines()) == 6
    assert len((roots["large"] / "main.tcl").read_text().splitlines()) == 10_000
    assert "source [file join" in (roots["source"] / "main.tcl").read_text()
    assert "proc target" in (roots["source"] / "lib.tcl").read_text()


def test_percentile_is_nearest_rank_and_empty_data_is_not_zero() -> None:
    assert tool.distribution(list(range(1, 21)))["p95_ms"] == 19
    with pytest.raises(ValueError):
        tool.distribution([])
    with pytest.raises(ValueError):
        tool.distribution([float("nan")])


def test_provisional_and_missing_answers_never_become_fast_samples() -> None:
    rows = tool.Measurements()
    rows.observe("point", lambda: {"data": {"kind": "provisional"}}, bool, 2)
    assert rows.rows["point"]["status"] == "unavailable"
    assert rows.rows["point"]["accepted"] == 0
    assert "p95_ms" not in rows.rows["point"]
    rows.observe("missing", lambda: None, bool)
    assert rows.rows["missing"]["accepted"] == 0


def test_valid_samples_keep_payload_counts_and_invalid_samples_are_explicit() -> None:
    rows = tool.Measurements()
    answers = iter([[{"uri": "file:///a"}], None])
    rows.observe("point", lambda: next(answers), bool, 2)
    row = rows.rows["point"]
    assert row["status"] == "partial"
    assert row["accepted"] == 1
    assert row["result_counts"] == [1]
    assert len(row["excluded"]) == 1


def test_artefacts_are_measured_or_explicitly_unavailable(tmp_path: Path) -> None:
    server = tmp_path / "server"
    server.write_bytes(b"server")
    rows = tool.Measurements()
    tool.artefacts(rows, server, [])
    assert rows.rows["size.server.native.bytes"]["value"] == 6
    for key in tool.ARTEFACTS[1:]:
        assert rows.rows[f"size.{key}.bytes"]["status"] == "unavailable"
    assert rows.rows["summaries.per_10000_lines_ms"]["status"] == "unavailable"
    with pytest.raises(ValueError):
        tool.artefacts(rows, server, ["unknown=/tmp/file"])
    report = tool.markdown(
        {"revision": "fixture", "dialect": "tcl8.6", "rows": rows.rows}
    )
    assert "unavailable" in report
    assert json.dumps(rows.rows, allow_nan=False)


def test_position_selects_the_call_not_the_declaration() -> None:
    for files in tool.workspace_sources().values():
        text = files["main.tcl"]
        position = tool.location_for(text)
        assert text.splitlines()[position["line"]][position["character"] :].startswith(
            "target "
        )
        assert "[target " in text.splitlines()[position["line"]]


def test_readiness_requires_the_requested_complete_document_version(
    monkeypatch,
) -> None:
    uri = "file:///fixture.tcl"
    replies = iter(
        [
            [{"params": {"uri": uri, "version": 1}}],
            [{"params": {"uri": uri, "version": 2, "provisional": True}}],
            [{"params": {"uri": uri, "version": 2}}],
        ]
    )
    observed = []

    class Client:
        def collect_notifications(self, method, timeout):
            observed.append(method)
            return next(replies)

    monkeypatch.setattr(tool, "DialectClient", lambda *args: Client())
    monkeypatch.setattr(tool, "semantic_tokens", lambda *args: None)
    monkeypatch.setattr(tool.time, "sleep", lambda *args: None)
    session = tool.Session(Path("/server"), Path("/workspace"), "tcl8.6", 1)
    session.ready(uri, 2)
    assert observed == ["textDocument/publishDiagnostics"] * 3


def test_tcllib_is_never_claimed_when_source_is_missing() -> None:
    from argparse import Namespace

    rows = tool.Measurements()
    tool.run_tcllib(Namespace(tcllib=None), rows)
    assert rows.rows["tcllib.end_to_end_ms"]["status"] == "unavailable"


def test_unreached_workspace_rows_are_explicitly_unavailable() -> None:
    rows = tool.Measurements().rows
    for name in ("small", "large", "source"):
        for metric in tool.WORKSPACE_METRICS:
            row = rows[f"workspace.{name}.{metric}"]
            assert row["status"] == "unavailable"
            assert "p95_ms" not in row


def test_a_failed_scan_cannot_supply_editor_latency_samples(
    tmp_path, monkeypatch
) -> None:
    from types import SimpleNamespace

    class Session:
        def __init__(self, *args):
            pass

        def start(self):
            return True

        def scan(self):
            raise TimeoutError("scan incomplete")

        def memory(self):
            return 1

        def open(self, *args):
            pytest.fail("a document must not be queried before scan readiness")

        def close(self):
            pass

    monkeypatch.setattr(tool, "Session", Session)
    args = SimpleNamespace(
        server=tmp_path / "server",
        dialect="tcl8.6",
        timeout=1,
        scan_timeout=2,
        samples=2,
    )
    rows = tool.Measurements()
    tool.run_workspace(args, rows, "small", tmp_path)
    assert rows.rows["workspace.small.cold_ms"]["status"] == "unavailable"
    for metric in (
        "point_warm_ms",
        "collection_ms",
        "transform_ms",
        "point_after_edit_ms",
    ):
        assert rows.rows[f"workspace.small.{metric}"]["status"] == "unavailable"
        assert "p95_ms" not in rows.rows[f"workspace.small.{metric}"]


def test_hard_timeout_stops_repeating_a_stalled_operation() -> None:
    attempts = []

    def stalled():
        attempts.append(True)
        raise TimeoutError("no complete response")

    rows = tool.Measurements()
    rows.observe("point", stalled, bool, 20)
    row = rows.rows["point"]
    assert len(attempts) == row["attempts"] == 1
    assert row["requested"] == 20
    assert row["accepted"] == 0
    assert "p95_ms" not in row


def write_case(root, case, revision="fixture", **changes):
    part = {
        "schema": tool.SCHEMA,
        "case": case,
        "revision": revision,
        "profile": "release",
        "dialect": "tcl8.6",
        "host": {"case": case},
        "configuration": {"samples": 20},
        "notes": [],
        "rows": tool.Measurements().rows,
    }
    part.update(changes)
    target = root / case
    target.mkdir()
    (target / "baseline.json").write_text(json.dumps(part))


def test_merge_preserves_case_ownership_and_marks_missing_cases(tmp_path):
    write_case(
        tmp_path,
        "small",
        rows={"workspace.small.cold_ms": {"status": "measured", "p95_ms": 7}},
    )
    write_case(tmp_path, "large")
    result = tool.merge_results(tmp_path, "fixture")
    assert result["rows"]["workspace.small.cold_ms"]["p95_ms"] == 7
    assert result["missing_cases"] == ["engines", "source", "tcllib"]
    assert result["hosts"] == {"small": {"case": "small"}, "large": {"case": "large"}}
    assert result["rows"]["workspace.source.cold_ms"]["status"] == "unavailable"
    assert result["rows"]["size.server.native.bytes"]["status"] == "unavailable"


@pytest.mark.parametrize(
    "changes",
    [{"revision": "other"}, {"configuration": {"samples": 1}}, {"profile": "debug"}],
)
def test_merge_rejects_incomparable_runs(tmp_path, changes):
    write_case(tmp_path, "small")
    write_case(tmp_path, "large", **changes)
    with pytest.raises(
        ValueError, match="different revisions or benchmark configurations"
    ):
        tool.merge_results(tmp_path, "fixture")
