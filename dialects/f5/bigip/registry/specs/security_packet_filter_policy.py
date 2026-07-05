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
            "security_packet_filter_policy",
            module="security",
            object_types=("packet-filter policy",),
        ),
        header_types=(("security", "packet-filter policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                allow_none=True,
                references=(
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("accept", "drop"),
                    ),
                    BigipPropertySpec(
                        name="description", value_type="string", in_sections=("rules",)
                    ),
                    BigipPropertySpec(
                        name="ipv6-extension-headers",
                        value_type="list",
                        in_sections=("rules",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="log",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="status",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("accept", "drop"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="ipv6-extension-headers",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="values",
                        value_type="list",
                        in_sections=("rules", "ipv6-extension-headers"),
                        required=True,
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="values",
                value_type="list",
                in_sections=("rules", "ipv6-extension-headers"),
                required=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="log",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="status",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
