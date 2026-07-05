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
            "sys_application_template",
            module="sys",
            object_types=("application template",),
        ),
        header_types=(("sys", "application template"),),
        properties=(
            BigipPropertySpec(
                name="actions",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="definition",
                        value_type="unknown",
                        in_sections=("actions",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="definition",
                value_type="unknown",
                in_sections=("actions",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="html-help",
                        value_type="string",
                        in_sections=("actions", "definition"),
                    ),
                    BigipPropertySpec(
                        name="implementation",
                        value_type="unknown",
                        in_sections=("actions", "definition"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="presentation",
                        value_type="unknown",
                        in_sections=("actions", "definition"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="role-acl",
                        value_type="list",
                        in_sections=("actions", "definition"),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="run-as",
                        value_type="string",
                        in_sections=("actions", "definition"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="html-help",
                value_type="string",
                in_sections=("actions", "definition"),
            ),
            BigipPropertySpec(
                name="implementation",
                value_type="unknown",
                in_sections=("actions", "definition"),
                shape_kind="object",
            ),
            BigipPropertySpec(
                name="presentation",
                value_type="unknown",
                in_sections=("actions", "definition"),
                shape_kind="object",
            ),
            BigipPropertySpec(
                name="role-acl",
                value_type="list",
                in_sections=("actions", "definition"),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="run-as",
                value_type="string",
                in_sections=("actions", "definition"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="metadata",
                value_type="unknown",
                default="persistent, which saves the data into the config file",
            ),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="requires-bigip-version-max",
                value_type="string",
                required=True,
            ),
            BigipPropertySpec(
                name="requires-bigip-version-min",
                value_type="string",
                required=True,
            ),
            BigipPropertySpec(
                name="requires-modules",
                value_type="list",
                required=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="ignore-verification",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="signing-key",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="tmpl-checksum",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="tmpl-signature",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
