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

"""Tests for BIG-IP object registry metadata and resolution."""

from __future__ import annotations

from dialects.f5.bigip.object_registry import (
    build_bigip_object_registry,
    candidate_kinds_for_key,
    candidate_kinds_for_section_item,
    kind_for_header,
)


def test_key_pool_in_source_address_translation_prefers_snat_pool():
    kinds = candidate_kinds_for_key(
        "pool",
        section="source-address-translation",
        container_module="ltm",
        container_object_type="virtual",
    )
    assert kinds
    assert kinds[0] == "ltm_snatpool"
    assert "ltm_pool" in kinds


def test_key_defaults_from_in_monitor_stanza_uses_monitor():
    kinds = candidate_kinds_for_key(
        "defaults-from",
        section=None,
        container_module="ltm",
        container_object_type="monitor http",
    )
    assert kinds
    assert kinds[0] == "ltm_monitor_http"


def test_section_members_in_pool_resolves_node_kind():
    kinds = candidate_kinds_for_section_item(
        "members",
        container_module="ltm",
        container_object_type="pool",
    )
    assert kinds == ("ltm_node",)


def test_kind_for_multiword_security_object_header():
    assert kind_for_header("security", "firewall policy") == "security_firewall_policy"


def test_static_registry_contains_transparent_nexthop_rule():
    registry = build_bigip_object_registry()
    kinds = registry.candidate_kinds_for_key(
        "transparent-nexthop",
        section=None,
        container_module="ltm",
        container_object_type="virtual",
    )
    assert "net_vlan" in kinds


def test_gtm_wideip_pools_section_maps_to_pool_kind():
    kinds = candidate_kinds_for_section_item(
        "pools",
        container_module="gtm",
        container_object_type="wideip a",
    )
    assert "gtm_pool_a" in kinds
