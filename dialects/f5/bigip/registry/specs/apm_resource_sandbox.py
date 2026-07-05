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
            "apm_resource_sandbox",
            module="apm",
            object_types=("resource sandbox",),
        ),
        header_types=(("apm", "resource sandbox"),),
        properties=(
            BigipPropertySpec(name="base-uri", value_type="string"),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="files",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="content-type", value_type="string", in_sections=("files",)
                    ),
                    BigipPropertySpec(
                        name="file-type",
                        value_type="enum",
                        in_sections=("files",),
                        required=True,
                        enum_values=("citrix-bundle", "customization", "unknown"),
                    ),
                    BigipPropertySpec(name="filename", value_type="string", in_sections=("files",)),
                    BigipPropertySpec(name="folder", value_type="string", in_sections=("files",)),
                    BigipPropertySpec(
                        name="local-path", value_type="string", in_sections=("files",)
                    ),
                ),
            ),
            BigipPropertySpec(name="content-type", value_type="string", in_sections=("files",)),
            BigipPropertySpec(
                name="file-type",
                value_type="enum",
                in_sections=("files",),
                required=True,
                enum_values=("citrix-bundle", "customization", "unknown"),
            ),
            BigipPropertySpec(name="filename", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="folder", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="local-path", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
