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
use super::statements::has_unescaped_subst;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

/// Whether `name` is a whole **bare** Tcl variable name — the run of characters
/// a `$name` reference consumes: ASCII alphanumerics, `_`, and `:` (namespace
/// separators). A name with any other character (`-`, `.`, `(`, `$`) is *not* a
/// whole bare reference, so e.g. `$item-suffix` is `$item` followed by literal
/// `-suffix`, not a variable called `item-suffix`.
///
/// This is the deliberately *looser* codegen-side contract recorded in
/// `docs/design/contracts/shared-utility-contracts-rust.md`, distinct from the
/// stricter `::`-segmented `tcl_syntax::naming::is_bare_var_name` that quick
/// fixes use. `pub(crate)` so the WASM backend consumes this one rather than
/// re-deriving the charset (issue #1459).
pub(crate) fn is_bare_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

/// The exact Tcl variable name `word` refers to, when the **whole** word is one
/// simple variable reference eligible for a direct load — otherwise `None`.
///
/// The single owner of the `$name` / `${name}` split for the codegen backends
/// (issue #1459); `codegen::wasm::backend` and `codegen::wasm::leaf_invoke`
/// each carried a byte-identical copy.
///
/// The two spellings are **not** validated alike, and that asymmetry is the
/// point: braces are Tcl's own escape for a name the bare charset cannot
/// express, so `${…}` accepts any non-empty name verbatim, while a bare
/// `$name` must be a whole [`is_bare_var_name`] run — otherwise the word is
/// `$name` *followed by literal text* (`$item-suffix`) and loading `item-suffix`
/// would be wrong. Routing the braced form through the charset check as well
/// would change behaviour, not just deduplicate.
///
/// Note the braced form here is decided by the *last* `}` in the word; the
/// release-aware `${…}` close rule lives with the decoders in
/// [`parse_simple_var_ref`] and is issue #1568's territory, not this one.
pub(crate) fn whole_var_reference(word: &str) -> Option<&str> {
    if let Some(name) = word.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return (!name.is_empty()).then_some(name);
    }
    let name = word.strip_prefix('$')?;
    is_bare_var_name(name).then_some(name)
}

/// Which literal-pool entry point a folded constant takes.
///
/// The only thing that ever distinguished the two value emitters' copies of
/// the constant-fold block, and not interchangeable: the verbatim path also
/// sets `push_verbatim`, which suppresses runtime word substitution on the
/// pushed literal. That matters for a folded result that still contains a `$`
/// — `[list {a$b}]` folds to `a$b`, since the braced word is literal — so each
/// emitter keeps the entry point it had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldedLiteral {
    /// [`CodegenCtx::push_lit_no_dedup_verbatim`] — `emit_value_interpolated`.
    Verbatim,
    /// [`CodegenCtx::push_lit_no_dedup`] — `emit_value`.
    NoDedup,
}

// -- Literal emission --

