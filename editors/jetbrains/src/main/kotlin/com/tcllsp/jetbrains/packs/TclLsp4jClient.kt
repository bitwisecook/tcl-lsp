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

import com.google.gson.Gson
import com.intellij.openapi.project.Project
import com.intellij.platform.lsp.api.Lsp4jClient
import com.intellij.platform.lsp.api.LspServerNotificationsHandler
import org.eclipse.lsp4j.jsonrpc.services.JsonNotification

/**
 * The plugin's LSP client, extended with the one custom notification tcl-lsp
 * sends.
 *
 * lsp4j builds its endpoint from the runtime class of the client object, so
 * an annotated method here is enough to receive a method the platform has
 * never heard of.
 */
@Suppress("UnstableApiUsage")
class TclLsp4jClient(
    handler: LspServerNotificationsHandler,
    private val project: Project,
) : Lsp4jClient(handler) {

    /**
     * `tcl-lsp/specPacksReloaded` — sent once a SpecTcl pack reload has fully
     * landed, carrying the extensions the resulting pack set claims.
     *
     * The client cannot derive this moment for itself: it sees the same
     * `.tclspec` write the server does, but not when the server has finished
     * acting on it, and it never sees an edit under an absolute
     * `tclLsp.specPacks` root at all.
     */
    @JsonNotification("tcl-lsp/specPacksReloaded")
    fun specPacksReloaded(params: Any?) {
        TclLspPackAssociations.getInstance().report(project, Gson().toJsonTree(params))
    }
}
