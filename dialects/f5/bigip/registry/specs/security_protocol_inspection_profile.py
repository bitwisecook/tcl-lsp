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
            "security_protocol_inspection_profile",
            module="security",
            object_types=("protocol-inspection profile",),
        ),
        header_types=(("security", "protocol-inspection profile"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string"),
            BigipPropertySpec(name="auto-add-new-inspections", value_type="unknown"),
            BigipPropertySpec(name="auto-publish-suggestion", value_type="unknown"),
            BigipPropertySpec(name="avr-stat-collect", value_type="unknown"),
            BigipPropertySpec(name="common-config", value_type="string"),
            BigipPropertySpec(name="common-config-merge-type", value_type="string"),
            BigipPropertySpec(name="compliance-enable", value_type="unknown"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="string",
                references=("security_protocol_inspection_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="services", value_type="list", repeated=True),
            BigipPropertySpec(name="signature-enable", value_type="unknown"),
            BigipPropertySpec(name="staging-period", value_type="integer"),
        ),
    )
