package com.tcllsp.jetbrains.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.util.xmlb.XmlSerializerUtil

@Service
@State(name = "TclLspSettings", storages = [Storage("TclLspSettings.xml")])
class TclLspSettings : PersistentStateComponent<TclLspSettings> {

    // General

    var pythonPath: String = "auto"
    var serverPath: String = ""
    var dialect: String = "tcl8.6"
    var extraCommands: String = ""  // comma-separated
    var libraryPaths: String = ""   // comma-separated

    // Feature toggles

    var featureHover: Boolean = true
    var featureCompletion: Boolean = true
    var featureDiagnostics: Boolean = true
    var featureFormatting: Boolean = true
    var featureSemanticTokens: Boolean = true
    var featureCodeActions: Boolean = true
    var featureDefinition: Boolean = true
    var featureReferences: Boolean = true
    var featureDocumentSymbols: Boolean = true
    var featureFolding: Boolean = true
    var featureRename: Boolean = true
    var featureSignatureHelp: Boolean = true
    var featureWorkspaceSymbols: Boolean = true
    var featureInlayHints: Boolean = true
    var featureCallHierarchy: Boolean = true
    var featureDocumentLinks: Boolean = true
    var featureSelectionRange: Boolean = true

    // Formatting

    var formattingIndentSize: Int = 4
    var formattingIndentStyle: String = "spaces"
    var formattingContinuationIndent: Int = 4
    var formattingBraceStyle: String = "k_and_r"
    var formattingSpaceBetweenBraces: Boolean = true
    var formattingEnforceBracedVariables: Boolean = false
    var formattingEnforceBracedExpr: Boolean = false
    var formattingMaxLineLength: Int = 120
    var formattingGoalLineLength: Int = 100
    var formattingExpandSingleLineBodies: Boolean = false
    var formattingMinBodyCommandsForExpansion: Int = 2
    var formattingSpaceAfterCommentHash: Boolean = true
    var formattingTrimTrailingWhitespace: Boolean = true
    var formattingAlignCommentsToCode: Boolean = true
    var formattingReplaceSemicolonsWithNewlines: Boolean = true
    var formattingBlankLinesBetweenProcs: Int = 1
    var formattingBlankLinesBetweenBlocks: Int = 1
    var formattingMaxConsecutiveBlankLines: Int = 2
    var formattingLineEnding: String = "lf"
    var formattingEnsureFinalNewline: Boolean = true

    // Diagnostics — Errors

    var diagnosticE001: Boolean = true
    var diagnosticE002: Boolean = true
    var diagnosticE003: Boolean = true
    var diagnosticE200: Boolean = true

    // Diagnostics — Warnings

    var diagnosticW001: Boolean = true
    var diagnosticW002: Boolean = true
    var diagnosticW100: Boolean = true
    var diagnosticW104: Boolean = true
    var diagnosticW105: Boolean = true
    var diagnosticW106: Boolean = true
    var diagnosticW108: Boolean = true
    var diagnosticW110: Boolean = true
    var diagnosticW111: Boolean = true
    var diagnosticW112: Boolean = true
    var diagnosticW113: Boolean = true
    var diagnosticW114: Boolean = true
    var diagnosticW115: Boolean = true
    var diagnosticW120: Boolean = true
    var diagnosticW121: Boolean = true
    var diagnosticW122: Boolean = true
    var diagnosticW200: Boolean = true
    var diagnosticW201: Boolean = true
    var diagnosticW210: Boolean = true
    var diagnosticW211: Boolean = true
    var diagnosticW212: Boolean = true
    var diagnosticW213: Boolean = true
    var diagnosticW214: Boolean = true
    var diagnosticW302: Boolean = true
    var diagnosticW304: Boolean = true
    var diagnosticW307: Boolean = true
    var diagnosticW308: Boolean = true
    var diagnosticW309: Boolean = true

    // Style

