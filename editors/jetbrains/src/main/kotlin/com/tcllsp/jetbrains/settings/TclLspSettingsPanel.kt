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
import com.intellij.openapi.util.text.StringUtil
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.ui.TitledSeparator
import com.intellij.ui.components.ActionLink
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import com.intellij.util.ui.JBUI
import com.intellij.util.ui.ThreeStateCheckBox
import com.intellij.util.ui.UIUtil
import com.tcllsp.jetbrains.TclLspServerSupportProvider
import java.awt.BorderLayout
import java.awt.Dimension
import java.awt.FlowLayout
import java.awt.Rectangle
import javax.swing.*

private val LOG = Logger.getInstance("com.tcllsp.jetbrains.settings.TclLspSettingsPanel")

/**
 * Width a settings hint wraps at, before HiDPI scaling.
 *
 * A hint sits in the form's right-hand column, so this plus the widest label
 * is most of what the page can never be narrower than. Keep it near a normal
 * reading measure: wider makes the whole settings pane refuse to shrink.
 */
private const val COMMENT_WIDTH = 440

/** Mouse-wheel step for the settings scroll pane, before HiDPI scaling. */
private const val SCROLL_UNIT = 16

/**
 * A wrapping hint under a setting.
 *
 * `FormBuilder.addTooltip` builds a plain `JBLabel` straight from the string,
 * and a `JLabel` never wraps: the longest hint on this page is 180 characters,
 * so on a single line it alone asked the settings pane for about 1200px and
 * ran off the right-hand edge.
 */
/**
 * The stored value a tri-state optimiser box currently represents.
 *
 * `null` is the third state, and it is the important one: it means "inherit
 * from the profile". The server treats a per-code override as beating the
 * profile in both directions — `true` lifts a code out of the profile's
 * disabled set, `false` forces it in — so only a real user choice may travel,
 * and `DONT_CARE` must serialise to no opinion at all rather than to `false`.
 */
private fun triState(box: ThreeStateCheckBox): Boolean? = when (box.state) {
    ThreeStateCheckBox.State.SELECTED -> true
    ThreeStateCheckBox.State.NOT_SELECTED -> false
    else -> null
}

/** The inverse of [triState], for loading a stored value back into the box. */
private fun threeState(value: Boolean?): ThreeStateCheckBox.State = when (value) {
    true -> ThreeStateCheckBox.State.SELECTED
    false -> ThreeStateCheckBox.State.NOT_SELECTED
    null -> ThreeStateCheckBox.State.DONT_CARE
}

private fun FormBuilder.addWrappedComment(text: String): FormBuilder =
    addComponentToRightColumn(
        JBLabel(
            "<html><body style='width:${JBUI.scale(COMMENT_WIDTH)}px'>" +
                StringUtil.escapeXmlEntities(text) + "</body></html>",
            UIUtil.ComponentStyle.SMALL,
            UIUtil.FontColor.BRIGHTER,
        ).apply { border = JBUI.Borders.emptyLeft(10) },
        1,
    )

/**
 * The component the settings dialog is handed, holding the form at the width
 * of the scroll pane the platform puts around it.
 *
 * `ConfigurableCardPanel` wraps whatever `createComponent` returns in its own
 * `JScrollPane` — unconditionally, unless the configurable implements
 * `Configurable.NoScroll` — so returning a scroll pane of our own nested one
 * inside the other. The inner pane was always exactly as large as its content,
 * so it never scrolled, and it swallowed the wheel events that would otherwise
 * have reached the outer one: the page could not be scrolled at all.
 *
 * Implementing [Scrollable] instead makes the platform's viewport size the
 * form to its own width, so the content reflows rather than overflowing to the
 * right, and leaves horizontal scrolling as the fallback for a pane narrower
 * than the form can honestly be squeezed to.
 */
internal class ScrollableForm(form: JComponent) : JPanel(BorderLayout()), Scrollable {
    init {
        add(form, BorderLayout.CENTER)
        isOpaque = false
    }

    override fun getPreferredScrollableViewportSize(): Dimension = preferredSize

    override fun getScrollableUnitIncrement(visible: Rectangle, orientation: Int, direction: Int): Int =
        JBUI.scale(SCROLL_UNIT)

    override fun getScrollableBlockIncrement(visible: Rectangle, orientation: Int, direction: Int): Int =
        if (orientation == SwingConstants.VERTICAL) visible.height else visible.width

    override fun getScrollableTracksViewportWidth(): Boolean {
        val viewport = parent as? JViewport ?: return false
        return viewport.width >= minimumSize.width
    }

    override fun getScrollableTracksViewportHeight(): Boolean = false
}

class TclLspSettingsPanel {

    // General
    private val serverPathField = JBTextField(30)
    private val dialectCombo = JComboBox(
        TclLspSettings.DIALECT_OPTIONS.map { it.second }.toTypedArray()
    )
    private val extraCommandsField = JBTextField(30)
    private val libraryPathsField = JBTextField(30)
    private val signatureHelpDisabledCommandsField = JBTextField(30)
    private val signatureHelpInheritDisabledCommands = JBCheckBox("Inherit from config.ini")

    // Feature toggles
    // Two annotations here, both derived from the published `lsp` platform
    // module rather than guessed. A dagger marks a server feature no IntelliJ
    // advertises at all: scanning every class in the module for
    // `TextDocumentClientCapabilities.set*` / `WorkspaceClientCapabilities.set*`
    // across 253, 261, 262 and 263 turns up no implementation, declaration,
    // linkedEditingRange or workspace fileOperations at any of them. A trailing
    // version marks one the IDE does ask for, but only from that build on —
    // codeLens lands in 261 and rename in 262, both above our 253 floor.
    // Note the capability set is assembled by the LSP module in the *user's*
    // IDE at runtime, so these track the IDE the user is running, not the SDK
    // this plugin compiles against.
    private val featureHover = JBCheckBox("Hover")
    private val featureCompletion = JBCheckBox("Completion")
    private val featureDiagnostics = JBCheckBox("Diagnostics")
    private val featureSemanticTokens = JBCheckBox("Semantic tokens")
    private val featureCodeActions = JBCheckBox("Code actions")
    private val featureDefinition = JBCheckBox("Go to definition")
    private val featureReferences = JBCheckBox("Find references")
    private val featureDocumentSymbols = JBCheckBox("Document symbols")
    private val featureFolding = JBCheckBox("Code folding")
    private val featureRename = JBCheckBox("Rename symbol (2026.2+)")
    private val featureSignatureHelp = JBCheckBox("Signature help")
    private val featureWorkspaceSymbols = JBCheckBox("Workspace symbols")
    private val featureInlayTypeHints = JBCheckBox("Inlay type hints")
    private val featureInlayParameterHints = JBCheckBox("Inlay parameter-name hints")
    private val featureCallHierarchy = JBCheckBox("Call hierarchy")
    private val featureDocumentLinks = JBCheckBox("Document links")
    private val featureSelectionRange = JBCheckBox("Selection range")
    private val featureDocumentHighlight = JBCheckBox("Document highlight")
    private val featureCodeLens = JBCheckBox("Code lens (2026.1+)")
    private val featureWorkspaceFileOps = JBCheckBox("Auto-rewrite source paths on rename \u2020")
    private val featureImplementation = JBCheckBox("Go to implementation \u2020")
    private val featureTypeDefinition = JBCheckBox("Go to type definition")
    private val featureDeclaration = JBCheckBox("Go to declaration \u2020")
    private val featureLinkedEditingRange = JBCheckBox("Linked editing range \u2020")

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
    private val fmtLineEnding = JComboBox(arrayOf("auto", "lf", "crlf", "cr"))
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
    private val diagE006 = JBCheckBox("E006: Invalid literal formal-parameter list")
    private val diagE200 = JBCheckBox("E200: Unterminated command")

