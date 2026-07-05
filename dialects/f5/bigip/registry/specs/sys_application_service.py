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
            "sys_application_service",
            module="sys",
            object_types=("application service",),
        ),
        header_types=(("sys", "application service"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="device-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="execute-action", value_type="reference"),
            BigipPropertySpec(
                name="lists",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="encrypted",
                        value_type="enum",
                        in_sections=("lists",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="value",
                        value_type="unknown",
                        in_sections=("lists",),
                        repeated=True,
                        allow_none=True,
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("lists",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="value",
                value_type="unknown",
                in_sections=("lists",),
                repeated=True,
                allow_none=True,
                shape_kind="object",
            ),
            BigipPropertySpec(
                name="metadata",
                value_type="unknown",
                default="persistent, which means the data will be saved into the config file",
            ),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="strict-updates",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="tables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="column-names",
                        value_type="list",
                        in_sections=("tables",),
                        repeated=True,
                    ),
                    BigipPropertySpec(
                        name="encrypted-columns",
                        value_type="list",
                        in_sections=("tables",),
                        repeated=True,
                    ),
                    BigipPropertySpec(
                        name="rows",
                        value_type="list",
                        in_sections=("tables",),
                        repeated=True,
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="column-names",
                value_type="list",
                in_sections=("tables",),
                repeated=True,
            ),
            BigipPropertySpec(
                name="encrypted-columns",
                value_type="list",
                in_sections=("tables",),
                repeated=True,
            ),
            BigipPropertySpec(
                name="rows",
                value_type="list",
                in_sections=("tables",),
                repeated=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="template",
                value_type="reference",
                references=(
                    "security_bot_defense_template",
                    "sys_application_template",
                    "vcmp_virtual_disk_template",
                ),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="variables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="encrypted",
                        value_type="enum",
                        in_sections=("variables",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="value", value_type="string", in_sections=("variables",)
                    ),
                ),
            ),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("variables",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("variables",)),
        ),
    )
