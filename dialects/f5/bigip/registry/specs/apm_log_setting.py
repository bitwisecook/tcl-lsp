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
            "apm_log_setting",
            module="apm",
            object_types=("log-setting",),
        ),
        header_types=(("apm", "log-setting"),),
        properties=(
            BigipPropertySpec(
                name="access",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="log-level",
                        value_type="unknown",
                        in_sections=("access",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="publisher", value_type="string", in_sections=("access",)
                    ),
                ),
            ),
            BigipPropertySpec(
                name="log-level",
                value_type="unknown",
                in_sections=("access",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="access-control",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="access-per-request",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="apm-acl",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="eca",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="paa",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="sso",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="swg",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="access-control",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="access-per-request",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="apm-acl",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="eca",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="paa",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="sso",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="swg",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(name="publisher", value_type="string", in_sections=("access",)),
            BigipPropertySpec(name="description", value_type="unknown"),
            BigipPropertySpec(
                name="url-filters",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="filter",
                        value_type="unknown",
                        in_sections=("url-filters",),
                        enum_values=("false", "true"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="publisher",
                        value_type="string",
                        in_sections=("url-filters",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="filter",
                value_type="unknown",
                in_sections=("url-filters",),
                enum_values=("false", "true"),
                shape_kind="object",
            ),
            BigipPropertySpec(name="publisher", value_type="string", in_sections=("url-filters",)),
        ),
    )
