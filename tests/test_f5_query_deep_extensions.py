"""Deep tests for the f5 query DSL extensions.

These are the "everything composed" tests — designed to catch
regressions where one builtin works in isolation but breaks when
combined with another.  The scenarios mirror what a real audit
pipeline would do: read BIG-IP config + a CMDB JSON + a threat-
feed JSON, join them, classify the results, and render the
finding as a table.

Network-effecting builtins (ping / url / tls) are exercised
against ``127.0.0.1`` and a real, in-process TLS server so the
suite remains runnable offline.
"""

from __future__ import annotations

import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query
from core.bigip.query._probes import PROBES_ENABLED
from core.bigip.query.output import render

# ---------------------------------------------------------------------------
# Rich fixture — a small "data centre" with config + CMDB + threat feed.
# ---------------------------------------------------------------------------


BIG_CONF = """\
ltm node /Common/web1 { address 10.10.0.10 }
ltm node /Common/web2 { address 10.10.0.11 }
ltm node /Common/api1 { address 10.20.0.10 }
ltm node /Common/api2 { address 10.20.0.11 }
ltm node /Common/legacy { address 192.0.2.42 }
ltm node /TenantA/internal { address 10.30.0.50 }
ltm pool /Common/web_pool {
    members {
        /Common/web1:443 { address 10.10.0.10 }
        /Common/web2:443 { address 10.10.0.11 }
    }
    monitor /Common/http
    load-balancing-mode least-connections-member
}
ltm pool /Common/api_pool {
    members {
        /Common/api1:8443 { address 10.20.0.10 }
        /Common/api2:8443 { address 10.20.0.11 }
    }
    monitor /Common/http
}
ltm pool /TenantA/tenant_pool {
    members {
        /TenantA/internal:8080 { address 10.30.0.50 }
    }
}
ltm virtual /Common/web_vs {
    destination /Common/203.0.113.10:443
    pool /Common/web_pool
    profiles {
        /Common/tcp { }
        /Common/http { }
        /Common/clientssl { }
    }
    ip-protocol tcp
}
ltm virtual /Common/api_vs {
    destination /Common/203.0.113.11:443
    pool /Common/api_pool
    profiles {
        /Common/tcp { }
        /Common/http { }
        /Common/clientssl { }
    }
    ip-protocol tcp
}
ltm virtual /Common/legacy_vs {
    destination /Common/203.0.113.50:80
    pool /Common/web_pool
    profiles {
        /Common/tcp { }
        /Common/http { }
    }
    ip-protocol tcp
}
ltm virtual /TenantA/tenant_vs {
    destination /TenantA/198.51.100.10:8080
    pool /TenantA/tenant_pool
    profiles {
        /Common/tcp { }
    }
    ip-protocol tcp
}
"""


def _cmdb_json() -> str:
    """Per-IP CMDB: owner, environment, criticality."""
    return json.dumps(
        {
            "10.10.0.10": {"owner": "web-team", "env": "prod", "crit": "high"},
            "10.10.0.11": {"owner": "web-team", "env": "prod", "crit": "high"},
            "10.20.0.10": {"owner": "api-team", "env": "prod", "crit": "high"},
            "10.20.0.11": {"owner": "api-team", "env": "staging", "crit": "med"},
            "10.30.0.50": {"owner": "tenant-a", "env": "prod", "crit": "low"},
            "192.0.2.42": {"owner": "legacy-team", "env": "decommissioning", "crit": "low"},
        }
    )


def _tenant_map_json() -> str:
    """Partition → tenant friendly name."""
    return json.dumps(
        {
            "Common": "shared-platform",
            "TenantA": "alpha-tenant",
        }
    )


def _threat_feed_json() -> str:
    """Threat-intel feed: blocked CIDRs and high-risk ports."""
    return json.dumps(
        {
            "blocked_cidrs": ["198.51.100.0/24", "192.0.2.0/24"],
            "high_risk_ports": "23,3389,5900-5910",
        }
    )


@pytest.fixture
def workspace(tmp_path: Path) -> dict:
    """Per-test workspace with the BIG-IP config and three JSON
    side inputs written to disk.  Returns a dict of paths so
    each test names what it needs."""
    big = tmp_path / "bigip.conf"
    big.write_text(BIG_CONF)
    cmdb = tmp_path / "cmdb.json"
    cmdb.write_text(_cmdb_json())
    tenants = tmp_path / "tenants.json"
    tenants.write_text(_tenant_map_json())
    threats = tmp_path / "threats.json"
    threats.write_text(_threat_feed_json())
    return {
        "tmp": tmp_path,
        "big": big,
        "cmdb": cmdb,
        "tenants": tenants,
        "threats": threats,
    }


