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
            "apm_aaa_radius",
            module="apm",
            object_types=("aaa radius",),
        ),
        header_types=(("apm", "aaa radius"),),
        properties=(
            BigipPropertySpec(name="acct-port", value_type="integer", default="radius-acct"),
            BigipPropertySpec(name="address", value_type="unknown", required=True, allow_none=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-port",
                value_type="integer",
                required=True,
                default="radius",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("acct", "auth", "both")),
            BigipPropertySpec(name="nas-ip-address", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="nas-ipv6-address", value_type="unknown", allow_none=True),
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
            BigipPropertySpec(name="retries", value_type="integer", default="3"),
            BigipPropertySpec(name="secret", value_type="string", required=True),
            BigipPropertySpec(
                name="service-type",
                value_type="enum",
                enum_values=(
                    "administrative",
                    "authenticate-only",
                    "call-check",
                    "callback-administrative",
                    "callback-framed",
                    "callback-login",
                    "callback-nas-promit",
                    "default",
                    "framed",
                    "login",
                    "nas-prompt",
                    "outbound",
                ),
                default="default, which behaves as authenticate-only",
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="5"),
            BigipPropertySpec(
                name="use-pool",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="none",
            ),
        ),
    )
