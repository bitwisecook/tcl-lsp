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

//! The legend, its modifier order and the editor scope mappings that must cover it.

use super::*;

#[test]
fn typed_token_kinds_project_custom_legend_names_in_wire_order() {
    let legend = legend_token_types();
    assert_eq!(ALL_TOKEN_KINDS.len(), legend.len());
    for (index, kind) in ALL_TOKEN_KINDS.iter().enumerate() {
        assert_eq!(
            *kind as usize, index,
            "token kind and legend order diverged"
        );
    }
    let custom = legend_custom_token_types().unwrap();
    assert!(custom.contains(&"bigipInterface"));
    assert!(!custom.contains(&"interface"));
}

#[test]
fn legend_includes_regexp_and_event() {
    let types = legend_token_types();
    assert_eq!(types[TokenKind::Regexp as usize], "regexp");
    assert_eq!(types[TokenKind::Event as usize], "event");
}

#[test]
fn legend_has_expected_entries() {
    let types = legend_token_types();
    assert_eq!(types[TokenKind::Keyword as usize], "keyword");
    assert_eq!(types[TokenKind::Function as usize], "function");
    assert_eq!(types[TokenKind::Variable as usize], "variable");
    assert_eq!(types[TokenKind::String as usize], "string");
    assert_eq!(types[TokenKind::Number as usize], "number");
    assert_eq!(types[TokenKind::Comment as usize], "comment");
    assert_eq!(types[TokenKind::Namespace as usize], "namespace");
}

#[test]
fn legend_modifiers_order() {
    // Order is load-bearing: `defaultLibrary` must be bit index 3.
    let mods = legend_token_modifiers();
    assert_eq!(
        mods,
        vec!["declaration", "definition", "readonly", "defaultLibrary"]
    );
    assert_eq!(MOD_DEFAULT_LIBRARY, 1 << 3);
}

/// Every semantic-token type the server advertises in [`legend_token_types`]
/// must be handled by the VS Code extension — either a standard LSP type VS
/// Code themes natively, or an explicit `semanticTokenScopes` mapping. This
/// is the alignment guard: add a token type to the legend (e.g. from richer
/// lexing) without wiring the editor, and this test fails instead of the
/// token silently rendering as an unstyled default in every theme.
/// The BIG-IP config token types: emitted only by `bigip_conf_full` for
/// `tcl-bigip` documents (a `bigip.conf` is not Tcl), so the Tcl-family
/// blocks are not required to map them — `tcl-bigip` is.  `object` is
/// deliberately **not** in this set: it is shared, typing BIG-IP object
/// references inside iRules too, so the Tcl blocks must keep mapping it.
const BIGIP_ONLY: &[&str] = &[
    "partition",
    "pool",
    "monitor",
    "profile",
    "vlan",
    "bigipInterface",
    "ipAddress",
    "port",
    "routeDomain",
    "fqdn",
    "username",
    "encrypted",
];

/// Record a failure for every token in `required` that `mapped` (a
/// language's `semanticTokenScopes` block) does not handle.
fn require_mapped(
    lang: &str,
    required: &[&str],
    mapped: &std::collections::BTreeSet<&str>,
    failures: &mut Vec<String>,
) {
    for tok in required {
        if !mapped.contains(tok) {
            failures.push(format!(
                "language `{lang}` does not handle legend token `{tok}` \
                     (add it to contributes.semanticTokenScopes in \
                     editors/vscode/package.json)"
            ));
        }
    }
}

#[test]
fn vscode_semantic_token_scopes_cover_the_server_legend() {
    // Standard LSP `SemanticTokenTypes` VS Code styles out of the box, so
    // they need no custom `semanticTokenScopes` entry.
    const STANDARD_LSP_TYPES: &[&str] = &[
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "decorator",
    ];

    // The Tcl-family languages that exercise the *full* legend — iRules /
    // iApps / BIG-IP code references BIG-IP `object`s and fires `event`s, so
    // these blocks must map every non-standard token type. The narrower
    // dialects (`tcl8.4`, EDA, `expect`) emit a subset and the bespoke
    // `tcl-apl` uses its own token set, so they are not required to cover
    // the whole legend here. `tcl-irule` is a superset of plain Tcl, so
    // covering it covers every token a plain `.tcl` file can emit too.
    const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];

    // The `apl*` types are emitted *only* by `apl_full`, for `tcl-apl`
    // documents (APL is not Tcl). They are therefore not required of the
    // Tcl-family blocks above — but `tcl-apl` must map every one of them,
    // which is checked separately below.
    let apl_types: Vec<&str> = legend_token_types()
        .into_iter()
        .filter(|t| t.starts_with("apl"))
        .collect();
    assert!(!apl_types.is_empty(), "legend lost its apl* token types");
    for tok in BIGIP_ONLY {
        assert!(
            legend_token_types().contains(tok),
            "legend lost the BIG-IP token type `{tok}`"
        );
    }

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");

    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let legend = legend_token_types();
    let mut failures = Vec::new();
    let mut checked_blocks = 0;

    let mut checked_apl = false;
    let mut checked_bigip = false;

    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        let is_full_vocab = FULL_VOCAB.contains(&lang);
        if !is_full_vocab && lang != "tcl-apl" {
            continue;
        }
        let mapped: std::collections::BTreeSet<&str> = block["scopes"]
            .as_object()
            .map_or_default(|m| m.keys().map(String::as_str).collect());

        // `tcl-apl` owns the `apl*` types and nothing else needs them.
        if lang == "tcl-apl" {
            checked_apl = true;
            require_mapped(lang, &apl_types, &mapped, &mut failures);
            continue;
        }

        checked_blocks += 1;
        // `tcl-bigip` additionally owns the config-file types.
        if lang == "tcl-bigip" {
            checked_bigip = true;
            require_mapped(lang, BIGIP_ONLY, &mapped, &mut failures);
        }
        let shared: Vec<&str> = legend
            .iter()
            .copied()
            .filter(|t| {
                !STANDARD_LSP_TYPES.contains(t) && !apl_types.contains(t) && !BIGIP_ONLY.contains(t)
            })
            .collect();
        require_mapped(lang, &shared, &mapped, &mut failures);
    }

    assert!(
        checked_blocks > 0,
        "found no tcl* semanticTokenScopes blocks to check"
    );
    assert!(
        checked_apl,
        "found no `tcl-apl` semanticTokenScopes block to check the apl* types against"
    );
    assert!(
        checked_bigip,
        "found no `tcl-bigip` semanticTokenScopes block to check the BIG-IP types against"
    );
    assert!(
        failures.is_empty(),
        "semantic-token legend not fully handled:\n  {}",
        failures.join("\n  "),
    );
}

