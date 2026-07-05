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
            "gtm_monitor_snmp",
            module="gtm",
            object_types=("monitor snmp",),
        ),
        header_types=(("gtm", "monitor snmp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="community",
                value_type="reference",
                allow_none=True,
                references=("net_routing_community_list",),
                default="public",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_snmp",),
                default="snmp_gtm",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="90 seconds"),
            BigipPropertySpec(name="port", value_type="integer", default="161"),
            BigipPropertySpec(name="probe", value_type="integer"),
            BigipPropertySpec(name="probe-interval", value_type="integer", default="1"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="timeout", value_type="integer", default="180 seconds"),
            BigipPropertySpec(name="version", value_type="integer", allow_none=True, default="v1"),
        ),
    )
