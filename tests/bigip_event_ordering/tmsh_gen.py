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

"""Generate tmsh configuration for BigIP event-ordering scenarios.

Produces setup and cleanup commands for deploying a virtual server,
pool, iRule, HSL logging pool, and optional SSL profiles to a BIG-IP
device.
"""

from __future__ import annotations

from .irule_gen import generate_irule
from .scenarios import Scenario


def _profile_list(scenario: Scenario) -> str:
    """Build the tmsh profiles clause.

    Maps profile types to their tmsh context syntax.  ClientSSL and
    ServerSSL need explicit ``context clientside`` / ``context serverside``.
    """
    parts: list[str] = []
    for ptype, pname in sorted(scenario.profiles.items()):
        if ptype == "clientssl":
            parts.append(f"{pname} {{ context clientside }}")
        elif ptype == "serverssl":
            parts.append(f"{pname} {{ context serverside }}")
        else:
            parts.append(f"{pname} {{ }}")
    return " ".join(parts)


# HSL pool (shared across all scenarios)


def generate_hsl_pool(
    receiver_ip: str,
    receiver_port: int = 5514,
) -> str:
    """Generate tmsh to create the HSL logging pool."""
    return (
        "# HSL logging pool (UDP to log receiver)"
        "create ltm pool evt_order_hsl_pool {\n"
        f"    members add {{ {receiver_ip}:{receiver_port} }}\n"
        "    monitor none\n"
        "}\n"
    )


def generate_cleanup_hsl() -> str:
    """Generate tmsh to remove the HSL logging pool."""
    return "delete ltm pool evt_order_hsl_pool\n"


# ClientSSL profile with custom certs


def generate_clientssl_profile(
    cert_name: str = "evt_order_clientssl.crt",
    key_name: str = "evt_order_clientssl.key",
    ca_name: str = "evt_order_ca.crt",
    profile_name: str = "evt_order_clientssl",
) -> str:
    """Generate tmsh to create a clientssl profile with uploaded certs.

    The cert/key/CA files must already be uploaded to the BIG-IP
    ``/Common/`` partition (e.g. via ``scp`` to
    ``/var/tmp/`` then ``tmsh install sys crypto cert ...``).
    """
    return (
        f"# ClientSSL profile with custom cert"
        f"create ltm profile client-ssl {profile_name} {{\n"
        f"    cert-key-chain add {{\n"
        f"        default {{\n"
        f"            cert {cert_name}\n"
        f"            key {key_name}\n"
        f"            chain {ca_name}\n"
        f"        }}\n"
        f"    }}\n"
        f"    defaults-from clientssl\n"
        f"}}\n"
    )


def generate_cleanup_clientssl(
    profile_name: str = "evt_order_clientssl",
) -> str:
    """Generate tmsh to remove the custom clientssl profile."""
    return f"delete ltm profile client-ssl {profile_name}\n"


# Per-scenario setup/cleanup


def generate_setup(
    scenario: Scenario,
    *,
    hsl_pool: str = "evt_order_hsl_pool",
) -> str:
    """Generate tmsh commands to create the test objects for a scenario."""
    prefix = f"evt_order_{scenario.name}"
    irule_body = generate_irule(scenario, hsl_pool=hsl_pool)

    lines = [
        f"# Setup: {scenario.description}",
        "",
        "# Pool (monitor none — pool member is a Python script)",
        f"create ltm pool {prefix}_pool {{",
        f"    members add {{ {scenario.pool_member_ip}:{scenario.pool_member_port} }}",
        "    monitor none",
        "}",
        "",
        "# iRule",
        f"create ltm rule {prefix} {{",
        irule_body,
        "}",
        "",
        "# Virtual server",
        f"create ltm virtual {prefix}_vs {{",
        f"    destination {scenario.vip}:{scenario.vip_port}",
        f"    pool {prefix}_pool",
        f"    profiles add {{ {_profile_list(scenario)} }}",
        f"    rules {{ {prefix} }}",
        "    source-address-translation { type automap }",
        "}",
    ]
    return "\n".join(lines) + "\n"


def generate_cleanup(scenario: Scenario) -> str:
    """Generate tmsh commands to remove the test objects for a scenario."""
    prefix = f"evt_order_{scenario.name}"
    lines = [
        f"# Cleanup: {scenario.description}",
        f"delete ltm virtual {prefix}_vs",
        f"delete ltm rule {prefix}",
        f"delete ltm pool {prefix}_pool",
    ]
    return "\n".join(lines) + "\n"


def generate_tmsh(
    scenario: Scenario,
    *,
    hsl_pool: str = "evt_order_hsl_pool",
) -> str:
    """Generate complete tmsh config (setup + cleanup) for a scenario."""
    return generate_setup(scenario, hsl_pool=hsl_pool) + "\n" + generate_cleanup(scenario)