/// The *narrow* dialects — the versioned Tcls, the EDA tools, Expect — must
/// map the **shared** custom vocabulary too.
///
/// A `.exp` / EDA / `tcl8.4` document reaches exactly the same
/// regex / format / clock / binary / escape sub-tokenisers a plain `.tcl`
/// file does, so a type missing from *their* scope block renders unstyled
/// there while looking perfectly fine in `.tcl` — the failure mode this
/// whole test family exists to prevent, just one dialect over.
#[test]
fn vscode_semantic_token_scopes_cover_the_narrow_dialects() {
    const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];
    const STANDARD_LSP_TYPES: &[&str] = &[
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "decorator",
    ];

    // `object` names a BIG-IP object, which only iRules / iApps / BIG-IP
    // config reach — a plain Tcl or EDA document never emits it.
    let shared_custom: Vec<&str> = legend_token_types()
        .into_iter()
        .filter(|t| {
            !STANDARD_LSP_TYPES.contains(t)
                && !t.starts_with("apl")
                && !BIGIP_ONLY.contains(t)
                && *t != "object"
        })
        .collect();
    assert!(
        !shared_custom.is_empty(),
        "legend lost its shared vocabulary"
    );

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");
    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let mut failures = Vec::new();
    let mut checked = 0;
    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        if !lang.starts_with("tcl") || FULL_VOCAB.contains(&lang) || lang == "tcl-apl" {
            continue;
        }
        checked += 1;
        let mapped: std::collections::BTreeSet<&str> = block["scopes"]
            .as_object()
            .map_or_default(|m| m.keys().map(String::as_str).collect());
        require_mapped(lang, &shared_custom, &mapped, &mut failures);
    }
    assert!(
        checked > 0,
        "found no narrow tcl* semanticTokenScopes blocks (tcl8.4 / EDA / expect) to check"
    );
    assert!(
        failures.is_empty(),
        "narrow-dialect scope gaps:\n  {}",
        failures.join("\n  ")
    );

    assert!(
        failures.is_empty(),
        "semantic-token legend not fully handled:\n  {}",
        failures.join("\n  ")
    );
}

/// `set`, `lassign`, `incr`, `lappend`, `append`, `expr` (every plain
/// builtin — `function` + `defaultLibrary`) render as unstyled plain text
/// for users whose theme has no rule for the custom `support.function.tcl`
/// scope. A `semanticTokenScopes` override mapping
/// `function.defaultLibrary` to that scope **replaces** (not supplements)
/// VS Code's built-in cross-theme default for the standard
/// `function`/`defaultLibrary` combo — so themes lacking that exact scope
/// lose highlighting entirely instead of falling back to the built-in
/// default the way every other standard type does. `operator`, `decorator`
/// and `namespace` carry the same risk for the same reason (and
/// `operator`'s scope, `keyword.operator.format.tcl`, is outright wrong for
/// the general case — it covers every `expr` operator and the `regsub` `\&`
/// backref, not just `format`). Standard LSP types get no
/// override unless the override is either essentially universal across
/// themes (`number`, `regexp` — near-ubiquitous `TextMate` scopes that
/// match the grammar's own naming) or the type has no sane built-in
/// default at all (custom types like `object`, `event`, `escape`, the
/// `regexp*`/`format*`/`clock*`/`binary*` families).
#[test]
fn vscode_semantic_token_scopes_do_not_shadow_standard_defaults() {
    const MUST_NOT_OVERRIDE: &[&str] = &[
        "function.defaultLibrary",
        "operator",
        "decorator",
        "namespace",
    ];

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");

    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let mut failures = Vec::new();
    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        let Some(scopes) = block["scopes"].as_object() else {
            continue;
        };
        for &key in MUST_NOT_OVERRIDE {
            if scopes.contains_key(key) {
                failures.push(format!(
                    "language `{lang}` overrides `{key}`, shadowing VS Code's \
                         built-in cross-theme default (issue #862) — remove it from \
                         contributes.semanticTokenScopes in editors/vscode/package.json"
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n  "));
}
