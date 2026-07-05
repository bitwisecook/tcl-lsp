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
            "cli_preference",
            module="cli",
            object_types=("preference",),
        ),
        header_types=(("cli", "preference"),),
        properties=(
            BigipPropertySpec(name="alias-path", value_type="unknown"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="confirm-edit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="display-threshold", value_type="integer"),
            BigipPropertySpec(name="editor", value_type="enum", enum_values=("nano", "vi")),
            BigipPropertySpec(
                name="fully-qualified-host",
                value_type="unknown",
                default="to not display this information",
            ),
            BigipPropertySpec(
                name="history-date-time",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="history-file-size", value_type="integer"),
            BigipPropertySpec(name="history-size", value_type="integer", default="500"),
            BigipPropertySpec(
                name="keymap",
                value_type="enum",
                enum_values=("default", "emacs", "vi"),
                default="default",
            ),
            BigipPropertySpec(
                name="list-all-properties",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="mcp-state",
                value_type="unknown",
                allow_none=True,
                default="to not display this information",
            ),
            BigipPropertySpec(
                name="pager",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="prompt", value_type="unknown", shape_kind="object"),
            BigipPropertySpec(
                name="show-aliases",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="stat-units",
                value_type="enum",
                enum_values=(
                    "default",
                    "exa",
                    "gig",
                    "kil",
                    "meg",
                    "peta",
                    "raw",
                    "tera",
                    "yotta",
                    "zetta",
                ),
            ),
            BigipPropertySpec(
                name="suppress-warnings",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "config-version", "none"),
                default="none",
            ),
            BigipPropertySpec(name="table-indent-width", value_type="integer"),
            BigipPropertySpec(
                name="tcl-syntax-highlighting",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="video",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="warn",
                value_type="enum",
                enum_values=("bell", "disabled", "visual-bell"),
                default="bell",
            ),
        ),
    )
