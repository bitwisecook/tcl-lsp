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

use super::{
    IlxContext, IlxDocument, IlxPluginRoot, IlxSite, IlxTarget, IlxUnresolved, definition,
    is_extension_entry, method_call_at, references, references_from_registration, registration_at,
};
use crate::vfs::MemoryStore;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

const RULE: &str = concat!(
    "when HTTP_REQUEST {\n",
    "    ILX::call [ILX::init my_plugin my_extension] my_js_function [HTTP::uri]\n",
    "    ILX::notify [ILX::init my_plugin my_extension] my_js_function logged\n",
    "}\n",
);

const EXTENSION: &str = concat!(
    "var f5 = require('f5-nodejs');\n",
    "var ilx = new f5.ILXServer();\n",
    "ilx.addMethod('my_js_function', function (req, res) {\n",
    "    res.reply('ok');\n",
    "});\n",
    "ilx.listen();\n",
);

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules())
}

fn method_fixture_call(doc: IlxDocument<'_>, context: IlxContext<'_>) -> super::IlxMethodCall {
    let offset = u32::try_from(
        doc.text
            .find("my_js_function")
            .expect("literal method word"),
    )
    .unwrap();
    let position = tcl_lexer::LineIndex::new(doc.text).position_at_utf16(offset, doc.text);
    method_call_at(doc, context, position.line, position.character.get())
        .expect("original method word")
}

/// A store laid out the way BIG-IP lays an ILX workspace out.
fn workspace_store() -> MemoryStore {
    let store = MemoryStore::new();
    store.upsert("/w/my_plugin/rules/rule1.tcl", RULE.as_bytes().to_vec());
    store.upsert(
        "/w/my_plugin/extensions/my_extension/index.js",
        EXTENSION.as_bytes().to_vec(),
    );
    store
}

#[test]
fn a_literal_call_resolves_to_the_registration() {
    let store = workspace_store();
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let ctx = IlxContext::new(&registry, &store);
    // The `my_js_function` word of the `ILX::call` line.
    let call = method_fixture_call(doc, ctx);
    assert_eq!(call.method, "my_js_function");
    let IlxTarget::SourceCandidate(candidate) = definition(doc, ctx, &call) else {
        panic!("expected a readonly source registration candidate");
    };
    assert_eq!(call.target, None);
    assert_ne!(
        candidate.obligations,
        [] as [tcl_irules::ilx::IlxSourceObligation; 0]
    );
    let location = candidate.location;
    assert_eq!(
        location.path,
        Path::new("/w/my_plugin/extensions/my_extension/index.js")
    );
    assert_eq!(location.range.start_line, 2);
}

#[test]
fn references_reach_both_call_sites_and_the_registration() {
    let store = workspace_store();
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let ctx = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, ctx);
    let candidates = super::source_candidates(doc, ctx, &call);
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.obligations.is_empty())
    );
    let found = candidates
        .into_iter()
        .map(|candidate| candidate.location)
        .collect::<Vec<_>>();
    assert_eq!(
        references(doc, ctx, &call).len(),
        1,
        "source labels do not identify runtime handles"
    );
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(
        found
            .iter()
            .filter(|l| l.path.ends_with("index.js"))
            .count(),
        1
    );
    assert_eq!(
        found
            .iter()
            .filter(|l| l.path.ends_with("rule1.tcl"))
            .count(),
        2,
        "the ILX::call and the ILX::notify are both sites"
    );
    // The registration is the declaration, and is labelled as such so a
    // caller can drop it for `includeDeclaration: false`.
    assert_eq!(
        found
            .iter()
            .filter(|l| l.kind == IlxSite::Registration)
            .map(|l| l.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from(
            "/w/my_plugin/extensions/my_extension/index.js"
        )]
    );
}

#[test]
fn references_from_the_registration_reach_the_rules() {
    let store = workspace_store();
    let registry = registry();
    let js = Path::new("/w/my_plugin/extensions/my_extension/index.js");
    let doc = IlxDocument {
        path: js,
        text: EXTENSION,
    };
    let ctx = IlxContext::new(&registry, &store);
    let registration = registration_at(doc, 2, 16).expect("a registration under the cursor");
    assert_eq!(registration.name, "my_js_function");
    let found = super::source_candidates_from_registration(doc, ctx, &registration)
        .into_iter()
        .map(|candidate| candidate.location)
        .collect::<Vec<_>>();
    assert_eq!(
        references_from_registration(doc, ctx, &registration).len(),
        1
    );
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(is_extension_entry(js, ctx.files));
}

