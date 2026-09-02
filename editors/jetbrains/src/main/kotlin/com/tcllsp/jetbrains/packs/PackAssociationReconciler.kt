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

// The reversibility rules for pack-declared file-extension registration
// (issue #1650), kept free of every platform type so they can be tested
// without an IDE.
//
// A JetBrains file-type association is IDE-global — there is no
// workspace-scoped layer to write into, the way VS Code's `files.associations`
// gives the extension one — so reconciliation cannot lean on a scope to bound
// the damage it can do. It has to answer "did the plugin install this?"
// exactly, from a ledger of what it wrote, and act only where the answer is
// yes. Everything below is that decision, expressed as a plan the caller
// applies.

package com.tcllsp.jetbrains.packs

import com.google.gson.JsonElement
import com.google.gson.JsonObject

/** One extension, resolved to the name of a file type the plugin contributes. */
data class PackExtensionClaim(val extension: String, val fileTypeName: String)

/**
 * The work one reconciliation pass has to do, plus the ledger that replaces
 * the old one once it is done.
 *
 * `deferred` is the claims left to a manual association — reported so the log
 * says why an extension a pack claims did not become ours.
 */
data class PackAssociationPlan(
    val associate: List<PackExtensionClaim>,
    val disassociate: List<PackExtensionClaim>,
    val owned: Map<String, String>,
    val deferred: List<PackExtensionClaim>,
) {
    val isEmpty: Boolean
        get() = associate.isEmpty() && disassociate.isEmpty()
}

object PackAssociationReconciler {
    /** The plugin's plain Tcl file type, as `FileType.getName()` spells it. */
    const val TCL_FILE_TYPE: String = "Tcl"

    /** The plugin's F5 iRule file type, as `FileType.getName()` spells it. */
    const val IRULE_FILE_TYPE: String = "iRule"

    private const val IRULE_LANGUAGE_ID = "tcl-irule"

    /**
     * The file type an advertised row rides.
     *
     * No editor can mint a language at runtime, so the server resolves every
     * pack-claimed extension onto an id an editor already contributes. Of
     * those the plugin contributes exactly two file types, and only
     * `tcl-irule` names the iRule one; everything else — a dialect with its
     * own id we have no separate file type for, and plain `tcl` for a row
     * with no dialect at all — is Tcl. The dialect is still decided
     * server-side, so the generic type costs nothing but the grammar's first
     * paint.
     */
    fun fileTypeNameFor(languageId: String?): String =
        if (languageId == IRULE_LANGUAGE_ID) IRULE_FILE_TYPE else TCL_FILE_TYPE

    /**
     * The claims carried by a `pack_file_extensions` array, or by an object
     * that has one — the `tcl-lsp/specPacksReloaded` params and the
     * `tcl-lsp.getEffectiveConfig` result both do, in the same shape.
     */
    fun claimsFrom(payload: JsonElement?): Map<String, String> {
        val rows = when {
            payload == null || payload.isJsonNull -> return emptyMap()
            payload.isJsonArray -> payload.asJsonArray
            payload.isJsonObject -> payload.asJsonObject.get("pack_file_extensions")
                ?.takeIf { it.isJsonArray }?.asJsonArray ?: return emptyMap()
            else -> return emptyMap()
        }
        val claims = LinkedHashMap<String, String>()
        for (element in rows) {
            val row = element as? JsonObject ?: continue
            val extension = row.string("extension")?.trim()?.lowercase() ?: continue
            if (extension.isEmpty() || extension.any { it == '.' || it.isWhitespace() }) continue
            claims.putIfAbsent(extension, fileTypeNameFor(row.string("language_id")))
        }
        return claims
    }

    /**
     * Merge the claim sets of every open project.
     *
     * Associations are global while claims are per project, so what gets
     * registered is the union. Two projects claiming one extension for
     * different file types is pathological rather than impossible, and the
     * plain Tcl type wins: both file types carry the same language, so it is
     * the safe superset, and picking by name rather than by whichever project
     * reported first keeps the result independent of open order.
     */
    fun union(claimSets: Collection<Map<String, String>>): Map<String, String> {
        val merged = sortedMapOf<String, String>()
        for (claims in claimSets) {
            for ((extension, fileTypeName) in claims) {
                merged.merge(extension, fileTypeName) { a, b -> minOf(a, b) }
            }
        }
        return merged
    }

    /**
     * Decide what to associate, what to retire, and what the ledger becomes.
     *
     * `associatedWith` reports the file type the IDE currently resolves an
     * extension to, or null when nothing claims it. It is asked inside the
     * same write action that applies the plan, so the answer cannot go stale
     * between the decision and the act.
     *
     * Two rules carry the whole design:
     *
     * 1. **Never remove what we did not add.** An extension is retired only
     *    when the ledger records it *and* the IDE still says exactly what the
     *    ledger records. A user who has since retargeted it no longer matches,
     *    so it is forgotten rather than removed.
     * 2. **A manual association wins.** An extension somebody else already
     *    owns is never claimed and never recorded — including one the user
     *    mapped to the plugin's own Tcl type by hand, which the plugin
     *    therefore also never removes.
     */
    fun plan(
        claimed: Map<String, String>,
        owned: Map<String, String>,
        associatedWith: (String) -> String?,
    ): PackAssociationPlan {
        val associate = mutableListOf<PackExtensionClaim>()
        val disassociate = mutableListOf<PackExtensionClaim>()
        val deferred = mutableListOf<PackExtensionClaim>()
        val ledger = LinkedHashMap<String, String>()

        for ((extension, fileTypeName) in claimed) {
            val current = associatedWith(extension)
            val recorded = owned[extension]
            when {
                current == null -> {
                    associate += PackExtensionClaim(extension, fileTypeName)
                    ledger[extension] = fileTypeName
                }
                // Ours, and still saying what we wrote.
                current == recorded -> {
                    if (current != fileTypeName) {
                        // The claim moved between our two file types — a pack
                        // gained or lost the `-dialect f5-irules` on its row.
                        disassociate += PackExtensionClaim(extension, current)
                        associate += PackExtensionClaim(extension, fileTypeName)
                    }
                    ledger[extension] = fileTypeName
                }
                else -> deferred += PackExtensionClaim(extension, current)
            }
        }

        for ((extension, fileTypeName) in owned) {
            if (claimed.containsKey(extension)) continue
            if (associatedWith(extension) == fileTypeName) {
                disassociate += PackExtensionClaim(extension, fileTypeName)
            }
            // Otherwise the user has changed it since we wrote it: drop it
            // from the ledger without touching the association itself.
        }

        return PackAssociationPlan(associate, disassociate, ledger, deferred)
    }

    private fun JsonObject.string(name: String): String? =
        get(name)?.takeIf { it.isJsonPrimitive }?.asString
}
