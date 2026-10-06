// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proof-bearing expression parsing, separate from recovery ASTs.

use super::{
    ExprParseFailureReason, NativeFunctionCallSyntax, ParseError, PrattParser, binop_from_text,
};
use crate::expr::{ExprNode, ExprSyntaxError};
use tcl_dialect::model::{Family, SpecProvider};
use tcl_dialect::{DialectProfile, TclVersion};

/// Native diagnostic policy, independently of source tokenisation overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeExprSyntax {
    /// A selected C Tcl release.
    Tcl(TclVersion),
    /// The independently measured Jim 0.84 expression engine.
    Jim084,
    /// Native expression syntax/presentation is not established.
    Unknown,
}

/// Original expression source representation changes during preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionSourceCachePreparation {
    /// Keep the original typed representation until a complete tree is prepared.
    OnSuccess,
    /// Retire the original typed representation before parsing, including failures.
    BeforeParsing,
    /// Apply the selected Jim tokenization/completeness/tree outcome receipt.
    /// Missing delimiters and empty input preserve the original primary.
    JimOutcome,
}

/// A parser's error-code state update, independent of the displayed diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExprSyntaxErrorCodeUpdate {
    /// Preserve the actual interpreter code object without storing a default.
    Unchanged,
    /// Store the exact selected parser code list.
    Set(Vec<u8>),
}

/// Direct parser failure and its separate interpreter propagation update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExprSyntaxErrorState {
    /// Direct expression API update.
    pub direct: NativeExprSyntaxErrorCodeUpdate,
    /// Update at actual interpreter Eval propagation.
    pub eval: NativeExprSyntaxErrorCodeUpdate,
}

impl NativeExprSyntax {
    /// Retain the selected parser's state actions for a proved syntax failure.
    /// Later C parser codes come from the same failure presenter. Unknown
    /// recipes and absent required codes cannot invent guest error state.
    #[must_use]
    pub fn error_state(self, parser_code: Option<&[u8]>) -> Option<NativeExprSyntaxErrorState> {
        use NativeExprSyntaxErrorCodeUpdate as Update;
        let (direct, eval) = match self {
            Self::Tcl(TclVersion::V8_4 | TclVersion::V8_5) => {
                (Update::Unchanged, Update::Set(b"NONE".to_vec()))
            }
            Self::Tcl(_) => {
                let code = parser_code?;
                (Update::Set(code.to_vec()), Update::Set(code.to_vec()))
            }
            Self::Jim084 => (Update::Unchanged, Update::Unchanged),
            Self::Unknown => return None,
        };
        Some(NativeExprSyntaxErrorState { direct, eval })
    }

    /// Representation protocol of the independently selected expression engine.
    /// Source bytes must be materialised on the original object before replacing
    /// its representation. This recipe does not establish a caller or engine.
    #[must_use]
    pub const fn source_cache_preparation(self) -> Option<ExpressionSourceCachePreparation> {
        match self {
            Self::Tcl(TclVersion::V8_4) => Some(ExpressionSourceCachePreparation::OnSuccess),
            Self::Tcl(_) => Some(ExpressionSourceCachePreparation::BeforeParsing),
            Self::Jim084 => Some(ExpressionSourceCachePreparation::JimOutcome),
            Self::Unknown => None,
        }
    }

    /// Argument parsing policy, independently of native function table contents.
    #[must_use]
    pub const fn function_call_syntax(self) -> Option<NativeFunctionCallSyntax> {
        match self {
            Self::Tcl(TclVersion::V8_4) => Some(NativeFunctionCallSyntax::TrailingComma),
            Self::Tcl(_) => Some(NativeFunctionCallSyntax::Strict),
            Self::Jim084 => Some(NativeFunctionCallSyntax::IgnoreEmptyArguments),
            Self::Unknown => None,
        }
    }
}

/// Independent expression parsing and native diagnostic axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprParseContext {
    /// Complete lexical grammar, including caller overrides.
    pub lexer_grammar: tcl_dialect::LexerGrammar,
    /// Release governing expression operators, independently of runtime release.
    pub expr_grammar_base: Option<TclVersion>,
    /// Host word operators retained from the actual source environment.
    pub f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
    /// Native rejection/diagnostic policy.
    pub native_syntax: NativeExprSyntax,
}

impl ExprParseContext {
    /// Copy the selected profile's axes without re-resolving its name.
    #[must_use]
    pub fn for_profile(profile: &DialectProfile) -> Self {
        Self {
            lexer_grammar: profile.grammar,
            expr_grammar_base: profile.expr_grammar_base,
            f5_word_grammar: profile.f5_core_expr_grammar(),
            native_syntax: profile.runtime_version().map_or_else(
                || {
                    if jim_profile(profile)
                        && profile.core_point.is_some_and(|point| {
                            point.release() == tcl_dialect::model::Release::JIM_0_84
                        })
                    {
                        NativeExprSyntax::Jim084
                    } else {
                        NativeExprSyntax::Unknown
                    }
                },
                NativeExprSyntax::Tcl,
            ),
        }
    }
}

/// Why no native expression syntax proof is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprParseUnsupported {
    /// No selected dialect grammar was supplied.
    UnknownProfile,
    /// Native syntax is not selected, so parser rejection alone cannot prove native failure.
    UnknownNativeSyntax,
    /// The lexer emitted recovery tokens or skipped unknown characters.
    UnknownLexing,
    /// The shared parser stopped at its nesting budget.
    DepthLimit,
    /// This release accepts function syntax outside the shared parser's grammar.
    FunctionGrammar,
}

/// A syntax rejection proved by the selected parser grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprSyntaxFailure {
    /// Why the parser rejected the expression.
    pub reason: ExprParseFailureReason,
    /// Lexer token extent at the rejection; EOF has no token.
    pub token: Option<(crate::expr::ExprOffset, crate::expr::ExprOffset)>,
    /// Original lexical rejection, when failure precedes Pratt parsing.
    pub lexical: Option<tcl_lexer::ExprLexicalFailure>,
}

