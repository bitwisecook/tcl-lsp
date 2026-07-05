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
            "ltm_lsn_log_profile",
            module="ltm",
            object_types=("lsn-log-profile",),
        ),
        header_types=(("ltm", "lsn-log-profile"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="csv-format",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="end-inbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("end-inbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-inbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="end-outbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("end-outbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                    BigipPropertySpec(
                        name="elements",
                        value_type="list",
                        in_sections=("end-outbound-session",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="list",
                in_sections=("end-outbound-session",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("end-outbound-session", "elements"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("end-outbound-session", "elements"),
            ),
            BigipPropertySpec(
                name="errors",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("errors",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("errors",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="quota-exceeded",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("quota-exceeded",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("quota-exceeded",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="start-inbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("start-inbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-inbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="start-outbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("start-outbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                    BigipPropertySpec(
                        name="elements",
                        value_type="list",
                        in_sections=("start-outbound-session",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="list",
                in_sections=("start-outbound-session",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("start-outbound-session", "elements"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("start-outbound-session", "elements"),
            ),
            BigipPropertySpec(
                name="subscriber-id",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
