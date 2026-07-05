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
            "sys_crypto_csr",
            module="sys",
            object_types=("crypto csr",),
        ),
        header_types=(("sys", "crypto csr"),),
        properties=(
            BigipPropertySpec(name="admin-email-address", value_type="string"),
            BigipPropertySpec(name="basic-constraints", value_type="string"),
            BigipPropertySpec(name="challenge-password", value_type="string"),
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="consumer", value_type="unknown"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(name="email-address", value_type="string"),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(name="key-usage", value_type="string"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
        ),
    )
