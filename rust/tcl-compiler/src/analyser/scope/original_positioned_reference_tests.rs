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

use crate::analyser::Analyser;

/// tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print `foo: bar` for this script:
/// the create installs `::e` and nothing else, so every later command
/// still resolves, and the ensemble looks up `::e::Foo` when `e foo` runs.
#[test]
fn literal_ensemble_creation_keeps_later_references_and_dispatches_to_its_target() {
    let source = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
    let at = |analysis: &crate::analyser::AnalysisResult, word: &str| {
        let offset = u32::try_from(source.rfind(word).unwrap()).unwrap();
        analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap()
            .resolved_command_reference
            .clone()
    };
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse(source, profile);
        assert!(at(&analysis, "proc").is_some(), "{profile}");
        let ensemble = at(&analysis, "e foo").expect("the created ensemble command");
        assert!(ensemble.definition().is_none(), "{profile}");
        let dispatch = at(&analysis, "foo bar").expect("the dispatch target receipt");
        let definition = dispatch.definition().expect("the target procedure");
        assert_eq!(
            analysis
                .proc_for_definition(definition, source)
                .unwrap()
                .qualified_name,
            "::e::Foo",
            "{profile}"
        );
    }
    // Tcl 8.4 has no `namespace ensemble`; the lenient profile spans it.
    for profile in ["tcl8.4", "tcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        assert!(at(&analysis, "e foo").is_none(), "{profile}");
    }
}

/// Deleting a namespace deletes every ensemble it created, wherever
/// `-command` put it (tclsh 8.5-9.1: `info commands ::top` and
/// `info commands ::e` are empty after `namespace delete`).
#[test]
fn deleting_an_ensemble_namespace_withdraws_its_command() {
    let resolves = |source: &str, head: &str| {
        let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
        Analyser::new()
            .analyse(source, "tcl8.6")
            .command_invocations
            .iter()
            .any(|invocation| {
                invocation.range.start() == offset
                    && invocation.resolved_command_reference.is_some()
            })
    };
    for (owner, create, head) in [("::c", "-command ::top", "top"), ("::e", "", "e")] {
        let created = format!("namespace eval {owner} {{namespace ensemble create {create}}}\n");
        let kept = format!("{created}{head}\n");
        assert!(resolves(&kept, head), "{kept}");
        let deleted = format!("{created}namespace delete {owner}\n{head}\n");
        assert!(!resolves(&deleted, head), "{deleted}");
    }
}

/// The same evidence: `-command` names the one created command, relative
/// to the current namespace or as written. A computed operand or an
/// abbreviated option cannot name it, so later references stay withdrawn.
#[test]
fn ensemble_creation_command_operand_selects_the_slot_or_withdraws() {
    let resolves = |source: &str, offset: usize| {
        let offset = u32::try_from(offset).unwrap();
        Analyser::new()
            .analyse(source, "tcl8.6")
            .command_invocations
            .iter()
            .any(|invocation| {
                invocation.range.start() == offset
                    && invocation.resolved_command_reference.is_some()
            })
    };
    let call = |source: &str| source.find("\np\n").unwrap() + 1;
    for (create, head) in [
        ("-command x -map {a ::b}", "::e::x"),
        ("-command ::top", "top"),
    ] {
        let source = format!(
            "namespace eval ::e {{namespace ensemble create {create}}}\nproc p {{}} {{}}\np\n{head}\n"
        );
        assert!(resolves(&source, call(&source)), "{source}");
        assert!(resolves(&source, source.rfind(head).unwrap()), "{source}");
    }
    for create in ["-command [lindex $::argv 0]", "-com x"] {
        let source = format!(
            "namespace eval ::e {{namespace ensemble create {create}}}\nproc p {{}} {{}}\np\n"
        );
        assert!(!resolves(&source, call(&source)), "{source}");
    }
}

/// evidence/tcl/ensemble-call-dispatches-its-target.md: a literal
/// ensemble's call runs its selected target and nothing else, a miss
/// without `-unknown` only raises, and the world after the call is the
/// target's. `configure`, a computed subcommand and `-unknown` on a miss
/// leave the call unknown.
#[test]
fn literal_ensemble_call_keeps_the_command_world_of_its_target() {
    let later_resolves = |calls: &str| {
        let source = format!(
            "namespace eval ::e {{namespace ensemble create -map {{foo ::e::Foo}}}}\nproc ::e::Foo {{args}} {{proc ::made {{}} {{}}}}\nproc p {{}} {{}}\n{calls}\np\nmade\n"
        );
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let resolves = |offset: usize| {
            let offset = u32::try_from(offset).unwrap();
            analysis.command_invocations.iter().any(|invocation| {
                invocation.range.start() == offset
                    && invocation.resolved_command_reference.is_some()
            })
        };
        let p = source.rfind("\np\n").unwrap() + 1;
        (resolves(p), resolves(p + 2))
    };
    assert_eq!(later_resolves("e foo bar"), (true, true));
    assert_eq!(later_resolves("e fo bar"), (true, true));
    assert_eq!(later_resolves("catch {e zz}\ne foo"), (true, true));
    for unknown in [
        "e [lindex $::argv 0]",
        "namespace ensemble configure ::e -map {foo ::q}\ne foo",
    ] {
        assert_eq!(later_resolves(unknown), (false, false), "{unknown}");
    }
    let source = "namespace eval ::u {namespace ensemble create -map {go ::u::go} -unknown ::h}\nproc p {} {}\ncatch {u zz}\np\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let p = u32::try_from(source.rfind("\np\n").unwrap() + 1).unwrap();
    assert!(analysis.command_invocations.iter().any(|invocation| {
        invocation.range.start() == p && invocation.resolved_command_reference.is_none()
    }));
}

