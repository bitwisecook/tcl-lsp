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
            "net_ipsec_traffic_selector",
            module="net",
            object_types=("ipsec traffic-selector",),
        ),
        header_types=(("net", "ipsec traffic-selector"),),
        properties=(
            BigipPropertySpec(name="action", value_type="unknown"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="destination-port", value_type="unknown"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("both", "in", "out"),
                default="both",
            ),
            BigipPropertySpec(name="ip-protocol", value_type="unknown"),
            BigipPropertySpec(
                name="ipsec-policy",
                value_type="reference",
                references=("net_ipsec_ipsec_policy",),
            ),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="source-port", value_type="unknown"),
        ),
    )
