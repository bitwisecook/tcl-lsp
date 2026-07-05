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
            "apm_sso_kerberos",
            module="apm",
            object_types=("sso kerberos",),
        ),
        header_types=(("apm", "sso kerberos"),),
        properties=(
            BigipPropertySpec(name="account-name", value_type="string", required=True),
            BigipPropertySpec(name="account-password", value_type="string", required=True),
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("headers",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="hname",
                        value_type="string",
                        in_sections=("headers",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="hvalue",
                        value_type="integer",
                        in_sections=("headers",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="hname",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="hvalue",
                value_type="integer",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(name="kdc", value_type="string", allow_none=True, default="none"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="realm", value_type="string", required=True),
            BigipPropertySpec(
                name="send-authorization",
                value_type="enum",
                enum_values=("401", "always"),
                default="always",
            ),
            BigipPropertySpec(
                name="spn-pattern",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ticket-lifetime",
                value_type="integer",
                allow_none=True,
                default="600 minutes",
            ),
            BigipPropertySpec(
                name="upn-support",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="user-realm-source", value_type="string", default="session"),
            BigipPropertySpec(name="username-source", value_type="string", default="session"),
        ),
    )
