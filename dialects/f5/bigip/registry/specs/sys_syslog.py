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
            "sys_syslog",
            module="sys",
            object_types=("syslog",),
        ),
        header_types=(("sys", "syslog"),),
        properties=(
            BigipPropertySpec(
                name="auth-priv-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="auth-priv-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(
                name="clustered-host-name",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="clustered-message-slot",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="console-log",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="cron-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="warning",
            ),
            BigipPropertySpec(
                name="cron-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(
                name="daemon-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="daemon-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="iso-date",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="kern-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="debug",
            ),
            BigipPropertySpec(
                name="kern-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(
                name="local6-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="local6-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(
                name="mail-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="mail-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
            BigipPropertySpec(
                name="messages-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="messages-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="warning",
            ),
            BigipPropertySpec(
                name="remote-servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
                block=(
                    BigipPropertySpec(
                        name="host",
                        value_type="unknown",
                        in_sections=("remote-servers",),
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="local-ip",
                        value_type="string",
                        in_sections=("remote-servers",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="remote-port",
                        value_type="unknown",
                        in_sections=("remote-servers",),
                        default="514",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="host",
                value_type="unknown",
                in_sections=("remote-servers",),
                default="none",
            ),
            BigipPropertySpec(
                name="local-ip",
                value_type="string",
                in_sections=("remote-servers",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="remote-port",
                value_type="unknown",
                in_sections=("remote-servers",),
                default="514",
            ),
            BigipPropertySpec(
                name="user-log-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="notice",
            ),
            BigipPropertySpec(
                name="user-log-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
                default="emerg",
            ),
        ),
    )
