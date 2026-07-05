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
            "apm_resource_remote_desktop_quest",
            module="apm",
            object_types=("resource remote-desktop quest",),
        ),
        header_types=(("apm", "resource remote-desktop quest"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-logon",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
                block=(
                    BigipPropertySpec(
                        name="caption",
                        value_type="string",
                        in_sections=("customization-group",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="detailed-description",
                        value_type="string",
                        in_sections=("customization-group",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="caption",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="detailed-description",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="domain-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.domain"),
                default="session",
            ),
            BigipPropertySpec(
                name="enable-serverside-ssl",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="host", value_type="unknown"),
            BigipPropertySpec(name="ip", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.password"),
                default="session",
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
            ),
            BigipPropertySpec(name="port", value_type="string", allow_none=True, default="8080"),
            BigipPropertySpec(
                name="username-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                default="session",
            ),
        ),
    )
