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

//! Original byte word and template substitution through the shared lexical owner.
//!
//! Compiled words retain bare dollar bytes as data and substitute `${…}`,
//! command bodies and selected Jim expression components. Written expression
//! references use their independent variable grammar. Executable array indices
//! are flat arena lists and evaluate with heap frames, without a recursive tree.

use tcl_runtime_api::Code;
#[cfg(test)]
use tcl_syntax::word_rules::whole_braced_word as whole_braced;

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// Index of the `]` closing the command substitution opening at `b[start]`,
/// or `None`.
///
/// The search is the shared owner's
/// ([`word_parts::command_subst_close`](tcl_lexer::word_parts::command_subst_close),
/// over `tcl_lexer::command_substitution_end`), which is brace-, quote- **and**
/// comment-aware because the substituted text is a script: a `]` written
/// inside `{…}`, inside a `"…"` word, or after a `#` at command position does
/// not close the substitution. This VM's private copy handled only braces, so
/// `subst {[list "a]b"]}` stopped at the quoted `]` — `tclsh` 8.6.16 and 9.0.4
/// both give `a\]b`.
///
/// The owner reports one byte *past* the `]`; the callers here index the `]`
/// itself, so this adapter steps back.
///
/// `config` is the VM's own grammar ([`Vm::lexer_config`]): the substituted
/// text is a script, so where a `]` stops is a release/dialect question
/// (`docs/design/registry/dialect-profile-model.md` §2.5), not the default grammar's.
///
/// The error is the owner's, not a flat `missing close-bracket`. A substituted
/// `[…]` is a *script*, so C recurses into it at the bracket and reports what
/// it meets inside: `subst {[set y ${a{b]}` is `missing close-brace for
/// variable name` on tclsh 8.6.16 and 9.0.4 alike. Discarding the owner's
/// message with `.ok()` and substituting the outer one at every call site is
/// exactly the error the owner's own contract warns against.
#[cfg(test)]
fn command_end(
    b: &[u8],
    start: usize,
    config: tcl_lexer::LexerConfig,
) -> Result<usize, &'static str> {
    tcl_lexer::word_parts::command_subst_close(
        b,
        start,
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    )
    .map(|end| end - 1)
}

/// Apply the selected original expression quote protocol to a bracket's
/// complete guest completion. Host refusals have already left this channel.
pub(crate) fn settle_expression_quote(
    mut completion: tcl_runtime_api::Completion<Value>,
    policy: tcl_registry::invocation_words::ExpressionQuoteControl,
) -> tcl_runtime_api::Completion<Value> {
    use tcl_registry::invocation_words::ExpressionQuoteControl;
    if policy == ExpressionQuoteControl::Propagate {
        return completion;
    }
    match completion.code {
        Code::Return => {
            completion.code = Code::Ok;
            completion.options =
                crate::command::with_return_option(&completion.options, "-code", Value::int(0));
            completion
        }
        Code::Break => crate::command::err_with_code("invoked \"break\" outside of a loop", "NONE"),
        Code::Continue => {
            crate::command::err_with_code("invoked \"continue\" outside of a loop", "NONE")
        }
        Code::Other(_) => tcl_runtime_api::Completion::new_error_metadata(
            Code::Error,
            completion.result,
            crate::command::options_dict(Code::Error, 0, &[("-errorcode", Value::string("NONE"))]),
        ),
        Code::Ok | Code::Error => completion,
    }
}

/// Blocking counterpart to the resumable original-word scanner. It uses the
/// same source/index geometry and retains a complete bracket completion until
/// the selected purpose settles it; it never uses `subst` command absorption.
pub(crate) fn subst_expression_string(
    vm: &mut Vm,
    source: impl Into<SubstSource>,
    control: SubstitutionControl,
) -> Result<tcl_runtime_api::Completion<Value>, TclError> {
    let mut state = SubstState::new(source, true, true, true, control);
    let mut options = Value::empty();
    loop {
        let mut completion = match subst_scan_step(vm, &mut state) {
            SubstStep::Done(bytes) => {
                return Ok(tcl_runtime_api::Completion::new(
                    Code::Ok,
                    Value::from_string_bytes(bytes),
                    options,
                ));
            }
            SubstStep::Error(error) => return Err(error),
            SubstStep::Bracket(script) => {
                vm.eval_source_image_at_internal(&tcl_lexer::SourceImage::native(script), None)?
            }
            SubstStep::ArrayIndex(index) => {
                subst_expression_string(vm, index, SubstitutionControl::Word)?
            }
        };
        if let SubstitutionControl::Expression(policy) = control {
            completion = settle_expression_quote(completion, policy);
        }
        if completion.code != Code::Ok {
            return Err(TclError::from_completion(completion));
        }
        options = completion.options;
        let value = if let Some(base) = state.pending_array.take() {
            let key = materialise_word_component(vm, &completion.result)?;
            vm.read_variable_result_bytes(&base, Some(&key))
                .map_err(TclError::from_completion)?
        } else {
            completion.result
        };
        state
            .out
            .extend_from_slice(&materialise_word_component(vm, &value)?);
    }
}

