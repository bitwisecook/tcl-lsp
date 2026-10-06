// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C string-match compiler selection and ordered stack operands.

use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::semantic_operation::SemanticOperationId;
use tcl_dialect::TclVersion;

/// Authenticated compiler registration's operand layout, independent of its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringMatchScope {
    /// The monolithic/public descriptor still includes the member operand.
    PublicMember,
    /// The actual private compiler registration receives only matcher operands.
    PrivateOperands,
}

/// Native stack matcher selected before evaluating either original operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringMatchOperation {
    /// A compile-known SIMPLE pattern has no native glob metacharacters.
    Equal,
    /// The native glob instruction receives its independently selected case flag.
    Glob {
        /// The original SIMPLE option is a native `-nocase` prefix of length two or more.
        nocase: bool,
    },
}

impl NativeStringMatchOperation {
    /// Only C84 `STR_MATCH` reuses an unshared pattern; `STR_EQ` creates a new Int.
    #[must_use]
    pub const fn reuses_unshared_pattern(self, version: TclVersion) -> bool {
        matches!((self, version), (Self::Glob { .. }, TclVersion::V8_4))
    }
}

/// Evaluate pattern then subject, and only then invoke the native getters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeStringMatchInstruction {
    /// Same original pattern word, including an original static expansion member.
    pub pattern: NativeCompilerWordOperand,
    /// Same original subject word, visited after the pattern.
    pub subject: NativeCompilerWordOperand,
    /// Authentic selected native matcher instruction.
    pub operation: NativeStringMatchOperation,
}

/// Actual private compiler registration and the public original member map.
pub const STRING_MATCH_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "match",
        slot: "::tcl::string::match",
        command: "string",
        prepended: &["match"],
    };

impl crate::CommandRegistry {
    /// Refine an issued monolithic registration with its unchanged original member.
    ///
    /// The caller independently owns the live registration and hook prerequisites.
    /// C84's String compiler reads its original literal member token; modern
    /// ensembles select actual workers through their separate map and token recipe.
    #[must_use]
    pub fn native_compilation_for_original_registration(
        &self,
        identity: &str,
        words: &NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let registered = self.native_compilation_for_registration(identity, dialect)?;
        if registered.grammar != crate::native_compilation::NativeCompilationGrammar::Unresolved
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(TclVersion::V8_4)
        {
            return Some(registered);
        }
        self.original_monolithic_string_registration(identity, words, operand_from, dialect)
            .or(Some(registered))
    }

    fn original_monolithic_string_registration(
        &self,
        identity: &str,
        words: &NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        if words.shapes().get(operand_from) != Some(&NativeCompilationWordShape::Literal) {
            return None;
        }
        let member = core::str::from_utf8(words.literal(operand_from)?).ok()?;
        let query = dialect.authoring_query()?;
        let registration =
            self.get_for_surface(identity.strip_prefix("::").unwrap_or(identity), Some(query))?;
        let selected = registration
            .resolve_subcommand_for_dialect(member, Some(query))?
            .native_compilation?;
        matches!(
            selected.grammar,
            crate::native_compilation::NativeCompilationGrammar::StringMatch(
                NativeStringMatchScope::PublicMember
            ) | crate::native_compilation::NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | crate::native_compilation::NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            )
        )
        .then_some(selected)
    }
}

/// Modern standalone match/equality instructions use interpreter execution constants.
#[must_use]
pub const fn uses_execution_constant(version: TclVersion) -> bool {
    matches!(
        version,
        TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    )
}

