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
            "security_ip_intelligence_feed_list",
            module="security",
            object_types=("ip-intelligence feed-list",),
        ),
        header_types=(("security", "ip-intelligence feed-list"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feeds",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="default-blacklist-category",
                        value_type="string",
                        in_sections=("feeds",),
                    ),
                    BigipPropertySpec(
                        name="default-list-type",
                        value_type="enum",
                        in_sections=("feeds",),
                        enum_values=("blacklist", "whitelist"),
                    ),
                    BigipPropertySpec(
                        name="poll",
                        value_type="unknown",
                        in_sections=("feeds",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="default-blacklist-category",
                value_type="string",
                in_sections=("feeds",),
            ),
            BigipPropertySpec(
                name="default-list-type",
                value_type="enum",
                in_sections=("feeds",),
                enum_values=("blacklist", "whitelist"),
            ),
            BigipPropertySpec(
                name="poll",
                value_type="unknown",
                in_sections=("feeds",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="interval",
                        value_type="integer",
                        in_sections=("feeds", "poll"),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="string",
                        in_sections=("feeds", "poll"),
                    ),
                    BigipPropertySpec(
                        name="url", value_type="string", in_sections=("feeds", "poll")
                    ),
                    BigipPropertySpec(
                        name="user", value_type="string", in_sections=("feeds", "poll")
                    ),
                ),
            ),
            BigipPropertySpec(name="interval", value_type="integer", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="password", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="url", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="user", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(
                name="load",
                value_type="unknown",
                references=("gtm_global_settings_load_balancing", "load"),
                shape_kind="object",
            ),
        ),
    )
