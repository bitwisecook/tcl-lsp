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

//! E006: an invalid formal parameter list.

use super::*;

#[test]
fn e007_needs_the_actual_store_and_the_same_initialised_constant() {
    let source = "const fixed 3\nset fixed\nset fixed 4\n";
    for profile in ["tcl9.0", "tcl9.1"] {
        let mut analyser = crate::analyser::Analyser::new();
        let result = analyser.analyse(source, profile);
        let matches = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::E007)
            .collect::<Vec<_>>();
        let [diagnostic] = matches.as_slice() else {
            panic!("expected one constant write for {profile}: {matches:?}");
        };
        assert_eq!(
            diagnostic.span.start() as usize,
            source.rfind("fixed").unwrap()
        );
        assert_eq!(&source[diagnostic.span.as_range()], "fixed");
    }
    assert!(!has_code(source, "tcl8.6", "E007"));
    assert!(has_code(source, "tcl8.6", "W002"));
    // While the stock channel proof holds no channel carries a Tcl handler,
    // so host I/O runs none: tclsh 9.1.0 rejects each of these final stores.
    for source in [
        "const fixed 3; puts -nonewline stdout ready; set fixed 4",
        "const fixed 3; chan put stdout ready; set fixed 4",
        "const fixed 3; chan puts stdout ready; set fixed 4",
        "const fixed 3; read stdin; set fixed 4",
        "const fixed 3; catch {puts -nonewline stdout ready}; set fixed 4",
    ] {
        assert!(
            has_code(source, "tcl9.1", "E007"),
            "missing E007 for {source:?}"
        );
    }
    for source in [
        "const fixed 3; exec true >@stdout; set fixed 4",
        "const fixed 3; update; set fixed 4",
        "const fixed 3; update idletasks; set fixed 4",
        "const fixed 3; vwait ready; set fixed 4",
        "const fixed 3; proc cmp args {rename set savedSet; proc set args {return 4}; return 0}; lsort -command cmp {1 2}; set fixed 4",
        "const fixed 3; rename set oldSet; proc set args {}; set fixed 4",
        "namespace eval Box {const fixed 3}; namespace delete Box; namespace eval Box {set fixed 4}",
    ] {
        assert!(
            !has_code(source, "tcl9.1", "E007"),
            "unexpected E007 for {source:?}"
        );
    }
}

const GLOBAL_AFTER_STDOUT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e05-global.tcl"
));
const CONSTANT_AFTER_STDOUT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e20-const-90.tcl"
));

#[test]
fn stock_process_stdout_preserves_original_variable_diagnostics() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analyser = crate::analyser::Analyser::new();
        let result = analyser.analyse(GLOBAL_AFTER_STDOUT, profile);
        let reads = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        let [read] = reads.as_slice() else {
            panic!("expected one missing read for {profile}: {reads:?}");
        };
        assert_eq!(&GLOBAL_AFTER_STDOUT[read.span.as_range()], "nope");
    }
    for profile in ["tcl9.0", "tcl9.1"] {
        assert!(has_code(CONSTANT_AFTER_STDOUT, profile, "E007"));
        assert!(has_code(
            "puts ready; const fixed 3; set fixed 4",
            profile,
            "E007"
        ));
    }
    assert!(has_code(CONSTANT_AFTER_STDOUT, "tcl8.6", "W002"));
    assert!(!has_code(CONSTANT_AFTER_STDOUT, "tcl8.6", "E007"));
}

#[test]
fn stock_process_stdout_is_not_reissued_for_supplied_unknown_history() {
    let dialect = "tcl9.1";
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(environment.grammar());
    let input = crate::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        context.clone(),
        config,
    );
    let fresh = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        context.commands(),
        &input,
        config,
        Some(input.unit_profile()),
    );
    assert!(fresh.stock_tclsh_stdout.is_some());
    let inherited = crate::command_binding::SourceAnalysisEntry {
        stock_tclsh_stdout: None,
        ..fresh.clone()
    };
    let unknown = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        ..fresh
    };
    for entry in [inherited, unknown] {
        let mut analyser = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .with_source_analysis_entry(std::sync::Arc::new(entry));
        let result = analyser.analyse(CONSTANT_AFTER_STDOUT, dialect);
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E007)
        );
    }
}

#[test]
fn stock_process_stdout_is_withdrawn_after_channel_mutation() {
    let source = "proc transform {op handle args} {return [lindex $args 0]}\n\
                  const fixed 3\n\
                  catch {chan push stdout transform}\n\
                  puts ready\n\
                  set fixed 4\n";
    assert!(!has_code(source, "tcl9.1", "E007"));
}

#[test]
fn e006_rejects_invalid_literal_formal_parameter_lists_from_registry_roles() {
    // Tcl 9.0.4 rejects a formal specifier with three fields, a namespace
    // qualified name, and an array element.  Duplicate parameter names remain
    // legal, and a value computed at runtime is deliberately outside the
    // analyser's proof boundary.
    for src in [
        "proc invalid {{a b c}} {}\n",
        "proc invalid {a::b} {}\n",
        "proc invalid {a(x)} {}\n",
    ] {
        assert!(has_code(src, "tcl9.0", "E006"), "expected E006 for {src:?}");
    }
    for src in [
        "proc duplicate {a a} {}\n",
        "set params {a::b}\nproc dynamic $params {}\n",
    ] {
        assert!(!has_code(src, "tcl9.0", "E006"), "must abstain for {src:?}");
    }
}

#[test]
fn e006_requires_unanimous_actual_formal_grammar_after_alias_mapping() {
    assert!(has_code(
        "interp alias {} declare {} proc p; declare {{a b c}} {}",
        "tcl8.6",
        "E006",
    ));
    for source in [
        "rename proc original; original proc args {return ordinary}; proc p {{a b c}} {}",
        "if {$unknown} {rename proc original; original proc args {return ordinary}}; proc p {{a b c}} {}",
        "proc p {*}$unknown",
        "external p {{a b c}} {}",
    ] {
        assert!(
            !has_code(source, "tcl8.6", "E006"),
            "opaque grammar: {source}"
        );
    }
}

#[test]
fn e006_offers_only_the_structurally_unambiguous_parameter_repair() {
    let mut analyser = crate::analyser::Analyser::new();
    let result = analyser.analyse("proc invalid {{a b c}} {}\n", "tcl9.0");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::E006)
        .expect("E006 is emitted");

    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "{a b c}");
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::analyser::types::FixSafety::RequiresReview
    );

    for source in ["proc invalid {a::b} {}\n", "proc invalid {a(x)} {}\n"] {
        let result = analyser.analyse(source, "tcl9.0");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E006)
            .expect("E006 is emitted");
        assert_eq!(diagnostic.fixes.len(), 0, "must abstain for {source:?}");
    }
}

#[test]
fn e006_discovers_definition_body_and_lambda_parameter_shapes() {
    // These are not ordinary top-level command arguments.  The TclOO method
    // shape comes from DefinitionBodyGrammar::member / MemberSpec::arg_roles;
    // `apply`'s nested first field comes from ArgRole::LambdaLiteral.
    assert!(has_code(
        "oo::class create C { method invalid {{a b c}} {} }\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code(
        "oo::class create C {}\noo::define C method invalid {{a b c}} {}\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code("apply {{{a b c}} {return 1}}\n", "tcl9.0", "E006",));
}
