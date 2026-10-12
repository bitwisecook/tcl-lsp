// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Whole original list operands select only existing namespace identities.

use super::{ModuleCommandBindings, SourceNamespaceKey};
use crate::signature_scan::scope::SignatureSourceNameInput;

/// All list elements must resolve before the caller installs a path.
/// This allocates no namespace and supplies no runtime token or storage.
pub(super) fn select(
    state: &ModuleCommandBindings,
    current: &SourceNamespaceKey,
    input: &SignatureSourceNameInput,
) -> Option<Vec<SourceNamespaceKey>> {
    // naming.namespace.original-counted-path-transfer
    // docs/design/analysis/name-resolution-proofs/namespace-original-counted-path-transfer.md
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
        return None;
    };
    let word = input.original_word_key()?;
    if version < tcl_dialect::TclVersion::V8_5
        || input.policy() != policy
        || state.has_opaque_domain()
        || state.baseline.unknown_entry
        || !state.namespaces.contains(current)
        || state.unknown_lookup_namespaces.contains(current)
        || !input.is_current(&state.source_variables)
        || state.current_source_origin.as_ref()?.source_image() != word.source_image()
    {
        return None;
    }
    let scope = state.original_namespace_geometry(current, policy)?;
    let elements =
        tcl_syntax::list::split_native_list_bytes(input.bytes(), policy.string_protocol()).ok()?;
    elements
        .iter()
        .map(|element| {
            let path = policy
                .recipe()
                .namespace_address_path(scope.context()?, element)
                .ok()?;
            match state.original_namespace_key_for_path(&path, policy) {
                super::namespace_slots::OriginalNamespaceKeyPresence::Available(key) => {
                    key.filter(|key| !state.unknown_lookup_namespaces.contains(key))
                }
                super::namespace_slots::OriginalNamespaceKeyPresence::Unavailable => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_dialect::TclVersion;

    fn analyse(source: &str, version: TclVersion) -> SourceCommandBindings {
        let context = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            context.commands(),
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn original_namespace_path_keeps_whole_lists_and_existing_identities() {
        // naming.namespace.original-counted-path-transfer
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-path-transfer.md
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let source = "namespace eval A {}; namespace eval B {}; namespace path {::B ::A ::B}";
            let bindings = analyse(source, version);
            let state = &bindings.final_state;
            let root = state.source_root_namespace_key().unwrap();
            assert!(
                !state.unknown_namespace_paths.contains(&root),
                "{version:?}"
            );
            let paths = state.namespace_paths.get(&root).unwrap();
            assert_eq!(paths.len(), 1, "{version:?}");
            let path = paths.iter().next().unwrap();
            assert_eq!(path.len(), 3, "{version:?}");
            let expected = [b"B".as_slice(), b"A".as_slice(), b"B".as_slice()];
            for (namespace, expected) in path.iter().zip(expected) {
                assert!(state.namespaces.contains(namespace));
                assert_eq!(
                    namespace.exact_native_path().unwrap().as_segments()[0].as_bytes(),
                    expected
                );
            }
            // The empty List is a reached clear operation, not a missing value.
            let cleared = analyse(
                "namespace eval A {}; namespace path {::A}; namespace path {}",
                version,
            );
            let state = &cleared.final_state;
            let root = state.source_root_namespace_key().unwrap();
            assert!(!state.unknown_namespace_paths.contains(&root));
            assert!(
                state
                    .namespace_paths
                    .get(&root)
                    .unwrap()
                    .iter()
                    .all(Vec::is_empty)
            );
        }
    }

    #[test]
    fn original_namespace_path_declines_missing_malformed_and_dynamic_elements() {
        // naming.namespace.original-counted-path-transfer
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-path-transfer.md
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for source in [
                "namespace eval A {}; namespace path {::A ::missing}",
                "namespace eval A {}; namespace path $unknown",
                "namespace eval A {}; namespace path \"{\"",
            ] {
                let bindings = analyse(source, version);
                let state = &bindings.final_state;
                let root = state.source_root_namespace_key().unwrap();
                assert!(
                    state.unknown_namespace_paths.contains(&root) || state.has_opaque_domain(),
                    "{version:?}: {source}"
                );
                assert!(!state.namespaces.iter().any(|namespace| {
                    namespace.exact_native_path().is_some_and(|path| {
                        path.as_segments()
                            .iter()
                            .any(|part| part.as_bytes() == b"missing")
                    })
                }));
            }
            let source = "namespace eval A {}; namespace path {::A}";
            let bindings = analyse(source, version);
            let config = bindings.lexer_config.unwrap();
            assert!(
                bindings.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config,
                )
            );
            assert!(!bindings.matches_original_source_image(
                &tcl_lexer::SourceImage::document("namespace path {::A}"),
                config,
            ));
        }
    }

    #[test]
    fn original_namespace_path_requires_current_original_policy_and_unique_incarnations() {
        // naming.namespace.original-counted-path-transfer
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-path-transfer.md
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        let source = "namespace eval A {}; namespace path {::A}";
        let bindings = analyse(source, TclVersion::V8_6);
        let state = &bindings.final_state;
        let root = state.source_root_namespace_key().unwrap();
        let config = bindings.lexer_config.unwrap();
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands.last().unwrap().words[2];
        let policy = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6)
            .authored_name_policy()
            .unwrap();
        let input = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        );
        let selected = super::select(state, &root, &input).unwrap();
        assert_eq!(selected.len(), 1);
        let super::super::SourceNamespaceKey::Allocated { site, path, .. } = &selected[0] else {
            panic!("actual source allocation");
        };
        let mut ambiguous = (**state).clone();
        std::sync::Arc::make_mut(&mut ambiguous.namespaces).insert(
            super::super::SourceNamespaceKey::Allocated {
                site: site.clone(),
                incarnation: super::super::AllocationIncarnation::Second,
                path: path.clone(),
            },
        );
        assert!(super::select(&ambiguous, &root, &input).is_none());
        let mut unknown = (**state).clone();
        std::sync::Arc::make_mut(&mut unknown.unknown_lookup_namespaces)
            .insert(selected[0].clone());
        assert!(super::select(&unknown, &root, &input).is_none());
        let other_policy = tcl_registry::InvocationDialect::for_version(TclVersion::V9_1)
            .authored_name_policy()
            .unwrap();
        let foreign = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                other_policy,
            )
            .unwrap(),
        );
        assert!(super::select(state, &root, &foreign).is_none());
        for profile in [
            tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
            crate::environment_ingress::resolve_environment("jim").unit_profile(),
        ] {
            let mut unsupported = (**state).clone();
            std::sync::Arc::make_mut(&mut unsupported.baseline).execution_name_policy =
                tcl_registry::InvocationDialect::of_profile(profile)
                    .authored_name_policy()
                    .map(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe);
            assert!(super::select(&unsupported, &root, &input).is_none());
        }
        let foreign_source = format!("{source}\n# changed");
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(&foreign_source),
            tcl_lexer::Span::new(0, u32::try_from(foreign_source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands.last().unwrap().words[2];
        let foreign = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        );
        assert!(super::select(state, &root, &foreign).is_none());
    }
}