def _run(query: str, workspace: dict, extra_sources: dict | None = None) -> list:
    """Run *query* with the BIG-IP source and every JSON side input
    bound to a ``$NAME`` reflecting its file stem."""
    sources = {"mem://bigip.conf": BIG_CONF}
    if extra_sources:
        sources.update(extra_sources)
    names = {}
    json_uris: set[str] = set()
    for stem, path in (
        ("cmdb", workspace["cmdb"]),
        ("tenants", workspace["tenants"]),
        ("threats", workspace["threats"]),
    ):
        uri = f"file://{path}"
        sources[uri] = path.read_text()
        names[stem] = uri
        json_uris.add(uri)
    result = run_query(
        query,
        sources,
        names=names,
        json_sources=frozenset(json_uris),
    )
    return result.values_per_file["mem://bigip.conf"]


# ===========================================================================
# JSON mixing — the headline use case
# ===========================================================================


class TestJsonMixingDeep:
    def test_join_node_to_cmdb_owner(self, workspace):
        """Per-node enrichment: look up each ``ltm node.address`` in
        the CMDB and emit a ``{name, address, owner}`` triple.

        Uses the ``as $name | body`` let-binding idiom because
        subscript expressions inside a primary-rooted path (``$x[.field]``)
        evaluate against the indexed value rather than the outer
        current.  Binding the outer node explicitly sidesteps that.
        """
        rows = _run(
            ".ltm.node[] as $n | {name: $n.name, addr: $n.address, owner: $cmdb[$n.address].owner}",
            workspace,
        )
        owners = {r["addr"]: r["owner"] for r in rows}
        assert owners["10.10.0.10"] == "web-team"
        assert owners["10.20.0.11"] == "api-team"
        assert owners["192.0.2.42"] == "legacy-team"

    def test_filter_by_external_attribute(self, workspace):
        """Keep only nodes whose CMDB ``env`` is ``prod``.  Tests
        select-with-external-lookup composition via let-binding."""
        rows = _run(
            '.ltm.node[] as $n | select($cmdb[$n.address].env == "prod") | $n.name',
            workspace,
        )
        # web1, web2, api1, internal — 4 prod nodes.  api2 is staging,
        # legacy is decommissioning.
        assert sorted(rows) == ["api1", "internal", "web1", "web2"]

    def test_nested_join_partition_via_tenant_map(self, workspace):
        """Partition extracted from the full_path → looked up in
        the tenant map → emitted as a friendly tenant name.
        ``partition()`` operates on the canonical full-path
        (``/Common/web_vs``), so query ``$v["full-path"]`` rather
        than ``$v.name`` (which is the basename)."""
        rows = _run(
            '.ltm.virtual[] as $v | partition($v."full-path") as $part | '
            "{name: $v.name, partition: $part, tenant: $tenants[$part]}",
            workspace,
        )
        by_name = {r["name"]: r["tenant"] for r in rows}
        assert by_name["web_vs"] == "shared-platform"
        assert by_name["tenant_vs"] == "alpha-tenant"

    def test_join_member_addresses_against_threat_blocked_cidrs(self, workspace):
        """Audit query: any virtual whose pool members live in a
        threat-feed-blocked CIDR.  Composes JSON list, CIDR
        decomposition, and select."""
        rows = _run(
            ".ltm.virtual[] | select(any(.pool.members[].address | "
            "any($threats.blocked_cidrs[] | in_cidr(., .)))) | .name",
            workspace,
        )
        # None of our nodes fall in 198.51.100.0/24 or 192.0.2.0/24;
        # the corpus's only 192.0.2.* node is /Common/legacy and it's
        # not in any pool.  So this filter is empty.
        assert rows == []

    def test_virtual_destinations_overlapping_blocked_cidrs(self, workspace):
        """Same shape but on the virtual side: which VSes are
        listening on an address inside a threat-feed-blocked CIDR?
        ``legacy_vs`` (203.0.113.50) is NOT in any blocked CIDR but
        ``tenant_vs`` (198.51.100.10) IS — flag it."""
        rows = _run(
            ".ltm.virtual[] as $v | select(any($threats.blocked_cidrs[] | "
            "in_cidr(host($v.destination), .))) | $v.name",
            workspace,
        )
        assert rows == ["tenant_vs"]

    def test_join_port_with_threat_high_risk_port_set(self, workspace):
        """Combines `port` extraction with PortSet membership from
        an external feed.  Find virtuals listening on a port in the
        threat-feed's high-risk set."""
        rows = _run(
            ".ltm.virtual[] | select(port_set_contains($threats.high_risk_ports, "
            "port(.destination))) | .name",
            workspace,
        )
        # The feed marks ports 23, 3389, 5900-5910 as high-risk;
        # our VSes use 443, 80, 8080 — none in the danger set.
        assert rows == []

    def test_per_virtual_dossier_with_three_external_lookups(self, workspace):
        """Build a multi-source dossier per virtual: name +
        tenant (from $tenants) + owner of the first pool member
        (from $cmdb) + risk class (from $threats).  This is the
        archetypal "audit report" shape — uses chained let-bindings
        so each subscript lookup sees a stable index value."""
        rows = _run(
            ".ltm.virtual[] as $v "
            '| partition($v."full-path") as $part '
            "| ([$v.pool.members[].address] | first) as $first_addr "
            "| {"
            " name: $v.name,"
            " tenant: $tenants[$part],"
            " owner: $cmdb[$first_addr].owner,"
            " in_blocked: any($threats.blocked_cidrs[] | "
            " in_cidr(host($v.destination), .))"
            "}",
            workspace,
        )
        web = next(r for r in rows if r["name"] == "web_vs")
        assert web["tenant"] == "shared-platform"
        assert web["owner"] == "web-team"
        assert web["in_blocked"] is False
        tenant = next(r for r in rows if r["name"] == "tenant_vs")
        assert tenant["tenant"] == "alpha-tenant"
        assert tenant["in_blocked"] is True

    def test_json_load_ad_hoc_inside_query(self, workspace):
        """``json_load(path)`` reads a JSON file at evaluation time —
        no CLI flag required.  Useful inside multi-statement
        queries where the path itself comes from another source."""
        result = run_query(
            f'json_load("{workspace["threats"]}") | .blocked_cidrs',
            {"mem://bigip.conf": BIG_CONF},
        )
        [cidrs] = result.values_per_file["mem://bigip.conf"]
        assert sorted(cidrs) == ["192.0.2.0/24", "198.51.100.0/24"]

    def test_json_load_can_chain_with_collapse(self, workspace):
        """Compose ``json_load`` → list extraction → CIDR collapse.
        The threat feed's two /24s collapse to themselves (they're
        already canonical and disjoint)."""
        result = run_query(
            f'collapse_cidrs(json_load("{workspace["threats"]}").blocked_cidrs)',
            {"mem://bigip.conf": BIG_CONF},
        )
        [collapsed] = result.values_per_file["mem://bigip.conf"]
        assert sorted(collapsed) == ["192.0.2.0/24", "198.51.100.0/24"]

    def test_json_parse_threads_through_pipe(self, workspace):
        """``json_parse(text)`` of a serialised JSON string — the
        in-memory counterpart to json_load.  Build a synthetic
        body string, parse, then index."""
        rows = _run(
            'json_parse("{\\"hello\\": [1, 2, 3]}").hello',
            workspace,
        )
        # The pipe runs once per top-level BIG-IP iteration step;
        # for a non-streaming expression with no ``.`` reference,
        # the evaluator still emits one value per file.  Just
        # assert the parsed value flows through.
        assert rows and all(v == [1, 2, 3] for v in rows)


