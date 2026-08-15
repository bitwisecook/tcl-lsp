// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-level guards for the browser shell's editor and provenance contracts.

const HTML: &str = include_str!("../web/studio.html");
const STUDIO_TS: &str = include_str!("../web/src/studio.ts");
const MONACO_TS: &str = include_str!("../web/src/monacoHost.ts");
const TEXTMATE_TS: &str = include_str!("../web/src/textmateHost.ts");
const BUILD_INFO_TS: &str = include_str!("../web/src/buildInfo.ts");
const BUILD_MJS: &str = include_str!("../web/build.mjs");

#[test]
fn every_code_pane_has_the_required_initial_language() {
    for id in ["dslEditor", "testEditor", "stubEditor"] {
        let marker = format!(
            r#"id="{id}" data-language="tcl" data-dialect="spectcl" data-tcl-version="9.0""#
        );
        assert!(HTML.contains(&marker), "{id} has the wrong initial profile");
    }
    assert!(HTML.contains(r#"id="rsEditor" data-language="rust""#));
    assert!(MONACO_TS.contains(r#"RUST_URI, "rust", null"#));
    assert!(MONACO_TS.contains(r#"STUB_URI, TCL_LANGUAGE, "spectcl""#));
    assert!(STUDIO_TS.contains(r#"dialect: "spectcl""#));
}

#[test]
fn monaco_preserves_the_servers_semantic_token_identifiers() {
    assert!(MONACO_TS.contains("tokenTypes: [...client.legend.tokenTypes]"));
    assert!(!MONACO_TS.contains("SCOPE_BY_TOKEN_TYPE"));
    assert!(MONACO_TS.contains(r"file:///__tcl_spec_studio__/pack.tclspec"));
    assert!(!MONACO_TS.contains("inmemory://studio"));
    assert!(MONACO_TS.contains("registerTclGrammar"));
    assert!(TEXTMATE_TS.contains("source.tcl"));
    assert!(TEXTMATE_TS.contains("vscode-textmate"));
}

#[test]
fn importing_a_pack_activates_a_renderable_command() {
    assert!(STUDIO_TS.contains("firstWritten ??= found.name"));
    assert!(
        STUDIO_TS.contains("if (view.pack) loadDraft(view.pack, packOrigin(view), firstWritten)")
    );
}

#[test]
fn every_pack_source_update_passes_through_the_shared_formatter() {
    assert!(STUDIO_TS.contains("wasm.format_pack(source)"));
    assert!(STUDIO_TS.contains("state.pack.source = formatted"));
    assert!(STUDIO_TS.contains("writeDsl(formatted, opts.fromDsl)"));
    assert!(STUDIO_TS.contains("mapSelectionThroughFormat(previous, source"));
}

#[test]
fn build_version_mismatch_has_a_one_shot_cache_break() {
    assert!(BUILD_MJS.contains(r#"["describe", "--tags", "--always", "--dirty"]"#));
    assert!(HTML.contains("__BUILD_INFO__"));
    assert!(BUILD_INFO_TS.contains("__SPEC_STUDIO_FRONTEND_VERSION__ !== info.version"));
    assert!(BUILD_INFO_TS.contains("window.location.replace"));
    assert!(BUILD_INFO_TS.contains("spec-studio-cache-bust"));
    assert!(BUILD_INFO_TS.contains("console.table"));
}
