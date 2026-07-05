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

"""Structural integrity of the event / profile / object graphs.

Read straight from the in-memory registry (no JSON dump): stable event
ordering, well-formed flow chains, and closed reference edges.  Any
front-end — Python today, Rust later — must reproduce a registry that
satisfies these invariants.
"""

from __future__ import annotations

import pytest

from compiler.registry import namespace_data as nd
from compiler.registry.namespace_registry import NAMESPACE_REGISTRY
from dialects.f5.bigip.registry.data import HEADER_KIND_MAP, OBJECT_KIND_SPECS


@pytest.fixture(scope="module")
def master_order() -> list[str]:
    return [event for event, _profiles in nd.MASTER_ORDER]


def test_master_order_is_unique_and_aligned(master_order: list[str]) -> None:
    assert len(master_order) == len(set(master_order)), "duplicate events in MASTER_ORDER"
    for position, event in enumerate(master_order):
        assert NAMESPACE_REGISTRY.event_index(event) == position


def test_every_ordered_event_is_in_master_order(master_order: list[str]) -> None:
    ordered = {
        name
        for name in NAMESPACE_REGISTRY.all_event_names()
        if NAMESPACE_REGISTRY.event_index(name) is not None
    }
    assert ordered == set(master_order)


def test_flow_chains_reference_known_profiles() -> None:
    profile_names = set(nd.PROFILE_SPECS)
    for chain_id, chain in nd.FLOW_CHAINS.items():
        assert chain.chain_id == chain_id
        assert chain.steps
        for profile in chain.profiles:
            assert profile in profile_names, f"{chain_id} -> unknown profile {profile}"


def test_protocol_namespace_profiles_are_known() -> None:
    profile_names = set(nd.PROFILE_SPECS)
    for prefix, spec in nd.PROTOCOL_NAMESPACE_SPECS.items():
        assert spec.prefix == prefix
        for profile in spec.profiles:
            assert profile in profile_names, f"{prefix} -> unknown profile {profile}"


def test_object_kinds_unique_and_header_map_closed() -> None:
    kinds = [spec.kind for spec in OBJECT_KIND_SPECS.values()]
    assert len(kinds) == len(set(kinds)), "duplicate object kinds"
    kind_set = set(kinds)
    for (module, object_type), kind in HEADER_KIND_MAP.items():
        assert kind in kind_set, f"headerKindMap[{module},{object_type}] -> unknown kind {kind}"
