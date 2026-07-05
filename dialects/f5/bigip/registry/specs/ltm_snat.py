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
            "ltm_snat",
            module="ltm",
            object_types=("snat",),
        ),
        header_types=(("ltm", "snat"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "disabled", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(
                name="mirror",
                value_type="unknown",
                allow_none=True,
                enum_values=("disabled", "enabled", "none"),
                shape_kind="object",
                default="none",
            ),
            BigipPropertySpec(name="origins", value_type="unknown", required=True),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="snatpool",
                value_type="reference",
                references=("ltm_snatpool",),
            ),
            BigipPropertySpec(
                name="source-port",
                value_type="enum",
                enum_values=("change", "preserve", "preserve-strict"),
                default="preserve",
            ),
            BigipPropertySpec(
                name="translation",
                value_type="reference",
                repeated=True,
                references=(
                    "ltm_snat_translation",
                    "security_nat_destination_translation",
                    "security_nat_source_translation",
                ),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="vlans",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
        ),
    )
