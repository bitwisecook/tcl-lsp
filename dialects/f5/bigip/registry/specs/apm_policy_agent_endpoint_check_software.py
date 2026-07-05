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
            "apm_policy_agent_endpoint_check_software",
            module="apm",
            object_types=("policy agent endpoint-check-software",),
        ),
        header_types=(("apm", "policy agent endpoint-check-software"),),
        properties=(
            BigipPropertySpec(
                name="check-list-type",
                value_type="enum",
                enum_values=("allow", "deny", "required"),
            ),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="continuous-check",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="items",
                value_type="enum",
                enum_values=(
                    "db-age",
                    "db-version",
                    "last-scan",
                    "missing-updates",
                    "platform",
                    "product_id",
                    "state",
                    "vendor_id",
                    "version",
                ),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "antispyware",
                    "antivirus",
                    "firewall",
                    "hard-disk-encryption",
                    "health-agent",
                    "patch-management",
                    "peer-to-peer",
                ),
            ),
        ),
    )
