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
            "net_bwc_policy",
            module="net",
            object_types=("bwc policy",),
        ),
        header_types=(("net", "bwc policy"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="categories",
                value_type="list",
                required=True,
                block=(
                    BigipPropertySpec(
                        name="ip-tos",
                        value_type="integer",
                        in_sections=("categories",),
                        enum_values=("pass-through",),
                        default="pass-through, which indicates, do not modify UDP packets",
                    ),
                    BigipPropertySpec(
                        name="link-qos",
                        value_type="enum",
                        in_sections=("categories",),
                        enum_values=("pass-through",),
                        default="pass-through, which indicates, do not modify UDP packets",
                    ),
                    BigipPropertySpec(
                        name="max-cat-rate",
                        value_type="integer",
                        in_sections=("categories",),
                    ),
                    BigipPropertySpec(
                        name="max-cat-rate-percentage",
                        value_type="integer",
                        in_sections=("categories",),
                    ),
                    BigipPropertySpec(
                        name="traffic-priority-map",
                        value_type="string",
                        in_sections=("categories",),
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ip-tos",
                value_type="integer",
                in_sections=("categories",),
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(
                name="link-qos",
                value_type="enum",
                in_sections=("categories",),
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(
                name="max-cat-rate",
                value_type="integer",
                in_sections=("categories",),
            ),
            BigipPropertySpec(
                name="max-cat-rate-percentage",
                value_type="integer",
                in_sections=("categories",),
            ),
            BigipPropertySpec(
                name="traffic-priority-map",
                value_type="string",
                in_sections=("categories",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dynamic",
                value_type="unknown",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="ip-tos",
                value_type="integer",
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(
                name="link-qos",
                value_type="enum",
                enum_values=("pass-through",),
                default="pass-through, which indicates, do not modify UDP packets",
            ),
            BigipPropertySpec(name="log-period", value_type="integer"),
            BigipPropertySpec(name="log-publisher", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate", value_type="integer"),
            BigipPropertySpec(
                name="max-user-rate-pps",
                value_type="integer",
                default="0 (not configured)",
            ),
            BigipPropertySpec(name="measure", value_type="unknown"),
            BigipPropertySpec(
                name="traffic-priority-map",
                value_type="string",
                usage_flags=frozenset(("optional",)),
            ),
        ),
    )
