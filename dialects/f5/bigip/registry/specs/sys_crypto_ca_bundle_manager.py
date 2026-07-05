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
            "sys_crypto_ca_bundle_manager",
            module="sys",
            object_types=("crypto ca-bundle-manager",),
        ),
        header_types=(("sys", "crypto ca-bundle-manager"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="proxy-port", value_type="unknown"),
            BigipPropertySpec(name="proxy-server", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="time-out", value_type="unknown", default="8 seconds"),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="unknown"),
            BigipPropertySpec(
                name="update-interval",
                value_type="unknown",
                default="0, which means the generated ca-bundle is not dynamically updated",
            ),
            BigipPropertySpec(
                name="update-now",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
        ),
    )