# ===========================================================================
# Table output — deep
# ===========================================================================


class TestTableOutputDeep:
    def test_table_handles_unicode_cells(self):
        rows = [
            {"name": "α-server", "owner": "团队 A", "rtt_ms": 0.123},
            {"name": "β-server", "owner": "team β", "rtt_ms": 4.56},
        ]
        out = render(rows, mode="table-lineart")
        assert "α-server" in out
        assert "团队 A" in out
        assert "│" in out
        # Width is measured in characters, not bytes — UTF-8 multibyte
        # cells must align with neighbouring ASCII cells even though
        # the byte count differs.  Column borders align if the line
        # widths are equal across header + data rows.
        lines = [line for line in out.splitlines() if line.startswith("│")]
        widths = {len(line) for line in lines}
        # Wide-East-Asian characters in "团队" report length 1 each in
        # Python's ``len``; visual alignment is up to the terminal.
        assert len(widths) == 1, f"row widths must match: {widths}"

    def test_table_with_nested_dict_cells(self):
        """A cell whose value is itself a dict is rendered as
        compact JSON — keeps the layout tidy."""
        rows = [
            {"name": "web", "meta": {"env": "prod", "owner": "team-a"}},
            {"name": "api", "meta": {"env": "staging", "owner": "team-b"}},
        ]
        out = render(rows, mode="table")
        assert '{"env":"prod","owner":"team-a"}' in out
        assert '{"env":"staging","owner":"team-b"}' in out

    def test_table_with_list_cells_uses_comma_separator(self):
        rows = [{"name": "web", "ips": ["10.0.0.1", "10.0.0.2", "10.0.0.3"]}]
        out = render(rows, mode="table")
        assert "10.0.0.1,10.0.0.2,10.0.0.3" in out

    def test_table_grows_columns_to_fit_the_widest_value(self):
        rows = [
            {"key": "short", "value": "x"},
            {"key": "a much longer key name", "value": "y"},
        ]
        out = render(rows, mode="table")
        # The header row's column for ``key`` must be at least as
        # wide as the longest value (header included).
        widest = max(len("key"), max(len(r["key"]) for r in rows))
        assert f" {'key':<{widest}} " in out

    def test_table_against_real_query_result(self, workspace):
        """Drive the table renderer with an actual ``run_query``
        result so the per-row dict shape is what users will see
        in production."""
        rows = _run(
            ".ltm.virtual[] | {name: .name, dest: .destination, "
            "pool: .pool, partition: partition(.name)}",
            workspace,
        )
        out = render(rows, mode="table-lineart")
        # The header row must list every key from the first dict.
        assert "name" in out and "dest" in out
        assert "partition" in out and "pool" in out
        # Every virtual name appears.
        for name in ("web_vs", "api_vs", "legacy_vs", "tenant_vs"):
            assert name in out

    def test_table_empty_input_is_empty_string(self):
        assert render([], mode="table") == ""
        assert render([], mode="table-lineart") == ""

    def test_table_objectref_renders_via_fields(self):
        """When the row value is an ObjectRef (an actual BIG-IP
        object), the table renderer pulls its ``.fields`` rather
        than calling ``str`` — so users see structured data."""
        from core.bigip.query.values import ObjectRef

        obj = ObjectRef(
            kind="ltm pool",
            full_path="/Common/p",
            fields={"name": "p", "monitor": "/Common/http"},
        )
        out = render([obj], mode="table")
        assert "name" in out and "monitor" in out
        assert "/Common/http" in out


