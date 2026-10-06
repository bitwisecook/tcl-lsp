// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original try compiler handler parsing, local reservations and protected visits.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationWordShape, NativeCompiledBodyContext,
};
use crate::native_compiler_word_projection::{
    NativeCompilerProjectionUnavailable, NativeCompilerWordOperand, NativeProjectedCompilerWord,
    project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep as Visit,
    project_native_local_scalar,
};
use crate::native_control_instructions::{NativeControlBody, native_control_body, visit_body};
use tcl_dialect::TclVersion;

/// Native completion matcher, selected before any protected script runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeTryCondition {
    /// Compare the actual completion code.
    On(i32),
    /// Compare an error-code prefix as original counted List members.
    Trap(Vec<Vec<u8>>),
}
impl NativeTryCondition {
    /// Completion code tested before any error-prefix comparison.
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::On(code) => *code,
            Self::Trap(_) => 1,
        }
    }
}

/// One original compiled handler and its independent binding/fallthrough roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTryHandler {
    /// Matcher evaluated in original clause order.
    pub condition: NativeTryCondition,
    /// Result local reserved while parsing this handler.
    pub result: Option<Vec<u8>>,
    /// Options local reserved after this handler's result.
    pub options: Option<Vec<u8>>,
    /// A dash clause has no script; bindings still belong to this clause.
    pub body: Option<NativeControlBody>,
    /// Original non-dash clause executed after a match or fallthrough.
    pub target: usize,
    /// C9.1 can emit an empty handler without a catch range.
    pub empty_body: bool,
}

/// Selected protected scripts and original cleanup geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTryInstruction {
    /// Native compiler release selecting matcher and cleanup instructions.
    pub version: TclVersion,
    /// Original protected body; dynamic substitution occurs inside its range.
    pub body: NativeControlBody,
    /// Original clauses, including all fallthrough binding owners.
    pub handlers: Vec<NativeTryHandler>,
    /// Actual finally script, protected independently from the original body.
    pub finally: Option<NativeControlBody>,
    /// Trapless C9.1 uses a numeric table rather than comparisons.
    pub numeric_handler_table: bool,
    /// A handler can select a normally completed body.
    pub catches_success: bool,
}

/// Missing compiler evidence, distinct from native generic selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTryUnavailable {
    /// An original operand index is out of range.
    Geometry,
    /// The native compilation environment is unknown.
    Context,
    /// Original parser expansion or body spans are unavailable.
    Projection(NativeCompilerProjectionUnavailable),
}

