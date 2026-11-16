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

#[test]
fn callback_signature_transport_retains_original_counts_and_source_purpose() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    // Readonly source assessment; the callback is not executed.
    use std::sync::Arc;
    use tcl_compiler::analyser::{
        Analyser, Diagnostic, DiagnosticSubject, ItemTree, Severity, SourceCallbackAritySubject,
    };
    let source = "proc cb {a b} {return 0}\nlsort -command {cb fixed} {3 1 2}";
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, "tcl9.0");
    let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
        .sigs()
        .into_iter()
        .filter_map(|sig| sig.original_declaration)
        .collect::<Vec<_>>();
    let prefix = analysis
        .command_invocations
        .iter()
        .find_map(|inv| inv.original_callback_prefix.as_ref())
        .unwrap();
    let subject =
        SourceCallbackAritySubject::from_source_signatures(Arc::clone(prefix), &headers).unwrap();
    let diagnostic = Diagnostic::new(
        subject.issue().code(),
        subject.span(),
        "presentation".to_owned(),
        Severity::Error,
    )
    .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
    let data = super::diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(data["subject"]["kind"], "callbackSourceArity");
    assert_eq!(data["subject"]["purpose"], "signature");
    assert_eq!(data["subject"]["scope"], "invokingFrame");
    assert_eq!(data["subject"]["counts"]["counts"], serde_json::json!([3]));
    assert_eq!(
        data["subject"]["declarations"][0]["parameterGrammar"],
        "tcl"
    );
    assert_eq!(data["subject"]["issue"]["kind"], "tooMany");
    assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
    assert!(super::DiagnosticSubjectData::from_value(&data, "W123").is_none());
    let mut wrong_code = data.clone();
    wrong_code["code"] = serde_json::json!("E002");
    assert!(
        super::DiagnosticSubjectData::from_value(&wrong_code, "E002").is_none(),
        "typed too-many issue cannot become too-few by transport code"
    );
    let mut changed = diagnostic;
    changed.message = "unrelated text".to_owned();
    assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
}

#[test]
fn original_callback_transport_keeps_unavailable_frame_and_registration_premises() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    // Genuine source signature advice; the callback is not executed and
    // its unknown frame is never replaced by the registration frame.
    use std::sync::Arc;
    use tcl_compiler::analyser::{
        Analyser, Diagnostic, DiagnosticSubject, Severity, SourceCallbackAritySubject,
    };
    let source =
        "# tcl-lsp: requires smtp\nproc cb {} {}\nsmtp::sendmessage message -tokenCallback ::cb";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let selection = analysis
        .command_invocations
        .iter()
        .find_map(|row| row.original_callback_signature_lookup.as_ref())
        .expect("authentic rooted source callback signature lookup");
    assert_eq!(selection.prefix().scope(), None);
    assert!(selection.prefix().lookup().is_none());
    assert!(selection.prefix().source_registration().is_some());
    let subject =
        SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
    let diagnostic = Diagnostic::new(
        subject.issue().code(),
        subject.span(),
        "presentation".to_owned(),
        Severity::Error,
    )
    .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
    let data = super::diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(data["subject"]["scope"], "unavailable");
    assert!(
        data["subject"]["sourceTarget"]["obligations"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("unavailableCallbackLookupFrame"))
    );
    assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
    let mut changed = diagnostic;
    changed.message = "unrelated presentation".to_owned();
    assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
}

#[test]
fn constructor_source_subject_retains_count_and_purpose_without_messages() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source =
        "oo::class create C {constructor {a b} { }}\ninterp alias {} make {} C new fixed\nmake";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let diagnostic = analysis.diagnostics.iter().find(|diagnostic| matches!(diagnostic.subject(), Some(tcl_compiler::analyser::DiagnosticSubject::ObjectSourceArity(subject)) if matches!(subject.as_ref(), tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(_)))).expect("constructor source signature");
    let mut changed = diagnostic.clone();
    changed.message = "unrelated presentation".to_owned();
    let first = super::diagnostic_subject_data(diagnostic).unwrap();
    assert_eq!(super::diagnostic_subject_data(&changed).unwrap(), first);
    assert_eq!(first["subject"]["purpose"], "constructor");
    assert_eq!(first["subject"]["minimumArgumentCount"], 1);
    assert_eq!(&source[changed.span.as_range()], "make");
}

