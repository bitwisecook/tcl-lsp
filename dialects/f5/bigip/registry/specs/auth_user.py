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
            "auth_user",
            module="auth",
            object_types=("user",),
        ),
        header_types=(("auth", "user"),),
        properties=(
            BigipPropertySpec(name="description", value_type="unknown", repeated=True),
            BigipPropertySpec(
                name="partition-access",
                value_type="list",
                references=("auth_partition",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="prompt-for-password", value_type="unknown"),
            BigipPropertySpec(name="session-limit", value_type="integer"),
            BigipPropertySpec(name="shell", value_type="reference"),
        ),
    )
