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

//! Headless smoke test for the compiler-explorer web GUI (`gui/`).
//!
//! The GUI is shipped source (`index.html`, `explorer-core.js`, `worker.js`)
//! that no Rust test otherwise exercises, so a producer/consumer mismatch in
//! the explorer contract could only be caught by opening a browser. Issues
//! #1182 / #1183 were exactly that: the `(module)` header stopped carrying
//! `types`, `renderWasmModuleHeader` threw a `TypeError` mid-render, the WASM
//! tab stayed blank, and — because the throw skipped the rest of the message
//! handler — the compile spinner span forever.
//!
//! This test produces a real contract payload in-process (the same
//! `serialise_result` the WASM facade calls) and drives the shipped GUI in
//! headless Chromium via `tests/gui/explorer-gui-smoke.mjs`.
//!
//! Source goes in the way the shipped UI accepts it: Monaco owns the editor
//! and the `#source` textarea behind it is hidden state plumbing, so the
//! driver writes through the editor host's `onChange` bridge rather than
//! typing into an element no user can reach.
//!
//! It needs `node` plus a Playwright install; without them it prints why and
//! passes, so a plain `cargo test` on a machine with no browser stack is not
//! blocked. Point it at a non-default install with:
//!
//! * `TCL_EXPLORER_PLAYWRIGHT` — module specifier / path for `playwright`
//!   (default: `playwright`, resolved from the driver's own directory),
//! * `TCL_EXPLORER_CHROMIUM` — Chromium executable to launch,
//! * `TCL_EXPLORER_GUI_TEST=1` — fail instead of skipping when the toolchain
//!   is missing (set this in CI once the browser stack is installed).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const BUILD_RS: &str = include_str!("../build.rs");
const GUI_RS: &str = include_str!("../src/commands/gui.rs");

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// True when the harness must fail rather than skip on a missing toolchain.
fn strict() -> bool {
    std::env::var("TCL_EXPLORER_GUI_TEST").is_ok_and(|v| v != "0" && !v.is_empty())
}

/// Report a missing prerequisite: fail under `TCL_EXPLORER_GUI_TEST`, else
/// print and skip.
fn skip(reason: &str) {
    assert!(!strict(), "explorer GUI smoke test cannot run: {reason}");
    println!("skipping explorer GUI smoke test: {reason}");
}