#[test]
fn constructor_transport_retains_distinct_source_applicability_obligations() {
    use tcl_compiler::command_binding::SourceCommandTransitionObligation as Obligation;

    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    // Source inventory is conditional; the procedure is not invoked.
    let source = "proc make {} {C new 1}\noo::class create C {constructor {a b} { }}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            matches!(
                diagnostic.object_source_arity(),
                Some(tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(_))
            )
        })
        .expect("deferred source constructor signature");
    let data = super::diagnostic_subject_data(diagnostic).unwrap();
    assert!(
        data["subject"]["obligations"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "futureOriginalProcedureSourceApplicability"
            ))
    );
    for (value, label) in [
        (
            Obligation::NativeBaselineSourceApplicability,
            "nativeBaselineSourceApplicability",
        ),
        (
            Obligation::OriginalProcedureBodyApplicability,
            "originalProcedureBodyApplicability",
        ),
        (
            Obligation::FutureOriginalProcedureSourceApplicability,
            "futureOriginalProcedureSourceApplicability",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(super::RegisteredInstanceObligationDescription::from(&value))
                .unwrap(),
            label
        );
    }
}

#[test]
fn lexical_next_arity_transport_retains_declaration_and_effective_count() {
    // naming.tcloo.original-lexical-member-context
    // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
    let source = "oo::class create Base {method f {a b} {}}\noo::class create C {superclass Base; method f {} {::::next 1}}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.object_source_arity().is_some())
        .unwrap();
    let data = super::diagnostic_subject_data(diagnostic).unwrap();
    assert_eq!(data["subject"]["kind"], "objectSourceArity");
    assert_eq!(
        data["subject"]["classSimpleNameBytes"],
        serde_json::json!([67])
    );
    assert_eq!(data["subject"]["minimumArgumentCount"], 1);
    assert_eq!(data["subject"]["indeterminateArgumentCount"], false);
    assert!(super::DiagnosticSubjectData::from_value(&data, "E002").is_some());
    assert!(super::DiagnosticSubjectData::from_value(&data, "W123").is_none());
    let mut changed = diagnostic.clone();
    changed.message = "other wording".to_owned();
    assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
}

#[test]
fn registered_instance_transport_retains_factory_and_applicability_without_message_identity() {
    // naming.source.original-registered-instance-words
    // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
    let source = "ttk::treeview .t; .t bogus";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.registered_instance_source().is_some())
        .unwrap();
    let value = super::diagnostic_subject_data(diagnostic).unwrap();
    assert_eq!(value["subject"]["kind"], "registeredInstanceSource");
    assert_eq!(value["subject"]["factoryCommand"], "ttk::treeview");
    assert_eq!(
        value["subject"]["instanceNameBytes"],
        serde_json::json!([46, 116])
    );
    assert!(super::DiagnosticSubjectData::from_value(&value, "W001").is_some());
    let mut changed = diagnostic.clone();
    changed.message = "different wording".to_owned();
    assert_eq!(super::diagnostic_subject_data(&changed), Some(value));
}

use super::*;
use tcl_compiler::analyser::Severity;
use tcl_core_types::DiagCode;

fn package(bytes: &[u8]) -> Diagnostic {
    Diagnostic::new(
        DiagCode::W120,
        tcl_lexer::Span::new(0, 1),
        "same message",
        Severity::Hint,
    )
    .with_subject(DiagnosticSubject::RequiredPackage(
        tcl_registry::native_package::NativePackageNameKey::from_native_units(
            bytes,
            NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
        ),
    ))
}

