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
            "security_datasync_global_profile",
            module="security",
            object_types=("datasync global-profile",),
        ),
        header_types=(("security", "datasync global-profile"),),
        properties=(
            BigipPropertySpec(name="activation-epoch", value_type="integer"),
            BigipPropertySpec(name="deactivation-epoch", value_type="integer"),
            BigipPropertySpec(name="grace-time", value_type="integer"),
            BigipPropertySpec(name="hash-alg", value_type="string"),
            BigipPropertySpec(name="mac-alg", value_type="string"),
            BigipPropertySpec(name="master-key", value_type="string"),
            BigipPropertySpec(name="max-rows", value_type="integer"),
            BigipPropertySpec(name="min-rows", value_type="integer"),
            BigipPropertySpec(name="mode-of-op", value_type="string"),
            BigipPropertySpec(name="params", value_type="string"),
            BigipPropertySpec(name="regen-interval", value_type="integer", allow_none=True),
            BigipPropertySpec(name="regen-time-offset", value_type="integer"),
            BigipPropertySpec(name="rsa-bits", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="rsa-exp",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none", "rsa-3", "rsa-f4"),
            ),
            BigipPropertySpec(name="scramble-alg", value_type="string"),
            BigipPropertySpec(name="table", value_type="reference"),
        ),
    )
