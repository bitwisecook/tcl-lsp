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
            "net_trunk",
            module="net",
            object_types=("trunk",),
        ),
        header_types=(("net", "trunk"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="bandwidth", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="distribution-hash",
                value_type="enum",
                enum_values=("dst-mac", "src-dst-mac"),
            ),
            BigipPropertySpec(name="interfaces", value_type="unknown"),
            BigipPropertySpec(
                name="lacp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="lacp-mode",
                value_type="enum",
                enum_values=("active", "passive"),
            ),
            BigipPropertySpec(
                name="lacp-timeout",
                value_type="enum",
                enum_values=("long", "short"),
                default="long",
            ),
            BigipPropertySpec(
                name="link-select-policy",
                value_type="enum",
                enum_values=("auto", "maximum-bandwidth"),
            ),
            BigipPropertySpec(name="mac-address", value_type="unknown"),
            BigipPropertySpec(name="qinq-ethertype", value_type="string", default="set to 0x8100"),
            BigipPropertySpec(
                name="stp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="stp-reset", value_type="unknown"),
        ),
    )
