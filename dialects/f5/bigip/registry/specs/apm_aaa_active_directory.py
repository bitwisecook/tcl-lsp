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
            "apm_aaa_active_directory",
            module="apm",
            object_types=("aaa active-directory",),
        ),
        header_types=(("apm", "aaa active-directory"),),
        properties=(
            BigipPropertySpec(
                name="admin-encrypted-password",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="admin-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cleanup-cache",
                value_type="enum",
                allow_none=True,
                enum_values=("group", "kerberos", "none", "pso"),
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="domain", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="domain-controller",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="domain-controllers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="ip",
                        value_type="string",
                        in_sections=("domain-controllers",),
                        shape_kind="ip-address",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ip",
                value_type="string",
                in_sections=("domain-controllers",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="group-cache-ttl", value_type="integer", default="30"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="padata", value_type="unknown", default="rc4-hmac"),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
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
            BigipPropertySpec(name="pso-cache-ttl", value_type="integer", default="30"),
            BigipPropertySpec(name="timeout", value_type="integer", default="15"),
        ),
    )