#[test]
fn conditional_visibility_transport_preserves_assumptions_without_message_parsing() {
    // Implementation contract: naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let source = "interp create -safe s\ninterp eval s {::source a.tcl}";
    let result = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let original = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.conditional_interpreter_visibility().is_some())
        .unwrap();
    let data = DiagnosticSubjectData::from_diagnostic(original).unwrap();
    assert_eq!(
        data.conditional_visibility_bytes(),
        Some(b"::source".as_slice())
    );
    let encoded = data.to_value().unwrap();
    assert_eq!(
        encoded["subject"]["rootSlot"],
        serde_json::json!([115, 111, 117, 114, 99, 101])
    );
    assert_eq!(
        encoded["subject"]["obligations"][0],
        "successfulSourceOperations"
    );
    assert_eq!(
        DiagnosticSubjectData::from_value(&encoded, "W129"),
        Some(data)
    );
    assert!(DiagnosticSubjectData::from_value(&encoded, "W123").is_none());
    let mut foreign = encoded.clone();
    foreign["subject"]["obligations"][0] = serde_json::json!("unknownFutureObligation");
    assert!(DiagnosticSubjectData::from_value(&foreign, "W129").is_none());
    let mut report = original.clone();
    report.message = "an arbitrary reporting sentence".to_owned();
    assert_eq!(
        DiagnosticSubjectData::from_diagnostic(&report)
            .unwrap()
            .to_value(),
        Some(encoded)
    );
}

#[test]
fn package_transport_ignores_message_and_fixes_but_retains_opaque_bytes() {
    // Proof naming.diagnostic.typed-subject-reporting:
    // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
    let mut diagnostic = package(b"quoted'pkg\xc0\x80\xed\xa0\x80");
    let first = diagnostic_subject_data(&diagnostic).unwrap();
    diagnostic.message = "different wording and unrelated quoted names".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(first.clone()));
    let decoded = DiagnosticSubjectData::from_value(&first, "W120").unwrap();
    assert_eq!(
        decoded.package_bytes(),
        Some(b"quoted'pkg\xc0\x80\xed\xa0\x80".as_slice())
    );
    assert_ne!(diagnostic_subject_data(&package(b"other")), Some(first));
}

#[test]
fn absent_malformed_future_or_wrong_code_payload_never_recreates_subject() {
    // Proof naming.diagnostic.typed-subject-reporting:
    // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
    let mut diagnostic = package(b"Tk");
    diagnostic.subject = None;
    assert!(diagnostic_subject_data(&diagnostic).is_none());
    assert!(DiagnosticSubjectData::from_value(&Value::Null, "W120").is_none());
    let mut payload = diagnostic_subject_data(&package(b"Tk")).unwrap();
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
    payload["code"] = "W123".into();
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
    payload["code"] = "W120".into();
    payload["version"] = 2.into();
    assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
}

#[test]
fn unresolved_transport_uses_the_emitting_original_source_owner() {
    // Proof naming.diagnostic.typed-subject-reporting:
    // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
    let source = r"missing\uD800 argument";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let mut diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W123)
        .expect("actual unresolved command diagnostic")
        .clone();
    let Some(DiagnosticSubject::UnresolvedCommand(original)) = diagnostic.subject() else {
        panic!("emitter retains original source name");
    };
    assert_eq!(original.name_input().bytes(), b"missing\xed\xa0\x80");
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(
        payload["subject"]["bytes"],
        serde_json::json!(b"missing\xed\xa0\x80")
    );
    assert_eq!(payload["subject"]["sourceChannel"], "document");
    diagnostic.message = "wording with misleading 'different' and `quoted` names".to_owned();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
}

#[test]
fn shape_advice_transport_keeps_original_purpose_and_operand_axes() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    for (source, code, purpose, argument, written) in [
        (
            "interp alias {} add {} append out\nadd \" $item\"",
            tcl_core_types::DiagCode::W104,
            "appendList",
            Some(1),
            Some(0),
        ),
        (
            "interp alias {} choose {} switch -regexp subject\nchoose a $body",
            tcl_core_types::DiagCode::W106,
            "caseBody",
            Some(3),
            Some(1),
        ),
        (
            "interp alias {} clean {} unset -nocomplain\nclean",
            tcl_core_types::DiagCode::W217,
            "optionOnly",
            None,
            None,
        ),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .expect("owned shape advice")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], purpose);
        assert_eq!(payload["subject"]["argument"], serde_json::json!(argument));
        assert_eq!(
            payload["subject"]["writtenArgument"],
            serde_json::json!(written)
        );
        assert!(DiagnosticSubjectData::from_value(&payload, code.as_str()).is_some());
        diagnostic.message = "translated misleading spelling".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
}

