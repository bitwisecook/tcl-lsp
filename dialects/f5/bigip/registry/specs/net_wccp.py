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
            "net_wccp",
            module="net",
            object_types=("wccp",),
        ),
        header_types=(("net", "wccp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cache-timeout", value_type="integer", default="10"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="services",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="alt-hash-fields",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest-ip", "none", "src-ip"),
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("services",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="hash-fields",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest-ip", "none", "src-ip"),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="port-type",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest", "none", "source"),
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="ports", value_type="integer", in_sections=("services",)
                    ),
                    BigipPropertySpec(
                        name="priority",
                        value_type="integer",
                        in_sections=("services",),
                        default="100",
                    ),
                    BigipPropertySpec(
                        name="protocol",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("tcp", "udp"),
                        default="tcp",
                    ),
                    BigipPropertySpec(
                        name="redirection-method",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("gre", "l2"),
                    ),
                    BigipPropertySpec(
                        name="return-method",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("gre", "l2"),
                    ),
                    BigipPropertySpec(
                        name="routers",
                        value_type="list",
                        in_sections=("services",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="traffic-assign",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("hash", "mask"),
                    ),
                    BigipPropertySpec(
                        name="tunnel-local-address",
                        value_type="string",
                        in_sections=("services",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="tunnel-remote-addresses",
                        value_type="list",
                        in_sections=("services",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="weight",
                        value_type="integer",
                        in_sections=("services",),
                        default="50",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="alt-hash-fields",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "none", "src-ip"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("services",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="hash-fields",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "none", "src-ip"),
            ),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="port-type",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest", "none", "source"),
                default="none",
            ),
            BigipPropertySpec(name="ports", value_type="integer", in_sections=("services",)),
            BigipPropertySpec(
                name="priority",
                value_type="integer",
                in_sections=("services",),
                default="100",
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                in_sections=("services",),
                enum_values=("tcp", "udp"),
                default="tcp",
            ),
            BigipPropertySpec(
                name="redirection-method",
                value_type="enum",
                in_sections=("services",),
                enum_values=("gre", "l2"),
            ),
            BigipPropertySpec(
                name="return-method",
                value_type="enum",
                in_sections=("services",),
                enum_values=("gre", "l2"),
            ),
            BigipPropertySpec(
                name="routers",
                value_type="list",
                in_sections=("services",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="traffic-assign",
                value_type="enum",
                in_sections=("services",),
                enum_values=("hash", "mask"),
            ),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                in_sections=("services",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="tunnel-remote-addresses",
                value_type="list",
                in_sections=("services",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="weight",
                value_type="integer",
                in_sections=("services",),
                default="50",
            ),
        ),
    )
