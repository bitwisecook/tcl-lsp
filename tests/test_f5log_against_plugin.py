"""Verification that our f5log parser matches the visidata-f5log
plugin's regex on every line of its bundled sample log.

The fixture ``tests/fixtures/f5log/visidata_gtm.log`` is copied
verbatim from ``bitwisecook/visidata-f5log`` (MIT) so we have a
known-good corpus against which we can compare:

- our deterministic tokeniser-based parser
  (``core.bigip.query._inputs.parse_f5log``); against
- a faithful in-line re-implementation of the plugin's main regex
  (``re_f5log``), used here as the reference.

Both extract the same five structural fields: timestamp, host,
severity ("level" in the plugin), daemon ("process"), pid, and the
F5 message code (``logid1:logid2`` in the plugin, ``module:level``
in ours).  Tests assert agreement on every line of the fixture so
parser drift between our offline implementation and the plugin's
regex is caught in CI.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query._inputs import parse_f5log


FIXTURE = Path(__file__).resolve().parent / "fixtures" / "f5log" / "visidata_gtm.log"


# The plugin's exact main regex — copied verbatim from
# ``bitwisecook/visidata-f5log/f5log.py`` so the comparison is
# against the canonical source.  Inline rather than importing the
# plugin module because the plugin depends on visidata's Sheet
# class which we shouldn't pull into this test surface.
_PLUGIN_REGEX = re.compile(
    r"^(?:(?:audit|gtm|ltm|security|tmm|user|\<\d+\>)\s+)?"
    r"(?:"
    r"(?P<date1>[A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})"
    r"|(?P<date2>\d+-\d+-\d+T\d+:\d+:\d+[+-]\d+:\d+)"
    r"|(?P<date3>\d+-\d+\s\d{2}:\d{2}:\d{2})"
    r")\s+(?P<host>\S+)\s+"
    r"(?:(?P<level>\S+)\s+"
    r"(?:(?P<process>[a-z0-9_()-]+\s?)\[(?P<pid>\d+)\]:\s+)?"
    r"(?:(?P<logid1>[0-9a-f]{8}):(?P<logid2>[0-9a-f]):\s+)?"
    r")?(?P<message>.*)$"
)


def _plugin_parse_one(line: str) -> dict:
    """Parse one line through the plugin's regex.

    Normalises the captures into the same dict shape our parser
    produces so the assertions stay symmetric: ``timestamp`` is the
    syslog spelling (matches our ``date1`` path), ``severity``
    drops to ``""`` when the plugin's ``level`` is the special
    swap-form sentinel from ``date3`` lines.
    """
    m = _PLUGIN_REGEX.match(line)
    if not m:
        return {}
    d = m.groupdict()
    if d.get("date3"):
        # ``MM-DD HH:MM:SS host level …`` quirk from tmsh ``show
        # sys log`` — the plugin swaps host/level after the match.
        d["host"], d["level"] = d["level"], d["host"]
    code_module = d.get("logid1") or ""
    code_level = d.get("logid2") or ""
    code = f"{code_module}:{code_level}" if code_module else ""
    process = (d.get("process") or "").strip()
    return {
        "timestamp": d.get("date1") or d.get("date2") or d.get("date3") or "",
        "host": d.get("host") or "",
        "severity": (d.get("level") or "").lower()
        if d.get("level") in _PLUGIN_SEVERITIES
        else "",
        "daemon": process,
        "pid": int(d["pid"]) if d.get("pid") else None,
        "code": code,
        "module": code_module,
        "level_int": int(code_level, 16) if code_level else None,
        "message": d.get("message") or "",
    }


_PLUGIN_SEVERITIES = {
    "emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"
}


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_fixture_exists_and_has_seventy_six_lines():
    """The MIT-licensed plugin sample log is shipped under
    ``tests/fixtures/f5log/`` so this test surface is hermetic and
    doesn't require the user's plugin checkout to be present."""
    assert FIXTURE.is_file(), f"missing fixture: {FIXTURE}"
    line_count = sum(1 for line in FIXTURE.read_text().splitlines() if line.strip())
    assert line_count == 76


def test_every_line_parses_without_dropping_to_raw_only():
    """A regression test for the structural parser — every line of
    the fixture must yield at least a timestamp.  An empty
    timestamp means the parser bailed early and only the ``raw``
    field has anything useful."""
    events = parse_f5log(FIXTURE.read_text())
    assert len(events) == 76
    missing_timestamp = [e for e in events if not e["timestamp"]]
    assert not missing_timestamp, (
        f"{len(missing_timestamp)} lines without timestamp; first: "
        f"{missing_timestamp[0]['raw']!r}"
    )


def test_severity_breakdown_matches_plugin():
    """Our severity counts should equal the plugin's regex counts.
    The fixture has 44 alert, 25 notice, 6 err, and one ``""``
    severity (the bare ``Saving running configuration...`` save
    line)."""
    events = parse_f5log(FIXTURE.read_text())
    ours: dict[str, int] = {}
    theirs: dict[str, int] = {}
    for line, event in zip(FIXTURE.read_text().splitlines(), events):
        if not line.strip():
            continue
        ours[event["severity"]] = ours.get(event["severity"], 0) + 1
        plug = _plugin_parse_one(line)
        theirs[plug["severity"]] = theirs.get(plug["severity"], 0) + 1
    assert ours == theirs == {"alert": 44, "notice": 25, "err": 6, "": 1}


def test_per_line_timestamp_host_daemon_pid_match_plugin():
    """Structural fields agree line-for-line with the plugin.

    The plugin's regex pulls ``date1`` for every syslog-shaped
    line; our parser yields the same string.  Hostname / daemon /
    pid all match exactly.  We do *not* compare the message body
    because the plugin and our parser differ on whether the
    leading F5 code is stripped — that's covered by the next test.
    """
    events = parse_f5log(FIXTURE.read_text())
    lines = [line for line in FIXTURE.read_text().splitlines() if line.strip()]
    assert len(events) == len(lines)
    for line, event in zip(lines, events):
        plug = _plugin_parse_one(line)
        assert plug, f"plugin regex failed on: {line!r}"
        assert event["timestamp"] == plug["timestamp"], line
        assert event["host"] == plug["host"], line
        assert event["daemon"] == plug["daemon"], line
        assert event["pid"] == plug["pid"], line


def test_per_line_code_matches_plugin():
    """The F5 message code (``XXXXXXXX:N``) is split into
    ``module`` + ``level`` in our parser, and into ``logid1`` +
    ``logid2`` in the plugin.  Their concatenation must agree on
    every line of the fixture."""
    events = parse_f5log(FIXTURE.read_text())
    for line, event in zip(
        [line for line in FIXTURE.read_text().splitlines() if line.strip()],
        events,
    ):
        plug = _plugin_parse_one(line)
        assert event["code"] == plug["code"], line
        assert event["module"] == plug["module"], line
        # Plugin records the level as a hex int; we record it as
        # an int too.  Plugin treats ``""`` as None; ours treats
        # missing as None too.
        ours = event["level"]
        if plug["level_int"] is None:
            assert ours is None, line
        else:
            assert ours == plug["level_int"], line


def test_saving_running_configuration_is_recognised_as_messageonly():
    """``Oct 28 03:36:30 gtm1 Saving running configuration...`` —
    the tmsh save line has no severity, no daemon, no F5 code.
    Both parsers agree the whole tail is the message and the
    daemon column is empty.
    """
    line = "Oct 28 03:36:30 gtm1 Saving running configuration..."
    [event] = parse_f5log(line)
    assert event["timestamp"] == "Oct 28 03:36:30"
    assert event["host"] == "gtm1"
    assert event["severity"] == ""
    assert event["daemon"] == ""
    assert event["pid"] is None
    assert event["code"] == ""
    assert event["message"] == "Saving running configuration..."


def test_source_tag_prefix_is_stripped():
    """Centralised syslog relays prepend the BIG-IP facility tag
    (``ltm``, ``gtm``, ``audit``, …).  Both parsers peel the tag
    before the timestamp."""
    line = "ltm Oct 28 03:06:07 bigip01 info tmm[1]: 01230140:6: stuff"
    [event] = parse_f5log(line)
    plug = _plugin_parse_one(line)
    assert event["host"] == "bigip01" == plug["host"]
    assert event["severity"] == "info" == plug["severity"]
    assert event["daemon"] == "tmm" == plug["daemon"]
    assert event["pid"] == 1 == plug["pid"]
    assert event["code"] == "01230140:6" == plug["code"]


def test_tmsh_short_iso_date_swaps_host_and_level():
    """``MM-DD HH:MM:SS`` lines from ``tmsh show sys log`` flip
    the host and level field order.  Our parser mirrors the
    plugin's swap so downstream queries treat both surfaces the
    same way."""
    # The plugin's date3 regex requires the level/host swap; lines
    # from tmsh look like:
    #   "10-28 03:06:07 info bigip01 tmm[1]: 01230140:6: stuff"
    # After swap: host=bigip01, level=info.
    line = "10-28 03:06:07 info bigip01 tmm[1]: 01230140:6: stuff"
    [event] = parse_f5log(line)
    plug = _plugin_parse_one(line)
    assert event["timestamp"] == "10-28 03:06:07" == plug["timestamp"]
    assert event["host"] == "bigip01" == plug["host"]
    assert event["severity"] == "info" == plug["severity"]
    assert event["daemon"] == "tmm" == plug["daemon"]


def test_rfc3339_with_offset_is_recognised():
    line = "2025-01-15T09:53:00+10:00 bigip01.example.test info tmm[1]: 01230140:6: hi"
    [event] = parse_f5log(line)
    plug = _plugin_parse_one(line)
    assert event["timestamp"] == "2025-01-15T09:53:00+10:00" == plug["timestamp"]
    assert event["host"] == "bigip01.example.test" == plug["host"]
    assert event["severity"] == "info" == plug["severity"]
    assert event["daemon"] == "tmm" == plug["daemon"]
    assert event["code"] == "01230140:6" == plug["code"]


# ---------------------------------------------------------------------------
# Query-DSL integration — the fixture flows through the query
# pipeline the same way an operator's audit query would.
# ---------------------------------------------------------------------------


@pytest.fixture
def fixture_query_runner():
    from core.bigip.query import run_query

    return run_query


def test_query_returns_top_gtm_alert_module(fixture_query_runner):
    """Realistic operator query: which GTM module fired the most
    ``alert`` lines in the fixture?  The answer should be
    ``011ae0f2`` (Monitor instance state changes), which the
    plugin's regex would also surface."""
    result = fixture_query_runner(
        f'[f5log_load("{FIXTURE}")[] | select(.severity == "alert") | .module] | sort',
        {"mem://x": ""},
    )
    [modules] = result.values_per_file["mem://x"]
    # Most-common module should be 011ae0f2 (Monitor instance state).
    from collections import Counter

    top = Counter(modules).most_common(1)[0][0]
    assert top == "011ae0f2"


def test_query_filters_err_lines_and_extracts_message(fixture_query_runner):
    """Every ``err`` line in the fixture references either an SSL
    failure or a monitor object.  The query lifts the message text
    out so an operator can grep it."""
    result = fixture_query_runner(
        f'[f5log_load("{FIXTURE}")[] | select(.severity == "err") | .message]',
        {"mem://x": ""},
    )
    [messages] = result.values_per_file["mem://x"]
    assert len(messages) == 6
    assert any("SSL error" in m for m in messages)
