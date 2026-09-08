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

package com.tcllsp.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState

/** How long [awaitRunningTclLspServer] waits for the lazily started Tcl LSP
 *  server to reach Running before giving up. */
internal const val TCL_LSP_SERVER_WAIT_TIMEOUT_MS = 10_000L

/**
 * Resolve a running Tcl LSP server, kicking it and waiting briefly if one
 * isn't ready yet. Returns null only after a real timeout.
 *
 * The server starts lazily on the first Tcl editor (see
 * [TclLspServerSupportProvider]), so a tool window that talks to it before the
 * user has opened a Tcl file finds nothing running. Every such caller has to
 * start it and wait rather than give up, or its request is silently dropped
 * and nothing retries it: the Compiler Explorer restored on IDE startup
 * reported a spurious "LSP server not running", and Spec Studio's dialect pin
 * left the sample under generic Tcl until the selector was touched again.
 *
 * Must be called off the EDT — it sleeps. `invokeAndWait` for the start
 * request is safe for that reason: a pooled thread cannot deadlock against
 * the dispatch it waits on.
 */
@Suppress("UnstableApiUsage")
internal fun awaitRunningTclLspServer(
    project: Project,
    timeoutMs: Long = TCL_LSP_SERVER_WAIT_TIMEOUT_MS,
): LspServer? {
    val manager = LspServerManager.getInstance(project)
    fun running(): LspServer? =
        manager.getServersForProvider(TclLspServerSupportProvider::class.java)
            .firstOrNull { it.state == LspServerState.Running }

    running()?.let { return it }

    // Kick the lazily-started server before the clock starts, so a busy EDT
    // during startup can't eat the timeout budget before the start request
    // even runs.
    ApplicationManager.getApplication().invokeAndWait {
        manager.startServersIfNeeded(TclLspServerSupportProvider::class.java)
    }

    // Monotonic clock: a wall-clock adjustment must not distort the wait.
    val deadlineNanos = System.nanoTime() + timeoutMs * 1_000_000
    while (System.nanoTime() < deadlineNanos) {
        if (project.isDisposed) return null
        try {
            Thread.sleep(150)
        } catch (e: InterruptedException) {
            // Preserve cancellation semantics for the pooled-thread task.
            Thread.currentThread().interrupt()
            return null
        }
        running()?.let { return it }
    }
    return null
}
