// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Case source regions on a separately conditioned successful continuation.

use crate::case_bodies::CaseBodyOperands;
use crate::spec::CaseInvocation;
use crate::{BodyInterpreter, BodyKind, CaseMatchMode, ResolvedInvocation};

/// One selected case layout under the question "if this source call continues".
/// This is no success certificate, Native compiler selection, or entered body.
/// The ordinary invocation facts and their option/error residual are unchanged.
#[derive(Debug, Clone)]
pub struct NormalSourceCaseLayout {
    invocation: CaseInvocation,
    possible: CaseBodyOperands,
}

impl NormalSourceCaseLayout {
    /// Registry-owned positional source layout, conditioned on continuation.
    #[must_use]
    pub const fn invocation(&self) -> CaseInvocation {
        self.invocation
    }

    /// Original possible-body projection, including its unmodified residuals.
    #[must_use]
    pub const fn possible_bodies(&self) -> &CaseBodyOperands {
        &self.possible
    }
}

/// Project only the reserved final subject/list source shape of C Tcl 8.5+.
/// Its handler reserves those two argv words even after a static option prefix.
/// A prefixed Native compiler can still decline without an option terminator;
/// this changes neither that compiler nor the ordinary complete argument roles.
#[must_use]
pub fn case_layout_on_normal_source_continuation(
    schema: &ResolvedInvocation<'_, '_>,
    values: tcl_syntax::word_rules::WordValueRules,
) -> Option<NormalSourceCaseLayout> {
    use tcl_dialect::{TclVersion, model::Family};
    let arguments = schema.words.arguments();
    let dialect = arguments.dialect()?;
    if dialect.family() != Some(Family::Tcl)
        || !matches!(
            dialect.tcl_version?,
            TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
        )
    {
        return None;
    }
    case_layout_on_source_continuation(schema, values)
}

/// Unversioned Tcl layout for an explicitly Logical declaring scope. This
/// retains every option/operand residual and supplies no Native handler entry.
#[must_use]
pub fn case_layout_on_logical_source_continuation(
    schema: &ResolvedInvocation<'_, '_>,
    values: tcl_syntax::word_rules::WordValueRules,
) -> Option<NormalSourceCaseLayout> {
    let dialect = schema.words.arguments().dialect()?;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        || dialect.tcl_version.is_some()
        || dialect.core_point.is_some()
        || dialect.authored_name_policy()?.authority()
            != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    {
        return None;
    }
    case_layout_on_source_continuation(schema, values)
}