/// Actual native math-function lookup for a rejected bare identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFunctionNameResolution<'a> {
    /// The independently retained native table contains this function name.
    Function(&'a str),
    /// The independently retained closed native table excludes this name.
    NonFunction(&'a str),
    /// Native function binding or table completeness is not established.
    Unknown,
}

/// Exact native diagnostic bytes, independently of error-info stack context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExprSyntaxDiagnostic<Text = String> {
    /// The native interpreter result.
    pub message: Text,
    /// The native diagnostic code list. A baseline NONE observation does not
    /// establish a direct store: consumers apply the selected `error_state`.
    pub error_code: Option<Text>,
}

/// Parsing distinguishes executable structure, proved rejection and lost evidence.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckedExprParse<Text = String> {
    /// Complete syntax represented by the shared AST.
    Parsed(ExprNode<Text>),
    /// Native syntax cannot accept this expression under the selected grammar.
    ProvedSyntaxFailure(ExprSyntaxFailure),
    /// A recovery AST or unsupported grammar must not become a syntax error.
    Unsupported(ExprParseUnsupported),
}

fn jim_profile(profile: &DialectProfile) -> bool {
    profile
        .grammar_union
        .contains(&SpecProvider::Core(Family::Jim))
}

impl ExprSyntaxFailure {
    /// Return the rejected identifier from its retained lexer extent.
    /// Recovery tokens or a changed source/grammar do not provide a name.
    #[must_use]
    pub fn bareword_name(self, source: &str, context: &ExprParseContext) -> Option<String> {
        if self.reason != ExprParseFailureReason::Bareword {
            return None;
        }
        let token = self.rejected_token(source, context)?;
        (token.kind == tcl_lexer::ExprTokenType::Function).then_some(token.text)
    }