/// Resumable state for a **yieldable** `subst` command activation:
/// the template, the scan cursor, the output accumulated so
/// far, and the three substitution switches. It lives on the `subst` frame (see
/// `crate::exec`), so it freezes with a suspended coroutine and resumes after
/// each `[…]` completes. Backslash / `$…` runs never yield, so they are scanned
/// natively; only a top-level `[…]` pauses the scan.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubstitutionControl {
    Command,
    Word,
    Expression(tcl_registry::invocation_words::ExpressionQuoteControl),
}

/// Immutable original template or an index list in the same source arena.
#[derive(Clone)]
pub(crate) enum SubstSource {
    Bytes(std::rc::Rc<[u8]>),
    Parts {
        arena: std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        list: tcl_lexer::PartListId,
    },
}
impl From<Vec<u8>> for SubstSource {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes.into())
    }
}
impl From<String> for SubstSource {
    fn from(text: String) -> Self {
        text.into_bytes().into()
    }
}
impl From<&str> for SubstSource {
    fn from(text: &str) -> Self {
        text.as_bytes().to_vec().into()
    }
}
impl From<&[u8]> for SubstSource {
    fn from(bytes: &[u8]) -> Self {
        bytes.to_vec().into()
    }
}

pub(crate) struct SubstState {
    pub(crate) control: SubstitutionControl,
    pub(crate) pending_array: Option<Vec<u8>>,
    source: SubstSource,
    cursor: usize,
    pub(crate) out: Vec<u8>,
    flags: tcl_lexer::word_parts::SubstFlags,
}

impl SubstState {
    pub(crate) fn new(
        template: impl Into<SubstSource>,
        backslashes: bool,
        commands: bool,
        variables: bool,
        control: SubstitutionControl,
    ) -> Self {
        Self {
            control,
            pending_array: None,
            source: template.into(),
            cursor: 0,
            out: Vec::new(),
            flags: tcl_lexer::word_parts::SubstFlags {
                backslashes,
                cmds: commands,
                vars: variables,
                ..Default::default()
            },
        }
    }

    fn prepare(&mut self, vm: &Vm) -> Result<(), TclError> {
        let SubstSource::Bytes(bytes) = &self.source else {
            return Ok(());
        };
        let end = u32::try_from(bytes.len()).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native substitution source extent",
            )
        })?;
        let image = tcl_lexer::SourceImage::native(bytes.as_ref());
        let arena = if self.control == SubstitutionControl::Command {
            let policy = vm.expression_template_policy().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native substitution template grammar",
                ),
            )?;
            tcl_lexer::ExecutablePartArena::decompose_template(
                image,
                tcl_lexer::Span::new(0, end),
                self.flags,
                vm.lexer_config(),
                policy.variable_syntax(),
            )
        } else {
            tcl_lexer::ExecutablePartArena::decompose(
                image,
                tcl_lexer::Span::new(0, end),
                self.flags,
                vm.lexer_config(),
            )
        }
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native substitution source geometry",
            )
        })?;
        let list = arena.root();
        self.source = SubstSource::Parts {
            arena: std::rc::Rc::new(arena),
            list,
        };
        Ok(())
    }
}

pub(crate) enum SubstStep {
    Done(Vec<u8>),
    Bracket(Vec<u8>),
    ArrayIndex(SubstSource),
    Error(TclError),
}