#[test]
fn opaque_head_reference_uses_original_slot_and_allocation_after_ui_maps_are_cleared() {
    let source =
        r"proc p\uD800 {} {return first}; proc p\uD801 {} {return second}; p\uD800; p\uD801";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let mut slots = Vec::new();
    for spelling in [r"p\uD800", r"p\uD801"] {
        let offset = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap();
        let reference = invocation
            .resolved_command_reference
            .as_ref()
            .expect("original post-argv call receipt");
        let declaration = reference
            .definition()
            .expect("original procedure allocation");
        let metadata = analysis.proc_for_definition(declaration, source).unwrap();
        slots.push(reference.original_slot().unwrap().clone());
        assert_eq!(
            metadata.name_span,
            analysis
                .original_procedure_metadata
                .iter()
                .find(|record| record.declaration_site() == &declaration.allocation().site)
                .unwrap()
                .metadata()
                .name_span
        );
    }
    assert_ne!(slots[0], slots[1]);
    analysis.all_procs.clear();
    for spelling in [r"p\uD800", r"p\uD801"] {
        let offset = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap();
        let reference = invocation.resolved_command_reference.as_ref().unwrap();
        assert!(
            analysis
                .proc_for_definition(reference.definition().unwrap(), source)
                .is_some()
        );
    }
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn original_trace_references_use_selected_subject_and_preserve_source_currency() {
    use crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver;
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject;
    let source = "set ::v\\uD800 1\ntrace info variable ::v\\uD800(k)\n";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.rfind("::v").unwrap() + 3).unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let reference = analysis
            .original_variable_symbol_in_source(&image, config, offset)
            .unwrap_or_else(|| panic!("{profile}: original trace subject"));
        assert_eq!(
            reference.receiver(),
            OriginalVariableSymbolReceiver::Operand(TraceSubject)
        );
        assert!(!reference.is_declaration());
        let declaration = analysis
            .original_variable_symbols
            .iter()
            .find(|row| row.is_declaration())
            .unwrap();
        assert_eq!(reference.symbol(), declaration.symbol());
        assert_eq!(
            reference.original_name_input().bytes(),
            b"::v\xed\xa0\x80(k)"
        );
        assert!(
            !analysis
                .original_variable_write_advice
                .iter()
                .any(|row| row.span() == reference.span())
        );
        assert!(
            analysis
                .original_variable_symbol_in_source(
                    &tcl_lexer::SourceImage::document(&source.replace(" 1", " 2")),
                    config,
                    offset
                )
                .is_none()
        );
    }
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn original_trace_reference_aliases_and_unknown_scope_remain_independent() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject;
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let source = "set ::shared 1\nproc p {} {global shared; trace info variable shared}\np\n";
        let analysis = Analyser::new().analyse(source, profile);
        let trace = analysis
            .original_variable_symbols
            .iter()
            .find(|row| row.receiver() == OriginalVariableSymbolReceiver::Operand(TraceSubject))
            .unwrap_or_else(|| panic!("{profile}: exact linked trace subject"));
        assert!(!trace.is_declaration());
        assert!(
            matches!(trace.symbol().slot(), SignatureSourceVariableSlot::C { namespace, simple } if namespace.is_root() && simple.as_bytes() == b"shared")
        );
        let unknown = "set auto_index(unknown_command) {source lib.tcl}; unknown_command; namespace eval N {trace info variable bare}; trace info variable ::qualified";
        let analysis = Analyser::new().analyse(unknown, profile);
        assert!(
            !analysis
                .original_variable_symbols
                .iter()
                .any(|row| row.original_name_input().bytes() == b"bare")
        );
        let absolute = analysis
            .original_variable_symbols
            .iter()
            .find(|row| row.original_name_input().bytes() == b"::qualified")
            .unwrap();
        assert!(!analysis.original_variable_rename_is_complete(absolute.symbol()));
    }
    for source in ["trace info command ::v", "trace info execution ::v"] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !analysis
                .original_variable_symbols
                .iter()
                .any(|row| row.receiver() == OriginalVariableSymbolReceiver::Operand(TraceSubject))
        );
    }
}
