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

//! Statement-level bytecode emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting IR statements
//! (assignments, calls, returns, barriers) with `startCommand`
//! wrapping.

use super::cmd_subst::{has_command_separator, is_pure_cmd_subst, parse_cmd_parts};
use super::helpers::{SubstPart, parse_subst_template};
use super::values::split_array_ref;
use super::{CodegenCtx, Op, Operand};
use crate::ir::{Statement, WordExpr};
use crate::word_subst::whole_word_command_tokens;
use tcl_registry::hooks::InlineCodegenHookId;
#[path = "statements/native_array.rs"]
mod native_array;
#[path = "native_control.rs"]
mod native_control;
#[path = "native_coroutine.rs"]
mod native_coroutine;
#[path = "native_each.rs"]
mod native_each;
#[path = "native_error.rs"]
mod native_error;
#[path = "native_info_exists.rs"]
mod native_info_exists;
#[path = "statements/native_introspection.rs"]
mod native_introspection;
#[path = "native_list_operations.rs"]
mod native_list_operations;
#[path = "native_namespace_upvar.rs"]
mod native_namespace_upvar;
#[path = "statements/native_scalar.rs"]
mod native_scalar;
#[path = "statements/native_string.rs"]
mod native_string;
#[path = "native_switch.rs"]
mod native_switch;
#[path = "native_try.rs"]
mod native_try;
#[path = "native_unset.rs"]
mod native_unset;
#[path = "native_upvar.rs"]
mod native_upvar;

/// Work retained on the heap so array indices and bracket scripts share one
/// compilation unit without recursive emitter calls or depth-limited views.
enum NativeEmissionTask {
    NativeArrayEachStart(
        tcl_dialect::TclVersion,
        [std::rc::Rc<std::cell::Cell<Option<usize>>>; 2],
    ),
    NativeEachStart(
        tcl_dialect::TclVersion,
        Vec<Vec<Vec<u8>>>,
        Vec<std::rc::Rc<std::cell::Cell<Option<usize>>>>,
        bool,
    ),
    ExpressionNode(
        tcl_registry::native_expression_program::NativeExpressionProgram,
        tcl_syntax::expr::NativeExprNode,
    ),
    ExpressionSyntax(Vec<u8>, Option<Vec<u8>>),
    PrivateExpressionNumber(
        tcl_dialect::TclVersion,
        tcl_bytecode::NativeExpressionNumberLiteral,
    ),
    PrivateLogicalBoolean85(bool),
    NativeCommandLiteral(Box<tcl_registry::native_command_literal::NativeCompiledCommandLiteral>),
    RegisteredExpressionNumber(
        tcl_dialect::TclVersion,
        Vec<u8>,
        tcl_bytecode::NativeExpressionNumberLiteral,
    ),
    RegisteredExpressionBoolean84(Vec<u8>),
    FixedMathCall(
        tcl_runtime_api::native_compilation::NativeMathFunctionBinding,
        u8,
    ),
    ControlScript(
        tcl_lexer::SourceImage,
        tcl_registry::native_control_instructions::NativeControlBody,
        tcl_registry::native_compilation::NativeCompiledBodyContext,
        tcl_lexer::LexerConfig,
    ),
    ControlContext(tcl_registry::native_compilation::NativeCompilationContext),
    BeginNativeCatch(String),
    BeginNativeCatchAt(String, String, String),
    BeginSpeculativeNativeCompilation(std::rc::Rc<std::cell::Cell<bool>>),
    FinishSpeculativeNativeCatch(
        NativeNamespaceRollback,
        tcl_lexer::NativeScriptCommandWords,
        std::rc::Rc<std::cell::Cell<bool>>,
        tcl_registry::native_compilation::NativeCompilationContext,
        u32,
    ),
    EndNativeCatch,
    BeginNativeCatchBranch(String),
    EndNativeCatchBranch,
    DeclareNativeTemporary(std::rc::Rc<std::cell::Cell<Option<usize>>>),
    NativeTemporaryOperation(
        Op,
        std::rc::Rc<std::cell::Cell<Option<usize>>>,
        Vec<Operand>,
    ),
    PrivateInteger(i64),
    PrivateList(
        Vec<Vec<u8>>,
        tcl_syntax::native_string::NativeStringProtocol,
    ),
    Word(tcl_lexer::NativeWord),
    List(
        std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        tcl_lexer::PartListId,
        usize,
    ),
    Part(
        std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        tcl_lexer::PartListId,
        usize,
    ),
    Script(
        tcl_lexer::SourceImage,
        tcl_lexer::Span,
        tcl_lexer::LexerConfig,
    ),
    Command(tcl_lexer::NativeScriptCommandWords),
    RestoreSource(NativeEmissionSource),
    Operation(Op, Vec<Operand>),
    NativeListIndex(tcl_syntax::native_compiled_index::NativeCompiledListIndex),
    NativeListRange(tcl_syntax::native_compiled_index::NativeCompiledListRange),
    ErrorReturn,
    SwitchOperation(Op, Vec<Operand>, tcl_dialect::TclVersion),
    SwitchTable(
        tcl_dialect::TclVersion,
        bool,
        std::collections::HashMap<Vec<u8>, String>,
        std::collections::HashMap<i64, String>,
    ),
    DeclareNamespaceLocal(Vec<u8>),
    NamespaceLocalOperation(Op, Vec<u8>),
    NamespaceGenericRollback(NativeNamespaceRollback, tcl_lexer::NativeScriptCommandWords),
    Literal(Vec<u8>),
    PoolLiteral(usize),
    PrivateReturnOptions(tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral),
    Label(String),
    Syntax(&'static str),
}

struct PreparedNativeRegistered {
    version: tcl_dialect::TclVersion,
    instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
    boundary: NativeRegisteredBoundary,
}

struct NativeRegisteredBoundary {
    namespace: tcl_core_types::ByteNamespacePath,
    prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
}

struct NativeNamespaceRollback {
    instructions: usize,
    labels: std::collections::HashMap<String, usize>,
    loop_regions: usize,
    command_index: u32,
}

struct NativeEmissionSource {
    image: tcl_lexer::SourceImage,
    lines: Option<tcl_lexer::LineIndex>,
    span: Option<tcl_lexer::Span>,
    command: bool,
    exact_command: Option<tcl_lexer::SourceImage>,
    namespace: tcl_runtime_api::ByteNamespacePath,
    namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// Tag used to identify `startCommand` instructions wrapping generic
/// invokes so the peephole pass can selectively remove them.
pub(crate) const SC_GENERIC_TAG: &str = "sc:generic";

/// Whether `s` contains a `$` or `[` that is *not* backslash-escaped, i.e. a
/// real variable / command substitution that must be resolved at runtime.
///
/// A word with only escaped markers (`\$`, `\[`) and other backslash escapes is
/// a pure compile-time literal: it can be backslash-substituted and pushed
/// directly. Without this distinction a word like `x\$y` or list-1.11's
/// `list e\n} f\$}` would keep its backslashes (real Tcl delivers `x$y` /
/// `list e\n} f$}`).
pub(crate) fn has_unescaped_subst(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            // A backslash escapes the next byte (`\$`, `\[`, `\\`); skip both.
            b'\\' => i += 2,
            b'$' | b'[' => return true,
            _ => i += 1,
        }
    }
    false
}

