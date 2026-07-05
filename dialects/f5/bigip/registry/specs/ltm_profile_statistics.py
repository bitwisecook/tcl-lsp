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
            "ltm_profile_statistics",
            module="ltm",
            object_types=("profile statistics",),
        ),
        header_types=(("ltm", "profile statistics"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_statistics",),
                default="stats",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="field1", value_type="string"),
            BigipPropertySpec(name="field10", value_type="string"),
            BigipPropertySpec(name="field11", value_type="string"),
            BigipPropertySpec(name="field12", value_type="string"),
            BigipPropertySpec(name="field13", value_type="string"),
            BigipPropertySpec(name="field14", value_type="string"),
            BigipPropertySpec(name="field15", value_type="string"),
            BigipPropertySpec(name="field16", value_type="string"),
            BigipPropertySpec(name="field17", value_type="string"),
            BigipPropertySpec(name="field18", value_type="string"),
            BigipPropertySpec(name="field19", value_type="string"),
            BigipPropertySpec(name="field2", value_type="string"),
            BigipPropertySpec(name="field20", value_type="string"),
            BigipPropertySpec(name="field21", value_type="string"),
            BigipPropertySpec(name="field22", value_type="string"),
            BigipPropertySpec(name="field23", value_type="string"),
            BigipPropertySpec(name="field24", value_type="string"),
            BigipPropertySpec(name="field25", value_type="string"),
            BigipPropertySpec(name="field26", value_type="string"),
            BigipPropertySpec(name="field27", value_type="string"),
            BigipPropertySpec(name="field28", value_type="string"),
            BigipPropertySpec(name="field29", value_type="string"),
            BigipPropertySpec(name="field3", value_type="string"),
            BigipPropertySpec(name="field30", value_type="string"),
            BigipPropertySpec(name="field31", value_type="string"),
            BigipPropertySpec(name="field32", value_type="string"),
            BigipPropertySpec(name="field4", value_type="string"),
            BigipPropertySpec(name="field5", value_type="string"),
            BigipPropertySpec(name="field6", value_type="string"),
            BigipPropertySpec(name="field7", value_type="string"),
            BigipPropertySpec(name="field8", value_type="string"),
            BigipPropertySpec(name="field9", value_type="string"),
        ),
    )
