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

//! Cross-file caller scans: file entries, loaded declarations and supplied source inputs.

use super::*;

#[test]
fn independent_caller_ingresses_keep_their_actual_sources_and_loader_identity() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Actual independent ingress remains available; changing source text
    // starts its own producer rather than querying a mismatched Module.
    let library = "proc helper {mode} {return $mode}";
    let entry = source_loader_entry(library);
    for value in ["FIRST", "SECOND"] {
        let source = format!("source library.tcl; helper {value}");
        let evidence = scan_loaded_caller(&source, &entry);
        let helper = evidence.get("::helper").expect("actual sourced caller");
        assert_eq!(helper.uniform_literal_at(0), Some(value));
        assert_eq!(helper.source_implementations.len(), 1);
        assert!(helper.source_implementations[0].matches_source_declaration(library, 0));
    }
}

#[test]
fn trusted_file_entry_retains_actual_loaded_declaration_identity() {
    let library = "proc helper {mode} {return $mode}";
    let entry = source_loader_entry(library);
    let evidence = scan_loaded_caller("source library.tcl; helper dev", &entry);
    let helper = evidence.get("::helper").expect("actual sourced caller");
    assert_eq!(helper.uniform_literal_at(0), Some("dev"));
    assert_eq!(helper.source_implementations.len(), 1);
    let allocation = &helper.source_implementations[0];
    assert!(allocation.matches_source_declaration(library, 0));
    assert!(!allocation.matches_source_declaration("proc helper {mode} {return WRONG}", 0));
    assert!(
        matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Loaded {path, ..} if path.as_ref() == "library.tcl")
    );
}

#[test]
fn file_entry_does_not_license_unloaded_shadowed_or_unknown_file_reads() {
    let entry = source_loader_entry("proc helper {mode} {return $mode}");
    for source in [
        "helper dev",
        "source $path; helper dev",
        "source other.tcl; helper dev",
        "proc source {path} {}; source library.tcl; helper dev",
    ] {
        let evidence = scan_loaded_caller(source, &entry);
        assert!(
            evidence
                .get("::helper")
                .is_none_or(|callee| callee.uniform_literal_at(0).is_none()),
            "{source}"
        );
    }
    let absent = crate::command_binding::SourceAnalysisEntry {
        trusted_source_modules: Vec::new(),
        ..entry
    };
    assert!(
        scan_loaded_caller("source library.tcl; helper dev", &absent)
            .get("::helper")
            .is_none_or(|callee| callee.uniform_literal_at(0).is_none())
    );
}

#[test]
fn sourced_partial_errors_and_redefinition_preserve_real_target_identity() {
    let partial =
        "proc helper {mode} {return $mode}; error BOOM; proc helper {mode} {return WRONG}";
    let entry = source_loader_entry(partial);
    let evidence = scan_loaded_caller("catch {source library.tcl}; helper dev", &entry);
    let helper = evidence
        .get("::helper")
        .expect("definition preceding error survives");
    assert_eq!(helper.uniform_literal_at(0), Some("dev"));
    assert!(
        helper
            .source_implementations
            .iter()
            .all(|allocation| allocation.matches_source_declaration(partial, 0))
    );
    let replaced = scan_loaded_caller(
        "source library.tcl; proc helper {mode} {return NEW}; helper dev",
        &source_loader_entry("proc helper {mode} {return OLD}"),
    );
    assert!(
        replaced
            .get("::helper")
            .expect("replacement caller")
            .source_implementations
            .iter()
            .all(|allocation| !allocation
                .matches_source_declaration("proc helper {mode} {return OLD}", 0))
    );
}

#[test]
fn supplied_native_caller_scan_retains_source_input_without_entry_or_frame_donation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context = std::sync::Arc::new(crate::environment_ingress::context_for_profile(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let registry = context.commands();
    // Unknown history is an explicit entry; with none supplied the
    // document runs in a fresh interpreter (user decision 2026-10-19T14:20Z).
    let entry = crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input);
    let source = "proc helper {} {}; $command unknown";
    let (module, cfg, _) = lower_source_callers(
        source,
        registry,
        None,
        profile,
        SourceCallSiteEntry {
            entry: Some(&entry),
            input: Some(&input),
        },
    );
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert!(
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &module)
            .is_some()
    );
    assert_eq!(
        cfg.top_level.metadata_context.source_analysis_input(),
        Some(&input)
    );
    assert!(module.source_entry.unknown_entry);
    assert!(module.source_entry.native_entry.is_none());
    assert!(
        module.procedures.is_empty(),
        "source cards cannot grant a Native body entry"
    );
    let evidence = scan_source_call_sites_with_source_input(
        source,
        None,
        &known(&["::helper"]),
        &["::helper".to_owned()],
        &input,
        &entry,
    );
    assert!(evidence.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&evidence, "::helper", 0), None);
}

#[test]
fn source_entry_caller_metadata_requires_an_independent_supplied_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Trusted source allocation remains independent of source metadata availability.
    let source = "source library.tcl; helper dev";
    let entry = source_loader_entry("proc helper {mode} {return $mode}");
    let current = scan_loaded_caller(source, &entry);
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("dev"));
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let missing = scan_source_call_sites_with_source_entry(
        source,
        &registry(),
        None,
        profile,
        &known(&["::helper"]),
        &["::helper".to_owned()],
        &entry,
    );
    assert_eq!(uniform(&missing, "::helper", 0), None);
    assert!(missing.get("::helper").unwrap().opaque_caller);
}
