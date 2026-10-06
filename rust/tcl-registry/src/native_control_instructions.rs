// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native if/while/for/catch compiler recipes.

use crate::InvocationDialect;
use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationGrammar, NativeCompiledBodyContext,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep as Visit,
    native_control_body_span, project_native_local_scalar,
};
use crate::native_expression_program::{
    NativeExpressionProgram, prepare_native_expression_program,
};
use tcl_lexer::Span;

/// A native script operand, including catch/for's substituted-script path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeControlBody {
    /// Original compiler operand evaluated by a dynamic body path.
    pub operand: NativeCompilerWordOperand,
    /// Static body extent; absent requires actual `EVAL_STK` semantics.
    pub script: Option<Span>,
}

/// A fresh-object Boolean probe or retained static expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControlTest {
    /// Native GetBooleanFromObj(NULL) accepted the original literal.
    Constant(bool),
    /// Native expression compilation is reached for this operand.
    Expression(NativeExpressionProgram),
}

/// One reachable conditional clause in source/compiler order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeConditionalClause {
    /// No test denotes the terminal default clause.
    pub test: Option<NativeControlTest>,
    /// Body compiled only when the native Boolean probe did not prune it.
    pub body: NativeControlBody,
}

/// Native catch stack layout and result-store order selected by release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCatchProtocol {
    /// Separate success/error epilogues, with the range opened before substitution.
    Tcl84,
    /// Result then code on the stack; result is stored before options.
    Tcl85,
    /// Result then code on the stack; options are stored before result.
    Tcl86,
    /// Code then result on the stack, using the native SWAP instruction.
    Tcl91,
}

impl NativeCatchProtocol {
    /// Whether the result store precedes the supplied options store.
    #[must_use]
    pub const fn result_before_options(self) -> bool {
        matches!(self, Self::Tcl84 | Self::Tcl85)
    }

    /// Whether `END_CATCH` publishes and resets the interpreter result before stores.
    /// Tcl 8.4 only closes the protected range at this instruction.
    #[must_use]
    pub const fn resets_result_before_stores(self) -> bool {
        !matches!(self, Self::Tcl84)
    }
}

/// Structured instructions selected by the original registered C compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControlInstruction {
    /// Reachable conditional clauses; masked bodies never get compiler visits.
    Conditional(Vec<NativeConditionalClause>),
    /// Rotated loop, whose body is compiled before a nonconstant test.
    While {
        /// Native condition, including a pruned false loop.
        test: NativeControlTest,
        /// Original repeated script.
        body: NativeControlBody,
    },
    /// Start, body, next, test preparation and separate loop ranges.
    For {
        /// Original one-time start script, possibly dynamically constructed.
        start: NativeControlBody,
        /// Original repeated expression.
        test: NativeExpressionProgram,
        /// Original next script; CONTINUE propagates from this range.
        next: NativeControlBody,
        /// Original repeated script; CONTINUE enters next.
        body: NativeControlBody,
    },
    /// Protected body evaluation and the release's exception-stack lifetime.
    Catch {
        /// Original release's exception-stack and store protocol.
        protocol: NativeCatchProtocol,
        /// Original protected body.
        body: NativeControlBody,
        /// Result local declared before body preparation.
        result: Option<Vec<u8>>,
        /// Options local declared after result; store order follows the protocol.
        options: Option<Vec<u8>>,
    },
}

impl NativeControlInstruction {
    /// Native compiler annotation for a reached original body operand.
    #[must_use]
    pub fn body_error_context(
        &self,
        operand: &NativeCompilerWordOperand,
    ) -> Option<crate::native_compilation::NativeCompiledBodyErrorContext> {
        use crate::native_compilation::NativeCompiledBodyErrorContext as Context;
        match self {
            Self::Conditional(clauses) => clauses.iter().find_map(|clause| {
                (&clause.body.operand == operand).then_some(if clause.test.is_some() {
                    Context::IfThen
                } else {
                    Context::IfElse
                })
            }),
            Self::While { body, .. } if &body.operand == operand => Some(Context::WhileBody),
            Self::For {
                start, body, next, ..
            } => {
                if &start.operand == operand {
                    Some(Context::ForInitial)
                } else if &body.operand == operand {
                    Some(Context::ForBody)
                } else if &next.operand == operand {
                    Some(Context::ForNext)
                } else {
                    None
                }
            }
            Self::Catch { body, .. } if &body.operand == operand => Some(Context::None),
            _ => None,
        }
    }