    // Diagnostics — Style & Best Practice
    private val diagW001 = JBCheckBox("W001: Unknown subcommand")
    private val diagW002 = JBCheckBox("W002: Command is disabled in active dialect profile")
    private val diagW003 = JBCheckBox("W003: Expression operator not available in active dialect")
    private val diagW004 = JBCheckBox("W004: Command option is not available in the active dialect")
    private val diagW100 = JBCheckBox("W100: Unbraced expression argument")
    private val diagW104 = JBCheckBox("W104: String concatenation for list building")
    private val diagW105 = JBCheckBox("W105: Unbraced code block argument. Escalates to Error whe...")
    private val diagW106 = JBCheckBox("W106: Dangerous unbraced switch body")
    private val diagW107 = JBCheckBox("W107: Source is not valid UTF-8")
    private val diagW108 = JBCheckBox("W108: Non-ASCII characters in token content")
    private val diagW109 = JBCheckBox("W109: Source does not look like UTF-8 text")
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
    private val diagW124 = JBCheckBox("W124: Invalid IP address literal")
    private val diagW125 = JBCheckBox("W125: Orphaned control-flow keyword used as standalone com...")
    private val diagW126 = JBCheckBox("W126: Non-channel value in channel argument position")
    private val diagW127 = JBCheckBox("W127: Value not in the command's allowed set")
    private val diagW128 = JBCheckBox("W128: Command called after it was renamed or deleted earli...")
    private val diagW129 = JBCheckBox("W129: Command is hidden in a safe interpreter")
    private val diagW135 = JBCheckBox("W135: Command requires a newer package version than the re...")
    private val diagW136 = JBCheckBox("W136: Option requires a newer package version than the res...")
    private val diagW137 = JBCheckBox("W137: Argument value requires a newer Tcl version than the...")
    private val diagW138 = JBCheckBox("W138: Format/scan conversion requires a newer Tcl version ...")
    private val diagW139 = JBCheckBox("W139: Command/option retired at the resolved package version")
    private val diagW140 = JBCheckBox("W140: interp eval / interp subcommand targets an interpret...")
    private val diagW141 = JBCheckBox("W141: Option value fails a declared shape/content check (e...")
    private val diagW142 = JBCheckBox("W142: Command invalid in its current lexical/dispatch cont...")
    private val diagW143 = JBCheckBox("W143: Direct call into a private ::tcl:: implementation na...")
    private val diagW144 = JBCheckBox("W144: Command/subcommand/option/argument value is deprecat...")
    private val diagW145 = JBCheckBox("W145: Ambiguous keyword abbreviation")
    private val diagW146 = JBCheckBox("W146: Literal argument violates a registry-declared relati...")
    private val diagW147 = JBCheckBox("W147: Mutually exclusive command options were supplied tog...")
    private val diagW148 = JBCheckBox("W148: Numeral spelling is not accepted by the document's r...")
    private val diagW149 = JBCheckBox("W149: Argument count matches a different release of the co...")
    private val diagW150 = JBCheckBox("W150: Not available across the project's declared version-...")
    private val diagW151 = JBCheckBox("W151: Numeral changes meaning or validity across the proje...")
    private val diagW152 = JBCheckBox("W152: A registry-declared option relation is unmet")
    private val diagW200 = JBCheckBox("W200: Unsigned (u) modifier on a binary format/binary scan...")
    private val diagW201 = JBCheckBox("W201: Manual path concatenation")
    private val diagW230 = JBCheckBox("W230: Constant list index out of range")
    private val diagW231 = JBCheckBox("W231: Constant list index out of range")
    private val diagW232 = JBCheckBox("W232: Constant string index out of range")
    private val diagW233 = JBCheckBox("W233: Division or modulo by a provably-zero divisor")
    private val diagW240 = JBCheckBox("W240: Loop condition is a constant false")
    private val diagW241 = JBCheckBox("W241: Loop is provably infinite")
    private val diagW250 = JBCheckBox("W250: Instantiating an oo::abstract class")
    private val diagW308 = JBCheckBox("W308: Unknown TclOO method")
    private val diagW314 = JBCheckBox("W314: Definition has no absolute (fully-qualified) name")
    private val diagW315 = JBCheckBox("W315: Class or object definition cannot run")

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
    private val diagW305 = JBCheckBox("W305: Bidirectional formatting control character in source...")
    private val diagW306 = JBCheckBox("W306: Substitution in literal-expected argument position")
    private val diagW307 = JBCheckBox("W307: Non-literal command name")
    private val diagW309 = JBCheckBox("W309: eval/uplevel with subst")
    private val diagW313 = JBCheckBox("W313: Destructive file operation with variable path")

    // Diagnostics — Hints
    private val diagH300 = JBCheckBox("H300: Possible paste error")
    private val diagH301 = JBCheckBox("H301: Command used above the package require that provides it")
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
    private val diagIRULE1004 = JBCheckBox("IRULE1004: Explicit event priority required by the registry policy")
    private val diagIRULE1005 = JBCheckBox("IRULE1005: Data event without its required registered collectio...")
    private val diagIRULE1006 = JBCheckBox("IRULE1006: Payload access without its required registered colle...")
    private val diagIRULE1007 = JBCheckBox("IRULE1007: Collection without its required registered release o...")
    private val diagIRULE1008 = JBCheckBox("IRULE1008: *::release without a matching *::collect on the same...")
    private val diagIRULE1201 = JBCheckBox("IRULE1201: HTTP command used after HTTP::respond/HTTP::redirect")
    private val diagIRULE1202 = JBCheckBox("IRULE1202: Multiple HTTP::respond/HTTP::redirect on different b...")
    private val diagIRULE2001 = JBCheckBox("IRULE2001: Deprecated matchclass")
    private val diagIRULE2002 = JBCheckBox("IRULE2002: Deprecated iRules command")
    private val diagIRULE2003 = JBCheckBox("IRULE2003: Unsafe iRules command")
    private val diagIRULE2004 = JBCheckBox("IRULE2004: Command refused by the iRules rule compiler at load")
    private val diagIRULE2101 = JBCheckBox("IRULE2101: Heavy regexp in a high-frequency event")
    private val diagIRULE5001 = JBCheckBox("IRULE5001: Ungated log in a high-frequency event")
    private val diagIRULE5002 = JBCheckBox("IRULE5002: drop/reject/discard without event disable all or return")
    private val diagIRULE5004 = JBCheckBox("IRULE5004: DNS::return without return")
    private val diagIRULE5005 = JBCheckBox("IRULE5005: Direct proc invocation without call")
    private val diagIRULE5006 = JBCheckBox("IRULE5006: Top-level-only command used inside a nested body")
    private val diagIRULE5007 = JBCheckBox("IRULE5007: Executable command used on iRules' declaration-only ...")
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