    fn rejected_token(
        self,
        source: &str,
        context: &ExprParseContext,
    ) -> Option<tcl_lexer::ExprToken> {
        let (tokens, unknown) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
            source,
            &context.lexer_grammar,
            context.expr_grammar_base,
            context.f5_word_grammar,
        );
        if unknown {
            return None;
        }
        tokens
            .into_iter()
            .find(|token| Some((token.start, token.end)) == self.token)
    }

    /// Present a legacy bareword failure only after resolving its native math name.
    #[must_use]
    pub fn native_diagnostic_with_bareword_resolution(
        self,
        source: &str,
        context: &ExprParseContext,
        resolution: NativeFunctionNameResolution<'_>,
    ) -> Option<NativeExprSyntaxDiagnostic> {
        if self.reason != ExprParseFailureReason::Bareword {
            return self.native_diagnostic_with_context(source, context);
        }
        let (name, function) = match resolution {
            NativeFunctionNameResolution::Function(name) => (name, true),
            NativeFunctionNameResolution::NonFunction(name) => (name, false),
            NativeFunctionNameResolution::Unknown => return None,
        };
        if self.bareword_name(source, context).as_deref() != Some(name) {
            return None;
        }
        let message = match context.native_syntax {
            NativeExprSyntax::Tcl(TclVersion::V8_4) => {
                let shown = source.get(..source.len().min(60))?;
                let reason = if function {
                    "expected parenthesis enclosing function arguments"
                } else {
                    "variable references require preceding $"
                };
                format!("syntax error in expression \"{shown}\": {reason}")
            }
            NativeExprSyntax::Jim084 => format!(
                "syntax error in expression: \"{source}\"{}",
                if function {
                    ": function requires parentheses"
                } else {
                    ""
                }
            ),
            NativeExprSyntax::Tcl(_) => {
                return self.native_diagnostic_with_context(source, context);
            }
            NativeExprSyntax::Unknown => return None,
        };
        Some(NativeExprSyntaxDiagnostic {
            message,
            error_code: Some("NONE".to_owned()),
        })
    }

    /// Present only independently modelled diagnostic bytes for this dialect.
    /// C 8.4 and Jim use legacy EOF wording; other legacy failures abstain.
    #[must_use]
    pub fn native_diagnostic(
        self,
        source: &str,
        profile: Option<&DialectProfile>,
    ) -> Option<NativeExprSyntaxDiagnostic> {
        self.native_diagnostic_with_context(source, &ExprParseContext::for_profile(profile?))
    }

    fn native_diagnostic_message(
        self,
        source: &[u8],
        context: &ExprParseContext,
        version: Option<TclVersion>,
    ) -> Option<Vec<u8>> {
        let message = if version == Some(TclVersion::V8_4) || version.is_none() {
            if source.is_empty() && version.is_none() {
                return Some(b"empty expression".to_vec());
            }
            let token_kind = || {
                tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
                    source,
                    &context.lexer_grammar,
                    context.expr_grammar_base,
                    context.f5_word_grammar,
                )
                .0
                .into_iter()
                .find(|token| Some((token.start, token.end)) == self.token)
                .map(|token| token.kind)
            };
            let reason = match self.reason {
                ExprParseFailureReason::PrematureEnd => "premature end of expression",
                ExprParseFailureReason::UnconsumedTokens if version.is_some() => {
                    "extra tokens at end of expression"
                }
                ExprParseFailureReason::MissingFunctionClose if version.is_some() => {
                    "missing close parenthesis at end of function call"
                }
                ExprParseFailureReason::UnexpectedToken
                    if version.is_some()
                        && token_kind() == Some(tcl_lexer::ExprTokenType::Comma) =>
                {
                    "commas can only separate function arguments"
                }
                ExprParseFailureReason::UnexpectedToken
                    if version.is_some()
                        && token_kind() == Some(tcl_lexer::ExprTokenType::ParenClose) =>
                {
                    "unexpected close parenthesis"
                }
                _ => return None,
            };
            let mut message = b"syntax error in expression \"".to_vec();
            let limit = if version.is_none() {
                source.len()
            } else {
                source.len().min(60)
            };
            message.extend_from_slice(crate::expr::syntax_error::nul_terminated(&source[..limit]));
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(reason.as_bytes());
            message
        } else {
            ExprSyntaxError::diagnose_bytes_with_expression_grammar(
                source,
                &context.lexer_grammar,
                context.expr_grammar_base,
                context.f5_word_grammar,
            )
            .message_bytes(source)
        };
        Some(message)
    }

    /// Native diagnostic bytes from the original counted expression source.
    /// Opaque bytes are never replaced by a Unicode display spelling.
    #[must_use]
    pub fn native_diagnostic_bytes_with_context(
        self,
        source: &[u8],
        context: &ExprParseContext,
    ) -> Option<NativeExprSyntaxDiagnostic<Vec<u8>>> {
        if let Some(failure) = self.lexical {
            return native_lexical_diagnostic(source, context, failure);
        }
        if self.reason == ExprParseFailureReason::UnavailableOperator {
            let NativeExprSyntax::Tcl(version) = context.native_syntax else {
                return None;
            };
            if context.expr_grammar_base != Some(version) || context.f5_word_grammar.is_some() {
                return None;
            }
            let (start, end) = self.token?;
            let word = source.get(start as usize..=end as usize)?;
            if word.is_empty() || !word.iter().all(u8::is_ascii_alphabetic) {
                return None;
            }
            if version == TclVersion::V8_4 {
                return Some(NativeExprSyntaxDiagnostic {
                    message: legacy_source_error(
                        source,
                        "extra tokens at end of expression",
                        false,
                    ),
                    error_code: Some(b"NONE".to_vec()),
                });
            }
        }
        let version = match context.native_syntax {
            NativeExprSyntax::Tcl(version) => Some(version),
            NativeExprSyntax::Jim084 => None,
            NativeExprSyntax::Unknown => return None,
        };
        let diagnose = || {
            ExprSyntaxError::diagnose_bytes_with_expression_grammar(
                source,
                &context.lexer_grammar,
                context.expr_grammar_base,
                context.f5_word_grammar,
            )
        };
        let message = self.native_diagnostic_message(source, context, version)?;
        let code = if version.is_some_and(|version| version >= TclVersion::V8_6) {
            diagnose().error_code().into_bytes()
        } else {
            b"NONE".to_vec()
        };
        Some(NativeExprSyntaxDiagnostic {
            message,
            error_code: Some(code),
        })
    }

    /// Present native diagnostics without losing source grammar overrides.
    #[must_use]
    pub fn native_diagnostic_with_context(
        self,
        source: &str,
        context: &ExprParseContext,
    ) -> Option<NativeExprSyntaxDiagnostic> {
        if let Some(diagnostic) =
            self.native_diagnostic_bytes_with_context(source.as_bytes(), context)
        {
            return Some(NativeExprSyntaxDiagnostic {
                message: String::from_utf8(diagnostic.message).ok()?,
                error_code: diagnostic
                    .error_code
                    .map(String::from_utf8)
                    .transpose()
                    .ok()?,
            });
        }
        if self.reason == ExprParseFailureReason::UnavailableOperator {
            return None;
        }
        let version = match context.native_syntax {
            NativeExprSyntax::Tcl(version) => Some(version),
            NativeExprSyntax::Jim084 => None,
            NativeExprSyntax::Unknown => return None,
        };
        let jim = context.native_syntax == NativeExprSyntax::Jim084;
        let diagnose = || {
            ExprSyntaxError::diagnose_with_expression_grammar(
                source,
                &context.lexer_grammar,
                context.expr_grammar_base,
                context.f5_word_grammar,
            )
        };
        let message = if version == Some(TclVersion::V8_4) || jim {
            if source.is_empty() && jim {
                return Some(NativeExprSyntaxDiagnostic {
                    message: "empty expression".to_owned(),
                    error_code: Some("NONE".to_owned()),
                });
            }
            let reason = if self.reason == ExprParseFailureReason::PrematureEnd {
                "premature end of expression"
            } else if version == Some(TclVersion::V8_4)
                && self.reason == ExprParseFailureReason::MissingFunctionClose
            {
                "missing close parenthesis at end of function call"
            } else if version == Some(TclVersion::V8_4)
                && self.reason == ExprParseFailureReason::UnexpectedToken
                && self.rejected_token(source, context)?.kind == tcl_lexer::ExprTokenType::Comma
            {
                "commas can only separate function arguments"
            } else if version == Some(TclVersion::V8_4)
                && self.reason == ExprParseFailureReason::UnexpectedToken
                && self.rejected_token(source, context)?.kind
                    == tcl_lexer::ExprTokenType::ParenClose
            {
                "unexpected close parenthesis"
            } else {
                return None;
            };
            let limit = if jim {
                source.len()
            } else {
                source.len().min(60)
            };
            let shown = source.get(..limit)?;
            format!("syntax error in expression \"{shown}\": {reason}")
        } else {
            diagnose().message(source)
        };
        let error_code = Some(
            if version.is_some_and(|version| version >= TclVersion::V8_6) {
                diagnose().error_code()
            } else {
                "NONE".to_owned()
            },
        );
        Some(NativeExprSyntaxDiagnostic {
            message,
            error_code,
        })
    }
}

/// Parse with explicit evidence; unsupported syntax and depth never imply native rejection.
#[must_use]
pub fn parse_expr_checked_for_profile(
    source: &str,
    profile: Option<&DialectProfile>,
) -> CheckedExprParse {
    let Some(profile) = profile else {
        return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownProfile);
    };
    parse_expr_checked_with_context(source, &ExprParseContext::for_profile(profile))
}

