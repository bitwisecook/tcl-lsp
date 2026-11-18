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

use std::rc::Rc;

use super::format::esc;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

struct OriginalCompilerWords {
    words: Vec<tcl_lexer::NativeWord>,
    version: tcl_dialect::TclVersion,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
}

/// Syntax derived only within one immutable original invocation scope.
/// The retained image keeps its allocation alive; equality of text is not a
/// cache key or a grant of command, object or compiler authority.
#[derive(Debug)]
pub(super) struct OriginalLexicalWords {
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    words: Rc<Vec<tcl_lexer::NativeWord>>,
}

impl OriginalLexicalWords {
    pub(super) fn matches_source(&self, image: &tcl_lexer::SourceImage) -> bool {
        self.image.bytes().as_ptr() == image.bytes().as_ptr()
            && self.image.len() == image.len()
            && self.image.channel() == image.channel()
    }

    fn matches(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> bool {
        self.matches_source(image) && self.config == config && self.protocol == protocol
    }
}

// Literal emission.

impl CodegenCtx<'_> {
    pub(super) fn push_native_pool_literal(&mut self, index: usize) {
        let instruction = self.emit(
            if index < 256 { Op::PUSH1 } else { Op::PUSH4 },
            vec![Operand::Imm(super::bytecode_imm(index))],
        );
        self.instructions[instruction].push_verbatim = true;
        self.instructions[instruction].literal_rep = tcl_bytecode::LiteralRep::Verbatim;
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
            || entry.command_name_policy()?.authority()
                != tcl_syntax::naming::NamePolicyAuthority::Native
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
        tokens: &tcl_compiler::ir::CommandTokens,
        named: &tcl_compiler::command_binding::SourceNamedInvocationProof,
    ) {
        let Some(entry) = self.native_entry else {
            self.push_lit_exact(named.captured_name());
            return;
        };
        if entry
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
            .is_none()
            || entry.command_name_policy().is_none_or(|protocol| {
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
            || tcl_compiler::registry_invocation::native_compiler_replay_source(
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
            match (entry.execution_point, entry.command_name_policy(), namespace) {
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
            if let Ok(tcl_compiler::native_byte_compilation::NativeByteCommandPlan::Registered(
                selected,
            )) = tcl_compiler::native_byte_compilation::native_byte_command_plan(
                &words,
                entry,
                self.registry,
                self.native_compilation,
            ) && selected.spec.grammar
                == tcl_registry::native_compilation::NativeCompilationGrammar::ArgumentList
            {
                let start = self.instructions.len();
                self.emit_native_words(&words);
                self.stamp_substitution_error_info(start, value.trim(), None);
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
                tcl_compiler::const_subst::ConstSubstCtx {
                    registry: self.registry,
                    resolution_namespace: namespace,
                    namespace_context: self
                        .invocation_tokens
                        .as_deref()
                        .and_then(tcl_compiler::registry_invocation::compiled_namespace_context),
                    version: self
                        .dialect
                        .and_then(tcl_dialect::DialectProfile::const_fold_version),
                    defining_class: None,
                    trusts: &trusts,
                    lookup_var: &lookup,
                    script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
                }
                .fold_cmd_subst_resolved(inner)
            })
            .filter(|fold| {
                // Only a list literal keeps a list result's reading as a
                // script; any other literal leaves such a result to its
                // command.
                fold.inline_codegen_hook == Some(tcl_registry::hooks::InlineCodegenHookId::List)
                    || !tcl_compiler::const_subst::folded_literal_reads_apart(
                        &fold.value,
                        self.vm_release(),
                    )
            });
        if let Some(fold) = fold {
            for invocation in &fold.invocations {
                self.require_invocation_binding(&invocation.binding, &invocation.shape);
            }
            for claim in &fold.site_claims {
                self.require_site_claim(claim);
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
            } else if fold.inline_codegen_hook
                == Some(tcl_registry::hooks::InlineCodegenHookId::List)
                && !fold.value.is_empty()
            {
                // `TclCompileListCmd` pushes the list it builds from literal
                // words as its list object (and `[list]` as an empty string):
                // a canonical list, which a script evaluation reads without
                // compiling.
                self.push_lit_no_dedup_verbatim(&fold.value);
                if let Some(push) = self.instructions.last_mut() {
                    push.literal_rep = tcl_bytecode::LiteralRep::List;
                }
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
        // The command substitutions of a word substituted at run time are
        // scripts C compiles with the command, their locals the procedure's.
        if self.compiles_locals()
            && value.contains('[')
            && let Some(parts) = super::helpers::parse_subst_template(value, self.lexer_config())
        {
            for part in parts {
                if let super::helpers::SubstPart::Cmd(script) = part {
                    self.intern_script_locals(&script);
                }
            }
        }
        let idx = self.literals.intern(value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", esc(value, 40)),
        );
    }

    /// Host-rendered literal text in the selected native spelling: C stores
    /// a NUL character as `C0 80`. A raw NUL written in the source stays raw,
    /// so this applies only to text a host fold produced.
    fn native_literal_bytes<'v>(&self, value: &'v [u8]) -> std::borrow::Cow<'v, [u8]> {
        match self.source_string_protocol {
            Some(protocol) => protocol.host_decoded_bytes(value),
            None => std::borrow::Cow::Borrowed(value),
        }
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
        self.instructions[pos].literal_rep = tcl_bytecode::LiteralRep::Verbatim;
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
        self.instructions[pos].literal_rep = tcl_bytecode::LiteralRep::Verbatim;
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
        self.instructions[pos].literal_rep = tcl_bytecode::LiteralRep::Verbatim;
    }

    /// Decode an authored literal fragment with the actual native escape policy.
    pub fn push_decoded_literal(&mut self, source: &str) {
        let bytes = match self.source_string_protocol {
            Some(protocol @ tcl_syntax::native_string::NativeStringProtocol::C(_)) => {
                tcl_syntax::backslash::native_source_string_bytes_in(
                    source.as_bytes(),
                    self.escapes,
                    protocol,
                )
                .unwrap_or_else(|_| {
                    tcl_lexer::backslash_subst_bytes_in(source.as_bytes(), self.escapes)
                })
            }
            _ => tcl_lexer::backslash_subst_bytes_in(source.as_bytes(), self.escapes),
        };
        self.push_lit_bytes_exact(&bytes);
    }

    /// Push normalized compiled-word text whose remaining substitutions belong
    /// to the runtime protocol. Bare dollar bytes are data; configured `${...}`,
    /// command and Jim expression markers retain the scanner. The shared policy
    /// is also used by the VM, so a Jim marker cannot be frozen by a C-only test.
    /// Finished values and decoded literal fragments use `push_lit_exact` or
    /// `push_lit_bytes_exact` directly and never enter this protocol.
    pub fn push_compiled_word(&mut self, value: &str) {
        match tcl_syntax::word_rules::compiled_word_substitution(
            value.as_bytes(),
            self.lexer_config(),
        ) {
            tcl_syntax::word_rules::CompiledWordSubstitution::LiteralValue => {
                self.push_lit_exact(value);
            }
            tcl_syntax::word_rules::CompiledWordSubstitution::Deferred => self.push_lit(value),
        }
    }

    /// Push a literal using a fresh slot (no deduplication).
    pub fn push_lit_no_dedup(&mut self, value: &str) {
        let idx = self
            .literals
            .register_bytes(&self.native_literal_bytes(value.as_bytes()));
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
        let idx = self
            .literals
            .register_bytes(&self.native_literal_bytes(value.as_bytes()));
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\" #nodedup", esc(value, 40)),
        );
        self.instructions[pos].push_verbatim = true;
        self.instructions[pos].literal_rep = tcl_bytecode::LiteralRep::Verbatim;
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
        if !tcl_syntax::naming::is_whole_variable_reference(spelling, self.lexer_config()) {
            return false;
        }
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
                    self.literal_scalar_stack(Op::LOAD_STK)
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

    /// Make the variable `name` a compiled local of the procedure, as a C
    /// compile procedure does for each variable word it compiles
    /// (`TclFindCompiledLocal`) even where it emits no access to it: the
    /// array an element names, nothing for a qualified name. A name whose
    /// word still holds a substitution or escape has no static local.
    pub(crate) fn intern_compiled_local(&mut self, name: &str) {
        if !self.compiles_locals() || is_qualified(name) || name.contains(['$', '[', '\\']) {
            return;
        }
        let base = split_array_ref(name).map_or(name, |(base, _)| base);
        self.lvt.intern(base);
    }

    /// Make the compiled locals of `script` locals of this procedure: the
    /// literal script of a command C compiles with its command, which this
    /// compiler leaves the command to run.
    pub(crate) fn intern_script_locals(&mut self, script: &str) {
        if !self.compiles_locals() {
            return;
        }
        let config = self.lexer_config();
        let key = (script.to_owned(), config.normalized());
        let retained = self.script_locals_memo.borrow().get(&key).cloned();
        let names = if let Some(names) = retained {
            names
        } else {
            let ir = tcl_compiler::lowering::lower_proc_body_module_for_bytecode(
                script,
                "",
                self.registry,
                config,
                self.dialect,
                false,
            );
            let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry_and_config(
                &ir,
                false,
                self.registry,
                config,
            );
            // The body's own compile shares this memo: its nested bodies'
            // locals are the same ones this compile's nested bodies intern.
            let mut nested = super::CodegenCtx::new(true, &[], self.registry);
            nested.script_locals_memo = std::rc::Rc::clone(&self.script_locals_memo);
            let asm = super::emitter::generate::generate(&mut nested, &cfg.top_level);
            // Native local names need not be Unicode; such a local has no
            // text spelling to intern here.
            let names: std::rc::Rc<[String]> = asm
                .lvt
                .entries()
                .iter()
                .filter(|name| asm.lvt.is_source_local_bytes(name.as_bytes()))
                .filter_map(|name| std::str::from_utf8(name.as_bytes()).ok())
                .map(str::to_owned)
                .collect();
            self.script_locals_memo
                .borrow_mut()
                .insert(key, std::rc::Rc::clone(&names));
            names
        };
        for name in names.iter() {
            self.intern_compiled_local(name);
        }
    }

    /// Make the compiled locals of the command C compiles in place of a
    /// generic surrogate ([`super::emitter::bytecoded::codegen_hook_locals`])
    /// locals of this procedure, as C's compile of that command does.
    pub(crate) fn intern_entered_command_locals(&mut self, command: &str, args: &[String]) {
        let Some(locals) = super::emitter::bytecoded::codegen_hook_locals(self, command, args)
        else {
            return;
        };
        for name in locals.entries() {
            // Native local names need not be Unicode; such a local has no
            // text spelling to intern here.
            if locals.is_source_local_bytes(name.as_bytes())
                && let Ok(name) = std::str::from_utf8(name.as_bytes())
            {
                self.intern_compiled_local(name);
            }
        }
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
            let op = self.literal_scalar_stack(Op::LOAD_STK);
            self.emit(op, vec![]);
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
            let op = if array {
                Op::STORE_ARRAY_STK
            } else {
                self.literal_scalar_stack(Op::STORE_STK)
            };
            self.emit(op, vec![]);
        }
    }

    /// Retain original lexical operands from their source owner. Original
    /// compiler rules use the physical entry; source values use their separate
    /// issuer. Unlocated/derived IR cannot manufacture lexical words.
    fn original_compiler_words(&self) -> Option<OriginalCompilerWords> {
        let tokens = self.invocation_tokens.as_deref()?;
        let version = if let Some(entry) = self.native_entry {
            entry.execution_point?.tcl_version()?
        } else if let Some(binding) = tokens.source_binding.as_ref() {
            binding.native_compiler_dialect()?.tcl_version?
        } else {
            tcl_compiler::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                self.lexer_config(),
            )
            .tcl_version?
        };
        let (words, protocol) = self.original_lexical_words()?;
        Some(OriginalCompilerWords {
            words: words.as_ref().clone(),
            version,
            protocol,
        })
    }