fn case_layout_on_source_continuation(
    schema: &ResolvedInvocation<'_, '_>,
    values: tcl_syntax::word_rules::WordValueRules,
) -> Option<NormalSourceCaseLayout> {
    let arguments = schema.words.arguments();
    if schema.semantics.argument_offset != 0
        || schema.facts().lowering_hook != Some(crate::hooks::LoweringHookId::Switch)
        || schema.semantics.body_kind != BodyKind::Plain
        || schema.semantics.body_interpreter != BodyInterpreter::Current
        || schema.semantics.frame_effect.is_some()
        || schema.semantics.traits.contains(crate::Traits::DEFERS_BODY)
        || !schema.semantics.options.value_roles(arguments)?.is_empty()
    {
        return None;
    }
    let count = arguments.exact_argv_len()?;
    let subject = count.checked_sub(2)?;
    // Only the one unknown final subject is admitted, not an unknown option
    // prefix, computed list, inline clauses or argv expansion.
    if !matches!(arguments.get(subject)?, crate::InvocationWord::Dynamic)
        || (0..count).any(|index| {
            index != subject
                && arguments.literal_at(index).is_none()
                && arguments.native_bytes_at(index).is_none()
        })
    {
        return None;
    }
    let (case, invocation) = schema.authored_source_case_invocation()?;
    if case.subject_args != 1
        || case.optional_subject_separator.is_some()
        || !case.clause_flags.is_empty()
        || !case.clause_value_flags.is_empty()
        || case.clause_force_inline_flag.is_some()
        || case.clause_force_list_flag.is_some()
        || case.clause_force_list_shape.is_some()
        || invocation.subject_index != Some(subject)
        || invocation.clause_list_index != Some(count - 1)
        || invocation.inline_clause_start.is_some()
        || invocation.mode == CaseMatchMode::Other
    {
        return None;
    }
    let possible = schema.authored_source_case_clause_projection(values)?;
    if possible.bodies.is_empty()
        || possible
            .bodies
            .iter()
            .chain(&possible.patterns)
            .any(|body| body.argument != count - 1 || body.list_element.is_none())
    {
        return None;
    }
    Some(NormalSourceCaseLayout {
        invocation,
        possible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationWord::{Dynamic, Expanded, Literal};

    #[test]
    fn normal_case_source_layout_keeps_native_compiler_decline_and_error_residuals() {
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection,
        };
        let registry = crate::CommandRegistry::build_default();
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let arguments = [
                Literal("-glob"),
                Dynamic,
                Literal("a* {break} default {break}"),
            ];
            let words = crate::InvocationWords::structured(Literal("switch"), &arguments)
                .with_dialect(dialect);
            let selected = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .unwrap();
            let config = tcl_lexer::LexerConfig::from_grammar(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                    .unwrap()
                    .grammar,
            );
            let shape = case_layout_on_normal_source_continuation(
                &selected,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
            .expect("same original subject and clause list on continuation");
            assert_eq!(shape.invocation().subject_index, Some(1));
            assert_eq!(shape.invocation().clause_list_index, Some(2));
            assert_eq!(shape.possible_bodies().bodies.len(), 2);
            assert!(shape.possible_bodies().selection_unknown);
            assert!(!shape.possible_bodies().no_match_possible);
            assert!(!selected.facts().arg_roles_complete);
            let spec = registry
                .native_compilation_for_registration("switch", dialect)
                .unwrap();
            let source = tcl_lexer::SourceImage::native(
                b"switch -glob $x {a* {break} default {break}}".as_slice(),
            );
            let parsed = tcl_lexer::native_script_words_in(
                source.clone(),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let original = crate::native_compiler_words::NativeCompilerWords::capture(
                &parsed.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            assert_eq!(
                spec.select_native_words(
                    &original,
                    1,
                    Some(dialect),
                    NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ProcedureCode,
                        ..NativeCompilationContext::default()
                    },
                ),
                NativeCompilationSelection::Generic,
            );
        }
    }

    #[test]
    fn logical_case_layout_requires_unversioned_source_and_the_same_bounded_shape() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = crate::CommandRegistry::build_default();
        let dialect =
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
        let config =
            tcl_lexer::LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
        let values = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        let arguments = [
            Literal("-glob"),
            Dynamic,
            Literal("a* {return 1} default {set z 9}"),
        ];
        let words =
            crate::InvocationWords::structured(Literal("switch"), &arguments).with_dialect(dialect);
        let selected = registry
            .resolve_structured_invocation(words, dialect.authoring_query())
            .resolved()
            .unwrap();
        assert!(case_layout_on_normal_source_continuation(&selected, values).is_none());
        let logical = case_layout_on_logical_source_continuation(&selected, values).unwrap();
        assert_eq!(logical.possible_bodies().bodies.len(), 2);
        assert!(logical.possible_bodies().selection_unknown);
        assert!(!selected.facts().arg_roles_complete);
        for arguments in [
            vec![Dynamic, Dynamic, Literal("a* {break}")],
            vec![Literal("-glob"), Dynamic, Dynamic],
            vec![Literal("-glob"), Dynamic, Literal("a*"), Literal("break")],
        ] {
            let words = crate::InvocationWords::structured(Literal("switch"), &arguments)
                .with_dialect(dialect);
            if let Some(selected) = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
            {
                assert!(case_layout_on_logical_source_continuation(&selected, values).is_none());
            }
        }
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let words = crate::InvocationWords::structured(Literal("switch"), &arguments)
                .with_dialect(dialect);
            if let Some(selected) = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
            {
                assert!(case_layout_on_logical_source_continuation(&selected, values).is_none());
            }
        }
    }

    #[test]
    fn normal_case_source_layout_refuses_other_versions_and_unrepresented_shapes() {
        let registry = crate::CommandRegistry::build_default();
        for (version, arguments) in [
            (
                tcl_dialect::TclVersion::V8_4,
                vec![Literal("-glob"), Dynamic, Literal("a* {break}")],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![Dynamic, Literal("a"), Literal("a* {break}")],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![Literal("-glob"), Dynamic, Dynamic],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![Literal("-glob"), Dynamic, Literal("a* {break} dangling")],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![
                    Literal("-regexp"),
                    Literal("-matchvar"),
                    Literal("v"),
                    Dynamic,
                    Literal("a* {break}"),
                ],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![Literal("-glob"), Expanded, Literal("a* {break}")],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                vec![Literal("-glob"), Dynamic, Literal("a*"), Literal("break")],
            ),
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("switch"), &arguments)
                        .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .unwrap();
            let config = tcl_lexer::LexerConfig::default();
            assert!(
                case_layout_on_normal_source_continuation(
                    &selected,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                )
                .is_none(),
            );
        }
    }
}
