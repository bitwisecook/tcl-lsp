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

//! Variable loading/storing and value emission.
//!
//! Extends [`CodegenCtx`] with methods for pushing literals, loading
//! and storing variables, emitting increments, and parsing variable
//! reference markers.

use super::format::esc;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

struct OriginalCompilerWords {
    words: Vec<tcl_lexer::NativeWord>,
    version: tcl_dialect::TclVersion,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
}

// Literal emission.

impl CodegenCtx<'_> {
    pub(super) fn push_native_pool_literal(&mut self, index: usize) {
        let instruction = self.emit(
            if index < 256 { Op::PUSH1 } else { Op::PUSH4 },
            vec![Operand::Imm(super::bytecode_imm(index))],
        );
        self.instructions[instruction].push_verbatim = true;
    }

    /// Register an unchanged original C command head through its actual owner.
    /// Authored compiler simulations and other engines supply no C cache action.
    pub(super) fn intern_original_native_command_literal(
        &mut self,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<usize> {
        // DIRECT source evaluates fresh original words; it owns no native
        // compiler literal registration or command-name priming action.
        if self.plain_command_dispatch {
            return None;
        }
        let entry = self.native_entry?;
        if entry.execution_point?.tcl_version().is_none()
            || entry.name_protocol?.authority() != tcl_syntax::naming::NamePolicyAuthority::Native
        {
            return None;
        }
        let selected = entry.source_string_protocol.and_then(|protocol| {
            let captured =
                tcl_registry::native_compiler_words::NativeCompilerWords::capture(words, protocol)
                    .ok()?;
            Some(
                tcl_registry::native_command_literal::native_compiled_command_literal(
                    entry, &captured,
                ),
            )
        });
        let Some(Ok(selected)) = selected else {
            self.refuse_native_dependency();
            return None;
        };
        let selected = selected?;
        Some(self.intern_native_command_recipe(selected))
    }

    pub(super) fn intern_native_command_recipe(
        &mut self,
        selected: tcl_registry::native_command_literal::NativeCompiledCommandLiteral,
    ) -> usize {
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming
            && !self.literals.prime_native_command_name(index, priming)
        {
            self.refuse_native_dependency();
        }
        if selected.hide && !self.literals.hide_native_literal(index) {
            self.refuse_native_dependency();
        }
        index
    }

    /// Emit a private command name from its retained original selection.
    /// The compiler-produced name is not parsed as a synthetic source word.
    pub(super) fn emit_selected_native_command_name(
        &mut self,
        tokens: &crate::ir::CommandTokens,
        named: &crate::command_binding::SourceNamedInvocationProof,
    ) {
        let Some(entry) = self.native_entry else {
            self.push_lit_exact(named.captured_name());
            return;
        };
        if entry
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
            .is_none()
            || entry.name_protocol.is_none_or(|protocol| {
                protocol.authority() != tcl_syntax::naming::NamePolicyAuthority::Native
            })
        {
            self.push_lit_exact(named.captured_name());
            return;
        }
        if tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.admitted_named_invocation())
            != Some(named)
            || crate::registry_invocation::native_compiler_replay_source(
                tokens,
                &named.compilation_site,
            )
            .is_none()
        {
            self.refuse_native_dependency();
            self.push_lit_exact(named.captured_name());
            return;
        }
        let selected = if let Some(required) = &named.compiler_prerequisite {
            let namespace = entry
                .namespaces
                .iter()
                .find(|namespace| namespace.token == entry.current_namespace);
            match (entry.execution_point, entry.name_protocol, namespace) {
                (Some(point), Some(protocol), Some(namespace)) => tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
                    point, protocol, tcl_runtime_api::native_command_name::NativeLiteralContext {
                        interpreter: entry.interpreter,
                        namespace_token: entry.current_namespace,
                        entry_epoch: entry.epoch,
                        namespace_path: namespace.path.clone(),
                    }, named.captured_name().as_bytes(), required.as_ref().clone(),
                ),
                _ => Err(tcl_registry::native_command_literal::NativeCommandLiteralUnavailable::Context),
            }
        } else {
            tcl_registry::native_command_literal::native_compiled_command_name_literal(
                entry,
                named.captured_name().as_bytes(),
            )
        };
        if let Ok(selected) = selected {
            let index = self.intern_native_command_recipe(selected);
            self.push_native_pool_literal(index);
        } else {
            self.refuse_native_dependency();
            self.push_lit_exact(named.captured_name());
        }
    }

    /// Emit the original head only when the caller kept its original operand.
    /// Derived private worker names require their own compiler recipe.
    pub(super) fn try_emit_original_command_head(&mut self, value: &str) -> bool {
        if self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.argv_texts.first())
            .is_none_or(|original| original != value)
        {
            return false;
        }
        let Some(original) = self.original_compiler_words() else {
            return false;
        };
        if let Some(index) = self.intern_original_native_command_literal(&original.words) {
            self.push_native_pool_literal(index);
        } else if let Some(head) = original.words.first() {
            self.emit_original_native_word(head);
        } else {
            return false;
        }
        true
    }

    /// Registry-owned constant command-substitution folds and the two `list`
    /// inlinings that sit beside them — one copy, shared by both value
    /// emitters.
    ///
    /// **A folded result is a value, so it is always pushed verbatim.** Folding
    /// runs the command at compile time; its result has no word rule left to
    /// apply, and handing it back to the VM's `subst_word` would substitute it
    /// a second time.
    ///
    /// Every fold is gated on its own builtin still *being* the builtin
    /// anywhere in this unit: a fold **is** that command's
    /// semantics, so after `rename list mylist` or a shadowing `proc format …`
    /// it would answer for a command that is no longer there. The active
    /// registry selects the exact command/subcommand callback and release
    /// surface, so an owned registry can replace a fold and an
    /// unavailable command cannot be manufactured by codegen.
    ///
    /// Returns `true` when it emitted the value and the caller must stop.
    pub(crate) fn try_emit_constant_fold(&mut self, value: &str) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        // Actual native folds require this value's unchanged original bracket
        // operand, not a representative string rebuilt as a new source image.
        if let Some(entry) = self.native_entry {
            let Some(words) = self.original_constant_command_words(value) else {
                return false;
            };
            if let Ok(crate::native_byte_compilation::NativeByteCommandPlan::Registered(selected)) =
                crate::native_byte_compilation::native_byte_command_plan(
                    &words,
                    entry,
                    self.registry,
                    self.native_compilation,
                )
                && selected.spec.grammar
                    == tcl_registry::native_compilation::NativeCompilationGrammar::ArgumentList
            {
                self.emit_native_words(&words);
                return true;
            }
        }
        let fold = value
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .and_then(|inner| {
                let trusts = |name: &str| self.trusts_builtin(name);
                let lookup = |_name: &str| None;
                let namespace = self.resolution_namespace()?;
                crate::const_subst::ConstSubstCtx {
                    registry: self.registry,
                    resolution_namespace: namespace,
                    namespace_context: self
                        .invocation_tokens
                        .as_deref()
                        .and_then(crate::registry_invocation::compiled_namespace_context),
                    version: self
                        .dialect
                        .and_then(tcl_dialect::DialectProfile::const_fold_version),
                    defining_class: None,
                    trusts: &trusts,
                    lookup_var: &lookup,
                }
                .fold_cmd_subst_resolved(inner)
            });
        if let Some(fold) = fold {
            for binding in &fold.command_bindings {
                self.require_command_binding(binding);
            }
            // The fold consumes a real command invocation. Retain its own
            // replay boundary so a command-table mutation in an earlier
            // argument of the enclosing active command cannot make these
            // literal pushes execute stale builtin semantics.
            let end = self.begin_consumed_inline_command(
                value
                    .strip_prefix('[')
                    .and_then(|inner| inner.strip_suffix(']'))
                    .expect("successful command fold has brackets"),
            );
            if fold.return_type == Some(tcl_registry::TclType::Dict) {
                self.push_lit_no_dedup_verbatim(&fold.value);
                self.emit(Op::DUP, vec![]);
                self.emit(Op::VERIFY_DICT, vec![]);
            } else {
                // A command result is already a final Tcl value. It must not
                // be parsed as another word: `$`, `[…]`, braces, and backslash
                // bytes in the result are data, not a second substitution.
                self.push_lit_no_dedup_verbatim(&fold.value);
            }
            self.place_label(&end);
            return true;
        }
        // Inline [list {*}$a {*}$b] → load a, load b, listConcat. tclsh 9.0
        // compiles two-list expansion as a specialised listConcat opcode
        // rather than a generic `list` invoke.
        if self.try_list_expand_concat(value) {
            return true;
        }
        // Inline [list arg ... [break] ...] / [list arg ... [continue] ...].
        // tclsh 9.0 compiles break/continue inside `list` command
        // substitutions as inline jumps with stack cleanup.
        if self.try_inline_list_with_break_continue(value) {
            return true;
        }
        false
    }
    /// Select one retained bare bracket word whose source spelling is unchanged.
    /// Its arena owns the nested body range in the original source image.
    fn original_constant_command_words(&self, value: &str) -> Option<Vec<tcl_lexer::NativeWord>> {
        let original = self.original_compiler_words()?;
        let mut matches = original.words.iter().filter(|word| {
            word.group().kind == tcl_lexer::WordKind::Bare
                && word.image().bytes().get(word.span().as_range()) == Some(value.as_bytes())
        });
        let word = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let arena = word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let tcl_lexer::ExecutablePart::Command { body } = &part.part else {
            return None;
        };
        let plan =
            tcl_lexer::native_script_words_in(arena.image().clone(), *body, word.config()).ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return None;
        }
        Some(plan.commands.into_iter().next()?.words)
    }
    /// Push a literal onto the stack with deduplication.
    pub fn push_lit(&mut self, value: &str) {
        let idx = self.literals.intern(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", esc(value, 40)),
        );
    }

    /// Push a *verbatim* literal — a braced / constant word that the VM must
    /// push exactly as-is, suppressing runtime word substitution. Apart from the
    /// brace-word `\<newline>` continuation collapse below, the literal bytes
    /// match [`push_lit`]; only the out-of-band `push_verbatim` flag differs, so
    /// disassembly stays byte-stable.
    pub fn push_lit_verbatim(&mut self, value: &str) {
        // A braced word's value collapses `\<newline>` continuations even inside
        // braces (the one substitution braces permit) *in every build of the
        // Tcl core*; `JimTcl` keeps the bytes, so the dialect answers rather
        // than this call site. Every other backslash stays verbatim. Only a braced word — or a bare constant, for which
        // this is a borrow-only no-op — reaches this verbatim path (a bare word
        // separates on the continuation; a quoted word is `backslash_subst`-
        // decoded through `push_lit`), so collapsing here is safe.
        let value = self.word_rules.collapse_braced_word(value);
        let idx = self.literals.intern(&value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", esc(&value, 40)),
        );
        self.instructions[pos].push_verbatim = true;
    }

    /// Push a *verbatim* literal **byte for byte** — a value that is already
    /// final, with no word-level rule left to apply.
    ///
    /// [`push_lit_verbatim`](Self::push_lit_verbatim) collapses `\<newline>`
    /// continuations because its input is a *braced word*, where Tcl really
    /// does collapse them (tclsh 8.6.16 / 9.0.4: `set {z1\`<newline>`y} B`
    /// creates the 4-byte name `z1 y`). A resolved **variable name** is not a
    /// word: a backslash inside `${…}` is an ordinary name byte, so the
    /// collapse would load a different variable. Verified identical on tclsh
    /// 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1:
    ///
    /// ```text
    /// set n [format a%c%cb 92 10]   ;# the 4-byte name a\<newline>b
    /// set $n VALUE
    /// set {a b} COLLAPSED
    /// set out ${a\`<newline>`b}
    /// -> VALUE      (the collapse would have read `a b` and given COLLAPSED)
    /// ```
    pub fn push_lit_exact(&mut self, value: &str) {
        let idx = self.literals.intern(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", esc(value, 40)),
        );
        self.instructions[pos].push_verbatim = true;
    }

    /// Push an already decoded native byte string without substitution.
    pub fn push_lit_bytes_exact(&mut self, value: &[u8]) {
        let idx = self.literals.intern_bytes(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", tcl_bytecode::format::esc_bytes(value, 40)),
        );
        self.instructions[pos].push_verbatim = true;
    }

    /// Decode an authored literal fragment with the actual native escape policy.
    pub fn push_decoded_literal(&mut self, source: &str) {
        let bytes = tcl_lexer::backslash_subst_bytes_in(source.as_bytes(), self.escapes);
        self.push_lit_bytes_exact(&bytes);
    }

    /// Push a word's **finished value** — text every word-level rule has
    /// already been applied to, so the VM must not run `subst_word` over it a
    /// second time.
    ///
    /// A de-quoted or plain word is a value, not source: `"{}"` *is* the
    /// two-byte string `{}`. Handed to the substituting [`push_lit`], the VM's
    /// `subst_word` reads it back as a whole-word braced literal and strips the
    /// braces a second time — `string index "{}" 0` would answer empty where
    /// both oracles say `{`, and `set v "{}"` would store the empty string.
    /// The braced and quoted halves are the same hole.
    ///
    /// The marker test is the VM's own, byte for byte: `subst_word` returns a
    /// word carrying no `${` and no `[` unchanged *apart from* that brace
    /// strip, so suppressing substitution on one is behaviour-preserving in
    /// every other respect. A value that does still carry a marker is one the
    /// codegen deliberately defers to the runtime scan, so it keeps
    /// [`push_lit`].
    ///
    /// The same rule, unconditionally, covers a *fragment* — the `Lit` part a
    /// composite word decomposes to. `parse_subst_template` has already carved
    /// the real substitutions out of it and decoded its escapes, so a marker
    /// left in one is data by construction and the three decomposition loops
    /// push a `Lit` through [`push_lit_exact`](Self::push_lit_exact) directly.
    /// A fragment that looked braced was being stripped too: `"{}$z"`
    /// concatenated to `x` rather than `{}x`.
    ///
    /// This is [`push_lit_exact`](Self::push_lit_exact), not
    /// [`push_lit_verbatim`](Self::push_lit_verbatim): the value is not a
    /// braced word, so the brace-word `\<newline>` collapse is not its rule —
    /// a quoted word's continuations were folded when its escapes were decoded.
    /// The literal bytes are unchanged either way, so disassembly stays stable
    /// and only the out-of-band `push_verbatim` flag differs.
    pub fn push_word_value(&mut self, value: &str) {
        if value.contains("${") || value.contains('[') {
            self.push_lit(value);
        } else {
            self.push_lit_exact(value);
        }
    }

    /// Push a literal using a fresh slot (no deduplication).
    pub fn push_lit_no_dedup(&mut self, value: &str) {
        let idx = self.literals.register(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\" #nodedup", esc(value, 40)),
        );
    }

    /// Push a *verbatim* no-dedup literal — a constant-folded result (e.g.
    /// `[list …]` / `[format …]`) that is already its own final value and must
    /// be pushed exactly, suppressing runtime word substitution. Without the
    /// `push_verbatim` flag a single-element fold like `[list "a b"]` → `{a b}`
    /// is mistaken for a braced literal and the braces are stripped at runtime.
    /// The literal bytes and disassembly comment match [`push_lit_no_dedup`], so
    /// the only difference is the out-of-band flag.
    pub fn push_lit_no_dedup_verbatim(&mut self, value: &str) {
        let idx = self.literals.register(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\" #nodedup", esc(value, 40)),
        );
        self.instructions[pos].push_verbatim = true;
    }

    /// Emit `startCommand` for non-first specialised commands.
    ///
    /// Must be paired with [`end_command`](Self::end_command) after the
    /// command's instructions are emitted.
    pub fn begin_command(&mut self, count: u32) {
        if self.cmd_index > 0 {
            let label = self.fresh_label("cmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![
                    Operand::Label(label.clone()),
                    Operand::Imm(i32::try_from(count).expect("count fits in i32")),
                ],
                "",
            );
            self.start_cmd_end_label = Some(label);
        } else {
            self.start_cmd_end_label = None;
        }
        self.cmd_index += 1;
    }

    /// Place the end label for the current `startCommand`.
    pub fn end_command(&mut self) {
        if let Some(label) = self.start_cmd_end_label.take() {
            self.place_label(&label);
        }
    }
}

