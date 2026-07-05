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
            "wam_ad_policy",
            module="wam",
            object_types=("ad-policy",),
        ),
        header_types=(("wam", "ad-policy"),),
        properties=(
            BigipPropertySpec(
                name="ad-insertion-order",
                value_type="enum",
                enum_values=("random", "sequential"),
            ),
            BigipPropertySpec(
                name="ads",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify")),
                block=(
                    BigipPropertySpec(
                        name="preroll",
                        value_type="enum",
                        in_sections=("ads",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(name="url", value_type="unknown", in_sections=("ads",)),
                ),
            ),
            BigipPropertySpec(
                name="preroll",
                value_type="enum",
                in_sections=("ads",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="url", value_type="unknown", in_sections=("ads",)),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