fn simple(word: &NativeProjectedCompilerWord) -> bool {
    matches!(
        word.shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}
fn code_literal(visits: &mut Vec<Visit>, version: TclVersion, code: i32) {
    visits.push(if version >= TclVersion::V9_1 {
        Visit::Integer(i64::from(code))
    } else {
        Visit::Literal(code.to_string().into_bytes())
    });
}
fn error_cleanup(visits: &mut Vec<Visit>) {
    visits.push(Visit::Literal(b"1".to_vec()));
    visits.push(Visit::Literal(b"-during".to_vec()));
}

fn try_handler_condition(
    kind: &NativeProjectedCompilerWord,
    matcher: &[u8],
    version: TclVersion,
) -> Option<NativeTryCondition> {
    let strings = tcl_syntax::native_string::NativeStringProtocol::C(version);
    match kind.literal.as_deref() {
        Some(b"on") => {
            let Ok(code) =
                crate::native_return_compilation::static_completion_code(matcher, version)
            else {
                return None;
            };
            Some(NativeTryCondition::On(code))
        }
        Some(b"trap") => {
            let Ok(pattern) = tcl_syntax::list::split_native_list_bytes(matcher, strings) else {
                return None;
            };
            if pattern.is_empty() {
                return None;
            }
            Some(NativeTryCondition::Trap(
                pattern
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect(),
            ))
        }
        _ => None,
    }
}

fn prepare_try_handlers(
    words: &NativeCompilerWords<'_>,
    args: &[&NativeProjectedCompilerWord],
    clause_count: usize,
    version: TclVersion,
    context: NativeCompilationContext,
    preparations: &mut Vec<Visit>,
) -> Result<Option<Vec<NativeTryHandler>>, NativeTryUnavailable> {
    let strings = tcl_syntax::native_string::NativeStringProtocol::C(version);
    let mut handlers = Vec::new();
    for clause in args[1..=(clause_count * 4)].as_chunks::<4>().0 {
        if !simple(clause[0]) {
            return Ok(None);
        }
        let Some(matcher) = &clause[1].literal else {
            return Ok(None);
        };
        let Some(condition) = try_handler_condition(clause[0], matcher, version) else {
            return Ok(None);
        };
        let Some(variables) = &clause[2].literal else {
            return Ok(None);
        };
        let Ok(variables) = tcl_syntax::list::split_native_list_bytes(variables, strings) else {
            return Ok(None);
        };
        if variables.len() > 2 {
            return Ok(None);
        }
        let mut bindings = Vec::new();
        for variable in variables {
            if context.frame != NativeCompilationFrame::ProcedureCode {
                return Ok(None);
            }
            let projected = project_native_local_scalar(&variable, version);
            if let Some(name) = projected.declaration {
                preparations.push(Visit::DeclareLocal(name.to_vec()));
            }
            if !projected.scalar {
                return Ok(None);
            }
            bindings.push(
                projected
                    .declaration
                    .expect("native scalar binding")
                    .to_vec(),
            );
        }
        if !simple(clause[3]) {
            return Ok(None);
        }
        let is_dash = clause[3].literal.as_deref() == Some(b"-");
        let empty_body = version >= TclVersion::V9_1
            && clause[3]
                .literal
                .as_ref()
                .is_some_and(|bytes| bytes.iter().copied().all(tcl_syntax::list::is_list_space));
        handlers.push(NativeTryHandler {
            condition,
            result: bindings.first().cloned(),
            options: bindings.get(1).cloned(),
            body: (!is_dash)
                .then(|| {
                    native_control_body(words, clause[3])
                        .map_err(|_| NativeTryUnavailable::Geometry)
                })
                .transpose()?,
            target: handlers.len(),
            empty_body,
        });
    }
    Ok(Some(handlers))
}

fn prepare_try_visits(
    version: TclVersion,
    body: &NativeControlBody,
    handlers: &[NativeTryHandler],
    finally: Option<&NativeControlBody>,
    preparations: &mut Vec<Visit>,
) -> (bool, bool) {
    let strings = tcl_syntax::native_string::NativeStringProtocol::C(version);
    let has_protection = !handlers.is_empty() || finally.is_some();
    visit_body(
        preparations,
        body,
        if has_protection {
            NativeCompiledBodyContext::ExceptionRange
        } else {
            NativeCompiledBodyContext::Inherit
        },
    );
    let catches_success = handlers.iter().any(|handler| handler.condition.code() == 0);
    let numeric_handler_table = version >= TclVersion::V9_1
        && !handlers.is_empty()
        && handlers
            .iter()
            .all(|handler| matches!(handler.condition, NativeTryCondition::On(_)));
    if catches_success {
        preparations.push(Visit::Literal(b"0".to_vec()));
    } else if finally.is_some() && !handlers.is_empty() {
        preparations.push(Visit::Literal(b"-level 0 -code 0".to_vec()));
    }
    for handler in handlers {
        if !numeric_handler_table {
            code_literal(preparations, version, handler.condition.code());
        }
        if let NativeTryCondition::Trap(pattern) = &handler.condition {
            preparations.push(Visit::Literal(b"-errorcode".to_vec()));
            preparations.push(if version >= TclVersion::V9_1 {
                Visit::List(pattern.clone())
            } else {
                Visit::Literal(
                    tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                        strings,
                    )
                    .render(pattern),
                )
            });
        }
        if let Some(body) = &handler.body {
            if handler.empty_body && finally.is_none() {
                preparations.push(Visit::Literal(Vec::new()));
            } else {
                visit_body(
                    preparations,
                    body,
                    NativeCompiledBodyContext::ExceptionRange,
                );
                if finally.is_some() && !numeric_handler_table {
                    preparations.push(Visit::Literal(b"0".to_vec()));
                }
                error_cleanup(preparations);
            }
        } else if finally.is_some() && (handler.result.is_some() || handler.options.is_some()) {
            error_cleanup(preparations);
        }
    }
    if let Some(finally) = finally {
        visit_body(
            preparations,
            finally,
            NativeCompiledBodyContext::ExceptionRange,
        );
        error_cleanup(preparations);
    }
    (numeric_handler_table, catches_success)
}