// Array reference helpers.

/// Split `arr(key)` into `("arr", "key")`, or `None` for scalars.
///
/// The codegen-side facade over the one element-split owner,
/// [`split_element_ref`](tcl_syntax::naming::split_element_ref) —
/// `TclObjLookupVarEx`'s rule (`tclVar.c(9.0.4):683-686`). Both halves may be
/// empty, so `(x)` is element `x` of the array named `""`.
#[must_use]
pub fn split_array_ref(name: &str) -> Option<(&str, &str)> {
    tcl_syntax::naming::split_element_ref(name)
}

/// Return `true` if `name` is an array reference like `arr(key)` — the
/// predicate half of [`split_array_ref`], from the same owner.
#[must_use]
pub fn is_array_ref(name: &str) -> bool {
    split_array_ref(name).is_some()
}

/// Whether a resolved variable's scalar or array base is namespace-qualified.
#[must_use]
pub fn is_qualified(name: &str) -> bool {
    let original = name.as_bytes();
    let base =
        tcl_syntax::naming::split_element_ref_bytes(original).map_or(original, |(base, _)| base);
    tcl_syntax::naming::is_qualified(base)
}

// Variable load/store.

impl CodegenCtx<'_> {
    /// Push an array element key onto the stack.
    ///
    /// A key that is *exactly* a whole variable reference (`${var}` or `$var`)
    /// takes the [`load_var`](Self::load_var) fast path (matching tclsh's
    /// `LOAD_SCALAR`-based key in proc context). A *composite* key that embeds
    /// a substitution (`-$opt`, `x$item`, `${item}suf`, `$a([f])`) is built by
    /// the full interpolation emitter so the substitution actually runs;
    /// pushing such a key as a raw literal leaves the variable in the index
    /// unexpanded and the element lookup fails. A pure literal key is pushed
    /// verbatim.
    pub fn push_array_key(&mut self, elem: &str) {
        let image =
            tcl_lexer::SourceImage::from_bytes(elem.as_bytes().to_vec(), self.source.channel());
        let extent = tcl_lexer::Span::new(
            0,
            u32::try_from(elem.len()).expect("array index fits source coordinates"),
        );
        let arena = tcl_lexer::word_parts::ExecutablePartArena::decompose(
            image,
            extent,
            tcl_lexer::word_parts::SubstFlags::default(),
            self.lexer_config(),
        )
        .expect("the complete original array index has valid geometry");
        self.emit_executable_arena(&arena);
    }

    /// Emit exactly one original variable operand, using its actual grammar.
    /// A braced parenthesised name selects an element with a finished key;
    /// only a bare index is evaluated as source. Declining emits nothing.
    pub(crate) fn emit_variable_reference(&mut self, spelling: &str) -> bool {
        let Some(_) =
            tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), self.lexer_config())
                .ok()
                .flatten()
        else {
            return false;
        };
        let image =
            tcl_lexer::SourceImage::from_bytes(spelling.as_bytes().to_vec(), self.source.channel());
        let arena = tcl_lexer::word_parts::ExecutablePartArena::decompose(
            image,
            tcl_lexer::Span::new(
                0,
                u32::try_from(spelling.len()).expect("variable reference fits source coordinates"),
            ),
            tcl_lexer::word_parts::SubstFlags::default(),
            self.lexer_config(),
        )
        .expect("the complete original reference has valid geometry");
        self.emit_executable_arena(&arena);
        true
    }

    /// Prepare one original variable component and return its selected load.
    /// Array source components evaluate their index separately. A single
    /// parenthesised component remains one complete name and only borrows an
    /// already existing slot for that complete name.
    pub(super) fn prepare_original_variable_load(
        &mut self,
        original: &[u8],
        source_index: bool,
    ) -> (Op, Vec<Operand>) {
        use tcl_syntax::naming::NativeCompiledVariableLookup;

        let lookup = self.compiled_variable_protocol.map_or(
            NativeCompiledVariableLookup::DynamicName,
            |protocol| {
                protocol.substitution_lookup(
                    original,
                    !source_index,
                    self.source_variable_environment(),
                )
            },
        );
        let slot = self.select_native_variable_slot(original, lookup);
        if let Some(slot) = slot {
            (
                if source_index && slot < 256 {
                    Op::LOAD_ARRAY1
                } else if source_index {
                    Op::LOAD_ARRAY4
                } else if slot < 256 {
                    Op::LOAD_SCALAR1
                } else {
                    Op::LOAD_SCALAR4
                },
                vec![Operand::Imm(bytecode_imm(slot))],
            )
        } else {
            self.push_lit_bytes_exact(original);
            (
                if source_index {
                    Op::LOAD_ARRAY_STK
                } else {
                    Op::LOAD_STK
                },
                vec![],
            )
        }
    }

    /// Resolve a command's finished scalar name or array base through the
    /// selected compiler, retaining the receipt when borrowing a frame layout.
    pub(super) fn command_variable_slot(&mut self, original: &[u8]) -> Option<usize> {
        let lookup = self.compiled_variable_protocol.map_or(
            tcl_syntax::naming::NativeCompiledVariableLookup::DynamicName,
            |protocol| protocol.command_lookup(original, self.source_variable_environment()),
        );
        self.select_native_variable_slot(original, lookup)
    }

    fn select_native_variable_slot(
        &mut self,
        original: &[u8],
        lookup: tcl_syntax::naming::NativeCompiledVariableLookup,
    ) -> Option<usize> {
        use tcl_syntax::naming::NativeCompiledVariableLookup;
        let protocol = self.compiled_variable_protocol?;
        let slot = match lookup {
            NativeCompiledVariableLookup::DynamicName => None,
            NativeCompiledVariableLookup::CreateLocal => {
                Some(self.lvt.intern_native(protocol, original))
            }
            NativeCompiledVariableLookup::ExistingLocalOnly => {
                self.lvt.find_native(protocol, original)
            }
        };
        if slot.is_some()
            && self.source_variable_environment()
                == tcl_syntax::naming::NativeCompiledVariableEnvironment::BorrowFrameSlots
        {
            self.required_compiled_local_layout
                .clone_from(&self.borrowed_local_layout);
        }
        slot
    }

    /// Prepare the target slot before emitting the right-hand side, so source
    /// local allocation follows the native command's operand order.
    pub(super) fn target_needs_stack(&mut self, name: &str) -> bool {
        let (base, index) = split_array_ref(name).map_or((name, false), |(base, _)| (base, true));
        self.command_variable_slot(base.as_bytes()).is_none() || index
    }

    /// Read a resolved name whose element key retains the legacy source policy.
    /// Original word and expression operands use `emit_variable_reference`.
    pub fn load_var(&mut self, name: &str) {
        if let Some((base, key)) = split_array_ref(name) {
            self.load_array_element(base, key, true);
        } else if let Some(slot) = self.command_variable_slot(name.as_bytes()) {
            let op = if slot < 256 {
                Op::LOAD_SCALAR1
            } else {
                Op::LOAD_SCALAR4
            };
            self.emit_comment(
                op,
                vec![Operand::Imm(bytecode_imm(slot))],
                &format!("var \"{name}\""),
            );
        } else {
            self.push_lit_exact(name);
            self.emit(Op::LOAD_STK, vec![]);
        }
    }

    /// A decoded braced name's key is data, including dollars and backslashes.
    pub(crate) fn load_literal_element(&mut self, base: &str, key: &str) {
        self.load_array_element(base, key, false);
    }

    fn load_array_element(&mut self, base: &str, key: &str, substitute_key: bool) {
        let slot = self.command_variable_slot(base.as_bytes());
        if slot.is_none() {
            self.push_lit_exact(base);
        }
        if substitute_key {
            self.push_array_key(key);
        } else {
            self.push_lit_exact(key);
        }
        if let Some(slot) = slot {
            self.emit_comment(
                if slot < 256 {
                    Op::LOAD_ARRAY1
                } else {
                    Op::LOAD_ARRAY4
                },
                vec![Operand::Imm(bytecode_imm(slot))],
                &format!("var \"{base}\""),
            );
        } else {
            self.emit(Op::LOAD_ARRAY_STK, vec![]);
        }
    }

    /// Emit store instructions for a variable reference.
    ///
    /// Caller must have pushed the value on TOS.  For proc bodies, uses
    /// `storeScalar1`/`storeArray1`.  For top-level, caller must have
    /// pushed name (and key for arrays) before the value.
    pub fn store_var(&mut self, name: &str) {
        let (base, array) = split_array_ref(name).map_or((name, false), |(base, _)| (base, true));
        if let Some(slot) = self.command_variable_slot(base.as_bytes()) {
            let op = match (array, slot < 256) {
                (true, true) => Op::STORE_ARRAY1,
                (true, false) => Op::STORE_ARRAY4,
                (false, true) => Op::STORE_SCALAR1,
                (false, false) => Op::STORE_SCALAR4,
            };
            self.emit_comment(
                op,
                vec![Operand::Imm(bytecode_imm(slot))],
                &format!("var \"{base}\""),
            );
        } else {
            self.emit(
                if array {
                    Op::STORE_ARRAY_STK
                } else {
                    Op::STORE_STK
                },
                vec![],
            );
        }
    }

    /// Retain original lexical operands from their source owner. Original
    /// compiler rules use the physical entry; source values use their separate
    /// issuer. Unlocated/derived IR cannot manufacture lexical words.
    fn original_compiler_words(&self) -> Option<OriginalCompilerWords> {
        let tokens = self.invocation_tokens.as_deref()?;
        if tokens.synthetic.is_some()
            || tokens.words().is_empty()
            || tokens
                .words()
                .iter()
                .any(|word| word.source().provenance != crate::ir::Provenance::Source)
        {
            return None;
        }
        let version = if let Some(entry) = self.native_entry {
            entry.execution_point?.tcl_version()?
        } else if let Some(binding) = tokens.source_binding.as_ref() {
            binding.native_compiler_dialect()?.tcl_version?
        } else {
            crate::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                self.lexer_config(),
            )
            .tcl_version?
        };
        let protocol = self.source_string_protocol?;
        if self
            .native_entry
            .is_some_and(|entry| entry.source_string_protocol != Some(protocol))
        {
            return None;
        }
        let image = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            .map_or_else(
                || self.source_image().clone(),
                |site| site.source.source_image().clone(),
            );
        let words = crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            tokens.words().first()?.source().span.start(),
            tokens.native_lexer_config(self.lexer_config()),
        )?;
        Some(OriginalCompilerWords {
            words,
            version,
            protocol,
        })
    }

    /// Project only this operand's original C variable compiler layout. A
    /// synthetic/captured operand without a unique written slot stays unknown.
    pub(super) fn original_variable_operand(
        &self,
        word: &crate::ir::WordExpr,
    ) -> Option<tcl_syntax::native_variable_words::NativeVariableWordOperand> {
        let original = self.original_compiler_words()?;
        let mut matches = self
            .invocation_tokens
            .as_deref()?
            .words()
            .iter()
            .enumerate()
            .filter(|(_, original)| *original == word);
        let (index, _) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let selected = original.words.get(index)?;
        tcl_syntax::native_variable_words::native_variable_word(
            selected,
            original.version,
            original.protocol,
        )
        .ok()
    }

    /// A literal scalar operand alone can use a compiler local slot. Source
    /// array and computed words retain their own stack evaluation. Hand-built
    /// authoring contexts keep their distinct checked compatibility projection.
    pub(super) fn original_scalar_operand_slot(
        &mut self,
        word: &crate::ir::WordExpr,
    ) -> Option<usize> {
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        let name = match self.original_variable_operand(word) {
            Some(NativeVariableWordOperand::Literal {
                name, index: None, ..
            }) => name,
            None if self.native_entry.is_none()
                && self
                    .invocation_tokens
                    .as_deref()
                    .is_none_or(|tokens| tokens.source_binding.is_none()) =>
            {
                crate::registry_invocation::compiled_local_name_value(
                    word,
                    self.escapes,
                    self.word_rules,
                )?
                .into_bytes()
            }
            Some(_) | None => return None,
        };
        self.command_variable_slot(&name)
    }

    /// Both typed and inline increments consume one original operand layout.
    /// Registration/admission is established by the caller, independently.
    pub(super) fn try_emit_original_increment(&mut self) -> bool {
        use tcl_registry::native_compiler_words::NativeCompilerWords;
        use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};
        let Some(original) = self.original_compiler_words() else {
            if self.native_entry.is_some() {
                self.refuse_native_dependency();
            }
            return false;
        };
        if !matches!(original.words.len(), 2 | 3) {
            return false;
        }
        let Ok(view) = NativeCompilerWords::capture(&original.words, original.protocol) else {
            self.refuse_native_dependency();
            return false;
        };
        let Ok(target) =
            native_variable_word(&original.words[1], original.version, original.protocol)
        else {
            self.refuse_native_dependency();
            return false;
        };
        let (slot, array) = match target {
            NativeVariableWordOperand::Literal { name, index, .. } => {
                let slot = self.command_variable_slot(&name).filter(|slot| *slot < 256);
                if slot.is_none() {
                    self.push_lit_bytes_exact(&name);
                }
                let array = index.is_some();
                if let Some(index) = index {
                    self.push_lit_bytes_exact(&index);
                }
                (slot, array)
            }
            NativeVariableWordOperand::CompoundArray { name, index, .. } => {
                let slot = self.command_variable_slot(&name).filter(|slot| *slot < 256);
                if slot.is_none() {
                    self.push_lit_bytes_exact(&name);
                }
                self.emit_executable_arena(&index);
                (slot, true)
            }
            NativeVariableWordOperand::DynamicWord => {
                self.emit_original_native_word(&original.words[1]);
                (None, false)
            }
        };
        let amount = original.words.get(2);
        let immediate = amount.map_or(Some(1), |_| view.increment_immediate(2, original.version));
        if immediate.is_none() {
            self.emit_original_native_word(amount.expect("non-immediate original amount"));
        }
        self.finish_increment(slot, array, immediate);
        true
    }

    fn finish_increment(&mut self, slot: Option<usize>, array: bool, immediate: Option<i32>) {
        let op = match (array, slot.is_some(), immediate.is_some()) {
            (false, true, true) => Op::INCR_SCALAR1_IMM,
            (false, true, false) => Op::INCR_SCALAR1,
            (true, true, true) => Op::INCR_ARRAY1_IMM,
            (true, true, false) => Op::INCR_ARRAY1,
            (false, false, true) => Op::INCR_STK_IMM,
            (false, false, false) => Op::INCR_STK,
            (true, false, true) => Op::INCR_ARRAY_STK_IMM,
            (true, false, false) => Op::INCR_ARRAY_STK,
        };
        let mut operands = Vec::new();
        if let Some(slot) = slot {
            operands.push(Operand::Imm(bytecode_imm(slot)));
        }
        if let Some(immediate) = immediate {
            operands.push(Operand::Imm(immediate));
        }
        self.emit(op, operands);
    }

    /// Emit incr bytecode, leaving the new value on TOS.
    ///
    /// Evaluate the original amount once and select the native scalar/array
    /// instruction family. One-byte local operands use dynamic lookup when the
    /// selected slot exceeds the native instruction's index extent.
    ///
    /// `name` is a [resolved store name](CodegenCtx::store_target) and
    /// `key_is_literal` its element-key half — the same contract
    /// [`push_var_ref`](CodegenCtx::push_var_ref) documents, so a name the
    /// compiler already resolved is never word-substituted again by the VM.
    pub fn emit_incr(&mut self, name: &str, key_is_literal: bool, amount: Option<&str>) {
        if self.try_emit_original_increment() {
            return;
        }
        // Authored, already-evaluated IR compatibility does not attest original
        // token shapes or an actual native compiler entry.
        let (base, key) =
            split_array_ref(name).map_or((name, None), |(base, key)| (base, Some(key)));
        // C's increment instructions have one-byte local operands. The
        // compiler still allocates a large slot before choosing its stack form.
        let slot = self
            .command_variable_slot(base.as_bytes())
            .filter(|slot| *slot < 256);
        if slot.is_none() {
            self.push_lit_exact(base);
        }
        if let Some(key) = key {
            if key_is_literal {
                self.push_lit_exact(key);
            } else {
                self.push_array_key(key);
            }
        }
        let immediate = match amount {
            None => Some(1),
            Some(text) if self.native_entry.is_none() && is_integer_literal(text) => self
                .parse_int_operand(text)
                .filter(|value| (-127..=127).contains(value))
                .and_then(|value| i32::try_from(value).ok()),
            Some(_) => None,
        };
        if immediate.is_none() {
            self.emit_increment_amount(amount.expect("non-immediate amount is present"));
        }
        self.finish_increment(slot, key.is_some(), immediate);
    }

    /// Evaluate the original amount word once, retaining its brace semantics.
    fn emit_increment_amount(&mut self, amount: &str) {
        let word = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.words().get(2))
            .cloned();
        if let Some(word) = word {
            let braced = matches!(word, crate::ir::WordExpr::BracedLiteral { .. });
            self.emit_word_from_source(amount, braced, Some(&word));
        } else {
            // Unlocated IR amounts have already been evaluated. Their bytes
            // cannot establish a new command or variable substitution.
            self.push_lit_exact(amount);
        }
    }
}

