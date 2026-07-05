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
            "net_stp_globals",
            module="net",
            object_types=("stp-globals",),
        ),
        header_types=(("net", "stp-globals"),),
        properties=(
            BigipPropertySpec(name="config-name", value_type="reference"),
            BigipPropertySpec(name="config-revision", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="fwd-delay",
                value_type="integer",
                default="15 seconds, and the valid range is 4 to 30",
            ),
            BigipPropertySpec(
                name="hello-time",
                value_type="integer",
                default="2 seconds, and the valid range is 1 - 10",
            ),
            BigipPropertySpec(
                name="max-age",
                value_type="integer",
                default="20 seconds, and the valid range is 6-40 seconds",
            ),
            BigipPropertySpec(name="max-hops", value_type="integer"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "mstp", "passthru", "rstp", "stp"),
            ),
            BigipPropertySpec(name="transmit-hold", value_type="integer"),
        ),
    )
