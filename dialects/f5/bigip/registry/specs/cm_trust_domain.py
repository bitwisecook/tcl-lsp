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
            "cm_trust_domain",
            module="cm",
            object_types=("trust-domain",),
        ),
        header_types=(("cm", "trust-domain"),),
        properties=(
            BigipPropertySpec(
                name="add-device",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="device-ip",
                        value_type="string",
                        in_sections=("add-device",),
                    ),
                    BigipPropertySpec(
                        name="device-name",
                        value_type="string",
                        in_sections=("add-device",),
                    ),
                    BigipPropertySpec(
                        name="device-port",
                        value_type="unknown",
                        in_sections=("add-device",),
                        usage_flags=frozenset(("optional",)),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="string",
                        in_sections=("add-device",),
                        required=True,
                    ),
                    BigipPropertySpec(
                        name="sha1-fingerprint",
                        value_type="string",
                        in_sections=("add-device",),
                        usage_flags=frozenset(("optional",)),
                    ),
                    BigipPropertySpec(
                        name="username",
                        value_type="string",
                        in_sections=("add-device",),
                        required=True,
                    ),
                ),
            ),
            BigipPropertySpec(name="device-ip", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(name="device-name", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="device-port",
                value_type="unknown",
                in_sections=("add-device",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="password",
                value_type="string",
                in_sections=("add-device",),
                required=True,
            ),
            BigipPropertySpec(
                name="sha1-fingerprint",
                value_type="string",
                in_sections=("add-device",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="username",
                value_type="string",
                in_sections=("add-device",),
                required=True,
            ),
            BigipPropertySpec(
                name="ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="deprecated", value_type="unknown"),
            BigipPropertySpec(
                name="devices",
                value_type="list",
                list_operators=frozenset(("delete",)),
            ),
            BigipPropertySpec(
                name="md5-fingerprint",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="non-ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="password", value_type="string", required=True),
            BigipPropertySpec(name="remove-device", value_type="string"),
            BigipPropertySpec(
                name="serial",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="sha1-fingerprint",
                value_type="string",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="username", value_type="string", required=True),
        ),
    )
