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
fn is_bare_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

// -- Literal emission --

impl CodegenCtx<'_> {
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
        // braces (the one substitution braces permit); every other backslash
        // stays verbatim. Only a braced word — or a bare constant, for which
        // this is a borrow-only no-op — reaches this verbatim path (a bare word
        // separates on the continuation; a quoted word is `backslash_subst`-
        // decoded through `push_lit`), so collapsing here is safe.
        let value = tcl_syntax::backslash::collapse_brace_continuations_str(value);
        let idx = self.literals.intern(&value);
        let op = if idx < 256 { Op::PUSH1 } else { Op::PUSH4 };
        let pos = self.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(idx))],
            &format!("\"{}\"", esc(&value, 40)),
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
#[must_use]
pub fn split_array_ref(name: &str) -> Option<(&str, &str)> {
    if let (Some(open), true) = (name.find('('), name.ends_with(')')) {
        Some((&name[..open], &name[open + 1..name.len() - 1]))
    } else {
        None
    }
}

/// Return `true` if `name` is an array reference like `arr(key)`.
#[must_use]
pub fn is_array_ref(name: &str) -> bool {
    name.contains('(') && name.ends_with(')')
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
                    super::helpers::SubstPart::Lit(text) => self.push_lit(text),
                    super::helpers::SubstPart::Cmd(cmd) => self.emit_inline_cmd_subst(cmd),
                    super::helpers::SubstPart::Scalar(name) => {
                        self.push_lit(name);
                        self.emit(Op::LOAD_STK, vec![]);
                    }
                    super::helpers::SubstPart::Var(name) => self.load_var(name),
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(parts.len()).expect("array-key part count fits in i32"),
                )],
            );
        } else if elem.contains('\\') {
            // Pure literal key carrying backslash escapes (`be(\w\w)`,
            // `be(a\ a)`): a non-braced array index is an ordinary Tcl word, so
            // its escapes are decoded (`\w` → `w`, `\ ` → space) before the
            // element lookup — matching C Tcl. (Braced keys like `set {a($x)} 1`
            // never reach here; `push_var_ref` pushes those literally.)
            self.push_lit(&tcl_lexer::backslash_subst_in(elem, self.escapes));
        } else {
            // Pure literal key (or a key the template parser left whole).
            self.push_lit(elem);
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
            if let Some((base, elem)) = split_array_ref(name) {
                self.push_lit(base);
                self.push_array_key(elem);
                self.emit(Op::LOAD_ARRAY_STK, vec![]);
            } else {
                self.push_lit(name);
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
    pub fn emit_incr(&mut self, name: &str, amount: Option<&str>) {
        if self.is_proc && !is_qualified(name) {
            self.emit_incr_local(name, amount);
        } else {
            self.emit_incr_global_or_array(name, amount);
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

    /// `incr` against a global / qualified / array element.
    fn emit_incr_global_or_array(&mut self, name: &str, amount: Option<&str>) {
        let arr = split_array_ref(name);
        match amount {
            None => {
                if let Some((base, elem)) = arr {
                    self.push_lit(base);
                    self.push_lit(elem);
                    self.emit(Op::INCR_ARRAY_STK_IMM, vec![Operand::Imm(1)]);
                } else {
                    self.push_lit(name);
                    self.emit(Op::INCR_STK_IMM, vec![Operand::Imm(1)]);
                }
            }
            Some(amt) if is_integer_literal(amt) => {
                if let Some(imm) = self.parse_int_operand(amt) {
                    if (-128..=127).contains(&imm) {
                        if let Some((base, elem)) = arr {
                            self.push_lit(base);
                            self.push_lit(elem);
                            self.emit(
                                Op::INCR_ARRAY_STK_IMM,
                                vec![Operand::Imm(
                                    i32::try_from(imm)
                                        .expect("incr literal fits in i32 after range check"),
                                )],
                            );
                        } else {
                            self.push_lit(name);
                            self.emit(
                                Op::INCR_STK_IMM,
                                vec![Operand::Imm(
                                    i32::try_from(imm)
                                        .expect("incr literal fits in i32 after range check"),
                                )],
                            );
                        }
                    } else {
                        self.invoke_incr_fallback(name, amt);
                    }
                } else {
                    self.invoke_incr_fallback(name, amt);
                }
            }
            Some(amt) => {
                let var_ref = parse_simple_var_ref(amt, self.braced_var);
                if let (None, Some(vr)) = (arr, var_ref) {
                    self.push_lit(name);
                    self.load_var(vr);
                    self.emit(Op::INCR_STK, vec![]);
                } else {
                    self.invoke_incr_fallback(name, amt);
                }
            }
        }
    }

    /// Fallback: emit `incr name amt` as a generic invokeStk1.
    fn invoke_incr_fallback(&mut self, name: &str, amt: &str) {
        self.push_lit("incr");
        self.push_lit(name);
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

/// Extract variable name from a braced-scalar marker `$={name}`.
///
/// In Tcl, braces prevent array interpretation, so `${a(1)}` must be
/// loaded as a scalar via `push + loadStk`.  The compiler marks these
/// with `$={name}`.
#[must_use]
pub fn parse_braced_scalar_ref(value: &str) -> Option<&str> {
    value.strip_prefix("$={").and_then(|s| s.strip_suffix('}'))
}

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

    // -- parse_braced_scalar_ref --

    #[test]
    fn parse_braced_scalar_basic() {
        assert_eq!(parse_braced_scalar_ref("$={a(1)}"), Some("a(1)"));
    }

    #[test]
    fn parse_braced_scalar_no_match() {
        assert_eq!(parse_braced_scalar_ref("${x}"), None);
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
        ctx.emit_incr("x", None);
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
        ctx.emit_incr("x", Some("5"));
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
        ctx.emit_incr("x", Some("999"));
        // Large amount → push_lit + INCR_SCALAR1
        assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "999"
        assert_eq!(ctx.instructions[1].op, Op::INCR_SCALAR1);
    }

    #[test]
    fn emit_incr_default_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_incr("x", None);
        // push "x" then INCR_STK_IMM
        assert_eq!(ctx.instructions[0].op, Op::PUSH1);
        assert_eq!(ctx.instructions[1].op, Op::INCR_STK_IMM);
    }

    #[test]
    fn emit_incr_large_amount_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_incr("x", Some("999"));
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