/// Resolve the `playwright` module the driver should import, honouring
/// `TCL_EXPLORER_PLAYWRIGHT` and falling back to a global npm install (the
/// driver's own `node_modules` lookup handles a local install).
fn playwright_specifier() -> Option<String> {
    if let Ok(explicit) = std::env::var("TCL_EXPLORER_PLAYWRIGHT") {
        return Some(explicit);
    }
    let global_root = Command::new("npm")
        .args(["root", "-g"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let root = String::from_utf8(global_root.stdout).ok()?;
    let candidate = Path::new(root.trim()).join("playwright/index.mjs");
    candidate.is_file().then(|| candidate.display().to_string())
}

fn assert_editor_and_trait_reference(first: &Value) {
    // The native Web UI has one editor implementation, and its registry-owned
    // trait glossary renders without a JavaScript name/description table.
    assert_eq!(first["monacoMounted"], true, "Monaco did not mount");
    assert_eq!(
        first["stateEditorDisplay"], "none",
        "the textarea editor remained visible behind Monaco"
    );
    assert!(
        first["traitRows"].as_u64().unwrap_or(0) > 90,
        "the trait reference did not render every registry trait"
    );
    assert!(
        first["traitGroups"].as_u64().unwrap_or(0) > 1,
        "the trait reference was not grouped"
    );
    let trait_text = first["traitText"].as_str().unwrap_or_default();
    assert!(
        trait_text.contains("TAINT_SOURCE") && trait_text.contains("taint source"),
        "the registry's trait documentation is not visible: {trait_text}"
    );
}

#[test]
fn native_serve_requires_the_complete_monaco_bundle() {
    for asset in [
        "editor/build-info.json",
        "editor/assets/monaco-host.js",
        "editor/lsp/worker.js",
        "editor/lsp/tcl_lsp_server_wasm_bg.wasm",
    ] {
        assert!(BUILD_RS.contains(asset), "build completeness omits {asset}");
    }
    assert!(BUILD_RS.contains("GUI_HAS_EDITOR"));
    assert!(GUI_RS.contains("!assets::GUI_HAS_WASM || !assets::GUI_HAS_EDITOR"));
    assert!(GUI_RS.contains("make explorer-build && cargo build -p tcl-cli --release"));
    assert!(!GUI_RS.contains("make explorer-wasm && cargo build -p tcl-cli --release"));
}

/// The other half of #1183: an edit made while the WASM module is still
/// loading must not be dropped. The driver holds module load open (the stubbed
/// `wasm_bindgen` parks on a gate), enters the source through the Monaco host
/// — the only editor the shipped page shows — and only then lets the worker
/// report ready, so this is a state the test enters on purpose rather than a
/// timing window it has to win.
fn assert_compile_survives_module_load(report: &Value, first: &Value, source: &str) {
    assert_eq!(
        report["queuedDuringLoad"], true,
        "the GUI never queued a compile for the edit made during module load"
    );
    assert_eq!(
        report["compilesDuringLoad"], 0,
        "the worker compiled before it finished loading"
    );
    assert_eq!(
        first["compiledSource"], source,
        "the compile owed from module load never reached the compiler"
    );
}

fn assert_compile_triggers(report: &Value) {
    let before = report["compilesBeforeButton"]
        .as_u64()
        .expect("compile count");
    let after = report["compilesAfterButton"]
        .as_u64()
        .expect("compile count");
    assert_eq!(
        after,
        before + 1,
        "clicking Compile did not trigger a fresh compile ({before} -> {after})"
    );

    let before_shortcut = report["compilesBeforeShortcut"]
        .as_u64()
        .expect("shortcut compile count");
    let after_shortcut = report["compilesAfterShortcut"]
        .as_u64()
        .expect("shortcut compile count");
    assert_eq!(
        after_shortcut,
        before_shortcut + 1,
        "Ctrl/Cmd+Enter in Monaco did not force compilation ({before_shortcut} -> {after_shortcut})"
    );

    // Monaco is the only editor: an ordinary edit reaches the compiler only
    // through its `onChange` bridge into the hidden `#source` textarea.
    let before_edit = report["compilesBeforeEdit"]
        .as_u64()
        .expect("edit compile count");
    let after_edit = report["compilesAfterEdit"]
        .as_u64()
        .expect("edit compile count");
    assert_eq!(
        after_edit,
        before_edit + 1,
        "editing in Monaco did not recompile ({before_edit} -> {after_edit})"
    );
}

/// The codegen-pass toggles: rows built from `meta.semanticOptimisations`
/// before any compile lands, a tick that forces a recompile, and — the part a
/// label alone would fake — the selection actually reaching the compiler.
fn assert_codegen_pass_toggles(report: &Value) {
    let rows = report["passRowCount"].as_u64().expect("pass row count");
    assert_eq!(
        rows,
        tcl_explorer::SemanticOptimisationPassId::all().len() as u64,
        "the toggle panel did not render one row per pass"
    );
    assert_eq!(
        report["summaryBefore"].as_str().map(str::trim),
        Some("Codegen passes: none"),
        "an untouched Explorer must show the generic lowering"
    );

    let before = report["compilesBeforeToggle"]
        .as_u64()
        .expect("compile count");
    let after = report["compilesAfterToggle"]
        .as_u64()
        .expect("compile count");
    assert_eq!(
        after,
        before + 1,
        "ticking a pass did not force a recompile ({before} -> {after})"
    );
    assert_eq!(
        report["passesSent"].as_str(),
        Some(tcl_explorer::SemanticOptimisationPassId::all()[0].as_str()),
        "the ticked pass never reached the compiler"
    );
    assert_ne!(
        report["summaryAfter"].as_str().map(str::trim),
        Some("Codegen passes: none")
    );

    // The group buttons are presets over the same checkboxes.
    assert_eq!(
        report["nativeTierTicked"].as_u64(),
        Some(4),
        "the `native-tier` preset did not tick its four passes"
    );

    // And the answer to "does it live-update": the WASM pane must repaint
    // from the new result. A recompile the page never renders would satisfy
    // every assertion above and leave the user looking at the old module.
    let before = &report["wasmBeforeToggle"];
    let after = &report["wasmAfterToggle"];
    assert_ne!(
        before["wasmText"], after["wasmText"],
        "the WASM pane did not repaint after a pass was ticked"
    );
    assert!(
        after["wasmInstructions"].as_u64().unwrap_or(0) > 0,
        "the repainted WASM pane rendered no instructions: {after}"
    );
    assert_eq!(
        after["spinnerDisplay"], "none",
        "the spinner never settled after the toggle recompile"
    );
    assert!(
        after["errorBoxes"].as_array().is_some_and(Vec::is_empty),
        "the toggle recompile surfaced an error box: {after}"
    );
}

#[test]
fn gui_renders_the_wasm_tab_and_settles_the_spinner() {
    if Command::new("node").arg("--version").output().is_err() {
        skip("`node` is not on PATH");
        return;
    }
    let Some(playwright) = playwright_specifier() else {
        skip("playwright is not installed (npm i -g playwright)");
        return;
    };

    // The payload the WASM facade would return for this source.
    let source = "proc add {a b} {\n    return [expr {$a + $b}]\n}\nputs [add 1 2]\n";
    let result = tcl_explorer::run_pipeline(source, "tcl8.6");
    let payload = serde_json::to_string(&tcl_explorer::serialise_result(&result))
        .expect("explorer payload serialises");

    // The payload the same source produces with the codegen passes on. The
    // stub returns this one whenever a pass selection is sent, so the driver
    // can tell a re-*render* from a mere re-compile: without it the WASM pane
    // would look identical however the toggles were set and the test would
    // pass on a page that never repainted.
    let optimised = serde_json::to_string(&tcl_explorer::serialise_result_with_optimisations(
        &result,
        tcl_explorer::SemanticOptimisationConfig::from_names("all").expect("a valid group"),
    ))
    .expect("optimised explorer payload serialises");
    assert_ne!(
        payload, optimised,
        "the two payloads must differ or the render assertion proves nothing"
    );

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&out_dir).expect("create target tmp dir");
    let payload_path = out_dir.join("explorer-gui-payload.json");
    std::fs::write(&payload_path, &payload).expect("write payload");
    let optimised_path = out_dir.join("explorer-gui-payload-optimised.json");
    std::fs::write(&optimised_path, &optimised).expect("write optimised payload");

    let driver = manifest_dir().join("tests/gui/explorer-gui-smoke.mjs");
    let gui = manifest_dir().join("gui");
    let mut cmd = Command::new("node");
    cmd.arg(&driver)
        .arg(&gui)
        .arg(&payload_path)
        .arg(&optimised_path)
        .env("TCL_EXPLORER_PLAYWRIGHT", playwright);
    if let Ok(chromium) = std::env::var("TCL_EXPLORER_CHROMIUM") {
        cmd.env("TCL_EXPLORER_CHROMIUM", chromium);
    }
    let output = cmd.output().expect("spawn node driver");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        // A missing browser binary is an environment problem, not a
        // regression — skip unless the harness is running in strict mode.
        if !strict()
            && (stderr.contains("Executable doesn't exist")
                || stderr.contains("browserType.launch")
                || stderr.contains("ERR_MODULE_NOT_FOUND"))
        {
            skip(&format!("browser unavailable: {}", stderr.trim()));
            return;
        }
        panic!("explorer GUI driver failed:\n{stderr}");
    }

    let report: Value =
        serde_json::from_slice(&output.stdout).expect("driver prints a JSON report");
    let first = &report["first"];

    assert_eq!(
        report["pageErrors"].as_array().map(Vec::len),
        Some(0),
        "the GUI raised uncaught errors while rendering: {}",
        report["pageErrors"]
    );
    assert_eq!(
        first["errorBoxes"].as_array().map(Vec::len),
        Some(0),
        "a pane reported a render failure: {}",
        first["errorBoxes"]
    );

    assert_compile_survives_module_load(&report, first, source);

    // Issue #1183: the throbber must stop once a result is in.
    assert_eq!(
        first["spinnerDisplay"], "none",
        "the compile spinner never stopped"
    );
    assert_eq!(first["statusLight"], "status-light synced");

    // Issue #1182: the WASM tab must actually show a disassembly.
    assert_eq!(
        first["wasmModuleHeaders"], 1,
        "the (module) header did not render"
    );
    assert!(
        first["wasmFunctions"].as_u64().unwrap_or(0) >= 2,
        "expected the top level plus ::add in the WASM tab, got {}",
        first["wasmFunctions"]
    );
    assert!(
        first["wasmInstructions"].as_u64().unwrap_or(0) > 0,
        "the WASM tab rendered no instructions"
    );
    let wasm_text = first["wasmText"].as_str().unwrap_or_default();
    assert!(
        wasm_text.contains("(module)") && wasm_text.contains("types"),
        "the module header is missing its import/type summary: {wasm_text}"
    );
    // The Tcl ASM tab shares the renderer — keep it honest too.
    assert!(
        first["asmFunctions"].as_u64().unwrap_or(0) >= 2,
        "the Tcl ASM tab did not render"
    );

    // Issue #1183: the dropdown must list every dialect before the first
    // result, and the toolbar must offer an explicit Compile button.
    let dialects = first["dialects"].as_array().expect("dialect list");
    assert!(
        dialects.len() > 1 && dialects.iter().any(|d| d == "tcl9.0"),
        "the dialect dropdown was not populated: {dialects:?}"
    );
    assert_eq!(first["hasCompileButton"], true, "no Compile button");

    assert_editor_and_trait_reference(first);
    assert!(
        report["traitRowsBeforeCompile"].as_u64().unwrap_or(0) > 90,
        "the ready-message trait reference was empty before compilation"
    );

    assert_compile_triggers(&report);
    assert_codegen_pass_toggles(&report);
}
