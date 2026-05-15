"""Tests for the multitier sample logs.

The logs under ``samples/for_f5_query/multitier/logs/`` are generated
by ``samples/for_f5_query/multitier/generate_logs.py`` from the
configs in the same directory.  These tests cover the realistic
query shapes an operator would actually run against a fleet of
BIG-IPs — severity rollups, audit extraction, monitor flap counts,
and joining log events against the LTM config they reference.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query


REPO_ROOT = Path(__file__).resolve().parent.parent
MULTITIER = REPO_ROOT / "samples" / "for_f5_query" / "multitier"
LOGS_DIR = MULTITIER / "logs"


def _flat(result, uri: str | None = None) -> list:
    if uri is not None:
        return result.values_per_file[uri]
    flat: list = []
    for values in result.values_per_file.values():
        flat.extend(values)
    return flat


# ---------------------------------------------------------------------------
# Fixture sanity — generator must have produced all the expected files.
# ---------------------------------------------------------------------------


def test_logs_dir_has_one_log_per_device():
    """The generator should produce a ``.log`` per device.  This is
    the cheapest test that catches generator drift before the more
    semantic checks below."""
    assert LOGS_DIR.is_dir(), (
        f"missing {LOGS_DIR} — run python {MULTITIER}/generate_logs.py first"
    )
    log_files = sorted(p.name for p in LOGS_DIR.glob("*.log"))
    # 1 GTM + 2 tier-1 + 24 tier-2 (12 pairs) + 1 tier-3 = 28 files.
    assert len(log_files) == 28, f"unexpected log set: {log_files}"
    assert "gtm-dns.log" in log_files
    assert "t1-a.log" in log_files and "t1-b.log" in log_files
    assert "t2-c01-a.log" in log_files and "t2-c12-b.log" in log_files
    assert "tier3-reaggregator.log" in log_files


@pytest.mark.parametrize(
    "log",
    sorted(LOGS_DIR.glob("*.log")) if LOGS_DIR.is_dir() else [],
)
def test_each_log_parses_cleanly(log: Path):
    """``f5log_load`` should produce a dict per non-blank line with
    a recognisable timestamp and message; ``raw`` always survives."""
    line_count = sum(1 for line in log.read_text().splitlines() if line.strip())
    result = run_query(f'f5log_load("{log}")', {"mem://x": ""})
    [events] = result.values_per_file["mem://x"]
    assert len(events) == line_count, f"{log.name}: parser dropped lines"
    for event in events:
        assert event["raw"], f"{log.name}: empty raw on {event!r}"
        # Every line was generated with a real timestamp; the
        # parser must recognise it.
        assert event["timestamp"], f"{log.name}: missing timestamp on {event!r}"


# ---------------------------------------------------------------------------
# Severity rollups (the cookbook "what fired overnight?" query).
# ---------------------------------------------------------------------------


def test_severity_breakdown_for_one_tier1_device():
    """Severity counts on ``t1-a`` should match the generator's
    explicit emission pattern: a couple of err lines, the monitor
    warnings, one notice per VS, the rest info."""
    log = LOGS_DIR / "t1-a.log"
    query = f"""
    [f5log_load("{log}")[] | .severity] as $sevs
    | {{
        info:    ([$sevs[] | select(. == "info")]    | count),
        notice:  ([$sevs[] | select(. == "notice")]  | count),
        warning: ([$sevs[] | select(. == "warning")] | count),
        err:     ([$sevs[] | select(. == "err")]     | count)
      }}
    """
    [counts] = _flat(run_query(query, {"mem://x": ""}))
    # t1-a config has 1 VS + 12 members.  Generator emits:
    #   boot+config+HA (info)               -> 3 info
    #   1 audit per VS                      -> 1 info
    #   12 monitor down (warning)           -> 12 warning
    #   12 monitor up (info)                -> 12 info
    #   1 conn-limit (notice)               -> 1 notice
    #   1 ssl handshake (warning)           -> 1 warning
    #   2 err lines                         -> 2 err
    #   1 audit save + 1 save (info)        -> 2 info
    # Totals: 18 info, 1 notice, 13 warning, 2 err.
    assert counts == {"info": 18, "notice": 1, "warning": 13, "err": 2}


def test_err_lines_quote_pool_and_member():
    """The ``err`` lines emitted by the generator reference real
    pool + member names lifted from the device's config.  An audit
    pipeline can extract them straight from the message text."""
    log = LOGS_DIR / "t2-c01-a.log"
    query = f"""
    [f5log_load("{log}")[]
       | select(.severity == "err")
       | .message
    ]
    """
    [messages] = _flat(run_query(query, {"mem://x": ""}))
    assert len(messages) == 2
    # All err lines should mention the backend pool from the config.
    assert all("/Common/backend_pool" in m for m in messages)
    assert any("backend_1:8443" in m for m in messages)
    assert any("backend_2:8443" in m for m in messages)


# ---------------------------------------------------------------------------
# Audit extraction (every tmsh ``modify`` lands as an AUDIT line).
# ---------------------------------------------------------------------------


def test_audit_lines_lift_out_via_module_filter():
    """The ``01070417`` module is reserved for AUDIT records.
    Filtering by module is the canonical "who changed what" pipeline
    F5 operators run after an incident."""
    log = LOGS_DIR / "t2-c05-a.log"
    query = f"""
    [f5log_load("{log}")[]
       | select(.module == "01070417")
       | .message
    ]
    """
    [audits] = _flat(run_query(query, {"mem://x": ""}))
    # Generator emits one AUDIT line per VS — tier-2 has exactly 1 VS.
    assert len(audits) == 1
    assert all("AUDIT" in a for a in audits)
    assert all("tmsh" in a for a in audits)


def test_save_config_audit_lands_in_save_module():
    """``01071be3`` is the AUDIT module for ``save sys config``.  This
    is what an operator greps for to find when the running config
    was last persisted."""
    log = LOGS_DIR / "t2-c08-a.log"
    query = f"""
    [f5log_load("{log}")[]
       | select(.module == "01071be3")
       | .message
    ]
    """
    [saves] = _flat(run_query(query, {"mem://x": ""}))
    assert len(saves) == 1
    assert "save sys config" in saves[0]


# ---------------------------------------------------------------------------
# Monitor flap detection — count down/up cycles per pool member.
# ---------------------------------------------------------------------------


def test_monitor_flap_count_per_member():
    """Each member's full down/up cycle is two lines, paired by
    module code.  An aggregation pipeline counts those pairs by
    member so the operator can spot the ones flapping hardest."""
    log = LOGS_DIR / "t1-a.log"
    query = f"""
    [f5log_load("{log}")[]
       | select(.module == "01340011" or .module == "01340012")
    ] | count
    """
    [flap_lines] = _flat(run_query(query, {"mem://x": ""}))
    # t1-a's t2_fanout_pool has 12 members.  Generator emits one
    # down + one up per member -> 24 lines.
    assert flap_lines == 24


# ---------------------------------------------------------------------------
# GTM-specific: wide-IP resolutions reference the same wide-IP
# the GTM config declares.
# ---------------------------------------------------------------------------


def test_gtm_wideip_resolves_reference_configured_wideip():
    log = LOGS_DIR / "gtm-dns.log"
    config = (MULTITIER / "gtm-dns.conf").read_text()
    # Query: every wideip mentioned in the log must appear in the
    # GTM config's wideip catalogue.  This is the join that ties
    # operational evidence (the resolution log) to the declarative
    # state (the SCF).
    query = f"""
    [f5log_load("{log}")[]
       | select(.module == "011a3055")
       | (.message | split(., " ")[1])
    ] | unique
    """
    [resolved_wideips] = _flat(run_query(query, {"mem://x": ""}))
    assert resolved_wideips == ["/Common/www.example.com"]
    # And the same wideip must exist in the config when we run the
    # query against the actual GTM SCF.
    config_query = '[.gtm["wideip"][] | ."full-path"] | unique'
    result = run_query(config_query, {"gtm.conf": config})
    [wideips] = result.values_per_file["gtm.conf"]
    assert wideips == ["/Common/www.example.com"]


# ---------------------------------------------------------------------------
# Cross-source: a log line names a virtual server, and we look up
# its destination in the corresponding LTM config.
# ---------------------------------------------------------------------------


def test_audit_lines_match_real_virtual_destinations():
    """For every tier-2 cluster the generator emits an AUDIT line
    naming ``/Common/app_vs``.  Pull that name out and confirm the
    same VS exists in the cluster's LTM config, with a destination
    that matches what's in the tier-1 fanout pool."""
    log = LOGS_DIR / "t2-c03-a.log"
    cfg = (MULTITIER / "tier2-c03-ltm-ha.conf").read_text()
    # The AUDIT message reads:
    #   AUDIT - jadmin - tmsh - tmsh -c modify ltm virtual /Common/app_vs description ...
    # Split on whitespace -> index 11 is the VS full-path.
    log_query = f"""
    [f5log_load("{log}")[]
       | select(.module == "01070417")
       | (.message | split(., " ")[11])
    ] | unique
    """
    [vs_names] = _flat(run_query(log_query, {"mem://x": ""}))
    assert vs_names == ["/Common/app_vs"]
    # The same VS exists in the config and is destined to the
    # tier-2 cluster's well-known VIP.
    config_query = '.ltm.virtual[] | { name: ."full-path", dest: .destination }'
    config_result = run_query(config_query, {"ltm.conf": cfg})
    vs_rows = config_result.values_per_file["ltm.conf"]
    by_name = {r["name"]: r for r in vs_rows}
    assert "/Common/app_vs" in by_name
    assert "10.2.0.10" in by_name["/Common/app_vs"]["dest"]


