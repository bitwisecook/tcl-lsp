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
            "security_firewall_port_misuse_policy",
            module="security",
            object_types=("firewall port-misuse-policy",),
        ),
        header_types=(("security", "firewall port-misuse-policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="drop-on-l7-mismatch",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="log-on-l7-mismatch",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
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
                        name="description", value_type="string", in_sections=("rules",)
                    ),
                    BigipPropertySpec(
                        name="drop-on-l7-mismatch",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("no", "use-policy-setting", "yes"),
                        default="yes",
                    ),
                    BigipPropertySpec(
                        name="ip-protocol",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("sctp", "tcp", "udp"),
                        default="tcp",
                    ),
                    BigipPropertySpec(
                        name="l7-protocol",
                        value_type="reference",
                        in_sections=("rules",),
                    ),
                    BigipPropertySpec(
                        name="log-on-l7-mismatch",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("no", "use-policy-setting", "yes"),
                        default="no",
                    ),
                    BigipPropertySpec(name="port", value_type="unknown", in_sections=("rules",)),
                ),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="drop-on-l7-mismatch",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "use-policy-setting", "yes"),
                default="yes",
            ),
            BigipPropertySpec(
                name="ip-protocol",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("sctp", "tcp", "udp"),
                default="tcp",
            ),
            BigipPropertySpec(name="l7-protocol", value_type="reference", in_sections=("rules",)),
            BigipPropertySpec(
                name="log-on-l7-mismatch",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "use-policy-setting", "yes"),
                default="no",
            ),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("rules",)),
        ),
    )
