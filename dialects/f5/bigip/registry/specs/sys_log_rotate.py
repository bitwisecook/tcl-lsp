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
            "sys_log_rotate",
            module="sys",
            object_types=("log-rotate",),
        ),
        header_types=(("sys", "log-rotate"),),
        properties=(
            BigipPropertySpec(name="common-backlogs", value_type="integer", default="24"),
            BigipPropertySpec(name="common-include", value_type="string", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ilx-include", value_type="string", default="none"),
            BigipPropertySpec(name="ilx-rotations", value_type="string", default="10"),
            BigipPropertySpec(name="ilx-schedule", value_type="string", default="daily"),
            BigipPropertySpec(name="ilx-size", value_type="string", default="10240 kilobytes"),
            BigipPropertySpec(name="include", value_type="string", default="none"),
            BigipPropertySpec(name="max-file-size", value_type="integer", default="1024000"),
            BigipPropertySpec(name="mysql-include", value_type="string"),
            BigipPropertySpec(name="syslog-include", value_type="string", default="none"),
            BigipPropertySpec(name="tomcat-include", value_type="string", default="none"),
            BigipPropertySpec(name="wa-include", value_type="string", default="none"),
        ),
    )
