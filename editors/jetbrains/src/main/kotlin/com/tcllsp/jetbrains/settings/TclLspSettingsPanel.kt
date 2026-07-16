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

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.ProjectManager
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.ui.TitledSeparator
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import com.intellij.util.ui.JBUI
import com.tcllsp.jetbrains.TclLspServerSupportProvider
import javax.swing.*

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.settings.TclLspSettingsPanel")

class TclLspSettingsPanel {

    // General
    private val serverPathField = JBTextField(30)
    private val dialectCombo = JComboBox(
        TclLspSettings.DIALECT_OPTIONS.map { it.second }.toTypedArray()
    )
    private val extraCommandsField = JBTextField(30)
    private val libraryPathsField = JBTextField(30)

    // Feature toggles
    private val featureHover = JBCheckBox("Hover")
    private val featureCompletion = JBCheckBox("Completion")
    private val featureDiagnostics = JBCheckBox("Diagnostics")
    private val featureSemanticTokens = JBCheckBox("Semantic tokens")
    private val featureCodeActions = JBCheckBox("Code actions")
    private val featureDefinition = JBCheckBox("Go to definition")
    private val featureReferences = JBCheckBox("Find references")
    private val featureDocumentSymbols = JBCheckBox("Document symbols")
    private val featureFolding = JBCheckBox("Code folding")
    private val featureRename = JBCheckBox("Rename symbol")
    private val featureSignatureHelp = JBCheckBox("Signature help")
    private val featureWorkspaceSymbols = JBCheckBox("Workspace symbols")
    private val featureInlayTypeHints = JBCheckBox("Inlay type hints")
    private val featureInlayParameterHints = JBCheckBox("Inlay parameter-name hints")
    private val featureCallHierarchy = JBCheckBox("Call hierarchy")
    private val featureDocumentLinks = JBCheckBox("Document links")
    private val featureSelectionRange = JBCheckBox("Selection range")
    private val featureDocumentHighlight = JBCheckBox("Document highlight")
    private val featureCodeLens = JBCheckBox("Code lens")
    private val featureWorkspaceFileOps = JBCheckBox("Auto-rewrite source paths on rename")
    private val featurePullDiagnostics =
        JBCheckBox("Pull diagnostics (opt-in; restart required)")
    private val featureImplementation = JBCheckBox("Go to implementation")
    private val featureTypeDefinition = JBCheckBox("Go to type definition")
    private val featureDeclaration = JBCheckBox("Go to declaration")
    private val featureLinkedEditingRange = JBCheckBox("Linked editing range")

    // Formatting
    private val fmtIndentSize = JSpinner(SpinnerNumberModel(4, 1, 16, 1))
    private val fmtIndentStyle = JComboBox(arrayOf("spaces", "tabs"))
    private val fmtContinuationIndent = JSpinner(SpinnerNumberModel(4, 0, 16, 1))
    private val fmtBraceStyle = JComboBox(arrayOf("k_and_r"))
    private val fmtSpaceBetweenBraces = JBCheckBox("Space between braces")
    private val fmtEnforceBracedVars = JBCheckBox("Enforce braced variables")
    private val fmtEnforceBracedExpr = JBCheckBox("Enforce braced expressions")
    private val fmtMaxLineLength = JSpinner(SpinnerNumberModel(120, 40, 500, 10))
    private val fmtGoalLineLength = JSpinner(SpinnerNumberModel(100, 40, 500, 10))
    private val fmtExpandSingleLine = JBCheckBox("Expand single-line bodies")
    private val fmtMinBodyCmds = JSpinner(SpinnerNumberModel(2, 1, 10, 1))
    private val fmtSpaceAfterHash = JBCheckBox("Space after # in comments")
    private val fmtTrimTrailing = JBCheckBox("Trim trailing whitespace")
    private val fmtAlignComments = JBCheckBox("Align comments to code")
    private val fmtReplaceSemicolons = JBCheckBox("Replace semicolons with newlines")
    private val fmtBlankProcs = JSpinner(SpinnerNumberModel(1, 0, 5, 1))
    private val fmtBlankBlocks = JSpinner(SpinnerNumberModel(1, 0, 5, 1))
    private val fmtMaxBlankLines = JSpinner(SpinnerNumberModel(2, 1, 10, 1))
    private val fmtLineEnding = JComboBox(arrayOf("lf", "crlf", "cr"))
    private val fmtFinalNewline = JBCheckBox("Ensure final newline")
    private val fmtDocstringStyle = JComboBox(arrayOf("preceding", "body", "none"))
    private val fmtDocstringTagStyle = JComboBox(arrayOf("doxygen", "plain", "none"))
    private val fmtDocstringDecoration = JBCheckBox("Docstring decoration borders")
    private val fmtDocstringDecorationChar = JComboBox(arrayOf(".", "-", "=", "*", "~"))
    private val fmtDocstringDecorationWidth = JSpinner(SpinnerNumberModel(70, 20, 120, 10))

    // @generated:diag-checkboxes:begin
    // Diagnostics — Errors
    private val diagE001 = JBCheckBox("E001: Missing dispatch word")
    private val diagE002 = JBCheckBox("E002: Too few arguments for command")
    private val diagE003 = JBCheckBox("E003: Too many arguments for command")
    private val diagE005 = JBCheckBox("E005: Wrong argument-count shape for command")
    private val diagE200 = JBCheckBox("E200: Unterminated command")

    // Diagnostics — Style & Best Practice
    private val diagW001 = JBCheckBox("W001: Unknown subcommand")
    private val diagW002 = JBCheckBox("W002: Command is disabled in active dialect profile")
    private val diagW003 = JBCheckBox("W003: Expression operator not available in active dialect")
    private val diagW004 = JBCheckBox("W004: Command option is not available in the active dialect")
    private val diagW100 = JBCheckBox("W100: Unbraced expression argument")
    private val diagW104 = JBCheckBox("W104: String concatenation for list building")
    private val diagW105 = JBCheckBox("W105: Unbraced code block or missing variable declaration ...")
    private val diagW106 = JBCheckBox("W106: Dangerous unbraced switch body")
    private val diagW108 = JBCheckBox("W108: Non-ASCII characters in token content")
    private val diagW110 = JBCheckBox("W110: Use eq/ne instead of ==/!= for string comparison")
    private val diagW111 = JBCheckBox("W111: Line exceeds maximum length (see tclLsp.style.lineLe...")
    private val diagW112 = JBCheckBox("W112: Trailing whitespace")
    private val diagW113 = JBCheckBox("W113: Procedure shadows built-in command")
    private val diagW114 = JBCheckBox("W114: Redundant nested [expr {...}]")
    private val diagW115 = JBCheckBox("W115: Backslash-newline in comment silently swallows the n...")
    private val diagW116 = JBCheckBox("W116: Stub command shadows built-in command")
    private val diagW117 = JBCheckBox("W117: Stub expression definition shadows built-in function...")
    private val diagW118 = JBCheckBox("W118: Inconsistent line endings")
    private val diagW120 = JBCheckBox("W120: Command used without a corresponding package require")
    private val diagW121 = JBCheckBox("W121: Subnet mask has non-contiguous bits")
    private val diagW122 = JBCheckBox("W122: Mistyped IPv4 address (octet > 255 or leading zero)")
    private val diagW124 = JBCheckBox("W124: Invalid IP address literal")
    private val diagW125 = JBCheckBox("W125: Orphaned control-flow keyword used as standalone com...")
    private val diagW126 = JBCheckBox("W126: Non-channel value in channel argument position")
    private val diagW127 = JBCheckBox("W127: Value not in the command's allowed set")
    private val diagW128 = JBCheckBox("W128: Command called after it was renamed or deleted earli...")
    private val diagW135 = JBCheckBox("W135: Command requires a newer package version than the re...")
    private val diagW136 = JBCheckBox("W136: Option requires a newer package version than the res...")
    private val diagW137 = JBCheckBox("W137: Argument value requires a newer Tcl version than the...")
    private val diagW138 = JBCheckBox("W138: Format/scan conversion requires a newer Tcl version ...")
    private val diagW200 = JBCheckBox("W200: exec result not captured or binary format modifier r...")
    private val diagW201 = JBCheckBox("W201: Manual path concatenation")
    private val diagW230 = JBCheckBox("W230: Constant list index out of range")
    private val diagW231 = JBCheckBox("W231: Constant list index out of range")
    private val diagW232 = JBCheckBox("W232: Constant string index out of range")
    private val diagW233 = JBCheckBox("W233: Division or modulo by a provably-zero divisor")
    private val diagW240 = JBCheckBox("W240: Loop condition is a constant false")
    private val diagW241 = JBCheckBox("W241: Loop is provably infinite")
    private val diagW250 = JBCheckBox("W250: Instantiating an oo::abstract class")
    private val diagW308 = JBCheckBox("W308: Unknown TclOO method")

