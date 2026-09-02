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
import com.google.gson.JsonElement
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileTypes.ExtensionFileNameMatcher
import com.intellij.openapi.fileTypes.FileType
import com.intellij.openapi.fileTypes.FileTypeManager
import com.intellij.openapi.fileTypes.UnknownFileType
import com.intellij.openapi.project.Project
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import com.tcllsp.jetbrains.TclFileType
import com.tcllsp.jetbrains.TclIruleFileType
import com.tcllsp.jetbrains.TclLspServerSupportProvider
import org.eclipse.lsp4j.ExecuteCommandParams
import java.util.WeakHashMap

private val LOG = Logger.getInstance(TclLspPackAssociations::class.java)

/**
 * Registers the file extensions discovered SpecTcl packs claim, and retires
 * them again when the pack that claimed them goes away (issue #1650).
 *
 * The advertised set arrives two ways, both carrying the same
 * `pack_file_extensions` array: pushed on `tcl-lsp/specPacksReloaded` once a
 * reload has fully landed, and pulled once with `tcl-lsp.getEffectiveConfig`
 * when a server finishes initialising, for the case where the push had
 * already gone by.
 *
 * Claims are tracked per project because a JetBrains file-type association is
 * not: `FileTypeManager` is application-wide, so what the plugin registers is
 * the union over open projects, and it retires an association only when no
 * open project still claims it. [PackAssociationReconciler] holds the rules;
 * this class holds the ledger, the threading, and the platform calls.
 */
@Service
@State(name = "TclLspPackAssociations", storages = [Storage("TclLspPackAssociations.xml")])
class TclLspPackAssociations : PersistentStateComponent<TclLspPackAssociations.State> {

    /**
     * The associations the plugin itself installed: extension to the file
     * type it was pointed at.
     *
     * Persisted because the question on cleanup — "did we write this, and is
     * it still what we wrote?" — has to survive the restart that separates
     * installing an association from discovering its pack is gone.
     */
    class State {
        var owned: MutableMap<String, String> = LinkedHashMap()
    }

    private val lock = Any()
    private var owned: MutableMap<String, String> = LinkedHashMap()
    private val claimsByProject = WeakHashMap<Project, Map<String, String>>()

    override fun getState(): State = State().also {
        it.owned = synchronized(lock) { LinkedHashMap(owned) }
    }

    override fun loadState(state: State) {
        synchronized(lock) { owned = LinkedHashMap(state.owned) }
        // The IDE keeps its file-type associations across a restart, so what
        // the ledger records is live again before any server has reported.
        // Seeding from it is what lets a session that only ever opens a
        // pack-claimed file start a server at all.
        PackClaimedExtensions.replaceWith(state.owned.keys.toSet())
    }

    /** Record what one project's server says its packs claim, then reconcile. */
    fun report(project: Project, payload: JsonElement?) {
        val claims = PackAssociationReconciler.claimsFrom(payload)
        synchronized(lock) {
            claimsByProject.keys.removeAll { it.isDisposed }
            claimsByProject[project] = claims
        }
        scheduleReconcile()
    }

    /**
     * Ask a freshly-initialised server what its packs claim.
     *
     * The push covers every reload including the startup one, so this is the
     * catch-up rather than the main path. Runs on a pooled thread: the request
     * is synchronous, and the caller is the platform's own server-initialised
     * callback, which is no place to block.
     */
    fun pull(project: Project) {
        ApplicationManager.getApplication().executeOnPooledThread(
            Runnable {
                val result = try {
                    requestEffectiveConfig(project)
                } catch (error: Exception) {
                    // A server still starting, or already stopped, has nothing
                    // to say about packs yet; the push brings the answer when
                    // it does.
                    LOG.debug("Tcl spec-pack extension pull skipped", error)
                    return@Runnable
                }
                if (result != null) {
                    report(project, result)
                }
            },
        )
    }

