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
            "ltm_monitor_inband",
            module="ltm",
            object_types=("monitor inband",),
        ),
        header_types=(("ltm", "monitor inband"),),
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
                references=("ltm_monitor_inband",),
                default="inband",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="failure-interval", value_type="integer", default="30"),
            BigipPropertySpec(
                name="failures",
                value_type="integer",
                default="3, which means that the system marks the pool member unavailable at the fourth failure",
            ),
            BigipPropertySpec(name="response-time", value_type="integer"),
            BigipPropertySpec(name="retry-time", value_type="integer"),
        ),
    )
