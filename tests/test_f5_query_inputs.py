"""Tests for the structured-input formats (JSONL / CSV / F5 log).

Each format ships with both a CLI flag (``--input-<kind>``) and a
peer builtin (``<kind>_load``).  These tests cover the parser,
the builtin, and the runner-level binding path so the two surfaces
stay in lockstep.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from textwrap import dedent

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query
from core.bigip.query._inputs import (
    InputSpec,
    parse_csv,
    parse_f5log,
    parse_jsonl,
)

BIG_CONF = "ltm node /Common/n1 { address 10.0.0.1 }\n"


# ---------------------------------------------------------------------------
# JSONL
# ---------------------------------------------------------------------------


class TestJSONL:
    def test_parse_basic(self):
        text = '{"a": 1}\n{"a": 2}\n{"a": 3}\n'
        assert parse_jsonl(text) == [{"a": 1}, {"a": 2}, {"a": 3}]

    def test_parse_skips_blank_lines(self):
        text = '{"a": 1}\n\n  \n{"a": 2}\n'
        assert parse_jsonl(text) == [{"a": 1}, {"a": 2}]

    def test_parse_quotes_line_number_on_error(self):
        text = '{"a": 1}\nnot-json\n{"a": 2}\n'
        from core.bigip.query._inputs import InputError

        with pytest.raises(InputError, match="line 2"):
            parse_jsonl(text)

    def test_jsonl_load_builtin(self, tmp_path: Path):
        p = tmp_path / "events.jsonl"
        p.write_text('{"vs": "web", "ok": true}\n{"vs": "api", "ok": false}\n')
        result = run_query(f'jsonl_load("{p}") | map(.vs)', {"mem://x": BIG_CONF})
        [names] = result.values_per_file["mem://x"]
        assert names == ["web", "api"]

    def test_jsonl_load_in_query_filter(self, tmp_path: Path):
        p = tmp_path / "events.jsonl"
        p.write_text(
            '\n'.join(json.dumps(r) for r in [
                {"severity": "info", "msg": "ok"},
                {"severity": "err", "msg": "kaboom"},
                {"severity": "warning", "msg": "ish"},
                {"severity": "err", "msg": "oops"},
            ]) + "\n"
        )
        result = run_query(
            f'[jsonl_load("{p}")[] | select(.severity == "err") | .msg]',
            {"mem://x": BIG_CONF},
        )
        [msgs] = result.values_per_file["mem://x"]
        assert msgs == ["kaboom", "oops"]

    def test_input_jsonl_via_runner_spec(self, tmp_path: Path):
        p = tmp_path / "events.jsonl"
        p.write_text('{"id": 1}\n{"id": 2}\n')
        result = run_query(
            "$events[] | .id",
            {"mem://x": BIG_CONF, str(p): p.read_text()},
            input_specs={str(p): InputSpec(kind="jsonl")},
        )
        # The events file is named ``events.jsonl`` — the runner
        # auto-derives ``$events`` from the URI stem.
        # Some buckets accumulate the iterations; flatten and check.
        flat = [v for vals in result.values_per_file.values() for v in vals]
        assert flat == [1, 2]


# ---------------------------------------------------------------------------
# CSV
# ---------------------------------------------------------------------------


class TestCSV:
    def test_parse_with_header_row(self):
        text = "name,port\nweb,443\napi,8443\n"
        assert parse_csv(text) == [
            {"name": "web", "port": "443"},
            {"name": "api", "port": "8443"},
        ]

    def test_parse_with_supplied_headers(self):
        text = "web,443\napi,8443\n"
        assert parse_csv(text, headers=["name", "port"]) == [
            {"name": "web", "port": "443"},
            {"name": "api", "port": "8443"},
        ]

    def test_parse_skips_blank_rows(self):
        text = "a,b\n1,2\n\n3,4\n"
        assert parse_csv(text) == [{"a": "1", "b": "2"}, {"a": "3", "b": "4"}]

    def test_parse_missing_columns_become_empty(self):
        text = "a,b,c\n1\n"
        assert parse_csv(text) == [{"a": "1", "b": "", "c": ""}]

    def test_parse_extra_columns_land_in_extra(self):
        text = "a\n1,extra,here\n"
        result = parse_csv(text)
        assert result == [{"a": "1", "_extra": ["extra", "here"]}]

    def test_csv_load_with_header_row(self, tmp_path: Path):
        p = tmp_path / "vips.csv"
        p.write_text("vs,port\nweb,443\napi,8443\n")
        result = run_query(
            f'csv_load("{p}") | map(.vs)', {"mem://x": BIG_CONF}
        )
        [names] = result.values_per_file["mem://x"]
        assert names == ["web", "api"]

    def test_csv_load_with_supplied_headers(self, tmp_path: Path):
        p = tmp_path / "nats.csv"
        p.write_text("10.0.0.10,203.0.113.10\n10.0.0.20,203.0.113.20\n")
        result = run_query(
            f'csv_load("{p}", ["internal", "external"])',
            {"mem://x": BIG_CONF},
        )
        [rows] = result.values_per_file["mem://x"]
        assert rows == [
            {"internal": "10.0.0.10", "external": "203.0.113.10"},
            {"internal": "10.0.0.20", "external": "203.0.113.20"},
        ]

    def test_csv_load_supplied_headers_must_be_strings(self, tmp_path: Path):
        from core.bigip.query.errors import QueryError

        p = tmp_path / "x.csv"
        p.write_text("a,b\n")
        with pytest.raises(QueryError, match="must be a string"):
            run_query(
                f'csv_load("{p}", [1, 2])', {"mem://x": BIG_CONF}
            )

    def test_input_csv_via_runner_spec_with_headers(self, tmp_path: Path):
        p = tmp_path / "nats.csv"
        p.write_text("10.0.0.10,203.0.113.10\n10.0.0.20,203.0.113.20\n")
        result = run_query(
            "$nats[] | .external",
            {"mem://x": BIG_CONF, str(p): p.read_text()},
            input_specs={
                str(p): InputSpec(
                    kind="csv",
                    options=(("headers", ("internal", "external")),),
                )
            },
            names={"nats": str(p)},
        )
        flat = [v for vals in result.values_per_file.values() for v in vals]
        assert flat == ["203.0.113.10", "203.0.113.20"]


class TestCSVInputFlagParsing:
    def test_header_suffix_is_split_from_final_simple_segment(self):
        from explorer.verbs.f5.query import _parse_input_bindings

        bindings, err = _parse_input_bindings(
            ["nats=/tmp/nats.csv:internal,external"],
            flag="--input-csv",
            allow_csv_headers=True,
        )
        assert err is None
        assert bindings == {"nats": ("/tmp/nats.csv", ["internal", "external"])}

    def test_path_colon_is_not_always_a_header_separator(self):
        from explorer.verbs.f5.query import _parse_input_bindings

        bindings, err = _parse_input_bindings(
            ["nats=/tmp/source:archive/nats.csv"],
            flag="--input-csv",
            allow_csv_headers=True,
        )
        assert err is None
        assert bindings == {"nats": ("/tmp/source:archive/nats.csv", None)}

    def test_windows_style_path_is_not_treated_as_headers(self):
        from explorer.verbs.f5.query import _parse_input_bindings

        bindings, err = _parse_input_bindings(
            [r"nats=C:\exports\nats.csv"],
            flag="--input-csv",
            allow_csv_headers=True,
        )
        assert err is None
        assert bindings == {r"nats": (r"C:\exports\nats.csv", None)}


# ---------------------------------------------------------------------------
# F5 log
# ---------------------------------------------------------------------------


class TestF5Log:
    def _line(self) -> str:
        return (
            "Nov 28 09:53:00 bigip01 info tmm[12345]: "
            "01230140:6: Connection limit reached for /Common/web_pool"
        )

    def test_parse_classic_syslog_with_code(self):
        [event] = parse_f5log(self._line())
        assert event["timestamp"] == "Nov 28 09:53:00"
        assert event["host"] == "bigip01"
        assert event["severity"] == "info"
        assert event["daemon"] == "tmm"
        assert event["pid"] == 12345
        assert event["code"] == "01230140:6"
        assert event["module"] == "01230140"
        assert event["level"] == 6
        assert "Connection limit reached" in event["message"]

    def test_parse_rfc3164_with_pri(self):
        text = "<27>" + self._line()
        [event] = parse_f5log(text)
        assert event["timestamp"] == "Nov 28 09:53:00"
        assert event["daemon"] == "tmm"
        assert event["module"] == "01230140"

    def test_parse_without_pid(self):
        text = "Nov 28 09:53:00 bigip01 notice mcpd: 01070417:5: AUDIT - tmsh updated /Common/x"
        [event] = parse_f5log(text)
        assert event["daemon"] == "mcpd"
        assert event["pid"] is None
        assert event["module"] == "01070417"
        assert event["level"] == 5
        assert "AUDIT" in event["message"]

    def test_parse_without_code_keeps_raw_message(self):
        text = "Nov 28 09:53:00 bigip01 info named[6543]: zone example.com loaded"
        [event] = parse_f5log(text)
        assert event["daemon"] == "named"
        assert event["pid"] == 6543
        assert event["code"] == ""
        assert event["module"] == ""
        assert event["level"] is None
        assert "zone example.com loaded" in event["message"]

    def test_parse_skips_blank_lines(self):
        text = self._line() + "\n\n  \n" + self._line() + "\n"
        events = parse_f5log(text)
        assert len(events) == 2

    def test_parse_empty_file(self):
        assert parse_f5log("") == []
        assert parse_f5log("\n  \n") == []

    def test_parse_unrecognised_line_keeps_raw(self):
        text = "this is not a syslog line at all"
        [event] = parse_f5log(text)
        assert event["raw"] == text
        assert event["timestamp"] == ""

    def test_f5log_load_builtin(self, tmp_path: Path):
        p = tmp_path / "ltm"
        p.write_text(
            "\n".join(
                [
                    "Nov 28 09:53:00 bigip01 info tmm[1]: 01230140:6: Connection limit",
                    "Nov 28 09:53:01 bigip01 err tmm[1]: 01230140:3: Pool down",
                    "Nov 28 09:53:02 bigip01 info bigd[2]: monitor /Common/http OK",
                ]
            )
            + "\n"
        )
        result = run_query(
            f'[f5log_load("{p}")[] | select(.severity == "err") | .message]',
            {"mem://x": BIG_CONF},
        )
        [errors] = result.values_per_file["mem://x"]
        assert errors == ["Pool down"]

    def test_f5log_load_filter_by_module(self, tmp_path: Path):
        p = tmp_path / "ltm"
        p.write_text(
            "\n".join(
                [
                    "Nov 28 09:53:00 bigip01 info logger[1]: 01070417:6: AUDIT - tmsh update /Common/x",
                    "Nov 28 09:53:01 bigip01 info tmm[2]: 01230140:6: stuff",
                    "Nov 28 09:53:02 bigip01 info logger[1]: 01070417:6: AUDIT - tmsh delete /Common/y",
                ]
            )
            + "\n"
        )
        result = run_query(
            f'[f5log_load("{p}")[] | select(.module == "01070417") | .message]',
            {"mem://x": BIG_CONF},
        )
        [audits] = result.values_per_file["mem://x"]
        assert len(audits) == 2
        assert all("AUDIT" in a for a in audits)

    def test_input_f5log_via_runner_spec(self, tmp_path: Path):
        p = tmp_path / "ltm"
        p.write_text(
            "Nov 28 09:53:00 bigip01 info tmm[1]: 01230140:6: stuff\n"
        )
        result = run_query(
            "$ltm[] | .module",
            {"mem://x": BIG_CONF, str(p): p.read_text()},
            input_specs={str(p): InputSpec(kind="f5log")},
            names={"ltm": str(p)},
        )
        flat = [v for vals in result.values_per_file.values() for v in vals]
        assert flat == ["01230140"]


# ---------------------------------------------------------------------------
# Cross-format: a query that joins JSONL events with a CSV inventory.
# ---------------------------------------------------------------------------


def test_join_jsonl_events_with_csv_inventory(tmp_path: Path):
    """Realistic shape: an event log (JSONL) gets enriched against
    an inventory CSV — for each error event, look up the VS owner."""
    events = tmp_path / "events.jsonl"
    events.write_text(
        dedent(
            """\
            {"vs": "web_vs", "severity": "err", "msg": "down"}
            {"vs": "api_vs", "severity": "info", "msg": "ok"}
            {"vs": "web_vs", "severity": "warning", "msg": "ish"}
            """
        )
    )
    inventory = tmp_path / "inventory.csv"
    inventory.write_text("vs,owner\nweb_vs,team-web\napi_vs,team-api\n")
    query = """
    [ jsonl_load("PATH_EVENTS")[]
      | select(.severity == "err")
      | . as $ev
      | csv_load("PATH_INVENTORY")[]
      | select(.vs == $ev.vs)
      | { vs: $ev.vs, msg: $ev.msg, owner: .owner }
    ]
    """.replace("PATH_EVENTS", str(events)).replace("PATH_INVENTORY", str(inventory))
    result = run_query(query, {"mem://x": BIG_CONF})
    [rows] = result.values_per_file["mem://x"]
    assert rows == [{"vs": "web_vs", "msg": "down", "owner": "team-web"}]
