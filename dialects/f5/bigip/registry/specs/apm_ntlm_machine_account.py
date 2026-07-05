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
            "apm_ntlm_machine_account",
            module="apm",
            object_types=("ntlm machine-account",),
        ),
        header_types=(("apm", "ntlm machine-account"),),
        properties=(
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("change-password", "noop"),
            ),
            BigipPropertySpec(
                name="administrator-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="administrator-password",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="domain-controller-fqdn", value_type="unknown"),
            BigipPropertySpec(name="domain-fqdn", value_type="unknown", required=True),
            BigipPropertySpec(name="machine-account-name", value_type="string", allow_none=True),
        ),
    )
