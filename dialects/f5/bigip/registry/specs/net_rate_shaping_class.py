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
            "net_rate_shaping_class",
            module="net",
            object_types=("rate-shaping class",),
        ),
        header_types=(("net", "rate-shaping class"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ceiling",
                value_type="integer",
                default="the value of the rate option",
            ),
            BigipPropertySpec(
                name="ceiling-percentage",
                value_type="integer",
                default="0 (zero), which indicates that the class uses the value of the ceiling option",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("any", "to-client", "to-server"),
                default="any",
            ),
            BigipPropertySpec(
                name="drop-policy",
                value_type="reference",
                references=("net_rate_shaping_drop_policy",),
                default="tail, which is the simplest drop policy",
            ),
            BigipPropertySpec(name="max-burst", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="parent", value_type="reference"),
            BigipPropertySpec(
                name="queue",
                value_type="enum",
                enum_values=("pfifp", "sfq"),
                references=(
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ),
                default="sfq",
            ),
            BigipPropertySpec(name="rate", value_type="integer"),
            BigipPropertySpec(
                name="rate-percentage",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the rate option",
            ),
            BigipPropertySpec(
                name="shaping-policy",
                value_type="reference",
                allow_none=True,
                references=("net_rate_shaping_shaping_policy",),
                default="none",
            ),
        ),
    )
