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
            "sys_log_config_destination_ipfix",
            module="sys",
            object_types=("log-config destination ipfix",),
        ),
        header_types=(("sys", "log-config destination ipfix"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="pool-name", value_type="string", required=True),
            BigipPropertySpec(
                name="protocol-version",
                value_type="enum",
                enum_values=("ipfix", "netflow-9"),
                default="ipfix",
            ),
            BigipPropertySpec(
                name="serverssl-profile",
                value_type="reference",
                default="not to use a server-side SSL profile",
            ),
            BigipPropertySpec(name="template-delete-delay", value_type="integer"),
            BigipPropertySpec(
                name="template-retransmit-interval",
                value_type="integer",
                default="30 seconds",
            ),
            BigipPropertySpec(
                name="transport-profile",
                value_type="reference",
                default="the default udp profile",
            ),
        ),
    )
