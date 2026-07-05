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
            "sys_file_data_group",
            module="sys",
            object_types=("file data-group",),
        ),
        header_types=(("sys", "file data-group"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="data-group-description",
                value_type="string",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="data-group-name",
                value_type="reference",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="separator", value_type="string", default=":="),
            BigipPropertySpec(name="source-path", value_type="unknown"),
            BigipPropertySpec(
                name="type",
                value_type="integer",
                required=True,
                enum_values=("ip",),
            ),
        ),
    )