/// Execute the shared flat component list in order. Index frames retain their
/// source arena; parse errors are observed only after preceding components.
pub(crate) fn subst_scan_step(vm: &mut Vm, st: &mut SubstState) -> SubstStep {
    use tcl_lexer::ExecutablePart;
    if let Err(error) = st.prepare(vm) {
        return SubstStep::Error(error);
    }
    let SubstSource::Parts { arena, list } = &st.source else {
        unreachable!("prepared arena");
    };
    let arena = std::rc::Rc::clone(arena);
    let list = *list;
    while let Some(component) = arena.list(list).get(st.cursor) {
        st.cursor += 1;
        let result = match &component.part {
            ExecutablePart::Text(_) => {
                let text = vm
                    .source_string_protocol()
                    .ok_or_else(|| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native executable text recipe",
                        )
                    })
                    .and_then(|protocol| {
                        tcl_syntax::backslash::native_arena_text(
                            &arena,
                            component,
                            vm.lexer_config().escapes,
                            protocol,
                        )
                        .map_err(|_| {
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native executable text decoding",
                            )
                        })
                    });
                match text {
                    Ok(text) => st.out.extend_from_slice(&text),
                    Err(error) => return SubstStep::Error(error.into()),
                }
                continue;
            }
            ExecutablePart::Variable {
                name,
                index: Some(index),
            } => {
                st.pending_array = Some(arena.bytes(*name).expect("arena name geometry").to_vec());
                return SubstStep::ArrayIndex(SubstSource::Parts {
                    arena: std::rc::Rc::clone(&arena),
                    list: *index,
                });
            }
            ExecutablePart::Variable { name, index: None } => vm
                .read_variable_result_bytes(arena.bytes(*name).expect("arena name geometry"), None)
                .map_err(TclError::from_completion),
            ExecutablePart::Command { body } => {
                return SubstStep::Bracket(
                    arena.bytes(*body).expect("arena command geometry").to_vec(),
                );
            }
            ExecutablePart::Expression { expression } => {
                vm.eval_expr_bytes(arena.bytes(*expression).expect("arena expression geometry"))
            }
            ExecutablePart::ParseError(message) => return SubstStep::Error(TclError::new(message)),
        };
        match result {
            Ok(value) => match materialise_word_component(vm, &value) {
                Ok(bytes) => st.out.extend_from_slice(&bytes),
                Err(error) => return SubstStep::Error(error),
            },
            Err(error) => return SubstStep::Error(error),
        }
    }
    SubstStep::Done(std::mem::take(&mut st.out))
}

/// A parsed `$`-reference split into its base name and optional raw array index.
#[cfg(test)]
pub(crate) struct VarRef<'a> {
    /// The `$name` / `${name}` base variable (or array) name.
    pub(crate) base: &'a str,
    /// The raw (unsubstituted) array index span, for `$name(index)`.
    pub(crate) index: Option<&'a str>,
    /// Byte offset just past the whole reference.
    pub(crate) next: usize,
}

/// Parse a `$`-variable reference starting at `s[at]` (`$name`, `${name}`,
/// `$name(idx)`) — a `&str` adapter over the shared owner
/// [`word_parts::scan_var_ref`](tcl_lexer::word_parts::scan_var_ref).
///
/// Both release axes ride on the owner: the `${…}` close rule (8.x ends the
/// name at the first literal `}`, 9.x counts nesting and skips `\X`, so
/// `subst {${a{b}c}}` reads `a{b` on 8.6 and `a{b}c` on 9.0)
/// and the array-index source mask.
///
/// `Ok(None)` means "not a variable reference" — the `$` is literal text. An
/// unterminated form is different: it is one of C's parse errors (`missing
/// close-brace for variable name`, `missing )`, `invalid character in array
/// index`), so it returns `Err`. Raising it here rather than at the top of
/// `subst` reproduces C's left-to-right substitution, where earlier command
/// substitutions in the same template have already run and kept their side
/// effects — verified against both oracles.
#[cfg(test)]
pub(crate) fn parse_var_ref_parts(
    s: &str,
    at: usize,
    config: tcl_lexer::LexerConfig,
) -> Result<Option<VarRef<'_>>, TclError> {
    match tcl_lexer::word_parts::scan_var_ref(s.as_bytes(), at, config) {
        Ok(None) => Ok(None),
        // The owner returns byte sub-slices of the same buffer; `s` is a
        // `&str` and every boundary the scan reports is a delimiter or an
        // ASCII name byte, so the ranges are always char boundaries.
        Ok(Some(raw)) => {
            let span = |sub: &[u8]| -> &str {
                let start = sub.as_ptr() as usize - s.as_bytes().as_ptr() as usize;
                &s[start..start + sub.len()]
            };
            Ok(Some(VarRef {
                base: span(raw.name),
                index: raw.index.map(span),
                next: raw.next,
            }))
        }
        Err(message) => Err(TclError::new(message)),
    }
}