/// A plugin whose name is *not* its workspace directory: the layout alone
/// cannot associate the two, so the declaration is the only thing that can.
const RENAMED_RULE: &str = concat!(
    "when HTTP_REQUEST {\n",
    "    ILX::call [ILX::init prod_plugin my_extension] my_js_function [HTTP::uri]\n",
    "}\n",
);

/// The same workspace as [`workspace_store`], but under a directory whose
/// name is not the plugin's — `create ilx plugin prod_plugin
/// from-workspace ws_alpha`.
fn renamed_plugin_store() -> MemoryStore {
    let store = MemoryStore::new();
    store.upsert(
        "/w/ws_alpha/rules/rule1.tcl",
        RENAMED_RULE.as_bytes().to_vec(),
    );
    store.upsert(
        "/w/ws_alpha/extensions/my_extension/index.js",
        EXTENSION.as_bytes().to_vec(),
    );
    store
}

fn declared(plugin: &str, workspace: &str, extra: &[&str]) -> Vec<IlxPluginRoot> {
    vec![IlxPluginRoot {
        plugin: plugin.to_owned(),
        workspace: PathBuf::from(workspace),
        extra_rule_dirs: extra.iter().map(PathBuf::from).collect(),
    }]
}

#[test]
fn a_plugin_named_unlike_its_directory_needs_a_declaration() {
    let store = renamed_plugin_store();
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/ws_alpha/rules/rule1.tcl"),
        text: RENAMED_RULE,
    };
    // Undeclared: the directory-name convention has nothing to match, and
    // guessing from the extension name alone is what criterion 4 forbids.
    let bare = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, bare);
    assert!(matches!(
        definition(doc, bare, &call),
        IlxTarget::Unresolved(IlxUnresolved::ExtensionNotFound)
    ));
    // Declared: the user has said where `prod_plugin` lives.
    let roots = declared("prod_plugin", "/w/ws_alpha", &[]);
    let ctx = bare.with_plugins(&roots);
    let IlxTarget::SourceCandidate(candidate) = definition(doc, ctx, &call) else {
        panic!("expected the declared workspace to resolve");
    };
    assert_eq!(call.target, None);
    assert_ne!(
        candidate.obligations,
        [] as [tcl_irules::ilx::IlxSourceObligation; 0]
    );
    let location = candidate.location;
    assert_eq!(
        location.path,
        Path::new("/w/ws_alpha/extensions/my_extension/index.js")
    );
}

#[test]
fn a_declaration_replaces_the_convention_rather_than_competing_with_it() {
    // Both a directory literally named `my_plugin` *and* a declaration
    // pointing elsewhere. Treating the declaration as one more candidate
    // would make it an ambiguity — an answer strictly worse than the one
    // the user asked for — so it replaces the convention outright.
    let store = MemoryStore::new();
    store.upsert("/w/my_plugin/rules/rule1.tcl", RULE.as_bytes().to_vec());
    store.upsert(
        "/w/my_plugin/extensions/my_extension/index.js",
        EXTENSION.as_bytes().to_vec(),
    );
    store.upsert(
        "/w/ws_beta/extensions/my_extension/index.js",
        EXTENSION.as_bytes().to_vec(),
    );
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let roots = declared("my_plugin", "/w/ws_beta", &[]);
    let ctx = IlxContext::new(&registry, &store).with_plugins(&roots);
    let call = method_fixture_call(doc, ctx);
    let IlxTarget::SourceCandidate(candidate) = definition(doc, ctx, &call) else {
        panic!("expected the declared workspace to win outright");
    };
    assert_eq!(call.target, None);
    assert_ne!(
        candidate.obligations,
        [] as [tcl_irules::ilx::IlxSourceObligation; 0]
    );
    let location = candidate.location;
    assert_eq!(
        location.path,
        Path::new("/w/ws_beta/extensions/my_extension/index.js")
    );
}

