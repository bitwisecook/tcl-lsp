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

//! Issue #1707 — an iRule's `ILX::call` method word and the Node.js
//! `ILXServer.addMethod` that implements it are one symbol, across two
//! languages.
//!
//! Every test here runs against a **real on-disk ILX workspace**, laid out the
//! way BIG-IP lays one out (verified against F5's tmsh `ilx workspace`
//! reference):
//!
//! ```text
//! <root>/<workspace>/rules/rule1.tcl
//! <root>/<workspace>/extensions/my_extension/index.js
//! ```
//!
//! The layout *is* the fixture: the association between an `ILX::init PLUGIN
//! EXTENSION` and a JavaScript file is derived from directory names, so a test
//! with two in-memory buffers would prove nothing about the thing that can
//! break.
//!
//! The JavaScript half is addressed as a **closed** document. Nothing needs it
//! open — the server reads a closed file through its source store — and our own
//! VS Code extension does not associate `.js` with the Tcl server, so a closed
//! URI is also the honest shape of a request an editor can make today.

use crate::common::Lsp;
use crate::common::helpers::{hover_text, locations};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The iRule, parameterised by the plugin name so concurrent runs (and any
/// stray `/tmp/<plugin>` directory) can never collide.
fn rule_source(plugin: &str) -> String {
    format!(
        "when HTTP_REQUEST {{\n\
         \x20   set handle [ILX::init {plugin} my_extension]\n\
         \x20   set reply [ILX::call $handle my_js_function [HTTP::uri]]\n\
         \x20   ILX::notify $handle my_js_function logged\n\
         }}\n"
    )
}

/// The extension's entry point, in the exact shape F5 documents.
const EXTENSION: &str = concat!(
    "var f5 = require('f5-nodejs');\n",
    "var ilx = new f5.ILXServer();\n",
    "ilx.addMethod('my_js_function', function (req, res) {\n",
    "    res.reply('ok');\n",
    "});\n",
    "ilx.listen();\n",
);

/// Line / column of the `my_js_function` word of the `ILX::call`.
const CALL_METHOD: (u32, u32) = (2, 34);
/// Line / column of the `my_js_function` word of the `ILX::notify`.
const NOTIFY_METHOD: (u32, u32) = (3, 24);
/// Line / column inside the `'my_js_function'` literal in the JavaScript.
const REGISTRATION: (u32, u32) = (2, 20);

/// One on-disk ILX workspace, plus the URIs of its two files.
struct Fixture {
    plugin: String,
    root: PathBuf,
    rule_uri: String,
    extension_uri: String,
}

impl Fixture {
    /// Lay out `<root>/<plugin>/{rules,extensions}` with `rule` and
    /// `extension` in it.  `label` keeps concurrent tests in their own tree.
    fn new(label: &str, extension_source: &str) -> Self {
        let plugin = format!("ilx_{label}_{}", std::process::id());
        let root = std::env::temp_dir().join(format!("tcl-lsp-e2e-1707-{plugin}"));
        let workspace = root.join(&plugin);
        let rules = workspace.join("rules");
        let extension_dir = workspace.join("extensions").join("my_extension");
        std::fs::create_dir_all(&rules).expect("mk rules dir");
        std::fs::create_dir_all(&extension_dir).expect("mk extension dir");
        let rule_path = rules.join("rule1.tcl");
        let extension_path = extension_dir.join("index.js");
        std::fs::write(&rule_path, rule_source(&plugin)).expect("write rule");
        std::fs::write(&extension_path, extension_source).expect("write extension");
        Self {
            plugin,
            rule_uri: uri_of(&rule_path),
            extension_uri: uri_of(&extension_path),
            root,
        }
    }

    /// A server rooted at this fixture, with the rule open as an iRule.
    fn serve(&self) -> Lsp {
        let mut lsp = Lsp::at_workspace_root(&self.root);
        lsp.open_ready_lang(&self.rule_uri, &rule_source(&self.plugin), "tcl-irule");
        lsp
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn uri_of(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

/// The `(line, character)` start of the first location in a result.
fn first_start(result: &Value) -> (i64, i64) {
    let found = locations(result);
    assert_eq!(found.len(), 1, "expected exactly one location: {found:?}");
    (
        found[0].range["start"]["line"].as_i64().expect("line"),
        found[0].range["start"]["character"]
            .as_i64()
            .expect("character"),
    )
}

#[test]
fn go_to_definition_crosses_into_the_extension_javascript() {
    let fixture = Fixture::new("def", EXTENSION);
    let mut lsp = fixture.serve();

    let result = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    let found = locations(&result);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, fixture.extension_uri, "{found:?}");
    // The `'my_js_function'` literal on the third line of the extension.
    assert_eq!(first_start(&result), (2, 14));

    // `ILX::notify` shares the method target.
    let notify = lsp.definition(&fixture.rule_uri, NOTIFY_METHOD.0, NOTIFY_METHOD.1);
    assert_eq!(locations(&notify)[0].uri, fixture.extension_uri);
}

#[test]
fn hover_names_the_extension_the_file_and_the_dispatch() {
    let fixture = Fixture::new("hover", EXTENSION);
    let mut lsp = fixture.serve();

    let call = hover_text(&lsp.hover(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert!(call.contains("my_js_function"), "{call}");
    assert!(call.contains("my_extension"), "{call}");
    assert!(call.contains(&fixture.plugin), "{call}");
    assert!(call.contains("index.js"), "{call}");
    assert!(
        call.contains("synchronous call"),
        "ILX::call is the blocking one: {call}"
    );

    let notify = hover_text(&lsp.hover(&fixture.rule_uri, NOTIFY_METHOD.0, NOTIFY_METHOD.1));
    assert!(
        notify.contains("best-effort notification"),
        "ILX::notify must stay distinguishable from ILX::call: {notify}"
    );
}

#[test]
fn references_span_both_languages_from_either_end() {
    let fixture = Fixture::new("refs", EXTENSION);
    let mut lsp = fixture.serve();

    // From the Tcl call site: the registration plus both call sites.
    let from_tcl =
        locations(&lsp.references(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1, true));
    assert_eq!(from_tcl.len(), 3, "{from_tcl:?}");
    assert_eq!(
        from_tcl
            .iter()
            .filter(|l| l.uri == fixture.extension_uri)
            .count(),
        1,
        "{from_tcl:?}"
    );
    assert_eq!(
        from_tcl
            .iter()
            .filter(|l| l.uri == fixture.rule_uri)
            .count(),
        2,
        "the ILX::call and the ILX::notify are both sites: {from_tcl:?}"
    );

    // From the JavaScript registration: the same set.
    let from_js =
        locations(&lsp.references(&fixture.extension_uri, REGISTRATION.0, REGISTRATION.1, true));
    assert_eq!(from_js.len(), 3, "{from_js:?}");
    assert_eq!(
        from_js.iter().filter(|l| l.uri == fixture.rule_uri).count(),
        2,
        "{from_js:?}"
    );

    // `includeDeclaration: false` drops the registration — it is this symbol's
    // declaration — from either end.
    for (uri, line, character) in [
        (&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1),
        (&fixture.extension_uri, REGISTRATION.0, REGISTRATION.1),
    ] {
        let uses = locations(&lsp.references(uri, line, character, false));
        assert_eq!(uses.len(), 2, "{uses:?}");
        assert!(
            uses.iter().all(|l| l.uri == fixture.rule_uri),
            "only the iRule call sites are uses: {uses:?}"
        );
    }
}

#[test]
fn a_dynamic_handle_abstains_instead_of_guessing() {
    let fixture = Fixture::new("dynamic", EXTENSION);
    let mut lsp = Lsp::at_workspace_root(&fixture.root);
    // Same rule, but the plugin name arrives in a variable — the association
    // is not statically knowable, so navigation must offer nothing.
    let source = rule_source(&fixture.plugin).replace(
        &format!("ILX::init {}", fixture.plugin),
        "ILX::init $plugin_name",
    );
    lsp.open_ready_lang(&fixture.rule_uri, &source, "tcl-irule");

    let result = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    assert!(
        locations(&result).is_empty(),
        "a dynamic handle must not resolve: {result}"
    );
    let hover = hover_text(&lsp.hover(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert!(
        hover.contains("Not resolved"),
        "hover must say why: {hover}"
    );
}

#[test]
fn a_duplicate_registration_is_reported_not_guessed() {
    let doubled = format!("{EXTENSION}ilx.addMethod('my_js_function', other);\n");
    let fixture = Fixture::new("dup", &doubled);
    let mut lsp = fixture.serve();

    let result = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    assert!(
        locations(&result).is_empty(),
        "two registrations are an ambiguity, not a target: {result}"
    );
    let hover = hover_text(&lsp.hover(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert!(hover.contains("2 times"), "{hover}");
}

#[test]
fn a_removed_method_resolves_to_nothing() {
    // The extension's *running* table has no `my_js_function` once
    // `removeMethod` has taken it out, so the earlier registration is not its
    // definition (issue #1707 review).
    let removed = format!("{EXTENSION}ilx.removeMethod('my_js_function');\n");
    let fixture = Fixture::new("removed", &removed);
    let mut lsp = fixture.serve();

    let result = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    assert!(
        locations(&result).is_empty(),
        "a removed method must not resolve: {result}"
    );
    let hover = hover_text(&lsp.hover(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert!(
        hover.contains("registers no method of this name"),
        "hover must report the empty table, not a target: {hover}"
    );
}

#[test]
fn a_handle_only_reaches_bodies_that_share_its_frame() {
    // Which bodies inherit the caller's frame is registry data
    // (`CommandSpec::body_kind`), and only those may inherit a handle binding
    // (issue #1707 review). A `proc` body runs in a fresh local frame, and so
    // does a `when` handler — `BodyKind::Structural` on both — so a handle
    // bound outside them is *undefined* where they run and must not resolve.
    // An `if` body inside the handler is the caller's own frame, and does.
    let fixture = Fixture::new("procscope", EXTENSION);
    let mut lsp = Lsp::at_workspace_root(&fixture.root);
    let source = format!(
        "set handle [ILX::init {plugin} my_extension]\n\
         proc helper {{}} {{\n\
         \x20   ILX::call $handle my_js_function\n\
         }}\n\
         when HTTP_REQUEST {{\n\
         \x20   ILX::call $handle my_js_function\n\
         }}\n\
         when CLIENT_ACCEPTED {{\n\
         \x20   set own [ILX::init {plugin} my_extension]\n\
         \x20   if {{1}} {{ ILX::call $own my_js_function }}\n\
         }}\n",
        plugin = fixture.plugin
    );
    lsp.open_ready_lang(&fixture.rule_uri, &source, "tcl-irule");

    for (label, line, character) in [("a proc body", 2, 22), ("an event handler body", 5, 22)] {
        let result = lsp.definition(&fixture.rule_uri, line, character);
        assert!(
            locations(&result).is_empty(),
            "{label} opens a new frame and must not inherit the handle: {result}"
        );
    }

    // The positive control: same file, a body that *is* the caller's frame.
    let inherited = lsp.definition(&fixture.rule_uri, 9, 30);
    assert_eq!(
        locations(&inherited)
            .iter()
            .map(|l| l.uri.clone())
            .collect::<Vec<_>>(),
        vec![fixture.extension_uri.clone()],
        "an `if` body shares the frame and still resolves"
    );
}

#[test]
fn an_unsaved_edit_to_the_extension_is_what_navigation_reads() {
    // The extension's JavaScript is another file, and the server reads other
    // files open-buffer-first — so an `addMethod` typed but not yet saved
    // resolves, and one deleted in the editor stops resolving, however stale
    // the bytes on disk are (issue #1707 review).
    let fixture = Fixture::new("unsaved", EXTENSION);
    let mut lsp = fixture.serve();
    lsp.open_ready_lang(
        &fixture.extension_uri,
        "var f5 = require('f5-nodejs');\nvar ilx = new f5.ILXServer();\n",
        "javascript",
    );

    let gone = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    assert!(
        locations(&gone).is_empty(),
        "the editor's copy no longer registers the method: {gone}"
    );

    // Type it back, three lines further down, and the jump follows the edit
    // rather than the file on disk.
    lsp.replace_document(
        &fixture.extension_uri,
        2,
        "var f5 = require('f5-nodejs');\nvar ilx = new f5.ILXServer();\n\n\n\nilx.addMethod('my_js_function', cb);\n",
    );
    let moved = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    let found = locations(&moved);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, fixture.extension_uri);
    assert_eq!(
        found[0].range["start"]["line"].as_i64(),
        Some(5),
        "the range comes from the unsaved text: {found:?}"
    );
}

#[test]
fn an_unregistered_method_resolves_to_nothing_and_no_diagnostic() {
    let fixture = Fixture::new("missing", "var f5 = require('f5-nodejs');\n");
    let mut lsp = fixture.serve();

    let result = lsp.definition(&fixture.rule_uri, CALL_METHOD.0, CALL_METHOD.1);
    assert!(locations(&result).is_empty(), "{result}");
    let diagnostics = lsp.pull_diagnostics(&fixture.rule_uri);
    let codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str))
        .collect();
    assert!(
        !codes.iter().any(|code| code.contains("ILX")),
        "an unknown method is not a diagnostic — the method table is only \
         known when the JavaScript is in the workspace: {codes:?}"
    );
}

#[test]
fn ordinary_tcl_named_ilx_call_is_untouched() {
    // Criterion 5, end to end: the same words in a plain Tcl document. The ILX
    // specs live on the iRules surface, so a Tcl registry holds no such
    // command and nothing here is an ILX site. Deliberately free of `when`
    // blocks, which the dialect detector reads as an iRule whatever the file
    // is called.
    let fixture = Fixture::new("plaintcl", EXTENSION);
    let plain = fixture
        .root
        .join(format!("{}/rules/plain.tcl", fixture.plugin));
    let source = format!(
        "proc ILX::init {{a b}} {{ return [list $a $b] }}\n\
         proc ILX::call {{h m}} {{ return $m }}\n\
         set handle [ILX::init {} my_extension]\n\
         set reply [ILX::call $handle my_js_function]\n",
        fixture.plugin
    );
    std::fs::write(&plain, &source).expect("write plain tcl");
    let uri = uri_of(&plain);
    let mut lsp = Lsp::at_workspace_root(&fixture.root);
    lsp.open_ready_lang(&uri, &source, "tcl");

    // The `my_js_function` word of line 3 is an ordinary argument here.
    let result = lsp.definition(&uri, 3, 34);
    let found = locations(&result);
    assert!(
        found.iter().all(|l| l.uri != fixture.extension_uri),
        "plain Tcl must not reach the extension: {found:?}"
    );
    let hover = hover_text(&lsp.hover(&uri, 3, 34));
    assert!(
        !hover.contains("iRulesLX method"),
        "plain Tcl must not get the ILX hover: {hover}"
    );
}

// ---------------------------------------------------------------------------
// The declared plugin ↔ workspace mapping, and the JavaScript-side route
// (issue #1707 criteria 2 and 3, the halves PR #1730 left open)
// ---------------------------------------------------------------------------

/// A workspace whose **directory name is not the plugin name** — the shape a
/// `create ilx plugin P from-workspace W` leaves behind, and the one the
/// directory-name convention cannot resolve — plus a caller kept outside the
/// workspace entirely, as a repository routinely does.
///
/// ```text
/// <root>/ws_alpha/rules/rule1.tcl
/// <root>/ws_alpha/extensions/my_extension/index.js
/// <root>/elsewhere/http/rule2.tcl
/// ```
struct DeclaredFixture {
    plugin: String,
    root: PathBuf,
    inside_uri: String,
    outside_uri: String,
    extension_uri: String,
}

impl DeclaredFixture {
    fn new(label: &str) -> Self {
        let plugin = format!("prod_{label}_{}", std::process::id());
        let root = std::env::temp_dir().join(format!("tcl-lsp-e2e-1707d-{plugin}"));
        let workspace = root.join("ws_alpha");
        let rules = workspace.join("rules");
        let extension_dir = workspace.join("extensions").join("my_extension");
        let outside = root.join("elsewhere").join("http");
        std::fs::create_dir_all(&rules).expect("mk rules dir");
        std::fs::create_dir_all(&extension_dir).expect("mk extension dir");
        std::fs::create_dir_all(&outside).expect("mk outside dir");
        let inside = rules.join("rule1.tcl");
        let outside_rule = outside.join("rule2.tcl");
        let extension_path = extension_dir.join("index.js");
        std::fs::write(&inside, rule_source(&plugin)).expect("write inside rule");
        std::fs::write(&outside_rule, rule_source(&plugin)).expect("write outside rule");
        std::fs::write(&extension_path, EXTENSION).expect("write extension");
        Self {
            plugin,
            inside_uri: uri_of(&inside),
            outside_uri: uri_of(&outside_rule),
            extension_uri: uri_of(&extension_path),
            root,
        }
    }

    /// The `tclLsp.iruleslx` settings a user writes to associate the plugin
    /// with its workspace, and to name where its callers live. Paths are
    /// folder-relative, exactly as they are in `.tcl-lsp.ini`.
    fn config(&self) -> Value {
        serde_json::json!({
            "iruleslx": {
                "plugins": { &self.plugin: "ws_alpha" },
                "rules": { &self.plugin: ["elsewhere"] },
            }
        })
    }

    fn serve(&self, config: Value) -> Lsp {
        let mut lsp = Lsp::with_config_at_root(config, &self.root);
        lsp.open_ready_lang(&self.inside_uri, &rule_source(&self.plugin), "tcl-irule");
        lsp
    }
}

impl Drop for DeclaredFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_declared_plugin_resolves_where_the_directory_name_cannot() {
    let fixture = DeclaredFixture::new("decl");

    // Unconfigured: nothing in the layout says `prod_…` is `ws_alpha`, and
    // matching on the extension name alone is the guess criterion 4 forbids —
    // so this is a definitive "no target", not a fall-through.
    let mut bare = fixture.serve(serde_json::json!({}));
    assert!(
        locations(&bare.definition(&fixture.inside_uri, CALL_METHOD.0, CALL_METHOD.1)).is_empty(),
        "the convention must not guess the workspace"
    );
    drop(bare);

    // Declared: the same request now crosses into the JavaScript.
    let mut lsp = fixture.serve(fixture.config());
    let found = locations(&lsp.definition(&fixture.inside_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, fixture.extension_uri, "{found:?}");
}

#[test]
fn declared_rule_directories_reach_a_caller_outside_the_workspace() {
    let fixture = DeclaredFixture::new("outside");
    let mut lsp = fixture.serve(fixture.config());

    let found = locations(&lsp.references(&fixture.inside_uri, CALL_METHOD.0, CALL_METHOD.1, true));
    // The registration, both sites in the open rule, and both sites in the
    // declared directory's rule — which no scan of `rules/` would ever see.
    assert_eq!(
        found
            .iter()
            .filter(|l| l.uri == fixture.outside_uri)
            .count(),
        2,
        "the declared directory's caller must be found: {found:?}"
    );
    assert_eq!(
        found
            .iter()
            .filter(|l| l.uri == fixture.extension_uri)
            .count(),
        1,
        "{found:?}"
    );
}

#[test]
fn the_javascript_end_answers_through_the_ilx_references_command() {
    let fixture = DeclaredFixture::new("jscmd");
    let mut lsp = fixture.serve(fixture.config());

    // What the VS Code client's second `ReferenceProvider` sends for a
    // JavaScript document the Tcl server does not own: the buffer travels with
    // the request, since the server holds no copy of it.
    let result = lsp.request(
        "workspace/executeCommand",
        serde_json::json!({
            "command": "tcl-lsp.ilxReferences",
            "arguments": [{
                "uri": fixture.extension_uri,
                "text": EXTENSION,
                "line": REGISTRATION.0,
                "character": REGISTRATION.1,
                "includeDeclaration": false,
            }],
        }),
    );
    let found = locations(&result);
    assert_eq!(found.len(), 4, "both rules, two sites each: {found:?}");
    assert!(
        found
            .iter()
            .all(|l| l.uri == fixture.inside_uri || l.uri == fixture.outside_uri),
        "`includeDeclaration: false` drops the registration itself: {found:?}"
    );

    // A position that is not on a registration name has no answer at all, so
    // the client falls back to the JavaScript language service rather than
    // showing an emphatic empty result.
    let elsewhere = lsp.request(
        "workspace/executeCommand",
        serde_json::json!({
            "command": "tcl-lsp.ilxReferences",
            "arguments": [{
                "uri": fixture.extension_uri,
                "text": EXTENSION,
                "line": 5,
                "character": 4,
            }],
        }),
    );
    assert!(elsewhere.is_null(), "{elsewhere:?}");
}

#[test]
fn the_ilx_references_command_ignores_an_ordinary_javascript_file() {
    let fixture = DeclaredFixture::new("plainjs");
    let mut lsp = fixture.serve(fixture.config());

    // Same content, but not an extension entry point: the server's own gate
    // refuses it even though the client's cheap pre-filter might not.
    let plain = fixture.root.join("tool.js");
    std::fs::write(&plain, EXTENSION).expect("write plain js");
    let result = lsp.request(
        "workspace/executeCommand",
        serde_json::json!({
            "command": "tcl-lsp.ilxReferences",
            "arguments": [{
                "uri": uri_of(&plain),
                "text": EXTENSION,
                "line": REGISTRATION.0,
                "character": REGISTRATION.1,
            }],
        }),
    );
    assert!(result.is_null(), "{result:?}");
}

/// A declaration that points **outside** the folder that made it — the shape a
/// `../shared/ws` mapping (or an absolute path, or a second workspace root)
/// takes.
///
/// ```text
/// <root>/app/rules/rule1.tcl                          <- the workspace folder
/// <root>/ws_alpha/rules/rule2.tcl                     <- outside it
/// <root>/ws_alpha/extensions/my_extension/index.js    <- outside it
/// ```
///
/// The JavaScript entry point is not under the configuring folder, so a
/// folder-scoped lookup keyed on its own URI finds no declaration and falls
/// back to the workspace's directory name — the very name the declaration
/// exists to correct.
struct OutsideFolderFixture {
    plugin: String,
    root: PathBuf,
    folder: PathBuf,
    inside_uri: String,
    workspace_rule_uri: String,
    extension_uri: String,
}

impl OutsideFolderFixture {
    fn new(label: &str) -> Self {
        let plugin = format!("prod_{label}_{}", std::process::id());
        let root = std::env::temp_dir().join(format!("tcl-lsp-e2e-1707o-{plugin}"));
        let folder = root.join("app");
        let workspace = root.join("ws_alpha");
        let folder_rules = folder.join("rules");
        let workspace_rules = workspace.join("rules");
        let extension_dir = workspace.join("extensions").join("my_extension");
        for dir in [&folder_rules, &workspace_rules, &extension_dir] {
            std::fs::create_dir_all(dir).expect("mk dir");
        }
        let inside = folder_rules.join("rule1.tcl");
        let workspace_rule = workspace_rules.join("rule2.tcl");
        let extension_path = extension_dir.join("index.js");
        std::fs::write(&inside, rule_source(&plugin)).expect("write folder rule");
        std::fs::write(&workspace_rule, rule_source(&plugin)).expect("write workspace rule");
        std::fs::write(&extension_path, EXTENSION).expect("write extension");
        Self {
            plugin,
            inside_uri: uri_of(&inside),
            workspace_rule_uri: uri_of(&workspace_rule),
            extension_uri: uri_of(&extension_path),
            folder,
            root,
        }
    }

    fn serve(&self) -> Lsp {
        let config = serde_json::json!({
            "iruleslx": { "plugins": { &self.plugin: "../ws_alpha" } }
        });
        let mut lsp = Lsp::with_config_at_root(config, &self.folder);
        lsp.open_ready_lang(&self.inside_uri, &rule_source(&self.plugin), "tcl-irule");
        lsp
    }
}

impl Drop for OutsideFolderFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_declaration_outside_its_folder_reaches_both_ends() {
    let fixture = OutsideFolderFixture::new("outsidefolder");
    let mut lsp = fixture.serve();

    // The Tcl end: the rule *is* under the configuring folder, so its own
    // folder's declarations describe it.
    let from_tcl = locations(&lsp.definition(&fixture.inside_uri, CALL_METHOD.0, CALL_METHOD.1));
    assert_eq!(from_tcl.len(), 1, "{from_tcl:?}");
    assert_eq!(from_tcl[0].uri, fixture.extension_uri, "{from_tcl:?}");

    // The JavaScript end: the entry point is *not* under that folder, so a
    // lookup keyed on its own URI sees no declaration and falls back to the
    // directory name `ws_alpha` — which no `ILX::init` in these rules writes.
    // Resolving against every declaration in the session is what keeps the two
    // directions symmetric.
    let from_js = locations(&lsp.references(
        &fixture.extension_uri,
        REGISTRATION.0,
        REGISTRATION.1,
        false,
    ));
    assert_eq!(
        from_js
            .iter()
            .filter(|l| l.uri == fixture.workspace_rule_uri)
            .count(),
        2,
        "the declared workspace's own rules/ must still be searched: {from_js:?}"
    );

    // …and through the command the VS Code client uses for an open buffer.
    let via_command = lsp.request(
        "workspace/executeCommand",
        serde_json::json!({
            "command": "tcl-lsp.ilxReferences",
            "arguments": [{
                "uri": fixture.extension_uri,
                "text": EXTENSION,
                "line": REGISTRATION.0,
                "character": REGISTRATION.1,
                "includeDeclaration": false,
            }],
        }),
    );
    let found = locations(&via_command);
    assert_eq!(
        found
            .iter()
            .filter(|l| l.uri == fixture.workspace_rule_uri)
            .count(),
        2,
        "{found:?}"
    );
}