/// Substitute the compiler's retained word convention. Bare `$` bytes are data;
/// `${…}`, complete `[…]` and selected Jim expression substitutions execute.
/// A single substitution retains its original result object.
pub fn subst_word(word: &str, vm: &mut Vm) -> Result<Value, TclError> {
    subst_word_bytes(word.as_bytes(), vm)
}

/// Substitute original compiled-word bytes without a Unicode name projection.
///
/// # Errors
/// Preserves guest completions and typed host source/materialisation refusals.
pub fn subst_word_bytes(word: &[u8], vm: &mut Vm) -> Result<Value, TclError> {
    if let Some(inner) = tcl_syntax::word_rules::whole_braced_word_bytes(word) {
        return Ok(Value::from_native_string_bytes(inner));
    }
    // Literal PUSH bytes already have their source escapes decoded. Only a
    // retained substitution causes the compiled-word scanner to run again.
    if !word.windows(2).any(|pair| pair == b"${")
        && !word.contains(&b'[')
        && !(vm.lexer_config().var_syntax.has_expr_sugar()
            && word.windows(2).any(|pair| pair == b"$("))
    {
        return Ok(Value::from_native_string_bytes(word));
    }
    let arena = substitution_arena(word, tcl_lexer::word_parts::SubstFlags::compiled_word(), vm)?;
    evaluate_arena(&arena, vm)
}

/// Evaluate one original expression variable reference, including its index.
/// The written-word grammar is independent of the compiled-word convention.
pub(crate) fn subst_variable_reference_bytes(
    reference: &[u8],
    vm: &mut Vm,
) -> Result<Value, TclError> {
    let arena = substitution_arena(reference, tcl_lexer::word_parts::SubstFlags::default(), vm)?;
    validate_arena(&arena)?;
    if !matches!(
        arena.list(arena.root()),
        [tcl_lexer::word_parts::SpannedExecutablePart {
            part: tcl_lexer::word_parts::ExecutablePart::Variable { .. },
            ..
        }]
    ) {
        return Err(substitution_host_refusal(
            vm,
            "expression variable operand lacks an original sole-reference extent".into(),
        ));
    }
    evaluate_arena(&arena, vm)
}

fn substitution_arena(
    source: &[u8],
    flags: tcl_lexer::word_parts::SubstFlags,
    vm: &mut Vm,
) -> Result<tcl_lexer::word_parts::ExecutablePartArena, TclError> {
    let end = u32::try_from(source.len()).map_err(|_| {
        substitution_host_refusal(vm, "substitution source extent unavailable".into())
    })?;
    tcl_lexer::word_parts::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, end),
        flags,
        vm.lexer_config(),
    )
    .map_err(|error| {
        substitution_host_refusal(
            vm,
            format!("substitution source geometry unavailable: {error:?}"),
        )
    })
}

fn substitution_host_refusal(vm: &mut Vm, reason: String) -> TclError {
    let _ = vm.refuse_host_command(reason);
    TclError::from_execution_failure(
        vm.execution_refusal
            .clone()
            .expect("recorded substitution host refusal"),
    )
}

fn validate_arena(arena: &tcl_lexer::word_parts::ExecutablePartArena) -> Result<(), TclError> {
    for component in arena.all_parts() {
        if let tcl_lexer::word_parts::ExecutablePart::ParseError(message) = component.part {
            return Err(TclError::new(message));
        }
    }
    Ok(())
}

struct WordEvaluationFrame {
    list: tcl_lexer::word_parts::PartListId,
    next: usize,
    result: Option<Value>,
    read_after: Option<tcl_lexer::Span>,
}

