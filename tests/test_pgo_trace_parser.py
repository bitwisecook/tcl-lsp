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

"""Tests for the F5 rule-profiler occurrence-log importer."""

from __future__ import annotations

from compiler.pgo.occurrence import OccurrenceKind
from compiler.pgo.profile_data import ProfileData, merge
from compiler.pgo.trace_parser import parse_f5_log, parse_f5_occurrences

# A small capture modelled on the community-article example (the dc_1 rule).
SAMPLE_LOG = """\
1511494952932589,RP_EVENT_ENTRY,/Common/vs1,CLIENT_ACCEPTED,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,
1511494952932622,RP_RULE_ENTRY,/Common/vs1,/Common/dc_1,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,0
1511494952932630,RP_CMD_ENTRY,/Common/vs1,IP::remote_addr,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,0
1511494952932633,RP_CMD_EXIT,/Common/vs1,IP::remote_addr,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,0
1511494952932638,RP_VAR_MOD,/Common/vs1,the_one=true,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,0
1511673715911280,RP_CMD_BYTECODE,/Common/vs1,push1,18291,0x9455,10.10.10.2,41772,0,10.10.10.160,80,2
"""


def test_parses_each_occurrence_kind() -> None:
    occs = parse_f5_occurrences(SAMPLE_LOG)
    kinds = [o.kind for o in occs]
    assert kinds == [
        OccurrenceKind.EVENT_ENTRY,
        OccurrenceKind.RULE_ENTRY,
        OccurrenceKind.CMD_ENTER,
        OccurrenceKind.CMD_EXIT,
        OccurrenceKind.VAR_MOD,
        OccurrenceKind.BYTECODE,
    ]


def test_aggregated_counts() -> None:
    prof = parse_f5_log(SAMPLE_LOG)
    assert prof.source == "f5-log"
    assert prof.event_counts == {"CLIENT_ACCEPTED": 1}
    assert prof.command_counts == {"IP::remote_addr": 1}
    assert prof.var_mod_counts == {"the_one": 1}
    assert prof.bytecode_counts == {"push1": 1}
    # F5 logs carry no source lines.
    assert not prof.has_line_data
    assert prof.has_command_data


def test_command_timing_from_enter_exit() -> None:
    prof = parse_f5_log(SAMPLE_LOG)
    # CMD_EXIT (…633) − CMD_ENTRY (…630) == 3µs for IP::remote_addr.
    assert prof.command_time_us["IP::remote_addr"] == 3


def test_flow_context_captured() -> None:
    occs = parse_f5_occurrences(SAMPLE_LOG)
    ctx = occs[0].context
    assert ctx is not None
    assert ctx.virtual_server == "/Common/vs1"
    assert ctx.remote_addr == "10.10.10.2"
    assert ctx.local_port == 80
    assert ctx.flow_id == "0x9455"


def test_malformed_and_unknown_lines_skipped() -> None:
    text = "\n".join(
        [
            "",
            "# a comment",
            "not,enough",
            "123,RP_UNKNOWN_TYPE,/Common/vs1,foo,1,0x1,a,1,0,b,2,0",
            "456,RP_CMD_ENTRY,/Common/vs1,log,1,0x1,a,1,0,b,2,0",
        ]
    )
    prof = parse_f5_log(text)
    assert prof.command_counts == {"log": 1}


def test_merge_sums_counts() -> None:
    a = parse_f5_log(SAMPLE_LOG)
    b = parse_f5_log(SAMPLE_LOG)
    m = merge([a, b])
    assert m.command_counts == {"IP::remote_addr": 2}
    assert m.event_counts == {"CLIENT_ACCEPTED": 2}


def test_empty_profile_is_empty() -> None:
    assert ProfileData().is_empty
    assert parse_f5_log("").is_empty


def test_tolerates_syslog_prefix() -> None:
    # Real /var/log/ltm lines wrap the CSV payload in a syslog prefix.
    line = (
        "Nov 24 06:22:32 bigip1 info tmm[18291]: "
        "1511494952932622,RP_RULE_ENTRY,/Common/vs1,/Common/dc_1,"
        "18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,0"
    )
    occs = parse_f5_occurrences(line)
    assert len(occs) == 1
    assert occs[0].kind is OccurrenceKind.RULE_ENTRY
    assert occs[0].value == "/Common/dc_1"
    assert occs[0].timestamp_us == 1511494952932622
    assert occs[0].context is not None
    assert occs[0].context.virtual_server == "/Common/vs1"
    assert occs[0].context.process_id == 18291


def test_rule_span_timing() -> None:
    # RP_RULE_ENTRY → RP_RULE_EXIT span correlates to BIG-IP per-rule timing.
    log = "\n".join(
        [
            "1000,RP_RULE_ENTRY,/Common/vs1,/Common/dc_1,1,0x1,a,1,0,b,2,0",
            "1042,RP_RULE_EXIT,/Common/vs1,/Common/dc_1,1,0x1,a,1,0,b,2,0",
        ]
    )
    prof = parse_f5_log(log)
    assert prof.rule_time_us == {"/Common/dc_1": 42}


def test_multi_tmm_timing_paired_per_flow() -> None:
    # Two TMM (distinct pids) interleave; spans must not cross pid/flow.
    log = "\n".join(
        [
            "1000,RP_CMD_ENTRY,/Common/vs1,table,1,0xA,a,1,0,b,2,0",
            "1005,RP_CMD_ENTRY,/Common/vs1,table,2,0xB,a,1,0,b,2,0",
            "1010,RP_CMD_EXIT,/Common/vs1,table,2,0xB,a,1,0,b,2,0",  # pid2: 5µs
            "1100,RP_CMD_EXIT,/Common/vs1,table,1,0xA,a,1,0,b,2,0",  # pid1: 100µs
        ]
    )
    prof = parse_f5_log(log)
    assert prof.command_time_us == {"table": 105}