    /// Native compiler annotation for a reached original expression operand.
    #[must_use]
    pub fn expression_error_context(
        &self,
        operand: &NativeCompilerWordOperand,
    ) -> Option<crate::native_compilation::NativeCompiledExpressionErrorContext> {
        use crate::native_compilation::NativeCompiledExpressionErrorContext as Context;
        self.expression_program(operand)?;
        Some(match self {
            Self::Conditional(_) => Context::IfTest,
            Self::While { .. } => Context::WhileTest,
            Self::For { .. } => Context::ForTest,
            Self::Catch { .. } => return None,
        })
    }

    /// Borrow the already checked tree at one reached expression preparation.
    /// This does not parse text or grant runtime operand authority.
    #[must_use]
    pub fn expression_program(
        &self,
        operand: &NativeCompilerWordOperand,
    ) -> Option<&NativeExpressionProgram> {
        match self {
            Self::Conditional(clauses) => {
                clauses
                    .iter()
                    .find_map(|clause| match clause.test.as_ref()? {
                        NativeControlTest::Expression(program) if &program.operand == operand => {
                            Some(program)
                        }
                        _ => None,
                    })
            }
            Self::While {
                test: NativeControlTest::Expression(program),
                ..
            }
            | Self::For { test: program, .. }
                if &program.operand == operand =>
            {
                Some(program)
            }
            _ => None,
        }
    }
}

/// Unavailable compiler syntax or original geometry, not a genuine decline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeControlInstructionUnavailable {
    /// The selected release or original parser projection is unavailable.
    Geometry,
    /// The retained expression parser cannot prove the native program.
    Expression,
    /// This descriptor is not one of these compiler recipes.
    Operation,
}

pub(crate) fn native_control_body(
    words: &NativeCompilerWords<'_>,
    word: &NativeProjectedCompilerWord,
) -> Result<NativeControlBody, NativeControlInstructionUnavailable> {
    Ok(NativeControlBody {
        operand: word.operand.clone(),
        script: simple(word)
            .then(|| native_control_body_span(words, &word.operand))
            .transpose()
            .map_err(|_| NativeControlInstructionUnavailable::Geometry)?,
    })
}
pub(crate) fn visit_body(
    visits: &mut Vec<Visit>,
    body: &NativeControlBody,
    context: NativeCompiledBodyContext,
) {
    visits.push(match body.script {
        Some(span) => Visit::Script {
            operand: body.operand.clone(),
            span,
            context,
        },
        None => Visit::Word(body.operand.clone()),
    });
}
fn simple(word: &NativeProjectedCompilerWord) -> bool {
    use crate::native_compilation::NativeCompilationWordShape as Shape;
    matches!(
        word.shape,
        Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
    )
}
fn boolean(word: &NativeProjectedCompilerWord, dialect: InvocationDialect) -> Option<bool> {
    use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
    let conversion = dialect
        .native_scalar_getter_protocol()?
        .fresh_conversion(NativeScalarGetterKind::Boolean, word.literal.as_deref()?)?;
    match conversion.outcome().ok()? {
        NativeScalarGetterValue::Boolean(value) => Some(value),
        _ => None,
    }
}
fn test(
    words: &NativeCompilerWords<'_>,
    word: &NativeProjectedCompilerWord,
    dialect: InvocationDialect,
) -> Result<NativeControlTest, NativeControlInstructionUnavailable> {
    if let Some(value) = boolean(word, dialect) {
        return Ok(NativeControlTest::Constant(value));
    }
    prepare_native_expression_program(words, &word.operand, dialect)
        .map(NativeControlTest::Expression)
        .map_err(|_| NativeControlInstructionUnavailable::Expression)
}