fn evaluate_arena(
    arena: &tcl_lexer::word_parts::ExecutablePartArena,
    vm: &mut Vm,
) -> Result<Value, TclError> {
    use tcl_lexer::word_parts::ExecutablePart;
    // Parse the complete word before executing any of its substitutions.
    validate_arena(arena)?;
    let mut frames = vec![WordEvaluationFrame {
        list: arena.root(),
        next: 0,
        result: None,
        read_after: None,
    }];
    loop {
        let frame = frames.last_mut().expect("root or index evaluation frame");
        let value = if let Some(component) = arena.list(frame.list).get(frame.next) {
            frame.next += 1;
            match &component.part {
                ExecutablePart::Text(_) => {
                    let protocol = vm.source_string_protocol().ok_or_else(|| {
                        substitution_host_refusal(
                            vm,
                            "native word source string protocol unavailable".into(),
                        )
                    })?;
                    let text = tcl_syntax::backslash::native_arena_text(
                        arena,
                        component,
                        vm.lexer_config().escapes,
                        protocol,
                    )
                    .map_err(|error| {
                        substitution_host_refusal(
                            vm,
                            format!("native word text unavailable: {error:?}"),
                        )
                    })?;
                    Value::from_native_string_bytes(text.as_ref())
                }
                ExecutablePart::Variable {
                    name,
                    index: Some(index),
                } => {
                    let child = WordEvaluationFrame {
                        list: *index,
                        next: 0,
                        result: None,
                        read_after: Some(*name),
                    };
                    frames.push(child);
                    continue;
                }
                ExecutablePart::Variable { name, index: None } => vm
                    .read_variable_result_bytes(
                        arena.bytes(*name).expect("validated original name extent"),
                        None,
                    )
                    .map_err(TclError::from_completion)?,
                ExecutablePart::Command { body } => {
                    let completion = vm.eval_source_image_at_internal(
                        &tcl_lexer::SourceImage::native(
                            arena.bytes(*body).expect("validated script extent"),
                        ),
                        None,
                    )?;
                    if completion.code != Code::Ok {
                        return Err(TclError::from_completion(completion));
                    }
                    completion.result
                }
                ExecutablePart::Expression { expression } => vm.eval_expr_bytes(
                    arena
                        .bytes(*expression)
                        .expect("validated expression extent"),
                )?,
                ExecutablePart::ParseError(_) => unreachable!("validated word syntax"),
            }
        } else {
            let completed = frames.pop().expect("completed evaluation frame");
            let value = completed
                .result
                .unwrap_or_else(|| Value::from_native_string_bytes(&b""[..]));
            let Some(name) = completed.read_after else {
                return Ok(value);
            };
            let index = materialise_word_component(vm, &value)?;
            vm.read_variable_result_bytes(
                arena.bytes(name).expect("validated original array name"),
                Some(&index),
            )
            .map_err(TclError::from_completion)?
        };
        let result = &mut frames.last_mut().expect("parent evaluation frame").result;
        *result = Some(match result.take() {
            None => value,
            Some(previous) => {
                // Materialisation follows completion of the next component,
                // matching the compiler's concatenation evaluation order.
                let mut bytes = materialise_word_component(vm, &previous)?.to_vec();
                bytes.extend_from_slice(&materialise_word_component(vm, &value)?);
                Value::from_native_string_bytes(bytes)
            }
        });
    }
}