impl CodegenCtx<'_> {
    /// The `[list …]` / `[format …]` / `[dict create …]` constant folds and
    /// the two `list` inlinings that sit between them — one copy, shared by
    /// both value emitters, which carried it identically apart from `kind`.
    ///
    /// Every fold is gated on its own builtin still *being* the builtin
    /// anywhere in this unit (issue #1585): a fold **is** that command's
    /// semantics, so after `rename list mylist` or a shadowing `proc format …`
    /// it would answer for a command that is no longer there. `dict` carries
    /// the release gate as well (issue #1427) — folding bypasses the runtime's
    /// availability check, so an ungated fold would make `dict create` *work*
    /// under `--tcl-version 8.4`.
    ///
    /// Returns `true` when it emitted the value and the caller must stop.
    pub(crate) fn try_emit_constant_fold(&mut self, value: &str, kind: FoldedLiteral) -> bool {
        let push = |ctx: &mut Self, folded: &str| match kind {
            FoldedLiteral::Verbatim => ctx.push_lit_no_dedup_verbatim(folded),
            FoldedLiteral::NoDedup => ctx.push_lit_no_dedup(folded),
        };
        // Constant-fold [list arg1 arg2 ...].
        if self.trusts_builtin("list")
            && let Some(folded) = super::helpers::fold_list_cmd(value)
        {
            push(self, &folded);
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
        // Constant-fold [format "..." arg ...] with literal args (%s/%d/%%).
        if self.trusts_builtin("format")
            && let Some(folded) = super::helpers::try_format_fold(value)
        {
            push(self, &folded);
            return true;
        }
        // Constant-fold [dict create k v ...].
        if self.registry.has_command_in_this_dialect("dict")
            && self.trusts_builtin("dict")
            && let Some(folded) = super::helpers::fold_dict_create_cmd(value)
        {
            self.push_lit(&folded);
            self.emit(Op::DUP, vec![]);
            self.emit(Op::VERIFY_DICT, vec![]);
            return true;
        }
        false
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

// -- Array reference helpers --

/// Split `arr(key)` into `("arr", "key")`, or `None` for scalars.
///
/// The codegen-side facade over the one element-split owner,
/// [`split_element_ref`](tcl_syntax::naming::split_element_ref) —
/// `TclObjLookupVarEx`'s rule (`tclVar.c(9.0.4):683-686`). It carried a
/// byte-equivalent re-implementation until issue #1606; both halves may be
/// empty, so `(x)` is element `x` of the array named `""` (issue #1458).
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

/// Return `true` for namespace-qualified variable names (`::foo`).
#[must_use]
pub fn is_qualified(name: &str) -> bool {
    name.starts_with("::")
}

/// True when a store/load needs the name/key pushed onto the stack
/// first (i.e. uses `*Stk` instructions).
#[must_use]
pub fn needs_stk_var_ref(name: &str, is_proc: bool) -> bool {
    if !is_proc {
        return true;
    }
    if is_qualified(name) {
        return true;
    }
    is_array_ref(name)
}

// -- Variable load/store --

impl CodegenCtx<'_> {
    /// Push an array element key onto the stack.
    ///
    /// A key that is *exactly* a whole variable reference (`${var}` or `$var`)
    /// takes the [`load_var`](Self::load_var) fast path (matching tclsh's
    /// `LOAD_SCALAR`-based key in proc context). A *composite* key that embeds
    /// a substitution (`-$opt`, `x$item`, `${item}suf`, `$a([f])`) is built by
    /// the full interpolation emitter so the substitution actually runs —
    /// previously such keys were pushed as a raw literal, so the variable in
    /// the index never expanded and the element lookup failed. A pure literal
    /// key is pushed verbatim.
    pub fn push_array_key(&mut self, elem: &str) {
        if let Some(inner) = parse_simple_var_ref(elem, self.braced_var) {
            // Whole braced variable reference: `${var}`.
            //
            // Resolved through the same release-aware decoder as every other
            // `${…}` consumer (issue #1568). This arm used to hand-roll the
            // scan — `strip_suffix('}')` plus a
            // `.filter(|inner| !inner.contains(['{', '}']))` guard — which was
            // a *fourth* copy of the close rule and the only one left
            // unthreaded. It got two shapes wrong:
            //
            //   set {a\}b} K; set arr(K) V; puts $arr(${a\}b})
            //   set {a\{b} K; set arr(K) V; puts $arr(${a\{b})
            //
            // A whole `${…}` key containing a backslash fell past this arm
            // (the guard rejects any brace in the name), past the bare-`$`
            // arm, and past the composite arm (one part, not >1), landing in
            // the trailing `elem.contains('\\')` literal arm — which ran
            // `backslash_subst_in` over the *whole* `${…}` spelling. Inside
            // `${}` the name is literal in C, so that decoded escapes that
            // must stay verbatim: the first vector failed at 9.x and the
            // second at **every** release. Composite keys (`x${a\}b}`) were
            // always correct, which is why only this arm needed the fix.
            self.load_var(inner);
        } else if let Some(var) = elem.strip_prefix('$').filter(|v| is_bare_var_name(v)) {
            // Whole bare variable reference: `$var` (the name runs to the end).
            self.load_var(var);
        } else if has_unescaped_subst(elem)
            && let Some(parts) =
                super::helpers::parse_subst_template(elem, self.escapes, self.braced_var)
            && parts.len() > 1
        {
            // Composite key with an embedded substitution (`-$opt`, `x$item`,
            // `${item}suf`, `$a([f])`): build the index string at compile time
            // by concatenating the decoded parts. The runtime `subst_word`
            // fallback only resolves a *normalised* `${name}`, so a bare `$item`
            // inside the index would otherwise never expand.
            for part in &parts {
                match part {
                    // Already decoded by the template parser, so byte-exact —
                    // see the finished-key arms below.
                    super::helpers::SubstPart::Lit(text) => self.push_lit_exact(text),
                    super::helpers::SubstPart::Cmd(cmd) => self.emit_inline_cmd_subst(cmd),
                    super::helpers::SubstPart::Var(name) => self.load_var(name),
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(parts.len()).expect("array-key part count fits in i32"),
                )],
            );
        } else if has_unescaped_subst(elem) {
            // A live `$` / `[` the template parser left whole (`a([f])`): only
            // the VM’s runtime word substitution can resolve it, so this is the
            // one key shape that still goes out through the substituting push.
            self.push_lit(elem);
        } else if elem.contains('\\') {
            // Pure literal key carrying backslash escapes (`be(\w\w)`,
            // `be(a\ a)`): a non-braced array index is an ordinary Tcl word, so
            // its escapes are decoded (`\w` → `w`, `\ ` → space) before the
            // element lookup — matching C Tcl. (Braced keys like `set {a($x)} 1`
            // never reach here; `push_var_ref` pushes those literally.)
            self.push_lit_exact(&tcl_lexer::backslash_subst_in(elem, self.escapes));
        } else {
            // Pure literal key: finished here, so it is pushed byte-exactly for
            // the same reason a resolved *name* is (see `push_var_ref`) — the
            // VM must not substitute it a second time. An index is not a word:
            // a brace in it is an ordinary key byte, so `subst_word` would strip
            // it and read the wrong element. tclsh 8.4.20 / 8.5.19 / 8.6.16 all
            // agree, and the store side already resolves the key this way:
            //
            // ```text
            // set a({a}) BRACED ; set b(\{a\}) ESCBRACE
            // array names a -> {a} ; array names b -> {a}
            // set v $b(\{a\}) -> ESCBRACE   (a substituting push read `b(a)`)
            // ```
            self.push_lit_exact(elem);
        }
    }

    /// Emit load instructions for a variable reference.
    ///
    /// Proc context uses LVT-based opcodes; top-level uses stack-based.
    /// Array references are decomposed into base + element.
    pub fn load_var(&mut self, name: &str) {
        if self.is_proc && !is_qualified(name) {
            if let Some((base, elem)) = split_array_ref(name) {
                let slot = self.lvt.intern(base);
                self.push_array_key(elem);
                self.emit_comment(
                    Op::LOAD_ARRAY1,
                    vec![Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{base}\""),
                );
            } else {
                let slot = self.lvt.intern(name);
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
            }
        } else {
            // The name is already *resolved* (`parse_simple_var_ref` /
            // `SubstPart::Var` hand this method a variable name, never a word),
            // so it goes out verbatim for the same reason a store name does:
            // the VM must not word-substitute a name a second time and strip
            // its outer braces — `set {{}} Z; puts ${{}}` reads the variable
            // `{}` on tclsh 9.0.4 / 9.1 and printed `can't read ""` here
            // (issue #1602). Only the element key still substitutes.
            //
            // Byte-exact, not `push_lit_verbatim`: a resolved name is not a
            // braced *word*, so its `\<newline>` bytes are name content and
            // must not collapse — see `push_lit_exact`.
            if let Some((base, elem)) = split_array_ref(name) {
                self.push_lit_exact(base);
                self.push_array_key(elem);
                self.emit(Op::LOAD_ARRAY_STK, vec![]);
            } else {
                self.push_lit_exact(name);
                self.emit(Op::LOAD_STK, vec![]);
            }
        }
    }

    /// Emit store instructions for a variable reference.
    ///
    /// Caller must have pushed the value on TOS.  For proc bodies, uses
    /// `storeScalar1`/`storeArray1`.  For top-level, caller must have
    /// pushed name (and key for arrays) before the value.
    pub fn store_var(&mut self, name: &str) {
        if self.is_proc && !is_qualified(name) {
            if let Some((base, _elem)) = split_array_ref(name) {
                let slot = self.lvt.intern(base);
                self.emit_comment(
                    Op::STORE_ARRAY1,
                    vec![Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{base}\""),
                );
            } else {
                let slot = self.lvt.intern(name);
                let op = if slot < 256 {
                    Op::STORE_SCALAR1
                } else {
                    Op::STORE_SCALAR4
                };
                self.emit_comment(
                    op,
                    vec![Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{name}\""),
                );
            }
        } else if is_array_ref(name) {
            self.emit(Op::STORE_ARRAY_STK, vec![]);
        } else {
            self.emit(Op::STORE_STK, vec![]);
        }
    }

    /// Emit incr bytecode, leaving the new value on TOS.
    ///
    /// Handles literal amounts (immediate or pushed), and variable
    /// amounts (load + incr).  For non-proc contexts, falls back to
    /// `invokeStk` when the amount is large or complex.
    ///
    /// `name` is a [resolved store name](CodegenCtx::store_target) and
    /// `key_is_literal` its element-key half — the same contract
    /// [`push_var_ref`](CodegenCtx::push_var_ref) documents, so a name the
    /// compiler already resolved is never word-substituted again by the VM.
    pub fn emit_incr(&mut self, name: &str, key_is_literal: bool, amount: Option<&str>) {
        if self.is_proc && !is_qualified(name) {
            self.emit_incr_local(name, amount);
        } else {
            self.emit_incr_global_or_array(name, key_is_literal, amount);
        }
    }

    /// `incr` against a local proc slot (LVT).
    fn emit_incr_local(&mut self, name: &str, amount: Option<&str>) {
        let slot = self.lvt.intern(name);
        match amount {
            None => {
                self.emit_comment(
                    Op::INCR_SCALAR1_IMM,
                    vec![Operand::Imm(bytecode_imm(slot)), Operand::Imm(1)],
                    &format!("var \"{name}\""),
                );
            }
            Some(amt) if is_integer_literal(amt) => {
                if let Some(imm) = self.parse_int_operand(amt) {
                    if (-128..=127).contains(&imm) {
                        self.emit_comment(
                            Op::INCR_SCALAR1_IMM,
                            vec![
                                Operand::Imm(bytecode_imm(slot)),
                                Operand::Imm(
                                    i32::try_from(imm)
                                        .expect("incr literal fits in i32 after range check"),
                                ),
                            ],
                            &format!("var \"{name}\""),
                        );
                    } else {
                        self.push_lit(amt);
                        self.emit_comment(
                            Op::INCR_SCALAR1,
                            vec![Operand::Imm(bytecode_imm(slot))],
                            &format!("var \"{name}\""),
                        );
                    }
                } else {
                    // Overflow — fall back to push + incr
                    self.push_lit(amt);
                    self.emit_comment(
                        Op::INCR_SCALAR1,
                        vec![Operand::Imm(bytecode_imm(slot))],
                        &format!("var \"{name}\""),
                    );
                }
            }
            Some(amt) => {
                // Variable amount — try to resolve as ${var} reference
                let var_ref = parse_simple_var_ref(amt, self.braced_var);
                self.load_var(var_ref.unwrap_or(amt));
                self.emit_comment(
                    Op::INCR_SCALAR1,
                    vec![Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{name}\""),
                );
            }
        }
    }

    /// Push the `incr` target's name halves. A resolved name / array base is a
    /// finished literal and goes out verbatim (see
    /// [`push_var_ref`](CodegenCtx::push_var_ref)); an element key keeps the
    /// substituting `push_lit` path until `key_is_literal` says it is finished
    /// too.
    fn push_incr_target(&mut self, name: &str, key_is_literal: bool) {
        match split_array_ref(name) {
            Some((base, elem)) => {
                self.push_lit_exact(base);
                if key_is_literal {
                    self.push_lit_exact(elem);
                } else {
                    self.push_lit(elem);
                }
            }
            None => self.push_lit_exact(name),
        }
    }

    /// `incr` against a global / qualified / array element.
    fn emit_incr_global_or_array(
        &mut self,
        name: &str,
        key_is_literal: bool,
        amount: Option<&str>,
    ) {
        let is_array = split_array_ref(name).is_some();
        let step = |ctx: &mut Self, imm: i32| {
            let op = if is_array {
                Op::INCR_ARRAY_STK_IMM
            } else {
                Op::INCR_STK_IMM
            };
            ctx.emit(op, vec![Operand::Imm(imm)]);
        };
        match amount {
            None => {
                self.push_incr_target(name, key_is_literal);
                step(self, 1);
            }
            Some(amt) if is_integer_literal(amt) => {
                match self.parse_int_operand(amt) {
                    Some(imm) if (-128..=127).contains(&imm) => {
                        self.push_incr_target(name, key_is_literal);
                        step(
                            self,
                            i32::try_from(imm).expect("incr literal fits in i32 after range check"),
                        );
                    }
                    // Out of the immediate range (or unparseable) — the generic
                    // `incr` invoke.
                    _ => self.invoke_incr_fallback(name, key_is_literal, amt),
                }
            }
            Some(amt) => {
                let var_ref = parse_simple_var_ref(amt, self.braced_var);
                if let (false, Some(vr)) = (is_array, var_ref) {
                    self.push_incr_target(name, key_is_literal);
                    self.load_var(vr);
                    self.emit(Op::INCR_STK, vec![]);
                } else {
                    self.invoke_incr_fallback(name, key_is_literal, amt);
                }
            }
        }
    }

    /// Fallback: emit `incr name amt` as a generic invokeStk1 — the shape an
    /// out-of-immediate-range or non-literal amount takes.
    ///
    /// The name word must reach the command *already resolved*. Pushing a
    /// [resolved store name](CodegenCtx::store_target) through the plain
    /// `push_lit` path sends it back through the VM's `subst_word`, which
    /// re-substitutes a base whose escapes the compiler has already decoded:
    /// `incr a\133b\135($i) 999` names the array `a[b]`, and re-substituting
    /// `a[b]($i)` executed the command `b` and incremented the array `a7`.
    /// tclsh 8.4.20 / 8.5.19 / 8.6.16 / 9.0.4 / 9.1 all agree:
    ///
    /// ```text
    /// proc b {} { puts "BOOM-b-ran" ; return 7 }
    /// set i K ; set a\133b\135(K) 5 ; incr a\133b\135($i) 999
    /// -> array `a[b]` (4 bytes) is `K 1004`, and `b` never runs
    /// ```
    ///
    /// So the halves are pushed separately — the resolved base verbatim, the
    /// live key through the substituting path — and joined into the one word
    /// the command takes. A fully resolved name (the scalar case, and any
    /// braced or escape-only target) is a single verbatim push as before.
    fn invoke_incr_fallback(&mut self, name: &str, key_is_literal: bool, amt: &str) {
        self.push_lit("incr");
        match split_array_ref(name) {
            Some((base, elem)) if !key_is_literal => {
                self.push_lit_exact(base);
                self.push_lit_exact("(");
                self.push_lit(elem);
                self.push_lit_exact(")");
                self.emit(Op::STR_CONCAT1, vec![Operand::Imm(4)]);
            }
            _ => self.push_lit_exact(name),
        }
        self.push_lit(amt);
        self.emit_comment(Op::INVOKE_STK1, vec![Operand::Imm(3)], "incr");
    }
}

