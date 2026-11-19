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

//! Which control commands the emulated release's compiler compiles: loops, dict scopes, `catch` and `try`.

use super::super::CodegenCtx;
use super::super::values::is_qualified;
use super::{ReleaseWord, is_release_local_scalar_name};
use tcl_registry::TryClauseKind;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::release_compilation::CompileWord;

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
                        if super::super::values::is_array_ref(var.text) {
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
        for name in tcl_registry::release_compilation::compiled_loop_variables_for(
            self.vm_release(),
            command,
            &compile_words,
        ) {
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
        let written = tcl_syntax::naming::canonical_written_command(command);
        let (sub, rest) = match tcl_syntax::naming::unroot_rooted_key(&written).unwrap_or(&written)
        {
            // registry-axis-ok: subcommands — dict's scoping subcommands in codegen; until the native compilation descriptor selects them
            "dict" => {
                let (sub, rest) = words.split_first()?;
                (sub.literal.then_some(sub.text)?, rest)
            }
            "tcl::dict::update" => ("update", words),
            "tcl::dict::with" => ("with", words),
            _ => return None,
        };
        let local_scalar = |word: &CompileWord<'_>| {
            word.literal
                && !is_qualified(word.text)
                && !super::super::values::is_array_ref(word.text)
        };
        let (body, scope) = rest.split_last()?;
        if !body.literal {
            return None;
        }
        let locals: Vec<&str> = match sub {
            // registry-axis-ok: subcommands — dict's scoping subcommands in codegen; until the native compilation descriptor selects them
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
            // registry-axis-ok: subcommands — dict's scoping subcommands in codegen; until the native compilation descriptor selects them
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
                            || self
                                .word_rules
                                .split_list(selector.text)
                                .is_ok_and(|codes| !codes.is_empty()))
                });
            let variables = clause
                .variable_list_index
                .and_then(word)
                .is_some_and(|list| {
                    list.known()
                        && self.word_rules.split_list(list.text).is_ok_and(|names| {
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
            .filter_map(|list| self.word_rules.split_list(list).ok())
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
}