/// Whether `s` contains a **backslash-escaped** substitution marker (`\$` or
/// `\[`).
///
/// The sibling of [`has_unescaped_subst`], and the question that decides
/// whether a word may be deferred to the VM's `subst_word`. It may not: the
/// VM reads a compiled literal under the convention that codegen already
/// decoded the word's escapes, so a surviving backslash is an ordinary
/// character and the `$` after it starts a live `${…}`. Handing over an
/// *undecoded* word therefore breaks that contract — `puts A:[string length
/// "\$\{x}"]` read the variable `x`, and with no closing brace it raised
/// `missing close-brace for variable name`, on scripts both oracles run.
pub(crate) fn has_escaped_subst_marker(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'\\' {
            if matches!(b[i + 1], b'$' | b'[') {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

/// Tag appended to no-dedup literal comments.
pub(crate) const NO_DEDUP_TAG: &str = " #nodedup";

/// A static store's name word, resolved by
/// [`CodegenCtx::store_target`] — see that method for the rule and its oracle.
struct StoreTarget {
    /// The variable name the store must use: the name word's *value*, with the
    /// element key still in source form when [`Self::key_is_literal`] is false.
    name: String,
    /// Whether the element key (if this names one) is already a finished
    /// literal, so codegen pushes it verbatim instead of substituting it.
    /// True for a braced word and for any word whose only substitutions were
    /// backslash escapes; false while a live `$` / `[` remains in the key.
    key_is_literal: bool,
}

impl CodegenCtx<'_> {
    /// Emit a statement, wrapping with `startCommand` if needed.
    ///
    /// `count_override` overrides the default count of 1 (e.g. 2 when
    /// a compound command and its init share the same bytecode offset).
    /// `deferred_end_label` uses a caller-provided label instead of
    /// creating and placing one automatically.
    pub fn emit_stmt_with_start_cmd(
        &mut self,
        stmt: &Statement,
        count_override: Option<u32>,
        deferred_end_label: Option<&str>,
    ) {
        if !stmt.is_executable_invocation() {
            return;
        }
        // The wrapping startCommand shares the statement's source span.
        self.set_command_source_span(stmt.span());
        let count = count_override.unwrap_or(1);
        let emit_sc = self.cmd_index > 0;
        let mut end_label: Option<String> = None;
        let mut sc_idx: Option<usize> = None;

        if emit_sc {
            let label = match deferred_end_label {
                Some(l) => l.to_owned(),
                None => self.fresh_label("cmd_end"),
            };
            sc_idx = Some(self.emit_comment(
                Op::START_CMD,
                vec![
                    Operand::Label(label.clone()),
                    Operand::Imm(i32::try_from(count).expect("startCommand count fits in i32")),
                ],
                "",
            ));
            end_label = Some(label);
        }

        let mut used_generic_invoke = false;
        self.emit_stmt(stmt, &mut used_generic_invoke);

        // A typed emitter may have consumed a binding from an inlined source
        // site whose namespace differs from this function's. The wrapper was
        // emitted before that binding was known, so finish the boundary from
        // the same central namespace state the specialised instructions used.
        if let Some(idx) = sc_idx {
            self.instructions[idx]
                .source_command_namespace
                .clone_from(&self.current_command_namespace);
            self.instructions[idx]
                .source_command_namespace_context
                .clone_from(&self.current_command_namespace_context);
        }

        if used_generic_invoke {
            self.seen_generic_invoke = true;
            // Tag this startCommand as wrapping a generic invoke
            if let Some(idx) = sc_idx
                && !self.is_proc
            {
                SC_GENERIC_TAG.clone_into(&mut self.instructions[idx].comment);
            }
        }

        if let Some(ref label) = end_label
            && deferred_end_label.is_none()
        {
            // Place end label before the trailing pop (or at end)
            let pos = if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
                self.instructions.len() - 1
            } else {
                self.instructions.len()
            };
            self.label_positions.insert(label.clone(), pos);
        }

        self.cmd_index += 1;
    }

    /// Emit one source command whose bytecode position is already covered by
    /// an enclosing structured command's `START_CMD`.
    ///
    /// A constant-true `if` and the first command in its selected body begin at
    /// the same bytecode offset in Tcl 9.0. The enclosing marker therefore
    /// carries count two and the nested command must not grow a second marker
    /// at that position. Keep this beside the ordinary statement wrapper so
    /// command indexing and generic-invoke bookkeeping cannot diverge between
    /// the two paths.
    pub fn emit_stmt_under_start_cmd(&mut self, stmt: &Statement) {
        if !stmt.is_executable_invocation() {
            return;
        }
        self.set_command_source_span(stmt.span());
        let mut used_generic_invoke = false;
        self.emit_stmt(stmt, &mut used_generic_invoke);
        if used_generic_invoke {
            self.seen_generic_invoke = true;
        }
        self.cmd_index += 1;
    }

    /// Resolve a *static store*'s name word for codegen.
    ///
    /// Tcl substitutes a command word **whole** and only then scans the result
    /// for the `(`…`)` that makes it an array element. The IR carries the
    /// word's *source spelling*, so codegen owns both halves; it resolves the
    /// two shapes a static store can take:
    ///
    /// * **Fully static** — a braced word (which substitutes nothing) or one
    ///   carrying only backslash escapes. The word's value is known here:
    ///   decode it, and both the name and any element key are finished
    ///   literals that must be pushed *verbatim* so the VM's `subst_word`
    ///   never touches them again.
    /// * **Live key** — the word still holds an unescaped `$` / `[`, which can
    ///   only be inside the element key: `lower_set` keeps a computed *base*
    ///   (`set $x v`, `set a[f] v`) a generic `Call` whose `STORE_STK` operand
    ///   is the substituted word. The split then runs on the source spelling
    ///   and `push_array_key` resolves the key at run time; the escape-only
    ///   base is still decoded.
    ///
    /// Two divergences close here. Not decoding puts the variable in the table
    /// under its source spelling, so a read through one spelling misses a write
    /// through the other; not pushing the resolved name verbatim lets
    /// `subst_word` substitute it a *second* time — stripping a name's outer
    /// braces, reading a `${…}` inside it, and running a `[…]` inside it.
    ///
    /// Identical on tclsh 8.4.20, 8.5.19, 8.6.14, 9.0.4 and 9.1:
    ///
    /// ```text
    /// # the name is the word's value, not its spelling
    /// set "z1\\" A ; set "z2\}" B ; set "z3\ x" C ; set z4\\ D ; set {z5\\} E
    /// set "z6\x41" F ; set "z7\t" G
    /// foreach n [lsort [info vars z*]] { puts "$n len=[string length $n]" }
    /// ->  z1\ len=3      (quoted `\\`   -> one backslash)
    ///     z2} len=3      (quoted `\}`   -> a brace)
    ///     z3 x len=4     (quoted `\ `   -> a space)
    ///     z4\ len=3      (bare   `\\`   -> one backslash)
    ///     z5\\ len=4     (BRACED `\\`   -> kept verbatim: the negative case)
    ///     z6A len=3      (quoted `\x41`)
    ///     z7<TAB> len=3  (quoted `\t`)
    ///
    /// # the resolved name is never substituted again
    /// set {{a}} V ; set {a[bogus]} V ; set {${x}} V ; set arr({k}) V
    /// ->  variables `{a}` (len 3), `a[bogus]` (len 8) and `${x}` (len 4),
    ///     plus `arr` with the single key `{k}` (len 3) — no command runs,
    ///     no `x` is read, no braces are stripped.
    ///
    /// # the two halves together: escapes decide where the element starts, and
    /// # an escaped `$` inside a key stays a literal `$`
    /// set i IDX ; set q\(b\) 1 ; set r(\$i) 2 ; set s($i) 3
    /// ->  array q key b ; array r key {$i} ; array s key IDX
    /// ```
    fn store_target(&mut self, name: &str, name_braced: bool) -> Option<StoreTarget> {
        // Braces suppressed every substitution *except* the one they permit:
        // a `\<newline>` continuation (and the whitespace after it) collapses
        // to a single space. Doing it here rather than at each push site keeps
        // the name final for the local-variable table too — tclsh 8.4.20 /
        // 8.5.19 / 8.6.16 / 9.0.4 / 9.1 all make `set {z1\<newline>y} B` the
        // 4-byte name `z1 y` (hex `7a312079`).
        if name_braced {
            return Some(StoreTarget {
                name: self.word_rules.collapse_braced_word(name).into_owned(),
                key_is_literal: true,
            });
        }
        if has_unescaped_subst(name) {
            // Live key: keep the key's source spelling so `push_array_key`
            // substitutes it, but resolve the escape-only base as Tcl does.
            let name = match split_array_ref(name) {
                Some((base, elem)) => format!("{}({elem})", self.decoded_name(base)?),
                None => name.to_owned(),
            };
            return Some(StoreTarget {
                name,
                key_is_literal: false,
            });
        }
        Some(StoreTarget {
            name: self.decoded_name(name)?,
            key_is_literal: true,
        })
    }

    /// A native name needs its own represented byte lookup protocol.
    /// Decline emission when that protocol cannot accept the decoded bytes.
    fn decoded_name(&mut self, source: &str) -> Option<String> {
        let bytes = tcl_lexer::backslash_subst_bytes_in(source.as_bytes(), self.escapes);
        if let Ok(name) = std::str::from_utf8(&bytes) {
            Some(name.to_owned())
        } else {
            self.refuse_native_dependency();
            None
        }
    }

    /// Emit the retained fused assignment with its nested expression boundary.
    fn emit_assign_expression(&mut self, stmt: &Statement) {
        let Statement::AssignExpr {
            name,
            name_braced,
            expr,
            command_binding,
            ..
        } = stmt
        else {
            unreachable!("selected expression assignment");
        };
        // The fused command retains the exact source binding it
        // consumed. Runtime revalidation checks this dependency at the
        // nested START_CMD boundary, so whole-unit mutation discovery
        // must not prematurely discard a still-valid specialisation.
        if let Some(command_binding) = command_binding {
            self.require_command_binding(command_binding);
        }
        let Some(target) = self.store_target(name, *name_braced) else {
            return;
        };
        let name = target.name.as_str();
        if self.target_needs_stack(name) {
            self.push_var_ref(name, target.key_is_literal);
        }
        let inner_end = self.fresh_label("cmd_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(inner_end.clone()), Operand::Imm(1)],
            "",
        );
        let guaranteed_numeric = self.emit_expr(expr);
        if !guaranteed_numeric {
            self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
        }
        self.place_label(&inner_end);
        self.store_var(name);
        self.emit(Op::POP, vec![]);
    }

    /// Emit assignments, increments and expression-result statements.
    fn emit_assign_or_incr(&mut self, stmt: &Statement) -> bool {
        match stmt {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                let Some(target) = self.store_target(name, *name_braced) else {
                    return true;
                };
                let name = target.name.as_str();
                if self.target_needs_stack(name) {
                    self.push_var_ref(name, target.key_is_literal);
                }
                // A constant value is verbatim: push it as-is so the VM does
                // not run word substitution on any `[…]` / `$` it contains
                // (e.g. `set x {a [b] $c}`).
                self.push_lit_verbatim(value);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::AssignValue {
                name,
                name_braced,
                value,
                value_needs_backsubst,
                tokens,
                ..
            } => {
                self.emit_assign_value(
                    name,
                    *name_braced,
                    value,
                    *value_needs_backsubst,
                    tokens.as_ref().and_then(|tokens| tokens.words().get(2)),
                );
                true
            }
            Statement::AssignExpr { .. } => {
                self.emit_assign_expression(stmt);
                true
            }
            Statement::Incr {
                name,
                name_braced,
                amount,
                ..
            } => {
                if self.try_emit_original_increment() {
                    self.emit(Op::POP, vec![]);
                    return true;
                }
                let Some(target) = self.store_target(name, *name_braced) else {
                    return true;
                };
                self.emit_incr(&target.name, target.key_is_literal, amount.as_deref());
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::ExprEval {
                command_binding,
                expr,
                ..
            } => {
                self.require_command_binding(command_binding);
                if self.emit_retained_native_expression() {
                    self.emit(Op::POP, vec![]);
                    return true;
                }
                let guaranteed_numeric = self.emit_expr(expr);
                if !guaranteed_numeric {
                    self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
                }
                self.emit(Op::POP, vec![]);
                true
            }
            _ => false,
        }
    }

    /// Emit the shared `set name VALUE` storage path. `AssignExpr` uses this
    /// too when its nested `expr` command was rebound after lowering fused it.
    fn emit_assign_value(
        &mut self,
        name: &str,
        name_braced: bool,
        value: &str,
        value_needs_backsubst: bool,
        value_word: Option<&WordExpr>,
    ) {
        // Whether this word's escapes were decoded *here*. A decoded value is
        // finished: every marker left in it came from an escape and is data, so
        // it must not be read as source again. `set v "\$\{x}"` decodes to the
        // four characters `${x}`, and re-reading that as a variable reference —
        // which is what the value emitter's `${…}` fast path does — loaded `x`
        // and stored one character instead.
        let decoded_here = value_needs_backsubst && !value.contains('[') && !value.contains("${");
        let value = if decoded_here {
            value.to_owned()
        } else {
            // A value carrying a `[` or `${` (an escaped `\[` / `\${` or
            // a real substitution) is left raw: the runtime `subst_word`
            // does backslash decoding plus command / variable substitution in
            // one left-to-right pass. Pre-decoding would turn escaped markers
            // into live substitutions and would double-decode other escapes.
            value.to_owned()
        };
        let inline = Self::assign_value_inlines_cmd_subst(&value);
        let Some(target) = self.store_target(name, name_braced) else {
            return;
        };
        let name = target.name.as_str();
        if self.target_needs_stack(name) {
            self.push_var_ref(name, target.key_is_literal);
        }

        // Let the registry-owned fold have first refusal. If the active
        // command surface declines the fold, dispatch the same whole-word
        // substitution through that surface's inline hook rather than a
        // command-name allow/deny list in this consumer.
        if decoded_here {
            // Finished above: byte-exact, with no further reading of the text.
            self.push_decoded_literal(&value);
        } else if self.try_emit_constant_fold(&value) {
            // The shared fold emitted the value.
        } else if inline {
            // Assignment already owns this shape's direct inline dispatcher:
            // it handles expanded and multi-command bodies as well as every
            // established specialised hook.  Carry source tokens only as an
            // optional fact for its `info` / `array` local-name decision.
            let nested = self.nested_command_tokens(value_word);
            self.emit_inline_cmd_subst_with_tokens(&value, nested.as_ref());
        } else {
            self.emit_value_interpolated_from_word(&value, value_word);
        }
        self.store_var(name);
        self.emit(Op::POP, vec![]);
    }

    /// Emit `Statement::Call` — handles the CFG placeholder names
    /// (`<cond>` / `<empty_clause>`), `{*}` expansion, and the
    /// generic call path.
    fn emit_call_stmt(
        &mut self,
        command: &str,
        args: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        // A statement the CFG builder synthesised carries an effect or a
        // placeholder, never a command to run. The identity is the *typed*
        // `CommandTokens::synthetic` discriminant, never the `command`
        // spelling: `<cond>`, `<empty_clause>` and the rest are all legal Tcl
        // command names a script may define and call (tclsh 8.4.20 / 8.5.19 /
        // 8.6.16 / 9.0.4 / 9.1 all run `proc <cond> {} { puts hit-cond };
        // <cond>`), so matching on the name silently dropped such a call.
        //
        // Dispatching a *marker*, conversely, either duplicates the callee's
        // side effects — a caller-frame barrier reusing the callee's own name
        // makes `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p` print twice —
        // or reaches the VM as an invalid command
        // name (`invalid command name "<global-frame-script>"`).
        if let Some(marker) = tokens.and_then(|t| t.synthetic) {
            if marker == crate::ir::SyntheticMarker::EmptyClause {
                // tclsh's bytecode keeps three `nop`s where a `for` clause is
                // empty; match it so the instruction stream stays byte-true.
                self.literals.intern("");
                self.emit(Op::NOP, vec![]);
                self.emit(Op::NOP, vec![]);
                self.emit(Op::NOP, vec![]);
            }
            return;
        }
        let has_expand = tokens
            .and_then(|t| t.expand_word.as_ref())
            .is_some_and(|ew| ew.iter().any(|&e| e));
        if has_expand {
            let ew = tokens.and_then(|t| t.expand_word.as_ref()).unwrap();
            self.emit_expanded_call(command, args, ew, tokens);
            *used_generic_invoke = true;
        } else {
            if self.emit_selected_try_statement(command, args, tokens) {
                *used_generic_invoke = true;
                return;
            }
            self.emit_call(command, args, tokens, used_generic_invoke);
        }
    }

    /// Dispatch a statement to the appropriate emission handler.
    pub fn emit_stmt(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        let source = self.pending_math_source.take();
        let previous_source = std::mem::replace(&mut self.math_source, source);
        let base = match stmt {
            Statement::AssignExpr { expr_base, .. }
            | Statement::ExprEval { expr_base, .. }
            | Statement::Return { expr_base, .. } => *expr_base,
            _ => None,
        };
        let previous_base = std::mem::replace(&mut self.math_expression_base, base);
        let source_proofs = self.source_proofs.clone();
        let tokens = stmt
            .tokens()
            .or_else(|| source_proofs.as_ref()?.tokens.get(&stmt.span().start()));
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.emit_stmt_inner(stmt, used_generic_invoke);
        });
        self.math_source = previous_source;
        self.math_expression_base = previous_base;
    }

    fn emit_opaque_switch(
        &mut self,
        raw_args: &[String],
        raw_arg_braced: &[bool],
        used_generic_invoke: &mut bool,
    ) {
        // Each word goes out the way it was written. A braced word is
        // data — its `[…]` / `${…}` must not run — and that is true of
        // every position, not just the trailing arm list: otherwise
        // `switch -glob -- {a[bc]d} {a\[bc\]d} …` runs `bc` for the
        // subject, and decodes the pattern to `a[bc]d`, which then
        // matches as a character class rather than as the literal both
        // oracles match.
        //
        // `raw_arg_braced` is the lexer's answer, carried through the
        // IR because `raw_args` are values and a value cannot say how
        // it was written. Empty means unknown (a hand-built
        // statement), which keeps the old all-interpolated behaviour.
        if raw_arg_braced.iter().any(|braced| *braced) {
            let n = raw_args.len();
            let mut kinds = vec![tcl_lexer::TokenType::Esc; n + 1];
            for (i, braced) in raw_arg_braced.iter().enumerate() {
                if *braced && i + 1 < kinds.len() {
                    kinds[i + 1] = tcl_lexer::TokenType::Str;
                }
            }
            let toks = crate::ir::CommandTokens::from_lossy_parts(
                Vec::new(),
                Vec::new(),
                kinds,
                vec![true; n + 1],
                Vec::new(),
                None,
            );
            self.emit_call_stmt("switch", raw_args, Some(&toks), used_generic_invoke);
        } else {
            self.emit_call_stmt("switch", raw_args, None, used_generic_invoke);
        }
    }

    fn emit_stmt_inner(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        // Stamp every instruction this statement lowers to with its source
        // span so the explorer can map each op back to source.
        self.set_command_source_span(stmt.span());
        if !stmt.is_executable_invocation() {
            return;
        }
        if !matches!(
            stmt,
            Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. }
        ) && self.emit_retained_native_dispatch(used_generic_invoke)
        {
            return;
        }
        if self.emit_assign_or_incr(stmt) {
            return;
        }
        match stmt {
            Statement::NativeCall { words, .. } => {
                self.emit_native_words(words);
                self.emit(Op::POP, vec![]);
                *used_generic_invoke = true;
            }
            Statement::Call {
                command,
                args,
                tokens,
                ..
            } => self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke),

            Statement::Barrier {
                command,
                args,
                reason,
                tokens,
                ..
            } => {
                if command.is_empty() {
                    // Retained source with missing context/admission is not an
                    // executable empty body. A real native source provider must
                    // discharge the obligation before this unit can enter.
                    if reason == "original body namespace context is unavailable"
                        || reason == "original body source admission is unavailable"
                    {
                        self.refuse_native_dependency();
                    }
                    self.emit_comment(Op::NOP, vec![], &format!("barrier: {reason}"));
                } else {
                    self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke);
                }
            }

            // Opaque (glob/regexp/fall-through) switch: the CFG builder keeps
            // it as a single statement rather than expanding arm blocks, so
            // emit a generic `switch` invoke — tclsh 9.0's un-compiled approach
            // for these modes.
            Statement::Switch {
                raw_args,
                raw_arg_braced,
                ..
            } => {
                self.emit_opaque_switch(raw_args, raw_arg_braced, used_generic_invoke);
            }

            Statement::Return { value, .. } => {
                let val = value.as_deref().unwrap_or("");
                self.emit_value_interpolated(val);
                // A top-level `RETURN_IMM` pops the result *and* an options dict;
                // push the empty options so the pair sits at the top of the
                // stack regardless of any leftover from a preceding statement
                // (e.g. an `if` with no `else`). Proc bodies fold this to a
                // `DONE`, which returns the single top value, so no options push.
                if !self.is_proc {
                    self.push_lit("");
                }
                // A plain `return` is `(code 0, level 1)`: C's
                // `TclMergeReturnOptions` defaults `-level` to 1, and
                // `CompileReturnInternal` puts the merged pair on the operands
                // (`tclCompCmdsGR.c:2410`). `(0, 0)` would mean "push the result
                // and fall through", not "return". Proc bodies re-key on the same
                // pair in `fold_tail_return_to_done`.
                self.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
            }

            // Static-body uplevel: `UpFrame` is the structured IR form the
            // analysers consume (interproc purity, code-sinking, var-escape, the
            // inline_uplevel pass). The whole-callee inline_uplevel splice
            // rewrites passthrough call-sites into `Statement::Block` (emitted
            // inline below); any `UpFrame` that reaches codegen unspliced is run
            // through the runtime `uplevel` builtin by re-emitting the original
            // `uplevel <level> {body}` invoke from its preserved command tokens.
            // This is byte-identical to the pre-static-lowering barrier path and
            // keeps correct frame semantics without needing frame-shift opcodes.
            Statement::UpFrame { tokens, .. } => {
                if let Some(ct) = tokens
                    && ct.argv_texts.len() >= 2
                {
                    let command = ct.argv_texts[0].clone();
                    let args: Vec<String> = ct.argv_texts[1..].to_vec();
                    self.emit_call_stmt(&command, &args, Some(ct), used_generic_invoke);
                } else {
                    // No preserved tokens (synthetic UpFrame) — nothing to
                    // dispatch; mark it rather than silently dropping a body.
                    self.emit_comment(Op::NOP, vec![], "unhandled: Statement::UpFrame (no tokens)");
                }
            }

            // Inline block: the inline_uplevel pass replaces
            // matching call-sites with ``Statement::Block { body, .. }``
            // — emit the body's statements as if they were inline at
            // this point. The block has no scope of its own at the
            // bytecode level (the body already ran in the caller's
            // frame in the original ``uplevel`` semantics).
            Statement::Block { body, .. } => {
                if crate::registry_invocation::block_requires_runtime_script(stmt) {
                    if let Some(Statement::Barrier {
                        command,
                        args,
                        tokens,
                        ..
                    }) = crate::registry_invocation::original_block_runtime_invocation(stmt)
                    {
                        self.emit_call_stmt(&command, &args, tokens.as_ref(), used_generic_invoke);
                    } else {
                        self.refuse_native_dependency();
                    }
                    return;
                }
                let begin = self.instructions.len();
                for inner in &body.statements {
                    self.emit_stmt(inner, used_generic_invoke);
                }
                if let Some(first) = (begin < self.instructions.len()).then_some(begin) {
                    self.mark_completion_option_scope(
                        first,
                        tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED,
                    );
                }
            }

            // Structured control flow is handled by the main emitter loop;
            // if we reach these here, emit a NOP placeholder.
            _ => {
                self.emit_comment(Op::NOP, vec![], "unhandled statement");
            }
        }
    }

    fn emit_retained_native_dispatch(&mut self, used_generic_invoke: &mut bool) -> bool {
        if self.invocation_specialisation_proved() {
            return false;
        }
        let Some(tokens) = self
            .invocation_tokens
            .as_deref()
            .filter(|tokens| tokens.synthetic.is_none() && tokens.source_binding.is_some())
            .cloned()
        else {
            return false;
        };
        let Some((head, arguments)) = tokens.argv_texts.split_first() else {
            self.refuse_native_dependency();
            return false;
        };
        self.emit_call_stmt(head, arguments, Some(&tokens), used_generic_invoke);
        true
    }

    /// Emit a simple value — handles `${var}` references and literals.
    ///
    /// This is a simplified value emitter that handles the common cases.
    /// Full interpolation with command substitution requires the main
    /// emitter pipeline.
    pub fn emit_value_interpolated(&mut self, value: &str) {
        self.emit_value_interpolated_from_word(value, None);
    }

    fn emit_variable_index(&mut self, value: &str) -> bool {
        // Whole-word variable-index array element `$arr($idx)`: route through
        // `load_var` (base + substituted key) rather than the literal/runtime
        // subst path, which cannot resolve the bare `$idx` inside the index.
        if value.starts_with('$')
            && value.ends_with(')')
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
            && parts.len() == 1
            && let SubstPart::Var(name) = &parts[0]
            && split_array_ref(name).is_some()
        {
            self.load_var(name);
            return true;
        }
        false
    }

    /// Emit a value while retaining a source word only for an aligned nested
    /// command substitution. Every other value follows the established path.
    fn emit_value_interpolated_from_word(&mut self, value: &str, word: Option<&WordExpr>) {
        if word.is_some_and(|word| self.emit_retained_expression(value, word)) {
            return;
        }
        if word.is_some_and(|word| self.emit_retained_template(value, word)) {
            return;
        }
        if let Some(WordExpr::Variable { spelling, .. }) = word
            && word.is_some_and(|word| value == word.legacy_text())
            && self.emit_variable_reference(spelling)
        {
            return;
        }
        if self.emit_variable_reference(value) {
            return;
        }
        // The `[list …]` / `[format …]` / `[dict create …]` folds and the two
        // `list` inlinings — one copy, shared with `emit_value`.
        if self.try_emit_constant_fold(value) {
            return;
        }
        if self.emit_variable_index(value) {
            return;
        }
        // Interpolated string containing an array-element variable
        // (`"…$arr($idx)…"`): the runtime `subst_word` fallback can substitute a
        // normalised `${name}` but not an array element with a substituted
        // index, so decompose the word at compile time. Plain `$scalar`
        // interpolation keeps its existing (deferred-literal) lowering.
        // Also decompose a `{…}`-wrapped value that still carries a live
        // substitution (e.g. `"{[list …]}"`, `"{$z}"`): the braces are literal
        // word content and the `[…]` / `${…}` must run, but pushing the value
        // raw would let the runtime `subst_word` mistake it for a braced
        // literal, strip the braces and hand back the *unsubstituted* inside:
        // `puts "{$z}"` would print `${z}`, and `set q "{$z}"` would store four
        // bytes where both oracles store three. This is the marker-carrying half of
        // the value rule `push_word_value` states: a value the runtime still
        // owns cannot simply be frozen, so it is resolved here instead. A
        // genuine braced argument never reaches here (it is emitted by the
        // braced-word path), so decomposing is safe.
        //
        // The brace test is the VM's own rule, through the same owner the VM
        // asks (`whole_braced_word`) — this arm exists precisely because
        // `subst_word` *will* strip, so anything looser decomposes words it
        // would not have touched and anything two-byte repeats the bug this
        // change is about. `{}${z}` is the discriminating case: it is not a
        // braced word, the VM's general scan substitutes it correctly, and it
        // stays on the literal path.
        if (value.contains('$') || value.contains('['))
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
            && parts.len() > 1
            && (parts.iter().any(|p| {
                matches!(p, SubstPart::LiteralElement { .. })
                    || matches!(p, SubstPart::Var(n) if split_array_ref(n).is_some())
            }) || (tcl_syntax::word_rules::whole_braced_word(value).is_some()
                && parts.iter().any(|p| {
                    matches!(
                        p,
                        SubstPart::Cmd(_) | SubstPart::Var(_) | SubstPart::LiteralElement { .. }
                    )
                })))
        {
            for part in &parts {
                match part {
                    // A decoded fragment is finished text, never source — see
                    // `push_word_value`, whose fragment rule this is.
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
                    SubstPart::Var(name) => self.load_var(name),
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
                    }
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(parts.len()).expect("part count fits in i32"),
                )],
            );
            return;
        }
        // A value with backslash escapes and a *bare* `$` (e.g. `f\$}` / the
        // `$}` in `list e\n} f\$}`) but no real `${…}` reference or `[…]`
        // command substitution is a pure literal — the bare `$` stays literal
        // because the codegen normalises genuine variables to `${name}`. Decode
        // its escapes once at compile time. Without this the value is pushed raw
        // and the runtime `subst_word` (which only decodes words carrying a real
        // `${`/`[`) leaves the `\\` escapes intact. Values the assign path
        // already backslash-decoded never reach here with a `$`, so there is no
        // double decode.
        if value.contains('\\')
            && value.contains('$')
            && !value.contains("${")
            && !value.contains('[')
        {
            self.push_decoded_literal(value);
            return;
        }
        // A whole-word command substitution compiles inline (on the explicit
        // stack) rather than via the runtime `subst_word` fallback, so a
        // `[yield]`/`[cmd]` inside it stays yieldable in a coroutine.
        if self.emit_inline_cmd_subst_from_word(value, word) {
            return;
        }
        // A word carrying an *escaped* marker cannot be deferred at all, even
        // when it also carries a live one. The VM reads a compiled literal's
        // backslashes as ordinary characters, so an undecoded `\$\{x}` handed
        // over raw is read back as a live `${x}`. Resolve the word here
        // instead: the template parser decodes each literal run and separates
        // the substitutions that really are live, so the escaped marker ends up
        // as data and the `[…]` beside it still runs.
        if has_escaped_subst_marker(value)
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
        {
            for part in &parts {
                match part {
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
                    SubstPart::Var(name) => self.load_var(name),
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
                    }
                }
            }
            if parts.len() > 1 {
                self.emit(
                    Op::STR_CONCAT1,
                    vec![Operand::Imm(
                        i32::try_from(parts.len()).expect("part count fits in i32"),
                    )],
                );
            } else if parts.is_empty() {
                self.push_lit_exact("");
            }
            return;
        }
        // Default: the word's own text is its value — push it unsubstituted
        // unless a `${…}` / `[…]` the runtime still owns survived the arms
        // above (`push_word_value`). Substituting a finished value strips a
        // brace layer that was never source: `set v "{}"` stored the empty
        // string where both oracles store the two-byte `{}`.
        self.push_word_value(value);
    }

    /// Evaluate the retained lexical components of an unchanged template.
    /// Command components retain their own source proof; decoded text is data.
    fn emit_retained_template(&mut self, value: &str, word: &WordExpr) -> bool {
        use crate::ir::WordPart;
        let WordExpr::Template { parts, .. } = word else {
            return false;
        };
        if value != word.legacy_text() {
            return false;
        }
        let mut operands = Vec::with_capacity(parts.len());
        for part in parts {
            let operand = match part {
                WordPart::Text { text, .. } => {
                    (SubstPart::decoded_literal(text, self.escapes), None)
                }
                WordPart::Variable { spelling, .. } => {
                    let Some(parsed) = parse_subst_template(spelling, self.lexer_config()) else {
                        return false;
                    };
                    let [variable @ (SubstPart::Var(_) | SubstPart::LiteralElement { .. })] =
                        parsed.as_slice()
                    else {
                        return false;
                    };
                    (variable.clone(), None)
                }
                WordPart::CommandSubstitution { spelling, source } => (
                    SubstPart::Cmd(spelling.clone()),
                    Some(WordExpr::CommandSubstitution {
                        spelling: spelling.clone(),
                        source: source.clone(),
                    }),
                ),
                WordPart::Opaque { text, .. } => {
                    let Some(parsed) = parse_subst_template(text, self.lexer_config()) else {
                        return false;
                    };
                    let [expression @ SubstPart::Expression(_)] = parsed.as_slice() else {
                        return false;
                    };
                    (expression.clone(), None)
                }
            };
            operands.push(operand);
        }
        for (operand, source) in &operands {
            match operand {
                SubstPart::Lit(text) => self.push_lit_exact(text),
                SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                SubstPart::Var(name) => self.load_var(name),
                SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                SubstPart::Expression(expression) => self.emit_expression_substitution(expression),
                SubstPart::Cmd(text) => {
                    self.emit_value_interpolated_from_word(text, source.as_ref());
                }
            }
        }
        // A lexical Template containing one substitution is a quoted word
        // (bare single substitutions have their own WordExpr variants). Jim
        // copies that word's bytes instead of retaining its source intrep.
        let copies_single = operands.len() == 1
            && matches!(
                operands[0].0,
                SubstPart::Var(_)
                    | SubstPart::LiteralElement { .. }
                    | SubstPart::Cmd(_)
                    | SubstPart::Expression(_)
            )
            && self.native_hook_dialect().is_some_and(|dialect| {
                dialect.quoted_substitution_preserves_object() == Some(false)
            });
        if operands.len() > 1 || copies_single {
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(super::bytecode_imm(operands.len()))],
            );
        } else if operands.is_empty() {
            self.push_lit_exact("");
        }
        true
    }

    /// Retain the native expression token for the VM's direct word evaluator.
    pub(super) fn emit_expression_substitution(&mut self, expression: &str) {
        self.push_lit(&format!("${expression}"));
    }

    /// Evaluate an original bare expression token without naming a command.
    fn emit_retained_expression(&mut self, value: &str, word: &WordExpr) -> bool {
        let WordExpr::Opaque {
            text,
            reason: crate::ir::WordOpacity::DialectSubstitution,
            ..
        } = word
        else {
            return false;
        };
        if text != value {
            return false;
        }
        let Some(parts) = parse_subst_template(text, self.lexer_config()) else {
            return false;
        };
        let [SubstPart::Expression(expression)] = parts.as_slice() else {
            return false;
        };
        self.emit_expression_substitution(expression);
        true
    }

    /// Whether a `set x VALUE` value should inline-compile its command
    /// substitution (`set x [cmd arg …]`) instead of pushing the raw `[…]` text
    /// and leaning on the runtime `subst_word` fallback. The conditions:
    /// a balanced `[…]` with inner whitespace (a bare `[foo]` with no args
    /// stays a literal). The caller first gives the registry-owned
    /// constant-fold path a chance to consume the value, so this shape
    /// predicate contains no per-command knowledge. The inline command emitter
    /// also owns `{*}` argument expansion in this assignment position; keeping
    /// that path live is what makes `set x [concat a {*}$args b]` invoke the
    /// command rather than store its source text. Command arguments keep the
    /// literal + `subst_word` path, which the inline command-parser cannot
    /// match on escaped brackets.
    fn assign_value_inlines_cmd_subst(value: &str) -> bool {
        // `is_pure_cmd_subst`, not "starts and ends with a bracket": the inline
        // emitter strips the outer brackets and word-splits the rest, so
        // `set r [llength $a]:[join $a ,]` — three concatenated parts — would
        // be mangled into one bogus command.
        is_pure_cmd_subst(value) && value.len() > 2 && value[1..value.len() - 1].contains(' ')
    }

    /// If `value` is a *single whole-word command substitution* (`[cmd …]` and
    /// nothing else) of a shape the command-parser handles faithfully, compile
    /// it as a generic `INVOKE` on the explicit activation stack and return
    /// `true`; otherwise return `false` and leave the caller to fall back to its
    /// literal / `subst_word` path.
    ///
    /// C Tcl never runs a whole-word substitution through a recursive
    /// `Tcl_EvalObjEx`; it compiles it to `INST_INVOKE`. Matching that keeps a
    /// `[yield]`/`[cmd]` inside the substitution **yieldable** in a coroutine —
    /// the runtime `subst_word` fallback re-enters the evaluator on the *host*
    /// stack, a boundary a `yield` cannot cross (`cannot yield: C stack busy`).
    ///
    /// An aligned original word retains its independently admitted native
    /// compiler operation. Its emitter validates the original operands and
    /// captures the compiler prerequisites before argument execution. Other
    /// substitutions invoke the actual command on the explicit activation stack.
    ///
    /// Conservative on shape: escaped brackets (`\[` / `\]`, which the parser
    /// cannot match — see [`Self::assign_value_inlines_cmd_subst`]), `{*}`
    /// expansion, and multi-command bodies (a `;`/newline separator) keep the
    /// runtime path. Those never carry the coroutine resume-value idiom, so
    /// nothing yieldable is lost.
    pub(crate) fn try_emit_whole_cmd_subst(&mut self, value: &str) -> bool {
        self.try_emit_whole_cmd_subst_with_tokens(value, None)
    }

    pub(super) fn nested_command_tokens(
        &self,
        word: Option<&WordExpr>,
    ) -> Option<crate::ir::CommandTokens> {
        let mut nested = whole_word_command_tokens(
            word?,
            tcl_lexer::LexerConfig::for_profile(self.registry.profile()),
        )?;
        // Re-parsing preserves the original word's source address, but does
        // not itself carry the selected nested callable. Only the owning
        // invocation can supply that exact-site proof; an omitted site stays
        // unknown through the shared inheritance contract.
        if let Some(parent) = self.invocation_tokens.as_deref() {
            nested.inherit_nested_bindings(parent);
        }
        Some(nested)
    }

    fn emit_inline_cmd_subst_from_word(&mut self, value: &str, word: Option<&WordExpr>) -> bool {
        let nested = self.nested_command_tokens(word);
        self.try_emit_whole_cmd_subst_with_tokens(value, nested.as_ref())
    }

    fn try_emit_whole_cmd_subst_with_tokens(
        &mut self,
        value: &str,
        tokens: Option<&crate::ir::CommandTokens>,
    ) -> bool {
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.try_emit_whole_cmd_subst_inner(value, tokens)
        })
    }

    fn try_emit_whole_cmd_subst_inner(
        &mut self,
        value: &str,
        tokens: Option<&crate::ir::CommandTokens>,
    ) -> bool {
        if value.contains("\\[")
            || value.contains("\\]")
            || value.contains("{*}")
            || has_command_separator(value)
        {
            return false;
        }
        if !matches!(
            parse_subst_template(value, self.lexer_config()).as_deref(),
            Some([SubstPart::Cmd(_)])
        ) {
            return false;
        }
        let parts = parse_cmd_parts(value);
        let Some((head, _)) = parts.split_first() else {
            return false;
        };
        let args = &parts[1..];
        let source_aware = tokens.is_some_and(|tokens| {
            tokens.words_align_with_argv_text()
                && tokens.argv_texts.len() == parts.len()
                && tokens
                    .argv_texts
                    .iter()
                    .zip(&parts)
                    .all(|(word, (text, _))| word == text)
        });
        if source_aware
            && self
                .inline_cmd_subst_hook_candidate(&head.0, args)
                .is_some()
        {
            self.emit_inline_cmd_subst_with_tokens(value, tokens);
            return true;
        }
        // A source snapshot may be unavailable for a compatibility caller.
        // Preserve its established generic value emission rather than
        // guessing from flattened text.
        self.emit_generic_cmd_subst(&head.0, &parts[1..]);
        true
    }

    /// Push variable reference for store operations (name/key on stack).
    ///
    /// `name` is a [resolved store name](Self::store_target) — the name word's
    /// *value*, not its source spelling — so every literal half goes out
    /// **verbatim and byte-exact**
    /// ([`push_lit_exact`](Self::push_lit_exact)): the VM must not run word
    /// substitution over a name it has already resolved, or it strips a name's
    /// outer braces (`set {{a}} V` creating `a` instead of `{a}`), reads a
    /// `${…}` inside one, and runs a `[…]` inside one. The
    /// literal bytes are unchanged — only the out-of-band `push_verbatim` flag
    /// differs — so the disassembly is byte-stable.
    ///
    /// `key_is_literal` says whether an array-element key is likewise finished
    /// (a braced word — `set {a($x)} v` keys on the literal `$x` — or one whose
    /// only substitutions were backslash escapes). A key that still substitutes
    /// (`set a($i) v`) keeps the [`push_array_key`](Self::push_array_key) path.
    pub fn push_var_ref(&mut self, name: &str, key_is_literal: bool) {
        let Some((base, elem)) = split_array_ref(name) else {
            self.push_lit_exact(name);
            return;
        };
        // In a proc the base is the LVT slot `store_var` names; only the key
        // goes on the stack.
        if self.command_variable_slot(base.as_bytes()).is_none() {
            self.push_lit_exact(base);
        }
        if key_is_literal {
            self.push_lit_exact(elem);
        } else {
            self.push_array_key(elem);
        }
    }

    /// Emit a command call with `{*}` expansion on marked arguments.
    /// Emit one command word: a braced single-token word (`{…}`) is pushed
    /// verbatim (substitution suppressed — a `proc` body, or a `{*}`-expanded
    /// braced list whose `[…]`/`$…` are data); a word whose only substitution
    /// markers are backslash-escaped is decoded; otherwise it is interpolated.
    /// Shared by the plain ([`emit_call`](Self::emit_call)) and `{*}`-expanded
    /// ([`emit_expanded_call`](Self::emit_expanded_call)) word loops.
    /// Emit the `i`-th argument of the command currently dispatching to a
    /// codegen hook, honouring whether that word was braced (see
    /// [`Self::cmd_arg_braced`]). Hooks that emit a word as a runtime value
    /// (`concat`, `llength`, …) call this instead of
    /// [`Self::emit_value_interpolated`] so a non-braced literal's backslash
    /// escapes are collapsed exactly like the generic per-word path — a braced
    /// word stays verbatim. Falls back to non-braced when no flags are recorded
    /// (hand-built test contexts).
    pub(crate) fn emit_word_arg(&mut self, i: usize, a: &str) {
        let braced = self.cmd_arg_braced.get(i).copied().unwrap_or(false);
        let word = self.original_hook_argument(i).and_then(|original| {
            self.invocation_tokens
                .as_deref()?
                .words()
                .get(original + 1)
                .cloned()
        });
        self.emit_word_from_source(a, braced, word.as_ref());
    }

    pub(super) fn emit_word_from_source(&mut self, a: &str, braced: bool, word: Option<&WordExpr>) {
        let first = self.instructions.len();
        if braced {
            self.push_lit_verbatim(a);
        } else if a.contains('\\') && !has_unescaped_subst(a) {
            // A non-braced word whose only substitution markers are
            // backslash-escaped (`\{`, `a\{b`, `\ x`, `x\$y`). If decoding it
            // would (re)introduce a `[` / `${` — the word carries an escaped
            // `\[` or `\${` — leave it raw so the runtime `subst_word` decodes
            // it in one left-to-right pass; pre-decoding here would create a
            // bare `[` the VM then mis-reads as a command substitution and
            // double-decodes. Otherwise it is a pure compile-time literal:
            // backslash-substitute it so the escapes don't reach the VM raw.
            if a.contains('[') || a.contains("${") {
                self.push_lit(a);
            } else {
                self.push_decoded_literal(a);
            }
        } else {
            self.emit_value_interpolated_from_word(a, word);
        }
        self.retain_literal_source_value_line(first, word);
    }

    /// Stamp a retained literal value independently of its command boundary.
    /// Captured, transformed and unlocated values retain no invented location.
    pub(super) fn retain_literal_source_value_line(
        &mut self,
        first: usize,
        word: Option<&WordExpr>,
    ) {
        let Some(word @ (WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. })) = word else {
            return;
        };
        let site = word.source();
        if site.provenance != crate::ir::Provenance::Source
            || self.source.is_empty()
            || self.source.get(site.span.as_range()).is_none()
        {
            return;
        }
        let line = self.source_line(site.span);
        let Some([instruction]) = self.instructions.get_mut(first..) else {
            return;
        };
        if matches!(instruction.op, Op::PUSH1 | Op::PUSH4) {
            instruction.source_value_line = Some(line);
        }
    }

    /// Emit a `{*}`-expanded command call: push each word (braced words stay
    /// verbatim — see [`emit_word_from_source`](Self::emit_word_from_source)), `EXPAND_STKTOP`-splitting
    /// the ones flagged in `expand_word`, then `INVOKE_EXPANDED`.
    pub fn emit_expanded_call(
        &mut self,
        cmd: &str,
        args: &[String],
        expand_word: &[bool],
        tokens: Option<&crate::ir::CommandTokens>,
    ) {
        if self.emit_native_original_preparation_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            return;
        }
        let parts = std::iter::once(cmd)
            .chain(args.iter().map(String::as_str))
            .enumerate()
            .map(|(index, word)| {
                (
                    word.to_owned(),
                    tokens
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(index.wrapping_sub(1))),
                    expand_word.get(index).copied().unwrap_or(false),
                )
            })
            .collect::<Vec<_>>();
        let relayed =
            self.with_invocation_tokens(tokens, |ctx| ctx.try_emit_expanded_native_call(&parts));
        if relayed {
            self.emit(Op::POP, vec![]);
            return;
        }
        let words = std::iter::once(cmd)
            .chain(args.iter().map(String::as_str))
            .enumerate()
            .map(|(index, word)| {
                // A braced single-token word stays verbatim even when
                // expanded: the split operates on its list value without
                // substituting `[…]` / `$…` inside it. This applies equally to
                // the command head (index zero) and to every argument.
                let braced = tokens.is_some_and(|t| {
                    t.argv_kinds.get(index) == Some(&tcl_lexer::TokenType::Str)
                        && t.single_token_word.get(index).copied().unwrap_or(false)
                });
                let expanded = expand_word.get(index).copied().unwrap_or(false);
                (word, braced, expanded)
            });
        self.emit_expanded_words(words, &format!("{cmd} (expanded)"));
        self.emit(Op::POP, vec![]);
    }

    /// Emit a regular command call.
    pub fn emit_call(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.emit_call_inner(cmd, args, tokens, used_generic_invoke);
        });
    }

    fn emit_call_inner(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        if self.emit_native_original_preparation_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            *used_generic_invoke = true;
            return;
        }
        if self.emit_native_procedure_noop(tokens) || self.emit_native_named_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            *used_generic_invoke = true;
            return;
        }
        if self.emit_lexical_loop_jump(cmd, args) {
            return;
        }

        // A braced single-token word (`{…}`, lexed as `TokenType::Str`) is a
        // verbatim literal — e.g. a `proc` body — so it is pushed as-is and its
        // backslashes are NOT collapsed; a non-braced word's escapes are. `argv[0]`
        // is the command word, so arg `i` maps to `argv_kinds[i + 1]`. Computed
        // once and stashed so a codegen hook can collapse its list args the same
        // way the generic per-word path below does (concat-1.4, llength-2.3).
        let braced_flags: Vec<bool> = (0..args.len())
            .map(|i| {
                tokens.is_some_and(|t| {
                    t.argv_kinds.get(i + 1) == Some(&tcl_lexer::TokenType::Str)
                        && t.single_token_word.get(i + 1).copied().unwrap_or(false)
                })
            })
            .collect();

        // Try a registered per-command codegen hook before the
        // generic invoke fallback.
        self.cmd_arg_braced = braced_flags;
        if super::emitter::bytecoded::try_bytecoded_with_tokens(
            self,
            cmd,
            args,
            tokens,
            used_generic_invoke,
        ) {
            self.cmd_arg_braced = Vec::new();
            return;
        }

        // Native compilation can select a token even when this backend uses
        // a generic surrogate. Capture that token before any argument executes.
        let entered_binding = self.entered_command_surrogate(cmd);
        let body_start = self.instructions.len();
        // The original head owns both its lexical evaluation and cache actions.
        if !self.try_emit_original_command_head(cmd) {
            self.emit_cmd_word(cmd, false);
        }
        for (i, a) in args.iter().enumerate() {
            let braced = self.cmd_arg_braced.get(i).copied().unwrap_or(false);
            let word = tokens.and_then(|tokens| tokens.words().get(i + 1));
            self.emit_word_from_source(a, braced, word);
        }
        self.cmd_arg_braced = Vec::new();
        let arg_count = i32::try_from(1 + args.len())
            .expect("invoke argument count fits in i32 (bytecode limit)");
        let op = if arg_count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit_comment(op, vec![Operand::Imm(arg_count)], cmd);
        self.retain_entered_command(body_start, entered_binding);
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
    }

    /// Use retained original words when the selected compiler owns preparation
    /// before handler dispatch, including its local and literal allocations.
    pub(super) fn emit_retained_native_expression(&mut self) -> bool {
        let tokens = self.invocation_tokens.clone();
        let Some(binding) = tokens
            .as_ref()
            .and_then(|tokens| tokens.source_binding.as_ref())
        else {
            return false;
        };
        if self.plain_command_dispatch {
            return false;
        }
        let Some((image, words, offset)) = binding.retained_original_compiler_source() else {
            return false;
        };
        let Some(dialect) = binding.compiler_word_dialect() else {
            return false;
        };
        let Some(original) = crate::registry_invocation::original_native_compiler_words(
            image,
            words,
            offset,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ) else {
            return false;
        };
        let Some(entry) = self.native_entry else {
            return false;
        };
        let Ok(crate::native_byte_compilation::NativeByteCommandPlan::Registered(plan)) =
            crate::native_byte_compilation::native_byte_command_plan(
                &original,
                entry,
                self.registry,
                self.native_compilation,
            )
        else {
            return false;
        };
        if plan.spec.grammar
            != tcl_registry::native_compilation::NativeCompilationGrammar::Expression
        {
            return false;
        }
        self.emit_native_words(&original);
        true
    }

    fn emit_native_original_preparation_invocation(
        &mut self,
        tokens: Option<&crate::ir::CommandTokens>,
    ) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let original = self.invocation_tokens.clone();
        let Some(tokens) = tokens.or(original.as_deref()) else {
            return false;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some((image, words, offset)) = binding.original_compiler_source(tokens) else {
            return false;
        };
        let Some(dialect) = binding.compiler_word_dialect() else {
            return false;
        };
        let Some(original) = crate::registry_invocation::original_native_compiler_words(
            image,
            words,
            offset,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ) else {
            return false;
        };
        let Some(entry) = self.native_entry else {
            return false;
        };
        let Ok(crate::native_byte_compilation::NativeByteCommandPlan::Registered(plan)) =
            crate::native_byte_compilation::native_byte_command_plan(
                &original,
                entry,
                self.registry,
                self.native_compilation,
            )
        else {
            return false;
        };
        if !plan.spec.requires_original_word_preparation() {
            return false;
        }
        self.emit_native_words(&original);
        true
    }

    /// Emit one original command through the selected native compiler owner.
    pub(super) fn emit_native_words(&mut self, words: &[tcl_lexer::NativeWord]) {
        let Some(first) = words.first() else {
            return;
        };
        let command = tcl_lexer::NativeScriptCommandWords {
            span: tcl_lexer::Span::new(first.span().start(), words.last().unwrap().span().end()),
            words: words.to_vec(),
        };
        self.run_native_emission(vec![NativeEmissionTask::Command(command)]);
    }

    pub(super) fn emit_executable_arena(&mut self, arena: &tcl_lexer::ExecutablePartArena) {
        let arena = std::rc::Rc::new(arena.clone());
        let root = arena.root();
        self.run_native_emission(vec![NativeEmissionTask::List(arena, root, 0)]);
    }

    /// Emit one original lexical word through the same heap work list.
    pub(super) fn emit_original_native_word(&mut self, word: &tcl_lexer::NativeWord) {
        self.run_native_emission(vec![NativeEmissionTask::Word(word.clone())]);
    }

    fn run_native_emission(&mut self, mut pending: Vec<NativeEmissionTask>) {
        use NativeEmissionTask as Task;
        while let Some(task) = pending.pop() {
            match task {
                selected @ (Task::PrivateExpressionNumber(..)
                | Task::PrivateLogicalBoolean85(..)
                | Task::NativeCommandLiteral(..)
                | Task::RegisteredExpressionNumber(..)
                | Task::RegisteredExpressionBoolean84(..)
                | Task::FixedMathCall(..)
                | Task::PrivateInteger(..)
                | Task::PrivateList(..)
                | Task::ErrorReturn
                | Task::Operation(..)
                | Task::NativeListIndex(..)
                | Task::NativeListRange(..)
                | Task::Literal(..)
                | Task::PrivateReturnOptions(..)
                | Task::PoolLiteral(..)
                | Task::Label(..)) => self.emit_native_literal_task(selected),
                selected @ (Task::ControlContext(..)
                | Task::ControlScript(..)
                | Task::BeginNativeCatch(..)
                | Task::BeginNativeCatchAt(..)
                | Task::BeginSpeculativeNativeCompilation(..)
                | Task::FinishSpeculativeNativeCatch(..)
                | Task::EndNativeCatch
                | Task::BeginNativeCatchBranch(..)
                | Task::EndNativeCatchBranch) => {
                    self.emit_native_control_task(selected, &mut pending);
                }
                selected @ (Task::NativeArrayEachStart(..)
                | Task::NativeEachStart(..)
                | Task::RestoreSource(..)
                | Task::DeclareNativeTemporary(..)
                | Task::NativeTemporaryOperation(..)
                | Task::DeclareNamespaceLocal(..)
                | Task::NamespaceLocalOperation(..)
                | Task::NamespaceGenericRollback(..)) => {
                    self.emit_native_binding_task(selected, &mut pending);
                }
                selected @ (Task::SwitchOperation(..) | Task::SwitchTable(..)) => {
                    self.emit_native_switch_task(selected);
                }
                selected @ (Task::ExpressionSyntax(..) | Task::Syntax(..)) => {
                    self.emit_native_syntax_task(selected, &mut pending);
                }
                selected @ (Task::Word(..)
                | Task::List(..)
                | Task::Script(..)
                | Task::Command(..)) => self.emit_native_source_task(selected, &mut pending),
                selected @ Task::Part(..) => self.emit_native_part_task(selected, &mut pending),
                selected @ Task::ExpressionNode(..) => {
                    self.emit_native_expression_task(selected, &mut pending);
                }
            }
        }
    }

    fn emit_native_literal_task(&mut self, task: NativeEmissionTask) {
        use NativeEmissionTask as Task;
        match task {
            Task::PrivateExpressionNumber(version, value) => {
                let index = self
                    .literals
                    .register_private_expression_number(version, value);
                self.push_native_pool_literal(index);
            }
            Task::PrivateLogicalBoolean85(value) => {
                let index = self.literals.register_private_logical_boolean85(value);
                self.push_native_pool_literal(index);
            }
            Task::NativeCommandLiteral(recipe) => {
                let index = self.intern_native_command_recipe(*recipe);
                self.push_native_pool_literal(index);
            }
            Task::RegisteredExpressionNumber(version, bytes, value) => {
                let index = self
                    .literals
                    .intern_expression_number(&bytes, version, value);
                self.push_native_pool_literal(index);
            }
            Task::RegisteredExpressionBoolean84(bytes) => {
                let index = self.literals.intern_expression_boolean84(&bytes);
                self.push_native_pool_literal(index);
            }
            Task::FixedMathCall(binding, argc) => {
                let index = self.emit(Op::CALL_FUNC1, vec![Operand::Imm(i32::from(argc))]);
                self.instructions[index].native_fixed_math_call = Some(binding);
            }
            Task::PrivateInteger(value) => {
                let slot = self.literals.register_private_integer(value);
                self.push_native_pool_literal(slot);
            }
            Task::PrivateList(members, protocol) => {
                let slot = self
                    .literals
                    .register_private_constant_list(&members, protocol);
                self.push_native_pool_literal(slot);
            }
            Task::ErrorReturn => self.emit_native_error_return(""),
            Task::Operation(op, operands) => {
                self.emit(op, operands);
            }
            Task::NativeListRange(range) => {
                let instruction = self.emit(
                    Op::LIST_RANGE_IMM,
                    vec![
                        Operand::Imm(range.first.encoded()),
                        Operand::Imm(range.last.encoded()),
                    ],
                );
                self.instructions[instruction].native_list_range = Some(range);
            }
            Task::NativeListIndex(index) => {
                let instruction =
                    self.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(index.encoded())]);
                self.instructions[instruction].native_list_index = Some(index);
            }
            Task::Literal(bytes) => {
                self.push_lit_bytes_exact(&bytes);
            }
            Task::PrivateReturnOptions(recipe) => {
                let index = self.literals.register_private_return_options(recipe);
                self.push_native_pool_literal(index);
            }
            Task::PoolLiteral(index) => {
                self.push_native_pool_literal(index);
            }
            Task::Label(label) => self.place_label(&label),
            _ => unreachable!("native literal task dispatch"),
        }
    }

    fn emit_native_control_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ControlContext(context) => self.native_compilation = context,
            Task::ControlScript(image, body, context, config) => {
                let saved = self.native_compilation;
                let entered = match context {
                    tcl_registry::native_compilation::NativeCompiledBodyContext::Inherit => saved,
                    tcl_registry::native_compilation::NativeCompiledBodyContext::Loop => {
                        tcl_registry::native_compilation::NativeCompilationContext {
                            loop_depth: saved.loop_depth + 1,
                            ..saved
                        }
                    }
                    tcl_registry::native_compilation::NativeCompiledBodyContext::ExceptionRange => {
                        saved.with_inline_exception_range()
                    }
                };
                self.native_compilation = entered;
                pending.push(Task::ControlContext(saved));
                if let Some(span) = body.script {
                    pending.push(Task::Script(image, span, config));
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::BeginNativeCatch(handler) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
                self.catch_depth += 1;
            }
            Task::BeginNativeCatchAt(handler, start, end) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
                self.instructions[index].catch_start = Some(start);
                self.instructions[index].catch_end = Some(end);
                self.catch_depth += 1;
            }
            Task::BeginSpeculativeNativeCompilation(scope) => {
                self.native_speculative_compilations.push(scope);
            }
            Task::FinishSpeculativeNativeCatch(saved, command, scope, context, catch_depth) => {
                let active = self
                    .native_speculative_compilations
                    .pop()
                    .expect("active speculative catch compiler");
                assert!(std::rc::Rc::ptr_eq(&active, &scope));
                self.native_compilation = context;
                self.catch_depth = catch_depth;
                if scope.get() {
                    self.instructions.truncate(saved.instructions);
                    self.label_positions = saved.labels;
                    self.inline_loop_regions.truncate(saved.loop_regions);
                    self.cmd_index = saved.command_index;
                    self.schedule_native_generic(command, pending);
                }
            }
            Task::EndNativeCatch => {
                self.emit(Op::END_CATCH, vec![]);
                self.catch_depth -= 1;
            }
            Task::BeginNativeCatchBranch(handler) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
            }
            Task::EndNativeCatchBranch => {
                self.emit(Op::END_CATCH, vec![]);
            }
            _ => unreachable!("native control task dispatch"),
        }
    }

    fn emit_native_binding_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::NativeArrayEachStart(version, variables) => {
                let Some(variables) = variables
                    .iter()
                    .map(|slot| slot.get())
                    .collect::<Option<Vec<_>>>()
                else {
                    self.refuse_native_dependency();
                    return;
                };
                let index = self.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
                self.instructions[index].native_each =
                    Some(std::sync::Arc::new(tcl_bytecode::NativeEachAuxiliary {
                        version,
                        variables: vec![variables],
                        temporaries: Vec::new(),
                    }));
            }
            Task::NativeEachStart(version, names, temporaries, collect) => {
                let variables = names
                    .iter()
                    .map(|group| {
                        group
                            .iter()
                            .map(|name| {
                                self.compiled_variable_protocol
                                    .and_then(|protocol| self.lvt.find_native(protocol, name))
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>();
                let temporaries = temporaries
                    .iter()
                    .map(|slot| slot.get())
                    .collect::<Option<Vec<_>>>();
                if let (Some(variables), Some(temporaries)) = (variables, temporaries) {
                    let index = self.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
                    self.instructions[index].native_each =
                        Some(std::sync::Arc::new(tcl_bytecode::NativeEachAuxiliary {
                            version,
                            variables,
                            temporaries,
                        }));
                    self.instructions[index].foreach_collect = collect;
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::RestoreSource(source) => {
                self.source = source.image;
                self.line_index = source.lines;
                self.current_span = source.span;
                self.current_span_is_command = source.command;
                self.exact_command_source = source.exact_command;
                self.current_command_namespace = source.namespace;
                self.current_command_namespace_context = source.namespace_context;
            }
            Task::DeclareNativeTemporary(slot) => {
                slot.set(Some(self.lvt.intern_anonymous()));
            }
            Task::NativeTemporaryOperation(op, slot, mut operands) => {
                if let Some(slot) = slot.get() {
                    let op = match (op, slot >= 256) {
                        (Op::LOAD_SCALAR1, true) => Op::LOAD_SCALAR4,
                        (Op::STORE_SCALAR1, true) => Op::STORE_SCALAR4,
                        _ => op,
                    };
                    operands.push(Operand::Imm(super::bytecode_imm(slot)));
                    self.emit(op, operands);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::DeclareNamespaceLocal(name) => {
                if self.command_variable_slot(&name).is_none() {
                    self.refuse_native_dependency();
                }
            }
            Task::NamespaceLocalOperation(op, name) => {
                let slot = self
                    .compiled_variable_protocol
                    .and_then(|protocol| self.lvt.find_native(protocol, &name));
                if let Some(slot) = slot {
                    let op = match (op, slot >= 256) {
                        (Op::LOAD_SCALAR1, true) => Op::LOAD_SCALAR4,
                        (Op::STORE_SCALAR1, true) => Op::STORE_SCALAR4,
                        _ => op,
                    };
                    self.emit(op, vec![Operand::Imm(super::bytecode_imm(slot))]);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::NamespaceGenericRollback(saved, command) => {
                self.instructions.truncate(saved.instructions);
                self.label_positions = saved.labels;
                self.inline_loop_regions.truncate(saved.loop_regions);
                self.cmd_index = saved.command_index;
                self.schedule_native_generic(command, pending);
            }
            _ => unreachable!("native binding task dispatch"),
        }
    }

    fn emit_native_switch_task(&mut self, task: NativeEmissionTask) {
        use NativeEmissionTask as Task;
        match task {
            Task::SwitchOperation(op, operands, version) => {
                let index = self.emit(op, operands);
                self.instructions[index].native_switch_version = Some(version);
            }
            Task::SwitchTable(version, numeric, bytes, integers) => {
                let index = self.emit(Op::JUMP_TABLE, vec![Operand::Imm(0)]);
                let instruction = &mut self.instructions[index];
                instruction.native_switch_version = Some(version);
                if numeric {
                    instruction.native_switch_integers = Some(integers);
                } else {
                    instruction.native_switch_bytes = Some(bytes);
                }
            }
            _ => unreachable!("native switch task dispatch"),
        }
    }

    fn emit_native_syntax_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ExpressionSyntax(message, error_code) => {
                if let Some(failed) = self.native_speculative_compilations.last() {
                    failed.set(true);
                    while !pending.last().is_some_and(|task| {
                        matches!(task,
                        Task::FinishSpeculativeNativeCatch(_, _, scope, _, _)
                            if std::rc::Rc::ptr_eq(scope, failed))
                    }) {
                        if pending.pop().is_none() {
                            break;
                        }
                    }
                    return;
                }
                let syntax = self.invocation_dialect.and_then(|dialect| {
                    dialect.native_return_options_application(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                    )
                });
                let message_index =
                    if syntax.is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::syntax_options_share_message) {
                        // TclAddLiteralObj keeps the original error message out
                        // of C9.1's global literal registration table.
                        self.literals.register_unshared(&message)
                    } else {
                        self.literals.intern_bytes(&message)
                    };
                self.push_native_pool_literal(message_index);
                let protocol = self
                    .invocation_dialect
                    .and_then(tcl_registry::InvocationDialect::native_string_protocol);
                if let Some(protocol) = protocol {
                    let options =
                        tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                            protocol,
                            words: vec![
                                tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                                    pieces: vec![b"-errorcode".to_vec()],
                                    composite: false,
                                },
                                tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                                    pieces: vec![error_code.unwrap_or_else(|| b"NONE".to_vec())],
                                    composite: false,
                                },
                            ],
                            code: 0,
                            level: 1,
                            size: 1,
                        };
                    let index = self.literals.register_private_return_options(options);
                    if syntax.is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::syntax_options_share_message) {
                        self.literals.retain_syntax_error_info(index, message_index);
                    }
                    self.push_native_pool_literal(index);
                    self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::Syntax(message) => {
                if let Some(failed) = self.native_speculative_compilations.last() {
                    failed.set(true);
                    while !pending.last().is_some_and(|task| {
                        matches!(task,
                        Task::FinishSpeculativeNativeCatch(_, _, scope, _, _)
                            if std::rc::Rc::ptr_eq(scope, failed))
                    }) {
                        if pending.pop().is_none() {
                            break;
                        }
                    }
                } else {
                    self.emit_script_parse_error(message.as_bytes());
                }
            }
            _ => unreachable!("native syntax task dispatch"),
        }
    }

    fn emit_native_source_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::Word(word) => {
                if word.group().kind == tcl_lexer::WordKind::Braced {
                    let content = word.content_span().expect("validated native word geometry");
                    let bytes = tcl_syntax::backslash::source_braced_word_bytes(
                        &word.image().bytes()[content.as_range()],
                        word.image().channel(),
                        word.config().brace_backslash_newline,
                    );
                    self.push_lit_bytes_exact(&bytes);
                } else {
                    let arena = std::rc::Rc::new(word.executable_parts().clone());
                    let root = arena.root();
                    pending.push(Task::List(arena, root, 0));
                }
            }
            Task::List(arena, id, position) => {
                let parts = arena.list(id);
                if parts.is_empty() {
                    self.push_lit_bytes_exact(b"");
                    return;
                }
                if position + 1 < parts.len() {
                    pending.push(Task::List(std::rc::Rc::clone(&arena), id, position + 1));
                }
                if position > 0 {
                    pending.push(Task::Operation(Op::STR_CONCAT1, vec![Operand::Imm(2)]));
                }
                pending.push(Task::Part(arena, id, position));
            }
            Task::Script(image, region, config) => {
                // Jim interprets bracket scripts; C compiles their original
                // words within this unit and shares its local allocation.
                if self.invocation_dialect.is_some_and(|dialect| {
                    dialect.family() == Some(tcl_dialect::model::Family::Jim)
                }) {
                    self.push_lit_bytes_exact(&image.bytes()[region.as_range()]);
                    self.emit(Op::EVAL_STK, vec![]);
                    return;
                }
                if self.native_entry.is_none() {
                    if let Ok(text) = std::str::from_utf8(&image.bytes()[region.as_range()]) {
                        self.emit_inline_cmd_subst(text);
                    } else {
                        self.refuse_native_dependency();
                        self.push_lit_bytes_exact(b"");
                    }
                    return;
                }
                let Ok(plan) = tcl_lexer::native_script_words_in(image, region, config) else {
                    self.refuse_native_dependency();
                    self.push_lit_bytes_exact(b"");
                    return;
                };
                if let Some(tail) = plan.fatal_tail {
                    pending.push(Task::Syntax(tail.cut.message));
                } else if plan.commands.is_empty() {
                    self.push_lit_bytes_exact(b"");
                }
                let count = plan.commands.len();
                for (index, command) in plan.commands.into_iter().enumerate().rev() {
                    if index + 1 < count || plan.fatal_tail.is_some() {
                        pending.push(Task::Operation(Op::POP, vec![]));
                    }
                    pending.push(Task::Command(command));
                }
            }
            Task::Command(command) => self.schedule_native_command(command, pending),
            _ => unreachable!("native source task dispatch"),
        }
    }

    fn emit_native_part_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_lexer::{ExecutablePart, ExecutableText};
        match task {
            Task::Part(arena, id, position) => {
                let part = &arena.list(id)[position];
                match &part.part {
                    ExecutablePart::Text(ExecutableText::Original) => {
                        let bytes = tcl_syntax::backslash::source_literal_bytes(
                            arena
                                .bytes(part.span)
                                .expect("original arena text geometry"),
                            arena.image().channel(),
                        );
                        self.push_lit_bytes_exact(&bytes);
                    }
                    ExecutablePart::Text(ExecutableText::Decoded(_)) => {
                        let bytes = self.source_string_protocol.and_then(|protocol| {
                            tcl_syntax::backslash::native_arena_text(
                                &arena,
                                part,
                                self.lexer_config().escapes,
                                protocol,
                            )
                            .ok()
                        });
                        if let Some(bytes) = bytes {
                            self.push_lit_bytes_exact(&bytes);
                        } else {
                            self.refuse_native_dependency();
                            self.push_lit_bytes_exact(b"");
                        }
                    }
                    ExecutablePart::Variable { name, index } => {
                        let name = tcl_syntax::backslash::source_literal_bytes(
                            arena.bytes(*name).expect("original arena variable name"),
                            arena.image().channel(),
                        );
                        let (op, operands) =
                            self.prepare_original_variable_load(&name, index.is_some());
                        if let Some(index) = index {
                            pending.push(Task::Operation(op, operands));
                            pending.push(Task::List(std::rc::Rc::clone(&arena), *index, 0));
                        } else {
                            self.emit(op, operands);
                        }
                    }
                    ExecutablePart::Command { body } => {
                        pending.push(Task::Script(
                            arena.image().clone(),
                            *body,
                            self.lexer_config(),
                        ));
                    }
                    ExecutablePart::Expression { expression } => {
                        self.push_lit_bytes_exact(
                            arena.bytes(*expression).expect("native arena expression"),
                        );
                        self.emit(Op::EXPR_STK, vec![]);
                    }
                    ExecutablePart::ParseError(message) => pending.push(Task::Syntax(message)),
                }
            }
            _ => unreachable!("native part task dispatch"),
        }
    }

    fn emit_native_expression_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ExpressionNode(program, node) => {
                self.schedule_native_expression_node(program, node, pending);
            }
            _ => unreachable!("native expression task dispatch"),
        }
    }

    /// A reached native script parse failure uses the same original-byte
    /// presentation in script, bracket, and structured-body compilation.
    pub(super) fn emit_script_parse_error(&mut self, message: &[u8]) {
        self.push_lit_bytes_exact(message);
        self.push_lit_bytes_exact(b"-errorcode NONE");
        self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
    }

    fn schedule_native_command(
        &mut self,
        command: tcl_lexer::NativeScriptCommandWords,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use crate::native_byte_compilation::{NativeByteCommandPlan, native_byte_command_plan};
        let Some(first) = command.words.first() else {
            return;
        };
        let image = first.image().clone();
        let Some(bytes) = image.bytes().get(command.span.as_range()) else {
            self.refuse_native_dependency();
            return;
        };
        let exact = tcl_lexer::SourceImage::from_bytes(bytes, image.channel());
        pending.push(NativeEmissionTask::RestoreSource(NativeEmissionSource {
            image: self.source.clone(),
            lines: self.line_index.clone(),
            span: self.current_span,
            command: self.current_span_is_command,
            exact_command: self.exact_command_source.clone(),
            namespace: self.current_command_namespace.clone(),
            namespace_context: self.current_command_namespace_context.clone(),
        }));
        if self.source != image {
            self.line_index = Some(tcl_lexer::LineIndex::from_bytes(image.bytes()));
            self.source = image;
        }
        self.set_command_source_span(command.span);
        self.exact_command_source = Some(exact);
        if let Some(namespace) = self.native_entry.and_then(|entry| {
            entry
                .namespaces
                .iter()
                .find(|namespace| namespace.token == entry.current_namespace)
        }) {
            self.current_command_namespace = namespace.path.clone();
            self.current_command_namespace_context = self.native_entry.and_then(|entry| {
                entry
                    .retained_namespace_context(namespace.token)
                    .ok()
                    .map(tcl_runtime_api::CompiledNamespaceContext::Native)
            });
        }
        let plan = if self
            .invocation_dialect
            .is_some_and(|dialect| dialect.family() == Some(tcl_dialect::model::Family::Jim))
        {
            Ok(NativeByteCommandPlan::Generic)
        } else if let Some(entry) = self.native_entry {
            native_byte_command_plan(
                &command.words,
                entry,
                self.registry,
                self.native_compilation,
            )
        } else {
            self.refuse_native_dependency();
            Ok(NativeByteCommandPlan::Generic)
        };
        match plan {
            Ok(NativeByteCommandPlan::Named(plan)) => {
                if !self.schedule_native_named(&command, &plan, pending) {
                    self.refuse_native_dependency();
                    pending.push(NativeEmissionTask::Literal(Vec::new()));
                }
            }
            Ok(NativeByteCommandPlan::Registered(plan)) => {
                if !self.schedule_native_registered(&command, &plan, pending) {
                    self.refuse_native_dependency();
                    pending.push(NativeEmissionTask::Literal(Vec::new()));
                }
            }
            Ok(NativeByteCommandPlan::Generic) => {
                self.schedule_native_generic(command, pending);
            }
            Err(_) => {
                self.refuse_native_dependency();
                pending.push(NativeEmissionTask::Literal(Vec::new()));
            }
        }
    }

    fn schedule_native_generic(
        &mut self,
        command: tcl_lexer::NativeScriptCommandWords,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        let boundary = self.emit(Op::NOP, vec![]);
        self.instructions[boundary].source_command_boundary =
            tcl_bytecode::SourceCommandBoundary::Start;
        self.instructions[boundary].no_fold = true;
        let expanded = command.words.iter().any(|word| word.group().expand);
        let count = super::bytecode_imm(command.words.len());
        pending.push(NativeEmissionTask::Operation(
            if expanded {
                Op::INVOKE_EXPANDED
            } else if count < 256 {
                Op::INVOKE_STK1
            } else {
                Op::INVOKE_STK4
            },
            if expanded {
                vec![]
            } else {
                vec![Operand::Imm(count)]
            },
        ));
        let command_head = self.intern_original_native_command_literal(&command.words);
        for (index, word) in command.words.into_iter().enumerate().rev() {
            if expanded && word.group().expand {
                pending.push(NativeEmissionTask::Operation(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(super::bytecode_imm(index + 1))],
                ));
            }
            if index == 0
                && let Some(head) = command_head
            {
                pending.push(NativeEmissionTask::PoolLiteral(head));
            } else {
                pending.push(NativeEmissionTask::Word(word));
            }
        }
        if expanded {
            self.emit(Op::EXPAND_START, vec![]);
        }
    }

    fn schedule_native_named(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteNamedInvocation,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_compilation::NativeNamedInvocationProtocol;
        use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
        let Some(entry) = self.native_entry else {
            return false;
        };
        let (Some(point), Some(names)) = (entry.execution_point, entry.name_protocol) else {
            return false;
        };
        let Some(namespace) = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == entry.current_namespace)
        else {
            return false;
        };
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: entry.interpreter,
            namespace_token: entry.current_namespace,
            entry_epoch: entry.epoch,
            namespace_path: namespace.path.clone(),
        };
        let Ok(literal) = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
            point, names, context, &plan.recipe.name, plan.prerequisite.clone(),
        ) else { return false; };
        self.retain_entry_named_compiler_prerequisite(&std::sync::Arc::new(
            plan.prerequisite.clone(),
        ));
        let rewrite = plan.recipe.protocol == NativeNamedInvocationProtocol::EnsembleRewrite;
        let mut operations = Vec::new();
        if !rewrite {
            operations.push(NativeEmissionTask::NativeCommandLiteral(Box::new(
                literal.clone(),
            )));
        }
        for word in &plan.recipe.words {
            operations.push(match word {
                NativeNamedInvocationWord::Original(operand) => {
                    let Some(task) =
                        Self::native_namespace_word_task(&command.words, operand.clone())
                    else {
                        return false;
                    };
                    task
                }
                NativeNamedInvocationWord::Replacement(bytes) => {
                    NativeEmissionTask::Literal(bytes.clone())
                }
            });
        }
        if rewrite {
            operations.push(NativeEmissionTask::NativeCommandLiteral(Box::new(literal)));
            operations.push(NativeEmissionTask::Operation(
                Op::INVOKE_REPLACE,
                vec![
                    Operand::Imm(super::bytecode_imm(plan.recipe.words.len())),
                    Operand::Imm(super::bytecode_imm(plan.recipe.arguments_from + 1)),
                ],
            ));
        } else {
            let count = super::bytecode_imm(plan.recipe.words.len() + 1);
            operations.push(NativeEmissionTask::Operation(
                if count < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                },
                vec![Operand::Imm(count)],
            ));
        }
        let end = self.fresh_label("native_named_command_end");
        let start = self.emit(Op::NOP, vec![]);
        self.instructions[start].source_command_boundary =
            tcl_bytecode::SourceCommandBoundary::Start;
        self.instructions[start].no_fold = true;
        self.instructions[start].native_compiler_selection = Some(tcl_bytecode::NativeCompilerSelectionSite {
            prerequisite: tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(std::sync::Arc::new(plan.prerequisite.clone())),
            end: end.clone(),
        });
        pending.push(NativeEmissionTask::Label(end));
        pending.extend(operations.into_iter().rev());
        true
    }

    /// Retain the actual compiler token before its original operands execute.
    /// Handler identity and value/cell selection remain independent obligations.
    fn schedule_native_registered(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(prepared) = self.prepare_native_registered(command, plan) else {
            return false;
        };
        let mut operations = Vec::new();
        if !self.append_native_registered_tasks(
            command,
            plan,
            prepared.version,
            prepared.instruction,
            &mut operations,
        ) {
            return false;
        }
        self.install_native_registered_boundary(
            command,
            plan,
            prepared.boundary,
            pending,
            operations,
        )
    }

    fn prepare_native_registered(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
    ) -> Option<PreparedNativeRegistered> {
        use tcl_registry::native_compiler_words::NativeCompilerWords;
        let entry = self.native_entry?;
        let version = entry
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)?;
        let protocol = self.source_string_protocol?;
        let words = NativeCompilerWords::capture(&command.words, protocol).ok()?;
        let guard = Self::native_registered_guard(plan)?;
        let head = words.literal(0)?;
        let compiler = plan.binding.compiler.clone()?;
        if compiler.ensemble.is_some() || !plan.dependencies.is_empty() {
            return None;
        }
        let namespace = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == entry.current_namespace)?;
        let prerequisite = plan.prerequisite.clone().unwrap_or_else(|| {
            tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
                interpreter: entry.interpreter,
                lookup_namespace_token: entry.current_namespace,
                invocation_word: tcl_runtime_api::NameBytes::from(head),
                slot: plan.binding.slot.clone(),
                namespace_token: plan.binding.namespace_token,
                token: plan.binding.token,
                implementation_generation: plan.binding.implementation_generation,
                compiler,
                selected_worker: None,
                nested_compilers: Vec::new(),
                guard: crate::registry_invocation::native_command_binding_guard(guard),
            }
        });
        let instruction = plan.original_instruction(&command.words)?.clone();
        Some(PreparedNativeRegistered {
            version,
            instruction,
            boundary: NativeRegisteredBoundary {
                namespace: namespace.path.clone(),
                prerequisite,
            },
        })
    }

    fn native_registered_guard(
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationGuard> {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        let guard =
            match plan.selection {
                NativeCompilationSelection::Inline { guard, .. } => guard,
                NativeCompilationSelection::Generic
                    if plan.spec.namespace_binding_kind().is_some()
                        || matches!(
                            plan.spec.grammar,
                            tcl_registry::native_compilation::NativeCompilationGrammar::Array { .. }
                        )
                        || matches!(plan.spec.grammar,
                tcl_registry::native_compilation::NativeCompilationGrammar::Conditional |
                tcl_registry::native_compilation::NativeCompilationGrammar::ForLoop |
                tcl_registry::native_compilation::NativeCompilationGrammar::Catch |
                tcl_registry::native_compilation::NativeCompilationGrammar::Try |
                tcl_registry::native_compilation::NativeCompilationGrammar::Foreach |
                tcl_registry::native_compilation::NativeCompilationGrammar::WhileLoop) =>
                {
                    tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments
                }
                _ => return None,
            };
        Some(guard)
    }

    fn append_native_registered_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
        version: tcl_dialect::TclVersion,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        match instruction {
            NativeInstructionPlan::Unset(recipe) => {
                let Some(tasks) = self.native_unset_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::Concat(recipe) => {
                let Some(tasks) = self.native_concat_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::DictionaryLookup(recipe) => {
                return Self::append_native_dictionary_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::Error(recipe) => {
                let Some(tasks) = Self::native_error_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::Coroutine(recipe) => {
                let Some(tasks) = self.native_coroutine_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::NamedInvocation(_) => return false,
            NativeInstructionPlan::Uplevel(recipe) => {
                return Self::append_native_uplevel_tasks(command, &recipe, operations);
            }
            NativeInstructionPlan::Each(recipe) => {
                *operations = match self.native_each_tasks(command, recipe) {
                    Some(operations) => operations,
                    None => return false,
                };
            }
            NativeInstructionPlan::Expression(recipe) => {
                let Some(tasks) = self.native_expression_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::Break => operations.push(Task::Operation(Op::BREAK, vec![])),
            NativeInstructionPlan::Continue => {
                operations.push(Task::Operation(Op::CONTINUE, vec![]));
            }
            NativeInstructionPlan::Control(recipe) => {
                *operations = match self.native_control_tasks(command, recipe) {
                    Some(operations) => operations,
                    None => return false,
                };
            }
            NativeInstructionPlan::NamespaceBindings(recipe) => {
                return self.append_native_namespace_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::StringTrim(recipe) => {
                return Self::append_native_string_trim_tasks(command, recipe, version, operations);
            }
            NativeInstructionPlan::StringMatch(recipe) => {
                return Self::append_native_string_match_tasks(
                    command, recipe, version, operations,
                );
            }
            NativeInstructionPlan::Upvar(recipe) => {
                return Self::append_native_upvar_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::InfoExists(recipe) => {
                return self.append_native_info_exists_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::ListOperations(recipe) => {
                return self.append_native_list_operation_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::ListIndex(recipe) => {
                return Self::append_native_list_index_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::Array(recipe) => {
                return self.append_native_array_tasks(command, recipe, version, operations);
            }
            NativeInstructionPlan::Introspection(recipe) => {
                return Self::append_native_introspection_tasks(
                    command, recipe, version, operations,
                );
            }
            NativeInstructionPlan::Scalar(recipe) => {
                return Self::append_native_scalar_tasks(command, recipe, version, operations);
            }
            NativeInstructionPlan::TclOoHelper(recipe) => {
                return Self::append_native_tcloo_tasks(command, version, recipe, operations);
            }
            NativeInstructionPlan::Try(recipe) => {
                let Some(tasks) = self.native_try_tasks(command, recipe) else {
                    return false;
                };
                *operations = tasks;
            }
            NativeInstructionPlan::Switch(recipe) => {
                *operations = self.native_switch_tasks(command, &recipe);
            }
            NativeInstructionPlan::ReturnOptions(recipe) => {
                return Self::append_native_return_tasks(command, recipe, operations);
            }
            NativeInstructionPlan::List(recipe) => {
                return self.append_native_list_tasks(command, plan, version, recipe, operations);
            }
            selected @ (NativeInstructionPlan::Load { .. }
            | NativeInstructionPlan::Store { .. }
            | NativeInstructionPlan::Increment { .. }
            | NativeInstructionPlan::Append { .. }) => {
                return self.append_native_variable_tasks(command, plan, selected, operations);
            }
        }
        true
    }

    fn append_native_dictionary_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_dictionary_compilation::NativeDictionaryLookupInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_dictionary_compilation::NativeDictionaryLookupKind;
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        let opcode = match recipe.kind {
            NativeDictionaryLookupKind::Get => Op::DICT_GET,
            NativeDictionaryLookupKind::Exists => Op::DICT_EXISTS,
            NativeDictionaryLookupKind::GetDefault => Op::DICT_GET_DEF,
        };
        let Ok(count) = i32::try_from(recipe.key_count) else {
            return false;
        };
        operations.push(Task::Operation(opcode, vec![Operand::Imm(count)]));

        true
    }

    fn append_native_list_index_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_list_index_compilation::NativeListIndexInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_list_index_compilation::NativeListIndexOperation as Operation;
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        operations.push(match recipe.operation {
            Operation::Immediate(index) => Task::NativeListIndex(index),
            Operation::Single => Task::Operation(Op::LIST_INDEX, vec![]),
            Operation::Multi(count) => Task::Operation(
                Op::LINDEX_MULTI,
                vec![Operand::Imm(
                    i32::try_from(count).expect("native list-index operand count"),
                )],
            ),
        });
        true
    }

    fn append_native_uplevel_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &tcl_registry::native_instruction_plan::NativeUplevelInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        operations.push(match recipe.level_word {
            Some(index) => Task::Word(command.words[index].clone()),
            None => Task::Literal(b"1".to_vec()),
        });
        operations.extend(
            command.words[recipe.script_words.clone()]
                .iter()
                .cloned()
                .map(Task::Word),
        );
        if recipe.script_words.len() > 1 {
            operations.push(Task::Operation(
                Op::CONCAT_STK,
                vec![Operand::Imm(super::bytecode_imm(recipe.script_words.len()))],
            ));
        }
        operations.push(Task::Operation(Op::UPLEVEL, vec![]));

        true
    }

    fn append_native_namespace_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingKind as Kind, NativeNamespaceBindingOutcome as Outcome,
        };
        let generic = recipe.outcome == Outcome::Generic;
        if recipe.outcome == Outcome::Unknown {
            return false;
        }
        if recipe.kind == Kind::Upvar {
            let Some(tasks) = Self::native_namespace_upvar_tasks(command, &recipe) else {
                return false;
            };
            *operations = tasks;
            if generic {
                let saved = NativeNamespaceRollback {
                    // The common original compiler guard is emitted
                    // before these speculative word/declaration tasks.
                    instructions: self.instructions.len() + 1,
                    labels: self.label_positions.clone(),
                    loop_regions: self.inline_loop_regions.len(),
                    command_index: self.cmd_index,
                };
                operations.push(Task::NamespaceGenericRollback(saved, command.clone()));
            } else {
                operations.push(Task::Operation(Op::POP, vec![]));
                operations.push(Task::Literal(Vec::new()));
            }
        } else {
            for visit in &recipe.visits {
                if let tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingVisit::Literal(value) = visit {
                        operations.push(Task::Literal(value.clone()));
                    }
            }
            for binding in recipe.bindings {
                operations.push(Task::DeclareNamespaceLocal(binding.local.clone()));
                let Some(name) = Self::native_namespace_word_task(&command.words, binding.name)
                else {
                    return false;
                };
                operations.push(name);
                operations.push(Task::NamespaceLocalOperation(
                    if recipe.kind == Kind::Global {
                        Op::NSUPVAR
                    } else {
                        Op::VARIABLE
                    },
                    binding.local.clone(),
                ));
                if let Some(value) = binding.value {
                    let Some(value) = Self::native_namespace_word_task(&command.words, value)
                    else {
                        return false;
                    };
                    operations.push(value);
                    operations.push(Task::NamespaceLocalOperation(
                        Op::STORE_SCALAR1,
                        binding.local,
                    ));
                    operations.push(Task::Operation(Op::POP, vec![]));
                }
            }
            if generic {
                let saved = NativeNamespaceRollback {
                    instructions: self.instructions.len() + 1,
                    labels: self.label_positions.clone(),
                    loop_regions: self.inline_loop_regions.len(),
                    command_index: self.cmd_index,
                };
                operations.push(Task::NamespaceGenericRollback(saved, command.clone()));
            } else {
                if recipe.kind == Kind::Global {
                    operations.push(Task::Operation(Op::POP, vec![]));
                }
                operations.push(Task::Literal(Vec::new()));
            }
        }

        true
    }

    fn append_native_tcloo_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        version: tcl_dialect::TclVersion,
        recipe: tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
        match recipe {
            Helper::SelfObject => operations.push(Task::Operation(Op::TCLOO_SELF, vec![])),
            Helper::SelfNamespace => {
                operations.push(Task::Operation(Op::TCLOO_SELF, vec![]));
                operations.push(Task::Operation(Op::POP, vec![]));
                operations.push(Task::Operation(Op::CURRENT_NAMESPACE, vec![]));
            }
            Helper::ObjectInfo { operation, operand } => {
                use tcl_registry::native_tcloo_compilation::NativeTclOoObjectInfo as ObjectInfo;
                let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                    return false;
                };
                operations.push(task);
                operations.push(Task::Operation(
                    match operation {
                        ObjectInfo::Class => Op::TCLOO_CLASS,
                        ObjectInfo::Namespace => Op::TCLOO_NS,
                        ObjectInfo::IsObject => Op::TCLOO_IS_OBJECT,
                        ObjectInfo::CreationId => Op::TCLOO_ID,
                    },
                    vec![],
                ));
            }
            Helper::Next { class, words, list } => {
                if let Some(steps) = list {
                    use tcl_registry::native_instruction_plan::NativeArgumentListStep;
                    for step in steps {
                        operations.push(match step {
                            NativeArgumentListStep::Word(index) => {
                                let Some(task) = Self::native_namespace_word_task(
                                    &command.words,
                                    words[index].clone(),
                                ) else {
                                    return false;
                                };
                                task
                            }
                            NativeArgumentListStep::List(count) => Task::Operation(
                                Op::LIST,
                                vec![Operand::Imm(super::bytecode_imm(count))],
                            ),
                            NativeArgumentListStep::Concat => {
                                Task::Operation(Op::LIST_CONCAT, vec![])
                            }
                        });
                    }
                    operations.push(Task::Operation(
                        if class {
                            Op::TCLOO_NEXT_CLASS_LIST
                        } else {
                            Op::TCLOO_NEXT_LIST
                        },
                        vec![],
                    ));
                } else {
                    let count = words.len();
                    for word in words {
                        let Some(task) = Self::native_namespace_word_task(&command.words, word)
                        else {
                            return false;
                        };
                        operations.push(task);
                    }
                    let opcode = match (class, version >= tcl_dialect::TclVersion::V9_1) {
                        (false, false) => Op::TCLOO_NEXT,
                        (true, false) => Op::TCLOO_NEXT_CLASS,
                        (false, true) => Op::TCLOO_NEXT4,
                        (true, true) => Op::TCLOO_NEXT_CLASS4,
                    };
                    operations.push(Task::Operation(
                        opcode,
                        vec![Operand::Imm(super::bytecode_imm(count))],
                    ));
                }
            }
        }

        true
    }

    fn append_native_return_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_return_compilation::NativeReturnInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_return_compilation::{
            NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
        };
        match &recipe.options {
            Options::StackWord(index) => {
                operations.push(Task::Word(command.words[*index].clone()));
            }
            Options::StackPairs(range) => {
                operations.extend(command.words[range.clone()].iter().cloned().map(Task::Word));
                operations.push(Task::Operation(
                    Op::LIST,
                    vec![Operand::Imm(super::bytecode_imm(range.len()))],
                ));
            }
            Options::Static(_) => {}
        }
        operations.push(match recipe.value_word {
            Some(index) => Task::Word(command.words[index].clone()),
            None => Task::Literal(Vec::new()),
        });
        match recipe.exit {
            Exit::Done => operations.push(Task::Operation(Op::DONE, vec![])),
            Exit::Fallthrough => {}
            Exit::Stack => operations.push(Task::Operation(Op::RETURN_STK, vec![])),
            Exit::Immediate => {
                let Options::Static(options) = recipe.options else {
                    unreachable!("immediate static options")
                };
                let (code, level) = (options.code, options.level);
                operations.push(Task::PrivateReturnOptions(options));
                operations.push(Task::Operation(
                    Op::RETURN_IMM,
                    vec![Operand::Imm(code), Operand::Imm(level)],
                ));
            }
        }

        true
    }

    fn append_native_list_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
        version: tcl_dialect::TclVersion,
        recipe: tcl_registry::native_compiler_words::NativeCompiledListRecipe,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_compiler_words::NativeCompiledListRecipe;
        match recipe {
            NativeCompiledListRecipe::EmptyString => operations.push(Task::Literal(Vec::new())),
            NativeCompiledListRecipe::DynamicElements => {
                operations.extend(
                    command.words[plan.operand_from..]
                        .iter()
                        .cloned()
                        .map(Task::Word),
                );
                operations.push(Task::Operation(
                    Op::LIST,
                    vec![Operand::Imm(super::bytecode_imm(
                        command.words.len() - plan.operand_from,
                    ))],
                ));
            }
            NativeCompiledListRecipe::PrivateConstant { members } => {
                let index = self.literals.register_private_constant_list(
                    &members,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                );
                operations.push(Task::PoolLiteral(index));
            }
        }
        true
    }

    fn append_native_variable_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let (target, value_word, amount_word, immediate, increment, append) = match instruction {
            NativeInstructionPlan::Load { target } => (target, None, None, None, false, None),
            NativeInstructionPlan::Store { target, value_word } => {
                (target, Some(value_word), None, None, false, None)
            }
            NativeInstructionPlan::Increment {
                target,
                amount_word,
                immediate,
            } => (target, None, amount_word, immediate, true, None),
            NativeInstructionPlan::Append { target, recipe } => {
                (target, None, None, None, false, Some(recipe))
            }
            NativeInstructionPlan::List(_) => unreachable!("List was handled above"),
            _ => return false,
        };
        let Some(target_word) = command.words.get(plan.operand_from) else {
            return false;
        };
        let (slot, array) =
            self.prepare_native_variable_tasks(target_word, target, increment, operations);
        if let Some(recipe) = append {
            return Self::append_native_append_tasks(command, &recipe, slot, array, operations);
        }
        if increment {
            return Self::append_native_increment_tasks(
                command,
                amount_word,
                immediate,
                slot,
                array,
                operations,
            );
        }
        Self::append_native_load_store_tasks(command, value_word, slot, array, operations);
        true
    }

    fn prepare_native_variable_tasks(
        &mut self,
        target_word: &tcl_lexer::NativeWord,
        target: tcl_syntax::native_variable_words::NativeVariableWordOperand,
        increment: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> (Option<usize>, bool) {
        use NativeEmissionTask as Task;
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        match target {
            NativeVariableWordOperand::Literal { name, index, .. } => {
                let slot = self
                    .command_variable_slot(&name)
                    .filter(|slot| !increment || *slot < 256);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                let array = index.is_some();
                if let Some(index) = index {
                    operations.push(Task::Literal(index));
                }
                (slot, array)
            }
            NativeVariableWordOperand::CompoundArray { name, index, .. } => {
                let slot = self
                    .command_variable_slot(&name)
                    .filter(|slot| !increment || *slot < 256);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                let index = std::rc::Rc::new(index);
                let root = index.root();
                operations.push(Task::List(index, root, 0));
                (slot, true)
            }
            NativeVariableWordOperand::DynamicWord => {
                operations.push(Task::Word(target_word.clone()));
                (None, false)
            }
        }
    }

    fn append_native_append_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &tcl_registry::native_instruction_plan::NativeAppendInstruction,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_compilation::NativeAppendKind;
        use tcl_registry::native_instruction_plan::{NativeAppendOperands, NativeArgumentListStep};
        let list = recipe.kind == NativeAppendKind::List;
        let list_values = matches!(recipe.operands, NativeAppendOperands::ListElements { .. });
        match &recipe.operands {
            NativeAppendOperands::StringObjects | NativeAppendOperands::ListElement => {
                operations.extend(
                    command.words[recipe.values.clone()]
                        .iter()
                        .cloned()
                        .map(Task::Word),
                );
            }
            NativeAppendOperands::ListElements {
                steps,
                strip_single_expanded,
            } => {
                for step in steps {
                    operations.push(match *step {
                        NativeArgumentListStep::Word(index) => {
                            Task::Word(command.words[index].clone())
                        }
                        NativeArgumentListStep::List(count) => Task::Operation(
                            Op::LIST,
                            vec![Operand::Imm(super::bytecode_imm(count))],
                        ),
                        NativeArgumentListStep::Concat => Task::Operation(Op::LIST_CONCAT, vec![]),
                    });
                }
                if *strip_single_expanded {
                    operations.push(Task::Operation(
                        Op::LIST_RANGE_IMM,
                        vec![Operand::Imm(0), Operand::Imm(tcl_bytecode::INDEX_END)],
                    ));
                }
            }
        }
        let op = match (list, list_values, array, slot) {
            (true, true, false, Some(_)) => Op::LAPPEND_LIST,
            (true, true, true, Some(_)) => Op::LAPPEND_LIST_ARRAY,
            (true, true, false, None) => Op::LAPPEND_LIST_STK,
            (true, true, true, None) => Op::LAPPEND_LIST_ARRAY_STK,
            (true, false, false, Some(slot)) if slot < 256 => Op::LAPPEND_SCALAR1,
            (true, false, false, Some(_)) => Op::LAPPEND_SCALAR4,
            (true, false, true, Some(slot)) if slot < 256 => Op::LAPPEND_ARRAY1,
            (true, false, true, Some(_)) => Op::LAPPEND_ARRAY4,
            (true, false, false, None) => Op::LAPPEND_STK,
            (true, false, true, None) => Op::LAPPEND_ARRAY_STK,
            (false, _, false, Some(slot)) if slot < 256 => Op::APPEND_SCALAR1,
            (false, _, false, Some(_)) => Op::APPEND_SCALAR4,
            (false, _, true, Some(slot)) if slot < 256 => Op::APPEND_ARRAY1,
            (false, _, true, Some(_)) => Op::APPEND_ARRAY4,
            (false, _, false, None) => Op::APPEND_STK,
            (false, _, true, None) => Op::APPEND_ARRAY_STK,
        };
        let operands = slot
            .map(|slot| vec![Operand::Imm(super::bytecode_imm(slot))])
            .unwrap_or_default();
        let count = if list { 1 } else { recipe.values.len() };
        if count > 1 {
            if slot.is_none() || array {
                return false;
            }
            operations.push(Task::Operation(
                Op::REVERSE,
                vec![Operand::Imm(super::bytecode_imm(count))],
            ));
        }
        for index in 0..count {
            operations.push(Task::Operation(op, operands.clone()));
            if index + 1 < count {
                operations.push(Task::Operation(Op::POP, vec![]));
            }
        }
        true
    }

    fn append_native_increment_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        amount_word: Option<usize>,
        immediate: Option<i32>,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        if immediate.is_none() {
            let Some(amount) = amount_word.and_then(|index| command.words.get(index)) else {
                return false;
            };
            operations.push(Task::Word(amount.clone()));
        }
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
            operands.push(Operand::Imm(super::bytecode_imm(slot)));
        }
        if let Some(immediate) = immediate {
            operands.push(Operand::Imm(immediate));
        }
        operations.push(Task::Operation(op, operands));
        true
    }

    fn append_native_load_store_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        value_word: Option<usize>,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let second = value_word.and_then(|index| command.words.get(index));
        if let Some(second) = second {
            operations.push(Task::Word(second.clone()));
        }
        let op = match (second.is_some(), array, slot) {
            (false, false, Some(slot)) if slot < 256 => Op::LOAD_SCALAR1,
            (false, false, Some(_)) => Op::LOAD_SCALAR4,
            (false, true, Some(slot)) if slot < 256 => Op::LOAD_ARRAY1,
            (false, true, Some(_)) => Op::LOAD_ARRAY4,
            (true, false, Some(slot)) if slot < 256 => Op::STORE_SCALAR1,
            (true, false, Some(_)) => Op::STORE_SCALAR4,
            (true, true, Some(slot)) if slot < 256 => Op::STORE_ARRAY1,
            (true, true, Some(_)) => Op::STORE_ARRAY4,
            (false, false, None) => Op::LOAD_STK,
            (false, true, None) => Op::LOAD_ARRAY_STK,
            (true, false, None) => Op::STORE_STK,
            (true, true, None) => Op::STORE_ARRAY_STK,
        };
        operations.push(Task::Operation(
            op,
            slot.map(|slot| vec![Operand::Imm(super::bytecode_imm(slot))])
                .unwrap_or_default(),
        ));
    }

    fn install_native_registered_boundary(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &crate::native_byte_compilation::NativeByteRegisteredCommand,
        boundary: NativeRegisteredBoundary,
        pending: &mut Vec<NativeEmissionTask>,
        operations: Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        let end = self.fresh_label("native_byte_command_end");
        let start = self.emit(Op::NOP, vec![]);
        let instruction = &mut self.instructions[start];
        instruction.source_cmd_text = tcl_lexer::SourceImage::from_bytes(
            &command.words[0].image().bytes()[command.span.as_range()],
            command.words[0].image().channel(),
        );
        instruction.source_command_namespace = boundary.namespace;
        instruction
            .source_command_namespace_context
            .clone_from(&self.current_command_namespace_context);
        instruction.source_command_boundary = tcl_bytecode::SourceCommandBoundary::Start;
        instruction.source_span = Some(command.span);
        instruction.no_fold = true;
        instruction.native_compiler_selection = Some(tcl_bytecode::NativeCompilerSelectionSite {
            prerequisite: if plan.prerequisite.is_some() {
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(
                    std::sync::Arc::new(boundary.prerequisite),
                )
            } else {
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(
                    std::sync::Arc::new(boundary.prerequisite),
                )
            },
            end: end.clone(),
        });
        pending.push(Task::Label(end));
        pending.extend(operations.into_iter().rev());
        true
    }

    fn native_concat_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_instruction_plan::NativeConcatInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use tcl_registry::native_instruction_plan::{
            NativeConcatInstruction, NativeConcatLiteralAllocation,
        };
        match recipe {
            NativeConcatInstruction::Literal { bytes, allocation } => {
                let index = match allocation {
                    NativeConcatLiteralAllocation::Registered => self.literals.intern_bytes(&bytes),
                    NativeConcatLiteralAllocation::PrivateEmpty => {
                        self.literals.register_unshared(&bytes)
                    }
                    NativeConcatLiteralAllocation::PrivateString => {
                        self.literals.register_private_concat_string(&bytes)
                    }
                };
                Some(vec![NativeEmissionTask::PoolLiteral(index)])
            }
            NativeConcatInstruction::Operands(operands) => {
                let count = operands.len();
                let mut tasks = operands
                    .into_iter()
                    .map(|operand| Self::native_namespace_word_task(&command.words, operand))
                    .collect::<Option<Vec<_>>>()?;
                tasks.push(NativeEmissionTask::Operation(
                    Op::CONCAT_STK,
                    vec![Operand::Imm(super::bytecode_imm(count))],
                ));
                Some(tasks)
            }
        }
    }

    fn native_namespace_word_task(
        words: &[tcl_lexer::NativeWord],
        operand: tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
    ) -> Option<NativeEmissionTask> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        match operand {
            NativeCompilerWordOperand::Original(index) => {
                words.get(index).cloned().map(NativeEmissionTask::Word)
            }
            NativeCompilerWordOperand::LiteralExpansion {
                original_word,
                value_span,
                value,
            } => {
                let word = words.get(original_word)?;
                word.image().bytes().get(value_span.as_range())?;
                Some(NativeEmissionTask::Literal(value))
            }
        }
    }

    /// Select a native loop opcode through the shared hook and retain its
    /// command requirement. Runtime replay must enter ordinary dispatch when
    /// a replacement command returns normally at this command boundary.
    fn emit_lexical_loop_jump(&mut self, command: &str, args: &[String]) -> bool {
        if !args.is_empty() {
            return false;
        }
        let hook = self.inline_cmd_subst_hook_candidate(command, &[]);
        let target = match hook {
            Some(InlineCodegenHookId::Break) => self.break_target.clone(),
            Some(InlineCodegenHookId::Continue) => self.continue_target.clone(),
            _ => None,
        };
        let Some(target) = target else { return false };
        if self.inline_codegen_hook(command, &[]) != hook {
            return false;
        }
        self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], command);
        true
    }
}