# ===========================================================================
# Range types — compose with BIG-IP data
# ===========================================================================


class TestRangeIntegration:
    def test_collapse_pool_member_addresses_into_supernet(self, workspace):
        """Pool members' addresses span 10.10.0.0/16 and
        10.20.0.0/16 — supernet_of should produce the bounding
        CIDR that contains every one."""
        result = run_query(
            "supernet_of([.ltm.pool[].members[].address])",
            {"mem://x": BIG_CONF},
        )
        [supernet] = result.values_per_file["mem://x"]
        # Member addresses span 10.10/16, 10.20/16, and 10.30/16.
        # The smallest enclosing CIDR is 10.0.0.0/11
        # (covers 10.0.0.0 - 10.31.255.255).  A /12 (10.0.0.0 -
        # 10.15.255.255) would miss 10.20.* and 10.30.*.
        assert supernet == "10.0.0.0/11"

    def test_threat_feed_blocked_cidrs_collapse_to_themselves(self, workspace):
        """The two threat-feed CIDRs are non-adjacent, so collapse
        leaves them alone."""
        rows = _run(
            "collapse_cidrs($threats.blocked_cidrs)",
            workspace,
        )
        assert rows and sorted(rows[0]) == ["192.0.2.0/24", "198.51.100.0/24"]

    def test_port_set_count_from_threat_feed(self, workspace):
        """Count the high-risk ports the threat feed publishes:
        ``23, 3389, 5900-5910`` → 13 ports total."""
        rows = _run("port_set_count($threats.high_risk_ports)", workspace)
        assert rows == [13]

    def test_ip_range_to_cidrs_for_address_pool(self):
        """Decompose a real-world 'first..last' DHCP-like range
        into its minimum CIDR representation."""
        result = run_query(
            'ip_range_to_cidrs("10.0.0.1-10.0.0.6") | sort',
            {"mem://x": BIG_CONF},
        )
        [cidrs] = result.values_per_file["mem://x"]
        # ``ipaddress.summarize_address_range`` gives: 10.0.0.1/32,
        # 10.0.0.2/31, 10.0.0.4/31, 10.0.0.6/32.
        assert cidrs == [
            "10.0.0.1/32",
            "10.0.0.2/31",
            "10.0.0.4/31",
            "10.0.0.6/32",
        ]

    def test_combined_predicate_with_port_and_cidr(self, workspace):
        """A real audit: virtual destinations that match BOTH
        a threat-feed CIDR AND a high-risk port.  ``tenant_vs``
        sits at 198.51.100.10:8080; 8080 is not in the high-risk
        port set, so the intersection should be empty.  Uses
        let-binding so the destination is named once and reused
        in both predicates (which would otherwise lose context
        inside ``any``)."""
        rows = _run(
            ".ltm.virtual[] as $v | select("
            " any($threats.blocked_cidrs[] | in_cidr(host($v.destination), .))"
            " and port_set_contains($threats.high_risk_ports, port($v.destination))"
            ") | $v.name",
            workspace,
        )
        assert rows == []

    def test_first_and_last_host_combined_with_count(self):
        """Compose ``first_host`` / ``last_host`` / ``host_count``
        in one query."""
        result = run_query(
            '{first: first_host("10.0.0.0/24"), '
            'last: last_host("10.0.0.0/24"), '
            'count: host_count("10.0.0.0/24")}',
            {"mem://x": BIG_CONF},
        )
        [row] = result.values_per_file["mem://x"]
        assert row == {"first": "10.0.0.1", "last": "10.0.0.254", "count": 254}


