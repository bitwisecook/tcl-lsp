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
            "apm_aaa_http_connector_request",
            module="apm",
            object_types=("aaa http-connector-request",),
        ),
        header_types=(("apm", "aaa http-connector-request"),),
        properties=(
            BigipPropertySpec(
                name="auth",
                value_type="enum",
                allow_none=True,
                enum_values=("basic", "bearer", "custom", "none"),
                default="none",
            ),
            BigipPropertySpec(name="method", value_type="string", required=True),
            BigipPropertySpec(name="password", value_type="string", allow_none=True),
            BigipPropertySpec(name="request-body", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="request-headers",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="response-action",
                value_type="enum",
                enum_values=("ignore", "parse", "save"),
            ),
            BigipPropertySpec(name="response-headers", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="secure-variables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="token", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="transport",
                value_type="reference",
                required=True,
                references=(
                    "apm_aaa_http_connector_transport",
                    "ltm_message_routing_diameter_transport_config",
                    "ltm_message_routing_generic_transport_config",
                    "ltm_message_routing_mqtt_transport_config",
                    "ltm_message_routing_sip_transport_config",
                ),
            ),
            BigipPropertySpec(name="url", value_type="string", required=True),
            BigipPropertySpec(name="username", value_type="string", allow_none=True),
        ),
    )
