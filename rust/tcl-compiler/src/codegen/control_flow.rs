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

//! Catch/try/error handling control flow emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting `beginCatch4`/`endCatch`
//! bytecodes for `catch` and `try` commands.

use tcl_lexer::Span;
use tcl_registry::completion::CompletionCode;
use tcl_registry::hooks::{InlineCodegenHookId, LoweringHookId};
use tcl_registry::release_compilation::CompileWord;
use tcl_registry::{Traits, TryClauseKind};
use tcl_runtime_api::completion_options::ControlOptionPolicy;

use crate::cfg::Function as CfgFunction;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;

use super::cmd_subst::{parse_script_parts_expand_with_config, parse_script_parts_with_config};
use super::values::is_qualified;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

// Constant expression error detection

/// Detect compile-time expression errors (e.g. divide by zero).
///
/// Returns `(error_message, error_options_string)` when the AST is a
/// fully constant expression that would raise a known error at runtime.
#[must_use]
pub fn detect_const_expr_error(node: &ExprNode) -> Option<(String, String)> {
    if let ExprNode::Binary { op, left, right } = node
        && matches!(op, BinOp::Div | BinOp::Mod)
        && let (ExprNode::Literal { .. }, ExprNode::Literal { text: rv, .. }) =
            (left.as_ref(), right.as_ref())
        && rv == "0"
    {
        return Some((
            "divide by zero".to_owned(),
            "-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}}".to_owned(),
        ));
    }
    None
}

#[derive(Clone, Copy)]
enum InlineBodyEmitter {
    Catch,
    TryHandler,
}

#[derive(Debug, Clone)]
struct TryHandlerBindings {
    body_index: usize,
    handler_body_index: usize,
    result_var: Option<String>,
    options_var: Option<String>,
}

/// Whether a value already resolved by Tcl's word/list grammar can safely use
/// the procedure-local scalar bytecodes emitted by the narrow catch/try
/// specialisers. Other valid Tcl variable names (qualified names, arrays, and
/// names requiring quoting) stay on the generic runtime path.
fn is_inline_local_scalar_name(name: &str) -> bool {
    tcl_syntax::naming::is_static_variable_source_word(name) && !is_qualified(name)
}

/// Whether the release's compiler binds the variable name `name` to a
/// procedure-local scalar (`TclLocalScalar`): no namespace separator anywhere
/// and no array element.
fn is_release_local_scalar_name(name: &str) -> bool {
    !name.contains("::") && !super::values::is_array_ref(name)
}

/// One word of a command as the release's compiler sees it.
#[derive(Clone, Copy)]
struct ReleaseWord<'a> {
    text: &'a str,
    braced: bool,
}

impl ReleaseWord<'_> {
    /// Known at compile time (`TclWordKnownAtCompileTime`): no variable or
    /// command substitution.
    fn known(self) -> bool {
        !tcl_syntax::naming::word_is_dynamic(self.text, self.braced)
    }

    /// One text token (`TCL_TOKEN_SIMPLE_WORD`): known, and no backslash
    /// substitution either.
    fn simple(self) -> bool {
        self.known() && (self.braced || !self.text.contains('\\'))
    }
}

/// Parse a `dict for`/`dict map` variable-list word by the Tcl list grammar
/// and return the two loop-variable names when it is exactly a two-element
/// list of *plain* names — not qualified (`::`), not an array element (`(`),
/// and not needing list quoting (whitespace/empty). Returns `None` (inline
/// emitter bails to the runtime invoke) otherwise, so a 1-element list like
/// `{{a b}}` errors at runtime instead of being miscompiled as two vars, and a
/// spaced name like `{{a b} v}` keeps its runtime semantics.
fn is_two_plain_names(vars_text: &str) -> Option<[String; 2]> {
    let elems = super::helpers::split_list_simple(vars_text);
    let [a, b] = <[String; 2]>::try_from(elems).ok()?;
    for v in [&a, &b] {
        if v.is_empty() || is_qualified(v) || v.contains('(') || v.chars().any(char::is_whitespace)
        {
            return None;
        }
    }
    Some([a, b])
}

/// Normalise a parsed `return -code C -level L` into the `(code, level)`
/// immediates `returnImm` carries, exactly as C's `TclMergeReturnOptions`
/// hands them to `CompileReturnInternal`.
///
/// Two rules do all the work (`tclResult.c:866-974`):
///
/// 1. `-level` defaults to **1**, not 0 — a plain `return` is `(0, 1)`, and
///    `(0, 0)` means something else entirely (`TclProcessReturn` at level 0
///    with code OK falls through to the next instruction).
/// 2. `-code return` is rewritten to `-code ok -level L+1`, so `TCL_RETURN`
///    never reaches the operand: `return -code return` is `(0, 2)`, and
///    `return -code return -level 3` is `(0, 4)`.
///
/// Every other code rides its own operand with the level untouched:
/// `-code error` → `(1, 1)`, `-code break` → `(3, 1)`, `-code continue` →
/// `(4, 1)`, `-code N` → `(N, 1)`.
fn merge_return_operands(code: Option<i32>, level: Option<i32>) -> (i32, i32) {
    let mut ret_code = code.unwrap_or(0);
    let mut ret_level = level.unwrap_or(1);
    if ret_code == 2 {
        ret_level = ret_level.saturating_add(1);
        ret_code = 0;
    }
    (ret_code, ret_level)
}

// CodegenCtx methods

/// Instruction indices in [`CodegenCtx::emit_dict_map`] whose slot operands are
/// emitted with a placeholder and back-patched once the result and iterator
/// temps are interned (they take the highest slots, after the body's locals).
struct DictMapPatch {
    res_store: usize,
    dict_first: usize,
    dict_set: usize,
    dict_next: usize,
    unset_it_err: usize,
    unset_res_err: usize,
    unset_it_exit: usize,
    res_load: usize,
    unset_res_exit: usize,
}

