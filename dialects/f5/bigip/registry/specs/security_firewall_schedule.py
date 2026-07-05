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
            "security_firewall_schedule",
            module="security",
            object_types=("firewall schedule",),
        ),
        header_types=(("security", "firewall schedule"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(
                name="daily-hour-end",
                value_type="unknown",
                default="24:00 (midnight)",
            ),
            BigipPropertySpec(
                name="daily-hour-start",
                value_type="unknown",
                default="0:00 (midnight at the start of the day)",
            ),
            BigipPropertySpec(
                name="date-valid-end",
                value_type="integer",
                default="19:14 1/18/2038 (the latest date expressible with a 32-bit integer)",
            ),
            BigipPropertySpec(
                name="date-valid-start",
                value_type="unknown",
                default="midnight 1/1/1970 (Unix epoch)",
            ),
            BigipPropertySpec(
                name="days-of-week",
                value_type="enum",
                repeated=True,
                enum_values=(
                    "friday",
                    "monday",
                    "saturday",
                    "sunday",
                    "thursday",
                    "tuesday",
                    "wednesday",
                ),
                default="all seven days",
            ),
            BigipPropertySpec(name="description", value_type="unknown"),
        ),
    )
