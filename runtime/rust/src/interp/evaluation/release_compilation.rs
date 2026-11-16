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

//! What the emulated release compiles: index and literal words, control commands, compiled locals and the 8.4 compile failure.

use super::super::{native_operation_currency, new_string, obj_bytes, release_all, Code, Interp};
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// `argv` with its index words at `at` respelt as the release compiles
    /// them, where the command is one it compiles, selected by literal words
    /// (`IndexCompiler::selecting_words`), and they are literals it reads
    /// apart from the command (`tcl_cmd_core::index::compiled_index_words`);
    /// `None` where it passes every word as written.
    pub(crate) fn compiled_index_argv(
        &self,
        compiler: tcl_cmd_core::index::IndexCompiler,
        argv: &[*mut TclObj],
        at: &[usize],
    ) -> Option<CompiledArgv> {
        if !tcl_cmd_core::index::compiles_literals_apart(self.runtime_version()) {
            return None;
        }
        let words: Vec<Option<String>> = {
            let literals = self.compiled_literals.borrow();
            let base = argv.as_ptr() as usize;
            if !(0..compiler.selecting_words()).all(|idx| literals.contains(&(base, idx))) {
                return None;
            }
            at.iter()
                .map(|&idx| {
                    (idx < argv.len() && literals.contains(&(base, idx)))
                        .then(|| String::from_utf8_lossy(&obj_bytes(argv[idx])).into_owned())
                })
                .collect()
        };
        let words: Vec<Option<&str>> = words.iter().map(Option::as_deref).collect();
        let respelt =
            tcl_cmd_core::index::compiled_index_words(compiler, &words, self.runtime_version());
        if respelt.iter().all(Option::is_none) {
            return None;
        }
        let mut compiled = CompiledArgv {
            argv: argv.to_vec(),
            fresh: Vec::new(),
        };
        for (&idx, spelling) in at.iter().zip(respelt) {
            if let Some(spelling) = spelling {
                let o = new_string(spelling.as_bytes());
                // SAFETY: a fresh object; `CompiledArgv` releases this hold.
                unsafe { obj::incr_ref_count(o) };
                compiled.argv[idx] = o;
                compiled.fresh.push(o);
            }
        }
        Some(compiled)
    }

    /// Which words of `argv` are literal where it is a command the release
    /// compiles, selected by its literal command word, and `None` where it
    /// is evaluated directly or invoked through a substituted word.
    pub(crate) fn compiled_words(&self, argv: &[*mut TclObj]) -> Option<Vec<bool>> {
        let literals = self.compiled_literals.borrow();
        let base = argv.as_ptr() as usize;
        literals.contains(&(base, 0)).then(|| {
            (0..argv.len())
                .map(|idx| literals.contains(&(base, idx)))
                .collect()
        })
    }

    /// Whether the release compiles the structured control command `argv`
    /// where it stands ([`tcl_registry::release_compilation::compiles_control`]):
    /// its bodies then run as code of the script holding it, an error leaving
    /// no frame of the command's own.
    pub(crate) fn release_compiles_control(&self, argv: &[*mut TclObj]) -> bool {
        let Some(literal) = self.compiled_words(argv) else {
            return false;
        };
        let texts: Vec<String> = argv
            .iter()
            .map(|&word| String::from_utf8_lossy(&obj_bytes(word)).into_owned())
            .collect();
        let words: Vec<tcl_registry::release_compilation::CompileWord<'_>> = texts[1..]
            .iter()
            .zip(&literal[1..])
            .map(
                |(text, &literal)| tcl_registry::release_compilation::CompileWord { text, literal },
            )
            .collect();
        tcl_registry::release_compilation::compiles_control(
            self.runtime_version(),
            &texts[0],
            &words,
            self.compiles_locals(),
        ) == Some(true)
    }

    /// The inline codegen hook through which the release compiles `argv`,
    /// a command it compiles ([`Self::compiled_words`]) whose subcommand
    /// word, where it has one, is literal too, with which words are literal;
    /// `None` where the command runs it.
    pub(crate) fn release_compiled_hook(
        &self,
        argv: &[*mut TclObj],
    ) -> Option<(tcl_registry::hooks::InlineCodegenHookId, Vec<bool>)> {
        let literal = self.compiled_words(argv)?;
        let registry = self.0.profile_registry.get()?;
        // An inline hook is a command's or a subcommand's, never a form's, so
        // the command and subcommand words select it.
        let words: Vec<String> = argv
            .iter()
            .take(2)
            .map(|&word| String::from_utf8_lossy(&obj_bytes(word)).into_owned())
            .collect();
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
        let call = registry.resolve_call(&words[0], &args, self.0.dialect_point.get())?;
        if call.sub.is_some() && !literal.get(1).copied().unwrap_or(false) {
            return None;
        }
        Some((call.inline_codegen_hook?, literal))
    }

    /// Whether each of `names` is a compiled local of the procedure body the
    /// active variable frame activates, which a script the release compiles
    /// in that frame finds (`TclFindCompiledLocal` without a `procPtr`).
    pub(crate) fn frame_compiled_locals(&self, names: &[Vec<u8>]) -> bool {
        let Some(unit) = self.frames.borrow().active_unit() else {
            return false;
        };
        let Some(registry) = self.0.profile_registry.get() else {
            return false;
        };
        unit.has_locals(
            names,
            &crate::compiled_locals::Release {
                registry,
                point: self.0.dialect_point.get(),
                version: self.runtime_version(),
                config: self.lexer_config(),
            },
        )
    }

    /// Why the emulated release refuses to compile `src` whole, as 8.4's
    /// `TclCompileScript` does (`tcl_registry::compile_84`), a procedure body
    /// where `in_procedure` says; `None` where it compiles, or the release
    /// compiles leniently.
    pub(in super::super) fn compile_failure_84(
        &self,
        src: &[u8],
        in_procedure: bool,
        namespace: NsId,
    ) -> Option<tcl_runtime_api::CompileFailure> {
        if !self.runtime_version().compile_errors_fail_scripts() {
            return None;
        }
        let registry = self.0.profile_registry.get()?;
        let source = std::str::from_utf8(src).ok()?;
        let parser = crate::compile_84::Parser {
            config: self.lexer_config(),
        };
        tcl_registry::compile_84::compile_failure(
            source,
            &tcl_registry::compile_84::Compile {
                registry,
                point: self.0.dialect_point.get(),
                segmenter: &parser,
                in_procedure,
                parses_expressions: self.dialect_profile().f5_core_expr_grammar().is_none(),
                compiles: &|head| self.compiles_with_stock_procedure(namespace, head.as_bytes()),
            },
        )
    }

    /// Whether `head` finds a stock command, whose compile procedure the
    /// registry describes, with no execution trace.
    fn compiles_with_stock_procedure(&self, namespace: NsId, head: &[u8]) -> bool {
        let Some((_, _, _, lookup)) = self.native_compiler_target(namespace, head) else {
            return false;
        };
        !self.traces.borrow().cmd_traces.iter().any(|trace| {
            trace.token == Some(lookup) && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
        })
    }

    /// Raise a compile's refusal: its message, with its `while compiling`
    /// frames and `tail` as the trace so far, and its error code.
    pub(in super::super) fn raise_compile_failure(
        &mut self,
        failure: &tcl_runtime_api::CompileFailure,
        tail: &[u8],
    ) -> Code {
        let mut info = failure.message.as_bytes().to_vec();
        info.extend_from_slice(failure.error_info.as_bytes());
        info.extend_from_slice(tail);
        self.error_line.set(failure.line);
        let code = failure.error_code.as_deref().unwrap_or("NONE");
        if self.uses_c84_global_error_info() && failure.error_code.is_none() {
            let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
                return Code::Error;
            };
            self.set_result_bytes(failure.message.as_bytes());
            if scope.currency().ensure_current_or_refuse().is_err() {
                return Code::Error;
            }
            // LogCompilationInfo reaches AddObjErrorInfo before the outer
            // catch. Reuse its initial global setters and counted append;
            // setting legacy_copy alone would skip those reached operations.
            let suffix = &info[failure.message.len()..];
            return match self.publish_c84_error_command_info(suffix, code.as_bytes()) {
                Ok(true) => Code::Error,
                Ok(false) => {
                    self.refuse_completion_value_access(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "entered C8.4 compilation error publication",
                        ),
                    );
                    Code::Error
                }
                Err(first) => self.refuse_native_execution(first),
            };
        }
        self.raise_with_info(failure.message.as_bytes(), &info, code.as_bytes())
    }

    /// Whether the commands now running are compiled as part of a procedure
    /// body, so a compile procedure creates the locals it names
    /// ([`InterpState::proc_body`]).
    pub(crate) fn compiles_locals(&self) -> bool {
        self.proc_body.get()
    }

    /// The recorded TIP 280 source location of a script obj (C's `lineLABCPtr`
    /// lookup), or `None` for a dynamic/computed script. Scans newest-first.
    pub(in super::super) fn arg_loc(&self, obj: *mut TclObj) -> Option<(Option<Rc<[u8]>>, u32)> {
        if let Some(location) = self
            .uses_jim_error_stack()
            .then(|| obj::script_location(obj))
            .flatten()
        {
            return Some(location);
        }
        self.arg_locs
            .borrow()
            .iter()
            .rev()
            .find(|(o, _, _)| *o == obj)
            .map(|(_, f, l)| (f.clone(), *l))
    }
}

/// A command's words with index words respelt as the release compiles them
/// ([`Interp::compiled_index_argv`]); the respelt words are released with it.
pub(crate) struct CompiledArgv {
    pub(crate) argv: Vec<*mut TclObj>,
    pub(in super::super) fresh: Vec<*mut TclObj>,
}

impl Drop for CompiledArgv {
    fn drop(&mut self) {
        release_all(&self.fresh);
    }
}