#[test]
fn declared_rule_directories_are_searched_recursively() {
    // A repository that keeps its iRules outside the workspace it builds:
    // `rules/` holds nothing, and the caller is two directories down in a
    // declared tree.
    let store = MemoryStore::new();
    store.upsert(
        "/w/ws_alpha/extensions/my_extension/index.js",
        EXTENSION.as_bytes().to_vec(),
    );
    store.upsert(
        "/repo/irules/http/rule1.tcl",
        RENAMED_RULE.as_bytes().to_vec(),
    );
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/ws_alpha/extensions/my_extension/index.js"),
        text: EXTENSION,
    };
    let roots = declared("prod_plugin", "/w/ws_alpha", &["/repo/irules"]);
    let ctx = IlxContext::new(&registry, &store).with_plugins(&roots);
    let registration = registration_at(doc, 2, 16).expect("the addMethod name");
    let found = super::source_candidates_from_registration(doc, ctx, &registration)
        .into_iter()
        .map(|candidate| candidate.location)
        .collect::<Vec<_>>();
    assert_eq!(
        found
            .iter()
            .filter(|l| l.path == Path::new("/repo/irules/http/rule1.tcl"))
            .count(),
        1,
        "the declared tree's caller must be found: {found:?}"
    );
    // Undeclared, the same request finds only the registration itself —
    // the workspace's own `rules/` is empty and nothing else is searched.
    let bare = IlxContext::new(&registry, &store);
    assert_eq!(
        references_from_registration(doc, bare, &registration).len(),
        1
    );
}

#[test]
fn a_plugin_word_that_names_no_workspace_abstains() {
    let store = workspace_store();
    let registry = registry();
    let text = RULE.replace("my_plugin my_extension", "other_plugin my_extension");
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: &text,
    };
    let ctx = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, ctx);
    assert_eq!(
        definition(doc, ctx, &call),
        IlxTarget::Unresolved(IlxUnresolved::ExtensionNotFound)
    );
}

#[test]
fn a_dynamic_handle_abstains_with_a_reason() {
    let store = workspace_store();
    let registry = registry();
    let text = RULE.replace("ILX::init my_plugin", "ILX::init $plugin");
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: &text,
    };
    let ctx = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, ctx);
    assert_eq!(
        definition(doc, ctx, &call),
        IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic)
    );
}

#[test]
fn a_duplicate_registration_is_an_ambiguity_not_a_target() {
    let store = workspace_store();
    store.upsert(
        "/w/my_plugin/extensions/my_extension/index.js",
        format!("{EXTENSION}ilx.addMethod('my_js_function', other);\n").into_bytes(),
    );
    let registry = registry();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let ctx = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, ctx);
    assert!(
        matches!(definition(doc, ctx, &call), IlxTarget::Ambiguous { count, .. } if count == 2)
    );
}

#[test]
fn a_package_main_entry_point_is_followed() {
    let store = MemoryStore::new();
    store.upsert("/w/p/rules/r.tcl", RULE.as_bytes().to_vec());
    store.upsert(
        "/w/p/extensions/my_extension/package.json",
        br#"{"main": "lib/server.js"}"#.to_vec(),
    );
    store.upsert(
        "/w/p/extensions/my_extension/lib/server.js",
        EXTENSION.as_bytes().to_vec(),
    );
    let registry = registry();
    let text = RULE.replace("my_plugin", "p");
    let doc = IlxDocument {
        path: Path::new("/w/p/rules/r.tcl"),
        text: &text,
    };
    let ctx = IlxContext::new(&registry, &store);
    let call = method_fixture_call(doc, ctx);
    let IlxTarget::SourceCandidate(candidate) = definition(doc, ctx, &call) else {
        panic!("expected a readonly source registration candidate");
    };
    assert_eq!(call.target, None);
    assert_ne!(
        candidate.obligations,
        [] as [tcl_irules::ilx::IlxSourceObligation; 0]
    );
    let location = candidate.location;
    assert_eq!(
        location.path,
        Path::new("/w/p/extensions/my_extension/lib/server.js")
    );
    assert!(is_extension_entry(
        Path::new("/w/p/extensions/my_extension/lib/server.js"),
        ctx.files
    ));
}

