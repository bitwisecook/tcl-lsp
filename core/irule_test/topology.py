"""SCF topology bridge -- generate Tcl test setup from bigip.conf.

This module uses the existing ``core.bigip.parser`` to parse an SCF/bigip.conf
file and generates the Tcl commands to load the topology into the iRule
test framework.  This is the glue between the Python-side editor tooling
(LSP, AI skills, MCP) and the pure-Tcl test framework.

Usage from AI/agentic context::

    from core.irule_test.topology import TopologyFromSCF

    topo = TopologyFromSCF.from_file("bigip.conf")
    tcl_setup = topo.generate_tcl_setup("/Common/my_vs")
    # tcl_setup is a string of Tcl commands that configure the orchestrator
"""

from __future__ import annotations

from pathlib import Path

from core.bigip.model import BigipConfig, ProfileType
from core.bigip.parser import parse_bigip_conf

# Map our ProfileType enum to the string the Tcl orchestrator expects
_PROFILE_TYPE_TO_TCL: dict[ProfileType, str] = {
    ProfileType.HTTP: "HTTP",
    ProfileType.TCP: "TCP",
    ProfileType.UDP: "UDP",
    ProfileType.CLIENT_SSL: "CLIENTSSL",
    ProfileType.SERVER_SSL: "SERVERSSL",
    ProfileType.FTP: "FTP",
    ProfileType.DNS: "DNS",
    ProfileType.SIP: "SIP",
    ProfileType.DIAMETER: "DIAMETER",
    ProfileType.FIX: "FIX",
    ProfileType.RADIUS: "RADIUS",
    ProfileType.MQTT: "MQTT",
    ProfileType.WEBSOCKET: "WS",
    ProfileType.STREAM: "STREAM",
    ProfileType.HTML: "HTML",
    ProfileType.REWRITE: "REWRITE",
    ProfileType.FASTHTTP: "FASTHTTP",
    ProfileType.FASTL4: "FASTL4",
    ProfileType.ONE_CONNECT: "ONE_CONNECT",
}


