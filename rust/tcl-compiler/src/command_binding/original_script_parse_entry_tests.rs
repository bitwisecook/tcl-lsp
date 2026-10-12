// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual Direct script parse entry, independent of recovered lexical prefix.

use super::*;
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeDirectScriptParseEntry,
};

fn original_direct(
    source: &str,
    environment: &str,
    unavailable_point: bool,
) -> (
    SourceCommandBindings,
    crate::analyser::ResolvedAnalysisInput,
    crate::environment_ingress::CapturedNativeEntry,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let mut owner = crate::environment_ingress::captured_native_entry(profile);
    assert_eq!(owner.execution_point, environment.point());
    if unavailable_point {
        owner.execution_point = None;
    }
    let entry = SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_entry: Some(Arc::new(owner.as_entry().clone())),
        native_compilation: NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            frame: NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        ..Default::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        entry.options(),
    );
    (bindings, input, owner)
}

#[test]
fn original_direct_parse_entry_uses_actual_engine_and_keeps_unknown_terminal() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Held interpreter entry supplies this physical policy; no external run claim.
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let (_, _, owner) = original_direct("", environment, false);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            ..Default::default()
        };
        let expected = if environment == "jimtcl" {
            NativeDirectScriptParseEntry::WholeScript
        } else {
            NativeDirectScriptParseEntry::CommandAtTime
        };
        assert_eq!(
            context.direct_script_parse_entry(owner.as_entry()),
            Some(expected)
        );
        for mode in [
            NativeCompilationMode::Unknown,
            NativeCompilationMode::BytecodeObject,
        ] {
            assert!(
                NativeCompilationContext { mode, ..context }
                    .direct_script_parse_entry(owner.as_entry())
                    .is_none()
            );
        }
        let mut missing = owner.as_entry().clone();
        missing.execution_point = None;
        assert!(context.direct_script_parse_entry(&missing).is_none());
        missing.execution_point = Some(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_83,
        ));
        assert!(context.direct_script_parse_entry(&missing).is_none());
    }
    let source = "proc selected {} {}; selected";
    let (missing, _, _owner) = original_direct(source, "jimtcl", true);
    assert!(missing.points.is_empty());
    assert!(!missing.final_state.procedure_bodies.contains("::selected"));
    assert!(missing.final_state.opaque_domain);
}

#[test]
fn original_jim_whole_script_parse_refuses_prefix_publication_but_keeps_valid_scripts() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let invalid = "proc selected {} {}; {a}b; selected";
    let (jim, input, _owner) = original_direct(invalid, "jimtcl", false);
    assert_eq!(jim.lexer_config, Some(input.lexer_config()));
    assert!(jim.points.is_empty());
    assert!(!jim.final_state.procedure_bodies.contains("::selected"));
    let image = tcl_lexer::SourceImage::document(invalid);
    let syntax = crate::segmenter::segment_script_image_with_offset_and_config(
        &image,
        0,
        input.lexer_config(),
    )
    .unwrap();
    assert_eq!(syntax.commands.len(), 1);
    assert!(syntax.fatal_tail.is_some());
    // A complete editor prefix is lexical syntax, not a Jim execution receipt.
    assert_eq!(syntax.commands[0].texts[0], "proc");
    let valid = "proc selected {} {}; selected";
    let (jim, _, _owner) = original_direct(valid, "jimtcl", false);
    assert!(!jim.points.is_empty());
    assert!(jim.final_state.procedure_bodies.contains("::selected"));
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (c, _, _owner) = original_direct(invalid, environment, false);
        assert!(
            c.final_state.procedure_bodies.contains("::selected"),
            "{environment}"
        );
        assert!(
            c.points.iter().any(|point| point.offset == 0),
            "{environment}"
        );
    }
}

#[test]
fn original_jim_nested_script_parse_stops_body_prefix_and_later_outer_commands() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // naming.source.jim-nested-direct-entry-and-prefix-effects
    // docs/design/analysis/name-resolution-proofs/source-jim-nested-direct-entry-and-prefix-effects.md
    // Fixed native probes bound physical Direct effects; this is a held-entry
    // software source-model control with independent input and binding guards.
    let source = "eval {proc inner {} {}; {a}b}; proc later {} {}";
    let (bindings, input, _owner) = original_direct(source, "jimtcl", false);
    assert_eq!(bindings.lexer_config, Some(input.lexer_config()));
    assert!(bindings.points.iter().any(|point| point.offset == 0));
    assert!(!bindings.final_state.procedure_bodies.contains("::inner"));
    assert!(!bindings.final_state.procedure_bodies.contains("::later"));
    let later = u32::try_from(source.find("proc later").unwrap()).unwrap();
    assert!(!bindings.points.iter().any(|point| point.offset == later));
    let body = "proc inner {} {}; {a}b";
    let image = tcl_lexer::SourceImage::document(body);
    assert!(
        tcl_lexer::first_parse_cut_image_checked(&image, input.lexer_config().nested())
            .unwrap()
            .is_some()
    );
}

#[test]
fn original_jim_command_child_parse_keeps_root_prefix_but_refuses_child_prefix() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // naming.source.jim-nested-direct-entry-and-prefix-effects
    // docs/design/analysis/name-resolution-proofs/source-jim-nested-direct-entry-and-prefix-effects.md
    // Fixed native probes bound physical Direct effects; this is a held-entry
    // software source-model control with independent input and binding guards.
    let source = "proc before {} {}; list [proc inner {} {}; list {a}b]; proc after {} {}";
    let (bindings, input, _owner) = original_direct(source, "jimtcl", false);
    assert_eq!(bindings.lexer_config, Some(input.lexer_config()));
    assert!(bindings.final_state.procedure_bodies.contains("::before"));
    assert!(!bindings.final_state.procedure_bodies.contains("::inner"));
    assert!(!bindings.final_state.procedure_bodies.contains("::after"));
    let after = u32::try_from(source.find("proc after").unwrap()).unwrap();
    assert!(!bindings.points.iter().any(|point| point.offset == after));
}

#[test]
fn original_jim_direct_delimiter_scan_does_not_borrow_c_bare_brace_syntax() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // naming.source.jim-nested-direct-entry-and-prefix-effects
    // docs/design/analysis/name-resolution-proofs/source-jim-nested-direct-entry-and-prefix-effects.md
    // Fixed native probes bound physical Direct effects; this is a held-entry
    // software source-model control with independent input and binding guards.
    // These held-entry software controls use the selected syntax owner;
    // separate fixed native probes record the physical Direct observations.
    for child in ["list {a", "list ${a", "list x{"] {
        let source = format!("proc before {{}} {{}}; list [{child}]; proc after {{}} {{}}");
        let (bindings, input, _owner) = original_direct(&source, "jimtcl", false);
        assert_eq!(bindings.lexer_config, Some(input.lexer_config()));
        assert!(bindings.points.is_empty(), "{child}");
        assert!(!bindings.final_state.procedure_bodies.contains("::before"));
        assert!(!bindings.final_state.procedure_bodies.contains("::after"));
    }
    let source = "proc before {} {}; list [list x{]; proc after {} {}";
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (bindings, _, _owner) = original_direct(source, environment, false);
        assert!(
            bindings.final_state.procedure_bodies.contains("::before"),
            "{environment}"
        );
        assert!(
            bindings.final_state.procedure_bodies.contains("::after"),
            "{environment}"
        );
    }
}
