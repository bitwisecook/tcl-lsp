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
            "ltm_message_routing_generic_protocol",
            module="ltm",
            object_types=("message-routing generic protocol",),
        ),
        header_types=(("ltm", "message-routing generic protocol"),),
        properties=(
            BigipPropertySpec(
                name="cur-pending-req-sweeper-interval",
                value_type="integer",
                default="60000ms",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_generic_protocol",),
                default="genericmsg",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="disable-parser",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="max-egress-buffer", value_type="integer", default="32768"),
            BigipPropertySpec(name="max-message-size", value_type="integer", default="32768"),
            BigipPropertySpec(name="message-terminator", value_type="string", default="en"),
            BigipPropertySpec(
                name="no-response",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="transaction-timeout",
                value_type="integer",
                default="10seconds",
            ),
        ),
    )
