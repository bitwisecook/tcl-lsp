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

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors as Colors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.platform.lsp.api.customization.LspCustomization
import com.intellij.platform.lsp.api.customization.LspDiagnosticsCustomizer
import com.intellij.platform.lsp.api.customization.LspSemanticTokensCustomizer
import com.intellij.platform.lsp.api.customization.LspSemanticTokensSupport
import com.intellij.psi.PsiFile
import com.tcllsp.jetbrains.settings.TclLspSettings

/**
 * Colour for every semantic-token type the server advertises in its
 * `initialize` legend (`tcl_lsp_core::semantic_tokens::legend_token_types`).
 *
 * `cargo xtask gen-jetbrains-catalog --check` fails when this map and that
 * legend disagree: a type the server emits but this map omits is a token the
 * IDE paints in the default foreground, which is indistinguishable from
 * having no semantic highlighting at all.
 *
 * A key the stock schemes give no foreground to leaves its token in the plain
 * text colour, so the ordinary parts of a Tcl file need a key that is painted
 * everywhere. FUNCTION_CALL, PREDEFINED_SYMBOL, LOCAL_VARIABLE, PARAMETER,
 * IDENTIFIER and OPERATION_SIGN are not: they stay unused except where the
 * IDE leaves the same token plain in its own languages too (an `operator`, a
 * bare name in a config file). So every command head — built-in, user proc,
 * method, or iRule event handler — is FUNCTION_DECLARATION, the key the
 * platform's own LSP default gives the `function` type.
 *
 * The palette is smaller than the legend, so several Tcl types share a key,
 * the same collapsing the VS Code extension does with `semanticTokenScopes`.
 * The colours stay the user's: every key is a stock one, retuned on the
 * scheme's Language Defaults page.
 */
// @generated-check:semantic-token-colors — keys are verified against the
// server legend by `cargo xtask gen-jetbrains-catalog`.
internal val SEMANTIC_TOKEN_COLORS: Map<String, TextAttributesKey> = mapOf(
    // Core Tcl. Every command head is a declaration key; the variable-shaped
    // types share the field key, which stock schemes colour.
    "keyword" to Colors.KEYWORD,
    "function" to Colors.FUNCTION_DECLARATION,
    "method" to Colors.FUNCTION_DECLARATION,
    "variable" to Colors.INSTANCE_FIELD,
    "parameter" to Colors.INSTANCE_FIELD,
    "string" to Colors.STRING,
    "number" to Colors.NUMBER,
    "comment" to Colors.LINE_COMMENT,
    "namespace" to Colors.CLASS_REFERENCE,
    "operator" to Colors.OPERATION_SIGN,
    "escape" to Colors.VALID_STRING_ESCAPE,
    "class" to Colors.CLASS_NAME,
    "object" to Colors.CLASS_REFERENCE,
    "decorator" to Colors.METADATA,
    "enumMember" to Colors.CONSTANT,
    "property" to Colors.INSTANCE_FIELD,
    // An iRule event block is a handler definition, not a tag.
    "event" to Colors.FUNCTION_DECLARATION,

    // Regular-expression interiors, mirroring IntelliJ's own RegExp
    // highlighter: meta characters and quantifiers read as keywords,
    // classes and escapes as string escapes.
    "regexp" to Colors.STRING,
    "regexpGroup" to Colors.KEYWORD,
    "regexpQuantifier" to Colors.KEYWORD,
    "regexpAnchor" to Colors.KEYWORD,
    "regexpAlternation" to Colors.KEYWORD,
    "regexpCharClass" to Colors.VALID_STRING_ESCAPE,
    "regexpEscape" to Colors.VALID_STRING_ESCAPE,
    "regexpBackref" to Colors.VALID_STRING_ESCAPE,

    // `format` / `clock` / `binary` specifiers, all inside a string literal —
    // IntelliJ paints a whole Java format specifier as a string escape.
    "formatPercent" to Colors.VALID_STRING_ESCAPE,
    "formatSpec" to Colors.VALID_STRING_ESCAPE,
    "formatFlag" to Colors.VALID_STRING_ESCAPE,
    "formatWidth" to Colors.NUMBER,
    "clockPercent" to Colors.VALID_STRING_ESCAPE,
    "clockSpec" to Colors.VALID_STRING_ESCAPE,
    "clockModifier" to Colors.VALID_STRING_ESCAPE,
    "binarySpec" to Colors.VALID_STRING_ESCAPE,
    "binaryCount" to Colors.NUMBER,
    "binaryFlag" to Colors.VALID_STRING_ESCAPE,

    // APL — the iApp presentation sublanguage.
    "aplSection" to Colors.KEYWORD,
    "aplSectionName" to Colors.FUNCTION_DECLARATION,
    "aplFieldType" to Colors.KEYWORD,
    "aplFieldName" to Colors.INSTANCE_FIELD,
    "aplAttribute" to Colors.METADATA,
    "aplDefine" to Colors.KEYWORD,
    "aplDefineName" to Colors.CONSTANT,
    "aplDirective" to Colors.METADATA,
    "aplOptional" to Colors.OPERATION_SIGN,
    "aplValidator" to Colors.FUNCTION_DECLARATION,

    // BIG-IP configuration objects and the values that name them.
    "partition" to Colors.CLASS_REFERENCE,
    "pool" to Colors.CLASS_REFERENCE,
    "monitor" to Colors.CLASS_REFERENCE,
    "profile" to Colors.CLASS_REFERENCE,
    "vlan" to Colors.CLASS_REFERENCE,
    "bigipInterface" to Colors.CLASS_REFERENCE,
    "ipAddress" to Colors.CONSTANT,
    "port" to Colors.NUMBER,
    "routeDomain" to Colors.NUMBER,
    "fqdn" to Colors.CONSTANT,
    "username" to Colors.IDENTIFIER,
    "encrypted" to Colors.STRING,
)