fn simple(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

/// Select only the original native compiler's accepted option and arity geometry.
#[must_use]
pub fn select(
    shapes: &[NativeCompilationWordShape],
    option: Option<&[u8]>,
    version: TclVersion,
    operation: SemanticOperationId,
    arguments_from: usize,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if shapes.contains(&NativeCompilationWordShape::Opaque) {
        return Selection::Unknown;
    }
    if shapes.contains(&NativeCompilationWordShape::Expanded) || !matches!(shapes.len(), 2 | 3) {
        return Selection::Generic;
    }
    if shapes.len() == 3
        && !(simple(shapes[0])
            && option.is_some_and(|value| value.len() >= 2 && b"-nocase".starts_with(value)))
    {
        return if version >= TclVersion::V8_6 {
            Selection::NamedInvocation {
                lookup: &STRING_MATCH_IMPLEMENTATION,
                arguments_from,
                protocol: NativeNamedInvocationProtocol::Direct,
            }
        } else {
            Selection::Generic
        };
    }
    Selection::Inline {
        operation,
        guard: if version == TclVersion::V8_4 {
            NativeCompilationGuard::ChunkEntry
        } else {
            NativeCompilationGuard::BeforeArguments
        },
    }
}

/// Preserve byte-valued native parser expansion and exact original operand addresses.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeStringMatchScope,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return NativeCompilationSelection::Unknown;
    };
    let from = operand_from + usize::from(scope == NativeStringMatchScope::PublicMember);
    if scope == NativeStringMatchScope::PublicMember
        && !projected.get(operand_from).is_some_and(|word| {
            simple(word.shape)
                && (version != TclVersion::V8_4
                    || word.shape == NativeCompilationWordShape::Literal)
                && word
                    .literal
                    .as_deref()
                    .is_some_and(|value| !value.is_empty() && b"match".starts_with(value))
        })
    {
        return NativeCompilationSelection::Generic;
    }
    let Some(arguments) = projected.get(from..) else {
        return NativeCompilationSelection::Unknown;
    };
    let shapes: Vec<_> = arguments.iter().map(|word| word.shape).collect();
    let Some(arguments_from) = from.checked_sub(1) else {
        return NativeCompilationSelection::Unknown;
    };
    select(
        &shapes,
        arguments.first().and_then(|word| word.literal.as_deref()),
        version,
        operation,
        arguments_from,
    )
}

