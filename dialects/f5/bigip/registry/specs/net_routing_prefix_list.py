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
            "net_routing_prefix_list",
            module="net",
            object_types=("routing prefix-list",),
        ),
        header_types=(("net", "routing prefix-list"),),
        properties=(
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="entries",
                value_type="reference",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                shape_kind="list",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="boolean",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(name="prefix", value_type="string", in_sections=("entries",)),
                    BigipPropertySpec(
                        name="prefix-len-range",
                        value_type="boolean",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="boolean",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(name="prefix", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="prefix-len-range",
                value_type="boolean",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
        ),
    )
