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
            "sys_global_settings",
            module="sys",
            object_types=("global-settings",),
        ),
        header_types=(("sys", "global-settings"),),
        properties=(
            BigipPropertySpec(name="aws-access-key", value_type="string", default="none"),
            BigipPropertySpec(name="aws-api-max-concurrency", value_type="integer", default="1"),
            BigipPropertySpec(name="aws-secret-key", value_type="string", default="none"),
            BigipPropertySpec(
                name="console-inactivity-timeout",
                value_type="integer",
                default="0 (zero), which means that no timeout is set",
            ),
            BigipPropertySpec(
                name="custom-addr",
                value_type="string",
                shape_kind="ip-address",
                default="::",
            ),
            BigipPropertySpec(name="description", value_type="string", default="no description"),
            BigipPropertySpec(
                name="failsafe-action",
                value_type="enum",
                enum_values=(
                    "failover-restart-tm",
                    "go-offline",
                    "go-offline-restart-tm",
                    "reboot",
                    "restart-all",
                ),
                default="go-offline-restart-tm",
            ),
            BigipPropertySpec(name="file-blacklist-path-prefix", value_type="string"),
            BigipPropertySpec(
                name="file-blacklist-read-only-path-prefix",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name="file-local-path-prefix", value_type="unknown"),
            BigipPropertySpec(name="file-whitelist-path-prefix", value_type="string"),
            BigipPropertySpec(
                name="gui-audit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="gui-expired-cert-alert",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="gui-security-banner",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="gui-security-banner-text",
                value_type="string",
                default="Welcome to the BIG-IP Configuration Utility",
            ),
            BigipPropertySpec(
                name="gui-setup",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="host-addr-mode",
                value_type="enum",
                enum_values=("custom", "management", "state-mirror"),
                default="management",
            ),
            BigipPropertySpec(name="hostname", value_type="string", default="bigip1"),
            BigipPropertySpec(name="hosts-allow-include", value_type="string"),
            BigipPropertySpec(
                name="lcd-display",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="mgmt-dhcp",
                value_type="enum",
                enum_values=("dhcpv4", "dhcpv6", "disabled", "enabled"),
                default="enabled for VE and disabled for all other platforms",
            ),
            BigipPropertySpec(
                name="net-reboot",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="password-prompt", value_type="string"),
            BigipPropertySpec(
                name="quiet-boot",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="remote-host",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
                block=(
                    BigipPropertySpec(
                        name="addr",
                        value_type="string",
                        in_sections=("remote-host",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="hostname",
                        value_type="string",
                        in_sections=("remote-host",),
                        default="bigip1",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="addr",
                value_type="string",
                in_sections=("remote-host",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                in_sections=("remote-host",),
                default="bigip1",
            ),
            BigipPropertySpec(
                name="ssh-max-session-limit",
                value_type="integer",
                default="10 and the range is 1 to 65535",
            ),
            BigipPropertySpec(name="ssh-max-session-limit-per-user", value_type="integer"),
            BigipPropertySpec(
                name="ssh-root-session-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="ssh-session-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="username-prompt", value_type="string"),
        ),
    )