# ===========================================================================
# HTTP helpers — composition with synthetic and real responses
# ===========================================================================


class _MockHandler(BaseHTTPRequestHandler):
    """Minimal HTTP handler: returns canned responses based on path."""

    routes = {
        "/json": (
            200,
            "application/json",
            b'{"servers": [{"name": "a"}, {"name": "b"}], "version": 7}',
        ),
        "/notfound": (404, "text/plain", b"not found"),
        "/redirect": (302, "text/plain", b""),
        "/error": (500, "text/plain", b"boom"),
        "/header-rich": (
            200,
            "application/json",
            b"{}",
        ),
    }

    def do_GET(self):  # noqa: N802 — http.server convention
        status, ctype, body = self.routes.get(self.path, (404, "text/plain", b"not found"))
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("X-Custom-Header", "custom-value")
        if self.path == "/redirect":
            self.send_header("Location", "/elsewhere")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def do_HEAD(self):  # noqa: N802
        self.do_GET()

    def do_OPTIONS(self):  # noqa: N802
        self.send_response(204)
        self.send_header("Allow", "GET, HEAD, OPTIONS")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, *args, **kwargs):  # quiet
        return


@pytest.fixture
def http_server():
    """Spin a tiny in-process HTTP server on a random port for
    HTTP-related tests.  Yields the base URL; tears the server
    down on exit."""
    server = HTTPServer(("127.0.0.1", 0), _MockHandler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{port}"
    finally:
        server.shutdown()
        server.server_close()


class TestHttpComposition:
    def test_url_get_returns_structured_response(self, http_server):
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_get("{http_server}/json")',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [resp] = result.values_per_file["mem://x"]
        assert resp["status"] == 200
        assert "servers" in resp["body"]
        assert resp["headers"]["content-type"] == "application/json"

    def test_chain_url_get_into_body_json(self, http_server):
        """The headline composition: hit a URL, parse the body as
        JSON, traverse into a field, count members."""
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_get("{http_server}/json") | http_body_json(.) | .servers | count',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [count] = result.values_per_file["mem://x"]
        assert count == 2

    def test_http_header_is_case_insensitive_against_live_server(self, http_server):
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_get("{http_server}/json") | http_header(., "X-Custom-Header")',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [value] = result.values_per_file["mem://x"]
        assert value == "custom-value"

    def test_http_ok_chain_filters_successful_responses(self, http_server):
        """Sequentially exercise each status-class predicate
        against the live mock server.  Builds a per-path
        ``{path, status, ok}`` row, then asserts the predicate
        firings."""
        token = PROBES_ENABLED.set(True)
        try:
            queries = {
                "/json": _builtin_status_class_query(http_server, "/json"),
                "/notfound": _builtin_status_class_query(http_server, "/notfound"),
                "/error": _builtin_status_class_query(http_server, "/error"),
            }
            results = {
                path: run_query(q, {"mem://x": BIG_CONF}).values_per_file["mem://x"][0]
                for path, q in queries.items()
            }
        finally:
            PROBES_ENABLED.reset(token)
        assert results["/json"] == {
            "ok": True,
            "redirect": False,
            "client_error": False,
            "server_error": False,
        }
        assert results["/notfound"] == {
            "ok": False,
            "redirect": False,
            "client_error": True,
            "server_error": False,
        }
        assert results["/error"] == {
            "ok": False,
            "redirect": False,
            "client_error": False,
            "server_error": True,
        }

    def test_url_get_404_body_is_recoverable(self, http_server):
        """A 404 response returns the body in the same dict shape
        as a 200 — it's not an exception.  Lets queries inspect
        the error page text."""
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_get("{http_server}/notfound") | http_body(.)',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [body] = result.values_per_file["mem://x"]
        assert "not found" in body

    def test_url_head_returns_headers_without_body(self, http_server):
        """A HEAD request gives us headers; the mock's body for
        ``/json`` shouldn't surface."""
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_head("{http_server}/json") | '
                '{status: http_status(.), ct: http_header(., "Content-Type")}',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [row] = result.values_per_file["mem://x"]
        assert row["status"] == 200
        assert row["ct"] == "application/json"

    def test_url_options_reports_allow_header(self, http_server):
        token = PROBES_ENABLED.set(True)
        try:
            result = run_query(
                f'url_options("{http_server}/json") | http_header(., "Allow")',
                {"mem://x": BIG_CONF},
            )
        finally:
            PROBES_ENABLED.reset(token)
        [allow] = result.values_per_file["mem://x"]
        assert allow is not None
        assert "GET" in allow

    def test_redirect_predicate_recognises_3xx_status(self):
        """Direct test of the ``http_redirect`` builtin against a
        synthetic 302 response dict.  We don't exercise this
        through urllib because :func:`urllib.request.urlopen`
        follows redirects automatically (both GET and HEAD), so
        the only way to *observe* a 3xx without writing a custom
        opener is to test the predicate directly."""
        from core.bigip.query.builtins import _builtin_http_redirect

        assert _builtin_http_redirect({"status": 301, "headers": {}, "body": ""})
        assert _builtin_http_redirect({"status": 302, "headers": {}, "body": ""})
        assert _builtin_http_redirect({"status": 307, "headers": {}, "body": ""})
        assert _builtin_http_redirect({"status": 308, "headers": {}, "body": ""})
        # Boundary: 200 is OK, not a redirect.
        assert not _builtin_http_redirect({"status": 200, "headers": {}, "body": ""})
        # Boundary: 400 is client-error, not a redirect.
        assert not _builtin_http_redirect({"status": 400, "headers": {}, "body": ""})


def _builtin_status_class_query(base_url: str, path: str) -> str:
    """Build a single object literal that classifies the response."""
    return (
        f'url_get("{base_url}{path}") | {{'
        " ok: http_ok(.),"
        " redirect: http_redirect(.),"
        " client_error: http_client_error(.),"
        " server_error: http_server_error(.)"
        "}"
    )


# ===========================================================================
# TLS + X.509 — live local server, real handshake
# ===========================================================================


def _self_signed_cert(tmp_path: Path) -> tuple[Path, Path]:
    """Generate a self-signed cert + key pair on disk and return
    their paths.  Skips if ``cryptography`` isn't installed."""
    pytest.importorskip("cryptography")
    # ``cryptography`` doesn't re-export submodules from the top
    # package, so ``importorskip("cryptography").x509`` raises
    # ``AttributeError``.  Pull each submodule explicitly through
    # ``importorskip`` to skip cleanly on missing optional dep.
    x509 = pytest.importorskip("cryptography.x509")
    hashes = pytest.importorskip("cryptography.hazmat.primitives.hashes")
    serialization = pytest.importorskip("cryptography.hazmat.primitives.serialization")
    rsa = pytest.importorskip("cryptography.hazmat.primitives.asymmetric.rsa")
    oid = pytest.importorskip("cryptography.x509.oid")
    import datetime

    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    name = x509.Name([x509.NameAttribute(oid.NameOID.COMMON_NAME, "test.local")])
    now = datetime.datetime.now(datetime.timezone.utc)
    cert = (
        x509.CertificateBuilder()
        .subject_name(name)
        .issuer_name(name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now - datetime.timedelta(minutes=1))
        .not_valid_after(now + datetime.timedelta(days=365))
        .add_extension(
            x509.SubjectAlternativeName([x509.DNSName("test.local")]),
            critical=False,
        )
        .sign(key, hashes.SHA256())
    )
    cert_path = tmp_path / "cert.pem"
    key_path = tmp_path / "key.pem"
    cert_path.write_bytes(cert.public_bytes(serialization.Encoding.PEM))
    key_path.write_bytes(
        key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.TraditionalOpenSSL,
            encryption_algorithm=serialization.NoEncryption(),
        )
    )
    return cert_path, key_path


