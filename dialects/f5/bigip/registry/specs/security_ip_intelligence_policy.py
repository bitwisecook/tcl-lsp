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
            "security_ip_intelligence_policy",
            module="security",
            object_types=("ip-intelligence policy",),
        ),
        header_types=(("security", "ip-intelligence policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(
                name="blacklist-categories",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("accept", "drop", "use-policy-setting"),
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="unknown",
                        in_sections=("blacklist-categories",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="unknown",
                        in_sections=("blacklist-categories",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="log-blacklist-hit-only",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("no", "use-policy-setting", "yes"),
                    ),
                    BigipPropertySpec(
                        name="log-blacklist-whitelist-hit",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("no", "use-policy-setting", "yes"),
                    ),
                    BigipPropertySpec(
                        name="match-direction-override",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=(
                            "match-destination",
                            "match-source",
                            "match-source-and-destination",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("accept", "drop", "use-policy-setting"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="unknown",
                in_sections=("blacklist-categories",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="unknown",
                in_sections=("blacklist-categories",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="log-blacklist-hit-only",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("no", "use-policy-setting", "yes"),
            ),
            BigipPropertySpec(
                name="log-blacklist-whitelist-hit",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("no", "use-policy-setting", "yes"),
            ),
            BigipPropertySpec(
                name="match-direction-override",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("match-destination", "match-source", "match-source-and-destination"),
            ),
            BigipPropertySpec(
                name="default-action",
                value_type="enum",
                enum_values=("accept", "drop"),
            ),
            BigipPropertySpec(
                name="default-log-blacklist-hit-only",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="default-log-blacklist-whitelist-hit",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feed-lists",
                value_type="list",
                references=("security_ip_intelligence_feed_list",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
