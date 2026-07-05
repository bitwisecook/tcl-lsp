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
            "apm_aaa_endpoint_management_system",
            module="apm",
            object_types=("aaa endpoint-management-system",),
        ),
        header_types=(("apm", "aaa endpoint-management-system"),),
        properties=(
            BigipPropertySpec(name="access-key", value_type="string", allow_none=True),
            BigipPropertySpec(name="app-version", value_type="string", allow_none=True),
            BigipPropertySpec(name="application-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="billing-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="client-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="client-secret", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                allow_none=True,
                references=("net_dns_resolver",),
            ),
            BigipPropertySpec(name="fqdn", value_type="string", required=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="mdm-token", value_type="string", allow_none=True),
            BigipPropertySpec(name="password", value_type="string", required=True),
            BigipPropertySpec(name="platform", value_type="string", allow_none=True),
            BigipPropertySpec(name="port", value_type="unknown", default="443"),
            BigipPropertySpec(name="serverssl-profile", value_type="reference", required=True),
            BigipPropertySpec(
                name="sync-interval",
                value_type="integer",
                allow_none=True,
                default="240 minutes",
            ),
            BigipPropertySpec(name="tenant-id", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                required=True,
                enum_values=("airwatch", "maas360", "ms-intune"),
            ),
            BigipPropertySpec(name="username", value_type="string", required=True),
        ),
    )