// Reference parsing.

/// Extract variable name from a normalised `${var}` reference, under the
/// target release's `${…}` close rule.
///
/// The lowering pass normalises actual variable substitutions to
/// `${varname}`; bare `$varname` (from braced literals like `{$x}`)
/// is left as-is.  Only the `${...}` form is treated as a resolvable
/// reference.
///
/// `style` is the release-aware `Tcl_ParseVarName` brace rule, resolved
/// through the shared owner [`tcl_lexer::braced_var_name_end`] rather than
/// re-scanned here. Walking brace *depth* and requiring the first balanced
/// close to be the final byte is the **9.x** rule; `helpers::parse_subst_template`
/// would then be reading the same encoding under the **8.x** first-`}` rule.
/// Two decoders reading one encoding under two different releases' rules
/// invert the outcome rather than merely getting it wrong.
///
/// The whole value must be the reference: a trailing byte after the name's
/// closer means this word is `${…}` followed by literal text, which is not a
/// simple variable load. Under 9.x that still admits nested references like
/// `${::a(${::a(1)})}`, because the nesting rule consumes the inner pairs.
///
/// An unterminated name yields `None`: there is no reference to load, and the
/// caller's fallback path re-reads the word. The shared owner reports
/// [`tcl_lexer::BracedVarEnd::Unterminated`] so no consumer has to invent its
/// own recovery.
///
/// Both wrong values of `style` are real defects — do not "simplify" this
/// parameter away in either direction.
///
/// Pinning it to `Tcl9Nesting` lets an 8.x compile accept `${a{b}c}` as one
/// reference to `a{b}c`; five tests fail.
///
/// Pinning it to `FirstClose` is **not** merely pessimal, which an earlier
/// revision of this comment claimed. Declining here forfeits the direct load
/// and hands the word to the runtime fallback, and that fallback is only
/// equivalent when the *name* survives ordinary word substitution unchanged.
/// It does not always:
///
/// ```tcl
/// set {{}} V
/// puts ${{}}          ;# 9.0: V   — with FirstClose: can't read "{}"
/// set {a\}b} K
/// set arr(K) V
/// puts $arr(${a\}b})  ;# 9.0: V   — with FirstClose: can't read "a"
/// ```
///
/// A leading `{` and an embedded `\}` both fail to round-trip through the
/// fallback, so declining loses the name rather than merely costing a fast
/// path. Declining is safe *only* where the fallback is equivalent, and these
/// are the counterexamples.
#[must_use]
pub fn parse_simple_var_ref(value: &str, style: tcl_dialect::BracedVarStyle) -> Option<&str> {
    let rest = value.strip_prefix("${")?;
    // `2` is the byte just past the `${`, i.e. where the name starts.
    match tcl_lexer::braced_var_name_end(value.as_bytes(), 2, style) {
        // The closer must be the final byte; anything after it makes the word
        // a concatenation rather than one whole reference.
        tcl_lexer::BracedVarEnd::Closed(end) if end == value.len() - 1 => Some(&rest[..end - 2]),
        _ => None,
    }
}