/// An editor holding unsaved text for some files.
#[derive(Default)]
struct OpenBuffers(Vec<(PathBuf, Arc<str>)>);

impl OpenBuffers {
    fn holding(path: &str, text: &str) -> Self {
        Self(vec![(PathBuf::from(path), Arc::from(text))])
    }
}

impl super::OpenDocuments for OpenBuffers {
    fn text(&self, path: &Path) -> Option<Arc<str>> {
        self.0
            .iter()
            .find(|(held, _)| held == path)
            .map(|(_, text)| Arc::clone(text))
    }
}

/// An editor holding one open document together with its own analysis.
struct AnalysedBuffer {
    path: PathBuf,
    text: Arc<str>,
    analysis: Arc<tcl_compiler::analyser::AnalysisResult>,
}

impl super::OpenDocuments for AnalysedBuffer {
    fn text(&self, path: &Path) -> Option<Arc<str>> {
        (self.path == path).then(|| Arc::clone(&self.text))
    }

    fn analysis(&self, path: &Path) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        (self.path == path).then(|| Arc::clone(&self.analysis))
    }
}

/// A sibling rule is scanned under its own retained analysis: the
/// initiating document's analysis cannot describe another file, and a
/// sibling analysis of other text is refused rather than trusted.
#[test]
fn an_open_sibling_rule_is_scanned_under_its_own_analysis() {
    let profile = tcl_dialect::DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let analyse = |source: &str| {
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    let store = workspace_store();
    let sibling = "/w/my_plugin/rules/rule2.tcl";
    // Different bytes from the initiating rule, so its analysis cannot
    // describe the sibling by coincidence.
    let sibling_text = format!("# sibling\n{RULE}");
    store.upsert(sibling, sibling_text.as_bytes().to_vec());
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let own = analyse(RULE);
    let sibling_sites = |open: &dyn super::OpenDocuments| {
        let ctx = IlxContext::new(context.commands(), &store)
            .with_analysis(&own)
            .with_open_documents(open);
        let call = method_fixture_call(doc, ctx);
        super::source_candidates(doc, ctx, &call)
            .iter()
            .filter(|candidate| candidate.location.path.ends_with("rule2.tcl"))
            .count()
    };
    assert_eq!(
        sibling_sites(&OpenBuffers::holding(sibling, &sibling_text)),
        0,
        "the initiating analysis cannot describe a sibling"
    );
    let analysed = AnalysedBuffer {
        path: PathBuf::from(sibling),
        text: Arc::from(sibling_text.as_str()),
        analysis: Arc::new(analyse(&sibling_text)),
    };
    assert_eq!(sibling_sites(&analysed), 2, "both sibling sites");
    let stale = AnalysedBuffer {
        analysis: Arc::new(analyse("when HTTP_REQUEST {}\n")),
        ..analysed
    };
    assert_eq!(sibling_sites(&stale), 0, "a stale sibling analysis");
}

/// A closed sibling is reported by [`super::sibling_rule_files`], and once
/// the host reads and analyses it — answering with exactly that text and
/// that analysis — it is scanned under its own input. Without the host's
/// input a document route borrows nothing, even for byte-identical text.
#[test]
fn a_closed_sibling_rule_is_scanned_under_the_input_the_host_supplies() {
    let profile = tcl_dialect::DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let analyse = |source: &str| {
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    let store = workspace_store();
    let sibling = "/w/my_plugin/rules/rule2.tcl";
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let own = analyse(RULE);
    let closed_text = format!("# closed sibling\n{RULE}");
    store.upsert(sibling, closed_text.as_bytes().to_vec());
    let none = OpenBuffers::default();
    let ctx = IlxContext::new(context.commands(), &store)
        .with_analysis(&own)
        .with_open_documents(&none);
    let call = method_fixture_call(doc, ctx);
    assert_eq!(
        super::sibling_rule_files(doc, ctx, &call),
        vec![PathBuf::from(sibling)],
        "the closed sibling the request reads, and never the document itself"
    );

    let sibling_sites = |sibling_text: &str, open: &dyn super::OpenDocuments| {
        store.upsert(sibling, sibling_text.as_bytes().to_vec());
        let ctx = IlxContext::new(context.commands(), &store)
            .with_analysis(&own)
            .with_open_documents(open);
        let call = method_fixture_call(doc, ctx);
        super::source_candidates(doc, ctx, &call)
            .iter()
            .filter(|candidate| candidate.location.path.ends_with("rule2.tcl"))
            .count()
    };
    // Byte-identical to the initiating rule: its analysis still describes
    // another file, so nothing is borrowed from it.
    assert_eq!(
        sibling_sites(RULE, &none),
        0,
        "no borrowed initiating facts"
    );
    assert_eq!(sibling_sites(&closed_text, &none), 0, "no host input");
    let supplied = AnalysedBuffer {
        path: PathBuf::from(sibling),
        text: Arc::from(closed_text.as_str()),
        analysis: Arc::new(analyse(&closed_text)),
    };
    assert_eq!(
        sibling_sites(&closed_text, &supplied),
        2,
        "both sites, under the sibling's own input"
    );

    // The JavaScript end lists the same rule files from the entry point.
    let entry = Path::new("/w/my_plugin/extensions/my_extension/index.js");
    let js_ctx = IlxContext::new(context.commands(), &store);
    assert_eq!(
        super::sibling_rule_files_from_registration(entry, js_ctx),
        vec![
            PathBuf::from("/w/my_plugin/rules/rule1.tcl"),
            PathBuf::from(sibling)
        ],
    );
}

#[test]
fn an_unsaved_buffer_wins_over_the_file_on_disk() {
    // The reader is one seam, so proving it on the JavaScript half proves
    // it for the sibling rules too: a registration typed but not yet saved
    // must resolve, and one deleted in the editor must stop resolving,
    // even though the disk still says otherwise.
    let store = workspace_store();
    let registry = registry();
    let js = "/w/my_plugin/extensions/my_extension/index.js";
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };

    let typed = OpenBuffers::holding(
        js,
        "var f5 = require('f5-nodejs');\nvar ilx = new f5.ILXServer();\n\n\n\nilx.addMethod('my_js_function', cb);\n",
    );
    let ctx = IlxContext::new(&registry, &store).with_open_documents(&typed);
    let call = method_fixture_call(doc, ctx);
    let IlxTarget::SourceCandidate(candidate) = definition(doc, ctx, &call) else {
        panic!("the editor's copy of the extension must be what is read");
    };
    assert_eq!(call.target, None);
    let location = candidate.location;
    assert_eq!(
        location.range.start_line, 5,
        "the range must come from the unsaved text, not the saved file"
    );

    // The same seam in the other direction: an editor that has *removed*
    // the registration must stop resolving it, however stale the disk is.
    let emptied = OpenBuffers::holding(js, "var f5 = require('f5-nodejs');\n");
    let ctx = IlxContext::new(&registry, &store).with_open_documents(&emptied);
    assert_eq!(
        definition(doc, ctx, &call),
        IlxTarget::Unresolved(IlxUnresolved::MethodNotRegistered)
    );
}

#[test]
fn plain_tcl_finds_no_method_word() {
    // Criterion 5 at the provider seam: the same source, resolved against a
    // stock Tcl registry, has no ILX relation at all.
    let store = workspace_store();
    let registry = CommandRegistry::build_default();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let ctx = IlxContext::new(&registry, &store);
    assert!(method_call_at(doc, ctx, 2, 34).is_none());
}
#[test]
fn original_ilx_navigation_does_not_join_unresolved_handles_by_method_labels() {
    // Implementation contract: naming.consumer.original-ilx-method-source-candidates
    // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "when HTTP_REQUEST {ILX::call $left m; ILX::call $right m}";
    let store = MemoryStore::new();
    let doc = IlxDocument {
        path: Path::new("/w/rule.tcl"),
        text: source,
    };
    let context = IlxContext::new(registry, &store);
    let calls = tcl_irules::ilx_method_calls(source, registry);
    assert_eq!(calls.len(), 2);
    let refs = references(doc, context, &calls[0]);
    assert_eq!(refs.len(), 1);
    assert_eq!(
        refs[0],
        super::location(doc.path, doc.text, calls[0].method_span, IlxSite::Call)
    );
    let changed = format!("{source}\n# unrelated source change");
    assert_eq!(
        references(
            IlxDocument {
                text: &changed,
                ..doc
            },
            context,
            &calls[0]
        ),
        [] as [crate::ilx_navigation::IlxLocation; 0]
    );
    let mut forged = calls[0].clone();
    forged.target = Some(super::IlxExtension {
        plugin: "p".to_owned(),
        extension: "e".to_owned(),
    });
    assert_eq!(
        references(doc, context, &forged),
        [] as [crate::ilx_navigation::IlxLocation; 0]
    );
    assert!(matches!(
        definition(doc, context, &forged),
        IlxTarget::Unresolved(_)
    ));
}

#[test]
fn hosted_constructor_candidates_revalidate_the_retained_analysis_owner() {
    let source = "when HTTP_REQUEST {set h [ILX::init my_plugin my_extension]; ILX::call $h my_js_function; ILX::notify $h my_js_function}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
    let store = workspace_store();
    let registry = analysis.resolved_registry().unwrap();
    let doc = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: source,
    };
    let ctx = IlxContext::new(registry, &store).with_analysis(&analysis);
    let call = method_fixture_call(doc, ctx);
    assert_eq!(call.target, None);
    assert!(matches!(
        definition(doc, ctx, &call),
        IlxTarget::SourceCandidate(_)
    ));
    assert_eq!(super::source_candidates(doc, ctx, &call).len(), 2);
    assert_eq!(
        references(doc, ctx, &call).len(),
        1,
        "source candidates do not establish handle identity"
    );
    let fresh = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
    let foreign = IlxContext::new(fresh.resolved_registry().unwrap(), &store).with_analysis(&fresh);
    assert!(matches!(
        definition(doc, foreign, &call),
        IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic)
    ));
    assert_eq!(
        super::source_candidates(doc, foreign, &call),
        [] as [super::IlxSourceLocation; 0]
    );
    let mut forged = call.clone();
    forged.source_target.as_mut().unwrap().plugin = "other".to_owned();
    assert_eq!(
        super::source_candidates(doc, ctx, &forged),
        [] as [super::IlxSourceLocation; 0]
    );
    let calls = tcl_irules::ilx::ilx_method_calls_from_analysis(source, &analysis);
    assert_eq!(calls.len(), 2);
    assert_eq!(
        definition(doc, ctx, &calls[1]),
        IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic)
    );
    assert_eq!(
        super::source_candidates(doc, ctx, &calls[1]),
        [] as [super::IlxSourceLocation; 0]
    );
}

