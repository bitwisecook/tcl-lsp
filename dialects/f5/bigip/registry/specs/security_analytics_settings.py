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
            "security_analytics_settings",
            module="security",
            object_types=("analytics settings",),
        ),
        header_types=(("security", "analytics settings"),),
        properties=(
            BigipPropertySpec(
                name="acl-rules",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="collect-client-ip",
                        value_type="enum",
                        in_sections=("acl-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="collect-client-port",
                        value_type="enum",
                        in_sections=("acl-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="collect-dest-ip",
                        value_type="enum",
                        in_sections=("acl-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="collect-dest-port",
                        value_type="enum",
                        in_sections=("acl-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="collect-server-side-stats",
                        value_type="enum",
                        in_sections=("acl-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collect-client-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collect-dest-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collect-server-side-stats",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collected-stats-external-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collected-stats-internal-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="dns",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="collect-client-ip",
                        value_type="enum",
                        in_sections=("dns",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dns",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="dos-l2-l4",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="collect-client-ip",
                        value_type="enum",
                        in_sections=("dos-l2-l4",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dos-l2-l4",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="l3-l4-errors",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="collect-client-ip",
                        value_type="enum",
                        in_sections=("l3-l4-errors",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="collect-dest-ip",
                        value_type="enum",
                        in_sections=("l3-l4-errors",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="publisher",
                value_type="reference",
                references=(
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ),
            ),
            BigipPropertySpec(name="smtp-config", value_type="reference"),
            BigipPropertySpec(
                name="stale-rules",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="collect",
                        value_type="enum",
                        in_sections=("stale-rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                in_sections=("stale-rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