// There is deliberately no `$={name}` "braced scalar" marker decoded here.
//
// Nothing in this workspace produces that spelling — the segmenter re-spells a
// braced variable word verbatim from source (`${…}`, see
// `segmenter::word_piece`), and nothing writes a `$=` prefix — so every word
// reaching such a decoder would be the user's own text, in which `$={y}` is
// literal in every supported release:
//
// ```tcl
// set y hi
// puts $={y}     ;# 8.4-9.0: $={y}
// ```
//
// (`$` is only a substitution trigger before a name character, and `=` is not
// one; `Tcl_ParseVarName`, tmp/tcl9.0.4/generic/tclParse.c.) Decoding it would
// be wrong code: a whole-word `$={name}` would load the variable `name`
// instead of pushing the literal, and no real program could tell the two
// readings apart, so no corpus test would catch it.
//
// The form such a marker would spell, `${a(1)}`, reaches
// `parse_simple_var_ref` and `load_var`, which agree with both tclsh oracles
// (`set {a(1)} S; array set a {1 A}; puts ${a(1)}` → `A`, because
// `TclObjLookupVar` parses the parens out of the *name* at lookup time).
//
// Do not introduce a marker in the `$…` space: any spelling a user can type
// collides with real source. An internal marker needs an out-of-band channel
// (an IR node or a word flag), not a string prefix.

