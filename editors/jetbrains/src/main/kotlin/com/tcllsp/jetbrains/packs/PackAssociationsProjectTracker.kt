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

import com.intellij.openapi.Disposable
import com.intellij.openapi.components.Service
import com.intellij.openapi.project.Project

/**
 * Tells the association service when a project goes away.
 *
 * A project service is disposed with its project, which is the only callback
 * that arrives at the moment a project stops claiming anything. Without it a
 * closed project's claims survive in the service until some *other* project
 * happens to report, so an extension nothing claims any more stays associated
 * — and, because the ledger is persisted, into the next session too.
 */
@Service(Service.Level.PROJECT)
class PackAssociationsProjectTracker(private val project: Project) : Disposable {

    override fun dispose() {
        TclLspPackAssociations.getInstance().projectClosed(project)
    }

    companion object {
        @JvmStatic
        fun getInstance(project: Project): PackAssociationsProjectTracker =
            project.getService(PackAssociationsProjectTracker::class.java)
    }
}
