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

package com.tcllsp.jetbrains.actions

import com.google.gson.Gson
import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project

// ---------------------------------------------------------------------------
// Document-modifying commands. The LSP server returns a WorkspaceEdit that
// lsp4j applies in-place, so the action just dispatches the command.
// ---------------------------------------------------------------------------

class OptimiseDocumentAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.optimiseDocument"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri, "full")
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Optimisations applied", NotificationType.INFORMATION)
    }
}

class MinifyDocumentAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.minifyDocument"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Document minified", NotificationType.INFORMATION)
    }
}

class FixAllSafeIssuesAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.fixAllSafeIssues"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Safe quick fixes applied", NotificationType.INFORMATION)
    }
}

// ---------------------------------------------------------------------------
// Source-as-input commands that return generated content. The result is
// opened in a scratch editor so the user can save / iterate on it.
// ---------------------------------------------------------------------------

class TkPreviewAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.tkPreview"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source)
    }
    override fun resultExtension(result: Any?): String = "html"
}

class TranslateXcAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.xcTranslate"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source, "both")
    }
    override fun resultExtension(result: Any?): String = "json"
}

class DiagramDataAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.diagramData"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val source = documentText ?: return null
        return listOf(source)
    }
    override fun resultExtension(result: Any?): String = "mmd"

    override fun presentResult(project: Project, result: Any?) {
        val data = result?.let { Gson().toJsonTree(it) }
        val mermaid = data?.let(::renderDiagramMermaid)
        if (mermaid == null) {
            notify(project, "Diagram command returned an invalid data payload", NotificationType.ERROR)
            return
        }
        // The LSP command deliberately returns tcl-diagram's structured
        // `{events, procedures}` contract. Render it here instead of placing
        // JSON in a `.mmd` scratch tab, which is neither readable nor a
        // Mermaid diagram.
        super.presentResult(project, mermaid)
    }
}

