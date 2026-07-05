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
            "apm_policy_agent_logon_page",
            module="apm",
            object_types=("policy agent logon-page",),
        ),
        header_types=(("apm", "policy agent logon-page"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="basic-auth-realm", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="clean-sess-var1",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="clean-sess-var2",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="clean-sess-var3",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="clean-sess-var4",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="clean-sess-var5",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-modifiable1",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="field-modifiable2",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="field-modifiable3",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="field-modifiable4",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="field-modifiable5",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="field-type1",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type2",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type3",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type4",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type5",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="http-401-auth-level",
                value_type="enum",
                allow_none=True,
                enum_values=("basic", "basic-negotiate", "negotiate", "none"),
            ),
            BigipPropertySpec(name="post-var-name1", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name2", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name3", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name4", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name5", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name1", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name2", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name3", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name4", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name5", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="split-username",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("401", "form-based")),
        ),
    )
