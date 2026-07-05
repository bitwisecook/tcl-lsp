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
            "pem_protocol_radius_avp",
            module="pem",
            object_types=("protocol radius-avp",),
        ),
        header_types=(("pem", "protocol radius-avp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="data-type",
                value_type="integer",
                enum_values=(
                    "3gpp-rat-type",
                    "3gpp-user-location-info",
                    "ipaddr",
                    "ipv6addr",
                    "ipv6prefix",
                    "octet",
                    "time",
                ),
                default="string",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-length", value_type="integer", default="253"),
            BigipPropertySpec(name="min-length", value_type="integer", default="1"),
            BigipPropertySpec(name="type", value_type="integer"),
            BigipPropertySpec(name="vendor-id", value_type="integer"),
            BigipPropertySpec(name="vendor-type", value_type="integer"),
        ),
    )
