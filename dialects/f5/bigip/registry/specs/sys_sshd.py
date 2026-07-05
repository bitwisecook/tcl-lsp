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
            "sys_sshd",
            module="sys",
            object_types=("sshd",),
        ),
        header_types=(("sys", "sshd"),),
        properties=(
            BigipPropertySpec(
                name="allow",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="all",
            ),
            BigipPropertySpec(
                name="banner",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="banner-text", value_type="string"),
            BigipPropertySpec(
                name="inactivity-timeout",
                value_type="integer",
                default="0 (zero) seconds, which indicates that inactivity timeout is disabled",
            ),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=(
                    "debug",
                    "debug1",
                    "debug2",
                    "debug3",
                    "error",
                    "fatal",
                    "info",
                    "quiet",
                    "verbose",
                ),
            ),
            BigipPropertySpec(
                name="login",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="port", value_type="integer", default="22"),
        ),
    )
