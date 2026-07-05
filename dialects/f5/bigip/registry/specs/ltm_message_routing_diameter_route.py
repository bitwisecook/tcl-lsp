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
            "ltm_message_routing_diameter_route",
            module="ltm",
            object_types=("message-routing diameter route",),
        ),
        header_types=(("ltm", "message-routing diameter route"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="application-id", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-realm", value_type="string", allow_none=True),
            BigipPropertySpec(name="origin-realm", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="peer-selection-mode",
                value_type="enum",
                enum_values=("ratio", "sequential"),
            ),
            BigipPropertySpec(
                name="peers",
                value_type="list",
                repeated=True,
                references=(
                    "ltm_message_routing_diameter_peer",
                    "ltm_message_routing_generic_peer",
                    "ltm_message_routing_mqtt_peer",
                    "ltm_message_routing_sip_peer",
                    "net_ipsec_ike_peer",
                ),
            ),
            BigipPropertySpec(
                name="virtual-server",
                value_type="reference",
                default="none which means the route is not restricted and messages originating on any connection may be routed to the route",
            ),
        ),
    )
