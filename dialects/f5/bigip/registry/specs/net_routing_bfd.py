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

from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "net_routing_bfd",
            module="net",
            object_types=("routing bfd",),
        ),
        header_types=(("net", "routing bfd"),),
        properties=(
            BigipPropertySpec(name="gtsm", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="gtsm-ttl", value_type="integer"),
            BigipPropertySpec(
                name="multihop-peer",
                value_type="reference",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                shape_kind="list",
                block=(
                    BigipPropertySpec(
                        name="interval",
                        value_type="boolean",
                        in_sections=("multihop-peer",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="minrx",
                        value_type="boolean",
                        in_sections=("multihop-peer",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="multiplier",
                        value_type="boolean",
                        in_sections=("multihop-peer",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="interval",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="minrx",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="multiplier",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="notification",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="slow-timer", value_type="integer"),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                references=("net_vlan",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                shape_kind="list",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("vlan",),
                        enum_values=("false", "true"),
                    ),
                    BigipPropertySpec(
                        name="interval",
                        value_type="boolean",
                        in_sections=("vlan",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="minrx",
                        value_type="boolean",
                        in_sections=("vlan",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="multiplier",
                        value_type="boolean",
                        in_sections=("vlan",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("vlan",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="interval",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="minrx",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="multiplier",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
        ),
    )
