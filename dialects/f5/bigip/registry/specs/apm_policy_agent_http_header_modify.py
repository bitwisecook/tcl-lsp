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
            "apm_policy_agent_http_header_modify",
            module="apm",
            object_types=("policy agent http-header-modify",),
        ),
        header_types=(("apm", "policy agent http-header-modify"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cookie-entries",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("cookie-entries",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="cookie-name",
                        value_type="string",
                        in_sections=("cookie-entries",),
                        required=True,
                    ),
                    BigipPropertySpec(
                        name="cookie-operation",
                        value_type="enum",
                        in_sections=("cookie-entries",),
                        enum_values=("cookie-delete", "cookie-update"),
                    ),
                    BigipPropertySpec(
                        name="cookie-value",
                        value_type="string",
                        in_sections=("cookie-entries",),
                        required=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("cookie-entries",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cookie-name",
                value_type="string",
                in_sections=("cookie-entries",),
                required=True,
            ),
            BigipPropertySpec(
                name="cookie-operation",
                value_type="enum",
                in_sections=("cookie-entries",),
                enum_values=("cookie-delete", "cookie-update"),
            ),
            BigipPropertySpec(
                name="cookie-value",
                value_type="string",
                in_sections=("cookie-entries",),
                required=True,
            ),
            BigipPropertySpec(
                name="header-entries",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("header-entries",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="header-delimiter",
                        value_type="string",
                        in_sections=("header-entries",),
                    ),
                    BigipPropertySpec(
                        name="header-name",
                        value_type="string",
                        in_sections=("header-entries",),
                        required=True,
                    ),
                    BigipPropertySpec(
                        name="header-operation",
                        value_type="enum",
                        in_sections=("header-entries",),
                        enum_values=(
                            "header-append",
                            "header-insert",
                            "header-remove",
                            "header-replace",
                        ),
                    ),
                    BigipPropertySpec(
                        name="header-value",
                        value_type="string",
                        in_sections=("header-entries",),
                        required=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("header-entries",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="header-delimiter",
                value_type="string",
                in_sections=("header-entries",),
            ),
            BigipPropertySpec(
                name="header-name",
                value_type="string",
                in_sections=("header-entries",),
                required=True,
            ),
            BigipPropertySpec(
                name="header-operation",
                value_type="enum",
                in_sections=("header-entries",),
                enum_values=("header-append", "header-insert", "header-remove", "header-replace"),
            ),
            BigipPropertySpec(
                name="header-value",
                value_type="string",
                in_sections=("header-entries",),
                required=True,
            ),
        ),
    )