#[test]
fn math_transport_preserves_expression_kind_without_reconstructing_a_command() {
    // naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    for (dialect, dispatch) in [
        ("tcl8.4", "fixedTable"),
        ("tcl8.5", "commandTable"),
        ("tcl8.6", "commandTable"),
        ("tcl9.0", "commandTable"),
        ("tcl9.1", "commandTable"),
        ("jim", "fixedTable"),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse("expr {Pi()}", dialect);
        let mut diagnostic = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.unresolved_math_function().is_some())
                .unwrap_or_else(|| panic!("{dialect}: original math diagnostic missing; invocations={:?}; diagnostics={:?}", analysis.command_invocations, analysis.diagnostics))
                .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["kind"], "unresolvedMathFunction");
        assert_eq!(payload["subject"]["bytes"], serde_json::json!(b"Pi"));
        assert_eq!(payload["subject"]["start"], 6);
        assert_eq!(payload["subject"]["end"], 8);
        assert_eq!(payload["subject"]["sourceChannel"], "document");
        assert_eq!(payload["subject"]["argumentCount"], 0);
        assert_eq!(payload["subject"]["ordinal"], 0);
        assert_eq!(payload["subject"]["dispatch"], dispatch);
        assert_eq!(payload["subject"]["classification"], "absent");
        let decoded = DiagnosticSubjectData::from_value(&payload, "W123").unwrap();
        assert_eq!(decoded.math_function_reporting_name(), Some("Pi"));
        assert!(decoded.command_reporting_name().is_none());
        assert!(decoded.package_bytes().is_none());
        assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
        diagnostic.message = "translated misleading 'Else'".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
        let mut future = payload.clone();
        future["version"] = 2.into();
        assert!(DiagnosticSubjectData::from_value(&future, "W123").is_none());
        let mut unknown_purpose = payload.clone();
        unknown_purpose["subject"]["dispatch"] = "future".into();
        assert!(DiagnosticSubjectData::from_value(&unknown_purpose, "W123").is_none());
        unknown_purpose = payload.clone();
        unknown_purpose["subject"]["classification"] = "unknown".into();
        assert!(DiagnosticSubjectData::from_value(&unknown_purpose, "W123").is_none());
        let mut malformed = payload;
        malformed["subject"]["start"] = "unknown".into();
        assert!(DiagnosticSubjectData::from_value(&malformed, "W123").is_none());
    }
}
#[test]
fn declared_source_transport_preserves_contract_purpose_and_provenance() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {expression:expr}\n# tcl-lsp: stubs-end\nselect \"$value + 1\"";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let mut diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.declared_source().is_some())
        .expect("declared source diagnostic")
        .clone();
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["kind"], "declaredSource");
    assert_eq!(payload["subject"]["purpose"], "expression");
    assert_eq!(payload["subject"]["declarationName"], "select");
    assert_eq!(payload["subject"]["provenance"], "document");
    assert!(
        payload["subject"]["obligations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|obligation| obligation == "declarationApplicability")
    );
    diagnostic.message = "translated misleading 'expr'".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
    let decoded = DiagnosticSubjectData::from_value(&payload, "W100").unwrap();
    assert!(decoded.command_reporting_name().is_none());
    assert!(decoded.package_bytes().is_none());
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
}
#[test]
fn unavailable_command_transport_retains_the_original_purpose_without_messages() {
    // naming.diagnostic.original-command-source-unavailability
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
    for (source, dialect, code, command, class, domain) in [
        (
            "::dict get {a 1} a",
            "tcl8.4",
            DiagCode::W002,
            "dict",
            "unavailable",
            "native",
        ),
        (
            "when RULE_INIT {rename a b}",
            "f5-irules",
            DiagCode::Irule2004,
            "rename",
            "ruleLoaderRefused",
            "hosted",
        ),
    ] {
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let original = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap()
            .clone();
        assert!(
            original
                .command_availability()
                .unwrap()
                .matches_analysis(&analysis)
        );
        let payload = DiagnosticSubjectData::from_diagnostic(&original)
            .unwrap()
            .to_value()
            .unwrap();
        assert_eq!(payload["subject"]["command"], command);
        assert_eq!(payload["subject"]["classification"], class);
        assert_eq!(payload["subject"]["domain"], domain);
        assert_eq!(payload["subject"]["sourceChannel"], "document");
        let mut report = original;
        report.message = "unrelated reporting text".into();
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        analysis.diagnostics.clear();
        assert!(
            report
                .command_availability()
                .unwrap()
                .matches_analysis(&analysis)
        );
        assert_eq!(
            DiagnosticSubjectData::from_diagnostic(&report)
                .unwrap()
                .to_value()
                .unwrap(),
            payload
        );
        let decoded = DiagnosticSubjectData::from_value(&payload, &code.to_string()).unwrap();
        assert!(decoded.package_bytes().is_none());
        assert!(decoded.command_reporting_name().is_none());
        assert!(decoded.math_function_reporting_name().is_none());
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
        for field in ["classification", "domain"] {
            let mut unsupported = payload.clone();
            unsupported["subject"][field] = Value::from("future");
            assert!(DiagnosticSubjectData::from_value(&unsupported, &code.to_string()).is_none());
        }
        let mut incorrect_purpose = payload;
        incorrect_purpose["subject"]["classification"] = Value::from(if class == "unavailable" {
            "ruleLoaderRefused"
        } else {
            "unavailable"
        });
        assert!(DiagnosticSubjectData::from_value(&incorrect_purpose, &code.to_string()).is_none());
    }
}
#[test]
fn declared_variable_name_transport_keeps_its_authored_purpose() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination:var value}\n# tcl-lsp: stubs-end\nstore $name payload";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let mut diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W212)
        .expect("declaration-owned variable name")
        .clone();
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["purpose"], "variableName");
    assert_eq!(payload["subject"]["argument"], 0);
    assert!(DiagnosticSubjectData::from_value(&payload, "W212").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W100").is_none());
    diagnostic.message = "unrelated translated report".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
}
#[test]
fn original_package_source_subject_transport_retains_selected_purpose_and_package() {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    let source = "csv::join {a b}";
    let result = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W120)
        .unwrap();
    let data = DiagnosticSubjectData::from_diagnostic(diagnostic).unwrap();
    let encoded = data.to_value().unwrap();
    assert_eq!(encoded["subject"]["purpose"], "packageRequirement");
    assert_eq!(encoded["subject"]["requiredPackage"], "csv");
    assert_eq!(data.package_bytes(), Some(b"csv".as_slice()));
    assert!(DiagnosticSubjectData::from_value(&encoded, "H301").is_none());
    let mut changed = diagnostic.clone();
    changed.message = "unrelated reporting text".to_owned();
    changed.fixes.clear();
    assert_eq!(diagnostic_subject_data(&changed), Some(encoded));
}
#[test]
fn callback_alias_transport_keeps_capture_baked_and_suffix_premises() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    use std::sync::Arc;
    use tcl_compiler::analyser::{
        Analyser, Diagnostic, DiagnosticSubject, Severity, SourceCallbackAritySubject,
    };
    let source = "proc target {a b c d} {}\ninterp alias {} inner {} target INNER\ninterp alias {} outer {} inner OUTER\nlsort -command {outer BAKED} {2 1}";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let selection = analysis
        .command_invocations
        .iter()
        .find_map(|row| {
            row.original_callback_signature_lookup
                .as_ref()
                .filter(|selection| {
                    selection.prefix().appended_arity()
                        == Some(tcl_registry::AppendedArity::Exactly(2))
                })
        })
        .unwrap();
    let subject =
        SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
    let diagnostic = Diagnostic::new(
        subject.issue().code(),
        subject.span(),
        "presentation".to_owned(),
        Severity::Error,
    )
    .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
    let data = super::diagnostic_subject_data(&diagnostic).unwrap();
    let details = &data["subject"];
    assert_eq!(details["bakedArgumentCount"], 1);
    assert_eq!(details["appendedCounts"]["counts"], serde_json::json!([2]));
    assert_eq!(details["counts"]["counts"], serde_json::json!([5]));
    let target = &details["sourceTarget"];
    assert_eq!(target["kind"], "localProcedure");
    assert_eq!(target["capturedMinimum"], 2);
    assert_eq!(target["capturedIndeterminate"], false);
    assert_eq!(
        target["captures"][0]["sourceLiteralBytes"],
        serde_json::json!(b"INNER".to_vec())
    );
    assert_eq!(
        target["captures"][1]["sourceLiteralBytes"],
        serde_json::json!(b"OUTER".to_vec())
    );
    assert_eq!(target["captures"][0]["origin"]["kind"], "written");
    assert_eq!(target["captures"][0]["origin"]["ordinal"], 6);
    assert_eq!(target["lineage"].as_array().unwrap().len(), 2);
    let obligations = target["obligations"].as_array().unwrap();
    assert!(obligations.contains(&serde_json::json!("originalCallbackTargetApplicability")));
    assert!(obligations.contains(&serde_json::json!("writtenTransitionApplicability")));
    assert!(selection.original().obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::WrittenTransitionApplicability
        ));
    assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
    let mut changed = diagnostic;
    changed.message = "unrelated presentation".to_owned();
    assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
}
#[test]
fn original_index_transport_preserves_captured_operand_purpose_without_message_names() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} at {} string range {λé} 9; at 12";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let mut diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W232)
        .unwrap()
        .clone();
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["purpose"], "indexBounds");
    assert_eq!(payload["subject"]["argument"], 2);
    assert_eq!(payload["subject"]["writtenArgument"], Value::Null);
    assert_eq!(payload["subject"]["start"], diagnostic.span.start());
    assert_eq!(payload["subject"]["end"], diagnostic.span.end());
    assert!(DiagnosticSubjectData::from_value(&payload, "W232").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
    diagnostic.message = "translated text with a misleading command and variable name".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
}
#[test]
fn original_template_transport_keeps_selected_prefix_purpose_and_ignores_presentation() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} render {} subst -nocommands; render \"pré $tmpl\"";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let mut diagnostic = result
        .diagnostics
        .into_iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W102)
        .expect("original source template warning");
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["purpose"], "templateSubstitution");
    assert_eq!(payload["subject"]["argument"], 1);
    assert_eq!(payload["subject"]["writtenArgument"], 0);
    assert_eq!(&source[diagnostic.span.as_range()], "\"pré $tmpl\"");
    assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
    diagnostic.message = "translated text with misleading command and operand names".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
}
#[test]
fn original_reparse_transport_keeps_both_selected_purposes_without_presentation_names() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} run {} eval {set local}; interp alias {} render {} subst; run [render $tmpl]";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    for code in [DiagCode::W101, DiagCode::W309] {
        let mut diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .expect("original selected reparse warning")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "scriptReparse");
        assert_eq!(payload["subject"]["argument"], 1);
        assert_eq!(payload["subject"]["writtenArgument"], 0);
        assert_eq!(&source[diagnostic.span.as_range()], "[render $tmpl]");
        assert!(
            DiagnosticSubjectData::from_value(
                &payload,
                if code == DiagCode::W101 {
                    "W101"
                } else {
                    "W309"
                }
            )
            .is_some()
        );
        assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
        diagnostic.message = "translated text with misleading command and operand names".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
}
#[test]
fn original_path_transport_keeps_alias_ordinals_and_proven_value_premises() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} load {} source -encoding utf-8; load \"rép/$path\"; proc f {} {set p {|literal}; open $p}";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    for (code, purpose, argument, literal) in [
        (DiagCode::W300, "sourceFilePath", 2, None),
        (DiagCode::W103, "channelPath", 0, Some("|literal")),
    ] {
        let mut diagnostic = result
            .diagnostics
            .iter()
            .find(|finding| finding.code == code)
            .expect("original path finding")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], purpose);
        assert_eq!(payload["subject"]["argument"], argument);
        assert_eq!(payload["subject"]["writtenArgument"], 0);
        if let Some(literal) = literal {
            assert_eq!(payload["subject"]["supplementalLiterals"][0]["argument"], 0);
            assert_eq!(
                payload["subject"]["supplementalLiterals"][0]["value"],
                literal
            );
        }
        assert!(DiagnosticSubjectData::from_value(&payload, code.as_str()).is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
        diagnostic.message = "translated 'open/source' with misleading values".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
}