    // Diagnostics — BIG-IP Configuration
    private val diagBIGIP6001 = JBCheckBox("BIGIP6001: iRule references a data group not found in the confi...")
    private val diagBIGIP6002 = JBCheckBox("BIGIP6002: iRule references a pool not found in the configuration")
    private val diagBIGIP6003 = JBCheckBox("BIGIP6003: Virtual server references an iRule that is not defin...")
    private val diagBIGIP6004 = JBCheckBox("BIGIP6004: An attached iRule uses HTTP:: or SSL:: commands with...")
    private val diagBIGIP6005 = JBCheckBox("BIGIP6005: Virtual server references a pool that is not defined...")
    private val diagBIGIP6006 = JBCheckBox("BIGIP6006: Data group is defined but not referenced by any iRul...")
    private val diagBIGIP6007 = JBCheckBox("BIGIP6007: iRule references an SNAT pool not found in the confi...")
    private val diagBIGIP6008 = JBCheckBox("BIGIP6008: Pool has no members defined")
    private val diagBIGIP6009 = JBCheckBox("BIGIP6009: Virtual server has a duplicate iRule attachment")
    private val diagBIGIP6010 = JBCheckBox("BIGIP6010: An attached iRule uses a persistence profile that is...")
    private val diagBIGIP6011 = JBCheckBox("BIGIP6011: IP-type data group contains an invalid IP address or...")
    private val diagBIGIP6012 = JBCheckBox("BIGIP6012: Attached iRules handle the same event at the same ef...")
    private val diagBIGIP6013 = JBCheckBox("BIGIP6013: A registry-declared BIG-IP object reference could no...")
    private val diagBIGIP6014 = JBCheckBox("BIGIP6014: A BIG-IP object declaration duplicates another objec...")
    private val diagBIGIP6038 = JBCheckBox("BIGIP6038: An iRule event requires a profile that is not active...")
    private val diagBIGIP6039 = JBCheckBox("BIGIP6039: A virtual server attaches incompatible profile types")
    private val diagIAPP7001 = JBCheckBox("IAPP7001: iApp implementation references a presentation field ...")
    private val diagIAPP7002 = JBCheckBox("IAPP7002: iApp presentation field is never referenced by the i...")
    private val diagIAPP7003 = JBCheckBox("IAPP7003: iApp presentation #include file could not be resolved")

    // Diagnostics — SslicTcl
    private val diagSSLIC1001 = JBCheckBox("SSLIC1001: SslicTcl declaration is not valid Tcl syntax or has ...")
    private val diagSSLIC1002 = JBCheckBox("SSLIC1002: SslicTcl declaration uses substitution or argument e...")
    private val diagSSLIC1003 = JBCheckBox("SSLIC1003: SslicTcl document is missing its sslictcl VERSION he...")
    private val diagSSLIC1004 = JBCheckBox("SSLIC1004: SslicTcl document declares its sslictcl header more ...")
    private val diagSSLIC1005 = JBCheckBox("SSLIC1005: SslicTcl declaration has the wrong number of words")
    private val diagSSLIC1006 = JBCheckBox("SSLIC1006: SslicTcl declaration body must be a braced literal")
    private val diagSSLIC1007 = JBCheckBox("SSLIC1007: SslicTcl closed block contains an unknown member")
    private val diagSSLIC1008 = JBCheckBox("SSLIC1008: SslicTcl declaration duplicates an earlier declarati...")
    private val diagSSLIC1009 = JBCheckBox("SSLIC1009: SslicTcl value is outside its declared domain")
    private val diagSSLIC1010 = JBCheckBox("SSLIC1010: SslicTcl declaration is missing a required member")
    private val diagSSLIC1011 = JBCheckBox("SSLIC1011: SslicTcl declaration references a name that is not d...")
    private val diagSSLIC1012 = JBCheckBox("SSLIC1012: SslicTcl declaration combines mutually exclusive mem...")
    private val diagSSLIC1101 = JBCheckBox("SSLIC1101: SslicTcl unknown declaration preserved as an extension")
    private val diagSSLIC1102 = JBCheckBox("SSLIC1102: SslicTcl document vocabulary is newer than this buil...")
    private val diagSSLIC1103 = JBCheckBox("SSLIC1103: SslicTcl predicate body is retained but never evalua...")
    // @generated:diag-checkboxes:end

    // XC Diagnostics
    private val xcDiagnosticsEnabled = JBCheckBox("Enable XC translatability diagnostics")

    // Style
    private val styleLineLength = JSpinner(SpinnerNumberModel(120, 40, 500, 10))

