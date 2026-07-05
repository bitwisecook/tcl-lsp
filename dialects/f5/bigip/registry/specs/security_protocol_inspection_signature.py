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
            "security_protocol_inspection_signature",
            module="security",
            object_types=("protocol-inspection signature",),
        ),
        header_types=(("security", "protocol-inspection signature"),),
        properties=(
            BigipPropertySpec(
                name="accuracy",
                value_type="enum",
                enum_values=("high", "low", "medium"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("accept", "drop", "reject"),
            ),
            BigipPropertySpec(name="app-service", value_type="string"),
            BigipPropertySpec(name="attack-type", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("any", "to-client", "to-server"),
            ),
            BigipPropertySpec(name="documentation", value_type="string"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="last-updated", value_type="unknown"),
            BigipPropertySpec(
                name="log",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="performance-impact",
                value_type="enum",
                enum_values=("high", "low", "medium"),
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                enum_values=("any", "tcp", "udp"),
            ),
            BigipPropertySpec(name="reference-links", value_type="string"),
            BigipPropertySpec(name="references", value_type="string"),
            BigipPropertySpec(name="revision", value_type="integer"),
            BigipPropertySpec(
                name="risk",
                value_type="enum",
                enum_values=("critical", "high", "low", "medium"),
            ),
            BigipPropertySpec(name="service", value_type="string"),
            BigipPropertySpec(name="sig", value_type="unknown"),
            BigipPropertySpec(name="systems", value_type="string"),
            BigipPropertySpec(
                name="user-defined",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