# ---------------------------------------------------------------------------
# Whole-fleet rollup — load every device's log at once via the
# ``--input-f5log`` runner-side surface (modelled as input_specs).
# ---------------------------------------------------------------------------


def test_whole_fleet_severity_rollup_via_runner_specs():
    """End-to-end query that loads every multitier log as a side
    input, aggregates ``err`` lines fleet-wide, and proves the new
    structured-input plumbing scales to many simultaneous side
    inputs.
    """
    from core.bigip.query._inputs import InputSpec

    log_files = sorted(LOGS_DIR.glob("*.log"))
    sources: dict[str, str] = {"mem://x": ""}
    specs: dict[str, InputSpec] = {}
    names: dict[str, str] = {}
    for p in log_files:
        sources[str(p)] = p.read_text()
        specs[str(p)] = InputSpec(kind="f5log")
        # Use the device label as the variable name (``t1_a``,
        # ``t2_c01_a``, …) — the stem maps directly when we
        # normalise hyphens.
        var = p.stem.replace("-", "_")
        names[var] = str(p)
    # For every log binding, emit the device + its err count.
    parts = []
    for var in sorted(names):
        parts.append(f'{{ device: "{var}", errs: ([${var}[] | select(.severity == "err")] | count) }}')
    query = "[ " + ", ".join(parts) + " ] | sort"
    result = run_query(query, sources, input_specs=specs, names=names)
    [rows] = _flat(result, uri="mem://x")
    # Every device gets a row.
    assert len(rows) == 28
    # Cherry-pick the tier-1 / one tier-2 / GTM rows to confirm the
    # err counts match what the generator emitted.
    by_device = {r["device"]: r["errs"] for r in rows}
    assert by_device["t1_a"] == 2
    assert by_device["t2_c01_a"] == 2
    assert by_device["gtm_dns"] == 0  # no err lines on the GTM
