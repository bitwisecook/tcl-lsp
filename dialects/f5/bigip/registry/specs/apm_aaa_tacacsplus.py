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
            "apm_aaa_tacacsplus",
            module="apm",
            object_types=("aaa tacacsplus",),
        ),
        header_types=(("apm", "aaa tacacsplus"),),
        properties=(
            BigipPropertySpec(name="address", value_type="unknown", required=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-service",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=(
                    "arap",
                    "enable",
                    "fwproxy",
                    "login",
                    "nasi",
                    "none",
                    "ppp",
                    "pt",
                    "rcmd",
                    "x25",
                ),
            ),
            BigipPropertySpec(
                name="auth-type",
                value_type="enum",
                enum_values=("arap", "ascii", "chap", "mschap", "pap"),
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="encrypt",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="pool",
                value_type="string",
                allow_none=True,
                references=(
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ),
                default="none",
            ),
            BigipPropertySpec(name="port", value_type="string", allow_none=True, default="49"),
            BigipPropertySpec(
                name="priv-lvl",
                value_type="enum",
                enum_values=("max", "min", "user"),
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                enum_values=(
                    "atalk",
                    "deccp",
                    "ftp",
                    "http",
                    "ip",
                    "ipx",
                    "lat",
                    "lcp",
                    "osicp",
                    "pad",
                    "rlogin",
                    "telnet",
                    "tn3270",
                    "unknown",
                    "vines",
                    "vpdn",
                    "xremote",
                ),
                default="unknown",
            ),
            BigipPropertySpec(name="secret", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="service",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "arap",
                    "connection",
                    "firewall",
                    "none",
                    "ppp",
                    "shell",
                    "slip",
                    "system",
                    "tty-daemon",
                ),
            ),
            BigipPropertySpec(
                name="use-pool",
                value_type="string",
                allow_none=True,
                default="none",
            ),
        ),
    )
