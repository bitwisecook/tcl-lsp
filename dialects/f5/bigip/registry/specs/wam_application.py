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
            "wam_application",
            module="wam",
            object_types=("application",),
        ),
        header_types=(("wam", "application"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="code", value_type="unknown"),
            BigipPropertySpec(
                name="collect-roi-statistics",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="content-expiration-time", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hosts",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("hosts",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(name="code", value_type="unknown", in_sections=("hosts",)),
                    BigipPropertySpec(
                        name="subdomain-number-of-http",
                        value_type="unknown",
                        in_sections=("hosts",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="subdomain-number-of-https",
                        value_type="unknown",
                        in_sections=("hosts",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="subdomain-prefix",
                        value_type="string",
                        in_sections=("hosts",),
                        default="wa",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("hosts",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="code", value_type="unknown", in_sections=("hosts",)),
            BigipPropertySpec(
                name="subdomain-number-of-http",
                value_type="unknown",
                in_sections=("hosts",),
                default="0",
            ),
            BigipPropertySpec(
                name="subdomain-number-of-https",
                value_type="unknown",
                in_sections=("hosts",),
                default="0",
            ),
            BigipPropertySpec(
                name="subdomain-prefix",
                value_type="string",
                in_sections=("hosts",),
                default="wa",
            ),
            BigipPropertySpec(
                name="ibr-adaptive-lifetime",
                value_type="unknown",
                default="864000 (10 days)",
            ),
            BigipPropertySpec(
                name="ibr-default-lifetime",
                value_type="unknown",
                default="15724800 (6 months)",
            ),
            BigipPropertySpec(name="ibr-prefix", value_type="string", default=""),
            BigipPropertySpec(
                name="info-header",
                value_type="enum",
                allow_none=True,
                enum_values=("debug", "none", "standard"),
            ),
            BigipPropertySpec(
                name="multibox",
                value_type="enum",
                required=True,
                enum_values=("disabled", "farm", "symmetric"),
            ),
            BigipPropertySpec(
                name="perf-monitor",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="perf-monitor-data-retention-period",
                value_type="unknown",
                default="30 days",
            ),
            BigipPropertySpec(name="policy", value_type="reference", references=("wam_policy",)),
            BigipPropertySpec(name="roi-report-collect-statistics", value_type="list"),
            BigipPropertySpec(name="roi-report-email-addresses", value_type="list"),
            BigipPropertySpec(
                name="roi-report-frequency",
                value_type="enum",
                enum_values=("every-month", "every-week"),
            ),
            BigipPropertySpec(name="roi-report-name", value_type="string"),
            BigipPropertySpec(name="roi-report-next-time", value_type="unknown"),
            BigipPropertySpec(name="roi-report-smtp-config", value_type="reference"),
            BigipPropertySpec(
                name="send-metadata",
                value_type="enum",
                enum_values=("always", "never", "uncompressed"),
                default="always",
            ),
        ),
    )