/// Checked parsing with the actual lexical axes and independent native policy.
#[must_use]
pub fn parse_expr_checked_with_context(
    source: &str,
    context: &ExprParseContext,
) -> CheckedExprParse {
    match parse_expr_bytes_checked_with_context(source.as_bytes(), context) {
        CheckedExprParse::Parsed(node) => CheckedExprParse::Parsed(
            node.map_text(|text| String::from_utf8(text).expect("Unicode original source leaves")),
        ),
        CheckedExprParse::ProvedSyntaxFailure(failure) => {
            CheckedExprParse::ProvedSyntaxFailure(failure)
        }
        CheckedExprParse::Unsupported(reason) => CheckedExprParse::Unsupported(reason),
    }
}

/// Parse original expression bytes using the same selected Pratt grammar.
#[must_use]
pub fn parse_expr_bytes_checked_with_context(
    source: &[u8],
    context: &ExprParseContext,
) -> CheckedExprParse<Vec<u8>> {
    prepare_expr_bytes_checked_with_context(source, context).parsed
}

/// Native parsing with independently retained representation actions.
/// A displayed prefix error cannot erase a later tokenization failure's
/// representation action. Unknown parser capabilities issue no action.
#[must_use]
pub fn prepare_expr_bytes_checked_with_context(
    source: &[u8],
    context: &ExprParseContext,
) -> super::super::native_objects::PreparedNativeExprParse {
    let (mut raw, failures) = tcl_lexer::tokenise_expr_bytes_with_failures(
        source,
        &context.lexer_grammar,
        context.expr_grammar_base,
        context.f5_word_grammar,
    );
    if context.native_syntax != NativeExprSyntax::Jim084 {
        return super::super::native_objects::PreparedNativeExprParse {
            parsed: parse_native_tokens(source, context, &mut raw, failures),
            jim: None,
        };
    }
    let preparation_tokens = raw.clone();
    let preparation_failures = failures.clone();
    let parsed = parse_native_tokens(source, context, &mut raw, failures);
    let jim = super::super::native_objects::jim_expression_preparation(
        source,
        context,
        &preparation_tokens,
        &preparation_failures,
        &parsed,
    );
    super::super::native_objects::PreparedNativeExprParse { parsed, jim }
}

pub(super) fn parse_native_tokens(
    source: &[u8],
    context: &ExprParseContext,
    raw: &mut Vec<tcl_lexer::ExprToken<Vec<u8>>>,
    failures: Vec<tcl_lexer::ExprLexicalFailure>,
) -> CheckedExprParse<Vec<u8>> {
    if context.native_syntax == NativeExprSyntax::Unknown {
        return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax);
    }
    for failure in failures {
        if context.native_syntax == NativeExprSyntax::Tcl(TclVersion::V8_4)
            && failure.kind == tcl_lexer::ExprLexicalFailureKind::MissingVariableName
        {
            if let Some(token) = raw
                .iter_mut()
                .find(|token| token.start as usize == failure.at)
            {
                token.kind = tcl_lexer::ExprTokenType::String;
            }
            continue;
        }
        if failure.kind == tcl_lexer::ExprLexicalFailureKind::InvalidCharacter
            && source.get(failure.at).is_some_and(|byte| !byte.is_ascii())
            && std::str::from_utf8(&source[failure.at..]).is_ok()
        {
            return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownLexing);
        }
        let prefix: Vec<_> = raw
            .iter()
            .filter(|token| (token.end as usize) < failure.at)
            .cloned()
            .collect();
        if let CheckedExprParse::ProvedSyntaxFailure(earlier) =
            parse_checked_tokens(prefix, false, context)
            && context.native_syntax != NativeExprSyntax::Jim084
            && earlier.token.is_some()
            && !matches!(
                earlier.reason,
                ExprParseFailureReason::MissingFunctionClose | ExprParseFailureReason::PrematureEnd
            )
        {
            return CheckedExprParse::ProvedSyntaxFailure(earlier);
        }
        let Ok(at) = crate::expr::ExprOffset::try_from(failure.at) else {
            return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownLexing);
        };
        return CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
            reason: ExprParseFailureReason::UnexpectedToken,
            token: Some((at, at)),
            lexical: Some(failure),
        });
    }
    parse_checked_tokens(std::mem::take(raw), false, context)
}

fn native_lexical_diagnostic(
    source: &[u8],
    context: &ExprParseContext,
    failure: tcl_lexer::ExprLexicalFailure,
) -> Option<NativeExprSyntaxDiagnostic<Vec<u8>>> {
    use tcl_lexer::ExprLexicalFailureKind as Kind;
    if context.native_syntax == NativeExprSyntax::Unknown {
        return None;
    }
    let mut error_code = b"NONE".to_vec();
    let mut message = match failure.kind {
        Kind::MissingQuote => match context.native_syntax {
            NativeExprSyntax::Jim084 => b"missing quote".to_vec(),
            NativeExprSyntax::Tcl(_) => b"missing \"".to_vec(),
            NativeExprSyntax::Unknown => return None,
        },
        Kind::MissingBrace => b"missing close-brace".to_vec(),
        Kind::MissingBracket => nested_command_lexical_message(source, context, failure.at)?,
        Kind::Variable(message) => message.as_bytes().to_vec(),
        Kind::MissingVariableName | Kind::InvalidCharacter => match context.native_syntax {
            NativeExprSyntax::Tcl(TclVersion::V8_4) => {
                let prefix = source.get(..failure.at)?;
                let complete = matches!(
                    parse_expr_bytes_checked_with_context(prefix, context),
                    CheckedExprParse::Parsed(_)
                );
                legacy_source_error(
                    source,
                    if complete {
                        "extra tokens at end of expression"
                    } else {
                        "character not legal in expressions"
                    },
                    false,
                )
            }
            NativeExprSyntax::Tcl(version) => {
                let error = ExprSyntaxError::lexical_character(source, failure.at)?;
                if version >= TclVersion::V8_6 {
                    error_code = error.error_code().into_bytes();
                }
                error.message_bytes(source)
            }
            NativeExprSyntax::Jim084 => legacy_source_error(source, "", true),
            NativeExprSyntax::Unknown => return None,
        },
    };
    if !matches!(
        failure.kind,
        Kind::MissingVariableName | Kind::InvalidCharacter
    ) && let NativeExprSyntax::Tcl(version) = context.native_syntax
        && version != TclVersion::V8_4
    {
        message.extend_from_slice(&ExprSyntaxError::lexical_context(source, failure.at));
        if version >= TclVersion::V8_6 {
            error_code = b"TCL PARSE EXPR UNBALANCED".to_vec();
        }
    }
    Some(NativeExprSyntaxDiagnostic {
        message,
        error_code: Some(error_code),
    })
}

