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
            "ltm_message_routing_sip_profile_router",
            module="ltm",
            object_types=("message-routing sip profile router",),
        ),
        header_types=(("ltm", "message-routing sip profile router"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="concurrent-sessions-per-subscriber", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_sip_profile_router",),
                default="router",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dialog-establishment-timeout", value_type="integer"),
            BigipPropertySpec(
                name="inherited-traffic-group",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="log-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="max-global-registrations", value_type="integer"),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer", default="32768"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer", default="64"),
            BigipPropertySpec(name="max-retries", value_type="integer", default="1"),
            BigipPropertySpec(
                name="media-proxy",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="max-media-sessions",
                        value_type="integer",
                        in_sections=("media-proxy",),
                    ),
                    BigipPropertySpec(
                        name="media-inactivity-timeout",
                        value_type="integer",
                        in_sections=("media-proxy",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="max-media-sessions",
                value_type="integer",
                in_sections=("media-proxy",),
            ),
            BigipPropertySpec(
                name="media-inactivity-timeout",
                value_type="integer",
                in_sections=("media-proxy",),
            ),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-callout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-listener",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="operation-mode",
                value_type="enum",
                enum_values=("application-level-gateway", "load-balancing"),
            ),
            BigipPropertySpec(
                name="per-peer-stats",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="registration-timeout", value_type="integer"),
            BigipPropertySpec(
                name="routes",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="max-session-timeout",
                        value_type="integer",
                        in_sections=("session",),
                    ),
                    BigipPropertySpec(
                        name="transaction-timeout",
                        value_type="integer",
                        in_sections=("session",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="max-session-timeout",
                value_type="integer",
                in_sections=("session",),
            ),
            BigipPropertySpec(
                name="transaction-timeout",
                value_type="integer",
                in_sections=("session",),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="use-local-connection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