fn materialise_word_component(vm: &mut Vm, value: &Value) -> Result<std::rc::Rc<[u8]>, TclError> {
    vm.native_name_operand_bytes(value).map_err(|error| {
        tcl_syntax::value::ValueError::NativeStringAccess(
            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
        )
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SubstState, SubstStep, SubstitutionControl, command_end, parse_var_ref_parts,
        subst_scan_step, whole_braced,
    };
    use crate::interp::Vm;
    use crate::value::Value;
    use tcl_dialect::{ArrayIndexSyntax, BracedVarStyle};

    #[test]
    fn compiled_byte_words_preserve_opaque_results_and_bare_dollar_data() {
        let mut vm = Vm::new();
        vm.set_var_bytes(b"\xff", Value::from_native_string_bytes(&b"R\xff\0"[..]))
            .unwrap();
        assert_eq!(
            super::subst_word_bytes(b"${\xff}", &mut vm)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"R\xff\0"
        );
        assert_eq!(
            super::subst_word_bytes(b"pre${\xff}tail", &mut vm)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"preR\xff\0tail"
        );
        assert_eq!(
            super::subst_word_bytes(b"$untouched\\n", &mut vm)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"$untouched\\n"
        );
        assert_eq!(
            super::subst_word_bytes(b"{${\xff}}", &mut vm)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"${\xff}"
        );
    }

    #[test]
    fn original_byte_variable_reference_evaluates_full_depth_indices() {
        let mut vm = Vm::new();
        vm.set_var_bytes(
            b"a(x\xff\0)",
            Value::from_native_string_bytes(&b"x\xff\0"[..]),
        )
        .unwrap();
        let mut reference = Vec::new();
        for _ in 0..2_000 {
            reference.extend_from_slice(b"$a(");
        }
        reference.extend_from_slice(b"x\xff\0");
        reference.extend(std::iter::repeat_n(b')', 2_000));
        assert_eq!(
            super::subst_variable_reference_bytes(&reference, &mut vm)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"x\xff\0"
        );
        assert!(
            super::subst_variable_reference_bytes(b"prefix${a}", &mut vm)
                .unwrap_err()
                .is_host()
        );
    }

    /// A config pinning just the two axes a var-reference scan turns on,
    /// over the default grammar — these tests exercise the release axes
    /// themselves, not a document's resolved grammar.
    /// [`command_end`] under the default grammar — these rows pin the
    /// bracket search itself (brace/quote/comment awareness), which no
    /// modelled grammar varies.
    fn command_end_default(b: &[u8], start: usize) -> Option<usize> {
        command_end(b, start, tcl_lexer::LexerConfig::default()).ok() // dialect-drift-ok: grammar-invariant rule under test
    }

    fn var_config(
        braced_var: BracedVarStyle,
        array_index: ArrayIndexSyntax,
    ) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig {
            braced_var,
            array_index,
            ..tcl_lexer::LexerConfig::default() // dialect-drift-ok: axis-under-test, not document text
        }
    }

    /// Exercise the actual command owner and trampoline, preserving any abrupt
    /// guest completion or operational host failure until this byte consumer.
    fn dispatch_subst(
        vm: &mut Vm,
        template: &str,
        backslashes: bool,
        commands: bool,
        variables: bool,
    ) -> Result<Vec<u8>, crate::error::TclError> {
        let mut args = Vec::new();
        for (enabled, option) in [
            (backslashes, "-nobackslashes"),
            (commands, "-nocommands"),
            (variables, "-novariables"),
        ] {
            if !enabled {
                args.push(Value::string(option));
            }
        }
        args.push(Value::string(template));
        let completion = vm.invoke_command("subst", &args);
        if let Some(refusal) = vm.execution_refusal.clone() {
            return Err(crate::error::TclError::from_execution_failure(refusal));
        }
        if completion.code != tcl_runtime_api::Code::Ok {
            return Err(crate::error::TclError::from_completion(completion));
        }
        Ok(completion.result.string_bytes().to_vec())
    }

    /// Actual `subst` dispatch with only backslash substitution enabled.
    fn subst_backslashes(template: &str) -> String {
        let mut vm = Vm::new();
        String::from_utf8(dispatch_subst(&mut vm, template, true, false, false).expect("subst"))
            .expect("Unicode fixture")
    }

    #[test]
    fn jim_subst_scanners_preserve_raw_results_and_expression_ingress() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        vm.set_var("raw", Value::from_string_bytes([0xff, 0].as_slice()))
            .expect("raw variable");
        let template = "pre$(1+2)$raw\\xff";
        let expected = b"pre3\xff\0\xff";
        assert_eq!(
            dispatch_subst(&mut vm, template, true, true, true).expect("subst"),
            expected
        );
        let mut state = SubstState::new(template, true, true, true, SubstitutionControl::Command);
        match subst_scan_step(&mut vm, &mut state) {
            SubstStep::Done(bytes) => assert_eq!(bytes, expected),
            _ => panic!("literal expression and variable should finish the scan"),
        }
        assert_eq!(
            dispatch_subst(&mut vm, "$(1+2)", true, true, false).expect("no variables"),
            b"$(1+2)"
        );
        assert!(dispatch_subst(&mut vm, "$(k)", true, true, true).is_err());
    }

    #[test]
    fn subst_hex_escape_consumes_exactly_two_digits() {
        // Tcl 9 caps `\x` at two hex digits (`TclParseBackslash`): `\x41BC`
        // is `A` + literal `BC`, never a wider code point or the 8.x
        // last-two-digits reading.
        assert_eq!(subst_backslashes(r"\x41BC"), "ABC");
        assert_eq!(subst_backslashes(r"\x4142"), "A42");
    }

    #[test]
    fn subst_backslash_continuation_accepts_only_raw_lf() {
        // TclParseBackslash recognises raw LF. A channel may translate CRLF
        // before this parser seam, but raw CR and CRLF remain data.
        assert_eq!(subst_backslashes("a\\\n   b"), "a b");
        assert_eq!(subst_backslashes("a\\\r\n\t b"), "a\r\n\t b");
        assert_eq!(subst_backslashes("a\\\rb"), "a\rb");
        // FP guard: `\\` is an escaped backslash, so the newline after it is
        // real content, not a continuation.
        assert_eq!(subst_backslashes("x\\\\\r\ny"), "x\\\r\ny");
    }

    #[test]
    fn variable_reference_scanner_consumes_colon_runs() {
        for (source, base, next) in [
            ("$a:::b", "a:::b", 6),
            ("$::a:::b", "::a:::b", 8),
            ("$foo:::", "foo:::", 7),
        ] {
            let parsed = parse_var_ref_parts(
                source,
                0,
                var_config(BracedVarStyle::Tcl9Nesting, ArrayIndexSyntax::Tcl9),
            )
            .expect("parses")
            .expect("variable reference");
            assert_eq!(parsed.base, base);
            assert_eq!(parsed.next, next);
        }
        let parsed = parse_var_ref_parts(
            "$a:::b(k)",
            0,
            var_config(BracedVarStyle::Tcl9Nesting, ArrayIndexSyntax::Tcl9),
        )
        .expect("parses")
        .expect("array reference");
        assert_eq!(parsed.base, "a:::b");
        assert_eq!(parsed.index, Some("k"));
    }

    #[test]
    fn array_index_source_mask_follows_the_release() {
        let legacy = parse_var_ref_parts(
            "$a({key})",
            0,
            var_config(BracedVarStyle::FirstClose, ArrayIndexSyntax::Tcl8),
        )
        .expect("Tcl 8 accepts raw braces")
        .expect("array reference");
        assert_eq!(legacy.index, Some("{key}"));
        let modern = parse_var_ref_parts(
            "$a({key})",
            0,
            var_config(BracedVarStyle::Tcl9Nesting, ArrayIndexSyntax::Tcl9),
        );
        let Err(modern) = modern else {
            panic!("Tcl 9 rejects raw braces");
        };
        assert_eq!(
            modern
                .message_unicode()
                .expect("Unicode fixture error")
                .as_ref(),
            tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX
        );
    }

    #[test]
    fn command_end_finds_matching_bracket() {
        // Flat: the lone `]` closes the substitution.
        assert_eq!(command_end_default(b"[set x]", 0), Some(6));
        // Nested `[...]` raises/lowers depth; the outer `]` matches.
        assert_eq!(command_end_default(b"[a [b] c]", 0), Some(8));
    }

    #[test]
    fn command_end_ignores_brackets_inside_braces() {
        // A `]` inside a brace group is literal and must not close the subst.
        // `[set x {]}]`: the `]` at index 8 is brace-protected; index 10 closes.
        assert_eq!(command_end_default(b"[set x {]}]", 0), Some(10));
    }

    #[test]
    fn command_end_skips_backslash_escapes_and_reports_unbalanced() {
        // `\]` is an escaped bracket, not the closer; the real `]` is at 5.
        assert_eq!(command_end_default(br"[a\]b]", 0), Some(5));
        // No closing bracket at all.
        assert_eq!(command_end_default(b"[a b", 0), None);
    }

    #[test]
    fn whole_braced_strips_a_balanced_whole_word_group() {
        assert_eq!(whole_braced("{abc}"), Some("abc"));
        assert_eq!(whole_braced("{}"), Some("")); // empty group
        // Nested balanced braces stay inside the returned content.
        assert_eq!(whole_braced("{a {b} c}"), Some("a {b} c"));
    }

    #[test]
    fn whole_braced_rejects_non_whole_or_unbalanced_words() {
        // The first `}` closes the leading `{` before the word ends.
        assert_eq!(whole_braced("{a} {b}"), None);
        // Not brace-wrapped / no closer / too short.
        assert_eq!(whole_braced("abc"), None);
        assert_eq!(whole_braced("{abc"), None);
        assert_eq!(whole_braced(""), None);
        // An escaped brace does not count toward depth, so the group is whole.
        assert_eq!(whole_braced(r"{a\}b}"), Some(r"a\}b"));
    }
}
