"""scenarios.py — declarative test cases driven by run_tests.py.

Every entry maps to: a ``curl`` invocation that drives traffic at a
specific virtual server in the lab SCF, plus a description of what the
captured pcap is expected to show so an operator (or
``f5 explain-pcap``) can validate the matching scenario.

Hostname → VIP resolution is done via curl's ``--resolve`` so we never
need DNS in the lab.  All VIPs live in 10.255.42.0/24 from the SCF.

If you add a scenario, also drop a corresponding short note in
README.md so users know what to expect.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Scenario:
    """A single explain-pcap lab test case.

    *curl_args* is the trailing argv passed to curl, after the URL.  The
    URL itself is rendered from *vip*, *port*, and *host*.

    *expect_status* is informational — we don't fail the harness on
    mismatch, since the point of several scenarios is to *capture* the
    failure (e.g. TLS handshake reset).  Use ``None`` when curl is
    expected not to return cleanly at all.

    *needs_keylog* tells the runner to set ``SSLKEYLOGFILE`` for this
    invocation so tshark / Wireshark can decrypt the captured TLS.

    *expect_in_explain* is a list of substrings that ``f5 explain-pcap``
    output should contain after running against the captured pcap.
    """

    name: str
    description: str
    vip: str
    port: int
    host: str
    path: str = "/"
    curl_args: tuple[str, ...] = ()
    sni: str = ""  # if empty, defaults to host on TLS scenarios
    https: bool = False
    needs_keylog: bool = False
    expect_status: int | None = None
    expect_in_explain: tuple[str, ...] = field(default_factory=tuple)
    # "automap" means BIG-IP source-NATs to one of its self-IPs; the
    # runner uses this to remind users they can run test_server.py on
    # the same host as run_tests.py without extra plumbing.
    snat: str = "automap"


SCENARIOS: tuple[Scenario, ...] = (
    Scenario(
        name="http_block_by_host",
        description=(
            "iRule blocks requests for blocked.lab.test with HTTP::respond 403. "
            "Captured pcap should show the request line, then a 403 response "
            "originated by BIG-IP (no back-side flow if ``-i 0.0:np`` is used)."
        ),
        vip="10.255.42.100",
        port=80,
        host="blocked.lab.test",
        path="/",
        expect_status=403,
        expect_in_explain=(
            "vs_block",
            "HTTP::host",
            "blocked.lab.test",
            "HTTP::respond",
        ),
    ),
    Scenario(
        name="http_block_admin_path",
        description=(
            "iRule blocks any /admin* path with 403, distinct from the host "
            "block.  Verifies HTTP::path branching in the iRule annotation."
        ),
        vip="10.255.42.100",
        port=80,
        host="app.lab.test",
        path="/admin/login",
        expect_status=403,
        expect_in_explain=("HTTP::path", "/admin/login"),
    ),
    Scenario(
        name="http_policy_rewrite",
        description=(
            "LTM policy rewrites /api/v1/health -> /api/v2/health.  Pool "
            "member should see the rewritten URI."
        ),
        vip="10.255.42.100",
        port=80,
        host="app.lab.test",
        path="/api/v1/health",
        expect_status=200,
        expect_in_explain=("lab_policy_rewrite", "/api/v2/health"),
    ),
    Scenario(
        name="https_sni_default",
        description=(
            "HTTPS request whose SNI (app.lab.test) is NOT api.lab.test, so "
            "the iRule leaves traffic on the default pool.  Captured pcap "
            "shows the ClientHello SNI matched and the back-side flow going "
            "to lab_pool_default."
        ),
        vip="10.255.42.101",
        port=443,
        host="app.lab.test",
        path="/",
        sni="app.lab.test",
        https=True,
        needs_keylog=True,
        expect_status=200,
        expect_in_explain=(
            "vs_sni",
            "CLIENTSSL_CLIENTHELLO",
            "SNI",
            "app.lab.test",
            "lab_pool_default",
        ),
    ),
    Scenario(
        name="https_sni_route_api",
        description=(
            "HTTPS request with SNI=api.lab.test triggers iRule pool "
            "switch to lab_pool_api.  This is the SNI-based load-balancing "
            "decision the user wanted to be able to explain."
        ),
        vip="10.255.42.101",
        port=443,
        host="api.lab.test",
        path="/",
        sni="api.lab.test",
        https=True,
        needs_keylog=True,
        expect_status=200,
        expect_in_explain=(
            "vs_sni",
            "SSL::extensions",
            "api.lab.test",
            "lab_pool_api",
        ),
    ),
    Scenario(
        name="tls_expired_cert",
        description=(
            "VS presents an expired server certificate.  Strict curl will "
            "abort the handshake; BIG-IP may issue an RST with a TLS-related "
            "F5 reset cause once the client tears down.  Run curl with -v to "
            "see the openssl error."
        ),
        vip="10.255.42.102",
        port=443,
        host="valid.lab.test",
        path="/",
        sni="valid.lab.test",
        https=True,
        needs_keylog=False,  # handshake fails; no master secret to log
        curl_args=("--max-time", "5"),
        expect_status=None,
        expect_in_explain=("vs_tls_expired", "RST"),
    ),
    Scenario(
        name="tls_selfsigned_strict",
        description=(
            "Self-signed cert; curl with default verification will reject. "
            "Captured pcap shows the handshake completing past ServerHello "
            "but client sending a TLS bad-certificate alert before tearing "
            "down the connection."
        ),
        vip="10.255.42.103",
        port=443,
        host="selfsigned.lab.test",
        path="/",
        sni="selfsigned.lab.test",
        https=True,
        curl_args=("--max-time", "5"),
        expect_status=None,
        expect_in_explain=("vs_tls_selfsigned", "alert"),
    ),
    Scenario(
        name="tls_selfsigned_relaxed",
        description=(
            "Same VS as tls_selfsigned_strict but curl runs with `-k` so the "
            "handshake completes.  Useful baseline against the strict run.  "
            "needs_keylog so we can decrypt the inner HTTP request."
        ),
        vip="10.255.42.103",
        port=443,
        host="selfsigned.lab.test",
        path="/",
        sni="selfsigned.lab.test",
        https=True,
        needs_keylog=True,
        curl_args=("-k",),
        expect_status=200,
        expect_in_explain=("vs_tls_selfsigned",),
    ),
    Scenario(
        name="pool_down",
        description=(
            "VS whose only pool member is unreachable; BIG-IP resets the "
            "client connection.  The F5 ethernet trailer's RST cause TLV "
            "should carry text mentioning POOL or NO_USABLE_MEMBER."
        ),
        vip="10.255.42.104",
        port=80,
        host="app.lab.test",
        path="/",
        curl_args=("--max-time", "10"),
        expect_status=None,
        expect_in_explain=("vs_pool_down", "RST", "lab_pool_dead"),
    ),
    Scenario(
        name="payload_reset",
        description=(
            "Plain TCP VS that resets when the magic 'RESETME' token "
            "appears in payload.  Drive it with `printf 'RESETME\\n' | nc`. "
            "The harness uses bash netcat for this scenario rather than curl."
        ),
        vip="10.255.42.105",
        port=7777,
        host="",
        path="",
        curl_args=(),  # special-cased in run_tests.py
        expect_status=None,
        expect_in_explain=("vs_payload_reset", "RST", "RESETME"),
    ),
)


def by_name(name: str) -> Scenario | None:
    """Return the scenario with *name*, or ``None``."""
    for s in SCENARIOS:
        if s.name == name:
            return s
    return None