/// Check if a string is an integer literal (optionally negative).
fn is_integer_literal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{CodegenCtx, Op};
    use tcl_registry::CommandRegistry;

    #[allow(clippy::unnecessary_wraps)] // test callback follows ConstFoldFn
    fn owned_list_fold(args: &[&str]) -> Option<String> {
        Some(format!("owned:{}", args.join(",")))
    }

    #[test]
    fn original_c84_direct_source_preserves_words_without_compiler_pool_actions() {
        use tcl_runtime_api::{CompileService, ScriptCompileTargetBytes};
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let source = tcl_runtime_api::SourceImage::native(
            b"list [info level] [info exists local]".as_slice(),
        );
        let module = service
            .compile_plain_script_bytes_with_entry(
                ScriptCompileTargetBytes {
                    source: &source,
                    namespace: &tcl_runtime_api::ByteNamespacePath::root(),
                },
                profile,
                &entry,
            )
            .unwrap();
        let function = &module.top_level;
        assert!(function.plain_command_dispatch);
        assert!(function.validate_native_compilation_entry().is_ok());
        assert!(
            function
                .literals
                .native_actions()
                .iter()
                .all(|action| { matches!(action, tcl_bytecode::NativeLiteralAction::Register(_)) })
        );
        for head in [b"list".as_slice(), b"info"] {
            assert!(
                function
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == head)
            );
        }
        assert_eq!(
            function
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4)
                })
                .count(),
            3
        );
    }

    #[test]
    fn native_decoded_literals_are_exact_bytes_and_never_replayed_as_source() {
        let environment = tcl_registry::model::ingress::resolve_environment("jim");
        let context = environment.default_context_registry();
        let mut ctx = CodegenCtx::new(false, &[], context.commands());
        ctx.escapes = environment.analyser_profile().grammar.escapes;
        ctx.push_decoded_literal(r"\xff");
        ctx.push_decoded_literal(r"\u00ff");
        assert_eq!(ctx.literals.entries()[0].bytes(), &[0xff]);
        assert_eq!(ctx.literals.entries()[1].unicode(), Ok("ÿ"));
        assert!(
            ctx.instructions
                .iter()
                .all(|instruction| instruction.push_verbatim)
        );
    }

    #[test]
    fn value_fold_uses_the_owned_registry_callback_and_binding() {
        let mut registry = CommandRegistry::build_default();
        let mut list = registry.get("list").expect("list spec").clone();
        list.const_fold = Some(owned_list_fold);
        registry.insert(list);

        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(ctx.try_emit_constant_fold("[list a b]"));
        assert!(ctx.literals.entries().iter().any(|lit| lit == "owned:a,b"));
        assert_eq!(
            ctx.command_binding_requirements,
            [tcl_runtime_api::CommandBindingIdentity::new("list", "list")]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn value_fold_obeys_owned_registry_removal_for_every_old_fast_path() {
        let mut registry = CommandRegistry::build_default();

        let mut list = registry.get("list").expect("list spec").clone();
        list.const_fold = None;
        registry.insert(list);

        let mut format = registry.get("format").expect("format spec").clone();
        format.const_fold_versioned = None;
        registry.insert(format);

        let mut dict = registry.get("dict").expect("dict spec").clone();
        let mut subcommands = dict.subcommands.to_vec();
        subcommands
            .iter_mut()
            .find(|sub| sub.name == "create")
            .expect("dict create spec")
            .const_fold = None;
        dict.subcommands = Box::leak(subcommands.into_boxed_slice());
        registry.insert(dict);

        for value in [
            "[list a b]",
            "[format {%s} value]",
            "[dict create key value]",
        ] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            assert!(
                !ctx.try_emit_constant_fold(value),
                "owned registry disabled the fold for {value}"
            );
            assert!(ctx.instructions.is_empty(), "partial emission for {value}");
        }
    }

    #[test]
    fn nested_value_fold_retains_every_resolved_command_binding() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(ctx.try_emit_constant_fold("[llength [list a b]]"));
        for name in ["list", "llength"] {
            assert!(
                ctx.command_binding_requirements
                    .contains(&tcl_runtime_api::CommandBindingIdentity::new(name, name)),
                "missing {name} dependency: {:?}",
                ctx.command_binding_requirements
            );
        }
    }

    #[test]
    fn consumed_value_fold_retains_an_executable_replay_boundary() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        assert!(ctx.try_emit_constant_fold("[list a b]"));

        let start = &ctx.instructions[0];
        assert_eq!(start.op, Op::START_CMD);
        assert_eq!(start.source_cmd_text, "list a b");
        assert_eq!(
            start.source_command_boundary,
            super::super::SourceCommandBoundary::InlineReplay,
            "an inline replay point is not a new outer source-command owner",
        );
        let end = match start.operands.first() {
            Some(Operand::Label(label)) => label,
            other => panic!("fold boundary has no continuation: {other:?}"),
        };
        assert_eq!(ctx.label_positions.get(end), Some(&ctx.instructions.len()));
        assert!(
            ctx.instructions
                .iter()
                .find(|instruction| matches!(instruction.op, Op::PUSH1 | Op::PUSH4))
                .expect("folded literal push")
                .push_verbatim,
            "a folded command result is already a final Tcl value",
        );
    }

    #[test]
    fn folded_dict_payload_is_verbatim_before_verification() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(ctx.try_emit_constant_fold("[dict create {[missing]} {${value}}]"));

        let push = ctx
            .instructions
            .iter()
            .find(|instruction| matches!(instruction.op, Op::PUSH1 | Op::PUSH4))
            .expect("dict payload push");
        assert!(push.push_verbatim);
        assert!(
            ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::VERIFY_DICT)
        );
    }

    // split_array_ref.

    #[test]
    fn split_array_ref_basic() {
        assert_eq!(split_array_ref("arr(key)"), Some(("arr", "key")));
    }

    #[test]
    fn split_array_ref_no_parens() {
        assert_eq!(split_array_ref("scalar"), None);
    }

    #[test]
    fn split_array_ref_nested() {
        assert_eq!(split_array_ref("arr(${inner})"), Some(("arr", "${inner}")));
    }

    /// The owner's edge cases hold through this facade:
    /// both halves may be empty, and a `(` with nothing closing it is not a
    /// reference. A local re-spelling that adds a "base must be non-empty"
    /// test would silently demote `set (x) 5` to a scalar.
    #[test]
    fn split_array_ref_matches_the_owners_edges() {
        assert_eq!(split_array_ref("(x)"), Some(("", "x")));
        assert_eq!(split_array_ref("arr()"), Some(("arr", "")));
        assert_eq!(split_array_ref("()"), Some(("", "")));
        assert_eq!(split_array_ref(")"), None);
        assert_eq!(split_array_ref("a(b"), None);
        assert!(is_array_ref("(x)"));
    }

    // is_array_ref, is_qualified.

    #[test]
    fn is_array_ref_yes() {
        assert!(is_array_ref("a(1)"));
    }

    #[test]
    fn is_array_ref_no() {
        assert!(!is_array_ref("x"));
    }

    #[test]
    fn is_qualified_yes() {
        assert!(is_qualified("::foo"));
        assert!(is_qualified("n::foo"));
        assert!(is_qualified("k\0n::foo"));
    }

    #[test]
    fn is_qualified_no() {
        assert!(!is_qualified("foo"));
        assert!(!is_qualified("a(k::z)"));
    }

    // parse_simple_var_ref.

    #[test]
    fn parse_simple_var_ref_basic() {
        assert_eq!(
            parse_simple_var_ref("${x}", tcl_dialect::BracedVarStyle::default()),
            Some("x")
        );
    }

    #[test]
    fn parse_simple_var_ref_qualified() {
        assert_eq!(
            parse_simple_var_ref("${::foo}", tcl_dialect::BracedVarStyle::default()),
            Some("::foo")
        );
    }

    #[test]
    fn parse_simple_var_ref_nested() {
        assert_eq!(
            parse_simple_var_ref("${::a(${::a(1)})}", tcl_dialect::BracedVarStyle::default()),
            Some("::a(${::a(1)})")
        );
    }

    #[test]
    fn parse_simple_var_ref_bare_dollar() {
        assert_eq!(
            parse_simple_var_ref("$x", tcl_dialect::BracedVarStyle::default()),
            None
        );
    }

    #[test]
    fn parse_simple_var_ref_no_close() {
        assert_eq!(
            parse_simple_var_ref("${x", tcl_dialect::BracedVarStyle::default()),
            None
        );
    }

    // The `$={name}` spelling is not a marker.

    /// A whole word spelt `$={name}` is the *user's* literal text — `=` is not
    /// a name character, so `Tcl_ParseVarName` never starts a substitution
    /// there and both tclsh oracles print `$={y}` for `puts $={y}`. Treating
    /// it as a "braced scalar" marker would compile it to
    /// `push "y"; loadStk`, silently reading a variable.
    #[test]
    fn dollar_equals_word_is_a_literal_not_a_variable_load() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["y"], &registry);
        ctx.emit_value("$={y}", true);
        assert_eq!(
            ctx.instructions.iter().map(|i| i.op).collect::<Vec<_>>(),
            vec![Op::PUSH1],
            "`$={{y}}` must be pushed whole, not decoded as a variable load"
        );
        assert!(ctx.literals.entries().iter().any(|l| l == "$={y}"));
        // The real braced form still loads.
        let mut ctx = CodegenCtx::new(true, &["y"], &registry);
        ctx.emit_value("${y}", true);
        assert_eq!(
            ctx.instructions.iter().map(|i| i.op).collect::<Vec<_>>(),
            vec![Op::LOAD_SCALAR1],
        );
    }

    // is_integer_literal.

    #[test]
    fn integer_literal_positive() {
        assert!(is_integer_literal("42"));
    }

    #[test]
    fn integer_literal_negative() {
        assert!(is_integer_literal("-7"));
    }

    #[test]
    fn integer_literal_non_integer() {
        assert!(!is_integer_literal("abc"));
        assert!(!is_integer_literal(""));
    }

    // CodegenCtx value emission.

    #[test]
    fn push_lit_dedup() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.push_lit("hello");
        ctx.push_lit("hello"); // dedup
        assert_eq!(ctx.literals.len(), 1);
        assert_eq!(ctx.instructions.len(), 2);
        // Both should reference index 0
        assert_eq!(
            ctx.instructions[0].operands[0],
            super::super::Operand::Imm(0)
        );
        assert_eq!(
            ctx.instructions[1].operands[0],
            super::super::Operand::Imm(0)
        );
    }

    #[test]
    fn push_lit_no_dedup_creates_fresh_slot() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.push_lit_no_dedup("x");
        ctx.push_lit_no_dedup("x");
        assert_eq!(ctx.literals.len(), 2); // two distinct slots
    }

    #[test]
    fn original_variable_operands_keep_literal_keys_and_evaluate_bare_indices() {
        let registry = CommandRegistry::build_default();
        for is_proc in [false, true] {
            let mut literal = CodegenCtx::new(is_proc, &[], &registry);
            assert!(literal.emit_variable_reference("${arr($i)}"));
            let mut dynamic = CodegenCtx::new(is_proc, &[], &registry);
            assert!(dynamic.emit_variable_reference("$arr($i)"));
            assert_eq!(
                literal
                    .instructions
                    .iter()
                    .map(|instruction| instruction.op)
                    .collect::<Vec<_>>(),
                vec![Op::PUSH1, Op::LOAD_STK]
            );
            assert_eq!(
                dynamic.instructions.last().unwrap().op,
                if is_proc {
                    Op::LOAD_ARRAY1
                } else {
                    Op::LOAD_ARRAY_STK
                }
            );
            assert!(
                literal
                    .literals
                    .entries()
                    .iter()
                    .any(|value| value == "arr($i)")
            );
            assert!(!literal.literals.entries().iter().any(|value| value == "$i"));
            assert!(!dynamic.literals.entries().iter().any(|value| value == "$i"));
            assert!(
                dynamic
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction.op, Op::LOAD_SCALAR1 | Op::LOAD_STK))
            );
            let mut compound = CodegenCtx::new(is_proc, &[], &registry);
            assert!(!compound.emit_variable_reference("$arr(k)suffix"));
            assert!(compound.instructions.is_empty());
        }
    }

    #[test]
    fn load_var_scalar_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.load_var("x");
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].op, Op::LOAD_SCALAR1);
        assert_eq!(
            ctx.instructions[0].operands[0],
            super::super::Operand::Imm(0)
        );
    }

    #[test]
    fn load_var_scalar_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.load_var("x");
        assert_eq!(ctx.instructions.len(), 2); // push name + loadStk
        assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "x"
        assert_eq!(ctx.instructions[1].op, Op::LOAD_STK);
    }

    #[test]
    fn load_var_array_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.load_var("arr(key)");
        // Should intern "arr", push_lit "key", then LOAD_ARRAY1
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY1);
    }

    #[test]
    fn load_var_array_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.load_var("arr(key)");
        // push "arr", push "key", LOAD_ARRAY_STK
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY_STK);
    }

    #[test]
    fn load_var_qualified_in_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.load_var("::global_var");
        // Qualified vars always use stack-based ops
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_STK);
    }

    #[test]
    fn store_var_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.store_var("x");
        assert_eq!(ctx.instructions[0].op, Op::STORE_SCALAR1);
    }

    #[test]
    fn store_var_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.store_var("x");
        assert_eq!(ctx.instructions[0].op, Op::STORE_STK);
    }

    fn increment_entry(
        version: tcl_dialect::TclVersion,
    ) -> tcl_runtime_api::NativeCompilationEntry {
        use tcl_runtime_api::native_compilation::{
            NativeCompilationFrame, NativeInterpreterIdentity, NativeVariableObserverPresence,
        };
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        let point = tcl_dialect::model::DialectPoint::for_tcl_version(version);
        let dialect = tcl_registry::InvocationDialect::of_point(point);
        tcl_runtime_api::NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: 1,
                interpreter: 0,
            },
            epoch: 0,
            profile: profile.cache_key(),
            invocation_policy: Some(profile.cache_key()),
            expression_policy:
                tcl_registry::native_expression_program::native_expression_evaluation_policy(
                    profile, point,
                ),
            execution_point: Some(point),
            name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
            compiled_variable_protocol:
                tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
            compiled_local_layout: None,
            ensemble_target_objects: None,
            source_string_protocol: dialect.native_source_string_protocol(),
            lexer_grammar: Some(profile.grammar),
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            empty_literal_world: None,
            compiler_pass_environment: None,
            variable_observers: NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: true,
            commands: vec![],
            namespaces: vec![],
            current_namespace: 0,
            frame: NativeCompilationFrame::Global,
        }
    }

    fn increment_tokens(
        source: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> crate::ir::CommandTokens {
        let segment =
            crate::segmenter::segment_commands_image_with_offset_and_config(source, 0, config)
                .unwrap()
                .remove(0);
        crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(source),
            config,
            &segment,
        )
    }

    #[test]
    fn original_variable_projection_keeps_the_final_substitution_closer() {
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for source in [
            "lappend ::events [list $n $i $op]",
            "lappend \"arr($i)\" [list $n]",
            "lappend {arr($i)} [list $n]",
        ] {
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let tokens = increment_tokens(&image, config);
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.source = image;
            ctx.native_entry = Some(&entry);
            ctx.source_string_protocol = entry.source_string_protocol;
            ctx.invocation_tokens = Some(Box::new(tokens.clone()));
            let original = ctx.original_compiler_words().unwrap();
            assert_eq!(original.words.len(), 3, "{source}");
            let variable = ctx.original_variable_operand(&tokens.words()[1]).unwrap();
            match (source, variable) {
                (
                    "lappend ::events [list $n $i $op]",
                    NativeVariableWordOperand::Literal { name, index, .. },
                ) => {
                    assert_eq!(name, b"::events");
                    assert_eq!(index, None);
                }
                (
                    "lappend \"arr($i)\" [list $n]",
                    NativeVariableWordOperand::CompoundArray { name, index, .. },
                ) => {
                    assert_eq!(name, b"arr");
                    assert_eq!(index.image().bytes(), original.words[1].image().bytes());
                }
                ("lappend {arr($i)} [list $n]", NativeVariableWordOperand::DynamicWord) => {}
                (source, variable) => panic!("{source}: actual C84 variable layout {variable:?}"),
            }
            let changed = increment_tokens(
                &tcl_lexer::SourceImage::native(b"lappend other [list $n]".as_slice()),
                config,
            );
            assert!(ctx.original_variable_operand(&changed.words()[1]).is_none());
            ctx.source_string_protocol = None;
            assert!(ctx.original_variable_operand(&tokens.words()[1]).is_none());
        }
    }

    #[test]
    fn named_unit_dependencies_require_the_original_entry_world() {
        use std::sync::Arc;
        use tcl_runtime_api::native_compilation::{
            NativeCommandCompiler, NativeCommandCompilerPrerequisite, NativeCommandImplementation,
            NativeCompilationBinding, NativeCompilationNamespace, NativeCompilerHookPresence,
            NativeCompilerSelectionPrerequisite, NativeEnsembleCompiler,
        };
        let registry = CommandRegistry::build_default();
        let mut entry = increment_entry(tcl_dialect::TclVersion::V9_0);
        entry.namespaces.push(NativeCompilationNamespace {
            path: tcl_core_types::ByteNamespacePath::root(),
            jim_namespace_object: None,
            token: 0,
            visible: true,
            exports: vec![],
            command_path: vec![],
            unknown_handler: None,
        });
        let compiler = NativeCommandCompiler {
            registry_identity: "info".to_owned(),
            ensemble: Some(NativeEnsembleCompiler {
                namespace_token: 0,
                map: vec![],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            }),
        };
        let binding = NativeCompilationBinding {
            slot: tcl_core_types::NativeByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                "info".into(),
            ),
            namespace_token: 0,
            token: 7,
            implementation_generation: 11,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: NativeCompilerHookPresence::Present,
            compiler: Some(compiler.clone()),
            procedure_header: None,
            has_execution_trace: false,
        };
        entry.commands.push(binding.clone());
        let required = NativeCommandCompilerPrerequisite {
            interpreter: entry.interpreter,
            lookup_namespace_token: 0,
            invocation_word: "info".into(),
            slot: binding.slot.clone(),
            namespace_token: 0,
            token: binding.token,
            implementation_generation: binding.implementation_generation,
            compiler,
            selected_worker: None,
            nested_compilers: Vec::new(),
            guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
        };
        for changed_axis in 0..6 {
            let mut selected = required.clone();
            match changed_axis {
                0 => {}
                1 => selected.interpreter.owner += 1,
                2 => selected.lookup_namespace_token += 1,
                3 => selected.token += 1,
                4 => selected.implementation_generation += 1,
                5 => selected.compiler.ensemble.as_mut().unwrap().prefixes = false,
                _ => unreachable!(),
            }
            let selected = Arc::new(selected);
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            ctx.native_entry = Some(&entry);
            ctx.retain_entry_named_compiler_prerequisite(&selected);
            if changed_axis == 0 {
                let mut entry_required = selected.as_ref().clone();
                entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
                assert_eq!(
                    ctx.native_compiler_prerequisites,
                    vec![NativeCompilerSelectionPrerequisite::Ensemble(Arc::new(
                        entry_required
                    ))]
                );
            } else {
                assert!(
                    ctx.native_compiler_prerequisites.is_empty(),
                    "axis {changed_axis}"
                );
            }
            // Every original selection retains its own temporal guard even
            // when a future or foreign world cannot become a unit dependency.
            assert_eq!(
                selected.guard,
                tcl_runtime_api::CommandBindingGuard::BeforeArguments
            );
        }
        entry.commands[0].has_execution_trace = true;
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.native_entry = Some(&entry);
        ctx.retain_entry_named_compiler_prerequisite(&Arc::new(required));
        assert!(ctx.native_compiler_prerequisites.is_empty());
    }

    #[test]
    fn native_command_literal_actions_retain_data_first_and_hide_only_c85_one_word() {
        use tcl_bytecode::{NativeLiteralAction, NativeLiteralAllocation};
        use tcl_runtime_api::native_compilation::NativeCompilationNamespace;
        let registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            for source in ["missing", "missing argument"] {
                let mut entry = increment_entry(version);
                entry.namespaces.push(NativeCompilationNamespace {
                    path: tcl_core_types::ByteNamespacePath::root(),
                    jim_namespace_object: None,
                    token: 0,
                    visible: true,
                    exports: vec![],
                    command_path: vec![],
                    unknown_handler: None,
                });
                let image = tcl_lexer::SourceImage::native(source.as_bytes());
                let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
                let tokens = tcl_lexer::Lexer::with_source_image(&image, config)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                let words = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
                let native_words = words[0]
                    .words
                    .iter()
                    .map(|word| {
                        tcl_lexer::NativeWord::from_group(image.clone(), config, &tokens, word)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                let mut ctx = CodegenCtx::new(false, &[], &registry);
                ctx.native_entry = Some(&entry);
                let data = ctx.literals.intern_bytes(b"missing");
                let command = ctx
                    .intern_original_native_command_literal(&native_words)
                    .unwrap();
                assert_eq!(command, data, "{version:?} {source}");
                assert_eq!(
                    ctx.literals.entries()[data].allocation(),
                    &NativeLiteralAllocation::RegisteredData
                );
                assert_eq!(
                    ctx.literals.native_actions(),
                    if version == tcl_dialect::TclVersion::V8_5 && source == "missing" {
                        vec![
                            NativeLiteralAction::Register(data),
                            NativeLiteralAction::Hide(data),
                        ]
                    } else {
                        vec![NativeLiteralAction::Register(data)]
                    }
                );
                assert!(!ctx.native_dependency_refusal);
            }
        }
    }

    #[test]
    fn source_increment_uses_physical_amount_recipe_and_original_array_layout() {
        let registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            for (source, immediate) in [
                ("incr arr($key) -127", Some(-127)),
                ("incr arr($key) -128", None),
                (
                    "incr arr($key) 4294967295",
                    (version <= tcl_dialect::TclVersion::V8_5).then_some(-1),
                ),
                (
                    "incr arr($key) \\x31",
                    (version >= tcl_dialect::TclVersion::V9_1).then_some(1),
                ),
            ] {
                let entry = increment_entry(version);
                let image = tcl_lexer::SourceImage::native(source.as_bytes());
                let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
                let tokens = increment_tokens(&image, config);
                let mut ctx = CodegenCtx::new(true, &["key"], &registry);
                ctx.native_entry = Some(&entry);
                ctx.dialect = tcl_dialect::DialectProfile::find("tcl8.4");
                ctx.ingress_lexer_config = Some(config);
                ctx.source_string_protocol = entry.source_string_protocol;
                ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
                ctx.set_source_image(image);
                ctx.with_invocation_tokens(Some(&tokens), |ctx| {
                    ctx.emit_incr("arr($key)", false, Some("ignored"));
                });
                assert_eq!(
                    ctx.instructions.last().unwrap().op,
                    if immediate.is_some() {
                        Op::INCR_ARRAY1_IMM
                    } else {
                        Op::INCR_ARRAY1
                    },
                    "{version:?} {source}"
                );
                if let Some(value) = immediate {
                    assert_eq!(
                        ctx.instructions.last().unwrap().operands.last(),
                        Some(&Operand::Imm(value))
                    );
                }
                assert_eq!(
                    ctx.instructions
                        .iter()
                        .filter(|instruction| instruction.op == Op::LOAD_SCALAR1)
                        .count(),
                    1
                );
                assert!(!ctx.native_dependency_refusal);
            }
        }
    }

    #[test]
    fn original_scalar_operand_slot_borrows_only_current_existing_layout_names() {
        use tcl_runtime_api::native_compilation::{
            NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
        };
        let registry = CommandRegistry::build_default();
        let entry = increment_entry(tcl_dialect::TclVersion::V9_0);
        let layout = NativeCompiledLocalLayout {
            owner: NativeInterpreterIdentity {
                owner: 1,
                interpreter: 0,
            },
            token: 7,
            epoch: 1,
            kind: NativeCompiledLocalLayoutKind::Procedure,
            names: vec![Some(tcl_runtime_api::NameBytes::from(
                b"retained".as_slice(),
            ))],
        };
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.native_entry = Some(&entry);
        ctx.source_string_protocol = entry.source_string_protocol;
        ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
        ctx.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
        ctx.lvt
            .set_native_protocol(entry.compiled_variable_protocol);
        ctx.borrowed_local_layout = Some(layout.clone());
        let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
        ctx.ingress_lexer_config = Some(config);
        for (source, expected) in [
            ("info exists retained", Some(0)),
            ("info exists missing", None),
            ("info exists retained($key)", None),
            ("info exists ret\\x61ined", None),
        ] {
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let tokens = increment_tokens(&image, config);
            let word = tokens.words()[2].clone();
            ctx.set_source_image(image);
            let slot = ctx.with_invocation_tokens(Some(&tokens), |ctx| {
                ctx.original_scalar_operand_slot(&word)
            });
            assert_eq!(slot, expected, "{source}");
            assert_eq!(ctx.lvt.len(), 1);
        }
        assert_eq!(ctx.required_compiled_local_layout, Some(layout));
    }

    #[test]
    fn unlocated_increment_amount_is_an_evaluated_value() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_incr("x", true, Some("${amount}"));
        assert_eq!(ctx.instructions.last().unwrap().op, Op::INCR_SCALAR1);
        assert!(ctx.instructions.iter().all(|instruction| !matches!(
            instruction.op,
            Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 | Op::LOAD_STK
        )));
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == "${amount}")
        );
    }

    #[test]
    fn supplied_unknown_compiler_point_cannot_borrow_authoring_increment_rules() {
        let registry = CommandRegistry::build_default();
        let mut entry = increment_entry(tcl_dialect::TclVersion::V9_0);
        entry.execution_point = None;
        let image = tcl_lexer::SourceImage::native(b"incr x 5".as_slice());
        let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
        let tokens = increment_tokens(&image, config);
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.native_entry = Some(&entry);
        ctx.source_string_protocol = entry.source_string_protocol;
        ctx.ingress_lexer_config = Some(config);
        ctx.set_source_image(image);
        ctx.with_invocation_tokens(Some(&tokens), |ctx| ctx.emit_incr("x", true, Some("5")));
        assert!(ctx.native_dependency_refusal);
        assert_eq!(ctx.instructions.last().unwrap().op, Op::INCR_SCALAR1);
    }

    #[test]
    fn emit_incr_default_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_incr("x", true, None);
        assert_eq!(ctx.instructions[0].op, Op::INCR_SCALAR1_IMM);
        // operands: slot 0, imm 1
        assert_eq!(
            ctx.instructions[0].operands[1],
            super::super::Operand::Imm(1)
        );
    }

    #[test]
    fn emit_incr_literal_amount_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_incr("x", true, Some("5"));
        assert_eq!(ctx.instructions[0].op, Op::INCR_SCALAR1_IMM);
        assert_eq!(
            ctx.instructions[0].operands[1],
            super::super::Operand::Imm(5)
        );
    }

    #[test]
    fn emit_incr_large_amount_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_incr("x", true, Some("999"));
        // Large amount → push_lit + INCR_SCALAR1
        assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "999"
        assert_eq!(ctx.instructions[1].op, Op::INCR_SCALAR1);
    }

    #[test]
    fn emit_incr_default_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_incr("x", true, None);
        // push "x" then INCR_STK_IMM
        assert_eq!(ctx.instructions[0].op, Op::PUSH1);
        assert_eq!(ctx.instructions[1].op, Op::INCR_STK_IMM);
    }

    #[test]
    fn emit_incr_large_amount_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_incr("x", true, Some("999"));
        assert_eq!(
            ctx.instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect::<Vec<_>>(),
            vec![Op::PUSH1, Op::PUSH1, Op::INCR_STK]
        );
        assert_eq!(ctx.literals.entries()[0].bytes(), b"x");
        assert_eq!(ctx.literals.entries()[1].bytes(), b"999");
    }

    #[test]
    fn begin_end_command() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // First command — no startCommand emitted
        ctx.begin_command(1);
        assert!(ctx.start_cmd_end_label.is_none());
        assert_eq!(ctx.cmd_index, 1);
        ctx.end_command();

        // Second command — startCommand emitted
        ctx.begin_command(1);
        assert!(ctx.start_cmd_end_label.is_some());
        assert_eq!(ctx.cmd_index, 2);
        assert_eq!(ctx.instructions[0].op, Op::START_CMD);
        ctx.end_command();
    }

    #[test]
    fn resolved_targets_prepare_slot_and_stack_operands() {
        let registry = CommandRegistry::build_default();
        let mut top = CodegenCtx::new(false, &[], &registry);
        assert!(top.target_needs_stack("x"));
        let mut procedure = CodegenCtx::new(true, &[], &registry);
        assert!(!procedure.target_needs_stack("x"));
        assert!(procedure.target_needs_stack("::x"));
        assert!(procedure.target_needs_stack("a(1)"));
    }

    #[test]
    fn borrowed_native_commands_reuse_cells_without_declaring_slots() {
        use tcl_runtime_api::native_compilation::{
            NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
        };
        let registry = CommandRegistry::build_default();
        for (point, borrows) in [
            (
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_4),
                false,
            ),
            (
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_5),
                false,
            ),
            (
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_6),
                true,
            ),
            (
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_0),
                true,
            ),
            (
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_1),
                true,
            ),
        ] {
            let protocol =
                tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point)
                    .unwrap();
            let names = vec![Some(tcl_runtime_api::NameBytes::from(b"k\0a".as_slice()))];
            let layout = NativeCompiledLocalLayout {
                owner: NativeInterpreterIdentity {
                    owner: 23,
                    interpreter: 5,
                },
                token: 7,
                epoch: 1,
                kind: NativeCompiledLocalLayoutKind::Procedure,
                names,
            };
            let mut context = CodegenCtx::new(false, &[], &registry);
            context.compiled_variable_protocol = Some(protocol);
            context.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
            context.lvt.set_native_protocol(Some(protocol));
            context.borrowed_local_layout = Some(layout.clone());
            assert_eq!(context.target_needs_stack("k\0b"), !borrows);
            if !borrows {
                context.push_var_ref("k\0b", true);
            }
            context.push_lit_exact("ALTER");
            context.store_var("k\0b");
            assert_eq!(
                context.instructions.last().unwrap().op,
                if borrows {
                    Op::STORE_SCALAR1
                } else {
                    Op::STORE_STK
                }
            );
            assert!(context.target_needs_stack("missing"));
            assert_eq!(context.lvt.len(), 1);
            let assembly = context.into_function_asm("borrowed".into());
            assert_eq!(
                assembly.required_compiled_local_layout,
                borrows.then_some(layout)
            );
        }
    }
    #[test]
    fn full_depth_text_indices_use_the_same_iterative_operand_emitter() {
        let registry = CommandRegistry::build_default();
        let source = include_str!("../../tests/data/native_deep_array_source.tcl");
        let operand = source.strip_prefix("set a(x) x; list ").unwrap();
        let mut context = CodegenCtx::new(true, &[], &registry);
        assert!(context.emit_variable_reference(operand));
        assert_eq!(
            context
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::LOAD_ARRAY1)
                .count(),
            2000,
        );
        assert_eq!(context.lvt.len(), 1);
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|entry| entry.bytes() == b"x")
        );
    }

    #[test]
    fn source_local_loads_select_counted_compiler_purposes_before_runtime_names() {
        use tcl_syntax::naming::NativeCompiledVariableProtocol;

        let registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let mut context = CodegenCtx::new(true, &[], &registry);
            let protocol = NativeCompiledVariableProtocol::for_native_point(
                tcl_dialect::model::DialectPoint::for_tcl_version(version),
            )
            .unwrap();
            context.compiled_variable_protocol = Some(protocol);
            context.lvt.set_native_protocol(Some(protocol));
            for original in [b"k\0a".as_slice(), b"k\0b".as_slice()] {
                let (operation, operands) = context.prepare_original_variable_load(original, false);
                assert_eq!(operation, Op::LOAD_SCALAR1);
                assert_eq!(operands, vec![Operand::Imm(0)]);
            }
            assert_eq!(context.lvt.len(), 1);
            let (operation, operands) = context.prepare_original_variable_load(b"k\0bb", false);
            assert_eq!(operation, Op::LOAD_SCALAR1);
            assert_eq!(operands, vec![Operand::Imm(1)]);
            for original in [b"k\0z::x".as_slice(), b"a(k::z)".as_slice()] {
                let (operation, operands) = context.prepare_original_variable_load(original, false);
                assert_eq!(operation, Op::LOAD_STK);
                assert!(operands.is_empty());
                assert!(
                    context
                        .literals
                        .entries()
                        .iter()
                        .any(|entry| entry.bytes() == original)
                );
            }
            assert_eq!(context.lvt.len(), 2);
        }
    }
}
