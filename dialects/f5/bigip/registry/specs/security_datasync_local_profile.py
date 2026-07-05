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
            "security_datasync_local_profile",
            module="security",
            object_types=("datasync local-profile",),
        ),
        header_types=(("security", "datasync local-profile"),),
        properties=(
            BigipPropertySpec(name="buf-size", value_type="integer"),
            BigipPropertySpec(
                name="ds-area",
                value_type="enum",
                allow_none=True,
                enum_values=("asm", "fps", "none"),
            ),
            BigipPropertySpec(name="gen-pause-sec", value_type="integer"),
            BigipPropertySpec(name="gen-timeout-sec", value_type="integer"),
            BigipPropertySpec(name="keep-conf-files", value_type="integer"),
            BigipPropertySpec(name="max-gen-rows", value_type="integer"),
            BigipPropertySpec(name="max-iowait-percent", value_type="integer"),
            BigipPropertySpec(name="min-cpu-percent", value_type="integer"),
            BigipPropertySpec(name="min-mem-mb", value_type="integer"),
            BigipPropertySpec(
                name="offline-until-gen",
                value_type="enum",
                enum_values=("disable", "enable"),
            ),
            BigipPropertySpec(name="rows-bulk", value_type="integer"),
        ),
    )
