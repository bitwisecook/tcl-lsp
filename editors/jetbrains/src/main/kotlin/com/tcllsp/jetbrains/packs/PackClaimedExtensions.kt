// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.packs

/**
 * The pack-claimed extensions that currently resolve to one of the plugin's
 * file types.
 *
 * Read by `TclFileType.isSupported`, which decides both whether to start the
 * language server for an opened file and which files the server descriptor
 * treats as its own. Associating an extension without this would open the
 * file as Tcl and leave it with no server attached, which is the same
 * half-working state the registration exists to fix.
 *
 * A plain holder rather than a call into the association service: file types
 * are registered during IDE bootstrap, long before an application service is
 * a reasonable thing to ask for.
 */
object PackClaimedExtensions {
    @Volatile
    private var extensions: Set<String> = emptySet()

    fun contains(extension: String): Boolean = extension in extensions

    fun replaceWith(claimed: Set<String>) {
        extensions = claimed
    }
}
