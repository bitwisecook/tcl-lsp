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
            "sys_cluster",
            module="sys",
            object_types=("cluster",),
        ),
        header_types=(("sys", "cluster"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="alt-address",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="members",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="enum",
                        in_sections=("members",),
                        allow_none=True,
                        enum_values=("none",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="alt-address",
                        value_type="enum",
                        in_sections=("members",),
                        allow_none=True,
                        enum_values=("none",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="priming",
                        value_type="enum",
                        in_sections=("members",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="enum",
                in_sections=("members",),
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="alt-address",
                value_type="enum",
                in_sections=("members",),
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="priming",
                value_type="enum",
                in_sections=("members",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="min-up-members", value_type="integer"),
            BigipPropertySpec(
                name="min-up-members-enabled",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