    /// Original lexical values use their retained source and String issuer.
    /// Reuse only syntax from this invocation scope, independently of Native
    /// object caches, command compilation and successful execution.
    fn original_lexical_words(
        &self,
    ) -> Option<(
        Rc<Vec<tcl_lexer::NativeWord>>,
        tcl_syntax::native_string::NativeStringProtocol,
    )> {
        let tokens = self.invocation_tokens.as_deref()?;
        if tokens.synthetic.is_some() || tokens.words().is_empty() {
            return None;
        }
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
            .map_or_else(|| self.source_image(), |site| site.source.source_image());
        let config = tokens.native_lexer_config(self.lexer_config());
        if let Some(cached) = self.original_lexical_words.borrow().as_ref()
            && cached.matches(image, config, protocol)
        {
            return Some((Rc::clone(&cached.words), protocol));
        }
        let words = Rc::new(
            tcl_compiler::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                tokens.words().first()?.source().span.start(),
                config,
            )?,
        );
        *self.original_lexical_words.borrow_mut() = Some(OriginalLexicalWords {
            image: image.clone(),
            config,
            protocol,
            words: Rc::clone(&words),
        });
        Some((words, protocol))
    }

    /// A source setter changes this projection only when the invocation has
    /// no independently retained original source owner.
    pub(super) fn invalidate_unbound_original_lexical_words(
        &mut self,
        image: &tcl_lexer::SourceImage,
    ) {
        let source_owned = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.invocation_site())
            .is_some();
        if !source_owned
            && self
                .original_lexical_words
                .get_mut()
                .as_ref()
                .is_some_and(|cached| !cached.matches_source(image))
        {
            self.original_lexical_words.get_mut().take();
        }
    }

    /// Emit a retained written operand with the same source-channel and escape
    /// rules as direct native source. Equal text alone does not locate a word.
    pub(super) fn try_emit_original_operand_word(
        &mut self,
        value: &str,
        word: &tcl_compiler::ir::WordExpr,
    ) -> bool {
        if value != word.legacy_text() {
            return false;
        }
        let Some(tokens) = self.invocation_tokens.as_deref() else {
            return false;
        };
        let Some(index) = tokens.words().iter().position(|original| original == word) else {
            return false;
        };
        let Some((words, _)) = self.original_lexical_words() else {
            return false;
        };
        let Some(original) = words.get(index) else {
            return false;
        };
        self.emit_original_native_word(original);
        true
    }

    /// Project only this operand's original C variable compiler layout. A
    /// synthetic/captured operand without a unique written slot stays unknown.
    pub(super) fn original_variable_operand(
        &self,
        word: &tcl_compiler::ir::WordExpr,
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

    /// Select only the original compiler's leading frame-level operand.
    /// Captured prefixes and expansions have no direct written layout here.
    pub(super) fn original_upvar_level_width(&self, argument_count: usize) -> Option<usize> {
        let original = self.original_compiler_words()?;
        let written = self.original_hook_argument(0)?;
        let first = written.checked_add(1)?;
        if first.checked_add(argument_count)? != original.words.len()
            || (0..argument_count).any(|argument| {
                self.original_hook_argument(argument) != written.checked_add(argument)
            })
            || original.words.iter().any(|word| word.group().expand)
        {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            &original.words,
            original.protocol,
        )
        .ok()?;
        let leading = words.literal(first)?;
        if leading.contains(&0) {
            return None;
        }
        tcl_registry::native_compilation::original_upvar_level_width(
            std::str::from_utf8(leading).ok()?,
            argument_count,
            original.version,
        )
        .ok()
    }

    /// A literal scalar operand alone can use a compiler local slot. Source
    /// array and computed words retain their own stack evaluation. Hand-built
    /// authoring contexts keep their distinct checked compatibility projection.
    pub(super) fn original_scalar_operand_slot(
        &mut self,
        word: &tcl_compiler::ir::WordExpr,
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
                tcl_compiler::registry_invocation::compiled_local_name_value(
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
        let literal_name = matches!(target, NativeVariableWordOperand::Literal { .. });
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
        self.finish_increment(slot, array, immediate, literal_name);
        true
    }

    /// C before 8.6 reaches a literal scalar name with no local slot through
    /// the scalar-specific stack forms (`TclPushVarName` isScalar): tclsh 8.5
    /// `disassemble script {set x 1}` stores with `storeScalarStk`.
    pub(crate) fn literal_scalar_stack(&self, op: Op) -> Op {
        let before_86 = self
            .native_entry
            .and_then(|entry| entry.execution_point?.tcl_version())
            .is_some_and(|version| version < tcl_dialect::TclVersion::V8_6);
        if !before_86 {
            return op;
        }
        match op {
            Op::LOAD_STK => Op::LOAD_SCALAR_STK,
            Op::STORE_STK => Op::STORE_SCALAR_STK,
            Op::INCR_STK => Op::INCR_SCALAR_STK,
            Op::INCR_STK_IMM => Op::INCR_SCALAR_STK_IMM,
            other => other,
        }
    }

    fn finish_increment(
        &mut self,
        slot: Option<usize>,
        array: bool,
        immediate: Option<i32>,
        literal_name: bool,
    ) {
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
        let op = if literal_name {
            self.literal_scalar_stack(op)
        } else {
            op
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
    ///
    /// `amount_braced` says the amount word was braced: its text is then its
    /// value (`incr x {$n}` raises `expected integer but got "$n"`).
    pub fn emit_incr(
        &mut self,
        name: &str,
        key_is_literal: bool,
        amount: Option<&str>,
        amount_braced: bool,
    ) {
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
            self.emit_increment_amount(
                amount.expect("non-immediate amount is present"),
                amount_braced,
            );
        }
        self.finish_increment(slot, key.is_some(), immediate, true);
    }

    /// Evaluate the original amount word once, retaining its brace semantics.
    /// A braced amount ([`Self::emit_incr`]'s `amount_braced`) is its text.
    fn emit_increment_amount(&mut self, amount: &str, amount_braced: bool) {
        let word = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.words().get(2))
            .cloned();
        if let Some(word) = word {
            let braced =
                amount_braced || matches!(word, tcl_compiler::ir::WordExpr::BracedLiteral { .. });
            self.emit_word_from_source(amount, braced, Some(&word));
        } else {
            // Unlocated IR amounts have already been evaluated. Their bytes
            // cannot establish a new command or variable substitution.
            self.push_lit_exact(amount);
        }
    }
}

/// Fixture projection for the shared braced-reference boundary assertions.
#[cfg(test)]
fn parse_simple_var_ref(value: &str, style: tcl_dialect::BracedVarStyle) -> Option<&str> {
    let (name, remainder) = tcl_syntax::naming::split_braced_var_ref(value, style)?;
    remainder.is_empty().then_some(name)
}

/// Check if a string is an integer literal (optionally negative).
fn is_integer_literal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests;
