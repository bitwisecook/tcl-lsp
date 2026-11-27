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

//! Substituting a word: expression sugar, variable and command substitution, located `subst` and array indices.

use super::super::{native_arguments, new_string, Code, Interp};
use crate::obj::TclObj;
use crate::parse::{WordBody, WordPart};
use crate::{obj, parse};
use std::rc::Rc;

impl Interp {
    /// Jim sugar enters the expression evaluator without command lookup or a
    /// fabricated script frame. Its result is owned by the caller.
    pub(in super::super) fn eval_word_expression(
        &mut self,
        source: &[u8],
    ) -> Result<*mut TclObj, Code> {
        #[cfg(have_tommath)]
        {
            if let Err(error) = core::str::from_utf8(source) {
                return Err(self.refuse_unicode_access(
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    },
                ));
            }
            crate::builtins::eval_expr_obj(self, source)
        }
        #[cfg(not(have_tommath))]
        {
            let _ = source;
            Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
            ))
        }
    }

    pub(in super::super) fn word_expression_bytes(
        &mut self,
        source: &[u8],
    ) -> Result<Vec<u8>, Code> {
        let arguments = match native_arguments::OriginalArguments::enter(self, &[]) {
            Ok(arguments) => arguments,
            Err(error) => return Err(self.report_cmd_error(error)),
        };
        let evaluated = self.eval_word_expression(source);
        match evaluated {
            Ok(value) => self
                .word_expression_result_bytes_checked(value, &arguments)
                .map_err(|error| self.report_cmd_error(error)),
            Err(code) => match arguments.check_for(self) {
                Ok(()) => Err(code),
                Err(error) => Err(self.report_cmd_error(error)),
            },
        }
    }

    /// Consume the expression evaluator's actual owning reference on every
    /// outcome; allocation-only argument leases do not add a working hold.
    pub(in super::super) fn word_expression_result_bytes_checked(
        &mut self,
        value: *mut TclObj,
        arguments: &native_arguments::OriginalArguments,
    ) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        let reached: Result<Vec<u8>, tcl_cmd_core::CmdError> = (|| {
            arguments.check_for(self)?;
            let original = native_arguments::OriginalArguments::enter(self, &[value])?;
            let bytes = original.bytes(0)?;
            original.check_original_for(self, 0, value)?;
            Ok(bytes.to_vec())
        })();
        if let Err(ref error) = reached {
            if let Some(first) = error.native_execution_refusal() {
                self.refuse_native_execution(first.clone());
            }
        }
        // SAFETY: the genuine expression result transfers one owning reference.
        unsafe { obj::decr_ref_count(value) };
        arguments.check_for(self)?;
        reached
    }

    /// Substitute one word's body into an **owned** (`+1`) object.
    /// A `Variable` reference to an unset variable, or a `[cmd]` that errors,
    /// returns `Err(code)` with the interp result already set.
    pub(in super::super) fn substitute_word(
        &mut self,
        src: &[u8],
        word: &parse::Word<'_>,
    ) -> Result<*mut TclObj, Code> {
        let native_text = !word.original_text.is_empty()
            || matches!(
                self.native_invocation_dialect()
                    .native_source_string_protocol(),
                Some(tcl_syntax::native_string::NativeStringProtocol::C(_))
            );
        match &word.body {
            WordBody::Literal(bytes) => {
                let obj = new_string(bytes);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
            WordBody::Parts(parts) => {
                // Object-passthrough fast path: a word that is
                // *exactly one* substitution returns that value's **object**
                // (preserving its internal rep), not a stringified copy. This is
                // what keeps `$list`→`lindex`/`llength` etc. O(1) instead of
                // re-shimmering the string each access (the hidden-O(N²) seam).
                if parts.len() == 1 {
                    match &parts[0] {
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(p) => Some(self.subst_index_value(p)?),
                                None => None,
                            };
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            let obj = match index.as_deref() {
                                Some(key) => self.var_get_elem(v.name, key),
                                None => self.var_get(v.name),
                            };
                            return match obj {
                                Some(o) => {
                                    // SAFETY: `o` is a live store-owned object;
                                    // we take an owning +1 to hand to the caller.
                                    unsafe { obj::incr_ref_count(o) };
                                    Ok(o)
                                }
                                None => Err(self.no_such_variable(v.name, index.as_deref())),
                            };
                        }
                        WordPart::Expression(expression) => {
                            return self.eval_word_expression(expression);
                        }
                        WordPart::Command(script) => {
                            // Command substitution propagates *any* non-OK
                            // completion code (`return`/`break`/`continue`, not
                            // just error) out of `[...]`, matching C Tcl.
                            let code = self.eval_command_subst(src, script);
                            if code != Code::Ok {
                                return Err(code);
                            }
                            let r = self.result.get();
                            // SAFETY: the interp result is live; take an owning +1.
                            unsafe { obj::incr_ref_count(r) };
                            return Ok(r);
                        }
                        // single Text/Backslash → fall through to the buffer path
                        _ => {}
                    }
                }
                let mut buf: Vec<u8> = Vec::new();
                for (part_index, part) in parts.iter().enumerate() {
                    match part {
                        WordPart::Text(b) => {
                            let original = if native_text {
                                word.original_text
                                    .iter()
                                    .find(|original| original.part_index == part_index)
                            } else {
                                None
                            };
                            if let Some(original) = original {
                                buf.extend_from_slice(&self.original_c_word_text(original)?);
                            } else {
                                buf.extend_from_slice(b);
                            }
                        }
                        WordPart::Expression(expression) => {
                            buf.extend_from_slice(&self.word_expression_bytes(expression)?);
                        }
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(parts) => Some(self.subst_index_value(parts)?),
                                None => None,
                            };
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => buf.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
                            }
                        }
                        WordPart::Command(script) => {
                            let code = self.eval_command_subst(src, script);
                            if code != Code::Ok {
                                return Err(code);
                            }
                            buf.extend_from_slice(&self.result_bytes());
                        }
                        WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
                    }
                }
                let obj = new_string(&buf);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
        }
    }

    pub(in super::super) fn original_c_word_text(
        &mut self,
        original: &parse::OriginalEscapedText,
    ) -> Result<Vec<u8>, Code> {
        use tcl_syntax::{
            native_string::NativeStringProtocol, raw_string::NativeValueAccessRefusal,
        };
        if let Err(first) = self.check_entered_native_operation() {
            self.refuse_native_execution(first);
            return Err(Code::Error);
        }
        let protocol = self
            .native_invocation_dialect()
            .native_source_string_protocol();
        let Some(protocol @ NativeStringProtocol::C(_)) = protocol else {
            return Err(self.refuse_native_access(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C word source producer",
                ),
            ));
        };
        if self.source_string_protocol() != Some(protocol) {
            return Err(self.refuse_native_access(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C word source producer",
                ),
            ));
        }
        let arena = original.arena.as_ref().map_err(|_| {
            self.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original escaped word geometry",
            ))
        })?;
        if arena.image().channel() != tcl_lexer::SourceChannel::NativeValue
            || arena.config() != self.lexer_config()
        {
            return Err(self.refuse_native_access(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original escaped word channel or grammar",
                ),
            ));
        }
        let mut at = original.span.start();
        let mut output = Vec::new();
        for component in arena.list(arena.root()) {
            let source_span = arena.source_span(component).ok_or_else(|| {
                self.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original escaped word source extent",
                ))
            })?;
            if source_span.start() != at
                || source_span.end() > original.span.end()
                || !matches!(component.part, tcl_lexer::ExecutablePart::Text(_))
            {
                return Err(self.refuse_native_access(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original escaped word component extent",
                    ),
                ));
            }
            let bytes = tcl_syntax::backslash::native_arena_text(
                arena,
                component,
                arena.config().escapes,
                protocol,
            )
            .map_err(|_| {
                self.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original escaped word decoding",
                ))
            })?;
            output.extend_from_slice(&bytes);
            at = source_span.end();
        }
        if at != original.span.end() {
            return Err(self.refuse_native_access(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original escaped word coverage",
                ),
            ));
        }
        if let Err(first) = self.check_entered_native_operation() {
            self.refuse_native_execution(first);
            return Err(Code::Error);
        }
        Ok(output)
    }

    /// Evaluate a quoted expression or raw index through its original arena.
    /// Command completions follow this purpose, independently of `subst`.
    #[cfg(have_tommath)]
    pub(crate) fn do_expression_subst(
        &mut self,
        source: &[u8],
        quoted: bool,
    ) -> Result<Vec<u8>, Code> {
        use tcl_lexer::{ExecutablePart, ExecutablePartArena, SourceImage, Span};
        use tcl_native_model::{ExpressionQuoteControl, LogicalExpressionQuoteProvider};
        let dialect = self.native_invocation_dialect();
        let policy = if quoted {
            Some(dialect.expression_quote_control().or_else(|| dialect.logical_expression_quote_control(LogicalExpressionQuoteProvider::Tcl84CoreSimulation)).ok_or_else(|| self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("native expression quote settlement")))?)
        } else {
            None
        };
        let end = u32::try_from(source.len()).map_err(|_| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression operand extent",
                ),
            )
        })?;
        let arena = ExecutablePartArena::decompose(
            SourceImage::native(source),
            Span::new(0, end),
            Default::default(),
            self.lexer_config(),
        )
        .map_err(|_| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression operand geometry",
                ),
            )
        })?;
        let mut stack = vec![(arena.root(), 0usize, Vec::<u8>::new(), None::<Vec<u8>>)];
        loop {
            let frame = stack.last_mut().expect("operand root frame");
            let Some(component) = arena.list(frame.0).get(frame.1) else {
                let (_, _, out, _) = stack.pop().expect("completed operand");
                let Some(parent) = stack.last_mut() else {
                    return Ok(out);
                };
                let name = parent.3.take().expect("index receiver");
                if let Some(code) = self.fire_read_trace(&name, Some(&out)) {
                    return Err(code);
                }
                let value = self
                    .read_var(&name, Some(&out))
                    .ok_or_else(|| self.no_such_variable(&name, Some(&out)))?;
                parent.2.extend_from_slice(&value);
                continue;
            };
            frame.1 += 1;
            match &component.part {
                ExecutablePart::Text(_) => {
                    let protocol = self.source_string_protocol().ok_or_else(|| {
                        self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("native executable text recipe"))
                    })?;
                    let text = tcl_syntax::backslash::native_arena_text(
                        &arena, component, self.lexer_config().escapes, protocol,
                    ).map_err(|_| self.refuse_native_access(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "native executable text decoding",
                        ),
                    ))?;
                    frame.2.extend_from_slice(&text);
                }
                ExecutablePart::Variable {
                    name,
                    index: Some(index),
                } => {
                    frame.3 = Some(arena.bytes(*name).expect("arena name geometry").to_vec());
                    stack.push((*index, 0, Vec::new(), None));
                }
                ExecutablePart::Variable { name, index: None } => {
                    let name = arena.bytes(*name).expect("arena name geometry");
                    if let Some(code) = self.fire_read_trace(name, None) {
                        return Err(code);
                    }
                    let value = self
                        .read_var(name, None)
                        .ok_or_else(|| self.no_such_variable(name, None))?;
                    frame.2.extend_from_slice(&value);
                }
                ExecutablePart::Command { body } => {
                    let code = self.eval_command_subst(
                        arena.image().bytes(),
                        arena.bytes(*body).expect("arena command geometry"),
                    );
                    let code = if policy == Some(ExpressionQuoteControl::Jim084) {
                        match code {
                            Code::Return => {
                                self.return_code.set(Code::Ok);
                                Code::Ok
                            }
                            Code::Break => self.error(b"invoked \"break\" outside of a loop"),
                            Code::Continue => self.error(b"invoked \"continue\" outside of a loop"),
                            Code::Other(_) => Code::Error,
                            other => other,
                        }
                    } else {
                        code
                    };
                    if code != Code::Ok {
                        return Err(code);
                    }
                    frame.2.extend_from_slice(&self.result_bytes());
                }
                ExecutablePart::Expression { expression } => {
                    frame.2.extend_from_slice(&self.word_expression_bytes(
                        arena.bytes(*expression).expect("arena expression geometry"),
                    )?)
                }
                ExecutablePart::ParseError(message) => return Err(self.error(message.as_bytes())),
            }
        }
    }

    /// Substitute with the input string's TIP 280 location
    /// (the `subst` command's argument word): a `[...]` inside the substituted
    /// string then reports the line it appears on (C compiles `subst` with the
    /// argument's line table). `loc` is `None` for internal callers that do not
    /// track lines (`dict map` key substitution), where the `[...]` is line-
    /// tracked relative to the enclosing frame as usual.
    pub(crate) fn do_subst_located(
        &mut self,
        src: &[u8],
        flags: crate::subst::SubstFlags,
        loc: Option<(Option<Rc<[u8]>>, u32)>,
    ) -> Result<Vec<u8>, Code> {
        let body = crate::subst::scan(src, flags, self.lexer_config());
        match &body {
            WordBody::Literal(b) => Ok(b.to_vec()),
            WordBody::Parts(parts) => {
                // Align the enclosing frame to the argument word's file+line, so
                // each `[...]`'s line (computed by `eval_command_subst` against
                // `src`) is file-absolute. Saved/restored around the resolution.
                let saved = loc.and_then(|(file, line)| {
                    let mut frames = self.cmd_frames.borrow_mut();
                    frames.last_mut().map(|top| {
                        let prev = (top.line_base, top.file.clone());
                        top.line_base = line.saturating_sub(1);
                        if file.is_some() {
                            top.file = file;
                        }
                        prev
                    })
                });
                let result = self.resolve_subst_parts(src, parts);
                if let Some((line_base, file)) = saved {
                    if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
                        top.line_base = line_base;
                        top.file = file;
                    }
                }
                result
            }
        }
    }

    /// Resolve substitution `parts` (scanned from `src`) to bytes, propagating
    /// any non-OK code (the `subst`-command path; cf. `substitute_word`, which
    /// builds an object). `src` lets a `[...]` part advance the `info frame`
    /// line to its own position via [`eval_command_subst`](Self::eval_command_subst).
    fn resolve_subst_parts(&mut self, src: &[u8], parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => out.extend_from_slice(b),
                WordPart::Expression(expression) => {
                    out.extend_from_slice(&self.word_expression_bytes(expression)?);
                }
                WordPart::Variable(v) => {
                    // A `break`/`continue`/`return` from a `[...]` in the array
                    // index diverts the whole variable substitution (C's
                    // `TCL_TOKEN_VARIABLE` arm of `TclSubstTokens`): on `break`
                    // the subst ends, on `continue` this variable contributes
                    // nothing, on `return` the result is substituted in place of
                    // the variable's value (which is not looked up).
                    let (index, idx_code) = match &v.index {
                        Some(p) => {
                            let (val, code) = self.subst_index(p)?;
                            (Some(val), code)
                        }
                        None => (None, Code::Ok),
                    };
                    match idx_code {
                        Code::Ok => {
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => out.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
                            }
                        }
                        Code::Break => break,
                        Code::Continue => {}
                        _ => out.extend_from_slice(&self.result_bytes()),
                    }
                }
                WordPart::Command(script) => {
                    // `subst`'s per-`[...]` completion-code rule (C's compiled
                    // subst / `TclSubstTokens`): `break` ends the whole
                    // substitution (returning what's accumulated), `continue`
                    // contributes nothing for this bracket, and any other
                    // non-error code (`return`, custom) substitutes its result.
                    // Only a genuine error propagates.
                    match self.eval_command_subst(src, script) {
                        Code::Ok | Code::Return | Code::Other(_) => {
                            out.extend_from_slice(&self.result_bytes())
                        }
                        Code::Break => break,
                        Code::Continue => {}
                        Code::Error => return Err(Code::Error),
                    }
                }
                // Reached in evaluation order, so `[...]` parts before it have
                // already run and kept their side effects — C's behaviour.
                WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
            }
        }
        Ok(out)
    }

    /// [`subst_index`](Self::subst_index) for the word-substitution path, where a
    /// non-OK completion code in the index propagates as an error/code (rather
    /// than diverting an enclosing `subst`).
    fn subst_index_value(&mut self, parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        match self.subst_index(parts)? {
            (val, Code::Ok) => Ok(val),
            (_, code) => Err(code),
        }
    }

    /// Resolve a `$arr(index)` index (itself substituted) to its bytes plus the
    /// completion code of the last `[...]` in it (`Ok` normally; `Break`/
    /// `Continue`/`Return` divert the enclosing variable substitution per C's
    /// `TclSubstTokens`). An error propagates as `Err`.
    fn subst_index(&mut self, parts: &[WordPart]) -> Result<(Vec<u8>, Code), Code> {
        let mut buf = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => buf.extend_from_slice(b),
                WordPart::Expression(expression) => {
                    buf.extend_from_slice(&self.word_expression_bytes(expression)?);
                }
                WordPart::Variable(v) => {
                    let (idx, idx_code) = match &v.index {
                        Some(p) => {
                            let (val, code) = self.subst_index(p)?;
                            (Some(val), code)
                        }
                        None => (None, Code::Ok),
                    };
                    match idx_code {
                        Code::Ok => match self.read_var(v.name, idx.as_deref()) {
                            Some(bytes) => buf.extend_from_slice(&bytes),
                            None => return Err(self.no_such_variable(v.name, idx.as_deref())),
                        },
                        other => return Ok((buf, other)),
                    }
                }
                WordPart::Command(script) => match self.eval_str(script) {
                    Code::Ok => buf.extend_from_slice(&self.result_bytes()),
                    Code::Error => return Err(Code::Error),
                    Code::Return => {
                        buf.extend_from_slice(&self.result_bytes());
                        return Ok((buf, Code::Return));
                    }
                    other => return Ok((buf, other)),
                },
                WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
            }
        }
        Ok((buf, Code::Ok))
    }
}
