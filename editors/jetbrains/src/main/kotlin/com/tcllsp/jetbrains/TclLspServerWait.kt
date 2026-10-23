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
 * Resolve a running server, starting it and waiting briefly when needed.
 * Returns null on timeout, project disposal or interrupted waiting.
 *
 * Start requests allow tool windows to use the lazy server before a Tcl
 * editor has opened. Must run off the EDT: waiting sleeps, and server startup
 * uses `invokeAndWait` on the EDT.
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