fn nested_command_lexical_message(
    source: &[u8],
    context: &ExprParseContext,
    at: usize,
) -> Option<Vec<u8>> {
    if !matches!(context.native_syntax, NativeExprSyntax::Tcl(_)) {
        return Some(b"missing close-bracket".to_vec());
    }
    // Tcl parses the original nested script before diagnosing its absent final
    // bracket. Its brace, quote and variable errors therefore take precedence.
    let body = source.get(at.checked_add(1)?..)?;
    let cut = tcl_lexer::first_parse_cut_image_checked(
        &tcl_lexer::SourceImage::native(body),
        tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar),
    )
    .ok()?;
    Some(cut.map_or(b"missing close-bracket".to_vec(), |cut| {
        cut.message.as_bytes().to_vec()
    }))
}

fn legacy_source_error(source: &[u8], reason: &str, jim: bool) -> Vec<u8> {
    let mut message = if jim {
        b"syntax error in expression: \"".to_vec()
    } else {
        b"syntax error in expression \"".to_vec()
    };
    let shown = if jim {
        source
    } else {
        &source[..source.len().min(60)]
    };
    message.extend_from_slice(crate::expr::syntax_error::nul_terminated(shown));
    message.push(b'"');
    if !reason.is_empty() {
        message.extend_from_slice(b": ");
        message.extend_from_slice(reason.as_bytes());
    }
    message
}

fn parse_checked_tokens<Text: super::ExprText>(
    raw: Vec<tcl_lexer::ExprToken<Text>>,
    unknown: bool,
    context: &ExprParseContext,
) -> CheckedExprParse<Text> {
    if unknown {
        return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownLexing);
    }
    if context.native_syntax == NativeExprSyntax::Unknown {
        return CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax);
    }
    let tokens: Vec<_> = raw
        .into_iter()
        .filter(|token| !token.kind.is_skipped())
        .collect();
    let unavailable = context.expr_grammar_base.and_then(|version| {
        tokens.iter().find(|token| {
            token.kind == tcl_lexer::ExprTokenType::Operator
                && binop_from_text(token.text.try_text().unwrap_or_default())
                    .and_then(|op| op.spec().expr_grammar_min_version)
                    .is_some_and(|since| version < since)
        })
    });
    if let Some(token) = unavailable {
        return CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
            reason: ExprParseFailureReason::UnavailableOperator,
            token: Some((token.start, token.end)),
            lexical: None,
        });
    }
    if tokens.is_empty() {
        return CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
            reason: ExprParseFailureReason::PrematureEnd,
            token: None,
            lexical: None,
        });
    }
    let mut parser = PrattParser::new(
        &tokens,
        context.lexer_grammar.numbers,
        context.expr_grammar_base,
    );
    parser.variable_config = tcl_lexer::LexerConfig::default().with_grammar(context.lexer_grammar);
    parser.function_syntax = context
        .native_syntax
        .function_call_syntax()
        .unwrap_or_default();
    let parsed = match parser.expression(0) {
        Ok(node) if parser.pos == tokens.len() => return CheckedExprParse::Parsed(node),
        Ok(_) => ParseError::Syntax {
            reason: ExprParseFailureReason::UnconsumedTokens,
            in_function: false,
            token: parser.peek().map(|token| (token.start, token.end)),
        },
        Err(error) => error,
    };
    match parsed {
        ParseError::DepthLimit => CheckedExprParse::Unsupported(ExprParseUnsupported::DepthLimit),
        ParseError::Syntax {
            reason,
            token,
            in_function: true,
        } if legacy_function_failure_requires_lookup(context, reason, token, &tokens) => {
            CheckedExprParse::Unsupported(ExprParseUnsupported::FunctionGrammar)
        }
        ParseError::Syntax { reason, token, .. } => {
            CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
                reason,
                token,
                lexical: None,
            })
        }
    }
}

fn legacy_function_failure_requires_lookup<Text: super::ExprText>(
    context: &ExprParseContext,
    reason: ExprParseFailureReason,
    token: Option<(crate::expr::ExprOffset, crate::expr::ExprOffset)>,
    tokens: &[tcl_lexer::ExprToken<Text>],
) -> bool {
    match context.native_syntax {
        NativeExprSyntax::Jim084 => true,
        NativeExprSyntax::Tcl(TclVersion::V8_4) => {
            !(matches!(
                reason,
                ExprParseFailureReason::MissingFunctionClose | ExprParseFailureReason::PrematureEnd
            ) || (reason == ExprParseFailureReason::UnexpectedToken
                && tokens.iter().any(|candidate| {
                    Some((candidate.start, candidate.end)) == token
                        && matches!(
                            candidate.kind,
                            tcl_lexer::ExprTokenType::ParenClose | tcl_lexer::ExprTokenType::Comma
                        )
                })))
        }
        NativeExprSyntax::Tcl(_) | NativeExprSyntax::Unknown => false,
    }
}

