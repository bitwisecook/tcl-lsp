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
            "net_tunnels_fec",
            module="net",
            object_types=("tunnels fec",),
        ),
        header_types=(("net", "tunnels fec"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="decode-idle-timeout",
                value_type="integer",
                default="1500 milliseconds",
            ),
            BigipPropertySpec(name="decode-max-packets", value_type="integer", default="512"),
            BigipPropertySpec(name="decode-queues", value_type="integer", default="32"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("net_tunnels_fec",),
                default="fec",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encode-max-delay",
                value_type="integer",
                default="500 microseconds",
            ),
            BigipPropertySpec(name="keepalive-interval", value_type="integer", default="5 seconds"),
            BigipPropertySpec(
                name="lzo",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="repair-adaptive",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="repair-packets", value_type="integer", default="15"),
            BigipPropertySpec(
                name="source-adaptive",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="source-packets", value_type="integer", default="15"),
            BigipPropertySpec(name="udp-port", value_type="integer", default="8288"),
        ),
    )