    // Diagnostics — Variables
    private val diagW210 = JBCheckBox("W210: Variable read before set")
    private val diagW211 = JBCheckBox("W211: Variable set but never used")
    private val diagW212 = JBCheckBox("W212: Variable substitution where name expected (set \$x, i...")
    private val diagW213 = JBCheckBox("W213: Variable may not exist")
    private val diagW214 = JBCheckBox("W214: Unused proc parameter")
    private val diagW215 = JBCheckBox("W215: Variable name unreachable via \$-substitution (creata...")
    private val diagW216 = JBCheckBox("W216: Broken brace-form array element reference")
    private val diagW217 = JBCheckBox("W217: unset unsets nothing")
    private val diagW218 = JBCheckBox("W218: args in a non-final parameter position is an ordinar...")
    private val diagW220 = JBCheckBox("W220: Dead store")

    // Diagnostics — Security
    private val diagW101 = JBCheckBox("W101: eval with string concatenation")
    private val diagW102 = JBCheckBox("W102: subst on variable input")
    private val diagW103 = JBCheckBox("W103: open with pipeline |")
    private val diagW300 = JBCheckBox("W300: source with variable argument")
    private val diagW301 = JBCheckBox("W301: uplevel with string-built script")
    private val diagW302 = JBCheckBox("W302: catch without result variable")
    private val diagW303 = JBCheckBox("W303: Regexp vulnerable to catastrophic backtracking (ReDoS)")
    private val diagW304 = JBCheckBox("W304: Missing option terminator -- on option-bearing commands")
    private val diagW306 = JBCheckBox("W306: Substitution in literal-expected argument position")
    private val diagW307 = JBCheckBox("W307: Non-literal command name")
    private val diagW309 = JBCheckBox("W309: eval/uplevel with subst")
    private val diagW313 = JBCheckBox("W313: Destructive file operation with variable path")

    // Diagnostics — Hints
    private val diagH300 = JBCheckBox("H300: Possible paste error")
    private val diagI230 = JBCheckBox("I230: Constant branch condition")
    private val diagI231 = JBCheckBox("I231: Constant switch arm condition")
    private val diagW123 = JBCheckBox("W123: Unresolved command")
    private val diagW242 = JBCheckBox("W242: Loop termination cannot be proven")

    // Diagnostics — Shimmer
    private val diagS100 = JBCheckBox("S100: Single shimmer outside a loop")
    private val diagS101 = JBCheckBox("S101: Shimmer inside a loop body")
    private val diagS102 = JBCheckBox("S102: Variable oscillates between two types across loop it...")
    private val diagS103 = JBCheckBox("S103: Mutation of a potentially shared value copies it")
    private val diagS110 = JBCheckBox("S110: Byte-array value coerced to a string by a string ope...")

    // Diagnostics — Taint
    private val diagT100 = JBCheckBox("T100: Tainted data flows into a dangerous sink: eval/uplev...")
    private val diagT101 = JBCheckBox("T101: Tainted data flows into an output command (puts)")
    private val diagT102 = JBCheckBox("T102: Tainted data in option position without -- terminator")
    private val diagT104 = JBCheckBox("T104: Tainted data in a network-address argument (e.g. soc...")
    private val diagT105 = JBCheckBox("T105: Tainted data in a cross-interpreter eval subcommand ...")

    // Diagnostics — iRules
    private val diagIRULE1001 = JBCheckBox("IRULE1001: Command invalid or ineffective in this iRules event")
    private val diagIRULE1002 = JBCheckBox("IRULE1002: Unknown iRules event name")
    private val diagIRULE1003 = JBCheckBox("IRULE1003: Deprecated iRules event")
    private val diagIRULE1004 = JBCheckBox("IRULE1004: when block missing explicit priority")
    private val diagIRULE1005 = JBCheckBox("IRULE1005: Data event without a matching *::collect call")
    private val diagIRULE1006 = JBCheckBox("IRULE1006: *::payload without a matching *::collect call")
    private val diagIRULE1007 = JBCheckBox("IRULE1007: *::collect without a matching *::release on the same...")
    private val diagIRULE1008 = JBCheckBox("IRULE1008: *::release without a matching *::collect on the same...")
    private val diagIRULE1201 = JBCheckBox("IRULE1201: HTTP command used after HTTP::respond/HTTP::redirect")
    private val diagIRULE1202 = JBCheckBox("IRULE1202: Multiple HTTP::respond/HTTP::redirect on different b...")
    private val diagIRULE2001 = JBCheckBox("IRULE2001: Deprecated matchclass")
    private val diagIRULE2002 = JBCheckBox("IRULE2002: Deprecated iRules command")
    private val diagIRULE2003 = JBCheckBox("IRULE2003: Unsafe iRules command")
    private val diagIRULE2101 = JBCheckBox("IRULE2101: Heavy regexp in a high-frequency event")
    private val diagIRULE5001 = JBCheckBox("IRULE5001: Ungated log in a high-frequency event")
    private val diagIRULE5002 = JBCheckBox("IRULE5002: drop/reject/discard without event disable all or return")
    private val diagIRULE5004 = JBCheckBox("IRULE5004: DNS::return without return")
    private val diagIRULE5005 = JBCheckBox("IRULE5005: Direct proc invocation without call")
    private val diagIRULE5006 = JBCheckBox("IRULE5006: Top-level-only command used inside a nested body")
    private val diagIRULE5007 = JBCheckBox("IRULE5007: Event-context command used at top level outside a wh...")
    private val diagIRULE3001 = JBCheckBox("IRULE3001: Tainted data in HTTP response body")
    private val diagIRULE3002 = JBCheckBox("IRULE3002: Tainted data in HTTP header or cookie value")
    private val diagIRULE3003 = JBCheckBox("IRULE3003: Tainted data in log command")
    private val diagIRULE3004 = JBCheckBox("IRULE3004: Tainted data in an HTTP::redirect URL")
    private val diagIRULE3101 = JBCheckBox("IRULE3101: HTTP::uri/HTTP::path set to value not provably start...")
    private val diagIRULE3102 = JBCheckBox("IRULE3102: HTTP::path/HTTP::uri/HTTP::query getter used without...")
    private val diagIRULE4001 = JBCheckBox("IRULE4001: Write to static:: variable outside RULE_INIT")
    private val diagIRULE4002 = JBCheckBox("IRULE4002: Generic static:: variable name")
    private val diagIRULE4003 = JBCheckBox("IRULE4003: Variable scoping concern across events")
    private val diagIRULE4004 = JBCheckBox("IRULE4004: Constant set in per-request event could be hoisted t...")
    private val diagIRULE4005 = JBCheckBox("IRULE4005: Potential race")

    // Diagnostics — Package Manager
    private val diagW130 = JBCheckBox("W130: tclpkg.tcl requires package but it is not in tclpkg....")
    private val diagW131 = JBCheckBox("W131: tclpkg.lock is out of sync with tclpkg.tcl")
    private val diagW132 = JBCheckBox("W132: tclpkg.lock integrity mismatch")
    private val diagW133 = JBCheckBox("W133: tclpkg.tcl directive not permitted in safe mode")
    private val diagW134 = JBCheckBox("W134: Package resolved but no pkgIndex.tcl found")
    // @generated:diag-checkboxes:end

    // XC Diagnostics
    private val xcDiagnosticsEnabled = JBCheckBox("Enable XC translatability diagnostics")

    // Style
    private val styleLineLength = JSpinner(SpinnerNumberModel(120, 40, 500, 10))

    // @generated:opt-checkboxes:begin
    private val optEnabled = JBCheckBox("Enable optimiser suggestions")
    private val optO100 = JBCheckBox("O100: Propagate constant variables into expressions and co...")
    private val optO101 = JBCheckBox("O101: Fold constant integer expressions")
    private val optO102 = JBCheckBox("O102: Forward a variable's single reaching literal load to...")
    private val optO103 = JBCheckBox("O103: Fold static procedure calls using interprocedural su...")
    private val optO104 = JBCheckBox("O104: Fold static string build chains into a single assign...")
    private val optO105 = JBCheckBox("O105: Propagate constants into variable references and det...")
    private val optO106 = JBCheckBox("O106: Hoist loop-invariant computations")
    private val optO107 = JBCheckBox("O107: Eliminate unreachable dead code")
    private val optO108 = JBCheckBox("O108: Eliminate transitively dead code")
    private val optO109 = JBCheckBox("O109: Eliminate dead stores")
    private val optO110 = JBCheckBox("O110: Canonicalise expressions (InstCombine)")
    private val optO111 = JBCheckBox("O111: Brace expression performance hints (paired with W100)")
    private val optO112 = JBCheckBox("O112: Eliminate constant-condition compound statements")
    private val optO113 = JBCheckBox("O113: Strength-reduce expressions (x**2 → x*x, x%8 → x&7)")
    private val optO114 = JBCheckBox("O114: Recognise incr idiom (set x [expr {\$x + N}] → incr x N)")
    private val optO115 = JBCheckBox("O115: Remove redundant nested [expr {...}] in expression c...")
    private val optO116 = JBCheckBox("O116: Fold constant [list a b c] to literal value")
    private val optO117 = JBCheckBox("O117: Simplify [string length \$s] == 0 → \$s eq \"\"")
    private val optO118 = JBCheckBox("O118: Fold constant [lindex {a b c} 1] to element")
    private val optO119 = JBCheckBox("O119: Pack consecutive set literals into lassign/foreach")
    private val optO120 = JBCheckBox("O120: Prefer eq/ne over ==/!= for string comparisons")
    private val optO121 = JBCheckBox("O121: Rewrite self-recursive tail calls to tailcall")
    private val optO122 = JBCheckBox("O122: Convert fully tail-recursive proc to iterative while...")
    private val optO123 = JBCheckBox("O123: Detect non-tail recursion eligible for accumulator i...")
    private val optO124 = JBCheckBox("O124: Comment out unused procs in iRules (not called from ...")
    private val optO125 = JBCheckBox("O125: Sink side-effect-free assignments into the deepest d...")
    private val optO126 = JBCheckBox("O126: Remove unused variable assignments")
    private val optO127 = JBCheckBox("O127: Inline single-use variable assignment")
    private val optO128 = JBCheckBox("O128: Rewrite [expr {[llength \$L] - N}] / [expr {[string l...")
    private val optO129 = JBCheckBox("O129: Fold a pure builtin command substitution with consta...")
    private val optO130 = JBCheckBox("O130: Fold static lappend list build chains into a single ...")
    // @generated:opt-checkboxes:end

