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
 * The keys the platform offers are a general-purpose set, so several Tcl
 * types share one — the regexp operators are all `OPERATION_SIGN`, the
 * `format`/`clock`/`binary` specifiers all read as escapes inside a string.
 * That is the same collapsing the VS Code extension does with its
 * `semanticTokenScopes` mapping.
 */
// @generated-check:semantic-token-colors — keys are verified against the
// server legend by `cargo xtask gen-jetbrains-catalog`.
internal val SEMANTIC_TOKEN_COLORS: Map<String, TextAttributesKey> = mapOf(
    // Core Tcl.
    "keyword" to Colors.KEYWORD,
    "function" to Colors.FUNCTION_CALL,
    "variable" to Colors.LOCAL_VARIABLE,
    "string" to Colors.STRING,
    "number" to Colors.NUMBER,
    "comment" to Colors.LINE_COMMENT,
    "namespace" to Colors.CLASS_REFERENCE,
    "operator" to Colors.OPERATION_SIGN,
    "escape" to Colors.VALID_STRING_ESCAPE,
    "parameter" to Colors.PARAMETER,
    "method" to Colors.INSTANCE_METHOD,
    "class" to Colors.CLASS_NAME,
    "object" to Colors.CLASS_REFERENCE,
    "decorator" to Colors.METADATA,
    "enumMember" to Colors.CONSTANT,
    "property" to Colors.INSTANCE_FIELD,
    // iRule events (`when HTTP_REQUEST`) — a tag-like name, not a call.
    "event" to Colors.MARKUP_TAG,

    // Regular-expression interiors.
    "regexp" to Colors.STRING,
    "regexpGroup" to Colors.OPERATION_SIGN,
    "regexpCharClass" to Colors.CONSTANT,
    "regexpQuantifier" to Colors.OPERATION_SIGN,
    "regexpAnchor" to Colors.OPERATION_SIGN,
    "regexpEscape" to Colors.VALID_STRING_ESCAPE,
    "regexpBackref" to Colors.CONSTANT,
    "regexpAlternation" to Colors.OPERATION_SIGN,

    // `format` / `clock` / `binary` specifiers, all inside a string literal.
    "formatPercent" to Colors.VALID_STRING_ESCAPE,
    "formatSpec" to Colors.VALID_STRING_ESCAPE,
    "formatFlag" to Colors.OPERATION_SIGN,
    "formatWidth" to Colors.NUMBER,
    "clockPercent" to Colors.VALID_STRING_ESCAPE,
    "clockSpec" to Colors.VALID_STRING_ESCAPE,
    "clockModifier" to Colors.OPERATION_SIGN,
    "binarySpec" to Colors.VALID_STRING_ESCAPE,
    "binaryCount" to Colors.NUMBER,
    "binaryFlag" to Colors.OPERATION_SIGN,

    // APL — the iApp presentation sublanguage.
    "aplSection" to Colors.KEYWORD,
    "aplSectionName" to Colors.CLASS_NAME,
    "aplFieldType" to Colors.CLASS_REFERENCE,
    "aplFieldName" to Colors.INSTANCE_FIELD,
    "aplAttribute" to Colors.MARKUP_ATTRIBUTE,
    "aplDefine" to Colors.KEYWORD,
    "aplDefineName" to Colors.CONSTANT,
    "aplDirective" to Colors.METADATA,
    "aplOptional" to Colors.OPERATION_SIGN,
    "aplValidator" to Colors.STATIC_METHOD,

    // BIG-IP configuration objects and the values that name them.
    "partition" to Colors.CLASS_NAME,
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
 * type (`legend_token_modifiers`). Only the pairs that carry real meaning in
 * an IntelliJ colour scheme are listed; anything else falls through to the
 * type's own colour.
 */
private val SEMANTIC_TOKEN_MODIFIER_COLORS: Map<Pair<String, String>, TextAttributesKey> = mapOf(
    // A command head resolving to a registry built-in, versus a user proc.
    ("function" to "defaultLibrary") to Colors.PREDEFINED_SYMBOL,
    ("method" to "defaultLibrary") to Colors.PREDEFINED_SYMBOL,
    // The name token of a `proc`/`method` definition, versus a call site.
    ("function" to "definition") to Colors.FUNCTION_DECLARATION,
    ("method" to "definition") to Colors.FUNCTION_DECLARATION,
    // A variable the theme should not present as mutable.
    ("variable" to "readonly") to Colors.CONSTANT,
)

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