fn generic(preparations: Vec<Visit>) -> NativeControlCompilation<NativeControlInstruction> {
    NativeControlCompilation {
        outcome: NativeControlOutcome::Generic,
        preparations,
    }
}

fn malformed(
    version: tcl_dialect::TclVersion,
    preparations: Vec<Visit>,
    message: String,
) -> NativeControlCompilation<NativeControlInstruction> {
    NativeControlCompilation {
        outcome: if version == tcl_dialect::TclVersion::V8_4 {
            NativeControlOutcome::Rejected(crate::native_compilation::NativeCompilationFailure {
                message: Some(message),
                error_code: None,
                error_info: None,
            })
        } else {
            NativeControlOutcome::Generic
        },
        preparations,
    }
}

fn inline(
    instruction: NativeControlInstruction,
    preparations: Vec<Visit>,
) -> NativeControlCompilation<NativeControlInstruction> {
    NativeControlCompilation {
        outcome: NativeControlOutcome::Inline(instruction),
        preparations,
    }
}

fn compile_conditional(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    version: tcl_dialect::TclVersion,
    dialect: InvocationDialect,
) -> Result<NativeControlCompilation<NativeControlInstruction>, NativeControlInstructionUnavailable>
{
    use NativeControlInstructionUnavailable as Unavailable;
    let mut preparations = Vec::new();

    // TclCompileIfCmd checks every original SIMPLE_WORD before pruning.
    if args.iter().any(|word| !simple(word)) {
        return Ok(generic(preparations));
    }
    if args.is_empty() {
        return Ok(malformed(
            version,
            preparations,
            "wrong # args: no expression after \"if\" argument".to_owned(),
        ));
    }
    let mut clauses = Vec::new();
    let mut at = 0;
    let mut masked = false;
    let mut fallback_allowed = false;
    while at < args.len() {
        let fallback = fallback_allowed && args[at].literal.as_deref() == Some(b"else");
        if fallback {
            at += 1;
        }
        let condition = if fallback {
            None
        } else {
            let word = args.get(at).ok_or(Unavailable::Geometry)?;
            at += 1;
            Some(if masked {
                NativeControlTest::Constant(false)
            } else {
                test(words, word, dialect)?
            })
        };
        if !fallback && args.get(at).and_then(|word| word.literal.as_deref()) == Some(b"then") {
            at += 1;
        }
        let Some(script_word) = args.get(at) else {
            return Ok(malformed(
                version,
                preparations,
                missing_conditional_script(words, args, at, fallback)?,
            ));
        };
        let script = native_control_body(words, script_word)?;
        at += 1;
        if !masked {
            let false_test = condition == Some(NativeControlTest::Constant(false));
            if let Some(NativeControlTest::Expression(program)) = &condition {
                preparations.push(Visit::Expression(program.operand.clone()));
            }
            if !false_test {
                visit_body(
                    &mut preparations,
                    &script,
                    NativeCompiledBodyContext::Inherit,
                );
                masked = fallback || condition == Some(NativeControlTest::Constant(true));
                clauses.push(NativeConditionalClause {
                    test: condition,
                    body: script,
                });
            }
        }
        if fallback && at < args.len() {
            return Ok(malformed(
                version,
                preparations,
                "wrong # args: extra words after \"else\" clause in \"if\" command".to_owned(),
            ));
        }
        fallback_allowed = match conditional_tail(
            words,
            args,
            &mut at,
            masked,
            &mut preparations,
            &mut clauses,
        )? {
            ConditionalTail::NextTest => false,
            ConditionalTail::DefaultAllowed => true,
            ConditionalTail::Malformed(message) => {
                return Ok(malformed(version, preparations, message.to_owned()));
            }
        };
    }
    let instruction = NativeControlInstruction::Conditional(clauses);
    Ok(inline(instruction, preparations))
}

