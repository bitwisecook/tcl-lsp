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
            "sys_ucs",
            module="sys",
            object_types=("ucs",),
        ),
        header_types=(("sys", "ucs"),),
        properties=(
            BigipPropertySpec(name="include-chassis-level-config", value_type="unknown"),
            BigipPropertySpec(
                name="load",
                value_type="reference",
                references=("gtm_global_settings_load_balancing", "load"),
            ),
            BigipPropertySpec(name="no-license", value_type="unknown"),
            BigipPropertySpec(name="no-platform-check", value_type="unknown"),
            BigipPropertySpec(name="no-private-key", value_type="unknown"),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="platform-migrate", value_type="unknown"),
            BigipPropertySpec(name="reset-trust", value_type="unknown"),
            BigipPropertySpec(name="save", value_type="reference", references=("save",)),
        ),
    )
