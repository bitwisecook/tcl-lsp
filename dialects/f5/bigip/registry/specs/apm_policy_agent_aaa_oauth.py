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
            "apm_policy_agent_aaa_oauth",
            module="apm",
            object_types=("policy agent aaa-oauth",),
        ),
        header_types=(("apm", "policy agent aaa-oauth"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="auth-redirect-request", value_type="reference"),
            BigipPropertySpec(
                name="grant-type",
                value_type="enum",
                enum_values=("authorization-code", "password"),
            ),
            BigipPropertySpec(name="redirection-uri", value_type="string"),
            BigipPropertySpec(
                name="response",
                value_type="reference",
                references=(
                    "api_protection_response",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "asm_response_code",
                    "ltm_profile_response_adapt",
                    "sys_crypto_cert_validation_response_ocsp",
                ),
            ),
            BigipPropertySpec(name="scope", value_type="string", allow_none=True),
            BigipPropertySpec(name="scope-data-request", value_type="reference"),
            BigipPropertySpec(
                name="server",
                value_type="reference",
                references=(
                    "api_protection_server",
                    "apm_aaa_oauth_server",
                    "apm_oauth_oauth_resource_server",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "auth_radius_server",
                    "gtm_listener_doh_server",
                    "gtm_monitor_real_server",
                    "gtm_server",
                    "ltm_auth_crldp_server",
                    "ltm_auth_radius_server",
                    "ltm_monitor_real_server",
                    "ltm_profile_doh_server",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "sys_crypto_server",
                    "sys_smtp_server",
                    "wom_server_discovery",
                ),
            ),
            BigipPropertySpec(name="token-refresh-request", value_type="reference"),
            BigipPropertySpec(name="token-request", value_type="reference"),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("client", "scope")),
            BigipPropertySpec(name="validation-scopes-request", value_type="reference"),
        ),
    )
