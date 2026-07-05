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
            "auth_password_policy",
            module="auth",
            object_types=("password-policy",),
        ),
        header_types=(("auth", "password-policy"),),
        properties=(
            BigipPropertySpec(name="expiration-warning", value_type="integer", default="7 days"),
            BigipPropertySpec(name="lockout-duration", value_type="integer"),
            BigipPropertySpec(name="max-duration", value_type="integer", default="99999"),
            BigipPropertySpec(
                name="max-login-failures",
                value_type="integer",
                default="0 (zero - disabled)",
            ),
            BigipPropertySpec(name="min-duration", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="minimum-length", value_type="integer", default="6"),
            BigipPropertySpec(name="password-memory", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(
                name="policy-enforcement",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="required-lowercase", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="required-numeric", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="required-special", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="required-uppercase", value_type="integer", default="0 (zero)"),
        ),
    )
