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
            "sys_config",
            module="sys",
            object_types=("config",),
        ),
        header_types=(("sys", "config"),),
        properties=(
            BigipPropertySpec(name="base", value_type="unknown"),
            BigipPropertySpec(name="binary", value_type="unknown"),
            BigipPropertySpec(name="current-partition", value_type="unknown"),
            BigipPropertySpec(name="exclude-gtm", value_type="unknown"),
            BigipPropertySpec(name="file", value_type="unknown"),
            BigipPropertySpec(name="files-folder", value_type="unknown"),
            BigipPropertySpec(name="from-terminal", value_type="unknown"),
            BigipPropertySpec(name="gtm-only", value_type="unknown"),
            BigipPropertySpec(name="load", value_type="unknown"),
            BigipPropertySpec(name="merge", value_type="unknown"),
            BigipPropertySpec(name="no-passphrase", value_type="unknown"),
            BigipPropertySpec(name="partitions", value_type="unknown"),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="replace", value_type="unknown"),
            BigipPropertySpec(name="save", value_type="unknown"),
            BigipPropertySpec(name="tar-file", value_type="unknown"),
            BigipPropertySpec(name="time-stamp", value_type="unknown"),
            BigipPropertySpec(name="user-only", value_type="unknown"),
            BigipPropertySpec(name="verify", value_type="unknown"),
            BigipPropertySpec(name="wait", value_type="unknown"),
        ),
    )
