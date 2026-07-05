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
            "ltm_global_settings_traffic_control",
            module="ltm",
            object_types=("global-settings traffic-control",),
        ),
        header_types=(("ltm", "global-settings traffic-control"),),
        properties=(
            BigipPropertySpec(
                name="accept-ip-options",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="accept-ip-source-route",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="allow-ip-source-route",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="continue-matching",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="max-icmp-rate",
                value_type="unknown",
                default="100 errors per second",
            ),
            BigipPropertySpec(
                name="max-reject-rate",
                value_type="integer",
                default="250 per second",
            ),
            BigipPropertySpec(
                name="max-reject-rate-timeout",
                value_type="integer",
                default="30 seconds",
            ),
            BigipPropertySpec(name="min-path-mtu", value_type="integer", default="296"),
            BigipPropertySpec(
                name="path-mtu-discovery",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="port-find-linear", value_type="integer", default="16"),
            BigipPropertySpec(name="port-find-random", value_type="integer", default="16"),
            BigipPropertySpec(
                name="port-find-threshold-timeout",
                value_type="unknown",
                default="30 (1/2 minute) with range from 0 - 300",
            ),
            BigipPropertySpec(
                name="port-find-threshold-trigger",
                value_type="unknown",
                default="8",
            ),
            BigipPropertySpec(
                name="port-find-threshold-warning",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="reject-unmatched",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