#[test]
fn original_channel_configuration_transport_keeps_selected_captured_option_values() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} configure {} fconfigure stdout -encoding binary -translation {é[$literal]}; configure";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let mut diagnostic = result
        .diagnostics
        .iter()
        .find(|finding| finding.code == DiagCode::W311)
        .expect("selected channel relationship")
        .clone();
    assert_eq!(&source[diagnostic.span.as_range()], "{é[$literal]}");
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["purpose"], "channelConfiguration");
    assert_eq!(payload["subject"]["argument"], 4);
    assert_eq!(
        payload["subject"]["writtenArgument"],
        serde_json::Value::Null
    );
    assert!(DiagnosticSubjectData::from_value(&payload, "W311").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W310").is_none());
    diagnostic.message = "translated presentation naming unrelated options".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
}

#[test]
fn original_credential_transport_keeps_captured_literals_separate_from_lexical_policy() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} request {} http::geturl URL -headers {é[$literal]}; request";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let mut diagnostic = result
        .diagnostics
        .iter()
        .find(|finding| finding.code == DiagCode::W310)
        .expect("captured credential metadata")
        .clone();
    assert_eq!(&source[diagnostic.span.as_range()], "{é[$literal]}");
    let payload = diagnostic_subject_data(&diagnostic).unwrap();
    assert_eq!(payload["subject"]["purpose"], "credentialLiteral");
    assert_eq!(payload["subject"]["argument"], 2);
    assert_eq!(
        payload["subject"]["writtenArgument"],
        serde_json::Value::Null
    );
    assert!(DiagnosticSubjectData::from_value(&payload, "W310").is_some());
    assert!(DiagnosticSubjectData::from_value(&payload, "W303").is_none());
    diagnostic.message = "translated presentation naming unrelated commands".to_owned();
    diagnostic.fixes.clear();
    assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    let lexical =
        tcl_compiler::analyser::Analyser::new().analyse("mycmd -password VALUE", "tcl8.6");
    let lexical = lexical
        .diagnostics
        .iter()
        .find(|finding| finding.code == DiagCode::W310)
        .expect("default lexical policy");
    assert!(diagnostic_subject_data(lexical).is_none());
}

