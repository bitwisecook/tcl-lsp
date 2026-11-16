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

//! Documents that are not Tcl keep their own token sets: iApp APL presentations and BIG-IP configuration.

use super::*;

/// APL (iApp presentation) is not Tcl: it must be tokenised by its own lexer,
/// not run through the Tcl pipeline.
///
/// Regression for the 2.1.6 state, where a `.apl` document fell through to the
/// Tcl tokeniser, which read each braced block as one literal word and emitted
/// whole *lines* as `string` tokens — mis-colouring the file rather than merely
/// under-colouring it.
#[test]
fn test_apl_presentation_uses_the_apl_token_set() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "#include \"f5.apl_common\"\n",
        "\n",
        "# a comment\n",
        "define choice yesno_choice {\n",
        "    \"Yes\" => \"yes\"\n",
        "}\n",
        "\n",
        "section basic {\n",
        "    string addr required validator \"IpAddress\"\n",
        "    message \"Welcome to the template.\"\n",
        "}\n",
    );
    let uri = unique_uri("apl");
    // Not `open_ready_lang`: an APL document is not indexed into workspace
    // state, so its `workspace_state.update` log never arrives. Await the
    // diagnostics publish instead — `semanticTokens/full` is ordered behind the
    // `didOpen` that produced it.
    lsp.open_document_lang(&uri, src, "tcl-apl", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    assert!(has("#include", "aplDirective"), "{tokens:?}");
    assert!(has("define", "aplDefine"), "{tokens:?}");
    assert!(has("choice", "aplFieldType"), "{tokens:?}");
    assert!(has("yesno_choice", "aplDefineName"), "{tokens:?}");
    assert!(has("section", "aplSection"), "{tokens:?}");
    assert!(has("basic", "aplSectionName"), "{tokens:?}");
    assert!(has("string", "aplFieldType"), "{tokens:?}");
    assert!(has("addr", "aplFieldName"), "{tokens:?}");
    assert!(has("required", "aplAttribute"), "{tokens:?}");
    assert!(has("validator", "aplAttribute"), "{tokens:?}");
    assert!(has("=>", "operator"), "{tokens:?}");
    assert!(has("# a comment", "comment"), "{tokens:?}");

    // The validator name is split out of its quotes rather than overlaid on
    // the string.
    assert!(has("IpAddress", "aplValidator"), "{tokens:?}");

    // A quoted value after a field-type keyword is a string, never the field's
    // name: a name group that matched non-whitespace greedily would swallow the
    // opening quote and emit `"Welcome` as a field name overlapping the string.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "aplFieldName" && covered(src, t).starts_with('"')),
        "a quoted value must not be typed as a field name: {tokens:?}",
    );

    // No whole-line `string` dumps: the Tcl fallback emitted one per line.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "string" && covered(src, t).trim_start().starts_with("string ")),
        "APL must not be tokenised as Tcl literal words: {tokens:?}",
    );

    // The LSP spec forbids overlapping tokens. 1.11.4 emitted 110 overlapping
    // pairs for `samples/apl/example.apl` by overlaying each specific type on a
    // generic one; that must never come back.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping APL tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}

/// BIG-IP config (`bigip.conf`, `.scf`) is not Tcl: it must be tokenised by its
/// own config lexer, not run through the Tcl pipeline.
///
/// Regression for the 2.1.6 state, where a `bigip.conf` fell through to the Tcl
/// tokeniser, which read each stanza body as one literal braced word — 272 of
/// the 302 tokens it produced for `samples/bigip/bigip.conf` were whole config
/// *lines* emitted as `string`.
#[test]
fn test_bigip_conf_uses_the_bigip_token_set() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "# a config comment\n",
        "auth partition Common {\n",
        "    description \"Default partition\"\n",
        "}\n",
        "auth user opsUser {\n",
        "    shell tmsh\n",
        "}\n",
        "ltm node /Common/web1 {\n",
        "    address 10.0.1.10\n",
        "}\n",
        "ltm pool /Common/web_pool {\n",
        "    members {\n",
        "        /Common/web1:80 {\n",
        "        }\n",
        "    }\n",
        "    monitor /Common/http\n",
        "}\n",
    );
    let uri = unique_uri("conf");
    lsp.open_document_lang(&uri, src, "tcl-bigip", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    assert!(has("# a config comment", "comment"), "{tokens:?}");
    assert!(has("ltm", "keyword"), "{tokens:?}");
    assert!(has("pool", "keyword"), "{tokens:?}");
    // A `/Common/x` path splits into its partition and its typed object name.
    assert!(has("Common", "partition"), "{tokens:?}");
    assert!(has("web_pool", "pool"), "{tokens:?}");
    assert!(has("web1", "object"), "{tokens:?}");
    assert!(has("http", "monitor"), "{tokens:?}");
    assert!(has("10.0.1.10", "ipAddress"), "{tokens:?}");
    assert!(has("80", "port"), "{tokens:?}");
    assert!(has("opsUser", "username"), "{tokens:?}");
    assert!(has("address", "property"), "{tokens:?}");

    // `auth partition Common` names a partition directly, not via a path.
    assert!(has("Common", "partition"), "{tokens:?}");

    // No whole-line `string` dumps — the Tcl fallback emitted one per line.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "string" && covered(src, t).trim_start().starts_with("members")),
        "BIG-IP config must not be tokenised as Tcl literal words: {tokens:?}",
    );

    // The LSP spec forbids overlapping tokens. 1.11.4 produced 65 overlapping
    // pairs for `bigip_base.conf` by overlaying each specific type on a generic
    // one; that must never come back.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping BIG-IP tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}