    private fun requestEffectiveConfig(project: Project): JsonElement? {
        @Suppress("UnstableApiUsage")
        val server = LspServerManager.getInstance(project)
            .getServersForProvider(TclLspServerSupportProvider::class.java)
            .firstOrNull() ?: return null
        // The timeout is passed explicitly on purpose — see the note in
        // TclLspActionBase.runCommand and the jetbrains-plugin-compat skill.
        val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
            lsp4j.workspaceService.executeCommand(
                // The pack set is process-global on the server — one reload
                // covers the bundled tier and every workspace folder — so this
                // asks with no document rather than implying a per-file answer.
                ExecuteCommandParams("tcl-lsp.getEffectiveConfig", listOf("")),
            )
        } ?: return null
        return Gson().toJsonTree(result)
    }

    private fun scheduleReconcile() {
        val application = ApplicationManager.getApplication()
        application.invokeLater(
            Runnable {
                // FileTypeManager mutation is a model change: event dispatch
                // thread, inside a write action. The platform fires its own
                // file-types-changed event from there, which is what makes an
                // editor already showing the file re-detect its type.
                application.runWriteAction(Runnable { reconcile() })
            },
        )
    }

    private fun reconcile() {
        val fileTypeManager = FileTypeManager.getInstance()
        val claimed: Map<String, String>
        val ledger: Map<String, String>
        synchronized(lock) {
            claimsByProject.keys.removeAll { it.isDisposed }
            claimed = PackAssociationReconciler.union(claimsByProject.values.toList())
            ledger = LinkedHashMap(owned)
        }

        val plan = PackAssociationReconciler.plan(claimed, ledger) { extension ->
            associatedFileTypeName(fileTypeManager, extension)
        }
        for (claim in plan.disassociate) {
            fileTypeFor(claim.fileTypeName)?.let {
                fileTypeManager.removeAssociation(it, ExtensionFileNameMatcher(claim.extension))
            }
        }
        for (claim in plan.associate) {
            fileTypeFor(claim.fileTypeName)?.let {
                fileTypeManager.associate(it, ExtensionFileNameMatcher(claim.extension))
            }
        }
        synchronized(lock) { owned = LinkedHashMap(plan.owned) }

        // Everything a pack claims that now resolves to one of our file
        // types — including an extension the user associated by hand, which
        // the plan deliberately leaves alone but the server still routes.
        PackClaimedExtensions.replaceWith(
            claimed.keys.filterTo(HashSet()) { isOurs(fileTypeManager.getFileTypeByExtension(it)) },
        )

        if (!plan.isEmpty) {
            LOG.info(
                "Tcl pack file associations: +" +
                    plan.associate.joinToString(",") { it.extension }.ifEmpty { "-" } +
                    " -" + plan.disassociate.joinToString(",") { it.extension }.ifEmpty { "-" },
            )
        }
        for (claim in plan.deferred) {
            LOG.info(
                "Tcl pack file association for .${claim.extension} skipped: " +
                    "already associated with \"${claim.fileTypeName}\"",
            )
        }
    }

    private fun associatedFileTypeName(manager: FileTypeManager, extension: String): String? {
        val fileType = manager.getFileTypeByExtension(extension)
        return if (fileType == UnknownFileType.INSTANCE) null else fileType.name
    }

    private fun fileTypeFor(name: String): FileType? = when (name) {
        PackAssociationReconciler.TCL_FILE_TYPE -> TclFileType.INSTANCE
        PackAssociationReconciler.IRULE_FILE_TYPE -> TclIruleFileType.INSTANCE
        else -> null
    }

    private fun isOurs(fileType: FileType?): Boolean =
        fileType === TclFileType.INSTANCE || fileType === TclIruleFileType.INSTANCE

    companion object {
        @JvmStatic
        fun getInstance(): TclLspPackAssociations =
            ApplicationManager.getApplication().getService(TclLspPackAssociations::class.java)
    }
}