#[test]
fn original_ilx_source_navigation_keeps_candidate_paths_and_handle_identity_separate() {
    // Implementation contract: naming.consumer.original-ilx-method-source-candidates
    // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
    let registry = registry();
    let store = workspace_store();
    let context = IlxContext::new(&registry, &store);
    let document = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: RULE,
    };
    let call = method_fixture_call(document, context);
    let IlxTarget::SourceCandidate(candidate) = definition(document, context, &call) else {
        panic!("genuine inline source constructor should suggest a registration");
    };
    assert_eq!(call.target, None);
    assert_ne!(
        candidate.obligations,
        [] as [tcl_irules::ilx::IlxSourceObligation; 0]
    );
    assert!(
        super::hover_markdown(&call, &IlxTarget::SourceCandidate(candidate))
            .contains("evaluated handle result remain unresolved")
    );
    let mut forged = call.clone();
    forged.source_target.as_mut().unwrap().plugin = "other_plugin".to_owned();
    assert_eq!(
        super::source_candidates(document, context, &forged),
        [] as [crate::ilx_navigation::IlxSourceLocation; 0]
    );
    assert!(matches!(
        definition(document, context, &forged),
        IlxTarget::Unresolved(_)
    ));
    let changed = format!("{RULE}# whole image changed\n");
    assert_eq!(
        super::source_candidates(
            IlxDocument {
                text: &changed,
                ..document
            },
            context,
            &call
        ),
        [] as [crate::ilx_navigation::IlxSourceLocation; 0]
    );
    let source =
        "when HTTP_REQUEST {set h [ILX::init my_plugin my_extension]; ILX::call $h my_js_function}";
    let document = IlxDocument {
        text: source,
        ..document
    };
    let call = method_fixture_call(document, context);
    assert_eq!(call.target, None);
    assert_eq!(call.source_target, None);
    assert_eq!(
        super::source_candidates(document, context, &call),
        [] as [crate::ilx_navigation::IlxSourceLocation; 0]
    );
    assert_eq!(
        definition(document, context, &call),
        IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic)
    );
    assert_eq!(references(document, context, &call).len(), 1);
}

