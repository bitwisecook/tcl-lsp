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
            "analytics_ssl_orchestrator_scheduled_report",
            module="analytics",
            object_types=("ssl-orchestrator scheduled-report",),
        ),
        header_types=(("analytics", "ssl-orchestrator scheduled-report"),),
        properties=(
            BigipPropertySpec(
                name="device-group",
                value_type="reference",
                references=("cm_device_group",),
            ),
            BigipPropertySpec(
                name="email-addresses",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="first-time", value_type="unknown"),
            BigipPropertySpec(
                name="frequency",
                value_type="enum",
                enum_values=(
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ),
            ),
            BigipPropertySpec(
                name="include-total",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="multi-leveled-report",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="chart-path",
                        value_type="list",
                        in_sections=("multi-leveled-report",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="limit",
                        value_type="unknown",
                        in_sections=("multi-leveled-report",),
                    ),
                    BigipPropertySpec(
                        name="measures",
                        value_type="list",
                        in_sections=("multi-leveled-report",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="time-diff",
                        value_type="enum",
                        in_sections=("multi-leveled-report",),
                        enum_values=(
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ),
                    ),
                    BigipPropertySpec(
                        name="view-by",
                        value_type="unknown",
                        in_sections=("multi-leveled-report",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="chart-path",
                value_type="list",
                in_sections=("multi-leveled-report",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="limit",
                value_type="unknown",
                in_sections=("multi-leveled-report",),
            ),
            BigipPropertySpec(
                name="measures",
                value_type="list",
                in_sections=("multi-leveled-report",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="time-diff",
                value_type="enum",
                in_sections=("multi-leveled-report",),
                enum_values=("last-day", "last-hour", "last-month", "last-week", "last-year"),
            ),
            BigipPropertySpec(
                name="view-by",
                value_type="unknown",
                in_sections=("multi-leveled-report",),
                shape_kind="object",
            ),
            BigipPropertySpec(name="predefined-report-name", value_type="reference"),
            BigipPropertySpec(name="smtp-config", value_type="reference"),
        ),
    )
