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

"""Per-command bytecoded codegen hooks."""


def register_all() -> None:
    """Register all per-command codegen hooks on the REGISTRY."""
    from . import _array, _control, _dict, _info, _list, _misc, _namespace, _regexp, _string, _var

    _var.register()
    _string.register()
    _list.register()
    _dict.register()
    _regexp.register()
    _control.register()
    _info.register()
    _namespace.register()
    _array.register()
    _misc.register()
