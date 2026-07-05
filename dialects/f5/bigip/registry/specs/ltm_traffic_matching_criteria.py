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
            "ltm_traffic_matching_criteria",
            module="ltm",
            object_types=("traffic-matching-criteria",),
        ),
        header_types=(("ltm", "traffic-matching-criteria"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address-inline",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="destination-address-list",
                value_type="reference",
                allow_none=True,
                default="any",
            ),
            BigipPropertySpec(name="destination-port-inline", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="destination-port-list",
                value_type="reference",
                allow_none=True,
                references=("net_port_list", "security_firewall_port_list"),
                default="any",
            ),
            BigipPropertySpec(name="source-address-inline", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="source-address-list",
                value_type="reference",
                allow_none=True,
                default="any",
            ),
            BigipPropertySpec(name="source-port-inline", value_type="unknown", default="none"),
        ),
    )
