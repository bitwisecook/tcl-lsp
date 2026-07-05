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
            "ltm_profile_socks",
            module="ltm",
            object_types=("profile socks",),
        ),
        header_types=(("ltm", "profile socks"),),
        properties=(
            BigipPropertySpec(
                name="default-connect-handling",
                value_type="enum",
                enum_values=("allow", "deny"),
                default="deny",
            ),
            BigipPropertySpec(name="dns-resolver", value_type="unknown", default="dns-resolver"),
            BigipPropertySpec(
                name="ipv6",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no, which will try a IPv4 lookup before a IPv6",
            ),
            BigipPropertySpec(name="protocol-versions", value_type="list"),
            BigipPropertySpec(
                name="route-domain",
                value_type="unknown",
                references=("net_route_domain",),
                default="0",
            ),
            BigipPropertySpec(name="tunnel-name", value_type="unknown", default="socks-tunnel"),
        ),
    )