impl CodegenCtx<'_> {
    /// How the release compiles a generic invocation of `command` it compiles
    /// where the VM leaves the command to run, following C's
    /// `TclCompileCatchCmd`, `TclCompileTryCmd` and `TclCompileForCmd`. Their
    /// script word, when it is not one literal, compiles through
    /// `TclCompileCmdWord`, whose `INST_EVAL_STK` compiles the value whatever
    /// it holds, and a compiled `try` binds the variables of the clause that
    /// matched where the command binds those of the clause whose script runs.
    /// Measured on tclsh 8.6.18, where a canonical list a command evaluates
    /// directly reads a literal index apart: `catch $b r`, `try $b` and
    /// `for $b {0} {} {}` in a procedure compile `$b`, `lmap x {1} $b` or
    /// `catch $b ::r` does not.
    ///
    /// The release compiles none of these in a command it evaluates directly,
    /// and binds a handler or result variable only to a local of a procedure
    /// body; a script `eval` compiles in a procedure frame can bind one of the
    /// procedure's existing locals, which the VM checks as the command runs.
    pub(crate) fn release_compiled(
        &mut self,
        command: &str,
        args: &[(&str, bool)],
    ) -> Option<tcl_bytecode::ReleaseCompiled> {
        if self.plain_command_dispatch || self.in_direct_command() {
            return None;
        }
        let words: Vec<ReleaseWord<'_>> = args
            .iter()
            .map(|&(text, braced)| ReleaseWord { text, braced })
            .collect();
        let arg_refs: Vec<&str> = args.iter().map(|&(text, _)| text).collect();
        // As the inline specialisations: the registry's hook for this
        // invocation, its binding retained so a later redefinition recompiles.
        let resolved =
            self.registry
                .resolve_call(command, &arg_refs, self.invocation_surface_query())?;
        if resolved.spec.name != command {
            return None;
        }
        let binding = self.command_binding_identity(command, resolved.spec.name);
        let mut in_procedure_only = false;
        let frame_local_words = match resolved.lowering_hook {
            Some(LoweringHookId::Catch) => {
                let admitted = self.release_admits_arity(command, args.len());
                let mask = admitted
                    .then(|| self.release_compiles_catch(&words))
                    .flatten();
                // A procedure body's `catch` makes each variable a local
                // (`LocalScalarFromToken`: an element names its array's, and
                // stops the compile), and compiles a literal script with it.
                if admitted && self.compiles_locals() {
                    for var in words.iter().skip(1) {
                        if !var.simple() || is_qualified(var.text) {
                            break;
                        }
                        self.intern_compiled_local(var.text);
                        if super::values::is_array_ref(var.text) {
                            break;
                        }
                    }
                }
                if mask == Some(0)
                    && let Some(script) = words.first().filter(|script| script.simple())
                {
                    self.intern_script_locals(script.text);
                }
                mask
            }
            Some(LoweringHookId::Try) => {
                let compiled = self.release_compiles_try(command, &words, &arg_refs);
                if compiled {
                    self.intern_try_variables(command, &arg_refs);
                }
                compiled.then_some(0)
            }
            // `TclCompileSwitchCmd` compiles each body of a form it takes,
            // making its locals, and the command logs no error of its own.
            Some(LoweringHookId::Switch) => {
                let compile_words: Vec<CompileWord<'_>> = words
                    .iter()
                    .map(|word| CompileWord {
                        text: word.text,
                        literal: word.simple(),
                    })
                    .collect();
                let bodies = tcl_registry::release_compilation::compiled_switch_scripts(
                    self.vm_release(),
                    &compile_words,
                )?;
                if self.compiles_locals() {
                    for body in bodies {
                        self.intern_script_locals(body);
                    }
                }
                Some(0)
            }
            Some(LoweringHookId::For) => matches!(words.as_slice(), [_, test, next, body]
                    if test.simple() && next.simple() && body.simple())
            .then_some(0),
            // `TclCompileWhileCmd` compiles a braced test with any body.
            Some(LoweringHookId::While) => {
                matches!(words.as_slice(), [test, _] if test.simple()).then_some(0)
            }
            _ => {
                in_procedure_only = !self.compiles_locals();
                self.release_compiles_each_loop(command, &words)
            }
        }?;
        let shape = self.invocation_shape(&arg_refs, |index| args[index].1);
        self.require_invocation_binding(&binding, &shape);
        Some(tcl_bytecode::ReleaseCompiled {
            script_word: words
                .first()
                .is_some_and(|script| !script.simple())
                .then_some(1),
            frame_local_words,
            in_procedure_only,
        })
    }

    /// `foreach`, `lmap`, `dict for` and `dict map` the release compiles: in
    /// a procedure body, whose locals their loop variables and the body's
    /// become, and the command logs no error of its own. A script compiled
    /// apart from a procedure body compiles one only where it runs as part of
    /// one ([`tcl_bytecode::ReleaseCompiled::in_procedure_only`]).
    fn release_compiles_each_loop(
        &mut self,
        command: &str,
        words: &[ReleaseWord<'_>],
    ) -> Option<u8> {
        let compile_words: Vec<CompileWord<'_>> = words
            .iter()
            .map(|word| CompileWord {
                text: word.text,
                literal: word.simple(),
            })
            .collect();
        let Some(compiled) = tcl_registry::release_compilation::compiles_control(
            self.vm_release(),
            command,
            &compile_words,
            true,
        ) else {
            return self.release_compiles_dict_scope(command, &compile_words);
        };
        if !compiled {
            return None;
        }
        if !self.compiles_locals() {
            return Some(0);
        }
        for name in
            tcl_registry::release_compilation::compiled_loop_variables(command, &compile_words)
        {
            self.intern_compiled_local(&name);
        }
        if let Some(body) = words.last() {
            self.intern_script_locals(body.text);
        }
        Some(0)
    }

    /// `dict update` (`TclCompileDictUpdateCmd`, from 8.5) and `dict with`
    /// (`TclCompileDictWithCmd`, from 8.6) the release compiles in a procedure
    /// body, where the command then logs no body frame of its own. Measured
    /// with `::tcl::unsupported::disassemble` on tclsh 8.5.19, 8.6.18 and
    /// 9.0.4: `dict update` needs a literal body and its dictionary and
    /// update variables literal local scalars (`LocalScalarFromToken`), so
    /// `::d` or `a(1)` leave it to the command; `dict with` needs only a
    /// literal body, its variable a local when it is a local scalar name.
    /// 8.5 has no `dict with` compiler. Apart from a procedure body neither
    /// compiles a body that can fail: an update needs the procedure's
    /// locals, and a non-empty `dict with` body its local table.
    fn release_compiles_dict_scope(
        &mut self,
        command: &str,
        words: &[CompileWord<'_>],
    ) -> Option<u8> {
        let (sub, rest) = match command.strip_prefix("::").unwrap_or(command) {
            "dict" => {
                let (sub, rest) = words.split_first()?;
                (sub.literal.then_some(sub.text)?, rest)
            }
            "tcl::dict::update" => ("update", words),
            "tcl::dict::with" => ("with", words),
            _ => return None,
        };
        let local_scalar = |word: &CompileWord<'_>| {
            word.literal && !is_qualified(word.text) && !super::values::is_array_ref(word.text)
        };
        let (body, scope) = rest.split_last()?;
        if !body.literal {
            return None;
        }
        let locals: Vec<&str> = match sub {
            "update" if self.vm_release() >= tcl_dialect::TclVersion::V8_5 => {
                let (variable, pairs) = scope.split_first()?;
                if !local_scalar(variable)
                    || pairs.is_empty()
                    || pairs.len() % 2 != 0
                    || !pairs.chunks(2).all(|pair| local_scalar(&pair[1]))
                {
                    return None;
                }
                std::iter::once(variable.text)
                    .chain(pairs.chunks(2).map(|pair| pair[1].text))
                    .collect()
            }
            "with" if self.vm_release() >= tcl_dialect::TclVersion::V8_6 => {
                let variable = scope.first()?;
                if local_scalar(variable) {
                    vec![variable.text]
                } else {
                    Vec::new()
                }
            }
            _ => return None,
        };
        if !self.compiles_locals() {
            return Some(0);
        }
        for name in locals {
            self.intern_compiled_local(name);
        }
        self.intern_script_locals(body.text);
        Some(0)
    }

    /// Whether the release's signature of `command` admits `count` arguments.
    pub(crate) fn release_admits_arity(&self, command: &str, count: usize) -> bool {
        self.registry.get(command).is_none_or(|spec| {
            self.registry
                .release_arity(spec)
                .accepts(u16::try_from(count).unwrap_or(u16::MAX))
        })
    }

    /// `TclCompileCatchCmd`: a result or options variable needs a local. A
    /// procedure body creates one; any other script finds only those of the
    /// procedure frame it runs in, so the answer is the words the VM checks
    /// there ([`tcl_bytecode::ReleaseCompiled::frame_local_words`]).
    fn release_compiles_catch(&self, words: &[ReleaseWord<'_>]) -> Option<u8> {
        let (_, vars) = words.split_first()?;
        if !vars
            .iter()
            .all(|var| var.simple() && is_release_local_scalar_name(var.text))
        {
            return None;
        }
        if self.compiles_locals() {
            return Some(0);
        }
        // The variables follow the command word and the script.
        Some((2..2 + vars.len()).fold(0, |mask, word| mask | 1 << word))
    }

    /// `TclCompileTryCmd`: every handler's selector and variable list known,
    /// its variables procedure locals and its script one literal, the
    /// `finally` script one literal, and a handler needs procedure locals of
    /// its own for the result and options.
    fn release_compiles_try(
        &self,
        command: &str,
        words: &[ReleaseWord<'_>],
        arg_refs: &[&str],
    ) -> bool {
        let Some(invocation) = self.registry.try_control_invocation(
            command,
            arg_refs,
            self.invocation_surface_query(),
        ) else {
            return false;
        };
        let word = |index: usize| words.get(index).copied();
        invocation.clauses.iter().all(|clause| {
            let body = word(clause.body_index).is_some_and(ReleaseWord::simple);
            if clause.kind == TryClauseKind::Finally {
                return body;
            }
            let selector = clause
                .selector_index
                .and_then(word)
                .is_some_and(|selector| {
                    selector.known()
                        && (clause.kind != TryClauseKind::Trap
                            || tcl_syntax::list::split_list(selector.text)
                                .is_ok_and(|codes| !codes.is_empty()))
                });
            let variables = clause
                .variable_list_index
                .and_then(word)
                .is_some_and(|list| {
                    list.known()
                        && tcl_syntax::list::split_list(list.text).is_ok_and(|names| {
                            names.len() <= 2
                                && names.iter().all(|name| is_release_local_scalar_name(name))
                        })
                });
            self.compiles_locals() && body && selector && variables
        })
    }

    /// Make each handler variable of a `try` compiled locals, as
    /// `TclCompileTryCmd` does in a procedure body.
    fn intern_try_variables(&mut self, command: &str, arg_refs: &[&str]) {
        let Some(invocation) = self.registry.try_control_invocation(
            command,
            arg_refs,
            self.invocation_surface_query(),
        ) else {
            return;
        };
        let names: Vec<String> = invocation
            .clauses
            .iter()
            .filter_map(|clause| clause.variable_list_index)
            .filter_map(|index| arg_refs.get(index))
            .filter_map(|list| tcl_syntax::list::split_list(list).ok())
            .flatten()
            .map(|name| name.to_string())
            .collect();
        for name in names {
            self.intern_compiled_local(&name);
        }
        // Then it compiles the body and each handler with it.
        let scripts: Vec<&str> = std::iter::once(invocation.body_index)
            .chain(invocation.clauses.iter().map(|clause| clause.body_index))
            .filter_map(|index| arg_refs.get(index).copied())
            .collect();
        for script in scripts {
            self.intern_script_locals(script);
        }
    }

    /// Whether the literal-body `catch` specialiser can preserve the exact
    /// source-word semantics of every argument.
    ///
    /// A braced body is already the script value which `catch` will evaluate.
    /// An unbraced/quoted body must first undergo Tcl substitution and escape
    /// decoding, so it belongs to the generic, explicit-stack runtime command.
    /// Result/options variable names may be bare literals, but only the shared
    /// Tcl word classifier and the local-scalar shape acceptor may prove them.
    pub(crate) fn catch_inline_args_are_static(args: &[(String, bool)]) -> bool {
        args.first().is_some_and(|(_, braced)| *braced)
            && args.iter().skip(1).all(|(name, braced)| {
                !tcl_syntax::naming::word_is_dynamic(name, *braced)
                    && !name.contains('\\')
                    && is_inline_local_scalar_name(name)
            })
    }

    /// Query the retained native protocol before opening a catch range. A
    /// handler identity alone cannot select the observable output store order.
    pub(crate) fn catch_output_order(
        &self,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> tcl_registry::catch_invocation::CatchOutputOrder {
        use tcl_registry::catch_invocation::{CatchInvocation, CatchOutputOrder};
        use tcl_registry::native_compilation::NativeCompilationSelection;
        if result_var.is_none() || options_var.is_none() {
            return CatchOutputOrder::ResultThenOptions;
        }
        let binding = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref());
        let dialect = binding
            .and_then(crate::command_binding::SourceInvocationBinding::native_compiler_dialect)
            .unwrap_or_else(|| {
                crate::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    self.dialect,
                    self.lexer_config(),
                )
            });
        let selection = binding.map_or(
            NativeCompilationSelection::Unknown,
            crate::command_binding::SourceInvocationBinding::native_compilation_admission_selection,
        );
        CatchInvocation {
            result_var_at: Some(1),
            options_var_at: Some(2),
            ..CatchInvocation::CAPTURE_TCL_PHASE
        }
        .output_order(dialect, selection)
    }

    /// This emitter handles the positional capture grammar only. In particular,
    /// Jim's filtering flags are parsed by the central grammar and stay generic.
    pub(crate) fn catch_inline_shape_is_supported(&self, args: &[(String, bool)]) -> bool {
        use tcl_registry::catch_invocation::{CatchInvocationSelection, CatchOutputOrder};
        let dialect = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(crate::command_binding::SourceInvocationBinding::native_compiler_dialect)
            .unwrap_or_else(|| {
                crate::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    self.dialect,
                    self.lexer_config(),
                )
            });
        let words: Vec<_> = args.iter().map(|(word, _)| word.as_str()).collect();
        let CatchInvocationSelection::Valid(capture) =
            tcl_registry::catch_invocation::select_catch_invocation(
                tcl_registry::InvocationArguments::literals(&words),
                dialect,
            )
        else {
            return false;
        };
        capture.script_at == 0
            && capture.ignored_codes == 0
            && capture.result_var_at == (args.len() >= 2).then_some(1)
            && capture.options_var_at == (args.len() >= 3).then_some(2)
            && Self::catch_inline_args_are_static(args)
            && self.catch_output_order(
                args.get(1).map(|(s, _)| s.as_str()),
                args.get(2).map(|(s, _)| s.as_str()),
            ) != CatchOutputOrder::Unknown
    }

    /// Consume captured result/options objects while looking up each output
    /// only at its own write. A preceding write trace may retarget the next name.
    fn emit_catch_output_stores(
        &mut self,
        result_var: Option<&str>,
        options_var: Option<&str>,
        order: tcl_registry::catch_invocation::CatchOutputOrder,
    ) {
        use tcl_registry::catch_invocation::CatchOutputOrder;
        if let Some(options) = options_var {
            match order {
                CatchOutputOrder::ResultThenOptions => {
                    self.emit(Op::REVERSE, vec![Operand::Imm(3)]);
                    if let Some(result) = result_var {
                        self.store_var(result);
                    }
                    self.emit(Op::POP, vec![]);
                    self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                    self.store_var(options);
                    self.emit(Op::POP, vec![]);
                    return;
                }
                CatchOutputOrder::OptionsThenResult => {
                    self.store_var(options);
                    self.emit(Op::POP, vec![]);
                }
                CatchOutputOrder::Unknown => unreachable!("unproved catch output protocol"),
            }
        }
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        if let Some(result) = result_var {
            self.store_var(result);
        }
        self.emit(Op::POP, vec![]);
    }

    /// Resolve the one `try body on error variableList handler` shape this
    /// inline emitter implements.
    ///
    /// The registry owns the typed clause grammar and Tcl's shared list codec
    /// owns `variableList`. This consumer only adds its code-generation proof:
    /// body and handler must be closed braced script values, and bound names
    /// must fit procedure-local scalar bytecodes. Anything else declines to
    /// the generic, yield-aware runtime `try` implementation.
    fn try_on_error_inline_bindings(
        &self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<TryHandlerBindings> {
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let invocation = self.registry.try_control_invocation(
            command,
            &arg_refs,
            self.invocation_surface_query(),
        )?;
        let [clause] = invocation.clauses.as_slice() else {
            return None;
        };
        if !matches!(clause.kind, TryClauseKind::On(selector) if selector.matches_route(
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(CompletionCode::Error)
        ) == Some(true))
            || clause.fallthrough
            || !args[invocation.body_index].1
            || !args[clause.body_index].1
        {
            return None;
        }
        let variable_list_index = clause.variable_list_index?;
        if tcl_syntax::naming::word_is_dynamic(
            &args[variable_list_index].0,
            args[variable_list_index].1,
        ) || (!args[variable_list_index].1 && args[variable_list_index].0.contains('\\'))
        {
            return None;
        }

        let names = tcl_syntax::list::split_list(&args[variable_list_index].0).ok()?;
        let result_var = names.first().map(ToString::to_string);
        let options_var = names.get(1).map(ToString::to_string);
        // Empty receivers have engine-specific output semantics and stay generic.
        if result_var
            .iter()
            .chain(options_var.iter())
            .any(|name| !is_inline_local_scalar_name(name))
        {
            return None;
        }

        Some(TryHandlerBindings {
            body_index: invocation.body_index,
            handler_body_index: clause.body_index,
            result_var,
            options_var,
        })
    }

    /// Emit the selected original Try statement using the same exception and
    /// body-entry owners as a Try nested in a protected command substitution.
    pub(super) fn emit_selected_try_statement(
        &mut self,
        command: &str,
        values: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
    ) -> bool {
        if self.native_entry.is_some() || !self.compiles_locals() {
            return false;
        }
        // CFG Try calls can omit their local token field while retaining the
        // original invocation tokens in emit_stmt's source scope.
        let original = self.invocation_tokens.clone();
        let source_tokens = tokens.or(original.as_deref());
        if source_tokens
            .and_then(|tokens| tokens.expand_word.as_ref())
            .is_some_and(|words| words.iter().any(|expanded| *expanded))
        {
            return false;
        }
        let args = values
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let braced = source_tokens.is_some_and(|tokens| {
                    tokens.argv_kinds.get(index + 1) == Some(&tcl_lexer::TokenType::Str)
                        && tokens
                            .single_token_word
                            .get(index + 1)
                            .copied()
                            .unwrap_or(false)
                });
                (argument.clone(), braced)
            })
            .collect::<Vec<_>>();
        if self.inline_cmd_subst_hook_candidate(command, &args) != Some(InlineCodegenHookId::Try)
            || self.try_on_error_inline_bindings(command, &args).is_none()
        {
            return false;
        }
        // Retain the exact command binding only after the source-form guard.
        if self.inline_cmd_subst_hook(command, &args) != Some(InlineCodegenHookId::Try) {
            return false;
        }
        let exit = self.fresh_label("try_statement_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(exit.clone()), Operand::Imm(1)],
            "",
        );
        self.cmd_index += 1;
        self.emit_try_on_error_inline(command, &args, &exit);
        self.place_label(&exit);
        self.emit(Op::POP, vec![]);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit one complete Tcl body without losing its command boundaries.
    ///
    /// Catch, try, and dictionary bodies share this owner. Each complete
    /// command retains its source binding and uses the existing command emitter.
    /// A malformed tail emits the same parse failure as native bracket scripts
    /// after the complete prefix, inside the surrounding live exception range.
    fn emit_segmented_inline_body(&mut self, body: &str, emitter: InlineBodyEmitter) {
        let config = self.lexer_config().nested();
        let Ok(segmented) = crate::lowering::command_at_time_script_image(
            &tcl_lexer::SourceImage::native(body.as_bytes()),
            config,
        ) else {
            self.refuse_native_dependency();
            self.push_lit_bytes_exact(b"");
            return;
        };
        if segmented.commands.is_empty() && segmented.fatal_tail.is_none() {
            self.push_lit("");
            return;
        }

        let last_complete = segmented.commands.len().saturating_sub(1);
        for (index, command) in segmented.commands.iter().enumerate() {
            let execution_span = command.execution_span(body);
            let command_text = body
                .get(execution_span.start() as usize..execution_span.end() as usize)
                .expect("execution spans index their source body");
            let emitted_start = self.instructions.len();
            let tokens = self.inline_body_command_tokens(command_text, command.span.start());
            let source_span = self.inline_body_source_base.and_then(|base| {
                let span = Span::new(
                    base.checked_add(execution_span.start())?,
                    base.checked_add(execution_span.end())?,
                );
                (self.source.get(span.as_range()) == Some(command_text.as_bytes())).then_some(span)
            });
            self.with_inline_command_source(source_span, command_text, |ctx| {
                ctx.with_invocation_tokens(tokens.as_ref(), |ctx| match emitter {
                    InlineBodyEmitter::Catch => ctx.emit_catch_body(command_text),
                    InlineBodyEmitter::TryHandler => ctx.emit_try_handler_command(command_text),
                });
                ctx.restamp_emitted_inline_command_boundaries(
                    emitted_start,
                    command_text,
                    ctx.span_line(),
                );
            });
            if segmented.fatal_tail.is_some() || index != last_complete {
                self.emit(Op::POP, vec![]);
            }
        }

        if let Some((_, message, _)) = segmented.fatal_tail {
            self.emit_script_parse_error(message.as_bytes());
        }
    }

    /// Preserve the authored coordinate only for the exact literal body.
    fn authored_inline_body_base(&self, body: &str, operand: usize) -> Option<u32> {
        let word = self
            .invocation_tokens
            .as_deref()?
            .words()
            .get(self.original_hook_argument(operand)?.checked_add(1)?)?;
        if !matches!(word, crate::ir::WordExpr::BracedLiteral { .. }) {
            return None;
        }
        crate::command_binding::ExecutedScriptSource::literal_word_base(
            self.source_unicode().ok()?,
            word,
            body,
            self.lexer_config(),
        )
    }

    /// Scope a literal script operand's own source coordinates. In particular,
    /// a try handler cannot inherit the enclosing catch body's first byte.
    fn with_inline_body_operand<T>(
        &mut self,
        body: &str,
        operand: usize,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.inline_body_source_base;
        let previous_context = self.native_compilation;
        self.inline_body_source_base = self.authored_inline_body_base(body, operand);
        let context = self.invocation_tokens.as_deref().and_then(|tokens| {
            crate::registry_invocation::proved_native_inline_body_context(
                self.registry,
                tokens,
                previous_context,
                self.original_hook_argument(operand)?,
            )
        });
        if let Some(context) = context {
            self.native_compilation = context;
        } else if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            crate::registry_invocation::proved_native_admitted_inline_operation(tokens).is_some()
        }) {
            self.refuse_native_dependency();
        }
        let result = emit(self);
        self.inline_body_source_base = previous;
        self.native_compilation = previous_context;
        result
    }

    /// Each body command receives its own retained proof. An authored parent
    /// cannot supply its dispatch proof to a child with a missing source site.
    fn inline_body_command_tokens(
        &self,
        command: &str,
        relative_offset: u32,
    ) -> Option<crate::ir::CommandTokens> {
        let offset = self
            .inline_body_source_base
            .and_then(|base| base.checked_add(relative_offset));
        if let Some(tokens) =
            offset.and_then(|offset| self.source_proofs.as_ref()?.tokens.get(&offset))
        {
            return Some(tokens.clone());
        }
        if let Some(offset) = offset {
            let config = self.lexer_config().nested();
            let segments =
                crate::segmenter::segment_commands_with_offset_and_config(command, offset, config);
            if let [segment] = segments.as_slice()
                && !segment.is_partial
            {
                let source = tcl_lexer::SourceMap::new(command).with_base(offset, 0, 0);
                let mut tokens = crate::ir::CommandTokens::from_segmented(&source, config, segment);
                if let Some(parent) = self.invocation_tokens.as_deref() {
                    tokens.inherit_nested_bindings(parent);
                }
                return Some(tokens);
            }
        }
        let mut tokens = self.original_inline_syntax(command)?;
        if self
            .invocation_tokens
            .as_deref()
            .is_some_and(|parent| parent.source_binding.is_some())
        {
            tokens.source_binding =
                Some(crate::command_binding::SourceInvocationBinding::unknown());
        }
        Some(tokens)
    }

    /// Emit `dict map`'s catch cleanup and rethrow. Returns the placeholder
    /// `unsetScalar` indices for
    /// the iterator and result temps, to be back-patched by the caller.
    fn emit_dict_map_err_epilogue(&mut self, iter_name: &str, result_name: &str) -> (usize, usize) {
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        let unset_it = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            iter_name,
        );
        let unset_res = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            result_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        (unset_it, unset_res)
    }

    /// Close `dict map` after the key/value stack entries have been consumed,
    /// unset the iterator, load the accumulated dict as the result,
    /// and unset the result temp. Returns the placeholder indices (iterator
    /// unset, result load, result unset) for the caller to back-patch.
    fn emit_dict_map_normal_exit_after_values(
        &mut self,
        iter_name: &str,
        result_name: &str,
    ) -> (usize, usize, usize) {
        self.emit(Op::END_CATCH, vec![]);
        let unset_it = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            iter_name,
        );
        let res_load = self.emit_comment(Op::LOAD_SCALAR1, vec![Operand::Imm(0)], result_name);
        let unset_res = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            result_name,
        );
        (unset_it, res_load, unset_res)
    }

    /// Back-patch the placeholder slot operands recorded in `p` with the now-known
    /// `res_slot` (result accumulator) and `iter_slot` (dict iterator).
    fn backpatch_dict_map(&mut self, p: &DictMapPatch, res_slot: i32, iter_slot: i32) {
        self.instructions[p.res_store].operands = vec![Operand::Imm(res_slot)];
        self.instructions[p.dict_first].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[p.dict_set].operands = vec![Operand::Imm(1), Operand::Imm(res_slot)];
        self.instructions[p.dict_next].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[p.unset_it_err].operands = vec![Operand::Imm(0), Operand::Imm(iter_slot)];
        self.instructions[p.unset_res_err].operands = vec![Operand::Imm(0), Operand::Imm(res_slot)];
        self.instructions[p.unset_it_exit].operands =
            vec![Operand::Imm(0), Operand::Imm(iter_slot)];
        self.instructions[p.res_load].operands = vec![Operand::Imm(res_slot)];
        self.instructions[p.unset_res_exit].operands =
            vec![Operand::Imm(0), Operand::Imm(res_slot)];
    }

    /// Emit a compiled `dict for {k v} DICT { body }` inline, matching C Tcl's
    /// low-level bytecode: `beginCatch4` → `dictFirst <iter>` → `jumpTrue`
    /// (skip empty) → store k/v → inline body → `dictNext` → `jumpFalse` (loop)
    /// → `jump` (normal exit) → catch epilogue → `pop pop` and cleanup. Returns `false`
    /// unless this is a procedure context with exactly two local targets and
    /// the original literal body.
    ///
    /// `vars_text` is the `{k v}` word, `dict_text` the dictionary operand, and
    /// `body_text` the braced body. Catch, try, and dictionary commands share the
    /// original command-at-time body compiler. Both exits release the iterator;
    /// break and continue target the same protected loop activation.
    pub fn emit_dict_for(&mut self, vars_text: &str, dict_text: &str, body_text: &str) -> bool {
        // Exactly two loop variables, both plain (compilable-local) names. The
        // `{k v}` word is a Tcl list, so split by the list grammar (not
        // whitespace) — a 1-element list like `{{a b}}` must error at runtime,
        // not be miscompiled as two vars — and bail to the runtime invoke for
        // any name that is qualified, an array element, or needs list quoting.
        let vnames = is_two_plain_names(vars_text);
        let Some(vnames) = vnames else {
            return false;
        };
        if !self.compiles_locals() {
            return false;
        }
        // The original braced operand supplies body coordinates and child
        // compiler receipts. A separately lowered string supplies neither.
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self.authored_inline_body_base(body_text, 3).is_none()
        }) {
            // `TclCompileDictForCmd` compiles any body, its loop variables
            // locals of the procedure all the same.
            self.intern_dict_iteration_locals(&vnames, body_text);
            return false;
        }

        // Slot allocation mirrors C Tcl's `TclCompileDictForCmd`: the two loop
        // variables, then a spare temp, then the body's locals, and finally the
        // iterator temp — so the iterator gets the *highest* slot. Because the
        // body (which interns its own locals) is emitted between `dictFirst` and
        // the iterator's allocation, the `dictFirst`/`dictNext`/`unsetScalar`
        // operands are emitted with a placeholder and back-patched once the
        // iterator slot is known.
        let (k_slot, v_slot) = self.dict_iteration_variable_slots(&vnames);
        // C Tcl allocates an unused companion temp right after the loop vars.
        let _spare = self
            .lvt
            .intern_synthetic(&format!("#dictfor_spare{}", self.catch_depth));
        let iter_name = format!("#dictfor{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_for_loop");
        let end_lbl = self.fresh_label("dict_for_end");
        let body_start = self.fresh_label("dict_for_body");
        let body_end = self.fresh_label("dict_for_body_end");
        let step_label = self.fresh_label("dict_for_step");
        let break_label = self.fresh_label("dict_for_break");

        // Load the dict value, then begin the iterator under a catch range.
        self.emit_value(dict_text, true);
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.mark_completion_option_scope(begin_idx, ControlOptionPolicy::FRESH_SETTLED);
        self.catch_depth += 1;
        let dict_first_idx = self.emit(Op::DICT_FIRST, vec![Operand::Imm(0)]);
        // Tag the loop-control jumps `dict_for` so the layout pass keeps them
        // 4-byte (matching C Tcl, which never narrows a dict-for's `jumpTrue4`/
        // `jumpFalse4`); the normal-exit `jump` below is left untagged so it
        // narrows to `jump1` as C Tcl's does.
        self.emit_comment(
            Op::JUMP_TRUE4,
            vec![Operand::Label(end_lbl.clone())],
            "dict_for",
        );

        // Loop body: bind key/value, run the body, advance.
        self.place_label(&loop_lbl);
        let iteration_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.mark_completion_option_scope(iteration_idx, ControlOptionPolicy::FRESH_SETTLED);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        self.place_label(&body_start);
        self.with_inline_body_operand(body_text, 3, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });
        self.emit(Op::POP, vec![]);
        self.place_label(&body_end);
        self.place_label(&step_label);
        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        let handler = self.fresh_label("dict_for_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        let unset_idx = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            &iter_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        // Now the body's locals are interned; the iterator gets the next
        // (highest) slot. Back-patch the placeholders.
        let iter_slot = bytecode_imm(self.lvt.intern_synthetic(&iter_name));
        self.instructions[dict_first_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[dict_next_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[unset_idx].operands = vec![Operand::Imm(0), Operand::Imm(iter_slot)];

        // Normal exit: drop the leftover key/value from the final dictNext.
        self.place_label(&end_lbl);
        let settle_idx = self.emit(Op::POP, vec![]);
        self.mark_completion_option_scope(settle_idx, ControlOptionPolicy::FRESH_SETTLED);
        self.emit(Op::POP, vec![]);
        self.place_label(&break_label);
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(iter_slot)],
            &iter_name,
        );
        // `dict for` yields the empty string.
        self.push_lit("");
        self.inline_loop_regions.push(super::InlineLoopRegion {
            start: body_start,
            end: body_end,
            continue_target: Some(step_label),
            break_target: break_label,
        });
        self.seen_generic_invoke = true;
        true
    }

    fn intern_dict_iteration_locals(&mut self, names: &[String; 2], body: &str) {
        for name in names {
            self.intern_compiled_local(name);
        }
        self.intern_script_locals(body);
    }

    fn dict_iteration_variable_slots(&mut self, names: &[String; 2]) -> (i32, i32) {
        let key = bytecode_imm(
            self.command_variable_slot(names[0].as_bytes())
                .expect("selected native local target"),
        );
        let value = bytecode_imm(
            self.command_variable_slot(names[1].as_bytes())
                .expect("selected native local target"),
        );
        (key, value)
    }

    fn dict_map_variable_names(&self, vars_text: &str, body_text: &str) -> Option<[String; 2]> {
        let names = is_two_plain_names(vars_text)?;
        if !self.compiles_locals()
            || self.invocation_tokens.as_deref().is_some_and(|tokens| {
                tokens.source_binding.is_some()
                    && self.authored_inline_body_base(body_text, 3).is_none()
            })
        {
            return None;
        }
        Some(names)
    }

    /// Emit a compiled `dict map {k v} DICT { body }` inline, matching C Tcl:
    /// like `dict for` but accumulating each iteration's body result into a new
    /// dictionary (`dictSet result[k] = body-result`) which becomes the value.
    /// Requires a procedure-local target pair and the original literal body;
    /// body commands share the catch/try command-at-time compiler.
    pub fn emit_dict_map(&mut self, vars_text: &str, dict_text: &str, body_text: &str) -> bool {
        let Some(vnames) = self.dict_map_variable_names(vars_text, body_text) else {
            // `TclCompileDictMapCmd` compiles any body, its loop variables
            // locals of the procedure all the same.
            if self.compiles_locals()
                && let Some(names) = is_two_plain_names(vars_text)
            {
                self.intern_dict_iteration_locals(&names, body_text);
            }
            return false;
        };

        // Slots: the result accumulator and iterator temps are allocated after
        // the loop vars and the body's locals (interned during body emission),
        // so both are back-patched once known.
        let (k_slot, v_slot) = self.dict_iteration_variable_slots(&vnames);
        let result_name = format!("#dictmap_res{}", self.catch_depth);
        let iter_name = format!("#dictmap_it{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_map_loop");
        let end_lbl = self.fresh_label("dict_map_end");
        let body_start = self.fresh_label("dict_map_body");
        let body_end = self.fresh_label("dict_map_body_end");
        let step_label = self.fresh_label("dict_map_step");
        let break_label = self.fresh_label("dict_map_break");

        // Initialise the accumulator to the empty dict.
        self.push_lit("");
        let res_store_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(0)], &result_name);
        self.emit(Op::POP, vec![]);

        self.emit_value(dict_text, true);
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.mark_completion_option_scope(begin_idx, ControlOptionPolicy::FRESH_FORWARDED);
        self.catch_depth += 1;
        let dict_first_idx = self.emit(Op::DICT_FIRST, vec![Operand::Imm(0)]);
        self.emit_comment(
            Op::JUMP_TRUE4,
            vec![Operand::Label(end_lbl.clone())],
            "dict_for",
        );

        self.place_label(&loop_lbl);
        let iteration_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.mark_completion_option_scope(iteration_idx, ControlOptionPolicy::FRESH_FORWARDED);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        self.place_label(&body_start);
        self.with_inline_body_operand(body_text, 3, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });
        self.place_label(&body_end);
        // Accumulate: result[key] = mapped value.
        self.emit_comment(Op::LOAD_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.emit(Op::OVER, vec![Operand::Imm(1)]);
        let dict_set_idx = self.emit_comment(
            Op::DICT_SET,
            vec![Operand::Imm(1), Operand::Imm(0)],
            &result_name,
        );
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);

        self.place_label(&step_label);
        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        let handler = self.fresh_label("dict_map_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        let (unset_it_err_idx, unset_res_err_idx) =
            self.emit_dict_map_err_epilogue(&iter_name, &result_name);
        self.catch_depth -= 1;

        // Normal exit: drop the leftover key/value, close the catch, unset the
        // iterator, load the accumulated dict as the result, unset the temp.
        self.place_label(&end_lbl);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        self.place_label(&break_label);
        let (unset_it_exit_idx, res_load_idx, unset_res_exit_idx) =
            self.emit_dict_map_normal_exit_after_values(&iter_name, &result_name);

        // Now intern the result + iterator temps (highest slots) and back-patch.
        let res_slot = bytecode_imm(self.lvt.intern_synthetic(&result_name));
        let iter_slot = bytecode_imm(self.lvt.intern_synthetic(&iter_name));
        self.backpatch_dict_map(
            &DictMapPatch {
                res_store: res_store_idx,
                dict_first: dict_first_idx,
                dict_set: dict_set_idx,
                dict_next: dict_next_idx,
                unset_it_err: unset_it_err_idx,
                unset_res_err: unset_res_err_idx,
                unset_it_exit: unset_it_exit_idx,
                res_load: res_load_idx,
                unset_res_exit: unset_res_exit_idx,
            },
            res_slot,
            iter_slot,
        );
        self.inline_loop_regions.push(super::InlineLoopRegion {
            start: body_start,
            end: body_end,
            continue_target: Some(step_label),
            break_target: break_label,
        });

        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict update DICTVAR key1 var1 ?key2 var2 …? { body }`
    /// Bind keyed values to actual local slots, compile the original body, and
    /// write the locals back on normal and exceptional exits. `rest` is
    /// `[dictvar, k1, v1, …, body]`; every target must be a procedure-local scalar.
    pub fn emit_dict_update(&mut self, rest: &[String]) -> bool {
        if !self.compiles_locals() || rest.len() < 4 || !rest.len().is_multiple_of(2) {
            return false;
        }
        let dict_var = &rest[0];
        if is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        let body_text = rest.last().expect("rest is non-empty");
        let mid = &rest[1..rest.len() - 1]; // (key, targetvar) pairs, even length
        let keys: Vec<String> = mid.iter().step_by(2).cloned().collect();
        let vars: Vec<String> = mid.iter().skip(1).step_by(2).cloned().collect();
        if vars.iter().any(|v| is_qualified(v) || v.contains('(')) {
            return false;
        }
        let body_operand = rest.len();
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self
                    .authored_inline_body_base(body_text, body_operand)
                    .is_none()
        }) {
            // `TclCompileDictUpdateCmd` compiles any body with the command,
            // and its variables are locals of the procedure all the same.
            self.intern_compiled_local(dict_var);
            for v in &vars {
                self.intern_compiled_local(v);
            }
            self.intern_script_locals(body_text);
            return false;
        }

        let dict_slot = bytecode_imm(
            self.command_variable_slot(dict_var.as_bytes())
                .expect("selected native dictionary local"),
        );
        let variable_slots = vars
            .iter()
            .map(|name| {
                self.command_variable_slot(name.as_bytes())
                    .expect("selected native dictionary target")
            })
            .collect::<Vec<_>>();
        let end_lbl = self.fresh_label("dict_update_end");

        // Push the key list.
        for k in &keys {
            self.emit_value_interpolated(k);
        }
        self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(keys.len()))]);

        // Prologue: read the dict, bind each keyed value to its target local.
        let start_idx = self.emit_comment(
            Op::DICT_UPDATE_START,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[start_idx].dict_vars = Some(variable_slots.clone());

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        self.with_inline_body_operand(body_text, body_operand, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });

        // Normal epilogue: swap the key list above the body result and write the
        // locals back into the dict.
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        let end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[end_idx].dict_vars = Some(variable_slots.clone());
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Every non-OK body completion still publishes the target locals before
        // propagating its original result and return options.
        let handler = self.fresh_label("dict_body_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(3)]);
        let err_end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[err_end_idx].dict_vars = Some(variable_slots);
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict with DICTVAR { body }` inline, matching C Tcl:
    /// Expand keys into same-named locals, compile the original body, and fold
    /// the locals back on normal and exceptional exits. The path form and
    /// nonlocal dictionary targets use the runtime command.
    pub fn emit_dict_with(&mut self, dict_var: &str, body_text: &str) -> bool {
        if !self.compiles_locals() || is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self.authored_inline_body_base(body_text, 2).is_none()
        }) {
            // As `emit_dict_update`: `TclCompileDictWithCmd` compiles any body.
            self.intern_compiled_local(dict_var);
            self.intern_script_locals(body_text);
            return false;
        }

        let dict_slot = bytecode_imm(
            self.command_variable_slot(dict_var.as_bytes())
                .expect("selected native dictionary local"),
        );
        let state_name = format!("#dictwith_state{}", self.catch_depth);
        let state_slot = bytecode_imm(self.lvt.intern_synthetic(&state_name));
        let end_lbl = self.fresh_label("dict_with_end");

        // Prologue: expand the dict into per-key locals; stash the recombine
        // state in a temp.
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.push_lit("");
        self.emit(Op::DICT_EXPAND, vec![]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit(Op::POP, vec![]);

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        self.with_inline_body_operand(body_text, 2, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });

        // Normal epilogue: recombine the per-key locals into the dict.
        self.emit(Op::END_CATCH, vec![]);
        self.push_lit("");
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(
            Op::DICT_RECOMBINE_IMM,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Every non-OK body completion still publishes the target locals before
        // propagating its original result and return options.
        let handler = self.fresh_label("dict_body_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        self.push_lit("");
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(
            Op::DICT_RECOMBINE_IMM,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit inline `beginCatch4`/`endCatch` bytecodes for `catch`.
    ///
    /// Compiles each complete body command inline, then emits the normal/handler
    /// paths and stores result/options variables.
    /// The catch return code is left on the stack. Returns false without
    /// emitting instructions when the output protocol has not been proved.
    pub fn emit_catch_inline(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> bool {
        if self.catch_output_order(result_var, options_var)
            == tcl_registry::catch_invocation::CatchOutputOrder::Unknown
        {
            return false;
        }
        let previous_context = self.native_compilation;
        self.native_compilation = previous_context.with_inline_exception_range();
        self.with_inline_body_operand(body_text, 0, |ctx| {
            ctx.emit_catch_inline_inner(body_text, result_var, options_var);
        });
        self.native_compilation = previous_context;
        true
    }

    fn emit_catch_inline_inner(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        // `parse_cmd_parts` has already resolved the outer argument word. The
        // value may legitimately begin and end with braced command words
        // (`catch {{set} x {1}}`), so it must not be stripped a second time.
        let body = body_text;
        let output_order = self.catch_output_order(result_var, options_var);

        // Pre-intern result_var so it gets a lower LVT slot
        if let Some(name) = result_var {
            self.command_variable_slot(name.as_bytes());
        }

        // beginCatch4 with current nesting depth. The handler label rides
        // out-of-band on the instruction (`Instruction::catch_target`, the
        // analogue of C's `ExceptionRange.catchOffset`) so the VM opens a
        // *live* range; the operand keeps C's range-index meaning and the
        // disassembly its shape. Back-patched once the handler is placed.
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Keep every complete command in this activation so a yield freezes the
        // live catch range itself. The shared segmenter owns command boundaries;
        // `emit_catch_body` remains the one-command emitter. Discard each
        // non-final result exactly as ordinary script execution does. For a
        // fatal or incomplete tail the compiler-owned command plan keeps the
        // complete prefix executable, then raises the catchable parse error if
        // that prefix finishes normally; substitutions in the bad command do
        // not run.
        self.emit_segmented_inline_body(body, InlineBodyEmitter::Catch);

        // Normal completion: push code "0".
        self.push_lit("0");

        let handler_label = self.fresh_label("catch_handler");
        if options_var.is_some() {
            // 3-arg catch: normal path jumps to shared pushReturnOpts.
            let opts_label = self.fresh_label("catch_opts");
            self.emit(Op::JUMP1, vec![Operand::Label(opts_label.clone())]);

            // Handler entry: push caught result and return code.
            self.place_label(&handler_label);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            // pushReturnOpts shared by both paths.
            self.place_label(&opts_label);
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        } else {
            // 2-arg catch: jump over handler to endCatch.
            let end_label = self.fresh_label("catch_end");
            self.emit(Op::JUMP1, vec![Operand::Label(end_label.clone())]);

            // Handler entry: push caught result and return code.
            self.place_label(&handler_label);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            self.place_label(&end_label);
        }
        self.instructions[begin_idx].catch_target = Some(handler_label);

        // endCatch.
        self.catch_depth -= 1;
        self.emit(Op::END_CATCH, vec![]);

        // Stack: [result, code], or [result, code, options].
        self.emit_catch_output_stores(result_var, options_var, output_order);
        // Return code stays on the stack as value of catch.
    }

    /// Compile one catch command using selected wrapping and emitter metadata.
    pub fn emit_catch_body(&mut self, body: &str) {
        // The resolved script keeps its own configured argument-expansion phase.
        if self.recognises_expand_syntax() && body.contains("{*}") {
            let Some(expanded) = parse_script_parts_expand_with_config(body, self.lexer_config())
            else {
                self.emit_script_eval(body);
                self.seen_generic_invoke = true;
                return;
            };
            if expanded.iter().any(|(_, _, expand)| *expand) {
                self.emit_expanded_cmd_subst(&expanded);
                self.seen_generic_invoke = true;
                return;
            }
        }
        let Some(body_parts) = parse_script_parts_with_config(body, self.lexer_config()) else {
            self.emit_script_eval(body);
            self.seen_generic_invoke = true;
            return;
        };
        if body_parts.is_empty() {
            self.push_lit("");
            return;
        }

        let body_cmd = &body_parts[0].0;
        let body_args = &body_parts[1..];

        let sc_label = self.start_catch_body_command(body_cmd, body_args);

        if super::emitter::bytecoded::try_value_bytecoded(self, body_cmd, body_args) {
            if let Some(label) = sc_label {
                self.place_label(&label);
            }
            return;
        }

        // Hooks this catch-body dispatcher does not specialise
        // (`Incr`, and `String`, `Lrange`, `Linsert` and `Lreplace` but for a
        // literal index read apart, …, which only the value-position
        // dispatcher in `cmd_subst` emits inline) fall to the generic invoke
        // arm, as do guard failures.
        match self.inline_cmd_subst_hook(body_cmd, body_args) {
            Some(InlineCodegenHookId::Yield) if body_args.len() <= 1 => {
                self.emit_inline_yield(body_args);
            }
            Some(InlineCodegenHookId::YieldTo) => {
                self.emit_inline_yield_to(
                    body_args
                        .iter()
                        .map(|(word, braced)| (word.as_str(), *braced, false)),
                );
            }
            Some(
                InlineCodegenHookId::String
                | InlineCodegenHookId::Lrange
                | InlineCodegenHookId::Linsert
                | InlineCodegenHookId::Lreplace,
            ) if self.try_emit_compiled_index_command(body_cmd, body_args) => {}
            Some(InlineCodegenHookId::Return) => self.emit_catch_return(body_args),
            Some(InlineCodegenHookId::Error) => self.emit_catch_error(body_cmd, body, body_args),
            Some(InlineCodegenHookId::Break) => {
                self.emit(Op::BREAK, vec![]);
            }
            Some(InlineCodegenHookId::Continue) => {
                self.emit(Op::CONTINUE, vec![]);
            }
            Some(InlineCodegenHookId::Expr)
                if super::cmd_subst::lone_spelled_expression(body_args) =>
            {
                self.emit_catch_expression(&body_args[0].0);
            }
            Some(InlineCodegenHookId::Try) if self.compiles_locals() => {
                if self
                    .try_on_error_inline_bindings(body_cmd, body_args)
                    .is_some()
                {
                    let try_sc = self.fresh_label("catch_body_end");
                    self.emit_comment(
                        Op::START_CMD,
                        vec![Operand::Label(try_sc.clone()), Operand::Imm(1)],
                        "",
                    );
                    self.cmd_index += 1;
                    self.emit_try_on_error_inline(body_cmd, body_args, &try_sc);
                    self.place_label(&try_sc);
                    self.seen_generic_invoke = true;
                } else {
                    self.emit_generic_cmd_subst(body_cmd, body_args);
                    self.seen_generic_invoke = true;
                }
            }
            _ => {
                // One shared fixed-arity generic emitter owns word
                // substitution and specialised entered-command metadata.
                self.emit_generic_cmd_subst(body_cmd, body_args);
                self.seen_generic_invoke = true;
            }
        }

        if let Some(label) = sc_label {
            self.place_label(&label);
        }
    }

    fn start_catch_body_command(
        &mut self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<String> {
        let needs_start_cmd = self
            .selected_invocation_traits(command, args)
            .is_some_and(|traits| traits.contains(Traits::NEEDS_START_CMD));

        if needs_start_cmd {
            let label = self.fresh_label("catch_body_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(1)],
                "",
            );
            self.cmd_index += 1;
            Some(label)
        } else {
            None
        }
    }

    fn emit_catch_expression(&mut self, source: &str) {
        // Parsed under the compile's dialect, as lowering parses a
        // statement-position `expr`.
        let node = self.parse_compile_expr(source);
        if let Some((message, options)) = detect_const_expr_error(&node) {
            self.push_lit(&message);
            self.push_lit(&options);
            self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
        } else {
            self.emit_expr(&node);
        }
    }

    /// Compile `return ?-code C? ?-level L? ?value?` inside a catch body.
    pub fn emit_catch_return(&mut self, args: &[(String, bool)]) {
        let code_names: &[(&str, i32)] = &[
            ("ok", 0),
            ("error", 1),
            ("return", 2),
            ("break", 3),
            ("continue", 4),
        ];
        let mut i = 0;
        let mut code: Option<i32> = None;
        let mut level: Option<i32> = None;

        while i < args.len() {
            let flag = &args[i].0;
            if flag == "-code" && i + 1 < args.len() {
                let val = &args[i + 1].0;
                code = code_names
                    .iter()
                    .find(|(n, _)| *n == val)
                    .map(|(_, c)| *c)
                    .or_else(|| {
                        self.parse_int_operand(val)
                            .and_then(|v| i32::try_from(v).ok())
                    });
                if code.is_none() {
                    break;
                }
                i += 2;
            } else if flag == "-level" && i + 1 < args.len() {
                level = self
                    .parse_int_operand(&args[i + 1].0)
                    .and_then(|v| i32::try_from(v).ok());
                if level.is_none() {
                    break;
                }
                i += 2;
            } else if flag == "--" {
                i += 1;
                break;
            } else {
                // Non-flag or unknown flag: stop option parsing
                break;
            }
        }

        let remaining = &args[i..];
        let value = if remaining.is_empty() {
            ""
        } else {
            &remaining[0].0
        };

        let (ret_code, ret_level) = merge_return_operands(code, level);

        self.emit_value(value, true);
        self.push_lit("");
        self.emit(
            Op::RETURN_IMM,
            vec![Operand::Imm(ret_code), Operand::Imm(ret_level)],
        );
    }

    /// Compile `error msg ?info? ?code?` inside a catch body.
    pub fn emit_catch_error(
        &mut self,
        command: &str,
        source_command: &str,
        args: &[(String, bool)],
    ) {
        if !self.emit_inline_error(args, source_command) {
            self.emit_generic_cmd_subst(command, args);
            self.seen_generic_invoke = true;
        }
    }

    // Inline try/on error compilation.

    /// Emit the `-during` merge sequence: load saved opts, prepend
    /// the new error opts, dict-set `-during` key, store back to
    /// temps slot.  Extracted from
    /// [`Self::emit_try_on_error_inline`].
    fn emit_try_during_merge(&mut self, temp_opts_slot: i32) {
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.push_lit("-during");
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::DICT_SET,
            vec![Operand::Imm(1), Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
    }

    /// Emit the no-match (code != 1) re-raise path: pop the
    /// dispatch flag, reload saved opts + result, and `RETURN_STK`.
    fn emit_try_no_match_reraise(&mut self, temp_opts_slot: i32, temp_result_slot: i32) {
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::RETURN_STK, vec![]);
    }

    /// Begin one inline exception range and advance the shared range index.
    fn begin_inline_catch_range(&mut self) -> usize {
        let begin = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        begin
    }

    /// Bind the caught error result/options to the handler's declared local
    /// variables. The typed try-clause parser owns which names are present;
    /// this helper owns their bytecode transfer from the shared temp slots.
    fn emit_try_error_bindings(
        &mut self,
        bindings: &TryHandlerBindings,
        result_slot: Option<i32>,
        options_slot: Option<i32>,
        temp_result_slot: i32,
        temp_opts_slot: i32,
    ) {
        if let (Some(name), Some(slot)) = (&bindings.result_var, result_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_result_slot)],
                &format!("temp var {temp_result_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
        if let (Some(name), Some(slot)) = (&bindings.options_var, options_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_opts_slot)],
                &format!("temp var {temp_opts_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
    }

    /// Emit inline `try { body } on error {var} { handler }` bytecodes.
    pub fn emit_try_on_error_inline(
        &mut self,
        command: &str,
        args: &[(String, bool)],
        normal_exit: &str,
    ) {
        let bindings = self
            .try_on_error_inline_bindings(command, args)
            .expect("caller proved the typed literal try/on-error shape");
        let try_body_text = &args[bindings.body_index].0;
        let handler_body_text = &args[bindings.handler_body_index].0;

        // Allocate LVT slots in tclsh order
        let msg_slot = bindings.result_var.as_ref().and_then(|name| {
            self.command_variable_slot(name.as_bytes())
                .map(bytecode_imm)
        });
        let handler_opts_slot = bindings.options_var.as_ref().and_then(|name| {
            self.command_variable_slot(name.as_bytes())
                .map(bytecode_imm)
        });
        let temp_result_name = format!("#temp{}", self.catch_depth);
        let temp_opts_name = format!("#temp{}", self.catch_depth + 1);
        let temp_result_slot = bytecode_imm(self.lvt.intern_synthetic(&temp_result_name));
        let temp_opts_slot = bytecode_imm(self.lvt.intern_synthetic(&temp_opts_name));

        let initial_depth = self.catch_depth;

        // Try body exception range
        let try_begin_idx = self.begin_inline_catch_range();

        // Compile the complete try body in this activation so a yield freezes
        // this phase's live exception range.
        self.with_inline_body_operand(try_body_text, bindings.body_index, |ctx| {
            ctx.emit_segmented_inline_body(try_body_text, InlineBodyEmitter::Catch);
        });

        // Normal exit from try body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for try body. The out-of-band target is the VM's
        // analogue of C Tcl's exception-range catch offset.
        let try_exception = self.fresh_label("try_on_body_exception");
        self.place_label(&try_exception);
        self.instructions[try_begin_idx].catch_target = Some(try_exception);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Store return opts and result to temps
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::POP, vec![]);

        // Return code dispatch: check if code == 1 (TCL_ERROR)
        self.emit(Op::DUP, vec![]);
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let no_match = self.fresh_label("try_on_nomatch");
        self.emit_comment(
            Op::JUMP_FALSE4,
            vec![Operand::Label(no_match.clone())],
            "try_on",
        );

        // Matched error handler (code == 1)
        self.emit(Op::POP, vec![]);
        self.emit_try_error_bindings(
            &bindings,
            msg_slot,
            handler_opts_slot,
            temp_result_slot,
            temp_opts_slot,
        );

        // Handler body exception range
        let handler_begin_idx = self.begin_inline_catch_range();

        // Compile handler body
        self.with_inline_body_operand(handler_body_text, bindings.handler_body_index, |ctx| {
            ctx.emit_try_handler_body(handler_body_text);
        });

        // Normal exit from handler body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for handler body.
        let handler_exception = self.fresh_label("try_on_handler_exception");
        self.place_label(&handler_exception);
        self.instructions[handler_begin_idx].catch_target = Some(handler_exception);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check if handler exception code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_on_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during merge
        self.emit_try_during_merge(temp_opts_slot);

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit(Op::RETURN_STK, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // No match (code != 1): re-raise
        self.place_label(&no_match);
        self.emit_try_no_match_reraise(temp_opts_slot, temp_result_slot);

        // Restore catch depth
        self.catch_depth = initial_depth;
    }

    /// Compile a complete try/on handler body inline with `startCommand` at
    /// each command boundary.
    pub fn emit_try_handler_body(&mut self, body_text: &str) {
        self.emit_segmented_inline_body(body_text, InlineBodyEmitter::TryHandler);
    }

    /// Compile one command of a try/on handler body.
    fn emit_try_handler_command(&mut self, body_text: &str) {
        let Some(parts) = parse_script_parts_with_config(body_text, self.lexer_config()) else {
            self.emit_script_eval(body_text);
            self.seen_generic_invoke = true;
            return;
        };
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        let cmd = &parts[0].0;
        let cmd_args = &parts[1..];

        let sc_label = self.fresh_label("handler_body_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(sc_label.clone()), Operand::Imm(1)],
            "",
        );
        self.cmd_index += 1;

        if super::emitter::bytecoded::try_value_bytecoded(self, cmd, cmd_args) {
            self.place_label(&sc_label);
            return;
        }

        let arg_refs: Vec<&str> = cmd_args.iter().map(|(arg, _)| arg.as_str()).collect();
        let inline_lowering = self.inline_lowering_hook(cmd, &arg_refs);
        match inline_lowering {
            // The registry's typed lowering hook proves that this is the
            // two-argument `set` shape, but its first word is still a Tcl
            // variable-name word. The specialised STORE_SCALAR1 path cannot
            // perform Tcl's dynamic name substitution (e.g. `set $v value`),
            // so decline it unless the shared scalar-name shape proves the
            // target is a procedure-local name.
            Some((LoweringHookId::Set, binding))
                if cmd_args.len() == 2
                    && !tcl_syntax::naming::word_is_dynamic(&cmd_args[0].0, cmd_args[0].1)
                    && is_inline_local_scalar_name(&cmd_args[0].0) =>
            {
                self.require_command_binding(&binding);
                self.emit_native_argument_word(1, &cmd_args[1].0, cmd_args[1].1);
                self.store_var(&cmd_args[0].0);
            }
            _ => {
                self.emit_generic_cmd_subst(cmd, cmd_args);
            }
        }

        self.place_label(&sc_label);
    }

    // Inline catch compilation.

    /// Open C's `catch` exception range.
    ///
    /// Pre-interns the result and options variables so the slots land where C
    /// puts them: `catch {set q $x} m o` gives m=1, o=2 and only then q=3.
    ///
    /// Returns the `beginCatch4` index and its handler label, for
    /// [`Self::emit_catch_region_epilogue`] to place and back-patch.
    pub fn emit_catch_region_prologue(
        &mut self,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> (usize, String) {
        for name in [result_var, options_var].into_iter().flatten() {
            self.command_variable_slot(name.as_bytes());
        }

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        (begin_idx, self.fresh_label("catch_handler"))
    }

    /// Close the range opened by [`Self::emit_catch_region_prologue`] and
    /// store `catch`'s results.
    ///
    /// Measured against `tcl::unsupported::disassemble` on 9.0.4. For
    /// `proc p {x} { catch {set q $x} m o }` C emits, after the body:
    ///
    /// ```text
    /// push1 0             # "0"      <- normal completion pushes code 0
    /// jump1 +4
    /// pushResult                     <- handler pushes result and code
    /// pushReturnCode
    /// pushReturnOpts                 <- 3-arg only, reached by both paths
    /// endCatch
    /// storeScalar1 %v2    # var "o"  <- options come off the top first
    /// pop
    /// reverse 2
    /// storeScalar1 %v1    # var "m"
    /// pop
    /// ```
    ///
    /// Both paths arrive the same depth: normal completion leaves the body's
    /// own result below the pushed code, the handler pushes result and code
    /// itself.
    pub fn emit_catch_region_epilogue(
        &mut self,
        begin_idx: usize,
        handler_label: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        self.push_lit("0");
        let converge = self.fresh_label("catch_converge");
        self.emit(Op::JUMP1, vec![Operand::Label(converge.clone())]);

        self.place_label(handler_label);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);

        self.place_label(&converge);
        if options_var.is_some() {
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        }
        self.instructions[begin_idx].catch_target = Some(handler_label.to_owned());

        self.catch_depth -= 1;
        self.emit(Op::END_CATCH, vec![]);

        let output_order = self.catch_output_order(result_var, options_var);
        self.emit_catch_output_stores(result_var, options_var, output_order);

        // `catch` in statement position: its return code is the statement's
        // value, and every statement's value is popped. In final position
        // `remove_trailing_pop` takes this one back off again, which is how C
        // ends `proc p {x} {catch {…}}` with the code still on the stack.
        self.emit(Op::POP, vec![]);
    }

    /// Emit a whole `catch` region: open the range, compile the body, close it.
    ///
    /// The body is emitted here rather than by the ordinary block walk because
    /// that walk pops every statement's value, and `catch` needs the body's
    /// last result left on the stack for its result variable. The CFG builder
    /// only inlines a body that lowers to this one straight-line block
    /// (`lower_catch_dispatch`), so there is never a later block to miss.
    pub fn emit_catch_region_inline(
        &mut self,
        cfg: &CfgFunction,
        catch_body_name: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        let body_blk = cfg
            .block_by_name(catch_body_name)
            .expect("catch body block present");

        let (begin_idx, handler_label) = self.emit_catch_region_prologue(result_var, options_var);
        // The captured wrapper is a source command before the body's first
        // command. Its children use the ordinary command-boundary owner.
        self.cmd_index += 1;
        // Only the *last* statement's value is `catch`'s result; every earlier
        // one is discarded exactly as ordinary script execution discards it.
        // `emit_try_body_stmt` strips the trailing pop to keep a value on the
        // stack, so it is right for the final statement and wrong for the
        // others — without this `catch {set x 1; set y 2}` leaves `1` under
        // the result, and a loop around it grows the operand stack without
        // bound. C pops after each non-final command.
        let last = body_blk
            .statements
            .iter()
            .rposition(Statement::is_executable_invocation);
        for (index, stmt) in body_blk.statements.iter().enumerate() {
            if !stmt.is_executable_invocation() {
                continue;
            }
            if Some(index) == last {
                self.emit_try_body_stmt(stmt);
            } else {
                self.emit_stmt_with_start_cmd(stmt, None, None);
            }
        }
        if last.is_none() {
            self.push_lit("");
        }
        self.emit_catch_region_epilogue(begin_idx, &handler_label, result_var, options_var);
    }

    // Inline try/finally compilation.

    /// Emit inline `try { body } finally { cleanup }` bytecodes.
    pub fn emit_try_finally_inline(
        &mut self,
        cfg: &CfgFunction,
        try_body_name: &str,
        try_finally_name: &str,
    ) {
        let body_blk = cfg
            .block_by_name(try_body_name)
            .expect("try body block present");
        let finally_blk = cfg
            .block_by_name(try_finally_name)
            .expect("try finally block present");

        // try body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &body_blk.statements {
            self.emit_try_body_stmt(stmt);
        }

        // Transition: jump over pushResult, converge at pushReturnOpts + endCatch
        let conv = self.fresh_label("try_conv");
        self.emit(Op::JUMP1, vec![Operand::Label(conv.clone())]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.place_label(&conv);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // finally body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &finally_blk.statements {
            self.emit_try_finally_stmt(stmt);
        }

        // Normal exit from finally
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::POP, vec![]);
        let normal_exit = self.fresh_label("try_normal");
        self.emit(Op::JUMP1, vec![Operand::Label(normal_exit.clone())]);

        // Exception handler for the finally body
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check return code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during option merging
        self.push_lit("-during");
        self.emit(Op::OVER, vec![Operand::Imm(3)]);
        self.emit(Op::LIST, vec![Operand::Imm(2)]);
        self.emit(Op::LIST_CONCAT, vec![]);

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(4)]);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        let return_target = self.fresh_label("try_return");
        self.emit(Op::JUMP1, vec![Operand::Label(return_target.clone())]);

        // Normal exit: swap result and return opts
        self.place_label(&normal_exit);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);

        // Return / re-raise
        self.place_label(&return_target);
        self.emit(Op::RETURN_STK, vec![]);

        // Restore catch depth
        self.catch_depth -= 2;
    }

    /// Emit a statement using the same selected native hook and original-word
    /// protocol inside the try body's protected compiler context.
    pub fn emit_try_body_stmt(&mut self, stmt: &Statement) {
        self.emit_stmt_with_start_cmd(stmt, None, None);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
    }

    /// Emit a statement in finally-body context.
    pub fn emit_try_finally_stmt(&mut self, stmt: &Statement) {
        self.emit_stmt_with_start_cmd(stmt, None, None);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::Block;
    use tcl_registry::CommandRegistry;

    #[test]
    fn original_inline_body_tokens_keep_nested_bom_and_parent_point_correspondence() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Original source geometry only; this control supplies no Native compilation admission.
        let mut prepared = crate::analyser::Analyser::new();
        let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
        let registry = input.borrowed_context_registry().commands();
        let source = "catch {\u{feff}unknown argument}";
        let bindings =
            crate::realm::document_realm_bindings_with_resolved_input(source, &input, None);
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let mut parent = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            input.lexer_config(),
            &command,
        );
        bindings
            .source_bindings_ref()
            .stamp_original_tokens(&mut parent);
        let body = "\u{feff}unknown argument";
        let mut context = CodegenCtx::new(true, &[], registry);
        context.ingress_lexer_config = Some(input.lexer_config());
        context.set_source_image(tcl_lexer::SourceImage::document(source));
        context.inline_body_source_base = Some(7);
        context.with_invocation_tokens(Some(&parent), |context| {
            let child = context.inline_body_command_tokens(body, 0).unwrap();
            assert_eq!(child.argv_texts, ["\u{feff}unknown", "argument"]);
            assert_eq!(child.argv[0].start(), 7);
            assert_eq!(
                source.get(child.argv[0].as_range()),
                Some("\u{feff}unknown")
            );
            let site = child
                .source_binding
                .as_ref()
                .unwrap()
                .invocation_site()
                .unwrap();
            assert_eq!(site.offset, 7);
            assert_eq!(site.source.source_image().bytes(), source.as_bytes());
            assert_eq!(context.ingress_lexer_config, Some(input.lexer_config()));
        });
        // A real foreign parent cannot lend its child point to equal-position
        // text from this body's source instance.
        let other = "catch {foreign argument}";
        let bindings =
            crate::realm::document_realm_bindings_with_resolved_input(other, &input, None);
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            other,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let mut foreign = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(other),
            input.lexer_config(),
            &command,
        );
        bindings
            .source_bindings_ref()
            .stamp_original_tokens(&mut foreign);
        context.with_invocation_tokens(Some(&foreign), |context| {
            let child = context.inline_body_command_tokens(body, 0).unwrap();
            assert!(
                child
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.original_lexer_config_for_tokens(&child))
                    .is_none()
            );
        });
    }

    #[test]
    fn original_try_handler_argument_emission_keeps_the_child_source_scope() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let registry = CommandRegistry::build_default();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let source =
            "try {set parent READY} on error {message options} {\nset result \"A\\u0000B\"\n}";
        let body = r#"set result "A\u0000B""#;
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let map = tcl_lexer::SourceMap::from_image(&image);
        let parent_segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let parent = crate::ir::CommandTokens::from_segmented(&map, config, &parent_segments[0]);
        let offset = u32::try_from(source.find(body).unwrap()).unwrap();
        let child_segments =
            crate::segmenter::segment_commands_with_offset_and_config(body, offset, config);
        assert_eq!(child_segments.len(), 1);
        let child = crate::ir::CommandTokens::from_segmented(&map, config, &child_segments[0]);
        let mut context = CodegenCtx::new(true, &[], &registry);
        context.invocation_dialect = Some(dialect);
        context.source_string_protocol = dialect.native_source_string_protocol();
        context.ingress_lexer_config = Some(config);
        context.set_source_image(image.clone());
        context.with_invocation_tokens(Some(&parent), |context| {
            context.with_invocation_tokens(Some(&child), |context| {
                context.emit_try_handler_command(body);
            });
            assert_eq!(
                context.invocation_tokens.as_deref().unwrap().words(),
                parent.words()
            );
        });
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"A\xc0\x80B")
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::STORE_SCALAR1 | Op::STORE_SCALAR4))
        );
        assert!(context.invocation_tokens.is_none());

        // An enclosing source/body receipt cannot substitute for the child command.
        let mut missing = CodegenCtx::new(true, &[], &registry);
        missing.invocation_dialect = Some(dialect);
        missing.source_string_protocol = dialect.native_source_string_protocol();
        missing.ingress_lexer_config = Some(config);
        missing.set_source_image(image);
        missing.with_invocation_tokens(Some(&parent), |context| {
            context.with_invocation_tokens(None, |context| context.emit_try_handler_command(body));
        });
        assert!(
            missing
                .literals
                .entries()
                .iter()
                .all(|literal| literal.bytes() != b"A\xc0\x80B")
        );
        assert!(missing.invocation_tokens.is_none());
    }

    #[test]
    fn inline_body_instructions_keep_the_inner_command_source_and_restore_the_parent() {
        let registry = CommandRegistry::build_default();
        let source = "catch {\nset value READY\nerror BODY\n}";
        let body = "\nset value READY\nerror BODY\n";
        for emitter in [InlineBodyEmitter::Catch, InlineBodyEmitter::TryHandler] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.set_source(source);
            let outer_span = Span::new(0, u32::try_from(source.len()).unwrap());
            ctx.set_command_source_span(outer_span);
            ctx.inline_body_source_base = Some(7);
            ctx.emit_segmented_inline_body(body, emitter);
            let failures = ctx
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction.op, Op::RETURN_IMM | Op::INVOKE_STK1)
                        && instruction.source_cmd_text.bytes() == b"error BODY"
                })
                .collect::<Vec<_>>();
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].source_span, Some(Span::new(24, 34)));
            assert_eq!(failures[0].source_line, 3);
            assert!(ctx.instructions.iter().any(|instruction| {
                instruction.source_cmd_text.bytes() == b"set value READY"
                    && instruction.source_span == Some(Span::new(8, 23))
                    && instruction.source_line == 2
            }));
            let trailing = ctx.emit(Op::NOP, vec![]);
            assert_eq!(ctx.instructions[trailing].source_span, Some(outer_span));
            assert_eq!(
                ctx.instructions[trailing].source_cmd_text.bytes(),
                source.as_bytes()
            );
            assert_eq!(ctx.instructions[trailing].source_line, 1);
        }
    }

    #[test]
    fn dictionary_iterations_close_the_same_indexed_search_on_both_exits() {
        let registry = CommandRegistry::build_default();
        for map in [false, true] {
            let mut ctx = CodegenCtx::new(true, &["d"], &registry);
            let emitted = if map {
                ctx.emit_dict_map("k v", "$d", "set result $v")
            } else {
                ctx.emit_dict_for("k v", "$d", "set result $v")
            };
            assert!(emitted);
            let begin = ctx
                .instructions
                .iter()
                .find(|instruction| instruction.op == Op::BEGIN_CATCH4)
                .unwrap();
            assert!(
                begin
                    .catch_target
                    .as_ref()
                    .is_some_and(|label| ctx.label_positions.contains_key(label))
            );
            let slot = ctx
                .instructions
                .iter()
                .find(|instruction| instruction.op == Op::DICT_FIRST)
                .unwrap()
                .operands[0]
                .clone();
            assert_eq!(
                ctx.instructions
                    .iter()
                    .filter(|instruction| {
                        instruction.op == Op::UNSET_SCALAR
                            && instruction.operands == [Operand::Imm(0), slot.clone()]
                    })
                    .count(),
                2
            );
            assert_eq!(
                ctx.instructions
                    .iter()
                    .filter(|instruction| instruction.op == Op::END_CATCH)
                    .count(),
                2
            );
        }
    }

    #[test]
    fn dictionary_bodies_keep_abrupt_completions_in_their_original_activation() {
        let registry = CommandRegistry::build_default();
        for body in ["error BODY", "break", "continue", "return BODY"] {
            for map in [false, true] {
                let mut ctx = CodegenCtx::new(true, &["d"], &registry);
                assert!(if map {
                    ctx.emit_dict_map("k v", "$d", body)
                } else {
                    ctx.emit_dict_for("k v", "$d", body)
                });
                assert_eq!(ctx.inline_loop_regions.len(), 1);
                assert!(
                    ctx.instructions
                        .iter()
                        .all(|instruction| instruction.op != Op::EVAL_STK)
                );
                assert!(
                    ctx.instructions
                        .iter()
                        .any(|instruction| instruction.op == Op::DICT_FIRST)
                );
                assert!(
                    ctx.instructions
                        .iter()
                        .any(|instruction| instruction.op == Op::RETURN_STK)
                );
            }
            for with in [false, true] {
                let mut ctx = CodegenCtx::new(true, &["d"], &registry);
                assert!(if with {
                    ctx.emit_dict_with("d", body)
                } else {
                    ctx.emit_dict_update(&["d".into(), "k".into(), "v".into(), body.into()])
                });
                assert!(
                    ctx.instructions
                        .iter()
                        .all(|instruction| instruction.op != Op::EVAL_STK)
                );
                let protected = ctx
                    .instructions
                    .iter()
                    .find(|i| i.op == Op::BEGIN_CATCH4)
                    .unwrap();
                assert!(
                    protected
                        .catch_target
                        .as_ref()
                        .is_some_and(|label| ctx.label_positions.contains_key(label))
                );
                assert_eq!(
                    ctx.instructions
                        .iter()
                        .filter(|i| i.op
                            == if with {
                                Op::DICT_RECOMBINE_IMM
                            } else {
                                Op::DICT_UPDATE_END
                            })
                        .count(),
                    2
                );
                assert!(ctx.instructions.iter().any(|i| i.op == Op::RETURN_STK));
            }
        }
    }

    /// The catch-body `expr` re-parse follows the compile's dialect, and its
    /// operator set follows the target release: `catch {expr {2 ** 3}}`
    /// compiled for 8.4 must not fold to a push of `8` and report success,
    /// since the same source evaluated through `exprStk` is rejected as C Tcl
    /// 8.4 rejects it.
    #[test]
    fn catch_body_expr_follows_the_compile_target_release() {
        let registry = CommandRegistry::build_default();

        let mut old = CodegenCtx::new(true, &[], &registry);
        old.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile());
        old.emit_catch_body("expr {2 ** 3}");
        let ops: Vec<Op> = old.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(old.literals.entries().iter().any(|l| l == "2 ** 3"));

        let mut modern = CodegenCtx::new(true, &[], &registry);
        modern.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile());
        modern.emit_catch_body("expr {2 ** 3}");
        let ops: Vec<Op> = modern.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(modern.literals.entries().iter().any(|l| l == "8"));
    }

    /// A catch body is a fresh script parse. Its `{*}` marker must reach the
    /// expansion-aware command emitter instead of becoming a literal `*`
    /// argument beside the value to expand.
    #[test]
    fn catch_body_preserves_argument_expansion() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["args"], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());

        ctx.emit_catch_body("Configure {*}$args");

        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EXPAND_START), "{ops:?}");
        assert!(ops.contains(&Op::EXPAND_STKTOP), "{ops:?}");
        assert!(ops.contains(&Op::INVOKE_EXPANDED), "{ops:?}");
        assert!(!ops.contains(&Op::INVOKE_STK1), "{ops:?}");
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != "*"),
            "the expansion marker must not enter argv: {:?}",
            ctx.literals.entries()
        );
    }

    /// `dict update` and `dict with` run as commands here, and the VM logs
    /// their body frame only where the release would not have compiled them.
    /// Measured with `::tcl::unsupported::disassemble`: 8.5.19 compiles
    /// `dict update d k v {}` in a procedure body (`dictUpdateStart`) and
    /// leaves `dict with` and a `::d` or `a(1)` update to the command; 8.6.18
    /// and 9.0.4 also compile `dict with d {...}` and `dict with ::d {...}`
    /// in one, but not `dict update` at the top level.
    #[test]
    fn the_release_compiles_dict_update_and_dict_with_in_a_procedure_body() {
        let registry = CommandRegistry::build_default();
        let compiled = |release: &str, is_proc: bool, args: &[(&str, bool)]| {
            let mut ctx = CodegenCtx::new(is_proc, &[], &registry);
            ctx.dialect =
                Some(tcl_registry::model::ingress::resolve_environment(release).analyser_profile());
            ctx.release_compiled("dict", args)
        };
        let update = [
            ("update", false),
            ("d", false),
            ("k", false),
            ("v", false),
            ("set v 2", true),
        ];
        let with = [("with", false), ("d", false), ("set a 1", true)];
        let in_procedure = Some(tcl_bytecode::ReleaseCompiled {
            script_word: None,
            frame_local_words: 0,
            in_procedure_only: false,
        });
        for release in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_eq!(compiled(release, true, &update), in_procedure, "{release}");
            assert_eq!(
                compiled(release, false, &update).map(|c| c.in_procedure_only),
                Some(true),
                "{release}"
            );
            for (variable, key) in [("::d", "v"), ("d", "a(1)")] {
                let args = [
                    ("update", false),
                    (variable, false),
                    ("k", false),
                    (key, false),
                    ("set x 1", true),
                ];
                assert_eq!(compiled(release, true, &args), None, "{release} {args:?}");
            }
            assert_eq!(
                compiled(
                    release,
                    true,
                    &[
                        ("update", false),
                        ("d", false),
                        ("k", false),
                        ("v", false),
                        ("$body", false)
                    ]
                ),
                None,
                "{release}: a substituted body is left to the command"
            );
        }
        assert_eq!(compiled("tcl8.5", true, &with), None);
        for release in ["tcl8.6", "tcl9.0"] {
            assert_eq!(compiled(release, true, &with), in_procedure, "{release}");
            assert_eq!(
                compiled(
                    release,
                    true,
                    &[("with", false), ("::d", false), ("set x 1", true)]
                ),
                in_procedure,
                "{release}"
            );
        }
    }

    /// C 8.6 compiles a substituted script word of `catch`, `try` and `for`
    /// with the command where its compiler compiles the command, and leaves
    /// it to the command otherwise. Measured on tclsh 8.6.18 with a canonical
    /// list holding a literal index it compiles apart.
    #[test]
    fn the_release_compiles_a_substituted_catch_try_or_for_script_word() {
        let registry = CommandRegistry::build_default();
        let compiled = |is_proc: bool, command: &str, args: &[(&str, bool)]| {
            CodegenCtx::new(is_proc, &[], &registry).release_compiled(command, args)
        };
        let word = |is_proc: bool, command: &str, args: &[(&str, bool)]| {
            compiled(is_proc, command, args).and_then(|compiled| compiled.script_word)
        };
        let b = ("$b", false);
        let handler = [("on", false), ("ok", false), ("r", true), ("set r", true)];
        let with_handler = |body| [&[body][..], &handler[..]].concat();
        assert_eq!(word(true, "catch", &[b, ("r", false)]), Some(1));
        assert_eq!(
            word(true, "catch", &[b, ("r", false), ("o", false)]),
            Some(1)
        );
        assert_eq!(word(false, "catch", &[b]), Some(1));
        // Outside a procedure body the variable must already be a local of
        // the frame the script runs in.
        assert_eq!(
            compiled(false, "catch", &[b, ("r", false)]),
            Some(tcl_bytecode::ReleaseCompiled {
                script_word: Some(1),
                frame_local_words: 0b100,
                in_procedure_only: false,
            })
        );
        assert_eq!(
            compiled(true, "catch", &[b, ("r", false)]).map(|c| c.frame_local_words),
            Some(0)
        );
        assert_eq!(word(true, "catch", &[b, ("::r", false)]), None);
        assert_eq!(word(true, "catch", &[b, ("a(x)", false)]), None);
        assert_eq!(word(true, "catch", &[b, ("$r", false)]), None);
        assert_eq!(
            word(true, "catch", &[("set x 1", true), ("r", false)]),
            None
        );
        assert_eq!(word(false, "try", &[b]), Some(1));
        assert_eq!(
            word(false, "try", &[b, ("finally", false), ("", true)]),
            Some(1)
        );
        assert_eq!(word(true, "try", &with_handler(b)), Some(1));
        assert_eq!(word(false, "try", &with_handler(b)), None);
        assert_eq!(
            word(
                true,
                "try",
                &[b, ("trap", false), ("", true), ("", true), ("", true)]
            ),
            None
        );
        assert_eq!(
            word(
                true,
                "try",
                &[b, ("on", false), ("ok", false), ("r", true), ("$h", false)]
            ),
            None
        );
        assert_eq!(
            word(
                true,
                "try",
                &[b, ("on", false), ("ok", false), ("::r", true), ("", true)]
            ),
            None
        );
        assert_eq!(
            word(true, "try", &[b, ("finally", false), ("$f", false)]),
            None
        );
        assert_eq!(
            word(true, "for", &[b, ("0", true), ("", true), ("", true)]),
            Some(1)
        );
        assert_eq!(word(true, "for", &[b, ("0", true), ("", true), b]), None);
        assert_eq!(word(true, "lmap", &[("x", false), ("1", true), b]), None);
        assert_eq!(word(true, "eval", &[b]), None);

        // A compiled `try` binds its matched clause's variables whatever its
        // body: `try {set x 1} on ok {r} - on error {e} {…}` in a procedure.
        let falls_through = [
            ("set x 1", true),
            ("on", false),
            ("ok", false),
            ("r", true),
            ("-", false),
            ("on", false),
            ("error", false),
            ("e", true),
            ("list $r", true),
        ];
        let literal = Some(tcl_bytecode::ReleaseCompiled {
            script_word: None,
            frame_local_words: 0,
            in_procedure_only: false,
        });
        assert_eq!(compiled(true, "try", &falls_through), literal);
        assert_eq!(compiled(false, "try", &falls_through), None);
        let mut dynamic = falls_through;
        dynamic[8] = ("$h", false);
        assert_eq!(compiled(true, "try", &dynamic), None);
    }

    #[test]
    fn catch_inline_emits_begin_end_catch() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x 1", Some("result"), None);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::BEGIN_CATCH4));
        assert!(ops.contains(&Op::END_CATCH));
    }

    fn assert_captured_body_original_source(
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let (_native_owner, entry) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let ir = crate::lowering::lower_procedure_target_module_for_bytecode_with_options(
            target,
            registry,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            Some(profile),
            false,
            Some(
                crate::compile_service::BytecodeCompileService::native_entry_options(
                    &entry,
                    Some(profile),
                ),
            ),
        );
        let statement = &ir.top_level.statements[0];
        let selection = statement.tokens().and_then(|tokens| {
            tokens.source_binding.as_ref().map(|binding| {
                (
                    binding.native_compilation_admission_selection(),
                    binding
                        .proved_execution_target()
                        .map(|target| target.command.as_str()),
                )
            })
        });
        assert!(
            matches!(statement, crate::ir::Statement::Catch { .. }),
            "native catch lowering must survive a suspended child: {selection:?}"
        );
        if let crate::ir::Statement::Catch { body, .. } = statement {
            assert_eq!(body.statements.len(), 2, "{body:?}");
            let words = body.statements[1]
                .tokens()
                .expect("original error words")
                .words();
            assert!(
                matches!(words.get(1), Some(crate::ir::WordExpr::Template { parts, .. })
                if parts.iter().any(|part| matches!(part, crate::ir::WordPart::Variable { source, .. }
                    if target.source.get(source.span.as_range()) == Some("$x")))),
                "error operand must retain its own quoted variable read: {words:?}"
            );
        }
        let prepared = crate::cfg_builder::prepare_cfg_context_bundle(&ir, registry);
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry_and_context(
            &ir,
            false,
            registry,
            &prepared,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        assert!(
            cfg.top_level
                .blocks
                .values()
                .any(|block| block.name.starts_with("catch_body_")),
            "captured native body must reach its own CFG region: {:?}",
            cfg.top_level
                .blocks
                .values()
                .map(|block| (&block.name, &block.statements))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn inline_catch_ignores_a_trailing_unseen_call_marker() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut cfg = CfgFunction::new("::p", "entry_0");
        let body = cfg.intern_block("catch_body_1");
        cfg.blocks.insert(
            body,
            Block {
                name: "catch_body_1".into(),
                statements: vec![
                    Statement::Call {
                        span: tcl_lexer::Span::new(0, 0),
                        command: "puts".into(),
                        canonical_command: None,
                        args: vec!["body".into()],
                        defs: vec![],
                        reads: vec![],
                        reads_own_defs: false,
                        safe_on_uninit: false,
                        tokens: None,
                        foreach_groups: None,
                    },
                    Statement::unseen_call_marker(tcl_lexer::Span::new(0, 0)),
                ],
                terminator: None,
            },
        );

        ctx.emit_catch_region_inline(&cfg, "catch_body_1", None, None);

        assert_eq!(
            ctx.cmd_index, 2,
            "only the catch wrapper and body are source commands"
        );
        assert!(
            ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::END_CATCH),
            "the real final command still supplies the catch result",
        );
    }

    #[test]
    fn catch_inline_multicommand_body_preserves_each_command_boundary() {
        use tcl_runtime_api::CompileService;
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let target = tcl_runtime_api::ProcedureCompileTarget {
            source: "catch {set x [yield a]; error \"boom-$x\"} result",
            namespace: "::",
            parameters: &[],
        };
        assert_captured_body_original_source(target, profile, registry);
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let module = service
            .compile_procedure_with_entry(
                target,
                profile,
                &entry,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        let ctx = &module.top_level_body;

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert!(ops.contains(&Op::BEGIN_CATCH4), "{ops:?}");
        assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
        assert!(ops.contains(&Op::END_CATCH), "{ops:?}");
        assert!(
            ops.contains(&Op::YIELD)
                && ops
                    .iter()
                    .any(|op| matches!(op, Op::STORE_SCALAR1 | Op::INVOKE_STK1)),
            "the selected nested yield and enclosing setter must both execute: {ops:?}"
        );
        assert!(
            ops.iter().filter(|&&op| op == Op::START_CMD).count() >= 2,
            "the setter and subsequent error retain separate command boundaries: {:?}",
            ctx.instructions
                .iter()
                .map(|instruction| (
                    instruction.op,
                    &instruction.operands,
                    &instruction.source_cmd_text,
                    instruction.source_command_boundary,
                ))
                .collect::<Vec<_>>()
        );
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != ";"),
            "the command separator must not become an argument: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.instructions.iter().any(|instruction| {
                instruction.op == Op::START_CMD
                    && instruction.source_cmd_text == "error \"boom-$x\""
            }),
            "nested replay text must retain the quoted last word: {:?}",
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::START_CMD)
                .map(|instruction| instruction
                    .source_cmd_text
                    .try_text()
                    .expect("Unicode fixture source"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn catch_inline_incomplete_body_uses_the_shared_parse_failure() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x {", Some("result"), None);

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert!(ops.contains(&Op::SYNTAX), "{ops:?}");
        assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
        let begin = ctx
            .instructions
            .iter()
            .find(|i| i.op == Op::BEGIN_CATCH4)
            .unwrap();
        assert!(
            begin
                .catch_target
                .as_ref()
                .is_some_and(|label| ctx.label_positions.contains_key(label))
        );
    }

    #[test]
    fn catch_inline_runs_complete_prefix_before_fatal_command_tail() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("incr side; set x \"", Some("result"), None);

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        let prefix_invoke = ops
            .iter()
            .position(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
            .expect("complete prefix command is emitted");
        let tail_failure = ops
            .iter()
            .position(|op| *op == Op::SYNTAX)
            .expect("malformed tail uses the original script parse failure");
        assert!(prefix_invoke < tail_failure, "{ops:?}");
        assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == "missing \""),
            "the exact reached parser message is retained: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.literals
                .entries()
                .iter()
                .all(|literal| literal != "incr side; set x \""),
            "the complete prefix must not be replayed: {:?}",
            ctx.literals.entries()
        );
    }

    #[test]
    fn nested_try_multicommand_phases_keep_live_ranges_and_boundaries() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());
        let normal = ctx.fresh_label("normal");

        ctx.emit_try_on_error_inline(
            "try",
            &[
                ("set x ok; error boom-$x".into(), true),
                ("on".into(), false),
                ("error".into(), false),
                ("message".into(), false),
                (
                    "set suffix handled; set message $message/$suffix".into(),
                    true,
                ),
            ],
            &normal,
        );
        ctx.place_label(&normal);

        let begin_ranges: Vec<_> = ctx
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::BEGIN_CATCH4)
            .collect();
        assert_eq!(begin_ranges.len(), 2);
        assert!(
            begin_ranges
                .iter()
                .all(|instruction| instruction.catch_target.is_some()),
            "both try phases need live VM exception targets: {begin_ranges:?}"
        );
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != ";"),
            "phase separators must not become arguments: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::POP)
                .count()
                >= 2,
            "each phase must discard its non-final command result"
        );
    }

    #[test]
    fn nested_try_binds_both_typed_handler_variables() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let normal = ctx.fresh_label("normal");

        ctx.emit_try_on_error_inline(
            "try",
            &[
                ("error boom".into(), true),
                ("on".into(), false),
                ("error".into(), false),
                ("message options".into(), true),
                ("list $message [dict get $options -code]".into(), true),
            ],
            &normal,
        );

        let lvt = ctx.lvt.entries();
        assert!(lvt.iter().any(|name| name == "message"), "{lvt:?}");
        assert!(lvt.iter().any(|name| name == "options"), "{lvt:?}");
        assert!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::STORE_SCALAR1)
                .count()
                >= 4,
            "result/options temps and both handler bindings must be stored"
        );
    }

    #[test]
    fn nested_try_receiver_name_data_retains_generic_output_protocol() {
        let registry = CommandRegistry::build_default();
        for variable_list in [
            "{} options",
            "message {}",
            "{$literal} options",
            "{arr(key)} options",
        ] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            let body =
                format!("try {{error boom}} on error {{{variable_list}}} {{set handled yes}}");
            assert!(ctx.emit_catch_inline(&body, Some("result"), None));
            let ops: Vec<_> = ctx
                .instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect();
            assert_eq!(
                ops.iter().filter(|&&op| op == Op::BEGIN_CATCH4).count(),
                1,
                "only the outer catch can specialise: {variable_list}: {ops:?}"
            );
            assert!(ops.contains(&Op::INVOKE_STK1), "{variable_list}: {ops:?}");
            assert!(
                ctx.literals
                    .entries()
                    .iter()
                    .any(|literal| literal == variable_list),
                "original receiver list must reach runtime: {:?}",
                ctx.literals.entries()
            );
            assert!(
                !ctx.lvt
                    .entries()
                    .iter()
                    .any(|name| name == "options" || name == "message"),
                "declined handler outputs must not acquire local slots: {:?}",
                ctx.lvt.entries()
            );
        }
    }

    #[test]
    fn nested_try_declines_unbraced_dynamic_phase_words() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);

        ctx.emit_catch_inline(
            "try $body on error {message options} $handler",
            Some("result"),
            None,
        );

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert_eq!(
            ops.iter().filter(|&&op| op == Op::BEGIN_CATCH4).count(),
            1,
            "only the outer catch may specialise a dynamic try: {ops:?}"
        );
        assert!(
            ops.contains(&Op::INVOKE_STK1),
            "generic try invoke: {ops:?}"
        );
        assert!(
            !ctx.lvt.entries().iter().any(|name| name == "message"),
            "a declined handler must not allocate specialised bindings"
        );
    }

    #[test]
    fn catch_inline_stores_result_var() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x 1", Some("res"), None);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn catch_inline_with_options_declines_without_selection() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        assert!(!ctx.emit_catch_inline("set x 1", Some("res"), Some("opts")));
        assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
        assert_eq!(ctx.lvt.entries(), [] as [tcl_runtime_api::NameBytes; 0]);
    }

    #[test]
    fn catch_return_simple() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_return(&[("hello".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn catch_return_with_code() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_return(&[
            ("-code".into(), false),
            ("error".into(), false),
            ("oops".into(), false),
        ]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn catch_error_emits_return_imm() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_error("error", "error oops", &[("oops".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn detect_div_by_zero() {
        let node = ExprNode::Binary {
            op: BinOp::Div,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 4,
                end: 5,
            }),
        };
        let result = detect_const_expr_error(&node);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "divide by zero");
    }

    #[test]
    fn no_error_for_non_zero() {
        let node = ExprNode::Binary {
            op: BinOp::Div,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 4,
                end: 5,
            }),
        };
        assert!(detect_const_expr_error(&node).is_none());
    }

    // Registry drift: catch-body classifications.

    /// The registry's `NEEDS_START_CMD` set must equal the set pinned here,
    /// so a stamping change is a conscious decision rather than a silent
    /// bytecode change.
    #[test]
    fn needs_start_cmd_trait_matches_previous_hardcoded_set() {
        let registry = CommandRegistry::build_default();
        let mut got = registry.commands_with_trait(Traits::NEEDS_START_CMD);
        got.sort_unstable();
        assert_eq!(got, ["break", "continue", "error", "expr", "return"]);
    }

    /// `break` in a catch body keeps its inline `break` opcode under a
    /// `startCommand` wrap (hook + trait both registry-resolved).
    #[test]
    fn catch_body_wrapping_declines_an_unavailable_compilation_scope() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.resolution_namespace.key = None;
        ctx.emit_catch_body("break");
        assert!(
            ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::INVOKE_STK1)
        );
        assert!(
            !ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::START_CMD)
        );
        assert!(
            !ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::BREAK)
        );
        assert!(ctx.command_binding_requirements.is_empty());
    }

    #[test]
    fn catch_body_break_emits_break_op() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("break");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::BREAK), "expected BREAK, got {ops:?}");
        assert!(
            ops.contains(&Op::START_CMD),
            "break needs a startCommand wrap, got {ops:?}"
        );
    }

    /// `::error` names the global builtin, which C compiles as it compiles
    /// `error` (`TclCompileErrorCmd`).
    #[test]
    fn catch_body_qualified_error_compiles_as_error() {
        let registry = CommandRegistry::build_default();
        let qualified = {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.emit_catch_body("::error boom");
            ctx.instructions.iter().map(|i| i.op).collect::<Vec<Op>>()
        };
        let bare = {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.emit_catch_body("error boom");
            ctx.instructions.iter().map(|i| i.op).collect::<Vec<Op>>()
        };
        assert_eq!(qualified, bare);
    }

    /// `throw` carries `CATCHABLE_THROW` like `error`, but its first
    /// argument is the error-code *type*, not the message — it has no
    /// `Error` inline hook, so a catch body must keep the generic
    /// invoke for it (guarding the throw ≠ error distinction).
    #[test]
    fn catch_body_throw_stays_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("throw {A B} boom");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::RETURN_IMM), "no inline throw: {ops:?}");
        assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke: {ops:?}");
    }
}
