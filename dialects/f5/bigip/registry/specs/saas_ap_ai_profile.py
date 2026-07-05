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
            "saas_ap_ai_profile",
            module="saas",
            object_types=("ap-ai profile",),
        ),
        header_types=(("saas", "ap-ai profile"),),
        properties=(
            BigipPropertySpec(
                name="account-protection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="add-connecting-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="ai-header-name", value_type="string", default="x-apg-sr"),
            BigipPropertySpec(name="ap-header-name", value_type="string", default="x-safe-fr"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="authentication-intelligence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="block-response-body", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="block-response-code",
                value_type="integer",
                allow_none=True,
                default="200",
            ),
            BigipPropertySpec(name="block-response-content-type", value_type="string"),
            BigipPropertySpec(
                name="connecting-ip-header",
                value_type="string",
                default="x-iapp-real-ip",
            ),
            BigipPropertySpec(name="customer-id", value_type="string"),
            BigipPropertySpec(
                name="decrypt-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("saas_ap_ai_profile",),
                default="ap-ai",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="domain-pool",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="encryption-key", value_type="string", default="none"),
            BigipPropertySpec(name="hostname", value_type="string", default="us"),
            BigipPropertySpec(
                name="irules",
                value_type="list",
                repeated=True,
                allow_none=True,
                references=("apm_policy_agent_irule_event", "pem_irule"),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(name="ivs-ssl", value_type="string"),
            BigipPropertySpec(
                name="js-inject-exclude-paths",
                value_type="list",
                repeated=True,
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="js-inject-exclude-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="js-inject-include-paths",
                value_type="list",
                repeated=True,
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="js-inject-include-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="js-inject-location",
                value_type="enum",
                enum_values=("body", "head"),
                default="after head",
            ),
            BigipPropertySpec(
                name="js-inject-script-attribute",
                value_type="enum",
                enum_values=("async", "async-defer", "defer", "sync"),
                default="async-defer",
            ),
            BigipPropertySpec(name="js-path", value_type="string"),
            BigipPropertySpec(
                name="protected-endpoints",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="ai-endpoint",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                        default="enabled",
                    ),
                    BigipPropertySpec(
                        name="ap-endpoint",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("protected-endpoints",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="enforcement-mode",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("mitigate", "monitor"),
                    ),
                    BigipPropertySpec(
                        name="host",
                        value_type="string",
                        in_sections=("protected-endpoints",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="max-cookie-age",
                        value_type="integer",
                        in_sections=("protected-endpoints",),
                        allow_none=True,
                        default="7",
                    ),
                    BigipPropertySpec(
                        name="mitigate-malformed-cookie",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="mitigate-max-cookie-age",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="mitigate-missing-cookie",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="mitigation-action",
                        value_type="enum",
                        in_sections=("protected-endpoints",),
                        enum_values=("block", "redirect"),
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="path",
                        value_type="string",
                        in_sections=("protected-endpoints",),
                        allow_none=True,
                        default="none",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ai-endpoint",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="ap-endpoint",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="enforcement-mode",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("mitigate", "monitor"),
            ),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="max-cookie-age",
                value_type="integer",
                in_sections=("protected-endpoints",),
                allow_none=True,
                default="7",
            ),
            BigipPropertySpec(
                name="mitigate-malformed-cookie",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="mitigate-max-cookie-age",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="mitigate-missing-cookie",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="mitigation-action",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("block", "redirect"),
                default="none",
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="proxy-destination",
                value_type="string",
                allow_none=True,
                default="https://us",
            ),
            BigipPropertySpec(
                name="proxy-password",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="proxy-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="proxy-username",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="recommendation-cookie-name",
                value_type="string",
                allow_none=True,
                default="_imp_apg_r_",
            ),
            BigipPropertySpec(
                name="redirect-path",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="redirect-response-code",
                value_type="integer",
                allow_none=True,
                default="302",
            ),
            BigipPropertySpec(name="telemetry-path", value_type="string"),
            BigipPropertySpec(
                name="use-proxy-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="use-sni",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
