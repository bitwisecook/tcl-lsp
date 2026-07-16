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

    // Kept for XML deserialization of old settings; the server is now a
    // bundled native binary launched directly, so no Python interpreter is
    // discovered or used.
    var pythonPath: String = "auto"
    var serverPath: String = ""
    var dialect: String = "tcl8.6"
    var extraCommands: String = ""  // comma-separated
    var libraryPaths: String = ""   // comma-separated

    // Feature toggles

    var featureHover: Boolean = true
    var featureCompletion: Boolean = true
    var featureDiagnostics: Boolean = true
    // Kept for XML deserialization of old settings; no longer sent to server.
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
    var featureInlayTypeHints: Boolean = false
    var featureInlayParameterHints: Boolean = false
    var featureCallHierarchy: Boolean = true
    var featureDocumentLinks: Boolean = true
    var featureSelectionRange: Boolean = true
    // New in 1.6.x — see editors/vscode/package.json for matching defaults.
    var featureDocumentHighlight: Boolean = true
    var featureCodeLens: Boolean = true
    var featureWorkspaceFileOps: Boolean = true
    // Pull diagnostics are opt-in: advertising diagnosticProvider flips
    // most LSP clients into pull mode and disables the push pipeline.
    var featurePullDiagnostics: Boolean = false
    var featureWillSaveWaitUntil: Boolean = false
    var featureImplementation: Boolean = true
    var featureTypeDefinition: Boolean = true
    var featureDeclaration: Boolean = true
    var featureLinkedEditingRange: Boolean = true

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
    var formattingDocstringStyle: String = "none"
    var formattingDocstringTagStyle: String = "doxygen"
    var formattingDocstringDecoration: Boolean = false
    var formattingDocstringDecorationChar: String = "."
    var formattingDocstringDecorationWidth: Int = 70

    // @generated:diagnostic-vars:begin
    var diagnosticE001: Boolean = true
    var diagnosticE002: Boolean = true
    var diagnosticE003: Boolean = true
    var diagnosticE005: Boolean = true
    var diagnosticE200: Boolean = true
    var diagnosticW001: Boolean = true
    var diagnosticW002: Boolean = true
    var diagnosticW003: Boolean = true
    var diagnosticW004: Boolean = true
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
    var diagnosticW116: Boolean = true
    var diagnosticW117: Boolean = true
    var diagnosticW118: Boolean = true
    var diagnosticW120: Boolean = true
    var diagnosticW121: Boolean = true
    var diagnosticW122: Boolean = true
    var diagnosticW124: Boolean = true
    var diagnosticW125: Boolean = true
    var diagnosticW126: Boolean = true
    var diagnosticW127: Boolean = true
    var diagnosticW128: Boolean = true
    var diagnosticW135: Boolean = true
    var diagnosticW136: Boolean = true
    var diagnosticW137: Boolean = true
    var diagnosticW138: Boolean = true
    var diagnosticW139: Boolean = true
    var diagnosticW200: Boolean = true
    var diagnosticW201: Boolean = true
    var diagnosticW230: Boolean = true
    var diagnosticW231: Boolean = true
    var diagnosticW232: Boolean = true
    var diagnosticW233: Boolean = true
    var diagnosticW240: Boolean = true
    var diagnosticW241: Boolean = true
    var diagnosticW250: Boolean = true
    var diagnosticW308: Boolean = true
    var diagnosticW210: Boolean = true
    var diagnosticW211: Boolean = true
    var diagnosticW212: Boolean = true
    var diagnosticW213: Boolean = true
    var diagnosticW214: Boolean = true
    var diagnosticW215: Boolean = true
    var diagnosticW216: Boolean = true
    var diagnosticW217: Boolean = true
    var diagnosticW218: Boolean = true
    var diagnosticW220: Boolean = true
    var diagnosticW101: Boolean = true
    var diagnosticW102: Boolean = true
    var diagnosticW103: Boolean = true
    var diagnosticW300: Boolean = true
    var diagnosticW301: Boolean = true
    var diagnosticW302: Boolean = true
    var diagnosticW303: Boolean = true
    var diagnosticW304: Boolean = true
    var diagnosticW306: Boolean = true
    var diagnosticW307: Boolean = true
    var diagnosticW309: Boolean = true
    var diagnosticW313: Boolean = true
    var diagnosticH300: Boolean = true
    var diagnosticI230: Boolean = true
    var diagnosticI231: Boolean = true
    var diagnosticW123: Boolean = true
    var diagnosticW242: Boolean = false
    var diagnosticS100: Boolean = true
    var diagnosticS101: Boolean = true
    var diagnosticS102: Boolean = true
    var diagnosticS103: Boolean = true
    var diagnosticS110: Boolean = true
    var diagnosticT100: Boolean = true
    var diagnosticT101: Boolean = true
    var diagnosticT102: Boolean = true
    var diagnosticT104: Boolean = true
    var diagnosticT105: Boolean = true
    var diagnosticIRULE1001: Boolean = true
    var diagnosticIRULE1002: Boolean = true
    var diagnosticIRULE1003: Boolean = true
    var diagnosticIRULE1004: Boolean = true
    var diagnosticIRULE1005: Boolean = true
    var diagnosticIRULE1006: Boolean = true
    var diagnosticIRULE1007: Boolean = true
    var diagnosticIRULE1008: Boolean = true
    var diagnosticIRULE1201: Boolean = true
    var diagnosticIRULE1202: Boolean = true
    var diagnosticIRULE2001: Boolean = true
    var diagnosticIRULE2002: Boolean = true
    var diagnosticIRULE2003: Boolean = true
    var diagnosticIRULE2101: Boolean = true
    var diagnosticIRULE5001: Boolean = true
    var diagnosticIRULE5002: Boolean = true
    var diagnosticIRULE5004: Boolean = true
    var diagnosticIRULE5005: Boolean = true
    var diagnosticIRULE5006: Boolean = true
    var diagnosticIRULE5007: Boolean = true
    var diagnosticIRULE3001: Boolean = true
    var diagnosticIRULE3002: Boolean = true
    var diagnosticIRULE3003: Boolean = true
    var diagnosticIRULE3004: Boolean = true
    var diagnosticIRULE3101: Boolean = true
    var diagnosticIRULE3102: Boolean = true
    var diagnosticIRULE4001: Boolean = true
    var diagnosticIRULE4002: Boolean = true
    var diagnosticIRULE4003: Boolean = true
    var diagnosticIRULE4004: Boolean = true
    var diagnosticIRULE4005: Boolean = true
    var diagnosticW130: Boolean = true
    var diagnosticW131: Boolean = true
    var diagnosticW132: Boolean = true
    var diagnosticW133: Boolean = true
    var diagnosticW134: Boolean = true
    // @generated:diagnostic-vars:end

    // Style

    var styleLineLength: Int = 120

    // @generated:optimiser-vars:begin
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
    var optimiserO127: Boolean = true
    var optimiserO128: Boolean = true
    var optimiserO129: Boolean = true
    var optimiserO130: Boolean = true
    // @generated:optimiser-vars:end

    // Shimmer

    var shimmerEnabled: Boolean = true

    // XC Diagnostics

    var xcDiagnosticsEnabled: Boolean = false

    // Runtime Validation

    var runtimeValidationEnabled: Boolean = false
    var runtimeValidationAdapter: String = "auto"
    var runtimeValidationTclshPath: String = ""
    var runtimeValidationTimeoutMs: Int = 5000

    // AI

    var aiEnabled: Boolean = false
    var aiExtraPrompts: String = ""  // JSON array stored as string

    // Diagnostic patterns

    var diagnosticsGenericVariablePatterns: String = ""  // newline-separated regexes

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
                "semanticTokens" to featureSemanticTokens,
                "codeActions" to featureCodeActions,
                "definition" to featureDefinition,
                "references" to featureReferences,
                "documentSymbols" to featureDocumentSymbols,
                "folding" to featureFolding,
                "rename" to featureRename,
                "signatureHelp" to featureSignatureHelp,
                "workspaceSymbols" to featureWorkspaceSymbols,
                "inlayTypeHints" to featureInlayTypeHints,
                "inlayParameterHints" to featureInlayParameterHints,
                "callHierarchy" to featureCallHierarchy,
                "documentLinks" to featureDocumentLinks,
                "selectionRange" to featureSelectionRange,
                "documentHighlight" to featureDocumentHighlight,
                "codeLens" to featureCodeLens,
                "workspaceFileOps" to featureWorkspaceFileOps,
                "pullDiagnostics" to featurePullDiagnostics,
                "implementation" to featureImplementation,
                "typeDefinition" to featureTypeDefinition,
                "declaration" to featureDeclaration,
                "linkedEditingRange" to featureLinkedEditingRange,
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
                "docstringStyle" to formattingDocstringStyle,
                "docstringTagStyle" to formattingDocstringTagStyle,
                "docstringDecoration" to formattingDocstringDecoration,
                "docstringDecorationChar" to formattingDocstringDecorationChar,
                "docstringDecorationWidth" to formattingDocstringDecorationWidth,
            ),
            "diagnostics" to mapOf(
                // @generated:diagnostic-map:begin
                "E001" to diagnosticE001,
                "E002" to diagnosticE002,
                "E003" to diagnosticE003,
                "E005" to diagnosticE005,
                "E200" to diagnosticE200,
                "W001" to diagnosticW001,
                "W002" to diagnosticW002,
                "W003" to diagnosticW003,
                "W004" to diagnosticW004,
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
                "W116" to diagnosticW116,
                "W117" to diagnosticW117,
                "W118" to diagnosticW118,
                "W120" to diagnosticW120,
                "W121" to diagnosticW121,
                "W122" to diagnosticW122,
                "W124" to diagnosticW124,
                "W125" to diagnosticW125,
                "W126" to diagnosticW126,
                "W127" to diagnosticW127,
                "W128" to diagnosticW128,
                "W135" to diagnosticW135,
                "W136" to diagnosticW136,
                "W137" to diagnosticW137,
                "W138" to diagnosticW138,
                "W139" to diagnosticW139,
                "W200" to diagnosticW200,
                "W201" to diagnosticW201,
                "W230" to diagnosticW230,
                "W231" to diagnosticW231,
                "W232" to diagnosticW232,
                "W233" to diagnosticW233,
                "W240" to diagnosticW240,
                "W241" to diagnosticW241,
                "W250" to diagnosticW250,
                "W308" to diagnosticW308,
                "W210" to diagnosticW210,
                "W211" to diagnosticW211,
                "W212" to diagnosticW212,
                "W213" to diagnosticW213,
                "W214" to diagnosticW214,
                "W215" to diagnosticW215,
                "W216" to diagnosticW216,
                "W217" to diagnosticW217,
                "W218" to diagnosticW218,
                "W220" to diagnosticW220,
                "W101" to diagnosticW101,
                "W102" to diagnosticW102,
                "W103" to diagnosticW103,
                "W300" to diagnosticW300,
                "W301" to diagnosticW301,
                "W302" to diagnosticW302,
                "W303" to diagnosticW303,
                "W304" to diagnosticW304,
                "W306" to diagnosticW306,
                "W307" to diagnosticW307,
                "W309" to diagnosticW309,
                "W313" to diagnosticW313,
                "H300" to diagnosticH300,
                "I230" to diagnosticI230,
                "I231" to diagnosticI231,
                "W123" to diagnosticW123,
                "W242" to diagnosticW242,
                "S100" to diagnosticS100,
                "S101" to diagnosticS101,
                "S102" to diagnosticS102,
                "S103" to diagnosticS103,
                "S110" to diagnosticS110,
                "T100" to diagnosticT100,
                "T101" to diagnosticT101,
                "T102" to diagnosticT102,
                "T104" to diagnosticT104,
                "T105" to diagnosticT105,
                "IRULE1001" to diagnosticIRULE1001,
                "IRULE1002" to diagnosticIRULE1002,
                "IRULE1003" to diagnosticIRULE1003,
                "IRULE1004" to diagnosticIRULE1004,
                "IRULE1005" to diagnosticIRULE1005,
                "IRULE1006" to diagnosticIRULE1006,
                "IRULE1007" to diagnosticIRULE1007,
                "IRULE1008" to diagnosticIRULE1008,
                "IRULE1201" to diagnosticIRULE1201,
                "IRULE1202" to diagnosticIRULE1202,
                "IRULE2001" to diagnosticIRULE2001,
                "IRULE2002" to diagnosticIRULE2002,
                "IRULE2003" to diagnosticIRULE2003,
                "IRULE2101" to diagnosticIRULE2101,
                "IRULE5001" to diagnosticIRULE5001,
                "IRULE5002" to diagnosticIRULE5002,
                "IRULE5004" to diagnosticIRULE5004,
                "IRULE5005" to diagnosticIRULE5005,
                "IRULE5006" to diagnosticIRULE5006,
                "IRULE5007" to diagnosticIRULE5007,
                "IRULE3001" to diagnosticIRULE3001,
                "IRULE3002" to diagnosticIRULE3002,
                "IRULE3003" to diagnosticIRULE3003,
                "IRULE3004" to diagnosticIRULE3004,
                "IRULE3101" to diagnosticIRULE3101,
                "IRULE3102" to diagnosticIRULE3102,
                "IRULE4001" to diagnosticIRULE4001,
                "IRULE4002" to diagnosticIRULE4002,
                "IRULE4003" to diagnosticIRULE4003,
                "IRULE4004" to diagnosticIRULE4004,
                "IRULE4005" to diagnosticIRULE4005,
                "W130" to diagnosticW130,
                "W131" to diagnosticW131,
                "W132" to diagnosticW132,
                "W133" to diagnosticW133,
                "W134" to diagnosticW134,
                // @generated:diagnostic-map:end
            ).let { map ->
                val patterns = diagnosticsGenericVariablePatterns
                    .split("\n").filter { it.isNotBlank() }
                if (patterns.isNotEmpty()) map + ("genericVariablePatterns" to patterns) else map
            },
            "style" to mapOf(
                "lineLength" to styleLineLength,
            ),
            "optimiser" to mapOf(
                // @generated:optimiser-map:begin
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
                "O127" to optimiserO127,
                "O128" to optimiserO128,
                "O129" to optimiserO129,
                "O130" to optimiserO130,
                // @generated:optimiser-map:end
            ),
            "shimmer" to mapOf(
                "enabled" to shimmerEnabled,
            ),
            "xcDiagnostics" to mapOf(
                "enabled" to xcDiagnosticsEnabled,
            ),
            "runtimeValidation" to mapOf(
                "enabled" to runtimeValidationEnabled,
                "adapter" to runtimeValidationAdapter,
                "tclshPath" to runtimeValidationTclshPath,
                "timeoutMs" to runtimeValidationTimeoutMs,
            ),
            "ai" to mapOf(
                "enabled" to aiEnabled,
                "extraPrompts" to aiExtraPrompts,
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
            "tcl9.1" to "Tcl 9.1",
            "f5-irules" to "F5 iRules",
            "f5-iapps" to "F5 iApps",
            "f5-tmsh" to "F5 tmsh Scripts",
            "f5-bigip" to "F5 BIG-IP",
            "synopsys-eda-tcl" to "Synopsys EDA",
            "cadence-eda-tcl" to "Cadence EDA",
            "xilinx-eda-tcl" to "Xilinx EDA",
            "intel-quartus-eda-tcl" to "Intel Quartus",
            "mentor-eda-tcl" to "Mentor EDA",
            "expect" to "Expect",
        )
    }
}