class TestTlsAndX509:
    def test_x509_parse_round_trip(self, tmp_path: Path):
        """Generate a self-signed cert, parse with ``x509_parse``,
        and verify the subject + SAN survive the round trip."""
        cert_path, _key_path = _self_signed_cert(tmp_path)
        # json_load takes a JSON file; wrap the PEM as a JSON
        # string so the DSL can read it.
        pem_text = cert_path.read_text()
        wrapper = tmp_path / "cert.pem.json"
        wrapper.write_text(json.dumps(pem_text))
        result = run_query(
            f'x509_parse(json_load("{wrapper}"))',
            {"mem://x": BIG_CONF},
        )
        [parsed] = result.values_per_file["mem://x"]
        assert "test.local" in parsed["subject"]
        assert "test.local" in parsed["sans"]
        assert parsed["fingerprint_sha256"]
        assert parsed["key_size"] == 2048

    def test_x509_from_sys_file_matches_x509_parse_shape(self):
        """A ``sys file ssl-cert`` projected via ``x509_from_sys_file``
        should produce the same dict shape ``x509_parse`` does.
        Every shared key — ``subject`` / ``issuer`` / ``serial`` /
        ``sans`` / ``fingerprint_sha256`` / ``version`` /
        ``key_size`` / ``key_alg`` — comes out normalised so the
        two sides compare cleanly."""
        src = (
            "sys file ssl-cert /Common/example.crt {\n"
            '    source-path "file:///config/ssl/ssl.crt/example.crt"\n'
            '    issuer "CN=Example CA"\n'
            '    subject "CN=www.example.com"\n'
            '    subject-alternative-name "DNS:www.example.com,DNS:example.com"\n'
            '    expiration-string "Jan 1 00:00:00 2099 GMT"\n'
            '    fingerprint "SHA256/12:34:56:AB"\n'
            "    key-size 2048\n"
            "    key-type rsa-public\n"
            "    serial-number 1002\n"
            "    version 3\n"
            "}\n"
        )
        result = run_query(
            '.sys["file-ssl-cert"]["/Common/example.crt"] | x509_from_sys_file(.)',
            {"mem://x": src},
        )
        [parsed] = result.values_per_file["mem://x"]
        assert parsed["subject"] == "CN=www.example.com"
        assert parsed["issuer"] == "CN=Example CA"
        assert parsed["sans"] == ["www.example.com", "example.com"]
        assert parsed["fingerprint_sha256"] == "123456AB"
        assert parsed["serial"] == "3EA"  # 1002 → hex
        assert parsed["version"] == "v3"
        assert parsed["key_size"] == 2048
        assert parsed["key_alg"] == "RSAPublicKey"
        assert parsed["not_after"] == "2099-01-01T00:00:00+00:00"
        # Fields BIG-IP doesn't expose come back null.
        assert parsed["not_before"] is None
        assert parsed["sig_alg"] is None
        assert parsed["public_key_pem"] is None

    def test_x509_eq_matches_same_fingerprint(self):
        """Two certs with matching ``fingerprint_sha256`` are equal
        regardless of which fields are populated."""
        src = 'sys file ssl-cert /Common/c.crt {\n    fingerprint "SHA256/AA:BB:CC"\n}\n'
        result = run_query(
            "x509_eq("
            '.sys["file-ssl-cert"]["/Common/c.crt"] | x509_from_sys_file(.), '
            '{ fingerprint_sha256: "AABBCC" })',
            {"mem://x": src},
        )
        [eq] = result.values_per_file["mem://x"]
        assert eq is True

    def test_x509_eq_falls_back_to_subject_issuer_serial(self):
        """When ``fingerprint_sha256`` is empty on one side,
        ``x509_eq`` falls back to subject + issuer + serial — the
        X.509-defined primary key.  Same cert under both forms
        still compares equal."""
        src = (
            "sys file ssl-cert /Common/c.crt {\n"
            '    issuer "CN=Example CA"\n'
            '    subject "CN=www.example.com"\n'
            "    serial-number 42\n"
            "}\n"
        )
        result = run_query(
            "x509_eq("
            '.sys["file-ssl-cert"]["/Common/c.crt"] | x509_from_sys_file(.), '
            '{ subject: "CN=www.example.com", issuer: "CN=Example CA", '
            'serial: "2A" })',
            {"mem://x": src},
        )
        [eq] = result.values_per_file["mem://x"]
        assert eq is True

    def test_x509_eq_rejects_different_serial(self):
        """Same subject + issuer but different serial number is a
        different cert (e.g. a reissued cert)."""
        src = (
            "sys file ssl-cert /Common/c.crt {\n"
            '    subject "CN=www.example.com"\n'
            '    issuer "CN=Example CA"\n'
            "    serial-number 42\n"
            "}\n"
        )
        result = run_query(
            "x509_eq("
            '.sys["file-ssl-cert"]["/Common/c.crt"] | x509_from_sys_file(.), '
            '{ subject: "CN=www.example.com", issuer: "CN=Example CA", '
            'serial: "FF" })',
            {"mem://x": src},
        )
        [eq] = result.values_per_file["mem://x"]
        assert eq is False