/// Project an independently admitted inline selection onto its actual stack instructions.
#[must_use]
pub fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeStringMatchScope,
    version: TclVersion,
) -> Option<NativeStringMatchInstruction> {
    if !matches!(
        select_original(
            words,
            operand_from,
            scope,
            version,
            SemanticOperationId::Invoke
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let from = operand_from + usize::from(scope == NativeStringMatchScope::PublicMember);
    let nocase = projected.len() - from == 3;
    let pattern = projected.get(from + usize::from(nocase))?;
    let subject = projected.get(from + usize::from(nocase) + 1)?;
    let trivial = !nocase
        && simple(pattern.shape)
        && pattern.literal.as_deref().is_some_and(|value| {
            let specials: &[u8] = if version == TclVersion::V8_4 {
                b"*[]?\\"
            } else {
                b"*[?\\"
            };
            !value
                .iter()
                .take_while(|&&byte| byte != 0)
                .any(|byte| specials.contains(byte))
        });
    Some(NativeStringMatchInstruction {
        pattern: pattern.operand.clone(),
        subject: subject.operand.clone(),
        operation: if trivial {
            NativeStringMatchOperation::Equal
        } else {
            NativeStringMatchOperation::Glob { nocase }
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    const CASES: [&[u8]; 12] = [
        b"string match * $subject",
        b"string match A $subject",
        b"string match {]} $subject",
        b"string match -n A $subject",
        b"string match - A $subject",
        b"string match -nocaseX A $subject",
        b"string match $flag A $subject",
        b"string match $pattern $subject",
        b"string match \\-n A $subject",
        b"string match {*}\"A\" $subject",
        b"string match {*}{* X}",
        b"string match A",
    ];

    #[test]
    fn monolithic_match_member_projection_preserves_registration_and_original_token_scope() {
        use crate::native_compilation::NativeCompilationGrammar;
        let registry = crate::CommandRegistry::build_default();
        for profile_name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(profile_name).unwrap();
            let dialect = crate::InvocationDialect::of_profile(profile);
            for (source, plain_member) in [
                (b"string match * $subject".as_slice(), true),
                (b"renamed mat * $subject".as_slice(), true),
                (b"string {match} * $subject".as_slice(), false),
                (b"string \"match\" * $subject".as_slice(), false),
                (b"string $member * $subject".as_slice(), false),
                (b"string range A 0 1".as_slice(), false),
            ] {
                let parsed = native_script_words_in(
                    SourceImage::native(source),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    NativeStringProtocol::C(dialect.tcl_version.unwrap()),
                )
                .unwrap();
                let selected = registry
                    .native_compilation_for_original_registration("string", &words, 1, dialect)
                    .unwrap();
                let refined = profile_name == "tcl8.4" && plain_member;
                assert_eq!(
                    matches!(
                        selected.grammar,
                        NativeCompilationGrammar::StringMatch(NativeStringMatchScope::PublicMember)
                    ),
                    refined,
                    "{profile_name}/{source:?}",
                );
                if !refined {
                    assert_eq!(selected.grammar, NativeCompilationGrammar::Unresolved);
                }
                assert!(
                    registry
                        .native_compilation_for_original_registration(
                            "unissued_registration",
                            &words,
                            1,
                            dialect,
                        )
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn original_string_match_selection_preserves_all_60_native_compiler_windows() {
        let rows = include_str!("../tests/data/native_string_compilation/windows.tsv");
        let mut inline_count = 0;
        for line in rows.lines().skip(1) {
            let fields: Vec<_> = line.split('\t').collect();
            let version = match fields[0] {
                "8.4.20" => TclVersion::V8_4,
                "8.5.19" => TclVersion::V8_5,
                "8.6.18" => TclVersion::V8_6,
                "9.0.4" => TclVersion::V9_0,
                "9.1.0" => TclVersion::V9_1,
                _ => unreachable!(),
            };
            let profile = tcl_dialect::DialectProfile::find(match version {
                TclVersion::V8_4 => "tcl8.4",
                TclVersion::V8_5 => "tcl8.5",
                TclVersion::V8_6 => "tcl8.6",
                TclVersion::V9_0 => "tcl9.0",
                TclVersion::V9_1 => "tcl9.1",
            })
            .unwrap();
            let source = CASES[fields[1].parse::<usize>().unwrap()];
            let parsed = native_script_words_in(
                SourceImage::native(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                LexerConfig::from_grammar(profile.grammar),
            );
            if version == TclVersion::V8_4 && matches!(fields[1], "9" | "10") {
                assert!(parsed.unwrap().fatal_tail.is_some());
                continue;
            }
            let mut parsed = parsed.unwrap();
            let words = parsed.commands.remove(0).words;
            let original =
                NativeCompilerWords::capture(&words, NativeStringProtocol::C(version)).unwrap();
            let recipe = instruction(&original, 1, NativeStringMatchScope::PublicMember, version);
            let native_inline = fields[8]
                .split(',')
                .any(|op| matches!(op, "streq" | "strmatch"));
            assert_eq!(recipe.is_some(), native_inline, "{line}");
            if let Some(recipe) = recipe {
                inline_count += 1;
                assert_eq!(
                    matches!(recipe.operation, NativeStringMatchOperation::Equal),
                    fields[8].split(',').any(|op| op == "streq"),
                    "{line}"
                );
            } else if matches!(fields[1], "4" | "5" | "6" | "8") {
                assert_eq!(
                    matches!(
                        select_original(
                            &original,
                            1,
                            NativeStringMatchScope::PublicMember,
                            version,
                            SemanticOperationId::Invoke
                        ),
                        NativeCompilationSelection::NamedInvocation { .. }
                    ),
                    version >= TclVersion::V8_6,
                    "{line}"
                );
            }
        }
        assert_eq!(inline_count, 33);
    }

    #[test]
    fn private_match_registration_does_not_reparse_invoked_name_or_binary_pattern() {
        let source: &[u8] = b"renamed A\0B $subject";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut parsed = native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = parsed.commands.remove(0).words;
        let original =
            NativeCompilerWords::capture(&words, NativeStringProtocol::C(TclVersion::V8_6))
                .unwrap();
        let recipe = instruction(
            &original,
            1,
            NativeStringMatchScope::PrivateOperands,
            TclVersion::V8_6,
        )
        .unwrap();
        assert_eq!(recipe.operation, NativeStringMatchOperation::Equal);
        assert!(
            instruction(
                &original,
                1,
                NativeStringMatchScope::PublicMember,
                TclVersion::V8_6
            )
            .is_none()
        );
        assert!(matches!(
            select_original(
                &original,
                0,
                NativeStringMatchScope::PrivateOperands,
                TclVersion::V8_6,
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Generic | NativeCompilationSelection::Unknown
        ));
    }
}
