"""Tests for the ``f5 query`` extensions added in the final pass.

Covers four areas:

* External JSON inputs (``--input-json`` / ``json_load``).
* ASCII / Unicode table output modes.
* ``PortSet`` / ``IPRange`` typed values and their DSL builtins.
* Probe gate — DNS works always; ping / tls / url etc. raise
  unless ``--enable-probes`` is set.

Network probes that require an actual network are exercised only
against localhost (``127.0.0.1``, ``localhost``) so the suite
stays runnable offline.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query
from core.bigip.query._probes import PROBES_ENABLED
from core.bigip.query.output import render
from core.bigip.types import IPRange, PortSet

# ---------------------------------------------------------------------------
# PortSet
# ---------------------------------------------------------------------------


class TestPortSet:
    def test_parse_comma_separated_segments(self):
        ps = PortSet.parse("80-82,8081")
        assert ps.count == 4
        assert ps.contains(81)
        assert not ps.contains(80 + 100)
        assert str(ps) == "80-82,8081"

    def test_segments_collapse_when_adjacent(self):
        ps = PortSet.parse("1,2,3,4,5")
        # Five single-port segments collapse to one [1-5] range.
        assert len(ps.segments) == 1
        assert ps.segments[0].low == 1
        assert ps.segments[0].high == 5

    def test_overlapping_segments_collapse(self):
        ps = PortSet.parse("1-10,5-15,20-30")
        # First two overlap → one range; third stays separate.
        assert len(ps.segments) == 2
        assert ps.segments[0].low == 1 and ps.segments[0].high == 15

    def test_wildcard_spelling(self):
        for spelling in ("any", "*", "0"):
            ps = PortSet.parse(spelling)
            assert ps.is_wildcard
            assert ps.count == 65536

    def test_overlap_detection(self):
        assert PortSet.parse("80-100").overlaps(PortSet.parse("90-110"))
        assert not PortSet.parse("80-100").overlaps(PortSet.parse("200-300"))

    def test_rejects_out_of_range(self):
        with pytest.raises(ValueError):
            PortSet.parse("80000")
        with pytest.raises(ValueError):
            PortSet.parse("-1-5")


# ---------------------------------------------------------------------------
# IPRange
# ---------------------------------------------------------------------------


class TestIPRange:
    def test_parse_two_endpoints(self):
        rng = IPRange.parse("192.168.9.77-192.168.9.83")
        assert rng.count == 7
        assert str(rng) == "192.168.9.77-192.168.9.83"

    def test_parse_single_address(self):
        rng = IPRange.parse("10.0.0.1")
        assert rng.count == 1
        assert str(rng) == "10.0.0.1"

    def test_membership(self):
        rng = IPRange.parse("10.0.0.5-10.0.0.10")
        assert rng.contains("10.0.0.7")
        assert not rng.contains("10.0.0.4")
        assert not rng.contains("10.0.0.11")

    def test_to_cidrs_summarises_misaligned_range(self):
        rng = IPRange.parse("192.168.9.77-192.168.9.83")
        cidrs = [str(c) for c in rng.to_cidrs()]
        # Three CIDRs to cover a non-CIDR-aligned 7-IP range.
        assert cidrs == [
            "192.168.9.77/32",
            "192.168.9.78/31",
            "192.168.9.80/30",
        ]

    def test_supernet_covers_endpoints(self):
        rng = IPRange.parse("192.168.9.77-192.168.9.83")
        supernet = rng.as_supernet()
        assert str(supernet) == "192.168.9.64/27"

    def test_mixed_family_rejected(self):
        with pytest.raises(ValueError):
            IPRange.parse("10.0.0.1-2001:db8::1")

    def test_reverse_range_rejected(self):
        with pytest.raises(ValueError):
            IPRange.parse("10.0.0.5-10.0.0.1")


# ---------------------------------------------------------------------------
# DSL builtins for the new types
# ---------------------------------------------------------------------------


_CFG = "ltm node /Common/n1 { address 10.0.0.1 }"


def _run(query: str) -> list:
    return run_query(query, {"mem://x": _CFG}).values_per_file["mem://x"]


class TestPortSetBuiltins:
    def test_contains(self):
        assert _run('port_set_contains("80-82,8081", 81)') == [True]
        assert _run('port_set_contains("80-82,8081", 90)') == [False]

    def test_count(self):
        assert _run('port_set_count("80-82,8081")') == [4]
        assert _run('port_set_count("any")') == [65536]

    def test_overlaps(self):
        assert _run('port_set_overlaps("80-90", "85-95")') == [True]
        assert _run('port_set_overlaps("80-90", "100-110")') == [False]


class TestIPRangeBuiltins:
    def test_to_cidrs(self):
        rows = _run('ip_range_to_cidrs("192.168.9.77-192.168.9.83")')
        assert rows == [["192.168.9.77/32", "192.168.9.78/31", "192.168.9.80/30"]]

    def test_supernet(self):
        assert _run('ip_range_supernet("192.168.9.77-192.168.9.83")') == ["192.168.9.64/27"]

    def test_count(self):
        assert _run('ip_range_count("10.0.0.5-10.0.0.9")') == [5]

    def test_contains(self):
        assert _run('ip_range_contains("10.0.0.5-10.0.0.9", "10.0.0.7")') == [True]
        assert _run('ip_range_contains("10.0.0.5-10.0.0.9", "10.0.0.20")') == [False]
        # Mixed-family inputs return False rather than raising.
        assert _run('ip_range_contains("10.0.0.5-10.0.0.9", "::1")') == [False]


# ---------------------------------------------------------------------------
# External JSON inputs
# ---------------------------------------------------------------------------


class TestJsonInputs:
    def test_input_json_binds_dict_to_named_variable(self, tmp_path: Path):
        """``--input-json NAME=PATH`` makes the parsed value
        available as ``$NAME``; the JSON file itself isn't
        iterated as a primary input."""
        json_path = tmp_path / "lookup.json"
        json_path.write_text('{"10.0.0.1": "web-1"}')

        result = run_query(
            ".ltm.node[] | {name: .name, addr: .address, owner: $lookup}",
            {
                "mem://b.conf": _CFG,
                f"file://{json_path}": json_path.read_text(),
            },
            names={"lookup": f"file://{json_path}"},
            json_sources={f"file://{json_path}"},
        )
        rows = result.values_per_file["mem://b.conf"]
        assert rows == [{"name": "n1", "addr": "10.0.0.1", "owner": {"10.0.0.1": "web-1"}}]

    def test_json_load_reads_external_file(self, tmp_path: Path):
        """``json_load(path)`` reads + parses ad-hoc inside a
        query — no CLI plumbing required."""
        json_path = tmp_path / "ad-hoc.json"
        json_path.write_text('{"version": 7, "servers": ["a", "b"]}')

        result = run_query(
            f'json_load("{json_path}")',
            {"mem://b.conf": _CFG},
        )
        [value] = result.values_per_file["mem://b.conf"]
        assert value == {"version": 7, "servers": ["a", "b"]}

    def test_json_load_raises_on_missing_file(self):
        from core.bigip.query.errors import QueryError

        with pytest.raises(QueryError, match="file not found"):
            run_query(
                'json_load("/definitely/not/a/real/path.json")',
                {"mem://b.conf": _CFG},
            )


# ---------------------------------------------------------------------------
# Table output
# ---------------------------------------------------------------------------


class TestTableOutput:
    def _sample_rows(self) -> list[dict]:
        return [
            {"name": "web", "port": 443, "tls": True},
            {"name": "api", "port": 8080, "tls": False},
        ]

    def test_ascii_table(self):
        out = render(self._sample_rows(), mode="table")
        # Border characters are pure ASCII.
        assert "│" not in out
        assert "+" in out
        assert "|" in out
        # Every header and cell renders.
        assert "name" in out
        assert "web" in out and "api" in out
        assert "443" in out and "8080" in out
        assert "true" in out and "false" in out

    def test_lineart_table(self):
        out = render(self._sample_rows(), mode="table-lineart")
        # Uses Unicode box-drawing.
        assert "│" in out
        assert "┌" in out
        assert "└" in out
        # No ASCII fallback characters.
        assert "+" not in out

    def test_table_column_widths_fit_data(self):
        out = render(
            [{"x": "short", "y": "a longer value here"}],
            mode="table",
        )
        # The "y" column's data length forces the header row to
        # be at least as wide as the longest cell.
        max_line = max(len(line) for line in out.splitlines())
        assert max_line >= len("a longer value here") + 4  # padding + borders

    def test_empty_input_renders_empty_string(self):
        assert render([], mode="table") == ""
        assert render([], mode="table-lineart") == ""

    def test_missing_keys_render_as_empty_cells(self):
        rows = [{"a": 1, "b": 2}, {"a": 3}]
        out = render(rows, mode="table")
        # Second row's missing ``b`` doesn't crash; just renders empty.
        assert " 3 " in out


# ---------------------------------------------------------------------------
# DNS (always on)
# ---------------------------------------------------------------------------


class TestDns:
    def test_localhost_forward(self):
        result = run_query('dns("localhost")', {"mem://x": _CFG})
        [addrs] = result.values_per_file["mem://x"]
        # ``localhost`` resolves to at least one loopback address on
        # every Unix-y box.
        assert addrs
        assert any(a.startswith("127.") or a == "::1" for a in addrs)

    def test_loopback_reverse(self):
        result = run_query('rev_dns("127.0.0.1")', {"mem://x": _CFG})
        [names] = result.values_per_file["mem://x"]
        # Most systems have "localhost" as the PTR; tolerate
        # platforms where /etc/hosts has been customised by only
        # asserting the call returns *some* name.
        assert isinstance(names, list)

    def test_nonexistent_returns_empty_list(self):
        result = run_query('dns("nonexistent.invalid")', {"mem://x": _CFG})
        assert result.values_per_file["mem://x"] == [[]]


# ---------------------------------------------------------------------------
# Probe gate — opt-in
# ---------------------------------------------------------------------------


class TestHttpResponseHelpers:
    """``http_*`` builtins make ``url_*`` responses ergonomic.

    These exercise the helpers directly via synthetic response
    dicts so the suite stays offline — no real URLs hit.  The
    integration with a live ``url_get`` is exercised by
    ``TestProbeGate``.
    """

    _RESP_OK = {
        "status": 200,
        "body": '{"version": 7, "items": [1, 2, 3]}',
        "headers": {"content-type": "application/json", "server": "nginx"},
        "error": None,
    }
    _RESP_404 = {"status": 404, "body": "not found", "headers": {}, "error": "Not Found"}
    _RESP_500 = {"status": 500, "body": "boom", "headers": {}, "error": "Server Error"}
    _RESP_302 = {
        "status": 302,
        "body": "",
        "headers": {"location": "https://elsewhere.example/"},
        "error": None,
    }

    def test_json_parse_round_trips_simple_values(self):
        from core.bigip.query.builtins import _builtin_json_parse

        assert _builtin_json_parse("[1, 2, 3]") == [1, 2, 3]
        assert _builtin_json_parse('{"a": 1}') == {"a": 1}
        assert _builtin_json_parse('"hello"') == "hello"
        assert _builtin_json_parse("null") is None

    def test_json_parse_raises_on_invalid_input(self):
        from core.bigip.query.builtins import _builtin_json_parse
        from core.bigip.query.errors import BuiltinError

        with pytest.raises(BuiltinError, match="invalid JSON"):
            _builtin_json_parse("not json at all")

    def test_http_status_returns_int(self):
        from core.bigip.query.builtins import _builtin_http_status

        assert _builtin_http_status(self._RESP_OK) == 200
        assert _builtin_http_status(self._RESP_404) == 404

    def test_http_body_returns_string(self):
        from core.bigip.query.builtins import _builtin_http_body

        body = _builtin_http_body(self._RESP_OK)
        assert isinstance(body, str)
        assert "version" in body

    def test_http_body_json_parses_json_payload(self):
        from core.bigip.query.builtins import _builtin_http_body_json

        parsed = _builtin_http_body_json(self._RESP_OK)
        assert parsed == {"version": 7, "items": [1, 2, 3]}

    def test_http_body_json_empty_body_returns_none(self):
        """A blank body (common on 204 No Content / 302 redirect)
        produces ``null`` rather than raising — keeps audit
        pipelines from blowing up on benign cases."""
        from core.bigip.query.builtins import _builtin_http_body_json

        assert _builtin_http_body_json(self._RESP_302) is None

    def test_http_body_json_raises_on_invalid_payload(self):
        from core.bigip.query.builtins import _builtin_http_body_json
        from core.bigip.query.errors import BuiltinError

        with pytest.raises(BuiltinError, match="invalid JSON"):
            _builtin_http_body_json(self._RESP_404)  # body is "not found"

    def test_http_header_is_case_insensitive(self):
        from core.bigip.query.builtins import _builtin_http_header

        assert _builtin_http_header(self._RESP_OK, "Content-Type") == "application/json"
        assert _builtin_http_header(self._RESP_OK, "content-type") == "application/json"
        assert _builtin_http_header(self._RESP_OK, "CONTENT-TYPE") == "application/json"
        assert _builtin_http_header(self._RESP_OK, "X-Missing") is None

    def test_http_headers_returns_dict(self):
        from core.bigip.query.builtins import _builtin_http_headers

        h = _builtin_http_headers(self._RESP_OK)
        assert h == {"content-type": "application/json", "server": "nginx"}

    def test_status_class_predicates(self):
        from core.bigip.query.builtins import (
            _builtin_http_client_error,
            _builtin_http_ok,
            _builtin_http_redirect,
            _builtin_http_server_error,
        )

        # 200 → ok only.
        assert _builtin_http_ok(self._RESP_OK)
        assert not _builtin_http_redirect(self._RESP_OK)
        assert not _builtin_http_client_error(self._RESP_OK)
        assert not _builtin_http_server_error(self._RESP_OK)
        # 302 → redirect only.
        assert _builtin_http_redirect(self._RESP_302)
        assert not _builtin_http_ok(self._RESP_302)
        # 404 → client_error only.
        assert _builtin_http_client_error(self._RESP_404)
        assert not _builtin_http_ok(self._RESP_404)
        # 500 → server_error only.
        assert _builtin_http_server_error(self._RESP_500)
        assert not _builtin_http_client_error(self._RESP_500)

    def test_predicates_handle_missing_status(self):
        """A response that failed before the server answered has
        ``status=None``.  Every class predicate must return
        ``False`` rather than raising."""
        from core.bigip.query.builtins import (
            _builtin_http_client_error,
            _builtin_http_ok,
            _builtin_http_redirect,
            _builtin_http_server_error,
        )

        no_status = {"status": None, "body": "", "headers": {}, "error": "connect timeout"}
        assert not _builtin_http_ok(no_status)
        assert not _builtin_http_redirect(no_status)
        assert not _builtin_http_client_error(no_status)
        assert not _builtin_http_server_error(no_status)

    def test_dsl_pipe_into_http_helpers(self):
        """End-to-end: build a synthetic response inside the DSL
        with an object literal, then chain ``http_*`` helpers off
        the pipe.  Hyphenated keys need string quoting."""
        from core.bigip.query import run_query

        cfg = "ltm node /Common/n {address 10.0.0.1}"
        # ``{"content-type": ...}`` form (key quoted because of the hyphen).
        q = (
            '{status: 200, body: "{\\"x\\": 1}", '
            '"headers": {"content-type": "application/json"}} '
            "| {code: http_status(.), parsed: http_body_json(.), "
            'ct: http_header(., "Content-Type"), ok: http_ok(.)}'
        )
        result = run_query(q, {"mem://x": cfg})
        [row] = result.values_per_file["mem://x"]
        assert row == {
            "code": 200,
            "parsed": {"x": 1},
            "ct": "application/json",
            "ok": True,
        }


class TestProbeGate:
    def test_ping_raises_without_probes(self):
        from core.bigip.query.errors import QueryError

        with pytest.raises(QueryError, match="probes are disabled"):
            run_query('ping("127.0.0.1")', {"mem://x": _CFG})

    def test_portping_raises_without_probes(self):
        from core.bigip.query.errors import QueryError

        with pytest.raises(QueryError, match="probes are disabled"):
            run_query('portping("127.0.0.1", 22)', {"mem://x": _CFG})

    def test_url_get_raises_without_probes(self):
        from core.bigip.query.errors import QueryError

        with pytest.raises(QueryError, match="probes are disabled"):
            run_query('url_get("http://127.0.0.1/")', {"mem://x": _CFG})

    def test_url_get_rejects_non_http_schemes_when_enabled(self):
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query('url_get("file:///etc/passwd")', {"mem://x": _CFG})
        finally:
            PROBES_ENABLED.reset(token)
        [row] = result.values_per_file["mem://x"]
        assert row["status"] is None
        assert row["body"] == ""
        assert row["error"] == "unsupported URL scheme: file"

    def test_ping_succeeds_when_enabled(self):
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query('ping("127.0.0.1")', {"mem://x": _CFG})
        finally:
            PROBES_ENABLED.reset(token)
        [row] = result.values_per_file["mem://x"]
        assert row["ok"] is True
        assert row["rtt_ms"] is not None

    def test_portping_loopback_returns_rtt(self):
        # Find an actually-open port: bind one ourselves.
        import socket as _socket

        with _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            s.listen(1)
            port = s.getsockname()[1]
            token = PROBES_ENABLED.set(True)
            try:
                result = run_query(
                    f'portping("127.0.0.1", {port})',
                    {"mem://x": _CFG},
                )
            finally:
                PROBES_ENABLED.reset(token)
        [row] = result.values_per_file["mem://x"]
        assert row["ok"] is True


# ---------------------------------------------------------------------------
# X.509 — parse a self-signed cert generated on-the-fly
# ---------------------------------------------------------------------------


def test_x509_parse_rejects_non_pem_input():
    """The X.509 parser must reject obvious garbage with a clear
    error rather than silently returning a malformed dict.

    Round-trip parsing against a real certificate is exercised
    indirectly via ``tls_handshake`` in integration tests; the
    self-signed fixture path needs ``cryptography`` and lives in
    the broader test-slow suite rather than the ci-fast bucket.
    """
    from core.bigip.query.errors import QueryError

    with pytest.raises(QueryError, match="not a PEM certificate"):
        run_query('x509_parse("not a certificate at all")', {"mem://x": _CFG})
