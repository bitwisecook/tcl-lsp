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
            "net_routing_profile_bgp",
            module="net",
            object_types=("routing profile bgp",),
        ),
        header_types=(("net", "routing profile bgp"),),
        properties=(
            BigipPropertySpec(
                name="adj-out",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="aggregate-nexthop-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="as-local-count", value_type="integer"),
            BigipPropertySpec(
                name="bgp-multiple-instance",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="boolean",
                allow_none=True,
                references=("net_routing_profile_bgp",),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="extended-asn-cap",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="max-paths",
                value_type="string",
                block=(
                    BigipPropertySpec(
                        name="ebgp", value_type="integer", in_sections=("max-paths",)
                    ),
                    BigipPropertySpec(
                        name="ibgp", value_type="integer", in_sections=("max-paths",)
                    ),
                ),
            ),
            BigipPropertySpec(name="ebgp", value_type="integer", in_sections=("max-paths",)),
            BigipPropertySpec(name="ibgp", value_type="integer", in_sections=("max-paths",)),
            BigipPropertySpec(
                name="nexthop-trigger",
                value_type="string",
                block=(
                    BigipPropertySpec(
                        name="delay",
                        value_type="integer",
                        in_sections=("nexthop-trigger",),
                    ),
                    BigipPropertySpec(
                        name="state",
                        value_type="enum",
                        in_sections=("nexthop-trigger",),
                        enum_values=("disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(name="delay", value_type="integer", in_sections=("nexthop-trigger",)),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("nexthop-trigger",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="rfc1771",
                value_type="string",
                block=(
                    BigipPropertySpec(
                        name="path-select",
                        value_type="enum",
                        in_sections=("rfc1771",),
                        enum_values=("disabled", "enabled"),
                    ),
                    BigipPropertySpec(
                        name="strict",
                        value_type="enum",
                        in_sections=("rfc1771",),
                        enum_values=("disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="path-select",
                value_type="enum",
                in_sections=("rfc1771",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="strict",
                value_type="enum",
                in_sections=("rfc1771",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="router-id", value_type="integer"),
        ),
    )
