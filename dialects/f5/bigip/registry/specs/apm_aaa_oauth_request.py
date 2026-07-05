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
            "apm_aaa_oauth_request",
            module="apm",
            object_types=("aaa oauth-request",),
        ),
        header_types=(("apm", "aaa oauth-request"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(name="value", value_type="unknown", in_sections=("headers",)),
                ),
            ),
            BigipPropertySpec(name="value", value_type="unknown", in_sections=("headers",)),
            BigipPropertySpec(name="method", value_type="enum", enum_values=("get", "post")),
            BigipPropertySpec(
                name="parameters",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="type", value_type="unknown", in_sections=("parameters",)
                    ),
                    BigipPropertySpec(
                        name="value",
                        value_type="string",
                        in_sections=("parameters",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(name="type", value_type="unknown", in_sections=("parameters",)),
            BigipPropertySpec(
                name="value",
                value_type="string",
                in_sections=("parameters",),
                allow_none=True,
            ),
            BigipPropertySpec(name="type", value_type="unknown"),
            BigipPropertySpec(name="uri", value_type="string", required=True, allow_none=True),
        ),
    )
