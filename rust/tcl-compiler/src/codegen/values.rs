//! Variable loading/storing and value emission.
//!
//! Extends [`CodegenCtx`] with methods for pushing literals, loading
//! and storing variables, emitting increments, and parsing variable
//! reference markers.  Ported from `core/compiler/codegen/_values.py`.

use super::format::esc;
use super::{bytecode_imm, CodegenCtx, Op, Operand};

// -- Literal emission --

impl CodegenCtx {
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

impl CodegenCtx {
    /// Push an array element key onto the stack.
    ///
    /// Handles `$var` references in the key by loading the variable.
    /// For complex keys (containing `$` or `[`), falls through to a
    /// literal push — full interpolation requires the main emitter.
    pub fn push_array_key(&mut self, elem: &str) {
        if let Some(inner) = elem.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
            // Braced variable reference: ${var}
            self.load_var(inner);
        } else if let Some(var) = elem.strip_prefix('$') {
            // Bare variable reference: $var
            self.load_var(var);
        } else {
            // Literal key (or complex key that needs the main emitter)
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
                    if let Ok(imm) = amt.parse::<i64>() {
                        if (-128..=127).contains(&imm) {
                            self.emit_comment(
                                Op::INCR_SCALAR1_IMM,
                                vec![Operand::Imm(bytecode_imm(slot)), Operand::Imm(i32::try_from(imm).expect("incr literal fits in i32 after range check"))],
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
                    let var_ref = parse_simple_var_ref(amt);
                    self.load_var(var_ref.unwrap_or(amt));
                    self.emit_comment(
                        Op::INCR_SCALAR1,
                        vec![Operand::Imm(bytecode_imm(slot))],
                        &format!("var \"{name}\""),
                    );
                }
            }
        } else {
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
                    if let Ok(imm) = amt.parse::<i64>() {
                        if (-128..=127).contains(&imm) {
                            if let Some((base, elem)) = arr {
                                self.push_lit(base);
                                self.push_lit(elem);
                                self.emit(Op::INCR_ARRAY_STK_IMM, vec![Operand::Imm(i32::try_from(imm).expect("incr literal fits in i32 after range check"))]);
                            } else {
                                self.push_lit(name);
                                self.emit(Op::INCR_STK_IMM, vec![Operand::Imm(i32::try_from(imm).expect("incr literal fits in i32 after range check"))]);
                            }
                        } else {
                            // Large increment — fall back to invokeStk
                            self.push_lit("incr");
                            self.push_lit(name);
                            self.push_lit(amt);
                            self.emit_comment(Op::INVOKE_STK1, vec![Operand::Imm(3)], "incr");
                        }
                    } else {
                        // Overflow — fall back to invokeStk
                        self.push_lit("incr");
                        self.push_lit(name);
                        self.push_lit(amt);
                        self.emit_comment(Op::INVOKE_STK1, vec![Operand::Imm(3)], "incr");
                    }
                }
                Some(amt) => {
                    let var_ref = parse_simple_var_ref(amt);
                    if let (None, Some(vr)) = (arr, var_ref) {
                        self.push_lit(name);
                        self.load_var(vr);
                        self.emit(Op::INCR_STK, vec![]);
                    } else {
                        self.push_lit("incr");
                        self.push_lit(name);
                        self.push_lit(amt);
                        self.emit_comment(Op::INVOKE_STK1, vec![Operand::Imm(3)], "incr");
                    }
                }
            }
        }
    }
}

// -- Reference parsing --

/// Extract variable name from a normalised `${var}` reference.
///
/// The lowering pass normalises actual variable substitutions to
/// `${varname}`; bare `$varname` (from braced literals like `{$x}`)
/// is left as-is.  Only the `${...}` form is treated as a resolvable
/// reference.
///
/// Handles nested variable references like `${::a(${::a(1)})}` where
/// the inner `${...}` creates additional brace pairs.
#[must_use]
pub fn parse_simple_var_ref(value: &str) -> Option<&str> {
    let rest = value.strip_prefix("${")?;
    if !value.ends_with('}') {
        return None;
    }

    // Verify the closing } matches the opening ${ by checking balanced braces.
    let mut depth: i32 = 0;
    for (i, ch) in value.char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                // The first balanced close should be the final character.
                return if i == value.len() - 1 {
                    Some(&rest[..rest.len() - 1])
                } else {
                    None
                };
            }
        }
    }
    None
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
        assert_eq!(parse_simple_var_ref("${x}"), Some("x"));
    }

    #[test]
    fn parse_simple_var_ref_qualified() {
        assert_eq!(parse_simple_var_ref("${::foo}"), Some("::foo"));
    }

    #[test]
    fn parse_simple_var_ref_nested() {
        assert_eq!(
            parse_simple_var_ref("${::a(${::a(1)})}"),
            Some("::a(${::a(1)})")
        );
    }

    #[test]
    fn parse_simple_var_ref_bare_dollar() {
        assert_eq!(parse_simple_var_ref("$x"), None);
    }

    #[test]
    fn parse_simple_var_ref_no_close() {
        assert_eq!(parse_simple_var_ref("${x"), None);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.push_lit_no_dedup("x");
        ctx.push_lit_no_dedup("x");
        assert_eq!(ctx.literals.len(), 2); // two distinct slots
    }

    #[test]
    fn load_var_scalar_proc() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.load_var("x");
        assert_eq!(ctx.instructions.len(), 2); // push name + loadStk
        assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "x"
        assert_eq!(ctx.instructions[1].op, Op::LOAD_STK);
    }

    #[test]
    fn load_var_array_proc() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.load_var("arr(key)");
        // Should intern "arr", push_lit "key", then LOAD_ARRAY1
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY1);
    }

    #[test]
    fn load_var_array_toplevel() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.load_var("arr(key)");
        // push "arr", push "key", LOAD_ARRAY_STK
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY_STK);
    }

    #[test]
    fn load_var_qualified_in_proc() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.load_var("::global_var");
        // Qualified vars always use stack-based ops
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_STK);
    }

    #[test]
    fn store_var_proc() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        ctx.store_var("x");
        assert_eq!(ctx.instructions[0].op, Op::STORE_SCALAR1);
    }

    #[test]
    fn store_var_toplevel() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.store_var("x");
        assert_eq!(ctx.instructions[0].op, Op::STORE_STK);
    }

    #[test]
    fn emit_incr_default_proc() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
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
        let mut ctx = CodegenCtx::new(true, &["x"]);
        ctx.emit_incr("x", Some("5"));
        assert_eq!(ctx.instructions[0].op, Op::INCR_SCALAR1_IMM);
        assert_eq!(
            ctx.instructions[0].operands[1],
            super::super::Operand::Imm(5)
        );
    }

    #[test]
    fn emit_incr_large_amount_proc() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        ctx.emit_incr("x", Some("999"));
        // Large amount → push_lit + INCR_SCALAR1
        assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "999"
        assert_eq!(ctx.instructions[1].op, Op::INCR_SCALAR1);
    }

    #[test]
    fn emit_incr_default_toplevel() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_incr("x", None);
        // push "x" then INCR_STK_IMM
        assert_eq!(ctx.instructions[0].op, Op::PUSH1);
        assert_eq!(ctx.instructions[1].op, Op::INCR_STK_IMM);
    }

    #[test]
    fn emit_incr_large_amount_toplevel() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_incr("x", Some("999"));
        // Large → invokeStk fallback
        assert!(ctx.instructions.iter().any(|i| i.op == Op::INVOKE_STK1));
    }

    #[test]
    fn begin_end_command() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
