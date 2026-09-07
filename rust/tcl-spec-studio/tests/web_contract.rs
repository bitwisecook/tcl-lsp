// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-level guards for the browser shell's editor and provenance contracts.

const HTML: &str = include_str!("../web/studio.html");
const STUDIO_TS: &str = include_str!("../web/src/studio.ts");
const EDITORS_TS: &str = include_str!("../web/src/editors.ts");
const MONACO_TS: &str = include_str!("../web/src/monacoHost.ts");
const NATIVE_HOST_TS: &str = include_str!("../web/src/nativeEditorHost.ts");
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
fn standalone_uses_only_monaco_and_ides_delegate_to_native_file_tabs() {
    assert!(STUDIO_TS.contains("Standalone and Pages use Monaco exclusively"));
    assert!(STUDIO_TS.contains("window.__tclSpecStudioHost !== undefined"));
    assert!(STUDIO_TS.contains("native-editor-controller"));
    assert!(BUILD_MJS.contains("nativeEditorHost.ts"));
    assert!(NATIVE_HOST_TS.contains("Open ${label} beside Studio"));
    assert!(NATIVE_HOST_TS.contains("using the IDE's native file editor beside Spec Studio"));
    assert!(!NATIVE_HOST_TS.contains("monaco"));
    assert!(!HTML.contains("__tclSpecStudioNativeModuleUrl"));
    assert!(HTML.contains(r#"id="dslText" hidden"#));
    assert!(HTML.contains(r#"id="testText" hidden"#));
    assert!(!STUDIO_TS.contains("dslEditor.js"));
    assert!(!STUDIO_TS.contains("addEventListener(\"input\", scheduleTest)"));
}

/// The IDE artefacts must not carry the standalone page's editor.
///
/// Both hosts inject `__tclSpecStudioHost`, so `mountEditorHost` always picks
/// the native controller and Monaco is dead weight there — about 3 MB, plus
/// ~21 MB for the browser LSP worker the native path never starts. The
/// standalone consumers (`GitHub Pages`, `tcl explore --serve`) still take the
/// whole dist, which is why this is a packaging rule rather than a build one,
/// and why `build-wasm.sh` must keep failing when Monaco is absent.
#[test]
fn the_ide_artefacts_exclude_the_standalone_editor() {
    let makefile = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Makefile"),
    )
    .expect("read the Makefile");

    // A recipe line can be continued with a trailing backslash, so join those
    // before matching or the excludes and the paths land on different lines.
    let joined = makefile.replace("\\\n", " ");
    let hosted: Vec<&str> = joined
        .lines()
        .filter(|line| line.contains("tcl-spec-studio-wasm/dist"))
        .filter(|line| line.contains("STAGE_DIR") || line.contains("JB_DIR"))
        .collect();
    assert_eq!(
        hosted.len(),
        2,
        "one staging copy each for the VSIX and the plugin: {hosted:#?}"
    );
    for line in hosted {
        assert!(
            line.contains("--exclude='assets/monaco-host.*'") && line.contains("--exclude='lsp/'"),
            "the hosted staging copy must exclude Monaco and the worker: {line}"
        );
    }

    // And the standalone build must keep requiring what it still loads.
    let build = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tcl-spec-studio-wasm/build-wasm.sh"),
    )
    .expect("read build-wasm.sh");
    assert!(
        build.contains("monaco-host.js"),
        "build-wasm.sh must still assert the standalone editor is present"
    );
}

#[test]
fn compiler_explorer_monaco_preserves_shortcuts_and_lsp_failures() {
    assert!(MONACO_TS.contains("monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter"));
    assert!(MONACO_TS.contains("if (ready) options.report?."));
    assert!(!MONACO_TS.contains(
        "options.report?.(\"using the shared Tcl Monaco editor\", \"ok\");\n  const ready"
    ));
}

#[test]
fn importing_a_pack_activates_a_renderable_command() {
    assert!(STUDIO_TS.contains("firstWritten ??= found.name"));
    assert!(
        STUDIO_TS.contains("if (view.pack) loadDraft(view.pack, packOrigin(view), firstWritten)")
    );
}

#[test]
fn selecting_a_registry_command_seeds_the_shared_pack_document() {
    // A selected built-in must not live only in the form/Rust renderer while
    // Pack DSL remains an unrelated empty document. The controller writes it
    // as a live override, reloads the one pack-backed draft, and protects that
    // draft from a later registry-list click overwriting the user's edits.
    assert!(
        STUDIO_TS.contains(
            "wasm.pack_set_command(state.pack.source, name, JSON.stringify(loaded), true)"
        )
    );
    assert!(STUDIO_TS.contains("setPackSource(written.source)"));
    assert!(STUDIO_TS.contains("loadDraft(view.pack, packOrigin(view), name)"));
    assert!(
        STUDIO_TS.contains("state.pack.view?.commands.some((command) => command.name === name)")
    );
    assert!(STUDIO_TS.contains("openPackCommand(name);\n    return;"));
}

#[test]
fn nested_option_metadata_has_inline_and_reference_help() {
    assert!(EDITORS_TS.contains(r#"ctx.fieldHelp("taints_var_write")"#));
    assert!(EDITORS_TS.contains("hasVarWriteRole"));
    assert!(EDITORS_TS.contains(r#"ctx.fieldHelp("variable_scope")"#));
    assert!(EDITORS_TS.contains(r#"ctx.fieldHelp("script_timing")"#));
    assert!(EDITORS_TS.contains(r#"ctx.fieldHelp("method_prefix_matching")"#));
    assert!(STUDIO_TS.contains("for (const field of schema.nestedFields)"));
    assert!(STUDIO_TS.contains("schema.nestedFields.find"));
}

#[test]
fn annotated_effect_arrows_keep_semantic_numbers_in_source_aligned_rows() {
    assert!(STUDIO_TS.contains("annotation, step: index + 1"));
    assert!(STUDIO_TS.contains(".sort((left, right) => left.at - right.at)"));
}

#[test]
fn every_pack_source_update_passes_through_the_shared_formatter() {
    assert!(STUDIO_TS.contains("wasm.format_pack(source)"));
    assert!(STUDIO_TS.contains("state.pack.source = formatted"));
    assert!(STUDIO_TS.contains("writeDsl(formatted)"));
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
