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
            "apm_aaa_ldap",
            module="apm",
            object_types=("aaa ldap",),
        ),
        header_types=(("apm", "aaa ldap"),),
        properties=(
            BigipPropertySpec(name="address", value_type="unknown", required=True, allow_none=True),
            BigipPropertySpec(name="admin-dn", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="admin-encrypted-password",
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
            BigipPropertySpec(name="base-dn", value_type="string"),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="is-ldaps",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
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
            BigipPropertySpec(
                name="port",
                value_type="unknown",
                required=True,
                allow_none=True,
                default="ldap",
            ),
            BigipPropertySpec(
                name="schema-attr",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="group-member",
                        value_type="string",
                        in_sections=("schema-attr",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="group-member-value",
                        value_type="string",
                        in_sections=("schema-attr",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="group-memberof",
                        value_type="string",
                        in_sections=("schema-attr",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="group-object-class",
                        value_type="string",
                        in_sections=("schema-attr",),
                    ),
                    BigipPropertySpec(
                        name="user-memberof",
                        value_type="string",
                        in_sections=("schema-attr",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="user-object-class",
                        value_type="string",
                        in_sections=("schema-attr",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="group-member",
                value_type="string",
                in_sections=("schema-attr",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="group-member-value",
                value_type="string",
                in_sections=("schema-attr",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="group-memberof",
                value_type="string",
                in_sections=("schema-attr",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="group-object-class",
                value_type="string",
                in_sections=("schema-attr",),
            ),
            BigipPropertySpec(
                name="user-memberof",
                value_type="string",
                in_sections=("schema-attr",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="user-object-class",
                value_type="string",
                in_sections=("schema-attr",),
            ),
            BigipPropertySpec(
                name="serverssl-profile",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "none",
                    "serverssl",
                    "serverssl-insecure-compatible",
                    "wom-default-serverssl",
                ),
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="15"),
            BigipPropertySpec(
                name="use-pool",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
