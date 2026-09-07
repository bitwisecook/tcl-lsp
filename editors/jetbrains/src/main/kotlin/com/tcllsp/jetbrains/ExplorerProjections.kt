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

import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.editor.event.CaretEvent
import com.intellij.openapi.editor.event.CaretListener
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.ScrollType
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.testFramework.LightVirtualFile

/**
 * A source span as the explorer's `range_dict` emits it.
 *
 * Both coordinate systems ride along: the byte offsets index the source text
 * the pipeline was handed, and the UTF-16 line/column pair is what an IntelliJ
 * caret is actually placed with.
 */
internal data class ExplorerRange(
    val startLine: Int,
    val startColUtf16: Int,
    val endLine: Int,
    val endColUtf16: Int,
)

/**
 * One rendered explorer view: the text a pane shows, and the source span each
 * of its lines points at.
 *
 * `lines` is index-aligned with the text's lines — the server produces the two
 * together in `render_view_mapped`, precisely so a map built by re-parsing the
 * rendered text cannot drift from it.
 */
internal data class ExplorerProjection(
    val text: String,
    val lines: List<ExplorerRange?>,
) {
    companion object {
        /** Read the `tcl-lsp.compilerExplorerView` reply, or null if it failed. */
        fun parse(result: Any?): ExplorerProjection? {
            val json = runCatching {
                JsonParser.parseString(Gson().toJson(result)).asJsonObject
            }.getOrNull() ?: return null
            if (json.has("error") && !json.get("error").isJsonNull) return null
            val text = json.get("text")?.takeIf { it.isJsonPrimitive }?.asString ?: return null
            val lines = json.getAsJsonArray("lines")?.map { element ->
                if (!element.isJsonObject) return@map null
                val o = element.asJsonObject
                runCatching {
                    ExplorerRange(
                        startLine = o.int("startLine"),
                        startColUtf16 = o.int("startColUtf16"),
                        endLine = o.int("endLine"),
                        endColUtf16 = o.int("endColUtf16"),
                    )
                }.getOrNull()
            } ?: emptyList()
            return ExplorerProjection(text, lines)
        }

        private fun JsonObject.int(key: String): Int = get(key).asInt
    }
}

/**
 * Shows explorer views in ordinary editor tabs, and navigates from them.
 *
 * The tool window keeps the panes that are genuinely graphical — the CFG's
 * routed edges, the WASM branch lanes — but everything linear reads better in
 * a real editor, where find, folding, and the colour scheme all work. A pane
 * is a read-only [LightVirtualFile]; moving the caret in one reveals the
 * matching span in the file the view was rendered from.
 */
@Service(Service.Level.PROJECT)
internal class ExplorerProjections(private val project: Project) {

    /** Live panes, keyed by view id, so re-opening replaces rather than stacks. */
    private val open = mutableMapOf<String, Pane>()

    private class Pane(
        val file: LightVirtualFile,
        var projection: ExplorerProjection,
        var origin: VirtualFile?,
    )

    /**
     * Open (or refresh) the pane for [view], and focus it.
     *
     * Called on the EDT.
     */
    fun open(
        label: String,
        view: String,
        projection: ExplorerProjection,
        origin: VirtualFile?,
    ) {
        val pane = open.getOrPut(view) {
            // `.txt` rather than a Tcl extension: a projection is a rendered
            // tree, not Tcl, and typing it as Tcl would attach the language
            // server to a buffer that is not a program.
            val file = LightVirtualFile("$label.txt", projection.text)
            file.isWritable = false
            Pane(file, projection, origin)
        }
        pane.projection = projection
        pane.origin = origin
        if (pane.file.content.toString() != projection.text) {
            pane.file.setContent(this, projection.text, false)
        }

        val editor = FileEditorManager.getInstance(project)
            .openTextEditor(OpenFileDescriptor(project, pane.file), true)
            ?: return
        // One listener per pane. `EditorFactory` hands out a fresh editor each
        // time the tab is re-opened, so attaching here — after the open —
        // covers a pane the user closed and re-opened.
        editor.caretModel.addCaretListener(object : CaretListener {
            override fun caretPositionChanged(event: CaretEvent) {
                navigate(view, event.newPosition.line)
            }
        })
    }

    /** Reveal the source span the pane's [line] points at. */
    private fun navigate(view: String, line: Int) {
        val pane = open[view] ?: return
        val range = pane.projection.lines.getOrNull(line) ?: return
        val origin = pane.origin ?: return
        val document = FileDocumentManager.getInstance().getDocument(origin) ?: return
        val editors = EditorFactory.getInstance().getEditors(document, project)
        // Reveal without stealing focus: arrowing down a projection should walk
        // the source alongside it, not tear the caret away on every keypress.
        val target = editors.firstOrNull()
            ?: FileEditorManager.getInstance(project)
                .openTextEditor(OpenFileDescriptor(project, origin), false)
            ?: return
        val start = target.logicalPositionToOffset(
            com.intellij.openapi.editor.LogicalPosition(range.startLine, range.startColUtf16),
        )
        val end = target.logicalPositionToOffset(
            com.intellij.openapi.editor.LogicalPosition(range.endLine, range.endColUtf16),
        )
        if (start < 0 || end > target.document.textLength || start > end) return
        target.selectionModel.setSelection(start, end)
        target.scrollingModel.scrollTo(target.offsetToLogicalPosition(start), ScrollType.CENTER_UP)
    }

    companion object {
        fun getInstance(project: Project): ExplorerProjections = project.service()
    }
}
