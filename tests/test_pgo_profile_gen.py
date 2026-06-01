"""Tests for generating rule-profiler logs from the iRule test framework."""

from __future__ import annotations

import pytest

from compiler.pgo import find_pgo_suggestions
from compiler.pgo.occurrence import FlowContext, Occurrence, OccurrenceKind
from compiler.pgo.trace_parser import (
    format_f5_log,
    parse_f5_log,
    parse_f5_occurrences,
)

# --- serializer (pure, no backend) -----------------------------------------


def test_format_round_trips_through_parser() -> None:
    occ = Occurrence(
        kind=OccurrenceKind.CMD_ENTER,
        value="IP::client_addr",
        timestamp_us=42,
        depth=0,
        context=FlowContext(
            virtual_server="/Common/vs1",
            process_id=1,
            flow_id="0x9",
            remote_addr="10.0.0.2",
            remote_port=1234,
            remote_rd=0,
            local_addr="10.0.0.9",
            local_port=80,
            local_rd=0,
        ),
    )
    text = format_f5_log([occ])
    (back,) = parse_f5_occurrences(text)
    assert back.kind is OccurrenceKind.CMD_ENTER
    assert back.value == "IP::client_addr"
    assert back.timestamp_us == 42
    assert back.context is not None
    assert back.context.virtual_server == "/Common/vs1"
    assert back.context.local_port == 80


# --- generator (needs the in-process Tcl backend) ---------------------------


def _backend_available() -> bool:
    try:
        from tooling.irule_test.bridge import IruleTestSessionSync

        with IruleTestSessionSync(profiles=["TCP", "HTTP"]):
            return True
    except Exception:
        return False


requires_backend = pytest.mark.skipif(
    not _backend_available(), reason="iRule test framework backend unavailable"
)

_DISPATCH = """\
when HTTP_REQUEST {
    if {[IP::client_addr] eq "10.0.0.1"} {
        log local0. low
    } elseif {[IP::client_addr] eq "10.0.0.2"} {
        HTTP::redirect http://busy/
    } elseif {[IP::client_addr] eq "10.0.0.3"} {
        HTTP::respond 200
    }
}
"""


@requires_backend
def test_generates_rule_profiler_log() -> None:
    from tooling.irule_test.profile_gen import Stimulus, generate_occurrence_log

    log = generate_occurrence_log(
        _DISPATCH,
        [Stimulus(state={"connection": {"client_addr": "10.0.0.2"}}, repeat=3)],
    )
    assert "RP_EVENT_ENTRY,/Common/vs_test,HTTP_REQUEST" in log
    assert "RP_CMD_ENTRY" in log
    assert "IP::client_addr" in log
    # The log parses back cleanly and reflects the fired event.
    prof = parse_f5_log(log)
    assert prof.event_counts.get("HTTP_REQUEST") == 3
    # Core compiled commands (e.g. the implicit set/if) never appear.
    assert "set" not in prof.command_counts
    assert "if" not in prof.command_counts


@requires_backend
def test_generated_profile_drives_branch_reorder() -> None:
    from tooling.irule_test.profile_gen import Stimulus, generate_profile

    prof = generate_profile(
        _DISPATCH,
        [
            Stimulus(state={"connection": {"client_addr": "10.0.0.2"}}, repeat=20),
            Stimulus(state={"connection": {"client_addr": "10.0.0.3"}}, repeat=5),
            Stimulus(state={"connection": {"client_addr": "10.0.0.1"}}, repeat=2),
        ],
    )
    assert prof.source == "test-gen"
    # HTTP::redirect (hot branch) is the most-invoked extension command.
    assert prof.command_counts.get("HTTP::redirect", 0) == 20
    sugs = find_pgo_suggestions(_DISPATCH, prof)
    assert len(sugs) == 1
    assert "10.0.0.2" in sugs[0].message