    var styleLineLength: Int = 120

    // Optimiser

    var optimiserEnabled: Boolean = true
    var optimiserO100: Boolean = true
    var optimiserO101: Boolean = true
    var optimiserO102: Boolean = true
    var optimiserO103: Boolean = true
    var optimiserO104: Boolean = true
    var optimiserO105: Boolean = true
    var optimiserO106: Boolean = true
    var optimiserO107: Boolean = true
    var optimiserO108: Boolean = true
    var optimiserO109: Boolean = true
    var optimiserO110: Boolean = true
    var optimiserO111: Boolean = true
    var optimiserO112: Boolean = true
    var optimiserO113: Boolean = true
    var optimiserO114: Boolean = true
    var optimiserO115: Boolean = true
    var optimiserO116: Boolean = true
    var optimiserO117: Boolean = true
    var optimiserO118: Boolean = true
    var optimiserO119: Boolean = true
    var optimiserO120: Boolean = true
    var optimiserO121: Boolean = true
    var optimiserO122: Boolean = true
    var optimiserO123: Boolean = true
    var optimiserO124: Boolean = true
    var optimiserO125: Boolean = true
    var optimiserO126: Boolean = true

    // Shimmer

    var shimmerEnabled: Boolean = true

    // Runtime Validation

    var runtimeValidationEnabled: Boolean = false

    override fun getState(): TclLspSettings = this

    override fun loadState(state: TclLspSettings) {
        XmlSerializerUtil.copyBean(state, this)
    }