/** Render the stable `tcl-diagram` `{events, procedures}` JSON contract. */
internal fun renderDiagramMermaid(data: JsonElement): String? {
    val root = data.takeIf { it.isJsonObject }?.asJsonObject ?: return null
    val events = root.array("events") ?: return null
    val procedures = root.array("procedures") ?: return null
    val out = mutableListOf("flowchart TD")
    val ids = MermaidIds()

    fun label(value: String): String = value
        .replace('"', '\'')
        .replace('[', '(')
        .replace(']', ')')
        // Mermaid uses a literal `|` to terminate an edge caption.  Numeric
        // HTML entities are decoded in the rendered label but cannot close
        // that delimiter while Mermaid parses the flowchart source.
        .replace("|", "&#124;")
        .replace('\n', ' ')

    fun node(text: String, shape: String = "box"): String {
        val id = ids.next()
        val escaped = label(text)
        out += when (shape) {
            "decision" -> "$id{$escaped}"
            "round" -> "$id([$escaped])"
            else -> "$id[\"$escaped\"]"
        }
        return id
    }

    fun connect(from: String?, to: String, caption: String? = null) {
        if (from != null) out += if (caption == null) "$from --> $to" else "$from -->|${label(caption)}| $to"
    }

    data class Tail(val id: String, val completion: DiagramCompletion = DiagramCompletion.NORMAL, val completionCode: Int? = null)

    fun completion(item: JsonObject): DiagramCompletion = when (item.string("completion")) {
        "error" -> DiagramCompletion.ERROR
        "return" -> DiagramCompletion.RETURN
        "break" -> DiagramCompletion.BREAK
        "continue" -> DiagramCompletion.CONTINUE
        "dynamic" -> DiagramCompletion.DYNAMIC
        "dynamic_return_or_error" -> DiagramCompletion.DYNAMIC_RETURN_OR_ERROR
        "custom" -> DiagramCompletion.EXACT_CUSTOM
        "process_exit" -> DiagramCompletion.PROCESS_EXIT
        "terminal" -> DiagramCompletion.TERMINAL
        else -> DiagramCompletion.NORMAL
    }

    fun connect(incoming: List<Tail>, to: String, caption: String? = null) {
        incoming.forEach { connect(it.id, to, caption) }
    }

    fun walk(flow: JsonArray, incoming: List<Tail>): List<Tail> {
        var active = incoming
        val abrupt = mutableListOf<Tail>()
        for (element in flow) {
            if (active.isEmpty()) break
            val item = element.takeIf { it.isJsonObject }?.asJsonObject ?: continue
            when (item.string("kind")) {
                "if" -> {
                    val decision = node("if", "decision")
                    connect(active, decision)
                    val branches = item.array("branches") ?: JsonArray()
                    val branchTails = mutableListOf<Tail>()
                    for (branchElement in branches) {
                        val branch = branchElement.takeIf { it.isJsonObject }?.asJsonObject ?: continue
                        val branchStart = node(branch.string("condition") ?: "branch")
                        connect(decision, branchStart, branch.string("condition"))
                        branchTails += walk(branch.array("body") ?: JsonArray(), listOf(Tail(branchStart)))
                    }
                    if (branchTails.isEmpty()) {
                        active = listOf(Tail(decision))
                    } else {
                        val join = node("if join", "round")
                        // A conditional without an explicit else also has a
                        // fall-through path; keep it connected to the same
                        // continuation rather than dropping it.
                        val hasElse = branches.any {
                            it.isJsonObject && it.asJsonObject.string("condition") == "else"
                        }
                        val normalTails = branchTails.filter { it.completion == DiagramCompletion.NORMAL }
                        normalTails.forEach { connect(it.id, join) }
                        if (!hasElse) {
                            connect(decision, join, "false")
                        }
                        abrupt += branchTails.filter { it.completion != DiagramCompletion.NORMAL }
                        active = listOf(Tail(join))
                    }
                }
                "switch" -> {
                    val decision = node("switch ${item.string("subject") ?: ""}", "decision")
                    connect(active, decision)
                    val armTails = mutableListOf<Tail>()
                    var hasDefault = false
                    for (armElement in item.array("arms") ?: JsonArray()) {
                        val arm = armElement.takeIf { it.isJsonObject }?.asJsonObject ?: continue
                        val pattern = arm.string("pattern") ?: "case"
                        val armStart = node(pattern)
                        connect(decision, armStart, pattern)
                        armTails += walk(arm.array("body") ?: JsonArray(), listOf(Tail(armStart)))
                        // `tcl-diagram` serialises the compiler's default
                        // arm as this exact lower-case spelling.  `Default`
                        // is an ordinary Tcl pattern and must retain the
                        // no-match continuation.
                        hasDefault = hasDefault || pattern == "default"
                    }
                    if (armTails.isEmpty()) {
                        active = listOf(Tail(decision))
                    } else {
                        // Every matching arm continues at one shared point.
                        // Without a default, Tcl also reaches that point when
                        // no pattern matched; a default consumes that path.
                        val join = node("switch join", "round")
                        armTails.filter { it.completion == DiagramCompletion.NORMAL }.forEach { connect(it.id, join) }
                        if (!hasDefault) {
                            connect(decision, join, "no match")
                        }
                        abrupt += armTails.filter { it.completion != DiagramCompletion.NORMAL }
                        active = listOf(Tail(join))
                    }
                }
                "loop" -> {
                    val loop = node(item.string("label") ?: item.string("kind") ?: "block", "round")
                    connect(active, loop)
                    val bodyTails = walk(item.array("body") ?: JsonArray(), listOf(Tail(loop)))
                    val exit = node("loop exit", "round")
                    bodyTails.filter { it.completion == DiagramCompletion.NORMAL }.forEach { connect(it.id, loop, "repeat") }
                    bodyTails.filter { it.completion == DiagramCompletion.CONTINUE }.forEach { connect(it.id, loop, "continue") }
                    bodyTails.filter { it.completion == DiagramCompletion.BREAK }.forEach { connect(it.id, exit, "break") }
                    abrupt += bodyTails.filter {
                        it.completion != DiagramCompletion.NORMAL &&
                            it.completion != DiagramCompletion.CONTINUE &&
                            it.completion != DiagramCompletion.BREAK
                    }
                    connect(loop, exit, item.string("exit") ?: "exit")
                    active = listOf(Tail(exit))
                }
                "catch" -> {
                    val block = node("catch", "round")
                    connect(active, block)
                    val bodyExits = walk(item.array("body") ?: JsonArray(), listOf(Tail(block)))
                    // `catch` converts every Tcl completion into its own
                    // normal integer result. Process exit is not a Tcl
                    // completion and cannot be caught, so it remains abrupt.
                    val processExits = bodyExits.filter { it.completion == DiagramCompletion.PROCESS_EXIT }
                    abrupt += processExits
                    active = bodyExits
                        .filter { it.completion != DiagramCompletion.PROCESS_EXIT }
                        .map { Tail(it.id) }
                }
                "try" -> {
                    val attempt = node("try", "round")
                    connect(active, attempt)
                    val bodyExits = walk(item.array("body") ?: JsonArray(), listOf(Tail(attempt)))
                    data class Handler(val data: JsonObject, val start: String, var exits: List<Tail>? = null)
                    val handlers = mutableListOf<Handler>()
                    for (handlerElement in item.array("handlers") ?: JsonArray()) {
                        val handler = handlerElement.takeIf { it.isJsonObject }?.asJsonObject ?: continue
                        val kind = handler.string("kind_handler") ?: "handler"
                        val match = handler.string("match")
                        val handlerLabel = listOfNotNull(kind, match)
                            .filter(String::isNotBlank)
                            .joinToString(" ")
                        val handlerStart = node(handlerLabel.ifEmpty { "handler" }, "round")
                        handlers += Handler(handler, handlerStart)
                    }

                    fun renderHandler(index: Int): List<Tail> {
                        val handler = handlers[index]
                        handler.exits?.let { return it }
                        val exits = if (handler.data.get("fallthrough")?.asBoolean == true && index + 1 < handlers.size) {
                            val next = handlers[index + 1]
                            // Tcl's `-` shares the next handler's body.  It is
                            // not a completion to `finally` of its own.
                            connect(handler.start, next.start, "fall through")
                            renderHandler(index + 1)
                        } else {
                            walk(handler.data.array("body") ?: JsonArray(), listOf(Tail(handler.start)))
                        }
                        handler.exits = exits
                        return exits
                    }

                    fun handlerMatches(handler: Handler, outcome: DiagramCompletion): Boolean {
                        if (handler.data.string("kind_handler") == "trap") {
                            return outcome == DiagramCompletion.ERROR ||
                                outcome == DiagramCompletion.DYNAMIC ||
                                outcome == DiagramCompletion.DYNAMIC_RETURN_OR_ERROR
                        }
                        if (handler.data.string("kind_handler") != "on") return false
                        val code = handler.data.get("completion_code")?.takeIf { it.isJsonPrimitive }?.asInt
                        return when (outcome) {
                            DiagramCompletion.NORMAL -> code == 0
                            DiagramCompletion.ERROR -> code == 1
                            DiagramCompletion.RETURN -> code == 2
                            DiagramCompletion.BREAK -> code == 3
                            DiagramCompletion.CONTINUE -> code == 4
                            DiagramCompletion.DYNAMIC -> false
                            DiagramCompletion.DYNAMIC_RETURN_OR_ERROR -> code == 1 || code == 2
                            DiagramCompletion.EXACT_CUSTOM -> false
                            DiagramCompletion.PROCESS_EXIT -> false
                            DiagramCompletion.TERMINAL -> false
                        }
                    }

                    fun completionOf(possible: PossibleCompletion): DiagramCompletion = when (possible) {
                        PossibleCompletion.NORMAL -> DiagramCompletion.NORMAL
                        PossibleCompletion.ERROR -> DiagramCompletion.ERROR
                        PossibleCompletion.RETURN -> DiagramCompletion.RETURN
                        PossibleCompletion.BREAK -> DiagramCompletion.BREAK
                        PossibleCompletion.CONTINUE -> DiagramCompletion.CONTINUE
                        is PossibleCompletion.CUSTOM, PossibleCompletion.UNKNOWN_OTHER -> DiagramCompletion.TERMINAL
                    }
                    fun onMatches(handler: Handler, possible: PossibleCompletion): Boolean {
                        val code = handler.data.get("completion_code")?.takeIf { it.isJsonPrimitive }?.asInt
                        return when (possible) {
                            PossibleCompletion.NORMAL -> code == 0
                            PossibleCompletion.ERROR -> code == 1
                            PossibleCompletion.RETURN -> code == 2
                            PossibleCompletion.BREAK -> code == 3
                            PossibleCompletion.CONTINUE -> code == 4
                            is PossibleCompletion.CUSTOM -> code == possible.code
                            PossibleCompletion.UNKNOWN_OTHER -> false
                        }
                    }
                    fun trapPattern(handler: Handler): List<String>? = handler.data.array("trap_pattern")
                        ?.mapNotNull { it.takeIf { value -> value.isJsonPrimitive }?.asString }
                    fun trapSubsumes(previous: List<String>?, candidate: List<String>?): Boolean {
                        if (previous == null || candidate == null) return false
                        if (previous == candidate || previous.isEmpty()) return true
                        // `trap` patterns are error-code list prefixes. A
                        // concrete prefix claims every longer error code with
                        // that prefix; otherwise retain the candidate because
                        // it may still match an unclaimed error-code class.
                        return previous.size <= candidate.size && previous.zip(candidate).all { (a, b) -> a == b }
                    }

                    val exits = mutableListOf<Tail>()
                    for (bodyExit in bodyExits) {
                        if (bodyExit.completion == DiagramCompletion.DYNAMIC ||
                            bodyExit.completion == DiagramCompletion.DYNAMIC_RETURN_OR_ERROR
                        ) {
                            val possible = if (bodyExit.completion == DiagramCompletion.DYNAMIC) {
                                val standard = listOf(
                                    PossibleCompletion.NORMAL,
                                    PossibleCompletion.ERROR,
                                    PossibleCompletion.RETURN,
                                    PossibleCompletion.BREAK,
                                    PossibleCompletion.CONTINUE,
                                )
                                val custom = handlers.asSequence()
                                    .filter { it.data.string("kind_handler") == "on" }
                                    .mapNotNull { it.data.get("completion_code")?.takeIf { value -> value.isJsonPrimitive }?.asInt }
                                    .filterNot { it in 0..4 }
                                    .distinct()
                                    .map { PossibleCompletion.CUSTOM(it) }
                                    .toList()
                                standard + custom + PossibleCompletion.UNKNOWN_OTHER
                            } else {
                                listOf(PossibleCompletion.ERROR, PossibleCompletion.RETURN)
                            }
                            for (outcome in possible) {
                                var certainlyCaught = false
                                val claimedTrapPatterns = mutableListOf<List<String>?>()
                                for ((index, handler) in handlers.withIndex()) {
                                    when (handler.data.string("kind_handler")) {
                                        "trap" -> if (outcome == PossibleCompletion.ERROR &&
                                            claimedTrapPatterns.none { trapSubsumes(it, trapPattern(handler)) }
                                        ) {
                                            connect(bodyExit.id, handler.start, "trap")
                                            exits += renderHandler(index)
                                            val pattern = trapPattern(handler)
                                            claimedTrapPatterns += pattern
                                            // The empty Tcl list is a prefix
                                            // of every error code. No later
                                            // trap or on-error handler can be
                                            // selected for this outcome.
                                            if (pattern?.isEmpty() == true) {
                                                certainlyCaught = true
                                                break
                                            }
                                        }
                                        "on" -> if (onMatches(handler, outcome)) {
                                            connect(bodyExit.id, handler.start, "on")
                                            exits += renderHandler(index)
                                            certainlyCaught = true
                                            break
                                        }
                                    }
                                }
                                if (!certainlyCaught) exits += Tail(bodyExit.id, completionOf(outcome))
                            }
                            continue
                        }
                        val matches = handlers.withIndex().filter { (_, handler) -> handlerMatches(handler, bodyExit.completion) }
                        val exactCustomMatches = if (bodyExit.completion == DiagramCompletion.EXACT_CUSTOM) {
                            handlers.withIndex().filter { (_, handler) ->
                                handler.data.string("kind_handler") == "on" &&
                                    handler.data.get("completion_code")?.takeIf { it.isJsonPrimitive }?.asInt == bodyExit.completionCode
                            }
                        } else emptyList()
                        // Exact errors still carry no concrete -errorcode in
                        // the compact diagram contract. Every non-shadowed
                        // trap is therefore possible, but Tcl tries them in
                        // source order and a prefix pattern claims all of a
                        // later, more-specific one. Keep the projected list
                        // elements as the comparison owner; raw match text
                        // would get quoting and escapes wrong.
                        val effectiveMatches = when {
                            exactCustomMatches.isNotEmpty() -> exactCustomMatches
                            bodyExit.completion == DiagramCompletion.ERROR -> {
                                val claimedTrapPatterns = mutableListOf<List<String>?>()
                                matches.filter { (_, handler) ->
                                    if (handler.data.string("kind_handler") != "trap") return@filter true
                                    val pattern = trapPattern(handler)
                                    if (claimedTrapPatterns.any { trapSubsumes(it, pattern) }) return@filter false
                                    claimedTrapPatterns += pattern
                                    true
                                }
                            }
                            else -> matches
                        }
                        val errorIsCertainlyHandled = bodyExit.completion == DiagramCompletion.ERROR && matches.any {
                            (_, handler) -> handler.data.string("kind_handler") == "on" ||
                                (handler.data.string("kind_handler") == "trap" &&
                                    trapPattern(handler)?.isEmpty() == true)
                        }
                        if (effectiveMatches.isEmpty() ||
                            (bodyExit.completion == DiagramCompletion.ERROR && !errorIsCertainlyHandled)
                        ) {
                            // A `trap` pattern needs error-code detail which
                            // this compact contract intentionally does not
                            // carry.  It is a possible handled path, but not
                            // proof that every error is caught, so retain the
                            // unhandled error as well.
                            exits += bodyExit
                        }
                        if (effectiveMatches.isNotEmpty()) {
                            // `on` clauses are selected in source order.  A
                            // `trap` pattern is not represented by the
                            // structured IR's completion code, so every
                            // preceding trap is a possible path; a matching
                            // `on` clause then catches all remaining paths.
                            for ((index, handler) in effectiveMatches) {
                                connect(bodyExit.id, handler.start, handler.data.string("kind_handler"))
                                exits += renderHandler(index)
                                val exhaustiveEmptyTrap = bodyExit.completion == DiagramCompletion.ERROR &&
                                    handler.data.string("kind_handler") == "trap" &&
                                    trapPattern(handler)?.isEmpty() == true
                                if (exhaustiveEmptyTrap ||
                                    (handler.data.string("kind_handler") == "on" &&
                                        bodyExit.completion != DiagramCompletion.DYNAMIC &&
                                        bodyExit.completion != DiagramCompletion.DYNAMIC_RETURN_OR_ERROR)
                                ) break
                            }
                        }
                    }
                    val finallyBody = item.array("finally")
                    if (finallyBody == null) {
                        val normalExits = exits.filter { it.completion == DiagramCompletion.NORMAL }
                        abrupt += exits.filter { it.completion != DiagramCompletion.NORMAL }
                        active = if (normalExits.isEmpty()) {
                            emptyList()
                        } else {
                            val join = node("try join", "round")
                            normalExits.forEach { connect(it.id, join) }
                            listOf(Tail(join))
                        }
                    } else {
                        val processExits = exits.filter { it.completion == DiagramCompletion.PROCESS_EXIT }
                        val finallyInputs = exits.filter { it.completion != DiagramCompletion.PROCESS_EXIT }
                        if (finallyInputs.isEmpty()) {
                            abrupt += processExits
                            active = emptyList()
                            continue
                        }
                        val cleanup = node("finally", "round")
                        finallyInputs.forEach { connect(it.id, cleanup) }
                        val finallyExits = walk(finallyBody, listOf(Tail(cleanup)))
                        // A normal finally body preserves the completion it
                        // received.  Thus only normal and handled-completing
                        // paths reach the statement after the try; return,
                        // break, and unhandled-error paths still execute
                        // cleanup but propagate afterwards.  A completion in
                        // the finally body itself overrides every incoming
                        // path, exactly as Tcl specifies.
                        val propagated = finallyExits.flatMap { finalExit ->
                            if (finalExit.completion == DiagramCompletion.NORMAL) {
                                finallyInputs.map { Tail(finalExit.id, it.completion, it.completionCode) }
                            } else {
                                listOf(finalExit)
                            }
                        } + processExits
                        abrupt += propagated.filter { it.completion != DiagramCompletion.NORMAL }
                        active = propagated.filter { it.completion == DiagramCompletion.NORMAL }
                    }
                }
                else -> {
                    val text = item.string("label")
                        ?: item.string("value")
                        ?: item.string("kind")
                        ?: "step"
                    val step = node(text)
                    connect(active, step)
                    val outcome = completion(item)
                    val completionCode = item.get("completion_code")?.takeIf { it.isJsonPrimitive }?.asInt
                    if (outcome == DiagramCompletion.NORMAL) {
                        active = listOf(Tail(step))
                    } else {
                        abrupt += Tail(step, outcome, completionCode)
                        active = emptyList()
                    }
                }
            }
        }
        return active + abrupt
    }

    for (eventElement in events) {
        val event = eventElement.takeIf { it.isJsonObject }?.asJsonObject ?: continue
        val name = event.string("name") ?: "event"
        val priority = event.get("priority")?.takeUnless { it.isJsonNull }?.asString
        val multiplicity = event.string("multiplicity")
        val detail = listOfNotNull(priority?.let { "priority $it" }, multiplicity).joinToString(", ")
        val start = node(if (detail.isEmpty()) "when $name" else "when $name ($detail)", "round")
        walk(event.array("flow") ?: JsonArray(), listOf(Tail(start)))
    }
    for (procedureElement in procedures) {
        val procedure = procedureElement.takeIf { it.isJsonObject }?.asJsonObject ?: continue
        val name = procedure.string("name") ?: "procedure"
        val params = procedure.array("params")?.joinToString(", ") { it.asString } ?: ""
        val start = node("proc $name($params)", "round")
        walk(procedure.array("flow") ?: JsonArray(), listOf(Tail(start)))
    }
    return out.joinToString("\n")
}

