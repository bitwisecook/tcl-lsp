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
            "wom_endpoint_discovery",
            module="wom",
            object_types=("endpoint-discovery",),
        ),
        header_types=(("wom", "endpoint-discovery"),),
        properties=(
            BigipPropertySpec(
                name="auto-save",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="discoverable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="discovered-endpoint",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="icmp-max-requests", value_type="integer", default="1024"),
            BigipPropertySpec(name="icmp-min-backoff", value_type="integer", default="5"),
            BigipPropertySpec(name="icmp-num-retries", value_type="integer", default="10"),
            BigipPropertySpec(
                name="max-endpoint-count",
                value_type="integer",
                default="0, which indicates no limit",
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disable", "enable-all", "enable-icmp", "enable-tcp"),
                default="enable-all",
            ),
        ),
    )