fn compile_while(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    version: tcl_dialect::TclVersion,
    dialect: InvocationDialect,
) -> Result<NativeControlCompilation<NativeControlInstruction>, NativeControlInstructionUnavailable>
{
    let mut preparations = Vec::new();

    if args.len() != 2 {
        return Ok(malformed(
            version,
            preparations,
            "wrong # args: should be \"while test command\"".to_owned(),
        ));
    }
    if args.iter().any(|word| !simple(word)) {
        return Ok(generic(preparations));
    }
    let test = test(words, &args[0], dialect)?;
    let body = native_control_body(words, &args[1])?;
    if test != NativeControlTest::Constant(false) {
        visit_body(&mut preparations, &body, NativeCompiledBodyContext::Loop);
        if let NativeControlTest::Expression(program) = &test {
            preparations.push(Visit::Expression(program.operand.clone()));
        }
    }
    let instruction = NativeControlInstruction::While { test, body };
    Ok(inline(instruction, preparations))
}

fn compile_for(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    version: tcl_dialect::TclVersion,
    dialect: InvocationDialect,
) -> Result<NativeControlCompilation<NativeControlInstruction>, NativeControlInstructionUnavailable>
{
    use NativeControlInstructionUnavailable as Unavailable;
    let mut preparations = Vec::new();

    if args.len() != 4 {
        return Ok(malformed(
            version,
            preparations,
            "wrong # args: should be \"for start test next command\"".to_owned(),
        ));
    }
    if args[1..].iter().any(|word| !simple(word)) {
        return Ok(generic(preparations));
    }
    let start = native_control_body(words, &args[0])?;
    let body = native_control_body(words, &args[3])?;
    let next = native_control_body(words, &args[2])?;
    visit_body(
        &mut preparations,
        &start,
        NativeCompiledBodyContext::Inherit,
    );
    visit_body(&mut preparations, &body, NativeCompiledBodyContext::Loop);
    visit_body(&mut preparations, &next, NativeCompiledBodyContext::Loop);
    let test = prepare_native_expression_program(words, &args[1].operand, dialect)
        .map_err(|_| Unavailable::Expression)?;
    preparations.push(Visit::Expression(test.operand.clone()));
    let instruction = NativeControlInstruction::For {
        start,
        test,
        next,
        body,
    };
    Ok(inline(instruction, preparations))
}

