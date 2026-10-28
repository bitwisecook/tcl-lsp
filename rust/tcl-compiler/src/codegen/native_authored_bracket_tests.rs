// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bracket scripts under an authored naming simulation compile generically;
//! a native policy keeps native scheduling and its refusal.

use super::*;

const SOURCE: &[u8] = b"set y [llength [info commands ::nope]]";

fn entry(
    name_protocol: impl FnOnce(
        tcl_dialect::model::DialectPoint,
    ) -> tcl_syntax::naming::NamePolicyProtocol,
    string_protocol: bool,
) -> crate::environment_ingress::CapturedNativeEntry {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut entry = crate::environment_ingress::captured_native_entry(profile);
    let point = entry.execution_point.unwrap();
    entry.name_protocol = Some(name_protocol(point));
    if !string_protocol {
        entry.source_string_protocol = None;
    }
    entry
}

fn emit_bracket_word<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
) -> CodegenCtx<'a> {
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(SOURCE),
        tcl_lexer::Span::new(0, u32::try_from(SOURCE.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.native_entry = Some(entry);
    ctx.invocation_dialect = Some(dialect);
    ctx.source_string_protocol = dialect.native_source_string_protocol();
    ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
    ctx.emit_original_native_word(&parsed.commands[0].words[2]);
    ctx
}

fn native(point: tcl_dialect::model::DialectPoint) -> tcl_syntax::naming::NamePolicyProtocol {
    tcl_syntax::naming::NamePolicyProtocol::for_native_point(point).unwrap()
}

fn authored(_: tcl_dialect::model::DialectPoint) -> tcl_syntax::naming::NamePolicyProtocol {
    tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
}

#[test]
fn an_authored_name_policy_compiles_nested_brackets_by_generic_dispatch() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for string_protocol in [true, false] {
        let entry = entry(authored, string_protocol);
        let ctx = emit_bracket_word(&registry, &entry);
        assert!(!ctx.native_dependency_refusal, "{string_protocol}");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"info"),
            "the nested head is dispatched by name"
        );
    }
}

#[test]
fn a_native_name_policy_keeps_native_scheduling_and_its_refusal() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let current = entry(native, true);
    assert!(!emit_bracket_word(&registry, &current).native_dependency_refusal);
    // The same entry without its source string protocol cannot select a
    // native recipe; the bracket script still requires the provider.
    let unprepared = entry(native, false);
    assert!(emit_bracket_word(&registry, &unprepared).native_dependency_refusal);
}