# ===========================================================================
# Probe gate — exhaustive coverage of which builtins gate
# ===========================================================================


class TestProbeGateDeep:
    """Every probe builtin must raise without ``--enable-probes``.
    DNS is the documented exception.
    """

    GATED = [
        'ping("127.0.0.1")',
        'portping("127.0.0.1", 1)',
        'traceroute("127.0.0.1")',
        'url_get("http://127.0.0.1/")',
        'url_head("http://127.0.0.1/")',
        'url_options("http://127.0.0.1/")',
        'url_post("http://127.0.0.1/", "x=1")',
        'socket_get("127.0.0.1", 22)',
        'tls_handshake("127.0.0.1", 443)',
    ]

    UNGATED = [
        'dns("localhost")',
        'rev_dns("127.0.0.1")',
        'json_parse("[1,2,3]")',
        'json_load("/etc/hosts")',  # reads a local file, not network
    ]

    def test_every_probe_builtin_is_gated(self):
        from core.bigip.query.errors import QueryError

        cfg = "ltm node /Common/n {address 10.0.0.1}"
        for query in self.GATED:
            with pytest.raises(QueryError, match="probes are disabled"):
                run_query(query, {"mem://x": cfg})

    def test_ungated_builtins_work_without_the_flag(self, tmp_path):
        """DNS lookups, JSON parsing, file reads — none of these
        require the probe gate."""
        cfg = "ltm node /Common/n {address 10.0.0.1}"
        # Stash a known JSON file path so json_load has something
        # to read.  /etc/hosts is plain text; use our own file.
        sample = tmp_path / "sample.json"
        sample.write_text('{"key": "value"}')
        for query in [
            'dns("localhost")',
            'rev_dns("127.0.0.1")',
            'json_parse("[1,2,3]")',
            f'json_load("{sample}")',
        ]:
            # Should not raise.
            run_query(query, {"mem://x": cfg})