/**
 * Semantic-token overrides for the modifiers the server sets alongside a
 * type (`legend_token_modifiers`). Only a pair that resolves to a different
 * key than the type itself is worth listing; anything else falls through to
 * the type's own colour.
 */
private val SEMANTIC_TOKEN_MODIFIER_COLORS: Map<Pair<String, String>, TextAttributesKey> = mapOf(
    // A variable the theme should not present as mutable.
    ("variable" to "readonly") to Colors.CONSTANT,
)

/**
 * The plugin's LSP customisation.
 *
 * Only semantic tokens is customised; every other feature keeps the
 * platform's default. This is the shape that superseded the flat
 * `LspServerDescriptor.lsp*Support` properties in 2025.2 — the old ones still
 * work through `DeprecatedLspCustomization`, but only while the descriptor
 * does not supply a customisation of its own, so the two cannot be mixed.
 */
class TclLspCustomization : LspCustomization() {
    override val semanticTokensCustomizer: LspSemanticTokensCustomizer = TclSemanticTokens()
    override val diagnosticsCustomizer: LspDiagnosticsCustomizer = TclLspDiagnostics()
}

/**
 * Paints the semantic tokens the Tcl language server publishes.
 *
 * The IDE only advertises `textDocument/semanticTokens` in its client
 * capabilities when a descriptor supplies one of these, so without it the
 * server's tokens are never requested and all colour comes from the TextMate
 * grammar.
 */
class TclSemanticTokens : LspSemanticTokensSupport() {

    /**
     * Honours the "Semantic tokens" feature toggle. The setting already
     * reaches the server through `workspace/configuration`, but a server that
     * has been told to stay quiet still answers the request, so the IDE has to
     * stop asking too.
     */
    override fun shouldAskServerForSemanticTokens(psiFile: PsiFile): Boolean =
        TclLspSettings.getInstance().featureSemanticTokens

    override fun getTextAttributesKey(
        tokenType: String,
        modifiers: List<String>,
    ): TextAttributesKey? {
        for (modifier in modifiers) {
            SEMANTIC_TOKEN_MODIFIER_COLORS[tokenType to modifier]?.let { return it }
        }
        return SEMANTIC_TOKEN_COLORS[tokenType]
            ?: super.getTextAttributesKey(tokenType, modifiers)
    }
}
