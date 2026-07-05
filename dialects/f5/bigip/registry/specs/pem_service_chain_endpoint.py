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
            "pem_service_chain_endpoint",
            module="pem",
            object_types=("service-chain-endpoint",),
        ),
        header_types=(("pem", "service-chain-endpoint"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="service-endpoints",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("service-endpoints",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="forwarding-endpoint",
                        value_type="unknown",
                        in_sections=("service-endpoints",),
                    ),
                    BigipPropertySpec(
                        name="from-vlan",
                        value_type="reference",
                        in_sections=("service-endpoints",),
                    ),
                    BigipPropertySpec(
                        name="http-adapt-service",
                        value_type="unknown",
                        in_sections=("service-endpoints",),
                    ),
                    BigipPropertySpec(
                        name="icap-type",
                        value_type="enum",
                        in_sections=("service-endpoints",),
                        allow_none=True,
                        enum_values=("both", "none", "request", "response"),
                    ),
                    BigipPropertySpec(
                        name="internal-virtual",
                        value_type="enum",
                        in_sections=("service-endpoints",),
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="order",
                        value_type="integer",
                        in_sections=("service-endpoints",),
                    ),
                    BigipPropertySpec(
                        name="service-option",
                        value_type="enum",
                        in_sections=("service-endpoints",),
                        enum_values=("mandatory", "optional"),
                    ),
                    BigipPropertySpec(
                        name="steering-policy",
                        value_type="reference",
                        in_sections=("service-endpoints",),
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="to-endpoint",
                        value_type="reference",
                        in_sections=("service-endpoints",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("service-endpoints",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="forwarding-endpoint",
                value_type="unknown",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="from-vlan",
                value_type="reference",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="http-adapt-service",
                value_type="unknown",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="icap-type",
                value_type="enum",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("both", "none", "request", "response"),
            ),
            BigipPropertySpec(
                name="internal-virtual",
                value_type="enum",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="order",
                value_type="integer",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="service-option",
                value_type="enum",
                in_sections=("service-endpoints",),
                enum_values=("mandatory", "optional"),
            ),
            BigipPropertySpec(
                name="steering-policy",
                value_type="reference",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="to-endpoint",
                value_type="reference",
                in_sections=("service-endpoints",),
            ),
        ),
    )