    // Shimmer
    private val shimmerEnabled = JBCheckBox("Enable shimmer analysis")

    // Runtime Validation
    private val runtimeValidation = JBCheckBox("Enable runtime validation on save")
    private val rtAdapter = JComboBox(arrayOf("auto", "tclsh", "expect"))
    private val rtTclshPath = JBTextField(30)
    private val rtTimeoutMs = JSpinner(SpinnerNumberModel(5000, 500, 120000, 500))

    // AI
    private val aiEnabled = JBCheckBox("Enable AI features")
    private val aiExtraPrompts = JBTextField(30)

    // Diagnostic patterns
    private val genericPatternsField = JBTextField(30)

    val root: JComponent

    init {
        val builder = FormBuilder.createFormBuilder()

        // General section
        builder.addComponent(TitledSeparator("General"))
        builder.addLabeledComponent(JBLabel("Server path:"), serverPathField)
        builder.addTooltip("Path to a tcl-lsp checkout root (probes target/{release,debug}/tcl-lsp-server) or directly to a built native binary (dev mode). Leave empty to use the bundled server.")
        builder.addLabeledComponent(JBLabel("Dialect:"), dialectCombo)
        builder.addLabeledComponent(JBLabel("Extra commands:"), extraCommandsField)
        builder.addTooltip("Comma-separated list of additional command names to treat as known.")
        builder.addLabeledComponent(JBLabel("Library paths:"), libraryPathsField)
        builder.addTooltip("Comma-separated directories to scan for Tcl packages.")

        // Features section
        builder.addComponent(TitledSeparator("Features"))
        val featurePanel = JPanel().apply {
            layout = BoxLayout(this, BoxLayout.Y_AXIS)
            val features = listOf(
                featureHover, featureCompletion, featureDiagnostics,
                featureSemanticTokens, featureCodeActions, featureDefinition, featureReferences,
                featureDocumentSymbols, featureFolding, featureRename, featureSignatureHelp,
                featureWorkspaceSymbols, featureInlayTypeHints, featureInlayParameterHints,
                featureCallHierarchy,
                featureDocumentLinks, featureSelectionRange,
                featureDocumentHighlight, featureCodeLens, featureWorkspaceFileOps,
                featurePullDiagnostics,
                featureImplementation, featureTypeDefinition, featureDeclaration,
                featureLinkedEditingRange,
            )
            // Lay out in a 3-column grid
            val grid = JPanel(java.awt.GridLayout(0, 3, 8, 2))
            features.forEach { grid.add(it) }
            add(grid)
        }
        builder.addComponent(featurePanel)

        // Formatting section
        builder.addComponent(TitledSeparator("Formatting"))
        builder.addLabeledComponent(JBLabel("Indent size:"), fmtIndentSize)
        builder.addLabeledComponent(JBLabel("Indent style:"), fmtIndentStyle)
        builder.addLabeledComponent(JBLabel("Continuation indent:"), fmtContinuationIndent)
        builder.addLabeledComponent(JBLabel("Brace style:"), fmtBraceStyle)
        builder.addComponent(fmtSpaceBetweenBraces)
        builder.addComponent(fmtEnforceBracedVars)
        builder.addComponent(fmtEnforceBracedExpr)
        builder.addLabeledComponent(JBLabel("Max line length:"), fmtMaxLineLength)
        builder.addLabeledComponent(JBLabel("Goal line length:"), fmtGoalLineLength)
        builder.addComponent(fmtExpandSingleLine)
        builder.addLabeledComponent(JBLabel("Min body commands for expansion:"), fmtMinBodyCmds)
        builder.addComponent(fmtSpaceAfterHash)
        builder.addComponent(fmtTrimTrailing)
        builder.addComponent(fmtAlignComments)
        builder.addComponent(fmtReplaceSemicolons)
        builder.addLabeledComponent(JBLabel("Blank lines between procs:"), fmtBlankProcs)
        builder.addLabeledComponent(JBLabel("Blank lines between blocks:"), fmtBlankBlocks)
        builder.addLabeledComponent(JBLabel("Max consecutive blank lines:"), fmtMaxBlankLines)
        builder.addLabeledComponent(JBLabel("Line ending:"), fmtLineEnding)
        builder.addComponent(fmtFinalNewline)

        builder.addComponent(TitledSeparator("Docstrings"))
        builder.addLabeledComponent(JBLabel("Docstring style:"), fmtDocstringStyle)
        builder.addLabeledComponent(JBLabel("Docstring tag style:"), fmtDocstringTagStyle)
        builder.addComponent(fmtDocstringDecoration)
        builder.addLabeledComponent(JBLabel("Decoration character:"), fmtDocstringDecorationChar)
        builder.addLabeledComponent(JBLabel("Decoration width:"), fmtDocstringDecorationWidth)

        // @generated:diag-ui:begin
        builder.addComponent(TitledSeparator("Diagnostics — Errors"))
        val diagErrorPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagE001, diagE002, diagE003, diagE005, diagE200,
        ).forEach { diagErrorPanel.add(it) }
        builder.addComponent(diagErrorPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Style & Best Practice"))
        val diagWarnPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagW001, diagW002, diagW003, diagW004, diagW100, diagW104,
            diagW105, diagW106, diagW108, diagW110, diagW111, diagW112,
            diagW113, diagW114, diagW115, diagW116, diagW117, diagW118,
            diagW120, diagW121, diagW122, diagW124, diagW125, diagW126,
            diagW127, diagW128, diagW135, diagW136, diagW137, diagW138,
            diagW200, diagW201, diagW230, diagW231, diagW232, diagW233,
            diagW240, diagW241, diagW250, diagW308,
        ).forEach { diagWarnPanel.add(it) }
        builder.addComponent(diagWarnPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Variables"))
        val diagVarPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagW210, diagW211, diagW212, diagW213, diagW214, diagW215,
            diagW216, diagW217, diagW218, diagW220,
        ).forEach { diagVarPanel.add(it) }
        builder.addComponent(diagVarPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Security"))
        val diagSecPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagW101, diagW102, diagW103, diagW300, diagW301, diagW302,
            diagW303, diagW304, diagW306, diagW307, diagW309, diagW313,
        ).forEach { diagSecPanel.add(it) }
        builder.addComponent(diagSecPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Hints"))
        val diagHintPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagH300, diagI230, diagI231, diagW123, diagW242,
        ).forEach { diagHintPanel.add(it) }
        builder.addComponent(diagHintPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Shimmer"))
        val diagShimmerPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagS100, diagS101, diagS102, diagS103, diagS110,
        ).forEach { diagShimmerPanel.add(it) }
        builder.addComponent(diagShimmerPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Taint"))
        val diagTaintPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagT100, diagT101, diagT102, diagT104, diagT105,
        ).forEach { diagTaintPanel.add(it) }
        builder.addComponent(diagTaintPanel)

