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
            "apm_report_custom_report_field",
            module="apm",
            object_types=("report custom-report-field",),
        ),
        header_types=(("apm", "report custom-report-field"),),
        properties=(
            BigipPropertySpec(name="alias", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", default="none"),
            BigipPropertySpec(name="field-position", value_type="integer"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="report-name", value_type="string"),
            BigipPropertySpec(
                name="sort-direction",
                value_type="enum",
                enum_values=("asc", "desc", "unsorted"),
                default="asc",
            ),
            BigipPropertySpec(name="sort-order", value_type="integer", default="100000"),
        ),
    )