#[cfg(test)]
mod native_byte_tests {
    use super::*;

    #[test]
    fn counted_native_expression_leaves_preserve_opaque_quotes_and_variables() {
        let mut contexts: Vec<_> = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(|version| {
                ExprParseContext::for_profile(
                    tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                        .expect("actual C profile"),
                )
            })
            .collect();
        let jim = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        contexts.push(ExprParseContext::for_profile(&jim));
        for context in contexts {
            for source in [
                b"\"\xff\0tail\"".as_slice(),
                b"{\xff\0tail}",
                b"${\xff}",
                b"$a(\xff)",
                b"[set {\xff}]",
            ] {
                let CheckedExprParse::Parsed(node) =
                    parse_expr_bytes_checked_with_context(source, &context)
                else {
                    panic!("native source was declined: {source:?}");
                };
                let (ExprNode::String { text, .. }
                | ExprNode::Var { text, .. }
                | ExprNode::Command { text, .. }) = node
                else {
                    panic!("unexpected leaf");
                };
                assert_eq!(text, source);
            }
        }
    }

    #[test]
    fn native_syntax_diagnostic_preserves_opaque_source_bytes() {
        for version in tcl_dialect::TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                .expect("actual C profile");
            let context = ExprParseContext::for_profile(profile);
            let source = b"\"\xff\" +";
            let CheckedExprParse::ProvedSyntaxFailure(failure) =
                parse_expr_bytes_checked_with_context(source, &context)
            else {
                panic!("expected syntax failure");
            };
            let message = failure
                .native_diagnostic_bytes_with_context(source, &context)
                .expect("native diagnostic")
                .message;
            let expected = if version == tcl_dialect::TclVersion::V8_4 {
                b"syntax error in expression \"\"\xff\" +\": premature end of expression".as_slice()
            } else {
                b"missing operand at _@_\nin expression \"\"\xff\" +_@_\""
            };
            assert_eq!(message, expected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(version: TclVersion) -> ExprParseContext {
        ExprParseContext::for_profile(DialectProfile::find(version.dialect_profile_name()).unwrap())
    }

    #[test]
    fn nested_command_syntax_preserves_35_original_c_delimiter_diagnostics() {
        fn bytes(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for line in
            include_str!("../../tests/data/native_nested_expression_syntax/native.tsv").lines()
        {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] == "jim" {
                continue;
            }
            let version = TclVersion::ALL
                .into_iter()
                .find(|version| version.dialect_name() == format!("tcl{}", row[0]))
                .unwrap();
            let context = context(version);
            let source = bytes(row[2]);
            let CheckedExprParse::ProvedSyntaxFailure(failure) =
                parse_expr_bytes_checked_with_context(&source, &context)
            else {
                panic!("{line}: original syntax not rejected")
            };
            assert_eq!(
                failure
                    .native_diagnostic_bytes_with_context(&source, &context)
                    .unwrap()
                    .message,
                bytes(row[4]),
                "{line}",
            );
            let mut unknown = context;
            unknown.native_syntax = NativeExprSyntax::Unknown;
            assert!(
                failure
                    .native_diagnostic_bytes_with_context(&source, &unknown)
                    .is_none()
            );
            compared += 1;
        }
        assert_eq!(compared, 35);
    }

    #[test]
    fn counted_native_lexical_failures_keep_original_bytes_and_native_chunks() {
        for &(engine, case, source, expected) in
            tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_FAILURES
        {
            let context = if engine == "jim" {
                jim_context()
            } else {
                ExprParseContext::for_profile(tcl_dialect::DialectProfile::find(engine).unwrap())
            };
            let message = if engine == "jim" {
                match crate::expr::jim_function_tree::prepare_jim_function_tree_bytes(
                    source,
                    &context,
                    |_| crate::expr::jim_function_tree::NativeFunctionArity::Absent,
                ) {
                    crate::expr::jim_function_tree::NativeFunctionTree::Rejected(diagnostic) => {
                        diagnostic.message
                    }
                    other => panic!("{engine}/{case}: {other:?}"),
                }
            } else {
                let CheckedExprParse::ProvedSyntaxFailure(failure) =
                    parse_expr_bytes_checked_with_context(source, &context)
                else {
                    panic!("{engine}/{case}: original syntax not rejected")
                };
                failure
                    .native_diagnostic_bytes_with_context(source, &context)
                    .unwrap()
                    .message
            };
            assert_eq!(message, expected, "{engine}/{case}");
        }
    }

    #[test]
    fn counted_native_syntax_errors_preserve_direct_state_until_eval() {
        for &(engine, case, source, message, direct, propagated) in
            tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
        {
            let context = if engine == "jim" {
                jim_context()
            } else {
                ExprParseContext::for_profile(DialectProfile::find(engine).unwrap())
            };
            let diagnostic = if engine == "jim" {
                let crate::expr::jim_function_tree::NativeFunctionTree::Rejected(diagnostic) =
                    crate::expr::jim_function_tree::prepare_jim_function_tree_bytes(
                        source,
                        &context,
                        |_| crate::expr::jim_function_tree::NativeFunctionArity::Absent,
                    )
                else {
                    panic!("{engine}/{case}")
                };
                diagnostic
            } else {
                let CheckedExprParse::ProvedSyntaxFailure(failure) =
                    parse_expr_bytes_checked_with_context(source, &context)
                else {
                    panic!("{engine}/{case}")
                };
                failure
                    .native_diagnostic_bytes_with_context(source, &context)
                    .unwrap()
            };
            assert_eq!(diagnostic.message, message, "{engine}/{case}");
            let state = context
                .native_syntax
                .error_state(diagnostic.error_code.as_deref())
                .unwrap();
            let apply = |update| match update {
                NativeExprSyntaxErrorCodeUpdate::Unchanged => b"PROBE BEFORE".to_vec(),
                NativeExprSyntaxErrorCodeUpdate::Set(code) => code,
            };
            assert_eq!(apply(state.direct), direct, "{engine}/{case} Direct");
            assert_eq!(apply(state.eval), propagated, "{engine}/{case} Eval");
        }
        assert!(
            NativeExprSyntax::Unknown
                .error_state(Some(b"NONE"))
                .is_none()
        );
        assert!(
            NativeExprSyntax::Tcl(TclVersion::V9_0)
                .error_state(None)
                .is_none()
        );
    }

    #[test]
    fn legacy_bare_dollar_is_a_literal_without_variable_lookup() {
        let legacy = context(TclVersion::V8_4);
        assert!(
            matches!(parse_expr_bytes_checked_with_context(b"$", &legacy), CheckedExprParse::Parsed(ExprNode::String { text, .. }) if text == b"$")
        );
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert!(matches!(
                parse_expr_bytes_checked_with_context(b"$", &context(version)),
                CheckedExprParse::ProvedSyntaxFailure(_)
            ));
        }
    }

    #[test]
    fn legacy_function_missing_operand_rejects_before_function_lookup() {
        let context = context(TclVersion::V8_4);
        for source in ["future_function(1+)", "abs(1+)", "(1+)"] {
            let CheckedExprParse::ProvedSyntaxFailure(error) =
                parse_expr_checked_with_context(source, &context)
            else {
                panic!("{source}");
            };
            assert_eq!(error.reason, ExprParseFailureReason::UnexpectedToken);
            let diagnostic = error
                .native_diagnostic_with_context(source, &context)
                .unwrap();
            assert_eq!(
                diagnostic.message,
                format!("syntax error in expression \"{source}\": unexpected close parenthesis")
            );
            assert_eq!(diagnostic.error_code.as_deref(), Some("NONE"));
        }
        assert_eq!(
            parse_expr_checked_with_context("abs(nope)", &context),
            CheckedExprParse::Unsupported(ExprParseUnsupported::FunctionGrammar)
        );
    }

    #[test]
    fn legacy_function_delimiters_reject_before_function_lookup() {
        let context = context(TclVersion::V8_4);
        for (source, reason) in [
            ("abs(,)", "commas can only separate function arguments"),
            (
                "future_function(,)",
                "commas can only separate function arguments",
            ),
            ("abs(1,,)", "commas can only separate function arguments"),
            (
                "abs(1 2)",
                "missing close parenthesis at end of function call",
            ),
            (
                "future_function(1",
                "missing close parenthesis at end of function call",
            ),
        ] {
            let CheckedExprParse::ProvedSyntaxFailure(error) =
                parse_expr_checked_with_context(source, &context)
            else {
                panic!("{source}");
            };
            let diagnostic = error
                .native_diagnostic_with_context(source, &context)
                .unwrap();
            assert_eq!(
                diagnostic.message,
                format!("syntax error in expression \"{source}\": {reason}")
            );
            assert_eq!(diagnostic.error_code.as_deref(), Some("NONE"));
        }
    }

    fn jim_context() -> ExprParseContext {
        let profile = DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        ExprParseContext::for_profile(&profile)
    }

    #[test]
    fn native_function_argument_grammars_retain_empty_segment_policy() {
        let legacy = context(TclVersion::V8_4);
        let jim = jim_context();
        for source in ["abs(1,)", "pow(2,3,)"] {
            assert!(matches!(
                parse_expr_checked_with_context(source, &legacy),
                CheckedExprParse::Parsed(ExprNode::Call { .. })
            ));
            assert!(matches!(
                parse_expr_checked_with_context(source, &jim),
                CheckedExprParse::Parsed(ExprNode::Call { .. })
            ));
            assert!(matches!(
                parse_expr_checked_with_context(source, &context(TclVersion::V8_6)),
                CheckedExprParse::ProvedSyntaxFailure(_)
            ));
        }
        for (source, count) in [
            ("abs(,1)", 1),
            ("abs(1,,)", 1),
            ("pow(2,,3)", 2),
            ("abs(,)", 0),
        ] {
            assert!(
                matches!(parse_expr_checked_with_context(source, &jim), CheckedExprParse::Parsed(ExprNode::Call { args, .. }) if args.len() == count)
            );
            assert!(matches!(
                parse_expr_checked_with_context(source, &legacy),
                CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
                    reason: ExprParseFailureReason::UnexpectedToken,
                    ..
                })
            ));
        }
    }

    #[test]
    fn legacy_bareword_presentation_requires_an_actual_function_lookup() {
        for (context, ordinary, function) in [
            (
                context(TclVersion::V8_4),
                "syntax error in expression \"nope\": variable references require preceding $",
                "syntax error in expression \"abs\": expected parenthesis enclosing function arguments",
            ),
            (
                jim_context(),
                "syntax error in expression: \"nope\"",
                "syntax error in expression: \"abs\": function requires parentheses",
            ),
        ] {
            for (source, resolution, expected) in [
                (
                    "nope",
                    NativeFunctionNameResolution::NonFunction("nope"),
                    ordinary,
                ),
                (
                    "abs",
                    NativeFunctionNameResolution::Function("abs"),
                    function,
                ),
            ] {
                let CheckedExprParse::ProvedSyntaxFailure(error) =
                    parse_expr_checked_with_context(source, &context)
                else {
                    panic!("{source}")
                };
                assert_eq!(
                    error.bareword_name(source, &context).as_deref(),
                    Some(source)
                );
                assert!(
                    error
                        .native_diagnostic_with_context(source, &context)
                        .is_none()
                );
                assert!(
                    error
                        .native_diagnostic_with_bareword_resolution(
                            source,
                            &context,
                            NativeFunctionNameResolution::Unknown
                        )
                        .is_none()
                );
                let diagnostic = error
                    .native_diagnostic_with_bareword_resolution(source, &context, resolution)
                    .unwrap();
                assert_eq!(diagnostic.message, expected);
                assert_eq!(diagnostic.error_code.as_deref(), Some("NONE"));
                assert!(
                    error
                        .native_diagnostic_with_bareword_resolution(
                            source,
                            &context,
                            NativeFunctionNameResolution::NonFunction("different")
                        )
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn premature_end_uses_native_release_messages_and_error_codes() {
        for version in TclVersion::ALL {
            let context = context(version);
            let CheckedExprParse::ProvedSyntaxFailure(error) =
                parse_expr_checked_with_context("+", &context)
            else {
                panic!("{version:?}")
            };
            let diagnostic = error.native_diagnostic_with_context("+", &context).unwrap();
            assert_eq!(
                diagnostic.message,
                if version == TclVersion::V8_4 {
                    "syntax error in expression \"+\": premature end of expression"
                } else {
                    "missing operand at _@_\nin expression \"+_@_\""
                }
            );
            assert_eq!(
                diagnostic.error_code.as_deref(),
                Some(if version >= TclVersion::V8_6 {
                    "TCL PARSE EXPR MISSING"
                } else {
                    "NONE"
                })
            );
        }
    }

    #[test]
    fn unsupported_function_grammar_and_depth_do_not_prove_native_rejection() {
        let legacy = context(TclVersion::V8_4);
        assert!(
            matches!(parse_expr_checked_with_context("abs(1,)", &legacy),
            CheckedExprParse::Parsed(ExprNode::Call { args, .. }) if args.len() == 1)
        );
        let modern = context(TclVersion::V8_6);
        assert!(matches!(
            parse_expr_checked_with_context("abs(1,)", &modern),
            CheckedExprParse::ProvedSyntaxFailure(_)
        ));
        let nested = format!("{}1{}", "(".repeat(9000), ")".repeat(9000));
        assert_eq!(
            parse_expr_checked_with_context(&nested, &modern),
            CheckedExprParse::Unsupported(ExprParseUnsupported::DepthLimit)
        );
        for source in ["{bad", "\"bad", "[bad", "$a(b"] {
            assert!(matches!(
                parse_expr_checked_with_context(source, &modern),
                CheckedExprParse::ProvedSyntaxFailure(ExprSyntaxFailure {
                    lexical: Some(_),
                    ..
                })
            ));
            let mut unknown = modern;
            unknown.native_syntax = NativeExprSyntax::Unknown;
            assert_eq!(
                parse_expr_checked_with_context(source, &unknown),
                CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax)
            );
        }
        assert!(matches!(
            parse_expr_checked_with_context("future_function(1)", &modern),
            CheckedExprParse::Parsed(ExprNode::Call { .. })
        ));
    }

    #[test]
    fn unavailable_native_word_operator_keeps_original_operand_bytes() {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            let selected = context(version);
            for source in [&b"{a} lt {b}"[..], &b"{\xff} lt {b}"[..]] {
                let CheckedExprParse::ProvedSyntaxFailure(error) =
                    parse_expr_bytes_checked_with_context(source, &selected)
                else {
                    panic!("unavailable operator {version:?}")
                };
                let diagnostic = error
                    .native_diagnostic_bytes_with_context(source, &selected)
                    .expect("authentic native word diagnostic");
                let expected = if version == TclVersion::V8_4 {
                    let mut bytes = b"syntax error in expression \"".to_vec();
                    bytes.extend_from_slice(source);
                    bytes.extend_from_slice(b"\": extra tokens at end of expression");
                    bytes
                } else {
                    let mut bytes = b"invalid bareword \"lt\"\nin expression \"".to_vec();
                    bytes.extend_from_slice(source);
                    bytes.extend_from_slice(
                        b"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...",
                    );
                    bytes
                };
                assert_eq!(diagnostic.message, expected);
            }
        }
    }

    #[test]
    fn checked_context_retains_independent_numeral_and_operator_axes() {
        let mut selected = context(TclVersion::V8_6);
        selected.lexer_grammar.numbers = context(TclVersion::V9_0).lexer_grammar.numbers;
        assert!(matches!(
            parse_expr_checked_with_context("0d99", &selected),
            CheckedExprParse::Parsed(_)
        ));
        selected.expr_grammar_base = Some(TclVersion::V8_4);
        let CheckedExprParse::ProvedSyntaxFailure(error) =
            parse_expr_checked_with_context("1 ** 2", &selected)
        else {
            panic!("selected operator grammar must reject exponentiation")
        };
        assert_eq!(error.reason, ExprParseFailureReason::UnavailableOperator);
        assert!(
            error
                .native_diagnostic_with_context("1 ** 2", &selected)
                .is_none()
        );
        for version in [TclVersion::V8_5, TclVersion::V8_6] {
            let selected = context(version);
            let CheckedExprParse::ProvedSyntaxFailure(error) =
                parse_expr_checked_with_context("1 lt 2", &selected)
            else {
                panic!("{version:?}")
            };
            let diagnostic = error
                .native_diagnostic_bytes_with_context(b"1 lt 2", &selected)
                .expect("native unavailable word operator");
            assert_eq!(diagnostic.message, b"invalid bareword \"lt\"\nin expression \"1 lt 2\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...");
            assert_eq!(
                diagnostic.error_code.as_deref(),
                Some(if version == TclVersion::V8_5 {
                    &b"NONE"[..]
                } else {
                    &b"TCL PARSE EXPR BAREWORD"[..]
                })
            );
        }
        selected.native_syntax = NativeExprSyntax::Unknown;
        assert_eq!(
            parse_expr_checked_with_context("+", &selected),
            CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax)
        );
        assert_eq!(
            parse_expr_checked_for_profile("+", None),
            CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownProfile)
        );
    }
}
