package com.tcllsp.jetbrains.settings

import com.intellij.ui.TitledSeparator
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import com.intellij.util.ui.JBUI
import javax.swing.*

class TclLspSettingsPanel {

    // General
    private val pythonPathField = JBTextField(30)
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
    private val featureFormatting = JBCheckBox("Formatting")
    private val featureSemanticTokens = JBCheckBox("Semantic tokens")
    private val featureCodeActions = JBCheckBox("Code actions")
    private val featureDefinition = JBCheckBox("Go to definition")
    private val featureReferences = JBCheckBox("Find references")
    private val featureDocumentSymbols = JBCheckBox("Document symbols")
    private val featureFolding = JBCheckBox("Code folding")
    private val featureRename = JBCheckBox("Rename symbol")
    private val featureSignatureHelp = JBCheckBox("Signature help")
    private val featureWorkspaceSymbols = JBCheckBox("Workspace symbols")
    private val featureInlayHints = JBCheckBox("Inlay hints")
    private val featureCallHierarchy = JBCheckBox("Call hierarchy")
    private val featureDocumentLinks = JBCheckBox("Document links")
    private val featureSelectionRange = JBCheckBox("Selection range")

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

    // Diagnostics
    private val diagE001 = JBCheckBox("E001: Missing subcommand")
    private val diagE002 = JBCheckBox("E002: Too few arguments")
    private val diagE003 = JBCheckBox("E003: Too many arguments")
    private val diagE200 = JBCheckBox("E200: Shimmer parse error")
    private val diagW001 = JBCheckBox("W001: Unknown subcommand")
    private val diagW002 = JBCheckBox("W002: Disabled command")
    private val diagW100 = JBCheckBox("W100: Unbraced expression")
    private val diagW104 = JBCheckBox("W104: String concat for list")
    private val diagW105 = JBCheckBox("W105: Unbraced code block")
    private val diagW106 = JBCheckBox("W106: Unbraced switch body")
    private val diagW108 = JBCheckBox("W108: Non-ASCII characters")
    private val diagW110 = JBCheckBox("W110: Use eq/ne for strings")
    private val diagW111 = JBCheckBox("W111: Line too long")
    private val diagW112 = JBCheckBox("W112: Trailing whitespace")
    private val diagW113 = JBCheckBox("W113: Proc shadows built-in")
    private val diagW114 = JBCheckBox("W114: Redundant nested expr")
    private val diagW115 = JBCheckBox("W115: Backslash-newline in comment")
    private val diagW120 = JBCheckBox("W120: Missing package require")
    private val diagW121 = JBCheckBox("W121: Invalid subnet mask")
    private val diagW122 = JBCheckBox("W122: Mistyped IPv4 address")
    private val diagW200 = JBCheckBox("W200: Exec result not captured")
    private val diagW201 = JBCheckBox("W201: Manual path concatenation")
    private val diagW210 = JBCheckBox("W210: Read before set")
    private val diagW211 = JBCheckBox("W211: Set but never used")
    private val diagW212 = JBCheckBox("W212: Variable substitution")
    private val diagW213 = JBCheckBox("W213: Variable may not exist")
    private val diagW214 = JBCheckBox("W214: Unused proc parameter")
    private val diagW302 = JBCheckBox("W302: Catch without result var")
    private val diagW304 = JBCheckBox("W304: Missing --")
    private val diagW307 = JBCheckBox("W307: Non-literal command")
    private val diagW308 = JBCheckBox("W308: subst without -nocommands")
    private val diagW309 = JBCheckBox("W309: eval/uplevel with subst")

    // Style
    private val styleLineLength = JSpinner(SpinnerNumberModel(120, 40, 500, 10))

