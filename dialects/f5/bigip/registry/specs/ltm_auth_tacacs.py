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
            "ltm_auth_tacacs",
            module="ltm",
            object_types=("auth tacacs",),
        ),
        header_types=(("ltm", "auth tacacs"),),
        properties=(
            BigipPropertySpec(
                name="accounting",
                value_type="enum",
                enum_values=("send-to-all-servers", "send-to-first-server"),
            ),
            BigipPropertySpec(
                name="authentication",
                value_type="enum",
                required=True,
                enum_values=("use-all-servers", "use-first-server"),
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encryption",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(name="secret", value_type="string", required=True),
            BigipPropertySpec(
                name="servers",
                value_type="string",
                required=True,
                shape_kind="endpoint",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="service",
                value_type="reference",
                required=True,
                allow_none=True,
                references=(
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "apm_aaa_f5_service_connector",
                    "apm_saml_artifact_resolution_service",
                    "apm_saml_attribute_consuming_service",
                    "net_service_policy",
                    "pem_service_chain_endpoint",
                    "security_bot_defense_micro_service",
                    "security_protocol_inspection_service",
                    "sys_application_service",
                    "sys_service",
                ),
            ),
        ),
    )