#[test]
fn original_ilx_analysis_ingress_keeps_configuration_and_never_recaptures_stale_source() {
    // Implementation contract: naming.consumer.original-ilx-method-source-candidates
    // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
    let source =
        "\u{feff}when HTTP_REQUEST {ILX::call [ILX::init my_plugin my_extension] my_js_function}";
    let profile = tcl_dialect::DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let analyse = |leading_bom| {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    let skipping = analyse(tcl_lexer::LeadingBom::Skip);
    let content_analysis = analyse(tcl_lexer::LeadingBom::Content);
    let store = workspace_store();
    let document = IlxDocument {
        path: Path::new("/w/my_plugin/rules/rule1.tcl"),
        text: source,
    };
    let owned = IlxContext::new(context.commands(), &store).with_analysis(&skipping);
    let retained: Vec<_> = skipping
        .original_vendor_source_names()
        .map(|row| (row.site().offset, row.original_words().len()))
        .collect();
    let source_cards =
        tcl_irules::OriginalIrulesSourceContext::from_source_analysis(source, &skipping)
            .expect("the exact supplied BOM-Skip source context remains current");
    let schemas: Vec<_> = source_cards
        .source_vectors()
        .iter()
        .map(|(span, words)| {
            (
                span.start(),
                span.end(),
                words.command(),
                words.arguments().len(),
            )
        })
        .collect();
    assert!(
        schemas
            .iter()
            .any(|(_, _, command, _)| *command == "ILX::call"),
        "actual method schema missing; original vectors={retained:?}, schemas={schemas:?}"
    );
    let call = method_fixture_call(document, owned);
    assert_eq!(call.target, None);
    assert!(matches!(
        definition(document, owned, &call),
        IlxTarget::SourceCandidate(_)
    ));
    let vectors =
        tcl_irules::OriginalIrulesSourceContext::from_source_analysis(source, &skipping).unwrap();
    assert!(
        vectors.events().is_empty() && vectors.commands().is_empty(),
        "source-only input issues no entered-event closure"
    );
    let different_config =
        IlxContext::new(context.commands(), &store).with_analysis(&content_analysis);
    assert_eq!(
        super::current_method_calls(document, different_config),
        [] as [tcl_irules::IlxMethodCall; 0]
    );
    assert_eq!(
        super::source_candidates(document, different_config, &call),
        [] as [crate::ilx_navigation::IlxSourceLocation; 0]
    );
    let changed = format!("{source}# changed");
    assert_eq!(
        super::current_method_calls(
            IlxDocument {
                text: &changed,
                ..document
            },
            owned
        ),
        [] as [tcl_irules::IlxMethodCall; 0]
    );
    let foreign_store = CommandRegistry::build_default();
    assert_eq!(
        super::current_method_calls(
            document,
            IlxContext::new(&foreign_store, &store).with_analysis(&skipping)
        ),
        [] as [tcl_irules::IlxMethodCall; 0]
    );
}

#[test]
fn original_ilx_hover_keeps_constructor_source_labels_separate_from_handles() {
    // Implementation contract: naming.consumer.original-ilx-method-source-candidates
    // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "when HTTP_REQUEST {ILX::call [ILX::init p e] m}";
    let calls = tcl_irules::ilx_method_calls(source, registry);
    assert_eq!(calls.len(), 1);
    let text = super::hover_markdown(
        &calls[0],
        &IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic),
    );
    assert!(
        text.contains("Possible source constructor"),
        "{text}; original source vectors: {:?}",
        tcl_irules::OriginalIrulesSourceContext::capture(source, registry).map(|context| {
            context
                .source_vectors()
                .iter()
                .map(|(span, words)| (*span, words.command().to_owned()))
                .collect::<Vec<_>>()
        })
    );
    assert!(text.contains("evaluated handle is unresolved"));
    assert!(text.contains("Written with"));
    assert_eq!(calls[0].target, None);
}