    // Optimiser
    private val optEnabled = JBCheckBox("Enable optimiser suggestions")
    private val optO100 = JBCheckBox("O100")
    private val optO101 = JBCheckBox("O101")
    private val optO102 = JBCheckBox("O102")
    private val optO103 = JBCheckBox("O103")
    private val optO104 = JBCheckBox("O104")
    private val optO105 = JBCheckBox("O105")
    private val optO106 = JBCheckBox("O106")
    private val optO107 = JBCheckBox("O107")
    private val optO108 = JBCheckBox("O108")
    private val optO109 = JBCheckBox("O109")
    private val optO110 = JBCheckBox("O110")
    private val optO111 = JBCheckBox("O111")
    private val optO112 = JBCheckBox("O112")
    private val optO113 = JBCheckBox("O113")
    private val optO114 = JBCheckBox("O114")
    private val optO115 = JBCheckBox("O115")
    private val optO116 = JBCheckBox("O116")
    private val optO117 = JBCheckBox("O117")
    private val optO118 = JBCheckBox("O118")
    private val optO119 = JBCheckBox("O119")
    private val optO120 = JBCheckBox("O120")
    private val optO121 = JBCheckBox("O121")
    private val optO122 = JBCheckBox("O122")
    private val optO123 = JBCheckBox("O123")
    private val optO124 = JBCheckBox("O124")
    private val optO125 = JBCheckBox("O125")
    private val optO126 = JBCheckBox("O126")

    // Shimmer
    private val shimmerEnabled = JBCheckBox("Enable shimmer analysis")

    // Runtime Validation
    private val runtimeValidation = JBCheckBox("Enable runtime validation on save")

    val root: JComponent

    init {
        val builder = FormBuilder.createFormBuilder()

        // General section
        builder.addComponent(TitledSeparator("General"))
        builder.addLabeledComponent(JBLabel("Python path:"), pythonPathField)
        builder.addTooltip("Path to Python 3.10+ interpreter. 'auto' discovers the best available.")
        builder.addLabeledComponent(JBLabel("Server path:"), serverPathField)
        builder.addTooltip("Path to tcl-lsp project root (dev mode). Leave empty for bundled server.")
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
                featureHover, featureCompletion, featureDiagnostics, featureFormatting,
                featureSemanticTokens, featureCodeActions, featureDefinition, featureReferences,
                featureDocumentSymbols, featureFolding, featureRename, featureSignatureHelp,
                featureWorkspaceSymbols, featureInlayHints, featureCallHierarchy,
                featureDocumentLinks, featureSelectionRange,
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

        // Diagnostics section
        builder.addComponent(TitledSeparator("Diagnostics — Errors"))
        val diagErrorPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(diagE001, diagE002, diagE003, diagE200).forEach { diagErrorPanel.add(it) }
        builder.addComponent(diagErrorPanel)

        builder.addComponent(TitledSeparator("Diagnostics — Warnings"))
        val diagWarnPanel = JPanel(java.awt.GridLayout(0, 2, 8, 2))
        listOf(
            diagW001, diagW002, diagW100, diagW104, diagW105, diagW106,
            diagW108, diagW110, diagW111, diagW112, diagW113, diagW114,
            diagW115, diagW120, diagW121, diagW122, diagW200, diagW201, diagW210, diagW211, diagW212,
            diagW213, diagW214, diagW302, diagW304, diagW307, diagW308, diagW309,
        ).forEach { diagWarnPanel.add(it) }
        builder.addComponent(diagWarnPanel)

        // Style section
        builder.addComponent(TitledSeparator("Style"))
        builder.addLabeledComponent(JBLabel("Line length (W111 threshold):"), styleLineLength)

        // Optimiser section
        builder.addComponent(TitledSeparator("Optimiser"))
        builder.addComponent(optEnabled)
        val optPanel = JPanel(java.awt.GridLayout(0, 4, 8, 2))
        listOf(
            optO100, optO101, optO102, optO103, optO104, optO105,
            optO106, optO107, optO108, optO109, optO110, optO111,
            optO112, optO113, optO114, optO115, optO116, optO117,
            optO118, optO119, optO120, optO121, optO122, optO123, optO124, optO125, optO126,
        ).forEach { optPanel.add(it) }
        builder.addComponent(optPanel)

        // Shimmer section
        builder.addComponent(TitledSeparator("Shimmer"))
        builder.addComponent(shimmerEnabled)

        // Runtime Validation
        builder.addComponent(TitledSeparator("Runtime Validation"))
        builder.addComponent(runtimeValidation)

        builder.addComponentFillVertically(JPanel(), 0)

        root = JScrollPane(builder.panel).apply {
            border = JBUI.Borders.empty()
        }

        reset()
    }

