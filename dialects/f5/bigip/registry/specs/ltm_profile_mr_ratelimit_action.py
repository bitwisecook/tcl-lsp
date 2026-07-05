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
            "ltm_profile_mr_ratelimit_action",
            module="ltm",
            object_types=("profile mr-ratelimit-action",),
        ),
        header_types=(("ltm", "profile mr-ratelimit-action"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_mr_ratelimit_action",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="priority-1",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "delay-100",
                    "delay-25",
                    "delay-50",
                    "drop-100",
                    "drop-25",
                    "drop-50",
                    "none",
                    "return-100",
                    "return-25",
                    "return-50",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="priority-2",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "delay-100",
                    "delay-25",
                    "delay-50",
                    "drop-100",
                    "drop-25",
                    "drop-50",
                    "none",
                    "return-100",
                    "return-25",
                    "return-50",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="priority-3",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "delay-100",
                    "delay-25",
                    "delay-50",
                    "drop-100",
                    "drop-25",
                    "drop-50",
                    "none",
                    "return-100",
                    "return-25",
                    "return-50",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="priority-4",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "delay-100",
                    "delay-25",
                    "delay-50",
                    "drop-100",
                    "drop-25",
                    "drop-50",
                    "none",
                    "return-100",
                    "return-25",
                    "return-50",
                ),
                default="none",
            ),
        ),
    )
