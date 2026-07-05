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
            "cm_device",
            module="cm",
            object_types=("device",),
        ),
        header_types=(("cm", "device"),),
        properties=(
            BigipPropertySpec(name="comment", value_type="string"),
            BigipPropertySpec(
                name="configsync-ip",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="contact", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ha-capacity", value_type="integer"),
            BigipPropertySpec(name="location", value_type="string"),
            BigipPropertySpec(
                name="mgmt-unicast-mode",
                value_type="enum",
                enum_values=("both", "ipv4", "ipv6"),
            ),
            BigipPropertySpec(
                name="mirror-ip",
                value_type="enum",
                enum_values=("any6",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="mirror-secondary-ip",
                value_type="enum",
                enum_values=("any6",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="multicast-interface", value_type="string"),
            BigipPropertySpec(name="multicast-ip", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="multicast-port", value_type="integer"),
            BigipPropertySpec(
                name="unicast-address",
                value_type="list",
                allow_none=True,
                block=(
                    BigipPropertySpec(
                        name="effective-ip",
                        value_type="string",
                        in_sections=("unicast-address",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="effective-port",
                        value_type="unknown",
                        in_sections=("unicast-address",),
                    ),
                    BigipPropertySpec(
                        name="ip",
                        value_type="string",
                        in_sections=("unicast-address",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="port",
                        value_type="unknown",
                        in_sections=("unicast-address",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="effective-ip",
                value_type="string",
                in_sections=("unicast-address",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="effective-port",
                value_type="unknown",
                in_sections=("unicast-address",),
            ),
            BigipPropertySpec(
                name="ip",
                value_type="string",
                in_sections=("unicast-address",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("unicast-address",)),
        ),
    )
