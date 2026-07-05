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
            "sys_ntp",
            module="sys",
            object_types=("ntp",),
        ),
        header_types=(("sys", "ntp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="restrict",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="string",
                        in_sections=("restrict",),
                        shape_kind="ip-address",
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="default-entry",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("restrict",),
                    ),
                    BigipPropertySpec(
                        name="ignore",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="kod",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="limited",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="low-priority-trap",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="mask",
                        value_type="string",
                        in_sections=("restrict",),
                        shape_kind="ip-address",
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="no-modify",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="no-peer",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="no-query",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="no-serve-packets",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="no-trap",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="no-trust",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="non-ntp-port",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="ntp-port",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="version",
                        value_type="enum",
                        in_sections=("restrict",),
                        enum_values=("disable", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("restrict",),
                shape_kind="ip-address",
                default="0",
            ),
            BigipPropertySpec(
                name="default-entry",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="ignore",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="kod",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="limited",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="low-priority-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="mask",
                value_type="string",
                in_sections=("restrict",),
                shape_kind="ip-address",
                default="0",
            ),
            BigipPropertySpec(
                name="no-modify",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="no-peer",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="no-query",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="no-serve-packets",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="no-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="no-trust",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="non-ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
                default="disabled",
            ),
            BigipPropertySpec(
                name="version",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="timezone", value_type="string"),
        ),
    )
