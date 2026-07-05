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
            "sys_icall_handler_periodic",
            module="sys",
            object_types=("icall handler periodic",),
        ),
        header_types=(("sys", "icall handler periodic"),),
        properties=(
            BigipPropertySpec(
                name="arguments",
                value_type="list",
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="value", value_type="string", in_sections=("arguments",)
                    ),
                ),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("arguments",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="first-occurrence", value_type="unknown"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="last-occurrence", value_type="unknown"),
            BigipPropertySpec(
                name="script",
                value_type="reference",
                references=(
                    "cli_script",
                    "pem_reporting_format_script",
                    "sys_application_apl_script",
                    "sys_icall_script",
                ),
            ),
            BigipPropertySpec(name="status", value_type="enum", enum_values=("active", "inactive")),
        ),
    )