    /**
     * Build the settings map matching the `tclLsp` configuration namespace
     * expected by the language server's `workspace/didChangeConfiguration`.
     */
    fun toServerSettings(): Map<String, Any?> {
        val extraCmds = extraCommands.split(",")
            .map { it.trim() }
            .filter { it.isNotEmpty() }
        val libPaths = libraryPaths.split(",")
            .map { it.trim() }
            .filter { it.isNotEmpty() }

        return mapOf(
            "dialect" to dialect,
            "extraCommands" to extraCmds,
            "libraryPaths" to libPaths,
            "features" to mapOf(
                "hover" to featureHover,
                "completion" to featureCompletion,
                "diagnostics" to featureDiagnostics,
                "formatting" to featureFormatting,
                "semanticTokens" to featureSemanticTokens,
                "codeActions" to featureCodeActions,
                "definition" to featureDefinition,
                "references" to featureReferences,
                "documentSymbols" to featureDocumentSymbols,
                "folding" to featureFolding,
                "rename" to featureRename,
                "signatureHelp" to featureSignatureHelp,
                "workspaceSymbols" to featureWorkspaceSymbols,
                "inlayHints" to featureInlayHints,
                "callHierarchy" to featureCallHierarchy,
                "documentLinks" to featureDocumentLinks,
                "selectionRange" to featureSelectionRange,
            ),
            "formatting" to mapOf(
                "indentSize" to formattingIndentSize,
                "indentStyle" to formattingIndentStyle,
                "continuationIndent" to formattingContinuationIndent,
                "braceStyle" to formattingBraceStyle,
                "spaceBetweenBraces" to formattingSpaceBetweenBraces,
                "enforceBracedVariables" to formattingEnforceBracedVariables,
                "enforceBracedExpr" to formattingEnforceBracedExpr,
                "maxLineLength" to formattingMaxLineLength,
                "goalLineLength" to formattingGoalLineLength,
                "expandSingleLineBodies" to formattingExpandSingleLineBodies,
                "minBodyCommandsForExpansion" to formattingMinBodyCommandsForExpansion,
                "spaceAfterCommentHash" to formattingSpaceAfterCommentHash,
                "trimTrailingWhitespace" to formattingTrimTrailingWhitespace,
                "alignCommentsToCode" to formattingAlignCommentsToCode,
                "replaceSemicolonsWithNewlines" to formattingReplaceSemicolonsWithNewlines,
                "blankLinesBetweenProcs" to formattingBlankLinesBetweenProcs,
                "blankLinesBetweenBlocks" to formattingBlankLinesBetweenBlocks,
                "maxConsecutiveBlankLines" to formattingMaxConsecutiveBlankLines,
                "lineEnding" to formattingLineEnding,
                "ensureFinalNewline" to formattingEnsureFinalNewline,
            ),
            "diagnostics" to mapOf(
                "E001" to diagnosticE001,
                "E002" to diagnosticE002,
                "E003" to diagnosticE003,
                "E200" to diagnosticE200,
                "W001" to diagnosticW001,
                "W002" to diagnosticW002,
                "W100" to diagnosticW100,
                "W104" to diagnosticW104,
                "W105" to diagnosticW105,
                "W106" to diagnosticW106,
                "W108" to diagnosticW108,
                "W110" to diagnosticW110,
                "W111" to diagnosticW111,
                "W112" to diagnosticW112,
                "W113" to diagnosticW113,
                "W114" to diagnosticW114,
                "W115" to diagnosticW115,
                "W120" to diagnosticW120,
                "W121" to diagnosticW121,
                "W122" to diagnosticW122,
                "W200" to diagnosticW200,
                "W201" to diagnosticW201,
                "W210" to diagnosticW210,
                "W211" to diagnosticW211,
                "W212" to diagnosticW212,
                "W213" to diagnosticW213,
                "W214" to diagnosticW214,
                "W302" to diagnosticW302,
                "W304" to diagnosticW304,
                "W307" to diagnosticW307,
                "W308" to diagnosticW308,
                "W309" to diagnosticW309,
            ),
            "style" to mapOf(
                "lineLength" to styleLineLength,
            ),
            "optimiser" to mapOf(
                "enabled" to optimiserEnabled,
                "O100" to optimiserO100,
                "O101" to optimiserO101,
                "O102" to optimiserO102,
                "O103" to optimiserO103,
                "O104" to optimiserO104,
                "O105" to optimiserO105,
                "O106" to optimiserO106,
                "O107" to optimiserO107,
                "O108" to optimiserO108,
                "O109" to optimiserO109,
                "O110" to optimiserO110,
                "O111" to optimiserO111,
                "O112" to optimiserO112,
                "O113" to optimiserO113,
                "O114" to optimiserO114,
                "O115" to optimiserO115,
                "O116" to optimiserO116,
                "O117" to optimiserO117,
                "O118" to optimiserO118,
                "O119" to optimiserO119,
                "O120" to optimiserO120,
                "O121" to optimiserO121,
                "O122" to optimiserO122,
                "O123" to optimiserO123,
                "O124" to optimiserO124,
                "O125" to optimiserO125,
                "O126" to optimiserO126,
            ),
            "shimmer" to mapOf(
                "enabled" to shimmerEnabled,
            ),
            "runtimeValidation" to mapOf(
                "enabled" to runtimeValidationEnabled,
            ),
        )
    }

    companion object {
        @JvmStatic
        fun getInstance(): TclLspSettings =
            ApplicationManager.getApplication().getService(TclLspSettings::class.java)

        val DIALECT_OPTIONS = listOf(
            "tcl8.4" to "Tcl 8.4",
            "tcl8.5" to "Tcl 8.5",
            "tcl8.6" to "Tcl 8.6",
            "tcl9.0" to "Tcl 9.0",
            "f5-irules" to "F5 iRules",
            "f5-iapps" to "F5 iApps",
            "synopsys-eda-tcl" to "Synopsys EDA",
            "cadence-eda-tcl" to "Cadence EDA",
            "xilinx-eda-tcl" to "Xilinx EDA",
            "intel-quartus-eda-tcl" to "Intel Quartus",
            "mentor-eda-tcl" to "Mentor EDA",
            "expect" to "Expect",
        )
    }
}