class TopologyFromSCF:
    """Parse a BIG-IP SCF config and generate Tcl test setup commands."""

    def __init__(self, config: BigipConfig) -> None:
        self._config = config

    @classmethod
    def from_file(cls, path: str | Path) -> TopologyFromSCF:
        """Parse a bigip.conf / SCF file."""
        source = Path(path).read_text()
        return cls(parse_bigip_conf(source))

    @classmethod
    def from_string(cls, source: str) -> TopologyFromSCF:
        """Parse an SCF config string."""
        return cls(parse_bigip_conf(source))

    @property
    def config(self) -> BigipConfig:
        return self._config

    def virtual_servers(self) -> list[str]:
        """Return all virtual server full paths."""
        return list(self._config.virtual_servers.keys())

    def generate_tcl_setup(self, vs_name: str) -> str:
        """Generate Tcl commands to configure the orchestrator for a VS.

        This produces a complete Tcl script fragment that:
        - Configures profiles
        - Registers pools with members
        - Registers data groups
        - Sets the VIP address/port
        - Loads attached iRules

        The output can be embedded in a test script or written to a file.
        """
        config = self._config
        vs_key = config.resolve_name(vs_name, config.virtual_servers)
        if vs_key is None:
            msg = f"Virtual server {vs_name!r} not found"
            raise KeyError(msg)

        vs = config.virtual_servers[vs_key]
        lines: list[str] = []
        lines.append("# Auto-generated from SCF topology")
        lines.append(f"# Virtual server: {vs_key}")
        lines.append("")

        # Determine profiles
        profile_types = self._resolve_profile_types(vs_key)
        # Ensure TCP if any TCP-based profile is present
        if profile_types and "TCP" not in profile_types and "UDP" not in profile_types:
            profile_types.insert(0, "TCP")

        profiles_tcl = " ".join(profile_types) if profile_types else "TCP HTTP"
        lines.append(f"::orch::configure -profiles {{{profiles_tcl}}}")

        # VIP address/port from destination
        if vs.destination:
            dest_part = vs.destination.rsplit("/", 1)[-1]
            colon = dest_part.rfind(":")
            if colon >= 0:
                addr = dest_part[:colon]
                port = dest_part[colon + 1 :]
                lines.append(f"::orch::configure -local_addr {addr} -local_port {port}")

        lines.append("")

        # Register all pools
        for pool_key, pool in config.pools.items():
            members = [m.name for m in pool.members]
            if members:
                members_tcl = " ".join(members)
                lines.append(f"::orch::add_pool {pool_key} {{{members_tcl}}}")
                # Also register by short name
                short = pool_key.rsplit("/", 1)[-1]
                if short != pool_key:
                    lines.append(f"::orch::add_pool {short} {{{members_tcl}}}")

        if config.pools:
            lines.append("")

        # Register data groups
        for dg_key, dg in config.data_groups.items():
            dg_type = dg.value_type or "string"
            records_parts: list[str] = []
            for rec in dg.records:
                # key -> empty value; use {} so the Tcl list stays even-length
                records_parts.extend([rec, "{}"])
            records_tcl = " ".join(
                f"{{{r}}}" if (" " in r and r != "{}") else r for r in records_parts
            )
            lines.append(f"::orch::add_datagroup {dg_key} {dg_type} {{{records_tcl}}}")
            short = dg_key.rsplit("/", 1)[-1]
            if short != dg_key:
                lines.append(f"::orch::add_datagroup {short} {dg_type} {{{records_tcl}}}")

        if config.data_groups:
            lines.append("")

        # Load attached iRules
        for rule_ref in vs.rules:
            rule_key = config.resolve_rule(rule_ref)
            if rule_key and rule_key in config.rules:
                rule = config.rules[rule_key]
                # Brace-quote the iRule source.  Valid Tcl/iRule source has
                # matched braces; comments with unmatched braces should use
                # backslash-escaped braces (\{ \}) per Tcl convention.
                lines.append(f"::orch::load_irule {{{rule.source}}}")

        lines.append("")
        return "\n".join(lines)

    def generate_full_test_script(self, vs_name: str) -> str:
        """Generate a complete standalone test script for a virtual server.

        Returns a Tcl script that sources the framework, loads the topology,
        and provides a test skeleton that users can customise.
        """
        setup = self.generate_tcl_setup(vs_name)

        script_lines = [
            "#!/usr/bin/env tclsh",
            "#",
            f"# Test script for virtual server: {vs_name}",
            "# Auto-generated -- customise the test scenarios below.",
            "#",
            "",
            "# Source the test framework",
            "set script_dir [file dirname [info script]]",
            "source [file join $script_dir orchestrator.tcl]",
            "source [file join $script_dir scf_loader.tcl]",
            "",
            "# Initialise the framework",
            "::orch::init",
            "",
            "# Topology setup",
            "",
            setup,
            "",
            "# Test scenarios",
            "",
            "# Scenario 1: Basic request",
            '::orch::run_http_request -method GET -uri "/" -host "example.com"',
            "",
            "# Add assertions here:",
            '# ::orch::assert_pool_selected "my_pool"',
            '# ::orch::assert_decision http redirect "https://example.com/"',
            '# ::orch::assert_log_contains "*example*"',
            "",
            "# Print test summary",
            "::orch::summary",
        ]
        return "\n".join(script_lines)

    def _resolve_profile_types(self, vs_key: str) -> list[str]:
        """Resolve profile types for a virtual server."""
        config = self._config
        vs = config.virtual_servers[vs_key]
        types: list[str] = []

        for pref in vs.profiles:
            resolved = config.resolve_profile(pref)
            if resolved and resolved in config.profiles:
                profile = config.profiles[resolved]
                tcl_type = _PROFILE_TYPE_TO_TCL.get(profile.profile_type)
                if tcl_type and tcl_type not in types:
                    types.append(tcl_type)
            else:
                # Infer from name
                inferred = self._infer_profile_type(pref)
                if inferred and inferred not in types:
                    types.append(inferred)

        return types

    @staticmethod
    def _infer_profile_type(pref: str) -> str | None:
        """Infer profile type from a profile reference name."""
        name = pref.rsplit("/", 1)[-1].lower()
        if "clientssl" in name or "client-ssl" in name:
            return "CLIENTSSL"
        if "serverssl" in name or "server-ssl" in name:
            return "SERVERSSL"
        if name in ("http", "http2"):
            return "HTTP"
        if name == "tcp":
            return "TCP"
        if name == "udp":
            return "UDP"
        if "dns" in name:
            return "DNS"
        if "fasthttp" in name:
            return "FASTHTTP"
        return None