enum TryFinally {
    Absent,
    Body(NativeControlBody),
    Generic,
}

fn prepare_try_finally(
    words: &NativeCompilerWords<'_>,
    remainder: &[&NativeProjectedCompilerWord],
    version: TclVersion,
) -> Result<TryFinally, NativeTryUnavailable> {
    let finally = match remainder {
        [] => TryFinally::Absent,
        [kind, final_body]
            if simple(kind)
                && kind.literal.as_deref() == Some(b"finally")
                && simple(final_body) =>
        {
            let empty = version >= TclVersion::V9_1
                && final_body.literal.as_ref().is_some_and(|bytes| {
                    bytes.iter().copied().all(tcl_syntax::list::is_list_space)
                });
            if empty {
                TryFinally::Absent
            } else {
                TryFinally::Body(
                    native_control_body(words, final_body)
                        .map_err(|_| NativeTryUnavailable::Geometry)?,
                )
            }
        }
        _ => TryFinally::Generic,
    };
    Ok(finally)
}

/// Parse original try clauses and retain `LocalScalar` side effects before a
/// later clause declines. Command registration is a separate caller obligation.
///
/// # Errors
/// Missing original source or physical compiler context remains explicit.
pub fn compile_native_try(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeControlCompilation<NativeTryInstruction>, NativeTryUnavailable> {
    use NativeControlOutcome::{Generic, Inline};
    let mut selected = NativeControlCompilation {
        outcome: Generic,
        preparations: Vec::new(),
    };
    if operand_from == 0 || operand_from > words.original_words().len() {
        return Err(NativeTryUnavailable::Geometry);
    }
    if version < TclVersion::V8_6 || context.mode == NativeCompilationMode::Direct {
        return Ok(selected);
    }
    if context.mode != NativeCompilationMode::BytecodeObject
        || context.frame == NativeCompilationFrame::Unknown
    {
        return Err(NativeTryUnavailable::Context);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(NativeTryUnavailable::Projection)?;
    if projected
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Ok(selected);
    }
    let args = projected
        .iter()
        .filter(|word| match word.operand {
            NativeCompilerWordOperand::Original(index) => index >= operand_from,
            NativeCompilerWordOperand::LiteralExpansion { original_word, .. } => {
                original_word >= operand_from
            }
        })
        .collect::<Vec<_>>();
    let Some(original_body) = args.first() else {
        return Ok(selected);
    };
    let clause_count = (args.len() - 1) / 4;
    let Some(mut handlers) = prepare_try_handlers(
        words,
        &args,
        clause_count,
        version,
        context,
        &mut selected.preparations,
    )?
    else {
        return Ok(selected);
    };
    if handlers
        .last()
        .is_some_and(|handler| handler.body.is_none())
    {
        return Ok(selected);
    }
    let remainder = &args[1 + clause_count * 4..];
    let finally = match prepare_try_finally(words, remainder, version)? {
        TryFinally::Absent => None,
        TryFinally::Body(body) => Some(body),
        TryFinally::Generic => return Ok(selected),
    };
    for index in (0..handlers.len()).rev() {
        if handlers[index].body.is_none() {
            handlers[index].target = handlers[index + 1].target;
        }
    }
    if !handlers.is_empty() {
        if context.frame != NativeCompilationFrame::ProcedureCode {
            return Ok(selected);
        }
        selected
            .preparations
            .extend([Visit::DeclareAnonymousLocal, Visit::DeclareAnonymousLocal]);
    }
    let body =
        native_control_body(words, original_body).map_err(|_| NativeTryUnavailable::Geometry)?;
    let (numeric_handler_table, catches_success) = prepare_try_visits(
        version,
        &body,
        &handlers,
        finally.as_ref(),
        &mut selected.preparations,
    );
    selected.outcome = Inline(NativeTryInstruction {
        version,
        body,
        handlers,
        finally,
        numeric_handler_table,
        catches_success,
    });
    Ok(selected)
}
