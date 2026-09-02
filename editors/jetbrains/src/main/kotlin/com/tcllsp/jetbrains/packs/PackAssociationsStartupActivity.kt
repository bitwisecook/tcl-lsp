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

import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.StartupActivity

/**
 * Loads the pack-association ledger while a project opens.
 *
 * The service is otherwise instantiated by the first report, which arrives
 * from a server that only starts once a file the plugin recognises is opened.
 * A session whose only Tcl files carry a pack-claimed extension would never
 * get that far, so the extensions the ledger records have to be back in play
 * before the first file is opened.
 */
class PackAssociationsStartupActivity : StartupActivity.DumbAware {
    override fun runActivity(project: Project) {
        TclLspPackAssociations.getInstance()
    }
}
