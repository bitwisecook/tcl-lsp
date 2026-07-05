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
            "apm_configuration_captcha",
            module="apm",
            object_types=("configuration captcha",),
        ),
        header_types=(("apm", "configuration captcha"),),
        properties=(
            BigipPropertySpec(
                name="captcha-data-size",
                value_type="enum",
                enum_values=("data-size-compact", "data-size-normal"),
                default="data-size-normal",
            ),
            BigipPropertySpec(
                name="captcha-data-theme",
                value_type="enum",
                enum_values=("data-theme-dark", "data-theme-light"),
                default="data-theme-light",
            ),
            BigipPropertySpec(
                name="captcha-data-type",
                value_type="enum",
                enum_values=("data-type-audio", "data-type-image"),
                default="data-type-image",
            ),
            BigipPropertySpec(
                name="captcha-theme",
                value_type="enum",
                enum_values=(
                    "theme-blackglass",
                    "theme-clean",
                    "theme-custom",
                    "theme-red",
                    "theme-white",
                ),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="challenge-url", value_type="string", default="www"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="exposition-threshold", value_type="integer", default="0"),
            BigipPropertySpec(name="noscript-url", value_type="string", default="www"),
            BigipPropertySpec(
                name="private-key",
                value_type="unknown",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="proceed-on-verification-error",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="public-key",
                value_type="unknown",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="secret", value_type="unknown", required=True),
            BigipPropertySpec(name="site-key", value_type="unknown", required=True),
            BigipPropertySpec(
                name="track-by-ip",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="track-by-username",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="verification-url", value_type="string", default="www"),
        ),
    )
