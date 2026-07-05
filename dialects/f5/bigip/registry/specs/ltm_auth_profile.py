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
            "ltm_auth_profile",
            module="ltm",
            object_types=("auth profile",),
        ),
        header_types=(("ltm", "auth profile"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="configuration",
                value_type="reference",
                required=True,
                allow_none=True,
                references=("apm_aaa_f5_mfa_configuration", "apm_configuration_captcha"),
            ),
            BigipPropertySpec(name="cookie-key", value_type="string", default="f5auth"),
            BigipPropertySpec(name="cookie-name", value_type="string", default="abc123"),
            BigipPropertySpec(name="credential-source", value_type="unknown"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                required=True,
                references=("ltm_auth_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(
                name="rule",
                value_type="reference",
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
            ),
        ),
    )