    fun isModified(): Boolean {
        val s = TclLspSettings.getInstance()
        return pythonPathField.text != s.pythonPath ||
            serverPathField.text != s.serverPath ||
            dialectCombo.selectedIndex != TclLspSettings.DIALECT_OPTIONS.indexOfFirst { it.first == s.dialect } ||
            extraCommandsField.text != s.extraCommands ||
            libraryPathsField.text != s.libraryPaths ||
            // Features
            featureHover.isSelected != s.featureHover ||
            featureCompletion.isSelected != s.featureCompletion ||
            featureDiagnostics.isSelected != s.featureDiagnostics ||
            featureFormatting.isSelected != s.featureFormatting ||
            featureSemanticTokens.isSelected != s.featureSemanticTokens ||
            featureCodeActions.isSelected != s.featureCodeActions ||
            featureDefinition.isSelected != s.featureDefinition ||
            featureReferences.isSelected != s.featureReferences ||
            featureDocumentSymbols.isSelected != s.featureDocumentSymbols ||
            featureFolding.isSelected != s.featureFolding ||
            featureRename.isSelected != s.featureRename ||
            featureSignatureHelp.isSelected != s.featureSignatureHelp ||
            featureWorkspaceSymbols.isSelected != s.featureWorkspaceSymbols ||
            featureInlayHints.isSelected != s.featureInlayHints ||
            featureCallHierarchy.isSelected != s.featureCallHierarchy ||
            featureDocumentLinks.isSelected != s.featureDocumentLinks ||
            featureSelectionRange.isSelected != s.featureSelectionRange ||
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
            // Diagnostics
            diagE001.isSelected != s.diagnosticE001 ||
            diagE002.isSelected != s.diagnosticE002 ||
            diagE003.isSelected != s.diagnosticE003 ||
            diagE200.isSelected != s.diagnosticE200 ||
            diagW001.isSelected != s.diagnosticW001 ||
            diagW002.isSelected != s.diagnosticW002 ||
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
            diagW120.isSelected != s.diagnosticW120 ||
            diagW121.isSelected != s.diagnosticW121 ||
            diagW122.isSelected != s.diagnosticW122 ||
            diagW200.isSelected != s.diagnosticW200 ||
            diagW201.isSelected != s.diagnosticW201 ||
            diagW210.isSelected != s.diagnosticW210 ||
            diagW211.isSelected != s.diagnosticW211 ||
            diagW212.isSelected != s.diagnosticW212 ||
            diagW213.isSelected != s.diagnosticW213 ||
            diagW214.isSelected != s.diagnosticW214 ||
            diagW302.isSelected != s.diagnosticW302 ||
            diagW304.isSelected != s.diagnosticW304 ||
            diagW307.isSelected != s.diagnosticW307 ||
            diagW308.isSelected != s.diagnosticW308 ||
            diagW309.isSelected != s.diagnosticW309 ||
            // Style
            (styleLineLength.value as Int) != s.styleLineLength ||
            // Optimiser
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
            // Shimmer
            shimmerEnabled.isSelected != s.shimmerEnabled ||
            // Runtime validation
            runtimeValidation.isSelected != s.runtimeValidationEnabled
    }

