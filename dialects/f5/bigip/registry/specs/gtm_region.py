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
            "gtm_region",
            module="gtm",
            object_types=("region",),
        ),
        header_types=(("gtm", "region"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="continent",
                value_type="enum",
                enum_values=("Africa", "Antarctica", "Asia", "Australia", "Europe", "unknown"),
            ),
            BigipPropertySpec(name="country", value_type="reference"),
            BigipPropertySpec(
                name="datacenter",
                value_type="reference",
                references=("gtm_datacenter",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="geoip-isp", value_type="reference"),
            BigipPropertySpec(
                name="isp",
                value_type="enum",
                enum_values=(
                    "AOL",
                    "BeijingCNC",
                    "CNC",
                    "ChinaTelecom",
                    "Comcast",
                    "Earthlink",
                    "ShanghaiCNC",
                    "ShanghaiTelecom",
                ),
            ),
            BigipPropertySpec(
                name="not",
                value_type="enum",
                enum_values=("continent", "country", "datacenter", "isp", "pool", "subnet"),
            ),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                references=(
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ),
            ),
            BigipPropertySpec(name="region-members", value_type="unknown"),
            BigipPropertySpec(name="region-name", value_type="reference"),
            BigipPropertySpec(
                name="state",
                value_type="reference",
                references=(
                    "security_firewall_current_state",
                    "sys_mcp_state",
                    "sys_state_mirroring",
                ),
            ),
            BigipPropertySpec(name="subnet", value_type="unknown"),
        ),
    )
