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
            "ltm_global_settings_connection",
            module="ltm",
            object_types=("global-settings connection",),
        ),
        header_types=(("ltm", "global-settings connection"),),
        properties=(
            BigipPropertySpec(
                name="adaptive-reaper-hiwater",
                value_type="integer",
                default="95",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="adaptive-reaper-lowater",
                value_type="integer",
                default="85",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="auto-last-hop",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="default-vs-syn-challenge-threshold",
                value_type="integer",
                enum_values=("infinite",),
                default="infinite",
            ),
            BigipPropertySpec(name="global-flow-eviction-policy", value_type="reference"),
            BigipPropertySpec(
                name="global-syn-challenge-threshold",
                value_type="integer",
                enum_values=("infinite",),
                default="64K",
            ),
            BigipPropertySpec(
                name="syncookies-threshold",
                value_type="integer",
                default="16384",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="vlan-keyed-conn",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="vlan-syn-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
