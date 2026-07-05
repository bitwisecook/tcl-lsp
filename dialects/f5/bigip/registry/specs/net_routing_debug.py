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
            "net_routing_debug",
            module="net",
            object_types=("routing debug",),
        ),
        header_types=(("net", "routing debug"),),
        properties=(
            BigipPropertySpec(
                name="bfd",
                value_type="enum",
                enum_values=("all", "event", "ipc-error", "ipc-event", "nsm", "packet", "session"),
            ),
            BigipPropertySpec(
                name="bgp",
                value_type="enum",
                enum_values=(
                    "all",
                    "bfd",
                    "dampening",
                    "events",
                    "filters",
                    "fsm",
                    "keepalives",
                    "nht",
                    "nsm",
                    "updates",
                    "updates-in",
                    "updates-out",
                ),
            ),
            BigipPropertySpec(
                name="nsm",
                value_type="enum",
                enum_values=(
                    "all",
                    "bfd",
                    "events",
                    "ha",
                    "ha-all",
                    "kernel",
                    "packet",
                    "packet-detail",
                    "packet-recv",
                    "packet-send",
                ),
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
        ),
    )
