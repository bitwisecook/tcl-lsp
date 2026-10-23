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
import com.intellij.platform.lsp.api.LspServerDescriptor
import com.intellij.platform.lsp.api.LspServerManager
import com.tcllsp.jetbrains.TclFileType
import com.tcllsp.jetbrains.TclIruleFileType
import com.tcllsp.jetbrains.TclLspServerDescriptor
import com.tcllsp.jetbrains.TclLspServerSupportProvider
import org.eclipse.lsp4j.ExecuteCommandParams
import org.eclipse.lsp4j.InitializeResult
import java.lang.ref.WeakReference
import java.util.WeakHashMap

private val LOG = Logger.getInstance(TclLspPackAssociations::class.java)

/**
 * Registers the file extensions discovered SpecTcl packs claim, and retires
 * them again when the pack that claimed them goes away.
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
    private class ProjectClaims(owner: Any, val claims: Map<String, String>) {
        val owner = WeakReference(owner)
        val initializedResult = (owner as? LspServer)?.initializeResult
    }

    private data class PendingPull(
        val server: LspServer,
        val initializedResult: InitializeResult,
        val baseline: ProjectClaims?,
    )

    private val activeClients = WeakHashMap<Project, WeakReference<TclLsp4jClient>>()
    private val claimsByProject = WeakHashMap<Project, ProjectClaims>()
    private val pullsInFlight = WeakHashMap<Project, WeakReference<LspServer>>()

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

    /** Retain the actual client endpoint created for this project's current server. */
    fun clientCreated(project: Project, client: TclLsp4jClient) {
        val changed = synchronized(lock) {
            if (project.isDisposed || activeClients[project]?.get() === client) {
                false
            } else {
                activeClients[project] = WeakReference(client)
                claimsByProject.remove(project)
                pullsInFlight.remove(project)
                true
            }
        }
        if (changed) scheduleReconcile()
    }

    /** Record a push only while it comes from the actual currently registered client. */
    fun report(project: Project, client: TclLsp4jClient, payload: JsonElement?) {
        val claims = PackAssociationReconciler.claimsFrom(payload)
        val accepted = synchronized(lock) {
            if (project.isDisposed || activeClients[project]?.get() !== client) {
                false
            } else {
                claimsByProject[project] = ProjectClaims(client, claims)
                true
            }
        }
        if (accepted) scheduleReconcile()
    }

    /**
     * Drop a closed project's claims, and retire whatever no open project
     * claims any more.
     *
     * Called from [PackAssociationsProjectTracker.dispose], the one callback
     * that arrives when a project stops claiming. Pruning on the next report
     * is not enough on its own: with no other project reporting, nothing would
     * ever come.
     */
    fun projectClosed(project: Project) {
        val remaining = synchronized(lock) {
            claimsByProject.remove(project)
            activeClients.remove(project)
            pullsInFlight.remove(project)
            claimsByProject.keys.removeAll { it.isDisposed }
            activeClients.keys.removeAll { it.isDisposed }
            pullsInFlight.keys.removeAll { it.isDisposed }
            claimsByProject.size
        }
        if (PackAssociationReconciler.reconcileAfterProjectClose(remaining)) {
            scheduleReconcile()
        }
    }

    /**
     * Ask a freshly-initialised server what its packs claim.
     *
     * The push covers every reload including the startup one, so this is the
     * catch-up rather than the main path. Runs on a pooled thread: the request
     * is synchronous, and the caller is the platform's own server-initialised
     * callback, which is no place to block.
     */
    @Suppress("UnstableApiUsage")
    fun pull(
        project: Project,
        descriptor: TclLspServerDescriptor,
        initializedResult: InitializeResult,
    ) {
        if (project.isDisposed) return
        val server = currentServer(project, descriptor, initializedResult) ?: return
        val pending = synchronized(lock) {
            if (project.isDisposed) return@synchronized null
            if (pullsInFlight[project]?.get() === server) return@synchronized null
            PendingPull(server, initializedResult, claimsByProject[project]).also {
                pullsInFlight[project] = WeakReference(server)
            }
        } ?: return
        ApplicationManager.getApplication().executeOnPooledThread(
            Runnable {
                try {
                    val result = try {
                        requestEffectiveConfig(server)
                    } catch (error: Exception) {
                        // A server still starting, or already stopped, has nothing
                        // to say about packs yet; the push brings the answer when it does.
                        LOG.debug("Tcl spec-pack extension pull skipped", error)
                        return@Runnable
                    }
                    if (result == null || project.isDisposed ||
                        currentServer(project, descriptor, pending.initializedResult) !== server
                    ) return@Runnable
                    val claims = PackAssociationReconciler.claimsFrom(result)
                    val accepted = synchronized(lock) {
                        if (project.isDisposed || claimsByProject[project] !== pending.baseline ||
                            pullsInFlight[project]?.get() !== server
                        ) {
                            false
                        } else {
                            claimsByProject[project] = ProjectClaims(server, claims)
                            true
                        }
                    }
                    if (accepted) scheduleReconcile()
                } finally {
                    synchronized(lock) {
                        if (pullsInFlight[project]?.get() === server) pullsInFlight.remove(project)
                    }
                }
            },
        )
    }

    @Suppress("UnstableApiUsage")
    private fun currentServer(
        project: Project,
        descriptor: LspServerDescriptor,
        initializedResult: InitializeResult,
    ): LspServer? =
        LspServerManager.getInstance(project)
            .getServersForProvider(TclLspServerSupportProvider::class.java)
            .singleOrNull { it.descriptor === descriptor && it.initializeResult === initializedResult }

    @Suppress("UnstableApiUsage")
    private fun requestEffectiveConfig(server: LspServer): JsonElement? {
        // The timeout is passed explicitly on purpose — see the note in
        // TclLspActionBase.runCommand and the jetbrains-plugin-compat skill.
        val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
            lsp4j.workspaceService.executeCommand(
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
        synchronized(lock) {
            claimsByProject.keys.removeAll { it.isDisposed }
            activeClients.keys.removeAll { it.isDisposed }
            claimsByProject.entries.removeAll { (project, snapshot) ->
                val owner = snapshot.owner.get()
                when (owner) {
                    is TclLsp4jClient -> project.isDisposed || activeClients[project]?.get() !== owner
                    is LspServer -> project.isDisposed || snapshot.initializedResult == null ||
                        currentServer(project, owner.descriptor, snapshot.initializedResult) !== owner
                    else -> true
                }
            }
            val claimed = PackAssociationReconciler.union(claimsByProject.values.map { it.claims })
            val plan = PackAssociationReconciler.plan(claimed, LinkedHashMap(owned)) { extension ->
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
            owned = LinkedHashMap(plan.owned)

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
