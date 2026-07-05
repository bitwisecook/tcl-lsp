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
            "sys_ha_group",
            module="sys",
            object_types=("ha-group",),
        ),
        header_types=(("sys", "ha-group"),),
        properties=(
            BigipPropertySpec(name="active-bonus", value_type="integer", default="10 (ten)"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="clusters",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("clusters",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="attribute",
                        value_type="unknown",
                        in_sections=("clusters",),
                    ),
                    BigipPropertySpec(
                        name="minimum-threshold",
                        value_type="integer",
                        in_sections=("clusters",),
                        default="0 (zero), which indicates this option is disabled",
                    ),
                    BigipPropertySpec(
                        name="sufficient",
                        value_type="enum",
                        in_sections=("clusters",),
                        enum_values=("all",),
                    ),
                    BigipPropertySpec(
                        name="threshold",
                        value_type="integer",
                        in_sections=("clusters",),
                        usage_flags=frozenset(("deprecated",)),
                    ),
                    BigipPropertySpec(
                        name="weight",
                        value_type="integer",
                        in_sections=("clusters",),
                        default="10",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("clusters",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("clusters",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("clusters",),
                default="0 (zero), which indicates this option is disabled",
            ),
            BigipPropertySpec(
                name="sufficient",
                value_type="enum",
                in_sections=("clusters",),
                enum_values=("all",),
            ),
            BigipPropertySpec(
                name="threshold",
                value_type="integer",
                in_sections=("clusters",),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="weight",
                value_type="integer",
                in_sections=("clusters",),
                default="10",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="pools",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("pools",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="attribute", value_type="unknown", in_sections=("pools",)
                    ),
                    BigipPropertySpec(
                        name="minimum-threshold",
                        value_type="integer",
                        in_sections=("pools",),
                        default="0 (zero), which indicates this option is disabled",
                    ),
                    BigipPropertySpec(
                        name="sufficient", value_type="integer", in_sections=("pools",)
                    ),
                    BigipPropertySpec(
                        name="threshold",
                        value_type="integer",
                        in_sections=("pools",),
                        usage_flags=frozenset(("deprecated",)),
                    ),
                    BigipPropertySpec(
                        name="weight",
                        value_type="integer",
                        in_sections=("pools",),
                        default="10",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("pools",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("pools",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("pools",),
                default="0 (zero), which indicates this option is disabled",
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(
                name="threshold",
                value_type="integer",
                in_sections=("pools",),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="weight",
                value_type="integer",
                in_sections=("pools",),
                default="10",
            ),
            BigipPropertySpec(
                name="trunks",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("trunks",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="attribute", value_type="unknown", in_sections=("trunks",)
                    ),
                    BigipPropertySpec(
                        name="minimum-threshold",
                        value_type="integer",
                        in_sections=("trunks",),
                        default="0 (zero), which indicates this option is disabled",
                    ),
                    BigipPropertySpec(
                        name="sufficient", value_type="integer", in_sections=("trunks",)
                    ),
                    BigipPropertySpec(
                        name="threshold",
                        value_type="integer",
                        in_sections=("trunks",),
                        usage_flags=frozenset(("deprecated",)),
                    ),
                    BigipPropertySpec(
                        name="weight",
                        value_type="integer",
                        in_sections=("trunks",),
                        default="10",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("trunks",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="attribute", value_type="unknown", in_sections=("trunks",)),
            BigipPropertySpec(
                name="minimum-threshold",
                value_type="integer",
                in_sections=("trunks",),
                default="0 (zero), which indicates this option is disabled",
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(
                name="threshold",
                value_type="integer",
                in_sections=("trunks",),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="weight",
                value_type="integer",
                in_sections=("trunks",),
                default="10",
            ),
        ),
    )
