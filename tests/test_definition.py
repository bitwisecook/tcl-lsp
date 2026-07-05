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

"""BigIP go-to-definition internals.

The Tcl proc/variable go-to-definition behaviour is exercised end-to-end against
the packaged server in ``tests/lsp_e2e/test_definition_e2e.py``.  What remains
here is ``get_bigip_definition`` — driven through the internal BigIP-config API
(parsed ``current_config`` / ``workspace_configs``) with no plain
``textDocument/definition`` surface in the default Tcl path.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.parser import parse_bigip_conf
from server.features.definition import get_bigip_definition


class TestBigipDefinition:
    def test_virtual_pool_jumps_to_pool_object(self):
        source = textwrap.dedent("""\
            ltm pool /Common/web_pool {
                members {
                    /Common/web1:80 { }
                }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                pool /Common/web_pool
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            7,
            14,  # inside /Common/web_pool
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].uri == "file:///bigip.conf"
        # ltm pool stanza is at line 0
        assert locations[0].range.start.line == 0

    def test_virtual_pool_jump_at_end_boundary(self):
        """Cursor at end.character + 1 (just past the pool token) still works."""
        source = textwrap.dedent("""\
            ltm pool /Common/web_pool {
                members {
                    /Common/web1:80 { }
                }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                pool /Common/web_pool
            }
        """)
        cfg = parse_bigip_conf(source)
        # Line 7: "    pool /Common/web_pool"
        # "/Common/web_pool" ends at character 24; place cursor at 25
        # (one past the last character, matching LSP end-exclusive convention).
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            7,
            25,  # one past the end of /Common/web_pool
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].uri == "file:///bigip.conf"
        assert locations[0].range.start.line == 0

    def test_virtual_pool_jumps_to_pool_in_other_bigip_file(self):
        vs_source = textwrap.dedent("""\
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                pool /Common/web_pool
            }
        """)
        pool_source = textwrap.dedent("""\
            ltm pool /Common/web_pool {
                members {
                    /Common/web1:80 { }
                }
            }
        """)
        vs_cfg = parse_bigip_conf(vs_source)
        pool_cfg = parse_bigip_conf(pool_source)
        locations = get_bigip_definition(
            vs_source,
            "file:///bigip.conf",
            2,
            14,  # inside /Common/web_pool
            current_config=vs_cfg,
            workspace_configs={
                "file:///bigip.conf": vs_cfg,
                "file:///bigip_base.conf": pool_cfg,
            },
        )
        assert len(locations) == 1
        assert locations[0].uri == "file:///bigip_base.conf"

    def test_virtual_rules_entry_jumps_to_rule(self):
        source = textwrap.dedent("""\
            ltm rule /Common/route_by_host {
                when HTTP_REQUEST { pool /Common/web_pool }
            }
            ltm virtual /Common/www_vs {
                rules {
                    /Common/route_by_host
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            5,
            18,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_profiles_entry_jumps_to_profile(self):
        source = textwrap.dedent("""\
            ltm profile http /Common/custom_http { defaults-from /Common/http }
            ltm virtual /Common/www_vs {
                profiles {
                    /Common/custom_http { }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            3,
            20,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_pool_member_entry_jumps_to_node(self):
        source = textwrap.dedent("""\
            ltm node /Common/web1 { address 10.0.0.10 }
            ltm pool /Common/web_pool {
                members {
                    /Common/web1:80 { }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            3,
            16,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_rule_body_pool_command_jumps_to_pool(self):
        source = textwrap.dedent("""\
            ltm pool /Common/api_pool { members { /Common/web1:80 { } } }
            ltm rule /Common/r1 {
                when HTTP_REQUEST {
                    pool /Common/api_pool
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            3,
            22,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_rule_body_class_match_jumps_to_data_group(self):
        source = textwrap.dedent("""\
            ltm data-group internal /Common/allowed_hosts {
                records { foo { } }
                type string
            }
            ltm rule /Common/r1 {
                when HTTP_REQUEST {
                    if { [class match [HTTP::host] equals allowed_hosts] } { pool /Common/p1 }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            6,
            46,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_defaults_from_jumps_to_parent_profile(self):
        source = textwrap.dedent("""\
            ltm profile tcp /Common/base_tcp { }
            ltm profile tcp /Common/custom_tcp {
                defaults-from /Common/base_tcp
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            2,
            26,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_route_domain_key_jumps_to_route_domain_object(self):
        source = textwrap.dedent("""\
            net route-domain /Common/0 {
                id 0
            }
            ltm profile socks /Common/custom_socks {
                route-domain /Common/0
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            4,
            24,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_traffic_group_key_jumps_to_cm_traffic_group(self):
        source = textwrap.dedent("""\
            cm traffic-group /Common/traffic-group-1 {
                unit-id 1
            }
            ltm virtual-address /Common/192.0.2.10 {
                traffic-group /Common/traffic-group-1
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            4,
            35,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_policies_entry_jumps_to_policy(self):
        source = textwrap.dedent("""\
            ltm policy /Common/web_policy {
                strategy /Common/first-match
            }
            ltm virtual /Common/www_vs {
                policies {
                    /Common/web_policy { }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            5,
            20,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_destination_jumps_to_virtual_address(self):
        source = textwrap.dedent("""\
            ltm virtual-address /Common/192.0.2.10 {
                traffic-group /Common/traffic-group-1
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                pool /Common/web_pool
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            4,
            25,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_sat_pool_jumps_to_snatpool(self):
        source = textwrap.dedent("""\
            ltm snatpool /Common/sp1 {
                members { 10.0.0.10 }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                source-address-translation {
                    type snat
                    pool /Common/sp1
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            7,
            22,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_self_vlan_key_jumps_to_vlan(self):
        source = textwrap.dedent("""\
            net vlan /Common/internal {
                interfaces { 1.1 { } }
            }
            net self /Common/10.0.0.10/24 {
                address 10.0.0.10/24
                vlan /Common/internal
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            5,
            18,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_auth_user_partition_access_jumps_to_partition(self):
        source = textwrap.dedent("""\
            auth partition /Common/TenantA {
                default-route-domain 0
            }
            auth user admin {
                partition-access {
                    /Common/TenantA { role admin }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            5,
            22,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_net_self_fw_policy_jumps_to_security_firewall_policy(self):
        source = textwrap.dedent("""\
            security firewall policy /Common/fw_pol_1 {
                rules { }
            }
            net self /Common/10.0.0.10/24 {
                address 10.0.0.10/24
                vlan /Common/internal
                fw-enforced-policy /Common/fw_pol_1
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            6,
            34,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_security_nat_policy_jumps_to_security_nat_policy(self):
        source = textwrap.dedent("""\
            security nat policy /Common/nat_pol_1 {
                rules { }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                security-nat-policy {
                    policy /Common/nat_pol_1
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            6,
            24,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_virtual_last_hop_pool_jumps_to_pool(self):
        source = textwrap.dedent("""\
            ltm pool /Common/lh_pool {
                members { /Common/web1:80 { } }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                last-hop-pool /Common/lh_pool
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            5,
            29,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_cm_traffic_group_ha_group_jumps_to_ha_group(self):
        source = textwrap.dedent("""\
            cm ha-group /Common/hag1 {
                pools { }
            }
            cm traffic-group /Common/traffic-group-1 {
                monitor {
                    ha-group /Common/hag1
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            5,
            27,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_cm_traffic_group_ha_order_jumps_to_device(self):
        source = textwrap.dedent("""\
            cm device /Common/bigip-a {
                hostname bigip-a.example.com
            }
            cm traffic-group /Common/traffic-group-1 {
                ha-order {
                    /Common/bigip-a
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            5,
            20,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_net_route_gateway_jumps_to_self(self):
        source = textwrap.dedent("""\
            net self /Common/10.0.0.10/24 {
                address 10.0.0.10/24
                vlan /Common/internal
            }
            net route /Common/default {
                gw /Common/10.0.0.10/24
                network default
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip_base.conf",
            5,
            18,
            current_config=cfg,
            workspace_configs={"file:///bigip_base.conf": cfg},
        )
        assert len(locations) == 1
        assert locations[0].range.start.line == 0

    def test_ltm_pool_reference_does_not_jump_to_gtm_pool(self):
        source = textwrap.dedent("""\
            gtm pool /Common/shared_pool {
                members { /Common/dc1_vs1:80 { } }
            }
            ltm pool /Common/shared_pool {
                members { /Common/web1:80 { } }
            }
            ltm virtual /Common/www_vs {
                destination /Common/192.0.2.10:443
                pool /Common/shared_pool
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            8,
            20,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        # Must resolve to ltm pool (line 3), not gtm pool (line 0).
        assert locations[0].range.start.line == 3

    def test_gtm_wideip_pool_reference_jumps_to_gtm_pool(self):
        source = textwrap.dedent("""\
            gtm pool a /Common/wip_pool {
                members { /Common/dc1_vs1:80 { } }
            }
            ltm pool /Common/wip_pool {
                members { /Common/web1:80 { } }
            }
            gtm wideip a /Common/example.com {
                pools {
                    /Common/wip_pool { ratio 1 }
                }
            }
        """)
        cfg = parse_bigip_conf(source)
        locations = get_bigip_definition(
            source,
            "file:///bigip.conf",
            8,
            20,
            current_config=cfg,
            workspace_configs={"file:///bigip.conf": cfg},
        )
        assert len(locations) == 1
        # Must resolve to gtm pool (line 0), not ltm pool (line 3).
        assert locations[0].range.start.line == 0
