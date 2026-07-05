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
            "ltm_global_settings_general",
            module="ltm",
            object_types=("global-settings general",),
        ),
        header_types=(("ltm", "global-settings general"),),
        properties=(
            BigipPropertySpec(name="gratuitous-arp-rate", value_type="unknown", default="0"),
            BigipPropertySpec(name="l2-cache-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(
                name="maintenance-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="mgmt-auto-lasthop",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="share-single-mac",
                value_type="enum",
                enum_values=("unique", "vmw-compat"),
                default="unique, which indicates that a VLAN uses a unique MAC address from the pool of mac addresses assigned to each hardware platform",
            ),
            BigipPropertySpec(
                name="snat-packet-forward",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