fn compile_catch(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeControlCompilation<NativeControlInstruction>, NativeControlInstructionUnavailable>
{
    let mut preparations = Vec::new();

    if !(1..=if version == tcl_dialect::TclVersion::V8_4 {
        2
    } else {
        3
    })
        .contains(&args.len())
    {
        return Ok(malformed(
            version,
            preparations,
            "wrong # args: should be \"catch command ?varName?\"".to_owned(),
        ));
    }
    if args.len() > 1
        && context.frame != crate::native_compilation::NativeCompilationFrame::ProcedureCode
    {
        return Ok(generic(preparations));
    }
    let mut names = Vec::new();
    for word in &args[1..] {
        let Some(name) = word.literal.as_deref() else {
            return Ok(generic(preparations));
        };
        let projected = project_native_local_scalar(name, version);
        if let Some(name) = projected.declaration {
            preparations.push(Visit::DeclareLocal(name.to_vec()));
        }
        if !projected.scalar {
            return Ok(generic(preparations));
        }
        names.push(name.to_vec());
    }
    let body = native_control_body(words, &args[0])?;
    visit_body(
        &mut preparations,
        &body,
        NativeCompiledBodyContext::ExceptionRange,
    );
    if version == tcl_dialect::TclVersion::V8_4 {
        if let Some(Visit::Script {
            operand,
            span,
            context,
        }) = preparations.pop()
        {
            preparations.push(Visit::SpeculativeScript {
                operand,
                span,
                context,
            });
        } else {
            visit_body(
                &mut preparations,
                &body,
                NativeCompiledBodyContext::ExceptionRange,
            );
        }
    }
    let instruction = NativeControlInstruction::Catch {
        protocol: match version {
            tcl_dialect::TclVersion::V8_4 => NativeCatchProtocol::Tcl84,
            tcl_dialect::TclVersion::V8_5 => NativeCatchProtocol::Tcl85,
            tcl_dialect::TclVersion::V8_6 | tcl_dialect::TclVersion::V9_0 => {
                NativeCatchProtocol::Tcl86
            }
            tcl_dialect::TclVersion::V9_1 => NativeCatchProtocol::Tcl91,
        },
        body,
        result: names.first().cloned(),
        options: names.get(1).cloned(),
    };
    Ok(inline(instruction, preparations))
}

fn missing_conditional_script(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    at: usize,
    fallback: bool,
) -> Result<String, NativeControlInstructionUnavailable> {
    let argument = if fallback {
        b"else".as_slice()
    } else if at > 0 && args[at - 1].literal.as_deref() == Some(b"then") {
        b"then".as_slice()
    } else {
        match &args[at - 1].operand {
            NativeCompilerWordOperand::Original(index) => {
                words.original_words()[*index].written_bytes()
            }
            NativeCompilerWordOperand::LiteralExpansion { value, .. } => value,
        }
    };
    Ok(format!(
        "wrong # args: no script following \"{}\" argument",
        std::str::from_utf8(&argument[..argument.len().min(50)])
            .map_err(|_| NativeControlInstructionUnavailable::Geometry)?
    ))
}

enum ConditionalTail {
    NextTest,
    DefaultAllowed,
    Malformed(&'static str),
}

fn conditional_tail(
    words: &NativeCompilerWords<'_>,
    args: &[NativeProjectedCompilerWord],
    at: &mut usize,
    masked: bool,
    preparations: &mut Vec<Visit>,
    clauses: &mut Vec<NativeConditionalClause>,
) -> Result<ConditionalTail, NativeControlInstructionUnavailable> {
    if *at < args.len() && args[*at].literal.as_deref() == Some(b"elseif") {
        *at += 1;
        if *at == args.len() {
            return Ok(ConditionalTail::Malformed(
                "wrong # args: no expression after \"elseif\" argument",
            ));
        }
        Ok(ConditionalTail::NextTest)
    } else if *at < args.len() && args[*at].literal.as_deref() != Some(b"else") {
        let script = native_control_body(words, &args[*at])?;
        if !masked {
            visit_body(preparations, &script, NativeCompiledBodyContext::Inherit);
            clauses.push(NativeConditionalClause {
                test: None,
                body: script,
            });
        }
        *at += 1;
        if *at < args.len() {
            return Ok(ConditionalTail::Malformed(
                "wrong # args: extra words after \"else\" clause in \"if\" command",
            ));
        }
        Ok(ConditionalTail::NextTest)
    } else {
        Ok(ConditionalTail::DefaultAllowed)
    }
}

/// Project the original compiler's ordered preparation and control geometry.
///
/// # Errors
/// Missing source or expression evidence does not grant generic-handler
/// permission after an independently admitted native inline selection.
pub fn native_control_instruction(
    grammar: NativeCompilationGrammar,
    words: &NativeCompilerWords<'_>,
    from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeControlCompilation<NativeControlInstruction>, NativeControlInstructionUnavailable>
{
    use NativeControlInstructionUnavailable as Unavailable;
    let version = dialect.tcl_version.ok_or(Unavailable::Geometry)?;
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Geometry)?;
    let args = projected.get(from..).ok_or(Unavailable::Geometry)?;
    match grammar {
        NativeCompilationGrammar::Conditional => compile_conditional(words, args, version, dialect),
        NativeCompilationGrammar::WhileLoop => compile_while(words, args, version, dialect),
        NativeCompilationGrammar::ForLoop => compile_for(words, args, version, dialect),
        NativeCompilationGrammar::Catch => compile_catch(words, args, version, context),
        _ => Err(Unavailable::Operation),
    }
}