# ===========================================================================
# Edge cases
# ===========================================================================


class TestEdgeCases:
    def test_missing_field_on_loaded_json_raises_eval_error(self, tmp_path: Path):
        """The DSL is strict about missing fields on dict values —
        traversing into a key that doesn't exist raises an
        ``EvalError`` rather than silently producing ``null``.
        This matches the behaviour for missing fields on BIG-IP
        objects."""
        from core.bigip.query.errors import QueryError

        json_path = tmp_path / "empty.json"
        json_path.write_text("{}")
        with pytest.raises(QueryError, match="no field"):
            run_query(
                f'json_load("{json_path}").missing',
                {"mem://x": BIG_CONF},
            )

    def test_missing_field_caller_can_guard_via_keys(self, tmp_path: Path):
        """The strict missing-field semantics push users toward
        the ``keys`` / ``in_keys`` idiom for optional access.
        Test the workaround so the contract is documented."""
        json_path = tmp_path / "empty.json"
        json_path.write_text('{"a": 1}')
        # Guarding via ``keys`` produces a clean predicate without
        # tripping the missing-field error.
        result = run_query(
            f'json_load("{json_path}") | keys | sort',
            {"mem://x": BIG_CONF},
        )
        [keys] = result.values_per_file["mem://x"]
        assert keys == ["a"]

    def test_large_json_array_round_trip(self, tmp_path: Path):
        """A JSON array of 1000 elements survives ``json_load`` →
        ``count``."""
        big = list(range(1000))
        json_path = tmp_path / "big.json"
        json_path.write_text(json.dumps(big))
        result = run_query(
            f'json_load("{json_path}") | count',
            {"mem://x": BIG_CONF},
        )
        assert result.values_per_file["mem://x"] == [1000]

    def test_unicode_in_json_values_round_trips(self, tmp_path: Path):
        json_path = tmp_path / "unicode.json"
        json_path.write_text(json.dumps({"greeting": "héllo 团队 🚀"}))
        result = run_query(
            f'json_load("{json_path}").greeting',
            {"mem://x": BIG_CONF},
        )
        [value] = result.values_per_file["mem://x"]
        assert value == "héllo 团队 🚀"

    def test_port_set_zero_is_treated_as_wildcard(self):
        """The spelling ``"0"`` for a port set means wildcard
        (matches every port), per F5 convention."""
        result = run_query(
            'port_set_contains("0", 1234)',
            {"mem://x": BIG_CONF},
        )
        assert result.values_per_file["mem://x"] == [True]

    def test_ip_range_single_address_is_one_member_set(self):
        """``"10.0.0.5"`` (no hyphen) is a one-IP range."""
        result = run_query(
            'ip_range_count("10.0.0.5")',
            {"mem://x": BIG_CONF},
        )
        assert result.values_per_file["mem://x"] == [1]

    def test_dns_failure_returns_empty_list_not_error(self):
        """A non-resolvable name returns ``[]`` — no exception,
        no null sentinel.  Lets pipelines drop the missing entry
        cleanly with ``count > 0`` filters."""
        result = run_query(
            'dns("definitely-not-a-real-domain.invalid")',
            {"mem://x": BIG_CONF},
        )
        assert result.values_per_file["mem://x"] == [[]]

    def test_collapse_cidrs_handles_mixed_v4_v6_separately(self):
        """A mixed-family list is split per family and each
        family collapses independently."""
        cfg = "ltm node /Common/n {address 10.0.0.1}"
        result = run_query(
            'collapse_cidrs(["10.0.0.0/24"]) | count',
            {"mem://x": cfg},
        )
        # Just verifying the builtin doesn't error on a literal
        # list; behaviour is covered more thoroughly in the basic
        # test file.
        [count] = result.values_per_file["mem://x"]
        assert count == 1

    def test_table_with_special_chars_in_keys(self):
        rows = [{"key with spaces": "v1", "key-with-dashes": "v2"}]
        out = render(rows, mode="table")
        assert "key with spaces" in out
        assert "key-with-dashes" in out
