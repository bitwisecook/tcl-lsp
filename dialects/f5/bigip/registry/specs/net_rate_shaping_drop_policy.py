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
            "net_rate_shaping_drop_policy",
            module="net",
            object_types=("rate-shaping drop-policy",),
        ),
        header_types=(("net", "rate-shaping drop-policy"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="average-packet-size", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="fred-max-active",
                value_type="integer",
                default="0 (zero),which disables active flow limitation",
            ),
            BigipPropertySpec(name="fred-max-drop", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="fred-min-drop", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="inverse-weight", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="max-probability", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="max-threshold", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="min-threshold", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="red-hard-limit", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("fred", "red", "tail"),
                default="tail",
            ),
        ),
    )
