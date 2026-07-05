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
            "net_stp",
            module="net",
            object_types=("stp",),
        ),
        header_types=(("net", "stp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="instance-id", value_type="integer"),
            BigipPropertySpec(
                name="interfaces",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("interfaces",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="external-path-cost",
                        value_type="integer",
                        in_sections=("interfaces",),
                        default="20000",
                    ),
                    BigipPropertySpec(
                        name="internal-path-cost",
                        value_type="integer",
                        in_sections=("interfaces",),
                        default="20000",
                    ),
                    BigipPropertySpec(
                        name="priority",
                        value_type="integer",
                        in_sections=("interfaces",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("interfaces",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="external-path-cost",
                value_type="integer",
                in_sections=("interfaces",),
                default="20000",
            ),
            BigipPropertySpec(
                name="internal-path-cost",
                value_type="integer",
                in_sections=("interfaces",),
                default="20000",
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("interfaces",)),
            BigipPropertySpec(name="priority", value_type="integer"),
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
                        name="external-path-cost",
                        value_type="integer",
                        in_sections=("trunks",),
                        default="20000",
                    ),
                    BigipPropertySpec(
                        name="internal-path-cost",
                        value_type="integer",
                        in_sections=("trunks",),
                        default="20000",
                    ),
                    BigipPropertySpec(
                        name="priority", value_type="integer", in_sections=("trunks",)
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
            BigipPropertySpec(
                name="external-path-cost",
                value_type="integer",
                in_sections=("trunks",),
                default="20000",
            ),
            BigipPropertySpec(
                name="internal-path-cost",
                value_type="integer",
                in_sections=("trunks",),
                default="20000",
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(
                name="vlans",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
