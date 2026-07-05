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
            "apm_sso_ntlmv2",
            module="apm",
            object_types=("sso ntlmv2",),
        ),
        header_types=(("apm", "sso ntlmv2"),),
        properties=(
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="app-service",
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
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="ntlm-domain", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.sso.token.last.password"),
            ),
            BigipPropertySpec(
                name="username-conversion",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="username-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
        ),
    )
