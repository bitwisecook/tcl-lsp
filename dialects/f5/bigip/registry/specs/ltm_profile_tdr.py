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
            "ltm_profile_tdr",
            module="ltm",
            object_types=("profile tdr",),
        ),
        header_types=(("ltm", "profile tdr"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_tdr",),
                default="http2",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="filters",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("filters",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="condition-pattern-1",
                        value_type="unknown",
                        in_sections=("filters",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="condition-pattern-2",
                        value_type="unknown",
                        in_sections=("filters",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="condition-pattern-3",
                        value_type="unknown",
                        in_sections=("filters",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="condition-pattern-4",
                        value_type="unknown",
                        in_sections=("filters",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("filters",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="message-type",
                        value_type="enum",
                        in_sections=("filters",),
                        enum_values=("all", "answer", "request"),
                    ),
                    BigipPropertySpec(
                        name="tdr-format",
                        value_type="string",
                        in_sections=("filters",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="traffic-direction",
                        value_type="enum",
                        in_sections=("filters",),
                        enum_values=("all", "egress", "ingress"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("filters",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="condition-pattern-1",
                value_type="unknown",
                in_sections=("filters",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="cmp-operator",
                        value_type="enum",
                        in_sections=("filters", "condition-pattern-1"),
                        allow_none=True,
                        enum_values=(
                            "contains",
                            "ends-with",
                            "equal",
                            "none",
                            "not-equal",
                            "starts-with",
                        ),
                    ),
                    BigipPropertySpec(
                        name="field-name",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-1"),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="field-value",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-1"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("filters", "condition-pattern-1"),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="string",
                in_sections=("filters", "condition-pattern-1"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="string",
                in_sections=("filters", "condition-pattern-1"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-2",
                value_type="unknown",
                in_sections=("filters",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="cmp-operator",
                        value_type="enum",
                        in_sections=("filters", "condition-pattern-2"),
                        allow_none=True,
                        enum_values=(
                            "contains",
                            "ends-with",
                            "equal",
                            "none",
                            "not-equal",
                            "starts-with",
                        ),
                    ),
                    BigipPropertySpec(
                        name="field-name",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-2"),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="field-value",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-2"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("filters", "condition-pattern-2"),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="string",
                in_sections=("filters", "condition-pattern-2"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="string",
                in_sections=("filters", "condition-pattern-2"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-3",
                value_type="unknown",
                in_sections=("filters",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="cmp-operator",
                        value_type="enum",
                        in_sections=("filters", "condition-pattern-3"),
                        allow_none=True,
                        enum_values=(
                            "contains",
                            "ends-with",
                            "equal",
                            "none",
                            "not-equal",
                            "starts-with",
                        ),
                    ),
                    BigipPropertySpec(
                        name="field-name",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-3"),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="field-value",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-3"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("filters", "condition-pattern-3"),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="string",
                in_sections=("filters", "condition-pattern-3"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="string",
                in_sections=("filters", "condition-pattern-3"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-4",
                value_type="unknown",
                in_sections=("filters",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="cmp-operator",
                        value_type="enum",
                        in_sections=("filters", "condition-pattern-4"),
                        allow_none=True,
                        enum_values=(
                            "contains",
                            "ends-with",
                            "equal",
                            "none",
                            "not-equal",
                            "starts-with",
                        ),
                    ),
                    BigipPropertySpec(
                        name="field-name",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-4"),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="field-value",
                        value_type="string",
                        in_sections=("filters", "condition-pattern-4"),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("filters", "condition-pattern-4"),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="string",
                in_sections=("filters", "condition-pattern-4"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="string",
                in_sections=("filters", "condition-pattern-4"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                in_sections=("filters",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="message-type",
                value_type="enum",
                in_sections=("filters",),
                enum_values=("all", "answer", "request"),
            ),
            BigipPropertySpec(
                name="tdr-format",
                value_type="string",
                in_sections=("filters",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="traffic-direction",
                value_type="enum",
                in_sections=("filters",),
                enum_values=("all", "egress", "ingress"),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
        ),
    )