private class MermaidIds {
    private var value = 0
    fun next(): String = "n${value++}"
}

/** Completion states defined by the shared `tcl-diagram` JSON contract. */
private enum class DiagramCompletion { NORMAL, ERROR, RETURN, BREAK, CONTINUE, DYNAMIC, DYNAMIC_RETURN_OR_ERROR, EXACT_CUSTOM, PROCESS_EXIT, TERMINAL }
private sealed class PossibleCompletion {
    data object NORMAL : PossibleCompletion()
    data object ERROR : PossibleCompletion()
    data object RETURN : PossibleCompletion()
    data object BREAK : PossibleCompletion()
    data object CONTINUE : PossibleCompletion()
    data class CUSTOM(val code: Int) : PossibleCompletion()
    data object UNKNOWN_OTHER : PossibleCompletion()
}

private fun JsonObject.string(name: String): String? =
    get(name)?.takeUnless { it.isJsonNull }?.asString

private fun JsonObject.array(name: String): JsonArray? =
    get(name)?.takeIf { it.isJsonArray }?.asJsonArray

// ---------------------------------------------------------------------------
// URI-as-input commands. extractLinkedObjects + bigipCleanup both want the
// current document's URI plus a default options dict; the server fills in
// the rest from the parsed config.
// ---------------------------------------------------------------------------

class BigipCleanupAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.bigipCleanup"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun resultExtension(result: Any?): String = "tmsh"
}

class ExtractLinkedObjectsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.extractLinkedObjects"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        return listOf(uri)
    }
    override fun resultExtension(result: Any?): String = "json"
}

// ---------------------------------------------------------------------------
// Information / catalogue queries. No document context required.
// ---------------------------------------------------------------------------

class ListIruleEventsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listIruleEvents"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> = emptyList()
    override fun resultExtension(result: Any?): String = "json"
}

class ListKnownPackagesAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listKnownPackages"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> = emptyList()
    override fun resultExtension(result: Any?): String = "json"
}

class GetEffectiveConfigAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.getEffectiveConfig"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return if (documentUri != null) listOf(documentUri) else listOf("")
    }
    override fun resultExtension(result: Any?): String = "json"
}

// ---------------------------------------------------------------------------
// Prompt-driven catalogue lookups.
// ---------------------------------------------------------------------------

class DescribeIruleEventAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.describeIruleEvent"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "iRule event name (e.g. HTTP_REQUEST):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class DescribeIruleCommandAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.describeIruleCommand"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "iRule command name (e.g. HTTP::redirect):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class SearchHelpAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.searchHelp"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Search Tcl LSP help for:"), false)
    }
    override fun resultExtension(result: Any?): String = "json"
}

class SuggestPackagesForSymbolAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.suggestPackagesForSymbol"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Symbol (e.g. ::json::parse):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class ListSubcommandsAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.listSubcommands"
    override val needsEditor = false
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any> {
        return listOf(prompt(project, "Ensemble command (e.g. string, dict, array):"))
    }
    override fun resultExtension(result: Any?): String = "json"
}

class RenamePartitionAction : TclLspActionBase() {
    override val commandId = "tcl-lsp.renamePartition"
    override fun buildArguments(project: Project, documentUri: String?, documentText: String?, event: AnActionEvent): List<Any>? {
        val uri = documentUri ?: return null
        val oldName = prompt(project, "Current partition name (e.g. Common):", "Common")
        val newName = prompt(project, "New partition name:")
        return listOf(uri, oldName, newName)
    }
    override fun presentResult(project: Project, result: Any?) {
        notify(project, "Partition renamed", NotificationType.INFORMATION)
    }
}