        builder.addComponent(TitledSeparator("Diagnostics — iRules"))
        val diagIRulePanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagIRULE1001, diagIRULE1002, diagIRULE1003, diagIRULE1004, diagIRULE1005, diagIRULE1006,
            diagIRULE1007, diagIRULE1008, diagIRULE1201, diagIRULE1202, diagIRULE2001, diagIRULE2002,
            diagIRULE2003, diagIRULE2101, diagIRULE5001, diagIRULE5002, diagIRULE5004, diagIRULE5005,
            diagIRULE5006, diagIRULE5007, diagIRULE3001, diagIRULE3002, diagIRULE3003, diagIRULE3004,
            diagIRULE3101, diagIRULE3102, diagIRULE4001, diagIRULE4002, diagIRULE4003, diagIRULE4004,
            diagIRULE4005,
        ).forEach { diagIRulePanel.add(it) }
        builder.addComponent(diagIRulePanel)

        builder.addComponent(TitledSeparator("Diagnostics — Package Manager"))
        val diagPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagW130, diagW131, diagW132, diagW133, diagW134,
        ).forEach { diagPanel.add(it) }
        builder.addComponent(diagPanel)
        // @generated:diag-ui:end

        // Style section
        builder.addComponent(TitledSeparator("Style"))
        builder.addLabeledComponent(JBLabel("Line length (W111 threshold):"), styleLineLength)

        // @generated:opt-ui:begin
        builder.addComponent(TitledSeparator("Optimiser"))
        builder.addComponent(optEnabled)
        val optPanel = JPanel(java.awt.GridLayout(0, 4, 8, 2))
        listOf(
            optO100, optO101, optO102, optO103, optO104, optO105,
            optO106, optO107, optO108, optO109, optO110, optO111,
            optO112, optO113, optO114, optO115, optO116, optO117,
            optO118, optO119, optO120, optO121, optO122, optO123,
            optO124, optO125, optO126, optO127, optO128, optO129,
            optO130,
        ).forEach { optPanel.add(it) }
        builder.addComponent(optPanel)
        // @generated:opt-ui:end

        // Shimmer section
        builder.addComponent(TitledSeparator("Shimmer"))
        builder.addComponent(shimmerEnabled)

        // XC Diagnostics
        builder.addComponent(TitledSeparator("XC Diagnostics"))
        builder.addComponent(xcDiagnosticsEnabled)

        // Runtime Validation
        builder.addComponent(TitledSeparator("Runtime Validation"))
        builder.addComponent(runtimeValidation)
        builder.addLabeledComponent(JBLabel("Adapter mode:"), rtAdapter)
        builder.addTooltip("auto: detect from dialect.  tclsh: use tclsh.  expect: use Expect.")
        builder.addLabeledComponent(JBLabel("tclsh path:"), rtTclshPath)
        builder.addTooltip("Path to tclsh interpreter. Leave empty for auto-discovery.")
        builder.addLabeledComponent(JBLabel("Timeout (ms):"), rtTimeoutMs)

        // AI
        builder.addComponent(TitledSeparator("AI"))
        builder.addComponent(aiEnabled)
        builder.addLabeledComponent(JBLabel("Extra prompts (JSON):"), aiExtraPrompts)
        builder.addTooltip("JSON array of prompt objects for AI-assisted features.")

        // Diagnostic patterns
        builder.addComponent(TitledSeparator("Diagnostic Patterns"))
        builder.addLabeledComponent(JBLabel("Generic variable patterns:"), genericPatternsField)
        builder.addTooltip("Newline-separated regex patterns for IRULE4002 generic variable detection.")

        builder.addComponentFillVertically(JPanel(), 0)

        root = JScrollPane(builder.panel).apply {
            border = JBUI.Borders.empty()
        }

        reset()
    }

    fun isModified(): Boolean {
        val s = TclLspSettings.getInstance()
        return serverPathField.text != s.serverPath ||
            dialectCombo.selectedIndex != TclLspSettings.DIALECT_OPTIONS.indexOfFirst { it.first == s.dialect } ||
            extraCommandsField.text != s.extraCommands ||
            libraryPathsField.text != s.libraryPaths ||
            // Features
            featureHover.isSelected != s.featureHover ||
            featureCompletion.isSelected != s.featureCompletion ||
            featureDiagnostics.isSelected != s.featureDiagnostics ||
            featureSemanticTokens.isSelected != s.featureSemanticTokens ||
            featureCodeActions.isSelected != s.featureCodeActions ||
            featureDefinition.isSelected != s.featureDefinition ||
            featureReferences.isSelected != s.featureReferences ||
            featureDocumentSymbols.isSelected != s.featureDocumentSymbols ||
            featureFolding.isSelected != s.featureFolding ||
            featureRename.isSelected != s.featureRename ||
            featureSignatureHelp.isSelected != s.featureSignatureHelp ||
            featureWorkspaceSymbols.isSelected != s.featureWorkspaceSymbols ||
            featureInlayTypeHints.isSelected != s.featureInlayTypeHints ||
            featureInlayParameterHints.isSelected != s.featureInlayParameterHints ||
            featureCallHierarchy.isSelected != s.featureCallHierarchy ||
            featureDocumentLinks.isSelected != s.featureDocumentLinks ||
            featureSelectionRange.isSelected != s.featureSelectionRange ||
            featureDocumentHighlight.isSelected != s.featureDocumentHighlight ||
            featureCodeLens.isSelected != s.featureCodeLens ||
            featureWorkspaceFileOps.isSelected != s.featureWorkspaceFileOps ||
            featurePullDiagnostics.isSelected != s.featurePullDiagnostics ||
            featureImplementation.isSelected != s.featureImplementation ||
            featureTypeDefinition.isSelected != s.featureTypeDefinition ||
            featureDeclaration.isSelected != s.featureDeclaration ||
            featureLinkedEditingRange.isSelected != s.featureLinkedEditingRange ||
            // Formatting
            (fmtIndentSize.value as Int) != s.formattingIndentSize ||
            fmtIndentStyle.selectedItem != s.formattingIndentStyle ||
            (fmtContinuationIndent.value as Int) != s.formattingContinuationIndent ||
            fmtBraceStyle.selectedItem != s.formattingBraceStyle ||
            fmtSpaceBetweenBraces.isSelected != s.formattingSpaceBetweenBraces ||
            fmtEnforceBracedVars.isSelected != s.formattingEnforceBracedVariables ||
            fmtEnforceBracedExpr.isSelected != s.formattingEnforceBracedExpr ||
            (fmtMaxLineLength.value as Int) != s.formattingMaxLineLength ||
            (fmtGoalLineLength.value as Int) != s.formattingGoalLineLength ||
            fmtExpandSingleLine.isSelected != s.formattingExpandSingleLineBodies ||
            (fmtMinBodyCmds.value as Int) != s.formattingMinBodyCommandsForExpansion ||
            fmtSpaceAfterHash.isSelected != s.formattingSpaceAfterCommentHash ||
            fmtTrimTrailing.isSelected != s.formattingTrimTrailingWhitespace ||
            fmtAlignComments.isSelected != s.formattingAlignCommentsToCode ||
            fmtReplaceSemicolons.isSelected != s.formattingReplaceSemicolonsWithNewlines ||
            (fmtBlankProcs.value as Int) != s.formattingBlankLinesBetweenProcs ||
            (fmtBlankBlocks.value as Int) != s.formattingBlankLinesBetweenBlocks ||
            (fmtMaxBlankLines.value as Int) != s.formattingMaxConsecutiveBlankLines ||
            fmtLineEnding.selectedItem != s.formattingLineEnding ||
            fmtFinalNewline.isSelected != s.formattingEnsureFinalNewline ||
            fmtDocstringStyle.selectedItem != s.formattingDocstringStyle ||
            fmtDocstringTagStyle.selectedItem != s.formattingDocstringTagStyle ||
            fmtDocstringDecoration.isSelected != s.formattingDocstringDecoration ||
            fmtDocstringDecorationChar.selectedItem != s.formattingDocstringDecorationChar ||
            (fmtDocstringDecorationWidth.value as Int) != s.formattingDocstringDecorationWidth ||
            // @generated:diag-dirty:begin
            diagE001.isSelected != s.diagnosticE001 ||
            diagE002.isSelected != s.diagnosticE002 ||
            diagE003.isSelected != s.diagnosticE003 ||
            diagE005.isSelected != s.diagnosticE005 ||
            diagE200.isSelected != s.diagnosticE200 ||
            diagW001.isSelected != s.diagnosticW001 ||
            diagW002.isSelected != s.diagnosticW002 ||
            diagW003.isSelected != s.diagnosticW003 ||
            diagW004.isSelected != s.diagnosticW004 ||
            diagW100.isSelected != s.diagnosticW100 ||
            diagW104.isSelected != s.diagnosticW104 ||
            diagW105.isSelected != s.diagnosticW105 ||
            diagW106.isSelected != s.diagnosticW106 ||
            diagW108.isSelected != s.diagnosticW108 ||
            diagW110.isSelected != s.diagnosticW110 ||
            diagW111.isSelected != s.diagnosticW111 ||
            diagW112.isSelected != s.diagnosticW112 ||
            diagW113.isSelected != s.diagnosticW113 ||
            diagW114.isSelected != s.diagnosticW114 ||
            diagW115.isSelected != s.diagnosticW115 ||
            diagW116.isSelected != s.diagnosticW116 ||
            diagW117.isSelected != s.diagnosticW117 ||
            diagW118.isSelected != s.diagnosticW118 ||
            diagW120.isSelected != s.diagnosticW120 ||
            diagW121.isSelected != s.diagnosticW121 ||
            diagW122.isSelected != s.diagnosticW122 ||
            diagW124.isSelected != s.diagnosticW124 ||
            diagW125.isSelected != s.diagnosticW125 ||
            diagW126.isSelected != s.diagnosticW126 ||
            diagW127.isSelected != s.diagnosticW127 ||
            diagW128.isSelected != s.diagnosticW128 ||
            diagW135.isSelected != s.diagnosticW135 ||
            diagW136.isSelected != s.diagnosticW136 ||
            diagW137.isSelected != s.diagnosticW137 ||
            diagW138.isSelected != s.diagnosticW138 ||
            diagW200.isSelected != s.diagnosticW200 ||
            diagW201.isSelected != s.diagnosticW201 ||
            diagW230.isSelected != s.diagnosticW230 ||
            diagW231.isSelected != s.diagnosticW231 ||
            diagW232.isSelected != s.diagnosticW232 ||
            diagW233.isSelected != s.diagnosticW233 ||
            diagW240.isSelected != s.diagnosticW240 ||
            diagW241.isSelected != s.diagnosticW241 ||
            diagW250.isSelected != s.diagnosticW250 ||
            diagW308.isSelected != s.diagnosticW308 ||
            diagW210.isSelected != s.diagnosticW210 ||
            diagW211.isSelected != s.diagnosticW211 ||
            diagW212.isSelected != s.diagnosticW212 ||
            diagW213.isSelected != s.diagnosticW213 ||
            diagW214.isSelected != s.diagnosticW214 ||
            diagW215.isSelected != s.diagnosticW215 ||
            diagW216.isSelected != s.diagnosticW216 ||
            diagW217.isSelected != s.diagnosticW217 ||
            diagW218.isSelected != s.diagnosticW218 ||
            diagW220.isSelected != s.diagnosticW220 ||
            diagW101.isSelected != s.diagnosticW101 ||
            diagW102.isSelected != s.diagnosticW102 ||
            diagW103.isSelected != s.diagnosticW103 ||
            diagW300.isSelected != s.diagnosticW300 ||
            diagW301.isSelected != s.diagnosticW301 ||
            diagW302.isSelected != s.diagnosticW302 ||
            diagW303.isSelected != s.diagnosticW303 ||
            diagW304.isSelected != s.diagnosticW304 ||
            diagW306.isSelected != s.diagnosticW306 ||
            diagW307.isSelected != s.diagnosticW307 ||
            diagW309.isSelected != s.diagnosticW309 ||
            diagW313.isSelected != s.diagnosticW313 ||
            diagH300.isSelected != s.diagnosticH300 ||
            diagI230.isSelected != s.diagnosticI230 ||
            diagI231.isSelected != s.diagnosticI231 ||
            diagW123.isSelected != s.diagnosticW123 ||
            diagW242.isSelected != s.diagnosticW242 ||
            diagS100.isSelected != s.diagnosticS100 ||
            diagS101.isSelected != s.diagnosticS101 ||
            diagS102.isSelected != s.diagnosticS102 ||
            diagS103.isSelected != s.diagnosticS103 ||
            diagS110.isSelected != s.diagnosticS110 ||
            diagT100.isSelected != s.diagnosticT100 ||
            diagT101.isSelected != s.diagnosticT101 ||
            diagT102.isSelected != s.diagnosticT102 ||
            diagT104.isSelected != s.diagnosticT104 ||
            diagT105.isSelected != s.diagnosticT105 ||
            diagIRULE1001.isSelected != s.diagnosticIRULE1001 ||
            diagIRULE1002.isSelected != s.diagnosticIRULE1002 ||
            diagIRULE1003.isSelected != s.diagnosticIRULE1003 ||
            diagIRULE1004.isSelected != s.diagnosticIRULE1004 ||
            diagIRULE1005.isSelected != s.diagnosticIRULE1005 ||
            diagIRULE1006.isSelected != s.diagnosticIRULE1006 ||
            diagIRULE1007.isSelected != s.diagnosticIRULE1007 ||
            diagIRULE1008.isSelected != s.diagnosticIRULE1008 ||
            diagIRULE1201.isSelected != s.diagnosticIRULE1201 ||
            diagIRULE1202.isSelected != s.diagnosticIRULE1202 ||
            diagIRULE2001.isSelected != s.diagnosticIRULE2001 ||
            diagIRULE2002.isSelected != s.diagnosticIRULE2002 ||
            diagIRULE2003.isSelected != s.diagnosticIRULE2003 ||
            diagIRULE2101.isSelected != s.diagnosticIRULE2101 ||
            diagIRULE5001.isSelected != s.diagnosticIRULE5001 ||
            diagIRULE5002.isSelected != s.diagnosticIRULE5002 ||
            diagIRULE5004.isSelected != s.diagnosticIRULE5004 ||
            diagIRULE5005.isSelected != s.diagnosticIRULE5005 ||
            diagIRULE5006.isSelected != s.diagnosticIRULE5006 ||
            diagIRULE5007.isSelected != s.diagnosticIRULE5007 ||
            diagIRULE3001.isSelected != s.diagnosticIRULE3001 ||
            diagIRULE3002.isSelected != s.diagnosticIRULE3002 ||
            diagIRULE3003.isSelected != s.diagnosticIRULE3003 ||
            diagIRULE3004.isSelected != s.diagnosticIRULE3004 ||
            diagIRULE3101.isSelected != s.diagnosticIRULE3101 ||
            diagIRULE3102.isSelected != s.diagnosticIRULE3102 ||
            diagIRULE4001.isSelected != s.diagnosticIRULE4001 ||
            diagIRULE4002.isSelected != s.diagnosticIRULE4002 ||
            diagIRULE4003.isSelected != s.diagnosticIRULE4003 ||
            diagIRULE4004.isSelected != s.diagnosticIRULE4004 ||
            diagIRULE4005.isSelected != s.diagnosticIRULE4005 ||
            diagW130.isSelected != s.diagnosticW130 ||
            diagW131.isSelected != s.diagnosticW131 ||
            diagW132.isSelected != s.diagnosticW132 ||
            diagW133.isSelected != s.diagnosticW133 ||
            diagW134.isSelected != s.diagnosticW134 ||
            // @generated:diag-dirty:end
            // XC Diagnostics
            xcDiagnosticsEnabled.isSelected != s.xcDiagnosticsEnabled ||
            // Style
            (styleLineLength.value as Int) != s.styleLineLength ||
            // @generated:opt-dirty:begin
            optEnabled.isSelected != s.optimiserEnabled ||
            optO100.isSelected != s.optimiserO100 ||
            optO101.isSelected != s.optimiserO101 ||
            optO102.isSelected != s.optimiserO102 ||
            optO103.isSelected != s.optimiserO103 ||
            optO104.isSelected != s.optimiserO104 ||
            optO105.isSelected != s.optimiserO105 ||
            optO106.isSelected != s.optimiserO106 ||
            optO107.isSelected != s.optimiserO107 ||
            optO108.isSelected != s.optimiserO108 ||
            optO109.isSelected != s.optimiserO109 ||
            optO110.isSelected != s.optimiserO110 ||
            optO111.isSelected != s.optimiserO111 ||
            optO112.isSelected != s.optimiserO112 ||
            optO113.isSelected != s.optimiserO113 ||
            optO114.isSelected != s.optimiserO114 ||
            optO115.isSelected != s.optimiserO115 ||
            optO116.isSelected != s.optimiserO116 ||
            optO117.isSelected != s.optimiserO117 ||
            optO118.isSelected != s.optimiserO118 ||
            optO119.isSelected != s.optimiserO119 ||
            optO120.isSelected != s.optimiserO120 ||
            optO121.isSelected != s.optimiserO121 ||
            optO122.isSelected != s.optimiserO122 ||
            optO123.isSelected != s.optimiserO123 ||
            optO124.isSelected != s.optimiserO124 ||
            optO125.isSelected != s.optimiserO125 ||
            optO126.isSelected != s.optimiserO126 ||
            optO127.isSelected != s.optimiserO127 ||
            optO128.isSelected != s.optimiserO128 ||
            optO129.isSelected != s.optimiserO129 ||
            optO130.isSelected != s.optimiserO130 ||
            // @generated:opt-dirty:end
            // Shimmer
            shimmerEnabled.isSelected != s.shimmerEnabled ||
            // Runtime validation
            runtimeValidation.isSelected != s.runtimeValidationEnabled ||
            rtAdapter.selectedItem != s.runtimeValidationAdapter ||
            rtTclshPath.text != s.runtimeValidationTclshPath ||
            (rtTimeoutMs.value as Int) != s.runtimeValidationTimeoutMs ||
            // AI
            aiEnabled.isSelected != s.aiEnabled ||
            aiExtraPrompts.text != s.aiExtraPrompts ||
            // Diagnostic patterns
            genericPatternsField.text != s.diagnosticsGenericVariablePatterns
    }

    fun apply() {
        val s = TclLspSettings.getInstance()
        // Capture the pre-apply launch settings so we can detect whether
        // the LSP server needs to be restarted to pick up a new command
        // line. Other settings flow through workspace/configuration on
        // the next request and don't require a restart.
        val oldServerPath = s.serverPath
        s.serverPath = serverPathField.text
        s.dialect = TclLspSettings.DIALECT_OPTIONS.getOrNull(dialectCombo.selectedIndex)?.first ?: "tcl8.6"
        s.extraCommands = extraCommandsField.text
        s.libraryPaths = libraryPathsField.text

        s.featureHover = featureHover.isSelected
        s.featureCompletion = featureCompletion.isSelected
        s.featureDiagnostics = featureDiagnostics.isSelected
        s.featureSemanticTokens = featureSemanticTokens.isSelected
        s.featureCodeActions = featureCodeActions.isSelected
        s.featureDefinition = featureDefinition.isSelected
        s.featureReferences = featureReferences.isSelected
        s.featureDocumentSymbols = featureDocumentSymbols.isSelected
        s.featureFolding = featureFolding.isSelected
        s.featureRename = featureRename.isSelected
        s.featureSignatureHelp = featureSignatureHelp.isSelected
        s.featureWorkspaceSymbols = featureWorkspaceSymbols.isSelected
        s.featureInlayTypeHints = featureInlayTypeHints.isSelected
        s.featureInlayParameterHints = featureInlayParameterHints.isSelected
        s.featureCallHierarchy = featureCallHierarchy.isSelected
        s.featureDocumentLinks = featureDocumentLinks.isSelected
        s.featureSelectionRange = featureSelectionRange.isSelected
        s.featureDocumentHighlight = featureDocumentHighlight.isSelected
        s.featureCodeLens = featureCodeLens.isSelected
        s.featureWorkspaceFileOps = featureWorkspaceFileOps.isSelected
        s.featurePullDiagnostics = featurePullDiagnostics.isSelected
        s.featureImplementation = featureImplementation.isSelected
        s.featureTypeDefinition = featureTypeDefinition.isSelected
        s.featureDeclaration = featureDeclaration.isSelected
        s.featureLinkedEditingRange = featureLinkedEditingRange.isSelected

        s.formattingIndentSize = fmtIndentSize.value as Int
        s.formattingIndentStyle = fmtIndentStyle.selectedItem as String
        s.formattingContinuationIndent = fmtContinuationIndent.value as Int
        s.formattingBraceStyle = fmtBraceStyle.selectedItem as String
        s.formattingSpaceBetweenBraces = fmtSpaceBetweenBraces.isSelected
        s.formattingEnforceBracedVariables = fmtEnforceBracedVars.isSelected
        s.formattingEnforceBracedExpr = fmtEnforceBracedExpr.isSelected
        s.formattingMaxLineLength = fmtMaxLineLength.value as Int
        s.formattingGoalLineLength = fmtGoalLineLength.value as Int
        s.formattingExpandSingleLineBodies = fmtExpandSingleLine.isSelected
        s.formattingMinBodyCommandsForExpansion = fmtMinBodyCmds.value as Int
        s.formattingSpaceAfterCommentHash = fmtSpaceAfterHash.isSelected
        s.formattingTrimTrailingWhitespace = fmtTrimTrailing.isSelected
        s.formattingAlignCommentsToCode = fmtAlignComments.isSelected
        s.formattingReplaceSemicolonsWithNewlines = fmtReplaceSemicolons.isSelected
        s.formattingBlankLinesBetweenProcs = fmtBlankProcs.value as Int
        s.formattingBlankLinesBetweenBlocks = fmtBlankBlocks.value as Int
        s.formattingMaxConsecutiveBlankLines = fmtMaxBlankLines.value as Int
        s.formattingLineEnding = fmtLineEnding.selectedItem as String
        s.formattingEnsureFinalNewline = fmtFinalNewline.isSelected
        s.formattingDocstringStyle = fmtDocstringStyle.selectedItem as String
        s.formattingDocstringTagStyle = fmtDocstringTagStyle.selectedItem as String
        s.formattingDocstringDecoration = fmtDocstringDecoration.isSelected
        s.formattingDocstringDecorationChar = fmtDocstringDecorationChar.selectedItem as String
        s.formattingDocstringDecorationWidth = fmtDocstringDecorationWidth.value as Int

        // @generated:diag-apply:begin
        s.diagnosticE001 = diagE001.isSelected
        s.diagnosticE002 = diagE002.isSelected
        s.diagnosticE003 = diagE003.isSelected
        s.diagnosticE005 = diagE005.isSelected
        s.diagnosticE200 = diagE200.isSelected
        s.diagnosticW001 = diagW001.isSelected
        s.diagnosticW002 = diagW002.isSelected
        s.diagnosticW003 = diagW003.isSelected
        s.diagnosticW004 = diagW004.isSelected
        s.diagnosticW100 = diagW100.isSelected
        s.diagnosticW104 = diagW104.isSelected
        s.diagnosticW105 = diagW105.isSelected
        s.diagnosticW106 = diagW106.isSelected
        s.diagnosticW108 = diagW108.isSelected
        s.diagnosticW110 = diagW110.isSelected
        s.diagnosticW111 = diagW111.isSelected
        s.diagnosticW112 = diagW112.isSelected
        s.diagnosticW113 = diagW113.isSelected
        s.diagnosticW114 = diagW114.isSelected
        s.diagnosticW115 = diagW115.isSelected
        s.diagnosticW116 = diagW116.isSelected
        s.diagnosticW117 = diagW117.isSelected
        s.diagnosticW118 = diagW118.isSelected
        s.diagnosticW120 = diagW120.isSelected
        s.diagnosticW121 = diagW121.isSelected
        s.diagnosticW122 = diagW122.isSelected
        s.diagnosticW124 = diagW124.isSelected
        s.diagnosticW125 = diagW125.isSelected
        s.diagnosticW126 = diagW126.isSelected
        s.diagnosticW127 = diagW127.isSelected
        s.diagnosticW128 = diagW128.isSelected
        s.diagnosticW135 = diagW135.isSelected
        s.diagnosticW136 = diagW136.isSelected
        s.diagnosticW137 = diagW137.isSelected
        s.diagnosticW138 = diagW138.isSelected
        s.diagnosticW200 = diagW200.isSelected
        s.diagnosticW201 = diagW201.isSelected
        s.diagnosticW230 = diagW230.isSelected
        s.diagnosticW231 = diagW231.isSelected
        s.diagnosticW232 = diagW232.isSelected
        s.diagnosticW233 = diagW233.isSelected
        s.diagnosticW240 = diagW240.isSelected
        s.diagnosticW241 = diagW241.isSelected
        s.diagnosticW250 = diagW250.isSelected
        s.diagnosticW308 = diagW308.isSelected
        s.diagnosticW210 = diagW210.isSelected
        s.diagnosticW211 = diagW211.isSelected
        s.diagnosticW212 = diagW212.isSelected
        s.diagnosticW213 = diagW213.isSelected
        s.diagnosticW214 = diagW214.isSelected
        s.diagnosticW215 = diagW215.isSelected
        s.diagnosticW216 = diagW216.isSelected
        s.diagnosticW217 = diagW217.isSelected
        s.diagnosticW218 = diagW218.isSelected
        s.diagnosticW220 = diagW220.isSelected
        s.diagnosticW101 = diagW101.isSelected
        s.diagnosticW102 = diagW102.isSelected
        s.diagnosticW103 = diagW103.isSelected
        s.diagnosticW300 = diagW300.isSelected
        s.diagnosticW301 = diagW301.isSelected
        s.diagnosticW302 = diagW302.isSelected
        s.diagnosticW303 = diagW303.isSelected
        s.diagnosticW304 = diagW304.isSelected
        s.diagnosticW306 = diagW306.isSelected
        s.diagnosticW307 = diagW307.isSelected
        s.diagnosticW309 = diagW309.isSelected
        s.diagnosticW313 = diagW313.isSelected
        s.diagnosticH300 = diagH300.isSelected
        s.diagnosticI230 = diagI230.isSelected
        s.diagnosticI231 = diagI231.isSelected
        s.diagnosticW123 = diagW123.isSelected
        s.diagnosticW242 = diagW242.isSelected
        s.diagnosticS100 = diagS100.isSelected
        s.diagnosticS101 = diagS101.isSelected
        s.diagnosticS102 = diagS102.isSelected
        s.diagnosticS103 = diagS103.isSelected
        s.diagnosticS110 = diagS110.isSelected
        s.diagnosticT100 = diagT100.isSelected
        s.diagnosticT101 = diagT101.isSelected
        s.diagnosticT102 = diagT102.isSelected
        s.diagnosticT104 = diagT104.isSelected
        s.diagnosticT105 = diagT105.isSelected
        s.diagnosticIRULE1001 = diagIRULE1001.isSelected
        s.diagnosticIRULE1002 = diagIRULE1002.isSelected
        s.diagnosticIRULE1003 = diagIRULE1003.isSelected
        s.diagnosticIRULE1004 = diagIRULE1004.isSelected
        s.diagnosticIRULE1005 = diagIRULE1005.isSelected
        s.diagnosticIRULE1006 = diagIRULE1006.isSelected
        s.diagnosticIRULE1007 = diagIRULE1007.isSelected
        s.diagnosticIRULE1008 = diagIRULE1008.isSelected
        s.diagnosticIRULE1201 = diagIRULE1201.isSelected
        s.diagnosticIRULE1202 = diagIRULE1202.isSelected
        s.diagnosticIRULE2001 = diagIRULE2001.isSelected
        s.diagnosticIRULE2002 = diagIRULE2002.isSelected
        s.diagnosticIRULE2003 = diagIRULE2003.isSelected
        s.diagnosticIRULE2101 = diagIRULE2101.isSelected
        s.diagnosticIRULE5001 = diagIRULE5001.isSelected
        s.diagnosticIRULE5002 = diagIRULE5002.isSelected
        s.diagnosticIRULE5004 = diagIRULE5004.isSelected
        s.diagnosticIRULE5005 = diagIRULE5005.isSelected
        s.diagnosticIRULE5006 = diagIRULE5006.isSelected
        s.diagnosticIRULE5007 = diagIRULE5007.isSelected
        s.diagnosticIRULE3001 = diagIRULE3001.isSelected
        s.diagnosticIRULE3002 = diagIRULE3002.isSelected
        s.diagnosticIRULE3003 = diagIRULE3003.isSelected
        s.diagnosticIRULE3004 = diagIRULE3004.isSelected
        s.diagnosticIRULE3101 = diagIRULE3101.isSelected
        s.diagnosticIRULE3102 = diagIRULE3102.isSelected
        s.diagnosticIRULE4001 = diagIRULE4001.isSelected
        s.diagnosticIRULE4002 = diagIRULE4002.isSelected
        s.diagnosticIRULE4003 = diagIRULE4003.isSelected
        s.diagnosticIRULE4004 = diagIRULE4004.isSelected
        s.diagnosticIRULE4005 = diagIRULE4005.isSelected
        s.diagnosticW130 = diagW130.isSelected
        s.diagnosticW131 = diagW131.isSelected
        s.diagnosticW132 = diagW132.isSelected
        s.diagnosticW133 = diagW133.isSelected
        s.diagnosticW134 = diagW134.isSelected
        // @generated:diag-apply:end
        s.xcDiagnosticsEnabled = xcDiagnosticsEnabled.isSelected

        s.styleLineLength = styleLineLength.value as Int

        // @generated:opt-apply:begin
        s.optimiserEnabled = optEnabled.isSelected
        s.optimiserO100 = optO100.isSelected
        s.optimiserO101 = optO101.isSelected
        s.optimiserO102 = optO102.isSelected
        s.optimiserO103 = optO103.isSelected
        s.optimiserO104 = optO104.isSelected
        s.optimiserO105 = optO105.isSelected
        s.optimiserO106 = optO106.isSelected
        s.optimiserO107 = optO107.isSelected
        s.optimiserO108 = optO108.isSelected
        s.optimiserO109 = optO109.isSelected
        s.optimiserO110 = optO110.isSelected
        s.optimiserO111 = optO111.isSelected
        s.optimiserO112 = optO112.isSelected
        s.optimiserO113 = optO113.isSelected
        s.optimiserO114 = optO114.isSelected
        s.optimiserO115 = optO115.isSelected
        s.optimiserO116 = optO116.isSelected
        s.optimiserO117 = optO117.isSelected
        s.optimiserO118 = optO118.isSelected
        s.optimiserO119 = optO119.isSelected
        s.optimiserO120 = optO120.isSelected
        s.optimiserO121 = optO121.isSelected
        s.optimiserO122 = optO122.isSelected
        s.optimiserO123 = optO123.isSelected
        s.optimiserO124 = optO124.isSelected
        s.optimiserO125 = optO125.isSelected
        s.optimiserO126 = optO126.isSelected
        s.optimiserO127 = optO127.isSelected
        s.optimiserO128 = optO128.isSelected
        s.optimiserO129 = optO129.isSelected
        s.optimiserO130 = optO130.isSelected
        // @generated:opt-apply:end

        s.shimmerEnabled = shimmerEnabled.isSelected
        s.runtimeValidationEnabled = runtimeValidation.isSelected
        s.runtimeValidationAdapter = rtAdapter.selectedItem as String
        s.runtimeValidationTclshPath = rtTclshPath.text
        s.runtimeValidationTimeoutMs = rtTimeoutMs.value as Int
        s.aiEnabled = aiEnabled.isSelected
        s.aiExtraPrompts = aiExtraPrompts.text
        s.diagnosticsGenericVariablePatterns = genericPatternsField.text

        if (s.serverPath != oldServerPath) {
            restartLspServers()
        }
    }

    /**
     * Restart the Tcl LSP server in every open project. Called after
     * launch-affecting settings change (server path) so
     * the user picks up the new command line without restarting the
     * IDE. Non-launch settings (features, formatting, diagnostics, …)
     * are sent to the running server via workspace/configuration and
     * don't need a restart.
     */
    @Suppress("UnstableApiUsage")
    private fun restartLspServers() {
        for (project in ProjectManager.getInstance().openProjects) {
            if (project.isDisposed) continue
            try {
                LspServerManager.getInstance(project)
                    .stopAndRestartIfNeeded(TclLspServerSupportProvider::class.java)
            } catch (e: Exception) {
                LOG.warn("Failed to restart Tcl LSP server for project ${project.name}", e)
            }
        }
    }

    fun reset() {
        val s = TclLspSettings.getInstance()
        serverPathField.text = s.serverPath
        dialectCombo.selectedIndex = TclLspSettings.DIALECT_OPTIONS.indexOfFirst { it.first == s.dialect }.coerceAtLeast(0)
        extraCommandsField.text = s.extraCommands
        libraryPathsField.text = s.libraryPaths

        featureHover.isSelected = s.featureHover
        featureCompletion.isSelected = s.featureCompletion
        featureDiagnostics.isSelected = s.featureDiagnostics
        featureSemanticTokens.isSelected = s.featureSemanticTokens
        featureCodeActions.isSelected = s.featureCodeActions
        featureDefinition.isSelected = s.featureDefinition
        featureReferences.isSelected = s.featureReferences
        featureDocumentSymbols.isSelected = s.featureDocumentSymbols
        featureFolding.isSelected = s.featureFolding
        featureRename.isSelected = s.featureRename
        featureSignatureHelp.isSelected = s.featureSignatureHelp
        featureWorkspaceSymbols.isSelected = s.featureWorkspaceSymbols
        featureInlayTypeHints.isSelected = s.featureInlayTypeHints
        featureInlayParameterHints.isSelected = s.featureInlayParameterHints
        featureCallHierarchy.isSelected = s.featureCallHierarchy
        featureDocumentLinks.isSelected = s.featureDocumentLinks
        featureSelectionRange.isSelected = s.featureSelectionRange
        featureDocumentHighlight.isSelected = s.featureDocumentHighlight
        featureCodeLens.isSelected = s.featureCodeLens
        featureWorkspaceFileOps.isSelected = s.featureWorkspaceFileOps
        featurePullDiagnostics.isSelected = s.featurePullDiagnostics
        featureImplementation.isSelected = s.featureImplementation
        featureTypeDefinition.isSelected = s.featureTypeDefinition
        featureDeclaration.isSelected = s.featureDeclaration
        featureLinkedEditingRange.isSelected = s.featureLinkedEditingRange

        fmtIndentSize.value = s.formattingIndentSize
        fmtIndentStyle.selectedItem = s.formattingIndentStyle
        fmtContinuationIndent.value = s.formattingContinuationIndent
        fmtBraceStyle.selectedItem = s.formattingBraceStyle
        fmtSpaceBetweenBraces.isSelected = s.formattingSpaceBetweenBraces
        fmtEnforceBracedVars.isSelected = s.formattingEnforceBracedVariables
        fmtEnforceBracedExpr.isSelected = s.formattingEnforceBracedExpr
        fmtMaxLineLength.value = s.formattingMaxLineLength
        fmtGoalLineLength.value = s.formattingGoalLineLength
        fmtExpandSingleLine.isSelected = s.formattingExpandSingleLineBodies
        fmtMinBodyCmds.value = s.formattingMinBodyCommandsForExpansion
        fmtSpaceAfterHash.isSelected = s.formattingSpaceAfterCommentHash
        fmtTrimTrailing.isSelected = s.formattingTrimTrailingWhitespace
        fmtAlignComments.isSelected = s.formattingAlignCommentsToCode
        fmtReplaceSemicolons.isSelected = s.formattingReplaceSemicolonsWithNewlines
        fmtBlankProcs.value = s.formattingBlankLinesBetweenProcs
        fmtBlankBlocks.value = s.formattingBlankLinesBetweenBlocks
        fmtMaxBlankLines.value = s.formattingMaxConsecutiveBlankLines
        fmtLineEnding.selectedItem = s.formattingLineEnding
        fmtFinalNewline.isSelected = s.formattingEnsureFinalNewline
        fmtDocstringStyle.selectedItem = s.formattingDocstringStyle
        fmtDocstringTagStyle.selectedItem = s.formattingDocstringTagStyle
        fmtDocstringDecoration.isSelected = s.formattingDocstringDecoration
        fmtDocstringDecorationChar.selectedItem = s.formattingDocstringDecorationChar
        fmtDocstringDecorationWidth.value = s.formattingDocstringDecorationWidth

        // @generated:diag-reset:begin
        diagE001.isSelected = s.diagnosticE001
        diagE002.isSelected = s.diagnosticE002
        diagE003.isSelected = s.diagnosticE003
        diagE005.isSelected = s.diagnosticE005
        diagE200.isSelected = s.diagnosticE200
        diagW001.isSelected = s.diagnosticW001
        diagW002.isSelected = s.diagnosticW002
        diagW003.isSelected = s.diagnosticW003
        diagW004.isSelected = s.diagnosticW004
        diagW100.isSelected = s.diagnosticW100
        diagW104.isSelected = s.diagnosticW104
        diagW105.isSelected = s.diagnosticW105
        diagW106.isSelected = s.diagnosticW106
        diagW108.isSelected = s.diagnosticW108
        diagW110.isSelected = s.diagnosticW110
        diagW111.isSelected = s.diagnosticW111
        diagW112.isSelected = s.diagnosticW112
        diagW113.isSelected = s.diagnosticW113
        diagW114.isSelected = s.diagnosticW114
        diagW115.isSelected = s.diagnosticW115
        diagW116.isSelected = s.diagnosticW116
        diagW117.isSelected = s.diagnosticW117
        diagW118.isSelected = s.diagnosticW118
        diagW120.isSelected = s.diagnosticW120
        diagW121.isSelected = s.diagnosticW121
        diagW122.isSelected = s.diagnosticW122
        diagW124.isSelected = s.diagnosticW124
        diagW125.isSelected = s.diagnosticW125
        diagW126.isSelected = s.diagnosticW126
        diagW127.isSelected = s.diagnosticW127
        diagW128.isSelected = s.diagnosticW128
        diagW135.isSelected = s.diagnosticW135
        diagW136.isSelected = s.diagnosticW136
        diagW137.isSelected = s.diagnosticW137
        diagW138.isSelected = s.diagnosticW138
        diagW200.isSelected = s.diagnosticW200
        diagW201.isSelected = s.diagnosticW201
        diagW230.isSelected = s.diagnosticW230
        diagW231.isSelected = s.diagnosticW231
        diagW232.isSelected = s.diagnosticW232
        diagW233.isSelected = s.diagnosticW233
        diagW240.isSelected = s.diagnosticW240
        diagW241.isSelected = s.diagnosticW241
        diagW250.isSelected = s.diagnosticW250
        diagW308.isSelected = s.diagnosticW308
        diagW210.isSelected = s.diagnosticW210
        diagW211.isSelected = s.diagnosticW211
        diagW212.isSelected = s.diagnosticW212
        diagW213.isSelected = s.diagnosticW213
        diagW214.isSelected = s.diagnosticW214
        diagW215.isSelected = s.diagnosticW215
        diagW216.isSelected = s.diagnosticW216
        diagW217.isSelected = s.diagnosticW217
        diagW218.isSelected = s.diagnosticW218
        diagW220.isSelected = s.diagnosticW220
        diagW101.isSelected = s.diagnosticW101
        diagW102.isSelected = s.diagnosticW102
        diagW103.isSelected = s.diagnosticW103
        diagW300.isSelected = s.diagnosticW300
        diagW301.isSelected = s.diagnosticW301
        diagW302.isSelected = s.diagnosticW302
        diagW303.isSelected = s.diagnosticW303
        diagW304.isSelected = s.diagnosticW304
        diagW306.isSelected = s.diagnosticW306
        diagW307.isSelected = s.diagnosticW307
        diagW309.isSelected = s.diagnosticW309
        diagW313.isSelected = s.diagnosticW313
        diagH300.isSelected = s.diagnosticH300
        diagI230.isSelected = s.diagnosticI230
        diagI231.isSelected = s.diagnosticI231
        diagW123.isSelected = s.diagnosticW123
        diagW242.isSelected = s.diagnosticW242
        diagS100.isSelected = s.diagnosticS100
        diagS101.isSelected = s.diagnosticS101
        diagS102.isSelected = s.diagnosticS102
        diagS103.isSelected = s.diagnosticS103
        diagS110.isSelected = s.diagnosticS110
        diagT100.isSelected = s.diagnosticT100
        diagT101.isSelected = s.diagnosticT101
        diagT102.isSelected = s.diagnosticT102
        diagT104.isSelected = s.diagnosticT104
        diagT105.isSelected = s.diagnosticT105
        diagIRULE1001.isSelected = s.diagnosticIRULE1001
        diagIRULE1002.isSelected = s.diagnosticIRULE1002
        diagIRULE1003.isSelected = s.diagnosticIRULE1003
        diagIRULE1004.isSelected = s.diagnosticIRULE1004
        diagIRULE1005.isSelected = s.diagnosticIRULE1005
        diagIRULE1006.isSelected = s.diagnosticIRULE1006
        diagIRULE1007.isSelected = s.diagnosticIRULE1007
        diagIRULE1008.isSelected = s.diagnosticIRULE1008
        diagIRULE1201.isSelected = s.diagnosticIRULE1201
        diagIRULE1202.isSelected = s.diagnosticIRULE1202
        diagIRULE2001.isSelected = s.diagnosticIRULE2001
        diagIRULE2002.isSelected = s.diagnosticIRULE2002
        diagIRULE2003.isSelected = s.diagnosticIRULE2003
        diagIRULE2101.isSelected = s.diagnosticIRULE2101
        diagIRULE5001.isSelected = s.diagnosticIRULE5001
        diagIRULE5002.isSelected = s.diagnosticIRULE5002
        diagIRULE5004.isSelected = s.diagnosticIRULE5004
        diagIRULE5005.isSelected = s.diagnosticIRULE5005
        diagIRULE5006.isSelected = s.diagnosticIRULE5006
        diagIRULE5007.isSelected = s.diagnosticIRULE5007
        diagIRULE3001.isSelected = s.diagnosticIRULE3001
        diagIRULE3002.isSelected = s.diagnosticIRULE3002
        diagIRULE3003.isSelected = s.diagnosticIRULE3003
        diagIRULE3004.isSelected = s.diagnosticIRULE3004
        diagIRULE3101.isSelected = s.diagnosticIRULE3101
        diagIRULE3102.isSelected = s.diagnosticIRULE3102
        diagIRULE4001.isSelected = s.diagnosticIRULE4001
        diagIRULE4002.isSelected = s.diagnosticIRULE4002
        diagIRULE4003.isSelected = s.diagnosticIRULE4003
        diagIRULE4004.isSelected = s.diagnosticIRULE4004
        diagIRULE4005.isSelected = s.diagnosticIRULE4005
        diagW130.isSelected = s.diagnosticW130
        diagW131.isSelected = s.diagnosticW131
        diagW132.isSelected = s.diagnosticW132
        diagW133.isSelected = s.diagnosticW133
        diagW134.isSelected = s.diagnosticW134
        // @generated:diag-reset:end
        xcDiagnosticsEnabled.isSelected = s.xcDiagnosticsEnabled

        styleLineLength.value = s.styleLineLength

        // @generated:opt-reset:begin
        optEnabled.isSelected = s.optimiserEnabled
        optO100.isSelected = s.optimiserO100
        optO101.isSelected = s.optimiserO101
        optO102.isSelected = s.optimiserO102
        optO103.isSelected = s.optimiserO103
        optO104.isSelected = s.optimiserO104
        optO105.isSelected = s.optimiserO105
        optO106.isSelected = s.optimiserO106
        optO107.isSelected = s.optimiserO107
        optO108.isSelected = s.optimiserO108
        optO109.isSelected = s.optimiserO109
        optO110.isSelected = s.optimiserO110
        optO111.isSelected = s.optimiserO111
        optO112.isSelected = s.optimiserO112
        optO113.isSelected = s.optimiserO113
        optO114.isSelected = s.optimiserO114
        optO115.isSelected = s.optimiserO115
        optO116.isSelected = s.optimiserO116
        optO117.isSelected = s.optimiserO117
        optO118.isSelected = s.optimiserO118
        optO119.isSelected = s.optimiserO119
        optO120.isSelected = s.optimiserO120
        optO121.isSelected = s.optimiserO121
        optO122.isSelected = s.optimiserO122
        optO123.isSelected = s.optimiserO123
        optO124.isSelected = s.optimiserO124
        optO125.isSelected = s.optimiserO125
        optO126.isSelected = s.optimiserO126
        optO127.isSelected = s.optimiserO127
        optO128.isSelected = s.optimiserO128
        optO129.isSelected = s.optimiserO129
        optO130.isSelected = s.optimiserO130
        // @generated:opt-reset:end

        shimmerEnabled.isSelected = s.shimmerEnabled
        runtimeValidation.isSelected = s.runtimeValidationEnabled
        rtAdapter.selectedItem = s.runtimeValidationAdapter
        rtTclshPath.text = s.runtimeValidationTclshPath
        rtTimeoutMs.value = s.runtimeValidationTimeoutMs
        aiEnabled.isSelected = s.aiEnabled
        aiExtraPrompts.text = s.aiExtraPrompts
        genericPatternsField.text = s.diagnosticsGenericVariablePatterns
    }
}
