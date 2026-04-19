"""Scenario definitions for BigIP event-ordering tests.

Each scenario describes a virtual server configuration (profiles, IPs,
ports) and its expected event ordering.  The generators (irule_gen,
tmsh_gen, etc.) use these to produce deployable artifacts.

Network addresses are parameterized via :class:`NetworkConfig` so the
harness can be pointed at any lab environment.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from core.commands.registry.namespace_data import FLOW_CHAINS, chain_event_names


@dataclass(frozen=True)
class NetworkConfig:
    """Network addresses for BigIP test deployment.

    Attributes:
        vip_ip: Virtual server destination IP.
        pool_member_ip: Pool member (backend server) IP.
        snat_ip: SNAT address (empty string = automap).
        hsl_receiver_ip: IP where the HSL log receiver listens.
        hsl_receiver_port: UDP port for the HSL log receiver.
    """

    vip_ip: str = "10.1.1.10"
    pool_member_ip: str = "10.1.1.100"
    snat_ip: str = ""
    hsl_receiver_ip: str = "10.1.1.200"
    hsl_receiver_port: int = 5514


@dataclass(frozen=True)
class Scenario:
    """A BigIP test scenario for validating event ordering.

    Attributes:
        name: Short identifier (used in filenames and object names).
        description: Human-readable label.
        chain_id: References a FlowChain in FLOW_CHAINS.
        profiles: BigIP profile config — maps profile type to profile name.
            E.g. ``{"tcp": "tcp", "http": "http", "clientssl": "clientssl"}``.
        vip: Virtual server destination IP.
        vip_port: Virtual server destination port.
        pool_member_ip: Pool member IP address.
        pool_member_port: Pool member port.
        client_protocol: Protocol the client uses: ``"tcp"``, ``"http"``,
            ``"https"``, ``"https_post"``, or ``"dns"``.
        extra_irule_lines: Additional iRule lines to inject (e.g. HTTP::collect).
    """

    name: str
    description: str
    chain_id: str
    profiles: dict[str, str]
    vip: str = "10.1.1.10"
    vip_port: int = 8080
    pool_member_ip: str = "10.1.1.100"
    pool_member_port: int = 9080
    client_protocol: str = "tcp"
    extra_irule_lines: dict[str, str] = field(default_factory=dict)

    @property
    def chain(self):
        return FLOW_CHAINS[self.chain_id]

    @property
    def expected_events(self) -> list[str]:
        """Non-conditional events from the flow chain."""
        return [step.event for step in self.chain.steps if not step.conditional]

    @property
    def all_events(self) -> list[str]:
        """All events from the flow chain (including conditional)."""
        return chain_event_names(self.chain)


# Scenario factory


def build_scenarios(net: NetworkConfig | None = None) -> dict[str, Scenario]:
    """Build all scenarios parameterized with *net* addresses.

    When *net* is ``None``, uses default addresses.
    """
    if net is None:
        net = NetworkConfig()

    return {
        "plain_tcp": Scenario(
            name="plain_tcp",
            description="Plain TCP virtual server",
            chain_id="plain_tcp",
            profiles={"tcp": "tcp"},
            vip=net.vip_ip,
            vip_port=8080,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=9080,
            client_protocol="tcp",
        ),
        "tcp_http": Scenario(
            name="tcp_http",
            description="TCP + HTTP virtual server",
            chain_id="tcp_http",
            profiles={"tcp": "tcp", "http": "http"},
            vip=net.vip_ip,
            vip_port=8081,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=9081,
            client_protocol="http",
        ),
        "tcp_clientssl_http": Scenario(
            name="tcp_clientssl_http",
            description="TCP + ClientSSL + HTTP (client-side TLS)",
            chain_id="tcp_clientssl_http",
            profiles={
                "tcp": "tcp",
                "clientssl": "clientssl",
                "http": "http",
            },
            vip=net.vip_ip,
            vip_port=8443,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=9082,
            client_protocol="https",
        ),
        "tcp_clientssl_serverssl_http": Scenario(
            name="tcp_clientssl_serverssl_http",
            description="Full HTTPS (ClientSSL + ServerSSL + HTTP)",
            chain_id="tcp_clientssl_serverssl_http",
            profiles={
                "tcp": "tcp",
                "clientssl": "clientssl",
                "serverssl": "serverssl",
                "http": "http",
            },
            vip=net.vip_ip,
            vip_port=8444,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=9443,
            client_protocol="https",
        ),
        "tcp_clientssl_serverssl_http_collect": Scenario(
            name="tcp_clientssl_serverssl_http_collect",
            description="Full HTTPS with HTTP::collect",
            chain_id="tcp_clientssl_serverssl_http_collect",
            profiles={
                "tcp": "tcp",
                "clientssl": "clientssl",
                "serverssl": "serverssl",
                "http": "http",
            },
            vip=net.vip_ip,
            vip_port=8445,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=9444,
            client_protocol="https_post",
            extra_irule_lines={
                "HTTP_REQUEST": "    HTTP::collect [HTTP::header Content-Length]",
                "HTTP_REQUEST_DATA": "    HTTP::release",
                "HTTP_RESPONSE": "    HTTP::collect [HTTP::header Content-Length]",
                "HTTP_RESPONSE_DATA": "    HTTP::release",
            },
        ),
        "udp_dns": Scenario(
            name="udp_dns",
            description="UDP + DNS virtual server",
            chain_id="udp_dns",
            profiles={"udp": "udp", "dns": "dns"},
            vip=net.vip_ip,
            vip_port=5353,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=5354,
            client_protocol="dns",
        ),
        "tcp_dns": Scenario(
            name="tcp_dns",
            description="TCP + DNS virtual server (zone transfer / large response)",
            chain_id="tcp_dns",
            profiles={"tcp": "tcp", "dns": "dns"},
            vip=net.vip_ip,
            vip_port=5355,
            pool_member_ip=net.pool_member_ip,
            pool_member_port=5356,
            client_protocol="dns",
        ),
    }


# Default scenarios (backward compatible)
SCENARIOS: dict[str, Scenario] = build_scenarios()