    // @generated:opt-checkboxes:begin
    private val optEnabled = JBCheckBox("Enable optimiser suggestions")
    private val optProfile = JComboBox(arrayOf("off", "readability", "standard", "full", "aggressive"))
    private val optO100 = ThreeStateCheckBox("O100: Propagate constant variables into expressions and co...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO101 = ThreeStateCheckBox("O101: Fold constant integer expressions", ThreeStateCheckBox.State.DONT_CARE)
    private val optO102 = ThreeStateCheckBox("O102: Forward a variable's single reaching literal load to...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO103 = ThreeStateCheckBox("O103: Fold static procedure calls using interprocedural su...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO104 = ThreeStateCheckBox("O104: Fold static string build chains into a single assign...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO105 = ThreeStateCheckBox("O105: Propagate constants into variable references and det...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO106 = ThreeStateCheckBox("O106: Hoist loop-invariant computations", ThreeStateCheckBox.State.DONT_CARE)
    private val optO107 = ThreeStateCheckBox("O107: Eliminate unreachable dead code", ThreeStateCheckBox.State.DONT_CARE)
    private val optO108 = ThreeStateCheckBox("O108: Eliminate transitively dead code", ThreeStateCheckBox.State.DONT_CARE)
    private val optO109 = ThreeStateCheckBox("O109: Eliminate dead stores", ThreeStateCheckBox.State.DONT_CARE)
    private val optO110 = ThreeStateCheckBox("O110: Canonicalise expressions (InstCombine)", ThreeStateCheckBox.State.DONT_CARE)
    private val optO111 = ThreeStateCheckBox("O111: Brace expression performance hints (paired with W100)", ThreeStateCheckBox.State.DONT_CARE)
    private val optO112 = ThreeStateCheckBox("O112: Eliminate constant-condition compound statements", ThreeStateCheckBox.State.DONT_CARE)
    private val optO113 = ThreeStateCheckBox("O113: Strength-reduce expressions (x**2 → x*x, x%8 → x&7)", ThreeStateCheckBox.State.DONT_CARE)
    private val optO114 = ThreeStateCheckBox("O114: Recognise incr idiom (set x [expr {\$x + N}] → incr x N)", ThreeStateCheckBox.State.DONT_CARE)
    private val optO115 = ThreeStateCheckBox("O115: Remove redundant nested [expr {...}] in expression c...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO116 = ThreeStateCheckBox("O116: Fold constant [list a b c] to literal value", ThreeStateCheckBox.State.DONT_CARE)
    private val optO117 = ThreeStateCheckBox("O117: Simplify [string length \$s] == 0 → \$s eq \"\"", ThreeStateCheckBox.State.DONT_CARE)
    private val optO118 = ThreeStateCheckBox("O118: Fold constant [lindex {a b c} 1] to element", ThreeStateCheckBox.State.DONT_CARE)
    private val optO119 = ThreeStateCheckBox("O119: Pack consecutive set literals into lassign/foreach", ThreeStateCheckBox.State.DONT_CARE)
    private val optO120 = ThreeStateCheckBox("O120: Prefer eq/ne over ==/!= for string comparisons", ThreeStateCheckBox.State.DONT_CARE)
    private val optO121 = ThreeStateCheckBox("O121: Rewrite self-recursive tail calls to tailcall", ThreeStateCheckBox.State.DONT_CARE)
    private val optO122 = ThreeStateCheckBox("O122: Convert fully tail-recursive proc to iterative while...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO123 = ThreeStateCheckBox("O123: Detect non-tail recursion eligible for accumulator i...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO124 = ThreeStateCheckBox("O124: Comment out unused procs in iRules (not called from ...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO125 = ThreeStateCheckBox("O125: Sink side-effect-free assignments into the deepest d...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO126 = ThreeStateCheckBox("O126: Remove unused variable assignments", ThreeStateCheckBox.State.DONT_CARE)
    private val optO127 = ThreeStateCheckBox("O127: Inline single-use variable assignment", ThreeStateCheckBox.State.DONT_CARE)
    private val optO128 = ThreeStateCheckBox("O128: Rewrite [expr {[llength \$L] - N}] / [expr {[string l...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO129 = ThreeStateCheckBox("O129: Fold a pure builtin command substitution with consta...", ThreeStateCheckBox.State.DONT_CARE)
    private val optO130 = ThreeStateCheckBox("O130: Fold static lappend list build chains into a single ...", ThreeStateCheckBox.State.DONT_CARE)
    private val optCodeBoxes: List<ThreeStateCheckBox> = listOf(
        optO100, optO101, optO102, optO103, optO104, optO105,
        optO106, optO107, optO108, optO109, optO110, optO111,
        optO112, optO113, optO114, optO115, optO116, optO117,
        optO118, optO119, optO120, optO121, optO122, optO123,
        optO124, optO125, optO126, optO127, optO128, optO129,
        optO130,
    )
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
    private val diagnosticsExcludeField = JBTextField(30)

    val root: JComponent

    init {
        val builder = FormBuilder.createFormBuilder()

        // General section
        builder.addComponent(TitledSeparator("General"))
        builder.addLabeledComponent(JBLabel("Server path:"), serverPathField)
        builder.addWrappedComment("Path to a tcl-lsp checkout root (probes target/{release,debug}/tcl-lsp-server) or directly to a built native binary (dev mode). Leave empty to use the bundled server.")
        builder.addLabeledComponent(JBLabel("Dialect:"), dialectCombo)
        builder.addLabeledComponent(JBLabel("Extra commands:"), extraCommandsField)
        builder.addWrappedComment("Comma-separated list of additional command names to treat as known.")
        builder.addLabeledComponent(JBLabel("Library paths:"), libraryPathsField)
        builder.addWrappedComment("Comma-separated directories to scan for Tcl packages.")
        builder.addLabeledComponent(
            JBLabel("Signature help disabled commands:"),
            signatureHelpDisabledCommandsField,
        )
        builder.addWrappedComment("Comma-separated built-in command names to silence, such as set,incr. Leave Inherit unchecked and this list empty to show every signature. User-defined proc signatures remain enabled.")
        builder.addComponent(signatureHelpInheritDisabledCommands)
        signatureHelpInheritDisabledCommands.toolTipText =
            "Use the disabled-command list from config.ini instead of overriding it in the IDE."
        signatureHelpInheritDisabledCommands.addActionListener {
            signatureHelpDisabledCommandsField.isEnabled =
                !signatureHelpInheritDisabledCommands.isSelected
        }

        // Features section
        builder.addComponent(TitledSeparator("Features"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    featureHover, featureCompletion, featureDiagnostics,
                    featureSemanticTokens, featureCodeActions, featureDefinition, featureReferences,
                    featureDocumentSymbols, featureFolding, featureRename, featureSignatureHelp,
                    featureWorkspaceSymbols, featureInlayTypeHints, featureInlayParameterHints,
                    featureCallHierarchy,
                    featureDocumentLinks, featureSelectionRange,
                    featureDocumentHighlight, featureCodeLens, featureWorkspaceFileOps,
                    featureImplementation, featureTypeDefinition, featureDeclaration,
                    featureLinkedEditingRange,
                ),
            ),
        )
        builder.addWrappedComment(
            "\u2020 Sent to the server, but never requested by any JetBrains IDE: IntelliJ's " +
                "LSP client does not implement these, so the toggle has no effect here. " +
                "A version in brackets is the IDE release that starts asking for that feature.",
        )

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
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagE001, diagE002, diagE003, diagE005, diagE006, diagE200,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Style & Best Practice"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagW001, diagW002, diagW003, diagW004, diagW100, diagW104,
                    diagW105, diagW106, diagW107, diagW108, diagW109, diagW110,
                    diagW111, diagW112, diagW113, diagW114, diagW115, diagW116,
                    diagW117, diagW118, diagW120, diagW121, diagW124, diagW125,
                    diagW126, diagW127, diagW128, diagW129, diagW135, diagW136,
                    diagW137, diagW138, diagW139, diagW140, diagW141, diagW142,
                    diagW143, diagW144, diagW145, diagW146, diagW147, diagW148,
                    diagW149, diagW150, diagW151, diagW152, diagW200, diagW201,
                    diagW230, diagW231, diagW232, diagW233, diagW240, diagW241,
                    diagW250, diagW308, diagW314, diagW315,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Variables"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagW210, diagW211, diagW212, diagW213, diagW214, diagW215,
                    diagW216, diagW217, diagW218, diagW220,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Security"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagW101, diagW102, diagW103, diagW300, diagW301, diagW302,
                    diagW303, diagW304, diagW305, diagW306, diagW307, diagW309,
                    diagW313,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Hints"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagH300, diagH301, diagI230, diagI231, diagW123, diagW242,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Shimmer"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagS100, diagS101, diagS102, diagS103, diagS110,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — Taint"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagT100, diagT101, diagT102, diagT104, diagT105,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — iRules"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagIRULE1001, diagIRULE1002, diagIRULE1003, diagIRULE1004, diagIRULE1005, diagIRULE1006,
                    diagIRULE1007, diagIRULE1008, diagIRULE1201, diagIRULE1202, diagIRULE2001, diagIRULE2002,
                    diagIRULE2003, diagIRULE2004, diagIRULE2101, diagIRULE5001, diagIRULE5002, diagIRULE5004,
                    diagIRULE5005, diagIRULE5006, diagIRULE5007, diagIRULE3001, diagIRULE3002, diagIRULE3003,
                    diagIRULE3004, diagIRULE3101, diagIRULE3102, diagIRULE4001, diagIRULE4002, diagIRULE4003,
                    diagIRULE4004, diagIRULE4005,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — BIG-IP Configuration"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagBIGIP6001, diagBIGIP6002, diagBIGIP6003, diagBIGIP6004, diagBIGIP6005, diagBIGIP6006,
                    diagBIGIP6007, diagBIGIP6008, diagBIGIP6009, diagBIGIP6010, diagBIGIP6011, diagBIGIP6012,
                    diagBIGIP6013, diagBIGIP6014, diagBIGIP6038, diagBIGIP6039, diagIAPP7001, diagIAPP7002,
                    diagIAPP7003,
                ),
            ),
        )

        builder.addComponent(TitledSeparator("Diagnostics — SslicTcl"))
        builder.addComponent(
            ReflowingGrid(
                listOf(
                    diagSSLIC1001, diagSSLIC1002, diagSSLIC1003, diagSSLIC1004, diagSSLIC1005, diagSSLIC1006,
                    diagSSLIC1007, diagSSLIC1008, diagSSLIC1009, diagSSLIC1010, diagSSLIC1011, diagSSLIC1012,
                    diagSSLIC1101, diagSSLIC1102, diagSSLIC1103,
                ),
            ),
        )
        // @generated:diag-ui:end

        // Style section
        builder.addComponent(TitledSeparator("Style"))
        builder.addLabeledComponent(JBLabel("Line length (W111 threshold):"), styleLineLength)

        // @generated:opt-ui:begin
        builder.addComponent(TitledSeparator("Optimiser"))
        builder.addComponent(optEnabled)
        builder.addLabeledComponent(JBLabel("Profile:"), profileRow())
        builder.addComponent(ReflowingGrid(optCodeBoxes))
        builder.addWrappedComment(
            "The profile chooses which optimisation families run. A per-code box left " +
                "in its mixed state inherits from the profile; tick or untick one to " +
                "force that code on or off regardless of the profile. Reset to profile " +
                "clears every override and hands the choice back to the profile.",
        )
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
        builder.addWrappedComment("auto: detect from dialect.  tclsh: use tclsh.  expect: use Expect.")
        builder.addLabeledComponent(JBLabel("tclsh path:"), rtTclshPath)
        builder.addWrappedComment("Path to tclsh interpreter. Leave empty for auto-discovery.")
        builder.addLabeledComponent(JBLabel("Timeout (ms):"), rtTimeoutMs)

        // AI
        builder.addComponent(TitledSeparator("AI"))
        builder.addComponent(aiEnabled)
        builder.addLabeledComponent(JBLabel("Extra prompts (JSON):"), aiExtraPrompts)
        builder.addWrappedComment("JSON array of prompt objects for AI-assisted features.")

        // Diagnostic patterns
        builder.addComponent(TitledSeparator("Diagnostic Patterns"))
        builder.addLabeledComponent(JBLabel("Generic variable patterns:"), genericPatternsField)
        builder.addWrappedComment("Newline-separated regex patterns for IRULE4002 generic variable detection.")
        builder.addLabeledComponent(JBLabel("Exclude files from diagnostics:"), diagnosticsExcludeField)
        builder.addWrappedComment("Newline-separated glob patterns (e.g. generated/** or *.gen.tcl); matching files publish no diagnostics. Longer lists are easier to keep in the [diagnostics] exclude key of .tcl-lsp.ini.")

        builder.addComponentFillVertically(JPanel(), 0)

        root = ScrollableForm(builder.panel)

        reset()
    }

    fun isModified(): Boolean {
        val s = TclLspSettings.getInstance()
        return serverPathField.text != s.serverPath ||
            dialectCombo.selectedIndex != TclLspSettings.DIALECT_OPTIONS.indexOfFirst { it.first == s.dialect } ||
            extraCommandsField.text != s.extraCommands ||
            libraryPathsField.text != s.libraryPaths ||
            signatureHelpDisabledCommandsOverride(
                signatureHelpInheritDisabledCommands.isSelected,
                signatureHelpDisabledCommandsField.text,
            ) != s.signatureHelpDisabledCommands ||
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
            diagE006.isSelected != s.diagnosticE006 ||
            diagE200.isSelected != s.diagnosticE200 ||
            diagW001.isSelected != s.diagnosticW001 ||
            diagW002.isSelected != s.diagnosticW002 ||
            diagW003.isSelected != s.diagnosticW003 ||
            diagW004.isSelected != s.diagnosticW004 ||
            diagW100.isSelected != s.diagnosticW100 ||
            diagW104.isSelected != s.diagnosticW104 ||
            diagW105.isSelected != s.diagnosticW105 ||
            diagW106.isSelected != s.diagnosticW106 ||
            diagW107.isSelected != s.diagnosticW107 ||
            diagW108.isSelected != s.diagnosticW108 ||
            diagW109.isSelected != s.diagnosticW109 ||
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
            diagW124.isSelected != s.diagnosticW124 ||
            diagW125.isSelected != s.diagnosticW125 ||
            diagW126.isSelected != s.diagnosticW126 ||
            diagW127.isSelected != s.diagnosticW127 ||
            diagW128.isSelected != s.diagnosticW128 ||
            diagW129.isSelected != s.diagnosticW129 ||
            diagW135.isSelected != s.diagnosticW135 ||
            diagW136.isSelected != s.diagnosticW136 ||
            diagW137.isSelected != s.diagnosticW137 ||
            diagW138.isSelected != s.diagnosticW138 ||
            diagW139.isSelected != s.diagnosticW139 ||
            diagW140.isSelected != s.diagnosticW140 ||
            diagW141.isSelected != s.diagnosticW141 ||
            diagW142.isSelected != s.diagnosticW142 ||
            diagW143.isSelected != s.diagnosticW143 ||
            diagW144.isSelected != s.diagnosticW144 ||
            diagW145.isSelected != s.diagnosticW145 ||
            diagW146.isSelected != s.diagnosticW146 ||
            diagW147.isSelected != s.diagnosticW147 ||
            diagW148.isSelected != s.diagnosticW148 ||
            diagW149.isSelected != s.diagnosticW149 ||
            diagW150.isSelected != s.diagnosticW150 ||
            diagW151.isSelected != s.diagnosticW151 ||
            diagW152.isSelected != s.diagnosticW152 ||
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
            diagW314.isSelected != s.diagnosticW314 ||
            diagW315.isSelected != s.diagnosticW315 ||
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
            diagW305.isSelected != s.diagnosticW305 ||
            diagW306.isSelected != s.diagnosticW306 ||
            diagW307.isSelected != s.diagnosticW307 ||
            diagW309.isSelected != s.diagnosticW309 ||
            diagW313.isSelected != s.diagnosticW313 ||
            diagH300.isSelected != s.diagnosticH300 ||
            diagH301.isSelected != s.diagnosticH301 ||
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
            diagIRULE2004.isSelected != s.diagnosticIRULE2004 ||
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
            diagBIGIP6001.isSelected != s.diagnosticBIGIP6001 ||
            diagBIGIP6002.isSelected != s.diagnosticBIGIP6002 ||
            diagBIGIP6003.isSelected != s.diagnosticBIGIP6003 ||
            diagBIGIP6004.isSelected != s.diagnosticBIGIP6004 ||
            diagBIGIP6005.isSelected != s.diagnosticBIGIP6005 ||
            diagBIGIP6006.isSelected != s.diagnosticBIGIP6006 ||
            diagBIGIP6007.isSelected != s.diagnosticBIGIP6007 ||
            diagBIGIP6008.isSelected != s.diagnosticBIGIP6008 ||
            diagBIGIP6009.isSelected != s.diagnosticBIGIP6009 ||
            diagBIGIP6010.isSelected != s.diagnosticBIGIP6010 ||
            diagBIGIP6011.isSelected != s.diagnosticBIGIP6011 ||
            diagBIGIP6012.isSelected != s.diagnosticBIGIP6012 ||
            diagBIGIP6013.isSelected != s.diagnosticBIGIP6013 ||
            diagBIGIP6014.isSelected != s.diagnosticBIGIP6014 ||
            diagBIGIP6038.isSelected != s.diagnosticBIGIP6038 ||
            diagBIGIP6039.isSelected != s.diagnosticBIGIP6039 ||
            diagIAPP7001.isSelected != s.diagnosticIAPP7001 ||
            diagIAPP7002.isSelected != s.diagnosticIAPP7002 ||
            diagIAPP7003.isSelected != s.diagnosticIAPP7003 ||
            diagSSLIC1001.isSelected != s.diagnosticSSLIC1001 ||
            diagSSLIC1002.isSelected != s.diagnosticSSLIC1002 ||
            diagSSLIC1003.isSelected != s.diagnosticSSLIC1003 ||
            diagSSLIC1004.isSelected != s.diagnosticSSLIC1004 ||
            diagSSLIC1005.isSelected != s.diagnosticSSLIC1005 ||
            diagSSLIC1006.isSelected != s.diagnosticSSLIC1006 ||
            diagSSLIC1007.isSelected != s.diagnosticSSLIC1007 ||
            diagSSLIC1008.isSelected != s.diagnosticSSLIC1008 ||
            diagSSLIC1009.isSelected != s.diagnosticSSLIC1009 ||
            diagSSLIC1010.isSelected != s.diagnosticSSLIC1010 ||
            diagSSLIC1011.isSelected != s.diagnosticSSLIC1011 ||
            diagSSLIC1012.isSelected != s.diagnosticSSLIC1012 ||
            diagSSLIC1101.isSelected != s.diagnosticSSLIC1101 ||
            diagSSLIC1102.isSelected != s.diagnosticSSLIC1102 ||
            diagSSLIC1103.isSelected != s.diagnosticSSLIC1103 ||
            // @generated:diag-dirty:end
            // XC Diagnostics
            xcDiagnosticsEnabled.isSelected != s.xcDiagnosticsEnabled ||
            // Style
            (styleLineLength.value as Int) != s.styleLineLength ||
            // @generated:opt-dirty:begin
            optEnabled.isSelected != s.optimiserEnabled ||
            optProfile.selectedItem != s.optimiserProfile ||
            triState(optO100) != s.optimiserO100 ||
            triState(optO101) != s.optimiserO101 ||
            triState(optO102) != s.optimiserO102 ||
            triState(optO103) != s.optimiserO103 ||
            triState(optO104) != s.optimiserO104 ||
            triState(optO105) != s.optimiserO105 ||
            triState(optO106) != s.optimiserO106 ||
            triState(optO107) != s.optimiserO107 ||
            triState(optO108) != s.optimiserO108 ||
            triState(optO109) != s.optimiserO109 ||
            triState(optO110) != s.optimiserO110 ||
            triState(optO111) != s.optimiserO111 ||
            triState(optO112) != s.optimiserO112 ||
            triState(optO113) != s.optimiserO113 ||
            triState(optO114) != s.optimiserO114 ||
            triState(optO115) != s.optimiserO115 ||
            triState(optO116) != s.optimiserO116 ||
            triState(optO117) != s.optimiserO117 ||
            triState(optO118) != s.optimiserO118 ||
            triState(optO119) != s.optimiserO119 ||
            triState(optO120) != s.optimiserO120 ||
            triState(optO121) != s.optimiserO121 ||
            triState(optO122) != s.optimiserO122 ||
            triState(optO123) != s.optimiserO123 ||
            triState(optO124) != s.optimiserO124 ||
            triState(optO125) != s.optimiserO125 ||
            triState(optO126) != s.optimiserO126 ||
            triState(optO127) != s.optimiserO127 ||
            triState(optO128) != s.optimiserO128 ||
            triState(optO129) != s.optimiserO129 ||
            triState(optO130) != s.optimiserO130 ||
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
            genericPatternsField.text != s.diagnosticsGenericVariablePatterns ||
            diagnosticsExcludeField.text != s.diagnosticsExclude
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
        s.signatureHelpDisabledCommands = signatureHelpDisabledCommandsOverride(
            signatureHelpInheritDisabledCommands.isSelected,
            signatureHelpDisabledCommandsField.text,
        )

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
        s.diagnosticE006 = diagE006.isSelected
        s.diagnosticE200 = diagE200.isSelected
        s.diagnosticW001 = diagW001.isSelected
        s.diagnosticW002 = diagW002.isSelected
        s.diagnosticW003 = diagW003.isSelected
        s.diagnosticW004 = diagW004.isSelected
        s.diagnosticW100 = diagW100.isSelected
        s.diagnosticW104 = diagW104.isSelected
        s.diagnosticW105 = diagW105.isSelected
        s.diagnosticW106 = diagW106.isSelected
        s.diagnosticW107 = diagW107.isSelected
        s.diagnosticW108 = diagW108.isSelected
        s.diagnosticW109 = diagW109.isSelected
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
        s.diagnosticW124 = diagW124.isSelected
        s.diagnosticW125 = diagW125.isSelected
        s.diagnosticW126 = diagW126.isSelected
        s.diagnosticW127 = diagW127.isSelected
        s.diagnosticW128 = diagW128.isSelected
        s.diagnosticW129 = diagW129.isSelected
        s.diagnosticW135 = diagW135.isSelected
        s.diagnosticW136 = diagW136.isSelected
        s.diagnosticW137 = diagW137.isSelected
        s.diagnosticW138 = diagW138.isSelected
        s.diagnosticW139 = diagW139.isSelected
        s.diagnosticW140 = diagW140.isSelected
        s.diagnosticW141 = diagW141.isSelected
        s.diagnosticW142 = diagW142.isSelected
        s.diagnosticW143 = diagW143.isSelected
        s.diagnosticW144 = diagW144.isSelected
        s.diagnosticW145 = diagW145.isSelected
        s.diagnosticW146 = diagW146.isSelected
        s.diagnosticW147 = diagW147.isSelected
        s.diagnosticW148 = diagW148.isSelected
        s.diagnosticW149 = diagW149.isSelected
        s.diagnosticW150 = diagW150.isSelected
        s.diagnosticW151 = diagW151.isSelected
        s.diagnosticW152 = diagW152.isSelected
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
        s.diagnosticW314 = diagW314.isSelected
        s.diagnosticW315 = diagW315.isSelected
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
        s.diagnosticW305 = diagW305.isSelected
        s.diagnosticW306 = diagW306.isSelected
        s.diagnosticW307 = diagW307.isSelected
        s.diagnosticW309 = diagW309.isSelected
        s.diagnosticW313 = diagW313.isSelected
        s.diagnosticH300 = diagH300.isSelected
        s.diagnosticH301 = diagH301.isSelected
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
        s.diagnosticIRULE2004 = diagIRULE2004.isSelected
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
        s.diagnosticBIGIP6001 = diagBIGIP6001.isSelected
        s.diagnosticBIGIP6002 = diagBIGIP6002.isSelected
        s.diagnosticBIGIP6003 = diagBIGIP6003.isSelected
        s.diagnosticBIGIP6004 = diagBIGIP6004.isSelected
        s.diagnosticBIGIP6005 = diagBIGIP6005.isSelected
        s.diagnosticBIGIP6006 = diagBIGIP6006.isSelected
        s.diagnosticBIGIP6007 = diagBIGIP6007.isSelected
        s.diagnosticBIGIP6008 = diagBIGIP6008.isSelected
        s.diagnosticBIGIP6009 = diagBIGIP6009.isSelected
        s.diagnosticBIGIP6010 = diagBIGIP6010.isSelected
        s.diagnosticBIGIP6011 = diagBIGIP6011.isSelected
        s.diagnosticBIGIP6012 = diagBIGIP6012.isSelected
        s.diagnosticBIGIP6013 = diagBIGIP6013.isSelected
        s.diagnosticBIGIP6014 = diagBIGIP6014.isSelected
        s.diagnosticBIGIP6038 = diagBIGIP6038.isSelected
        s.diagnosticBIGIP6039 = diagBIGIP6039.isSelected
        s.diagnosticIAPP7001 = diagIAPP7001.isSelected
        s.diagnosticIAPP7002 = diagIAPP7002.isSelected
        s.diagnosticIAPP7003 = diagIAPP7003.isSelected
        s.diagnosticSSLIC1001 = diagSSLIC1001.isSelected
        s.diagnosticSSLIC1002 = diagSSLIC1002.isSelected
        s.diagnosticSSLIC1003 = diagSSLIC1003.isSelected
        s.diagnosticSSLIC1004 = diagSSLIC1004.isSelected
        s.diagnosticSSLIC1005 = diagSSLIC1005.isSelected
        s.diagnosticSSLIC1006 = diagSSLIC1006.isSelected
        s.diagnosticSSLIC1007 = diagSSLIC1007.isSelected
        s.diagnosticSSLIC1008 = diagSSLIC1008.isSelected
        s.diagnosticSSLIC1009 = diagSSLIC1009.isSelected
        s.diagnosticSSLIC1010 = diagSSLIC1010.isSelected
        s.diagnosticSSLIC1011 = diagSSLIC1011.isSelected
        s.diagnosticSSLIC1012 = diagSSLIC1012.isSelected
        s.diagnosticSSLIC1101 = diagSSLIC1101.isSelected
        s.diagnosticSSLIC1102 = diagSSLIC1102.isSelected
        s.diagnosticSSLIC1103 = diagSSLIC1103.isSelected
        // @generated:diag-apply:end
        s.xcDiagnosticsEnabled = xcDiagnosticsEnabled.isSelected

        s.styleLineLength = styleLineLength.value as Int

        // @generated:opt-apply:begin
        s.optimiserEnabled = optEnabled.isSelected
        s.optimiserProfile = optProfile.selectedItem as? String ?: s.optimiserProfile
        s.optimiserO100 = triState(optO100)
        s.optimiserO101 = triState(optO101)
        s.optimiserO102 = triState(optO102)
        s.optimiserO103 = triState(optO103)
        s.optimiserO104 = triState(optO104)
        s.optimiserO105 = triState(optO105)
        s.optimiserO106 = triState(optO106)
        s.optimiserO107 = triState(optO107)
        s.optimiserO108 = triState(optO108)
        s.optimiserO109 = triState(optO109)
        s.optimiserO110 = triState(optO110)
        s.optimiserO111 = triState(optO111)
        s.optimiserO112 = triState(optO112)
        s.optimiserO113 = triState(optO113)
        s.optimiserO114 = triState(optO114)
        s.optimiserO115 = triState(optO115)
        s.optimiserO116 = triState(optO116)
        s.optimiserO117 = triState(optO117)
        s.optimiserO118 = triState(optO118)
        s.optimiserO119 = triState(optO119)
        s.optimiserO120 = triState(optO120)
        s.optimiserO121 = triState(optO121)
        s.optimiserO122 = triState(optO122)
        s.optimiserO123 = triState(optO123)
        s.optimiserO124 = triState(optO124)
        s.optimiserO125 = triState(optO125)
        s.optimiserO126 = triState(optO126)
        s.optimiserO127 = triState(optO127)
        s.optimiserO128 = triState(optO128)
        s.optimiserO129 = triState(optO129)
        s.optimiserO130 = triState(optO130)
        // @generated:opt-apply:end

        s.shimmerEnabled = shimmerEnabled.isSelected
        s.runtimeValidationEnabled = runtimeValidation.isSelected
        s.runtimeValidationAdapter = rtAdapter.selectedItem as String
        s.runtimeValidationTclshPath = rtTclshPath.text
        s.runtimeValidationTimeoutMs = rtTimeoutMs.value as Int
        s.aiEnabled = aiEnabled.isSelected
        s.aiExtraPrompts = aiExtraPrompts.text
        s.diagnosticsGenericVariablePatterns = genericPatternsField.text
        s.diagnosticsExclude = diagnosticsExcludeField.text

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
    /**
     * The profile selector, with the link that clears every per-code override
     * beside it.
     *
     * A function rather than a property: it reads `optProfile` and
     * `optCodeBoxes`, both generated declarations, and building it while the
     * form is assembled keeps it independent of the order those initialise in.
     */
    private fun profileRow(): JPanel =
        JPanel(FlowLayout(FlowLayout.LEFT, JBUI.scale(8), 0)).apply {
            add(optProfile)
            add(ActionLink("Reset codes to profile") { resetCodesToProfile() })
        }

    /**
     * Hand every per-code choice back to the profile.
     *
     * The third state is the one that defers, so this resets to "no opinion"
     * rather than to a set of ticks: without it, a user who explicitly set a
     * handful of codes has no way to find which, or to undo them short of
     * clicking each back to mixed.
     *
     * Only the controls change — `isModified` then reports the panel dirty and
     * the usual Apply writes it through, so this is as undoable as any other
     * edit on the page.
     */
    private fun resetCodesToProfile() {
        optCodeBoxes.forEach { it.state = ThreeStateCheckBox.State.DONT_CARE }
    }

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
        signatureHelpDisabledCommandsField.text = s.signatureHelpDisabledCommands.orEmpty()
        signatureHelpInheritDisabledCommands.isSelected = s.signatureHelpDisabledCommands == null
        signatureHelpDisabledCommandsField.isEnabled = !signatureHelpInheritDisabledCommands.isSelected

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
        diagE006.isSelected = s.diagnosticE006
        diagE200.isSelected = s.diagnosticE200
        diagW001.isSelected = s.diagnosticW001
        diagW002.isSelected = s.diagnosticW002
        diagW003.isSelected = s.diagnosticW003
        diagW004.isSelected = s.diagnosticW004
        diagW100.isSelected = s.diagnosticW100
        diagW104.isSelected = s.diagnosticW104
        diagW105.isSelected = s.diagnosticW105
        diagW106.isSelected = s.diagnosticW106
        diagW107.isSelected = s.diagnosticW107
        diagW108.isSelected = s.diagnosticW108
        diagW109.isSelected = s.diagnosticW109
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
        diagW124.isSelected = s.diagnosticW124
        diagW125.isSelected = s.diagnosticW125
        diagW126.isSelected = s.diagnosticW126
        diagW127.isSelected = s.diagnosticW127
        diagW128.isSelected = s.diagnosticW128
        diagW129.isSelected = s.diagnosticW129
        diagW135.isSelected = s.diagnosticW135
        diagW136.isSelected = s.diagnosticW136
        diagW137.isSelected = s.diagnosticW137
        diagW138.isSelected = s.diagnosticW138
        diagW139.isSelected = s.diagnosticW139
        diagW140.isSelected = s.diagnosticW140
        diagW141.isSelected = s.diagnosticW141
        diagW142.isSelected = s.diagnosticW142
        diagW143.isSelected = s.diagnosticW143
        diagW144.isSelected = s.diagnosticW144
        diagW145.isSelected = s.diagnosticW145
        diagW146.isSelected = s.diagnosticW146
        diagW147.isSelected = s.diagnosticW147
        diagW148.isSelected = s.diagnosticW148
        diagW149.isSelected = s.diagnosticW149
        diagW150.isSelected = s.diagnosticW150
        diagW151.isSelected = s.diagnosticW151
        diagW152.isSelected = s.diagnosticW152
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
        diagW314.isSelected = s.diagnosticW314
        diagW315.isSelected = s.diagnosticW315
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
        diagW305.isSelected = s.diagnosticW305
        diagW306.isSelected = s.diagnosticW306
        diagW307.isSelected = s.diagnosticW307
        diagW309.isSelected = s.diagnosticW309
        diagW313.isSelected = s.diagnosticW313
        diagH300.isSelected = s.diagnosticH300
        diagH301.isSelected = s.diagnosticH301
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
        diagIRULE2004.isSelected = s.diagnosticIRULE2004
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
        diagBIGIP6001.isSelected = s.diagnosticBIGIP6001
        diagBIGIP6002.isSelected = s.diagnosticBIGIP6002
        diagBIGIP6003.isSelected = s.diagnosticBIGIP6003
        diagBIGIP6004.isSelected = s.diagnosticBIGIP6004
        diagBIGIP6005.isSelected = s.diagnosticBIGIP6005
        diagBIGIP6006.isSelected = s.diagnosticBIGIP6006
        diagBIGIP6007.isSelected = s.diagnosticBIGIP6007
        diagBIGIP6008.isSelected = s.diagnosticBIGIP6008
        diagBIGIP6009.isSelected = s.diagnosticBIGIP6009
        diagBIGIP6010.isSelected = s.diagnosticBIGIP6010
        diagBIGIP6011.isSelected = s.diagnosticBIGIP6011
        diagBIGIP6012.isSelected = s.diagnosticBIGIP6012
        diagBIGIP6013.isSelected = s.diagnosticBIGIP6013
        diagBIGIP6014.isSelected = s.diagnosticBIGIP6014
        diagBIGIP6038.isSelected = s.diagnosticBIGIP6038
        diagBIGIP6039.isSelected = s.diagnosticBIGIP6039
        diagIAPP7001.isSelected = s.diagnosticIAPP7001
        diagIAPP7002.isSelected = s.diagnosticIAPP7002
        diagIAPP7003.isSelected = s.diagnosticIAPP7003
        diagSSLIC1001.isSelected = s.diagnosticSSLIC1001
        diagSSLIC1002.isSelected = s.diagnosticSSLIC1002
        diagSSLIC1003.isSelected = s.diagnosticSSLIC1003
        diagSSLIC1004.isSelected = s.diagnosticSSLIC1004
        diagSSLIC1005.isSelected = s.diagnosticSSLIC1005
        diagSSLIC1006.isSelected = s.diagnosticSSLIC1006
        diagSSLIC1007.isSelected = s.diagnosticSSLIC1007
        diagSSLIC1008.isSelected = s.diagnosticSSLIC1008
        diagSSLIC1009.isSelected = s.diagnosticSSLIC1009
        diagSSLIC1010.isSelected = s.diagnosticSSLIC1010
        diagSSLIC1011.isSelected = s.diagnosticSSLIC1011
        diagSSLIC1012.isSelected = s.diagnosticSSLIC1012
        diagSSLIC1101.isSelected = s.diagnosticSSLIC1101
        diagSSLIC1102.isSelected = s.diagnosticSSLIC1102
        diagSSLIC1103.isSelected = s.diagnosticSSLIC1103
        // @generated:diag-reset:end
        xcDiagnosticsEnabled.isSelected = s.xcDiagnosticsEnabled

        styleLineLength.value = s.styleLineLength

        // @generated:opt-reset:begin
        optEnabled.isSelected = s.optimiserEnabled
        optProfile.selectedItem = s.optimiserProfile
        optO100.state = threeState(s.optimiserO100)
        optO101.state = threeState(s.optimiserO101)
        optO102.state = threeState(s.optimiserO102)
        optO103.state = threeState(s.optimiserO103)
        optO104.state = threeState(s.optimiserO104)
        optO105.state = threeState(s.optimiserO105)
        optO106.state = threeState(s.optimiserO106)
        optO107.state = threeState(s.optimiserO107)
        optO108.state = threeState(s.optimiserO108)
        optO109.state = threeState(s.optimiserO109)
        optO110.state = threeState(s.optimiserO110)
        optO111.state = threeState(s.optimiserO111)
        optO112.state = threeState(s.optimiserO112)
        optO113.state = threeState(s.optimiserO113)
        optO114.state = threeState(s.optimiserO114)
        optO115.state = threeState(s.optimiserO115)
        optO116.state = threeState(s.optimiserO116)
        optO117.state = threeState(s.optimiserO117)
        optO118.state = threeState(s.optimiserO118)
        optO119.state = threeState(s.optimiserO119)
        optO120.state = threeState(s.optimiserO120)
        optO121.state = threeState(s.optimiserO121)
        optO122.state = threeState(s.optimiserO122)
        optO123.state = threeState(s.optimiserO123)
        optO124.state = threeState(s.optimiserO124)
        optO125.state = threeState(s.optimiserO125)
        optO126.state = threeState(s.optimiserO126)
        optO127.state = threeState(s.optimiserO127)
        optO128.state = threeState(s.optimiserO128)
        optO129.state = threeState(s.optimiserO129)
        optO130.state = threeState(s.optimiserO130)
        // @generated:opt-reset:end

        shimmerEnabled.isSelected = s.shimmerEnabled
        runtimeValidation.isSelected = s.runtimeValidationEnabled
        rtAdapter.selectedItem = s.runtimeValidationAdapter
        rtTclshPath.text = s.runtimeValidationTclshPath
        rtTimeoutMs.value = s.runtimeValidationTimeoutMs
        aiEnabled.isSelected = s.aiEnabled
        aiExtraPrompts.text = s.aiExtraPrompts
        genericPatternsField.text = s.diagnosticsGenericVariablePatterns
        diagnosticsExcludeField.text = s.diagnosticsExclude
    }
}