    fun apply() {
        val s = TclLspSettings.getInstance()
        s.pythonPath = pythonPathField.text
        s.serverPath = serverPathField.text
        s.dialect = TclLspSettings.DIALECT_OPTIONS.getOrNull(dialectCombo.selectedIndex)?.first ?: "tcl8.6"
        s.extraCommands = extraCommandsField.text
        s.libraryPaths = libraryPathsField.text

        s.featureHover = featureHover.isSelected
        s.featureCompletion = featureCompletion.isSelected
        s.featureDiagnostics = featureDiagnostics.isSelected
        s.featureFormatting = featureFormatting.isSelected
        s.featureSemanticTokens = featureSemanticTokens.isSelected
        s.featureCodeActions = featureCodeActions.isSelected
        s.featureDefinition = featureDefinition.isSelected
        s.featureReferences = featureReferences.isSelected
        s.featureDocumentSymbols = featureDocumentSymbols.isSelected
        s.featureFolding = featureFolding.isSelected
        s.featureRename = featureRename.isSelected
        s.featureSignatureHelp = featureSignatureHelp.isSelected
        s.featureWorkspaceSymbols = featureWorkspaceSymbols.isSelected
        s.featureInlayHints = featureInlayHints.isSelected
        s.featureCallHierarchy = featureCallHierarchy.isSelected
        s.featureDocumentLinks = featureDocumentLinks.isSelected
        s.featureSelectionRange = featureSelectionRange.isSelected

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

        s.diagnosticE001 = diagE001.isSelected
        s.diagnosticE002 = diagE002.isSelected
        s.diagnosticE003 = diagE003.isSelected
        s.diagnosticE200 = diagE200.isSelected
        s.diagnosticW001 = diagW001.isSelected
        s.diagnosticW002 = diagW002.isSelected
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
        s.diagnosticW120 = diagW120.isSelected
        s.diagnosticW121 = diagW121.isSelected
        s.diagnosticW122 = diagW122.isSelected
        s.diagnosticW200 = diagW200.isSelected
        s.diagnosticW201 = diagW201.isSelected
        s.diagnosticW210 = diagW210.isSelected
        s.diagnosticW211 = diagW211.isSelected
        s.diagnosticW212 = diagW212.isSelected
        s.diagnosticW213 = diagW213.isSelected
        s.diagnosticW214 = diagW214.isSelected
        s.diagnosticW302 = diagW302.isSelected
        s.diagnosticW304 = diagW304.isSelected
        s.diagnosticW307 = diagW307.isSelected
        s.diagnosticW308 = diagW308.isSelected
        s.diagnosticW309 = diagW309.isSelected

        s.styleLineLength = styleLineLength.value as Int

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

        s.shimmerEnabled = shimmerEnabled.isSelected
        s.runtimeValidationEnabled = runtimeValidation.isSelected
    }

    fun reset() {
        val s = TclLspSettings.getInstance()
        pythonPathField.text = s.pythonPath
        serverPathField.text = s.serverPath
        dialectCombo.selectedIndex = TclLspSettings.DIALECT_OPTIONS.indexOfFirst { it.first == s.dialect }.coerceAtLeast(0)
        extraCommandsField.text = s.extraCommands
        libraryPathsField.text = s.libraryPaths

        featureHover.isSelected = s.featureHover
        featureCompletion.isSelected = s.featureCompletion
        featureDiagnostics.isSelected = s.featureDiagnostics
        featureFormatting.isSelected = s.featureFormatting
        featureSemanticTokens.isSelected = s.featureSemanticTokens
        featureCodeActions.isSelected = s.featureCodeActions
        featureDefinition.isSelected = s.featureDefinition
        featureReferences.isSelected = s.featureReferences
        featureDocumentSymbols.isSelected = s.featureDocumentSymbols
        featureFolding.isSelected = s.featureFolding
        featureRename.isSelected = s.featureRename
        featureSignatureHelp.isSelected = s.featureSignatureHelp
        featureWorkspaceSymbols.isSelected = s.featureWorkspaceSymbols
        featureInlayHints.isSelected = s.featureInlayHints
        featureCallHierarchy.isSelected = s.featureCallHierarchy
        featureDocumentLinks.isSelected = s.featureDocumentLinks
        featureSelectionRange.isSelected = s.featureSelectionRange

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

        diagE001.isSelected = s.diagnosticE001
        diagE002.isSelected = s.diagnosticE002
        diagE003.isSelected = s.diagnosticE003
        diagE200.isSelected = s.diagnosticE200
        diagW001.isSelected = s.diagnosticW001
        diagW002.isSelected = s.diagnosticW002
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
        diagW120.isSelected = s.diagnosticW120
        diagW121.isSelected = s.diagnosticW121
        diagW122.isSelected = s.diagnosticW122
        diagW200.isSelected = s.diagnosticW200
        diagW201.isSelected = s.diagnosticW201
        diagW210.isSelected = s.diagnosticW210
        diagW211.isSelected = s.diagnosticW211
        diagW212.isSelected = s.diagnosticW212
        diagW213.isSelected = s.diagnosticW213
        diagW214.isSelected = s.diagnosticW214
        diagW302.isSelected = s.diagnosticW302
        diagW304.isSelected = s.diagnosticW304
        diagW307.isSelected = s.diagnosticW307
        diagW308.isSelected = s.diagnosticW308
        diagW309.isSelected = s.diagnosticW309

        styleLineLength.value = s.styleLineLength

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

        shimmerEnabled.isSelected = s.shimmerEnabled
        runtimeValidation.isSelected = s.runtimeValidationEnabled
    }
}