// -- Reference parsing --

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
/// re-scanned here. This function used to walk brace *depth* and require the
/// first balanced close to be the final byte — which is the **9.x** rule,
/// applied at every release — while `helpers::parse_subst_template` used the
/// **8.x** first-`}` rule. Two decoders reading one encoding under two
/// different releases' rules is what made issue #1568's outcome *inverted*
/// rather than merely wrong.
///
/// The whole value must be the reference: a trailing byte after the name's
/// closer means this word is `${…}` followed by literal text, which is not a
/// simple variable load. Under 9.x that still admits nested references like
/// `${::a(${::a(1)})}`, because the nesting rule consumes the inner pairs.
///
/// An unterminated name yields `None`: there is no reference to load, and the
/// caller's fallback path re-reads the word (issue #1457 gave the shared owner
/// [`tcl_lexer::BracedVarEnd::Unterminated`] precisely so each consumer stops
/// inventing its own recovery).
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

// The `$={name}` "braced scalar" marker used to be decoded here, by a
// `strip_prefix("$={")` + `strip_suffix('}')` pair. It is gone (issue #1617).
//
// It was a port artifact: no pass in this workspace has ever *produced* that
// spelling — the segmenter re-spells a braced variable word verbatim from
// source (`${…}`, see `segmenter::word_piece`), and nothing else writes a `$=`
// prefix. A marker with no producer is not an internal encoding at all: every
// word that reached the decoder was the user's own text, and `$={y}` is
// literal text in every supported release —
//
// ```tcl
// set y hi
// puts $={y}     ;# 8.4-9.0: $={y}   — we compiled it to `push y; loadStk` → hi
// ```
//
// (`$` is only a substitution trigger before a name character, and `=` is not
// one; `Tcl_ParseVarName`, tmp/tcl9.0.4/generic/tclParse.c). So the decoder was
// pure wrong-code: a whole-word `$={name}` loaded the variable `name` instead
// of pushing the literal. It was also unpinnable — the mutation inventory's
// M3b (flip the marker arm's close scan) survived the whole corpus precisely
// because no *real* program could tell the two halves apart (#1615).
//
// Nothing is lost by dropping it: the form it existed to spell, `${a(1)}`,
// reaches `parse_simple_var_ref` and `load_var`, which agree with both tclsh
// oracles (`set {a(1)} S; array set a {1 A}; puts ${a(1)}` → `A`, because
// `TclObjLookupVar` parses the parens out of the *name* at lookup time).
//
// Do not reintroduce a marker in the `$…` space: any spelling a user can type
// collides with real source. A future internal marker needs an out-of-band
// channel (an IR node or a word flag), not a string prefix.

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

    // -- split_array_ref --

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

    /// The owner's edge cases hold through this facade (issues #1458, #1606):
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

    // -- is_array_ref, is_qualified --

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
    }

    #[test]
    fn is_qualified_no() {
        assert!(!is_qualified("foo"));
    }

    // -- parse_simple_var_ref --

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

    // -- whole_var_reference (issue #1459) --

    /// The braced and bare spellings are validated **differently**, and the
    /// asymmetry is load-bearing: braces are Tcl's own escape for a name the
    /// bare charset cannot express. Hoisting the braced arm through
    /// `is_bare_var_name` — the obvious "deduplication" — would change
    /// behaviour, so it is pinned here.
    #[test]
    fn whole_var_reference_accepts_any_non_empty_braced_name() {
        assert_eq!(whole_var_reference("${a-b}"), Some("a-b"));
        assert_eq!(whole_var_reference("${a.b}"), Some("a.b"));
        assert_eq!(whole_var_reference("${a(b)}"), Some("a(b)"));
        assert_eq!(whole_var_reference("${x}"), Some("x"));
        // …but not an empty one.
        assert_eq!(whole_var_reference("${}"), None);
    }

    #[test]
    fn whole_var_reference_charset_checks_only_the_bare_form() {
        assert_eq!(whole_var_reference("$x"), Some("x"));
        assert_eq!(whole_var_reference("$::ns::x"), Some("::ns::x"));
        assert_eq!(whole_var_reference("$x_1"), Some("x_1"));
        // `$item-suffix` is `$item` followed by literal text, not a whole
        // reference — so the *word* is not a simple variable load.
        assert_eq!(whole_var_reference("$item-suffix"), None);
        assert_eq!(whole_var_reference("$a(b)"), None);
        assert_eq!(whole_var_reference("$"), None);
    }

    #[test]
    fn whole_var_reference_rejects_a_word_that_is_not_a_reference() {
        assert_eq!(whole_var_reference("x"), None);
        assert_eq!(whole_var_reference(""), None);
        assert_eq!(whole_var_reference("[f]"), None);
        assert_eq!(whole_var_reference("a$b"), None);
    }

    // -- the retired `$={name}` marker (issue #1617) --

    /// A whole word spelt `$={name}` is the *user's* literal text — `=` is not
    /// a name character, so `Tcl_ParseVarName` never starts a substitution
    /// there and both tclsh oracles print `$={y}` for `puts $={y}`. The
    /// producer-less "braced scalar" marker that used to claim this spelling
    /// compiled it to `push "y"; loadStk`, silently reading a variable.
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

    // -- is_integer_literal --

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

    // -- CodegenCtx value emission --

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
        // Large → invokeStk fallback
        assert!(ctx.instructions.iter().any(|i| i.op == Op::INVOKE_STK1));
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
    fn needs_stk_var_ref_cases() {
        // Top-level always needs stack
        assert!(needs_stk_var_ref("x", false));
        // Proc scalar doesn't
        assert!(!needs_stk_var_ref("x", true));
        // Proc qualified does
        assert!(needs_stk_var_ref("::x", true));
        // Proc array does (for key)
        assert!(needs_stk_var_ref("a(1)", true));
    }
}
