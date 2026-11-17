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

"""The pure parts of pack_churn_stress.py."""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import pack_churn_stress as churn


def test_select_documents_keeps_small_tcl_files_sorted(tmp_path: Path) -> None:
    (tmp_path / "sub").mkdir()
    (tmp_path / "b.tcl").write_text("puts b\n")
    (tmp_path / "sub" / "a.tcl").write_text("puts a\n")
    (tmp_path / "big.tcl").write_text("x" * 100)
    (tmp_path / "notes.txt").write_text("puts c\n")
    got = churn.select_documents(tmp_path, 50)
    assert got == sorted([tmp_path / "b.tcl", tmp_path / "sub" / "a.tcl"])


@pytest.mark.parametrize(
    ("round_no", "existed", "want"),
    [
        (0, False, "write"),
        (1, True, "write"),
        (2, True, "delete"),
        (2, False, "write"),
        (5, True, "delete"),
    ],
)
def test_pack_action_deletes_one_round_in_three(
    round_no: int, existed: bool, want: str
) -> None:
    assert churn.pack_action(round_no, existed) == want


def test_pack_body_is_a_one_command_speclib() -> None:
    body = churn.pack_body(7)
    assert body.startswith("speclib churnpack 1.0 {")
    assert body.count("command ") == 1
    assert body == churn.pack_body(8)


def test_distinct_pack_bodies_differ_per_round() -> None:
    assert churn.pack_body(7, distinct=True) != churn.pack_body(8, distinct=True)


def test_change_type_follows_the_lsp_file_change_codes() -> None:
    assert churn.change_type("write", existed=False) == 1
    assert churn.change_type("write", existed=True) == 2
    assert churn.change_type("delete", existed=True) == 3


def test_parse_status_reads_a_kib_field() -> None:
    text = "Name:\ttcl-lsp-server\nVmPeak:\t  9000 kB\nVmRSS:\t  4321 kB\n"
    assert churn.parse_status_kib(text, "VmRSS") == 4321
    assert churn.parse_status_kib(text, "VmSwap") is None


def test_parse_stat_sums_user_and_system_ticks_past_a_spaced_name() -> None:
    fields = ["S"] + [str(n) for n in range(1, 50)]
    # Fields after the name start at field 3; utime and stime are fields 14 and 15.
    fields[11] = "70"
    fields[12] = "5"
    stat = "1234 (tokio rt worker) " + " ".join(fields)
    assert churn.parse_stat_cpu_ticks(stat) == 75


def test_parse_io_reads_the_counters() -> None:
    text = "rchar: 10\nwchar: 20\nsyscr: 3\n"
    assert churn.parse_io(text) == {"rchar": 10, "wchar": 20, "syscr": 3}


def test_parse_state_reads_the_state_line() -> None:
    assert churn.parse_state("Name:\tx\nState:\tS (sleeping)\n") == "S (sleeping)"


@pytest.mark.parametrize(
    ("delta", "want"),
    [
        ({"cpu": 0, "rchar": 570, "wchar": 0}, "idle-wedge"),
        ({"cpu": 63, "rchar": 0, "wchar": 0}, "busy"),
        ({"cpu": 0, "rchar": 0, "wchar": 40}, "writing"),
    ],
)
def test_classify_wedge_separates_the_idle_wedge_from_slowness(
    delta: dict, want: str
) -> None:
    assert churn.classify_wedge(delta) == want


def test_proc_sample_delta() -> None:
    before = churn.ProcSample(10, 100, 50)
    after = churn.ProcSample(15, 170, 50)
    assert before.delta(after) == {"cpu": 5, "rchar": 70, "wchar": 0}


def test_parse_args_rejects_a_zero_rss_interval() -> None:
    with pytest.raises(SystemExit):
        churn.parse_args(["--rss-every", "0"])


def test_reopen_batch_is_seeded_and_bounded(tmp_path: Path) -> None:
    docs = [tmp_path / f"{n}.tcl" for n in range(5)]
    first = churn.reopen_batch(docs, 3, churn.random.Random(7))
    assert first == churn.reopen_batch(docs, 3, churn.random.Random(7))
    assert len(set(first)) == 3
    assert set(first) <= set(docs)
    assert sorted(churn.reopen_batch(docs, 9, churn.random.Random(7))) == docs


class FakeClient:
    def __init__(self) -> None:
        self.commands: list[str] = []

    def send_request(self, method: str, params: dict, timeout: float) -> dict:
        self.commands.append(params["command"])
        return {"ran": params["command"]}


def test_probe_runs_an_asked_command_on_its_own_thread() -> None:
    client = FakeClient()
    probe = churn.Probe(client, timeout=1.0, interval=0.01)
    thread = churn.threading.Thread(target=probe.run, daemon=True)
    thread.start()
    try:
        assert probe.ask("tcl-lsp.debugMemory", 5.0) == {"ran": "tcl-lsp.debugMemory"}
    finally:
        probe.stop.set()
        thread.join(timeout=5.0)
    assert "tcl-lsp.getEffectiveConfig" in client.commands
    assert probe.answered >= 1