#[cfg(test)]
#[path = "native_concat_tests.rs"]
mod native_concat_tests;
#[cfg(test)]
#[path = "native_list_index_tests.rs"]
mod native_list_index_tests;

#[cfg(test)]
#[path = "native_variable_preparation_tests.rs"]
mod native_variable_preparation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenCtx;
    use crate::expr_ast::ExprNode;
    use crate::ir::CommandTokens;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    #[test]
    fn folded_script_without_original_words_requires_a_provider() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let body = crate::ir::Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 1),
            name: "x".to_owned(),
            name_braced: false,
            value: "CHILD".to_owned(),
            value_span: None,
        }]);
        let statement = Statement::Block {
            span: Span::new(0, 1),
            body,
            namespace: "::".to_owned(),
            tokens: None,
            error_context: Some(tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation),
        };
        ctx.emit_stmt(&statement, &mut false);
        let function = ctx.into_function_asm("unknown-body".to_owned());
        assert_eq!(
            function.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::ProviderRequired
        );
        assert!(function.instructions.is_empty());
        assert!(
            !function
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"CHILD")
        );
    }

    fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
        ctx.instructions.iter().map(|i| i.op).collect()
    }

    fn sp() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn opaque_native_statement_emits_original_literal_bytes_and_generic_dispatch() {
        let registry = CommandRegistry::build_default();
        let statement = crate::ir::native_call_for_test(b"opaque \xff");
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut generic = false;
        ctx.emit_stmt(&statement, &mut generic);
        assert!(generic);
        assert_eq!(
            opcodes(&ctx),
            [Op::NOP, Op::PUSH1, Op::PUSH1, Op::INVOKE_STK1, Op::POP]
        );
        assert_eq!(
            ctx.instructions[0].source_command_boundary,
            tcl_bytecode::SourceCommandBoundary::Start
        );
        assert_eq!(ctx.literals.entries()[0].bytes(), b"opaque");
        assert_eq!(ctx.literals.entries()[1].bytes(), b"\xff");
        assert!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::PUSH1)
                .all(|instruction| instruction.push_verbatim)
        );
    }

    #[test]
    fn native_array_indices_emit_iteratively_from_the_full_original_arena() {
        let registry = CommandRegistry::build_default();
        let source = include_bytes!("../../tests/data/native_deep_array_word.tcl");
        let statement = crate::ir::native_call_for_test(source);
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut generic = false;
        ctx.emit_stmt(&statement, &mut generic);
        assert!(generic);
        assert_eq!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::LOAD_ARRAY_STK)
                .count(),
            2000,
        );
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"x\xff")
        );
        assert_eq!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::INVOKE_STK1)
                .count(),
            1,
        );
    }

    #[test]
    fn native_registered_return_keeps_original_result_and_exception_range_exit() {
        use tcl_registry::native_compilation::NativeCompilationFrame;
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let image = tcl_lexer::SourceImage::native(&b"return {A\xff\0tail}"[..]);
            let script = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            for (procedure, catch_depth, terminal) in [
                (true, 0, Op::DONE),
                (true, 1, Op::RETURN_IMM),
                (false, 0, Op::RETURN_IMM),
            ] {
                if name == "tcl8.4" && (!procedure || catch_depth != 0) {
                    continue; // Native C8.4 deliberately invokes these returns.
                }
                let mut context = CodegenCtx::new(procedure, &[], &registry);
                context.native_entry = Some(&entry);
                context.invocation_dialect =
                    Some(tcl_registry::InvocationDialect::of_profile(profile));
                context.source_string_protocol = entry.source_string_protocol;
                context.native_compilation.frame = if procedure {
                    NativeCompilationFrame::ProcedureCode
                } else {
                    NativeCompilationFrame::ScriptCode
                };
                context.native_compilation.catch_depth = Some(catch_depth);
                context.emit_native_words(&script.commands[0].words);
                assert!(
                    !context.native_dependency_refusal,
                    "{name}/{procedure}/{catch_depth}"
                );
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.op == terminal),
                    "{name}/{procedure}/{catch_depth}"
                );
                assert!(!context.instructions.iter().any(|instruction| matches!(
                    instruction.op,
                    Op::INVOKE_STK1 | Op::INVOKE_STK4
                )));
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.native_compiler_selection.is_some())
                );
                assert!(
                    context
                        .literals
                        .entries()
                        .iter()
                        .any(|literal| literal.bytes() == b"A\xff\0tail")
                );
                if terminal == Op::RETURN_IMM {
                    let exit = context
                        .instructions
                        .iter()
                        .position(|instruction| instruction.op == terminal)
                        .unwrap();
                    assert_eq!(
                        context.instructions[exit].operands,
                        vec![Operand::Imm(0), Operand::Imm(1)]
                    );
                    assert!(matches!(
                        context.instructions[exit - 1].op,
                        Op::PUSH1 | Op::PUSH4
                    ));
                    let Operand::Imm(slot) = context.instructions[exit - 1].operands[0] else {
                        panic!("native private options PUSH")
                    };
                    assert!(
                        matches!(context.literals.entries()[usize::try_from(slot).unwrap()].allocation(),tcl_bytecode::NativeLiteralAllocation::PrivateReturnOptions(recipe) if recipe.words.is_empty() && (recipe.code,recipe.level,recipe.size)==(0,1,0))
                    );
                }
            }
        }
    }

    #[test]
    fn native_registered_return_options_keep_private_literals_and_ordered_stack_operands() {
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            for source in [
                b"return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY"
                    .as_slice(),
                b"return -options $options $result",
                b"return -code $code $result",
            ] {
                let image = tcl_lexer::SourceImage::native(source);
                let script = tcl_lexer::native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let mut context = CodegenCtx::new(true, &[], &registry);
                context.native_entry = Some(&entry);
                context.invocation_dialect =
                    Some(tcl_registry::InvocationDialect::of_profile(profile));
                context.source_string_protocol = entry.source_string_protocol;
                context.native_compilation.frame =
                    tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
                context.native_compilation.catch_depth = Some(0);
                context.emit_native_words(&script.commands[0].words);
                assert!(!context.native_dependency_refusal, "{name}/{source:?}");
                if source.starts_with(b"return -level") {
                    let instruction = context
                        .instructions
                        .iter()
                        .position(|instruction| instruction.op == Op::RETURN_IMM)
                        .unwrap();
                    assert_eq!(
                        context.instructions[instruction].operands,
                        vec![Operand::Imm(1), Operand::Imm(0)]
                    );
                    let Operand::Imm(slot) = context.instructions[instruction - 1].operands[0]
                    else {
                        panic!("private options PUSH")
                    };
                    assert!(
                        matches!(context.literals.entries()[usize::try_from(slot).unwrap()].allocation(),tcl_bytecode::NativeLiteralAllocation::PrivateReturnOptions(recipe) if (recipe.code,recipe.level,recipe.size)==(1,0,2))
                    );
                    assert!(
                        context.literals.entries()[usize::try_from(slot).unwrap()]
                            .byte_payload()
                            .is_none()
                    );
                } else if name == "tcl8.5" && source.starts_with(b"return -code") {
                    assert!(context.instructions.iter().any(|instruction| matches!(
                        instruction.op,
                        Op::INVOKE_STK1 | Op::INVOKE_STK4
                    )));
                } else {
                    assert!(
                        context
                            .instructions
                            .iter()
                            .any(|instruction| instruction.op == Op::RETURN_STK)
                    );
                    assert!(!context.instructions.iter().any(|instruction| matches!(
                        instruction.op,
                        Op::INVOKE_STK1 | Op::INVOKE_STK4
                    )));
                    if source.starts_with(b"return -code") {
                        assert!(
                            context
                                .instructions
                                .iter()
                                .any(|instruction| instruction.op == Op::LIST
                                    && instruction.operands == vec![Operand::Imm(2)])
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn native_namespace_emission_matches_all_100_instruction_and_local_windows() {
        let bodies: [&[u8]; 20] = [
            b"variable v",
            b"variable {}",
            b"variable :::",
            b"variable {a)}",
            b"variable v $val w [set x 3]",
            b"variable v 1 {a)} 2",
            b"variable ${ns}::v",
            b"variable ${ns}v",
            b"variable ${ns}::[set x 1]",
            b"variable ${ns}::v 1 ${ns}v 2",
            b"variable {*}{v 1}",
            b"variable",
            b"global v",
            b"global {}",
            b"global :::",
            b"global {a)}",
            b"global ${ns}::v",
            b"global ${ns}v",
            b"global v {a)}",
            b"global {*}{v w}",
        ];
        for (name, expected) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv"
                ),
            ),
        ] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            for (case, row) in expected.lines().skip(1).enumerate() {
                compare_native_namespace_case(
                    name,
                    profile,
                    &registry,
                    &entry,
                    case,
                    bodies[case],
                    row,
                );
            }
        }
    }

    fn compare_native_namespace_case(
        name: &str,
        profile: &tcl_dialect::DialectProfile,
        registry: &CommandRegistry,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        case: usize,
        source: &[u8],
        row: &str,
    ) {
        let columns = row.split('\t').collect::<Vec<_>>();
        let image = tcl_lexer::SourceImage::native(source);
        let script = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        if columns[2] == "-1" {
            assert!(
                script.fatal_tail.is_some(),
                "{name}/{case}: authentic unsupported expansion syntax"
            );
            assert!(script.commands.is_empty());
            return;
        }
        let mut context = CodegenCtx::new(true, &["ns", "val"], registry);
        context.native_entry = Some(entry);
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.emit_native_words(&script.commands[0].words);
        assert!(!context.native_dependency_refusal, "{name}/{case}");
        for (column, op) in [(2, Op::VARIABLE), (3, Op::NSUPVAR)] {
            assert_eq!(
                context.instructions.iter().filter(|i| i.op == op).count(),
                columns[column].parse::<usize>().unwrap(),
                "{name}/{case}/{op:?}"
            );
        }
        let locals = context
            .lvt
            .native_slot_names()
            .iter()
            .map(|name| {
                use std::fmt::Write;
                name.as_ref()
                    .unwrap()
                    .as_bytes()
                    .iter()
                    .fold(String::new(), |mut hex, byte| {
                        write!(hex, "{byte:02x}").unwrap();
                        hex
                    })
            })
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(locals, columns[4], "{name}/{case}");
        if case == 15 && name != "tcl8.4" {
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == b"::"),
                "{name}: failed first global tail retains its implicit literal"
            );
        }
        if case == 5 && name != "tcl8.4" {
            assert_native_namespace_rollback(&context, entry, name, case);
        }
    }

    fn assert_native_namespace_rollback(
        context: &CodegenCtx<'_>,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        name: &str,
        case: usize,
    ) {
        let selections = context
            .instructions
            .iter()
            .filter_map(|instruction| {
                instruction
                    .native_compiler_selection
                    .as_ref()
                    .map(|selection| (instruction, selection))
            })
            .collect::<Vec<_>>();
        let [(instruction, selection)] = selections.as_slice() else {
            panic!("{name}/{case}: original compiler selection before rollback")
        };
        assert_eq!(instruction.op, Op::NOP, "{name}/{case}");
        let tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(
            required,
        ) = &selection.prerequisite
        else {
            panic!("{name}/{case}: original variable compiler registration")
        };
        assert_eq!(
            required.guard,
            tcl_runtime_api::CommandBindingGuard::BeforeArguments,
            "{name}/{case}"
        );
        assert_eq!(required.invocation_word.as_bytes(), b"variable");
        let original = entry
            .lookup_command_bytes(
                required.lookup_namespace_token,
                required.invocation_word.as_bytes(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(original.compiler.as_ref(), Some(&required.compiler));
        assert_eq!(original.token, required.token);
        assert_eq!(original.namespace_token, required.namespace_token);
        assert!(
            context
                .instructions
                .iter()
                .any(|i| matches!(i.op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
        );
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"1")
        );
    }

    #[test]
    fn original_byte_commands_retain_exact_boundaries_and_restore_enclosing_source() {
        let registry = CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let image =
            tcl_lexer::SourceImage::native(b"opaque \"first\xff\";\nother \"second\"".as_slice());
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let mut context = CodegenCtx::new(false, &[], &registry);
        context.native_entry = Some(&entry);
        context.set_source("enclosing command");
        context.set_command_source_span(Span::new(0, 17));
        for command in &plan.commands {
            context.emit_native_words(&command.words);
        }
        let boundaries = context
            .instructions
            .iter()
            .filter(|instruction| instruction.source_command_boundary.is_start())
            .collect::<Vec<_>>();
        assert_eq!(boundaries.len(), 2);
        for (instruction, command) in boundaries.iter().zip(&plan.commands) {
            assert_eq!(
                instruction.source_cmd_text.bytes(),
                &image.bytes()[command.span.as_range()]
            );
            assert_eq!(instruction.source_span, Some(command.span));
        }
        assert_eq!(boundaries[1].source_line, 2);
        let trailing = context.emit(Op::NOP, vec![]);
        assert_eq!(
            context.instructions[trailing].source_cmd_text.bytes(),
            b"enclosing command"
        );
        assert_eq!(context.source_image().bytes(), b"enclosing command");
    }

    #[test]
    fn retained_quoted_components_execute_but_braced_and_rewritten_values_do_not() {
        let registry = CommandRegistry::build_default();
        let source = "list \"boom-$x\" {boom-$x}";
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let segment = crate::segmenter::segment_commands(source).remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        let quoted = &tokens.words()[1];
        let mut active = CodegenCtx::new(true, &[], &registry);
        active.emit_word_from_source(&quoted.legacy_text(), false, Some(quoted));
        assert!(opcodes(&active).contains(&Op::LOAD_SCALAR1));
        assert!(opcodes(&active).contains(&Op::STR_CONCAT1));

        let braced = &tokens.words()[2];
        let mut literal = CodegenCtx::new(true, &[], &registry);
        literal.emit_word_from_source(&braced.legacy_text(), true, Some(braced));
        assert_eq!(opcodes(&literal), vec![Op::PUSH1]);
        assert!(literal.instructions[0].push_verbatim);

        let mut rewritten = CodegenCtx::new(true, &[], &registry);
        rewritten.emit_word_from_source("REWRITTEN", false, Some(quoted));
        assert_eq!(opcodes(&rewritten), vec![Op::PUSH1]);
    }

    #[test]
    fn literal_script_value_retains_its_word_line_separately_from_the_command() {
        let source = "proc p \\\n{} \\\n{\nerror BOOM\n}";
        let registry = CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::default();
        let segment = crate::segmenter::segment_commands(source).remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        let word = &tokens.words()[3];
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.set_source(source);
        ctx.set_command_source_span(segment.span);
        ctx.emit_word_from_source(&word.legacy_text(), true, Some(word));
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].source_line, 1);
        assert_eq!(ctx.instructions[0].source_value_line, Some(3));

        let opaque = WordExpr::BracedLiteral {
            text: word.legacy_text(),
            source: crate::ir::SourceSite::opaque(word.source().span),
        };
        ctx.emit_word_from_source(&opaque.legacy_text(), true, Some(&opaque));
        assert_eq!(ctx.instructions[1].source_value_line, None);
        ctx.set_source("");
        ctx.emit_word_from_source(&word.legacy_text(), true, Some(word));
        assert_eq!(ctx.instructions[2].source_value_line, None);
    }

    #[test]
    fn nested_value_emission_retains_only_the_owning_source_dispatch() {
        let registry = CommandRegistry::build_default();
        let source = "set result [list VALUE]";
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
        let segment = crate::segmenter::segment_commands(source).remove(0);
        let mut parent = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut parent);
        let word = parent.words()[2].clone();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.with_invocation_tokens(Some(&parent), |ctx| {
            let nested = ctx.nested_command_tokens(Some(&word)).unwrap();
            assert!(
                nested
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .proved_execution_target()
                    .is_some()
            );
        });

        let retained = std::mem::take(&mut parent.nested_bindings);
        ctx.with_invocation_tokens(Some(&parent), |ctx| {
            let nested = ctx.nested_command_tokens(Some(&word)).unwrap();
            assert!(nested.has_unproved_source_binding());
        });
        let opaque = WordExpr::CommandSubstitution {
            spelling: "[list VALUE]".into(),
            source: crate::ir::SourceSite::opaque(word.source().span),
        };
        parent.nested_bindings = retained;
        ctx.with_invocation_tokens(Some(&parent), |ctx| {
            let nested = ctx.nested_command_tokens(Some(&opaque)).unwrap();
            assert!(nested.has_unproved_source_binding());
        });
    }

    #[test]
    fn emit_assign_const_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::STORE_SCALAR1, Op::POP]);
    }

    #[test]
    fn emit_assign_const_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(
            opcodes(&ctx),
            vec![Op::PUSH1, Op::PUSH1, Op::STORE_STK, Op::POP]
        );
    }

    #[test]
    fn emit_incr_stmt() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let stmt = Statement::Incr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::INCR_SCALAR1_IMM, Op::POP]);
    }

    #[test]
    fn emit_call_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Call {
            span: sp(),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_call_break_in_loop() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.break_target = Some("loop_end_0".into());
        let stmt = Statement::Call {
            span: sp(),
            command: "break".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::JUMP4]);
    }

    #[test]
    fn plain_dispatch_keeps_loop_control_spellings_as_runtime_calls() {
        let registry = CommandRegistry::build_default();
        for command in ["break", "continue"] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.plain_command_dispatch = true;
            ctx.break_target = Some("loop_end_0".into());
            ctx.continue_target = Some("loop_next_0".into());
            let stmt = Statement::Call {
                span: sp(),
                command: command.into(),
                canonical_command: None,
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            };
            let mut used_generic_invoke = false;
            ctx.emit_stmt(&stmt, &mut used_generic_invoke);
            let ops = opcodes(&ctx);
            assert!(used_generic_invoke, "{command}: {ops:?}");
            assert!(ops.contains(&Op::INVOKE_STK1), "{command}: {ops:?}");
            assert!(!ops.contains(&Op::JUMP4), "{command}: {ops:?}");
        }
    }

    #[test]
    fn plain_dispatch_keeps_internal_dict_loops_as_runtime_calls() {
        let registry = CommandRegistry::build_default();
        for command in ["::tcl::dict::for", "::tcl::dict::map"] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.plain_command_dispatch = true;
            let stmt = Statement::Call {
                span: sp(),
                command: command.into(),
                canonical_command: None,
                args: vec!["k v".into(), "a 1".into(), "set seen $k".into()],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            };
            let mut used_generic_invoke = false;
            ctx.emit_stmt(&stmt, &mut used_generic_invoke);
            let ops = opcodes(&ctx);
            assert!(used_generic_invoke, "{command}: {ops:?}");
            assert!(ops.contains(&Op::INVOKE_STK1), "{command}: {ops:?}");
            assert!(
                !ops.contains(&Op::DICT_FIRST) && !ops.contains(&Op::DICT_NEXT),
                "{command} must not use builtin dict-loop opcodes: {ops:?}"
            );
        }
    }

    #[test]
    fn emit_return_empty() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Return {
            expr_base: None,
            tokens: None,
            span: sp(),
            value: None,
            value_word: None,
            expr: None,
            command_binding: None,
            braced: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::RETURN_IMM));
    }

    #[test]
    fn emit_stmt_with_start_cmd_numbering() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "1".into(),
            name_braced: false,
            value_span: None,
        };
        ctx.emit_stmt_with_start_cmd(&stmt, None, None);
        assert!(!opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 1);

        let stmt2 = Statement::AssignConst {
            span: sp(),
            name: "y".into(),
            value: "2".into(),
            name_braced: false,
            value_span: None,
        };
        ctx.emit_stmt_with_start_cmd(&stmt2, None, None);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 2);
    }

    /// A `Statement::Call` carrying the `<empty_clause>` name.
    fn empty_clause_call(marked: bool) -> Statement {
        Statement::Call {
            span: sp(),
            command: "<empty_clause>".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: marked
                .then(|| crate::ir::CommandTokens::marker(crate::ir::SyntheticMarker::EmptyClause)),
            foreach_groups: None,
        }
    }

    #[test]
    fn emit_empty_clause() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut ugi = false;
        ctx.emit_stmt(&empty_clause_call(true), &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP, Op::NOP, Op::NOP]);
    }

    /// The marker identity is the typed `SyntheticMarker`, never the command
    /// spelling: `<empty_clause>` — like every other marker name — is a legal
    /// Tcl command name, and tclsh 8.4.20 / 8.5.19 / 8.6.16 / 9.0.4 / 9.1 all
    /// run `proc <empty_clause> {} { puts hit-ec }; <empty_clause>`. An
    /// unmarked call of that name must therefore be *dispatched*, not folded
    /// away.
    #[test]
    fn an_unmarked_call_named_like_a_marker_is_still_dispatched() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut ugi = false;
        ctx.emit_stmt(&empty_clause_call(false), &mut ugi);
        let ops = opcodes(&ctx);
        assert!(
            ops.contains(&Op::INVOKE_STK1),
            "expected a real invoke, got {ops:?}"
        );
        assert!(!ops.contains(&Op::NOP), "must not fold to nops: {ops:?}");
    }

    #[test]
    fn emit_barrier_with_command() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "test".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["script".into()],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_barrier_without_command() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "side-effect".into(),
            command: String::new(),
            canonical_command: None,
            args: vec![],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP]);
    }

    #[test]
    fn registry_barrier_marker_is_not_dispatched() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "scalar facts".into(),
            command: "<registry-barrier>".into(),
            canonical_command: None,
            args: vec![],
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::RegistryBarrier,
            )),
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert!(opcodes(&ctx).is_empty());
    }

    #[test]
    fn registry_barrier_wrapper_keeps_the_next_command_boundary() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let first = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "1".into(),
            name_braced: false,
            value_span: None,
        };
        let barrier = Statement::Barrier {
            span: sp(),
            reason: "scalar facts".into(),
            command: "<registry-barrier>".into(),
            canonical_command: None,
            args: vec![],
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::RegistryBarrier,
            )),
        };
        let second = Statement::AssignConst {
            span: sp(),
            name: "y".into(),
            value: "2".into(),
            name_braced: false,
            value_span: None,
        };

        ctx.emit_stmt_with_start_cmd(&first, None, None);
        ctx.emit_stmt_with_start_cmd(&barrier, None, None);
        ctx.emit_stmt_with_start_cmd(&second, None, None);

        assert_eq!(
            opcodes(&ctx)
                .iter()
                .filter(|&&op| op == Op::START_CMD)
                .count(),
            1,
            "only the second real command receives a startCommand boundary",
        );
        assert_eq!(ctx.cmd_index, 2, "the marker is not a source command");
    }

    #[test]
    fn registry_barrier_under_start_cmd_does_not_advance_command_index() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let first = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "1".into(),
            name_braced: false,
            value_span: None,
        };
        let barrier = Statement::Barrier {
            span: sp(),
            reason: "scalar facts".into(),
            command: "<registry-barrier>".into(),
            canonical_command: None,
            args: vec![],
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::RegistryBarrier,
            )),
        };

        ctx.emit_stmt_with_start_cmd(&first, None, None);
        ctx.emit_stmt_under_start_cmd(&barrier);

        assert_eq!(ctx.cmd_index, 1, "the marker is not a source command");
    }

    #[test]
    fn emit_assign_expr() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let stmt = Statement::AssignExpr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            expr: ExprNode::Literal {
                text: "42".into(),
                start: 0,
                end: 2,
            },
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: "[expr {42}]".into(),
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert!(opcodes(&ctx).contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn assign_expr_with_whole_unit_mutation_retains_the_runtime_binding() {
        let registry = CommandRegistry::build_default();
        let mutations = crate::command_binding::ModuleCommandMutations::distrust_all();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.command_bindings = Some(&mutations);
        let stmt = Statement::AssignExpr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            expr: ExprNode::Literal {
                text: "42".into(),
                start: 0,
                end: 2,
            },
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: "[expr {42}]".into(),
        };

        let mut used_generic_invoke = false;
        ctx.emit_stmt(&stmt, &mut used_generic_invoke);
        let ops = opcodes(&ctx);
        assert!(
            !ops.contains(&Op::INVOKE_STK1),
            "the entered expression may stay specialised until its live boundary: {ops:?}"
        );
        assert!(
            ctx.command_binding_requirements.contains(
                &tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")
            ),
            "the fused expression must retain the binding that runtime revalidation protects"
        );
        assert!(ops.contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn syntax_owned_assign_expr_ignores_command_mutation() {
        let registry = CommandRegistry::build_default();
        let mutations = crate::command_binding::ModuleCommandMutations::distrust_all();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.command_bindings = Some(&mutations);
        let stmt = Statement::AssignExpr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            expr: ExprNode::Literal {
                text: "42".into(),
                start: 0,
                end: 2,
            },
            command_binding: None,
            expr_base: None,
            fallback_value: "$(42)".into(),
        };

        let mut used_generic_invoke = false;
        ctx.emit_stmt(&stmt, &mut used_generic_invoke);
        let ops = opcodes(&ctx);
        assert!(
            !ops.contains(&Op::INVOKE_STK1),
            "syntax-owned Jim expression must remain intrinsic: {ops:?}"
        );
        assert!(ctx.command_binding_requirements.is_empty());
        assert!(ops.contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn emit_expr_eval() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::ExprEval {
            span: sp(),
            command_binding: tcl_runtime_api::CommandBindingIdentity::new("expr", "expr"),
            expr: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            expr_base: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::POP));
        assert_eq!(
            ctx.command_binding_requirements,
            [tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn emit_expanded_call_basic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_expanded_call(
            "list head",
            &["ordinary".into(), "tail one tail two".into()],
            &[true, false, true],
            None,
        );
        let ops = opcodes(&ctx);
        assert_eq!(
            ops,
            vec![
                Op::EXPAND_START,
                Op::PUSH1,
                Op::EXPAND_STKTOP,
                Op::PUSH1,
                Op::PUSH1,
                Op::EXPAND_STKTOP,
                Op::INVOKE_EXPANDED,
                Op::POP,
            ],
            "head and tail words must use one ordered expansion path",
        );
    }
}
