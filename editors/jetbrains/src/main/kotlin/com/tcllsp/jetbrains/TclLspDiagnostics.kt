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

import com.google.gson.JsonObject
import com.google.gson.JsonPrimitive
import com.intellij.openapi.util.text.StringUtil
import com.intellij.platform.lsp.api.customization.LspDiagnosticsSupport
import com.tcllsp.jetbrains.settings.generated.DiagnosticCatalog
import org.eclipse.lsp4j.Diagnostic

/**
 * How a diagnostic reads in this IDE.
 *
 * The server sends the code in the LSP `code` field, which is where it
 * belongs — VS Code renders it in its own column, so putting it in the message
 * text would show it twice there. IntelliJ's problems view shows the message
 * and nothing else, so a bare "Forward literal load of 'a' from its single
 * reaching definition" arrives with no code, no indication it is an optimiser
 * rewrite rather than a defect, and no hint whether accepting it would change
 * the file. This puts that back, on the client that drops it, without touching
 * what goes over the wire.
 */
class TclLspDiagnostics : LspDiagnosticsSupport() {

    override fun getMessage(diagnostic: Diagnostic): String {
        val code = codeOf(diagnostic) ?: return diagnostic.message
        return "[$code] ${diagnostic.message}"
    }

    override fun getTooltip(diagnostic: Diagnostic): String {
        val code = codeOf(diagnostic)
        val rows = buildList {
            add(StringUtil.escapeXmlEntities(diagnostic.message))
            code?.let { add("<b>${StringUtil.escapeXmlEntities(it)}</b> — ${kindOf(it)}") }
            code?.let { c -> describe(c)?.let { add(StringUtil.escapeXmlEntities(it)) } }
            add(fixSummary(diagnostic))
        }
        return rows.joinToString("<br/>")
    }

    private companion object {
        /**
         * Codes the catalogue lists as optimiser rewrites.
         *
         * Read from the generated catalogue rather than matched on an `O`
         * prefix, so a family renamed in the Rust `DiagCode` source cannot
         * leave this labelling silently wrong.
         */
        val OPTIMISATIONS: Set<String> =
            DiagnosticCatalog.optimisations.mapTo(mutableSetOf()) { it.code }

        /** Code → the documentation section it belongs to. */
        val SECTIONS: Map<String, String> =
            DiagnosticCatalog.diagnostics.associate { it.code to it.section }

        /** Code → its catalogue description, with the leading `CODE: ` removed. */
        val DESCRIPTIONS: Map<String, String> =
            (DiagnosticCatalog.diagnostics.map { it.code to it.label } +
                DiagnosticCatalog.optimisations.map { it.code to it.label })
                .associate { (code, label) -> code to label.removePrefix("$code: ") }

        fun codeOf(diagnostic: Diagnostic): String? {
            val code = diagnostic.code ?: return null
            return when {
                code.isLeft -> code.left
                code.isRight -> code.right?.toString()
                else -> null
            }?.takeIf { it.isNotBlank() }
        }

        fun kindOf(code: String): String = when {
            code in OPTIMISATIONS -> "optimiser rewrite"
            else -> SECTIONS[code]?.let { "$it diagnostic" } ?: "diagnostic"
        }

        fun describe(code: String): String? = DESCRIPTIONS[code]?.takeIf { it.isNotBlank() }

        /**
         * What accepting this diagnostic would do.
         *
         * The server attaches a `replacement` only when the rewrite has a
         * precise sub-span to target; a hint-only optimisation deliberately
         * carries no payload, because its span covers the whole consuming
         * statement and splicing the fragment in would corrupt it. Saying
         * which of the two this is means a suggestion that cannot be applied
         * no longer looks like one that simply has not been tried.
         */
        fun fixSummary(diagnostic: Diagnostic): String {
            val data = diagnostic.data as? JsonObject ?: return "No automatic fix."
            val replacement = (data.get("replacement") as? JsonPrimitive)?.asString
                ?: return "No automatic fix."
            if (replacement.isEmpty()) return "Quick fix: deletes the highlighted code."
            return "Quick fix: replaces the highlighted code with " +
                "<code>${StringUtil.escapeXmlEntities(replacement)}</code>."
        }
    }
}
