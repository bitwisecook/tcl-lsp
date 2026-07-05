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
            "apm_sso_form_based",
            module="apm",
            object_types=("sso form-based",),
        ),
        header_types=(("apm", "sso form-based"),),
        properties=(
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="external-access-management",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "oam"),
            ),
            BigipPropertySpec(
                name="form-action",
                value_type="unknown",
                allow_none=True,
                default="none",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="form-field",
                value_type="string",
                required=True,
                default="none",
            ),
            BigipPropertySpec(
                name="form-method",
                value_type="enum",
                enum_values=("get", "post"),
                default="post",
            ),
            BigipPropertySpec(name="form-password", value_type="string"),
            BigipPropertySpec(name="form-username", value_type="string"),
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
                        value_type="unknown",
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
                value_type="unknown",
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
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.sso.token.last.password"),
                default="session",
            ),
            BigipPropertySpec(
                name="start-uri",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="success-match-type",
                value_type="enum",
                allow_none=True,
                enum_values=("cookie", "none", "url"),
                default="none",
            ),
            BigipPropertySpec(name="success-match-value", value_type="string", default="none"),
            BigipPropertySpec(
                name="username-source",
                value_type="unknown",
                allow_none=True,
                default="session",
            ),
        ),
    )
