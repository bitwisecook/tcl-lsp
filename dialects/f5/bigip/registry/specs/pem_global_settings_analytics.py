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
            "pem_global_settings_analytics",
            module="pem",
            object_types=("global-settings analytics",),
        ),
        header_types=(("pem", "global-settings analytics"),),
        properties=(
            BigipPropertySpec(
                name="logging",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="hsl",
                        value_type="unknown",
                        in_sections=("logging",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="hsl",
                value_type="unknown",
                in_sections=("logging",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="endpoint-id",
                        value_type="unknown",
                        in_sections=("logging", "hsl"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="endpoint-id",
                value_type="unknown",
                in_sections=("logging", "hsl"),
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="subscriber-aware",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
