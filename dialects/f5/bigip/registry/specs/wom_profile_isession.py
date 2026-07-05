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
            "wom_profile_isession",
            module="wom",
            object_types=("profile isession",),
        ),
        header_types=(("wom", "profile isession"),),
        properties=(
            BigipPropertySpec(
                name="adaptive-compression",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="compression",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="compression-codecs",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="bzip2",
                        value_type="unknown",
                        in_sections=("compression-codecs",),
                    ),
                    BigipPropertySpec(
                        name="deflate",
                        value_type="unknown",
                        in_sections=("compression-codecs",),
                    ),
                    BigipPropertySpec(
                        name="lzo",
                        value_type="unknown",
                        in_sections=("compression-codecs",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="bzip2",
                value_type="unknown",
                in_sections=("compression-codecs",),
            ),
            BigipPropertySpec(
                name="deflate",
                value_type="unknown",
                in_sections=("compression-codecs",),
            ),
            BigipPropertySpec(
                name="lzo",
                value_type="unknown",
                in_sections=("compression-codecs",),
            ),
            BigipPropertySpec(
                name="data-encryption",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="deduplication",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("wom_profile_isession",),
                default="isession",
            ),
            BigipPropertySpec(name="deflate-compression-level", value_type="integer", default="1"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="port-transparency",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="reuse-connection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="target-virtual",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "host-match-all",
                    "host-match-no-isession",
                    "none",
                    "virtual-match-all",
                ),
                default="virtual-match-all",
            ),
        ),
    )