#[test]
fn original_pattern_shape_transport_keeps_child_ancestry_and_value_premises() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for (source, argument, written, list_path, literal) in [
        (
            "interp alias {} choose {} switch -regexp literal {(é+)+ {BODY}}; choose",
            2,
            None,
            Some(0),
            None,
        ),
        (
            "proc f {} {set p {(a+)+}; regexp $p literal}",
            0,
            Some(0),
            None,
            Some("(a+)+"),
        ),
    ] {
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        let mut diagnostic = result
            .diagnostics
            .iter()
            .find(|finding| finding.code == DiagCode::W303)
            .expect("original selected pattern")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "patternShape");
        assert_eq!(payload["subject"]["argument"], argument);
        assert_eq!(
            payload["subject"]["writtenArgument"],
            serde_json::json!(written)
        );
        if let Some(index) = list_path {
            assert_eq!(
                payload["subject"]["listElementPath"],
                serde_json::json!([index])
            );
            assert_eq!(&source[diagnostic.span.as_range()], "(é+)+");
        } else {
            assert!(payload["subject"].get("listElementPath").is_none());
        }
        if let Some(literal) = literal {
            assert_eq!(
                payload["subject"]["supplementalLiterals"][0]["value"],
                literal
            );
        }
        assert!(DiagnosticSubjectData::from_value(&payload, "W303").is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W306").is_none());
        diagnostic.message = "translated regexp/switch with invented operand text".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
}

#[test]
fn original_crossing_transport_keeps_captured_ordinals_and_ignores_presentation() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "interp alias {} frame {} uplevel 1; interp alias {} child {} interp invokehidden {} -namespace ::N; frame \"pré $value\"; child $command";
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    for (code, argument, span) in [
        (DiagCode::W301, 1, "\"pré $value\""),
        (DiagCode::W312, 4, "$command"),
    ] {
        let mut diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .expect("original crossing warning")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "scriptReparse");
        assert_eq!(payload["subject"]["argument"], argument);
        assert_eq!(payload["subject"]["writtenArgument"], 0);
        assert_eq!(&source[diagnostic.span.as_range()], span);
        assert!(
            DiagnosticSubjectData::from_value(
                &payload,
                if code == DiagCode::W301 {
                    "W301"
                } else {
                    "W312"
                }
            )
            .is_some()
        );
        assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
        diagnostic.message = "unrelated presentation names".to_owned();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
}
