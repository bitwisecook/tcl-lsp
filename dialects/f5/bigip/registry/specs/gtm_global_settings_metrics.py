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
            "gtm_global_settings_metrics",
            module="gtm",
            object_types=("global-settings metrics",),
        ),
        header_types=(("gtm", "global-settings metrics"),),
        properties=(
            BigipPropertySpec(name="default-probe-limit", value_type="integer", default="12"),
            BigipPropertySpec(name="hops-packet-length", value_type="integer", default="64"),
            BigipPropertySpec(name="hops-sample-count", value_type="integer", default="3"),
            BigipPropertySpec(name="hops-timeout", value_type="integer", default="3"),
            BigipPropertySpec(name="hops-ttl", value_type="integer", default="604800"),
            BigipPropertySpec(
                name="inactive-ldns-ttl",
                value_type="integer",
                default="2419200 (28 days)",
            ),
            BigipPropertySpec(
                name="inactive-paths-ttl",
                value_type="integer",
                default="604800 (7 days)",
            ),
            BigipPropertySpec(
                name="ldns-update-interval",
                value_type="integer",
                default="20 seconds",
            ),
            BigipPropertySpec(
                name="max-synchronous-monitor-requests",
                value_type="integer",
                default="20",
            ),
            BigipPropertySpec(
                name="metrics-caching",
                value_type="integer",
                min_value=0,
                max_value=604800,
                default="3600",
            ),
            BigipPropertySpec(
                name="metrics-collection-protocols",
                value_type="unknown",
                allow_none=True,
            ),
            BigipPropertySpec(name="path-ttl", value_type="integer", default="2400"),
            BigipPropertySpec(name="paths-retry", value_type="integer", default="120"),
        ),
    )
