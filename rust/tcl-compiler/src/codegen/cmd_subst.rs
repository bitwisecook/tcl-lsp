//! Command substitution parsing and inline emission.
//!
//! Extends [`CodegenCtx`] with methods for parsing `[cmd arg ...]`
//! substitutions and emitting specialised bytecode sequences for
//! common Tcl commands (expr, incr, string, list, dict, etc.).
//! Ported from `core/compiler/codegen/_cmd_subst.py`.

#![allow(
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::similar_names,
    clippy::doc_markdown
)]

use super::helpers::{parse_subst_template, regexp_to_glob, SubstPart};
use super::values::{is_qualified, parse_braced_scalar_ref, parse_simple_var_ref, split_array_ref};
use super::{parse_tcl_index, str_class_id, CodegenCtx, Op, Operand, INDEX_END};

// ---------------------------------------------------------------------------
// Free functions — pure parsing, no emission state needed
// ---------------------------------------------------------------------------

/// Unroll `[set y [set z 42]]` into `["y", "z", "42"]`.
///
/// Returns the variable names followed by the innermost value,
/// or `None` if `value` is not a chain of nested `set` commands.
#[must_use]
pub fn unroll_nested_set(value: &str) -> Option<Vec<String>> {
    let mut chain = Vec::new();
    let mut v = value;
    while v.starts_with("[set ") && v.ends_with(']') {
        let inner = &v[1..v.len() - 1]; // strip [ ]
        let mut parts = inner.splitn(3, char::is_whitespace);
        let cmd = parts.next()?;
        if cmd != "set" {
            return None;
        }
        let var = parts.next()?;
        let rest = parts.next()?;
        if rest.is_empty() {
            return None;
        }
        chain.push(var.to_owned());
        v = rest;
    }
    if chain.is_empty() {
        return None;
    }
    chain.push(v.to_owned()); // innermost value
    Some(chain)
}

/// Return `true` if `value` is a single balanced `[cmd ...]`.
#[must_use]
pub fn is_pure_cmd_subst(value: &str) -> bool {
    if !value.starts_with('[') || !value.ends_with(']') {
        return false;
    }
    let mut depth: i32 = 0;
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' {
            i += 2; // skip escaped char (crude but sufficient)
            continue;
        }
        if ch == b'[' {
            depth += 1;
        } else if ch == b']' {
            depth -= 1;
        }
        if depth == 0 {
            return i == bytes.len() - 1;
        }
        i += 1;
    }
    false
}

/// Return `true` if `text` contains `;` or newline outside quotes/braces.
///
/// Used to detect multi-command scripts in `[...]` substitutions so
/// they can be deferred to runtime `EVAL_STK` instead of being
/// inlined as a single command call.
#[must_use]
pub fn has_command_separator(text: &str) -> bool {
    let inner = if text.starts_with('[') && text.ends_with(']') {
        &text[1..text.len() - 1]
    } else {
        text
    };
    let bytes = inner.as_bytes();
    let mut in_quotes = false;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' && i + 1 < bytes.len() {
            i += 2; // skip escaped character
            continue;
        }
        if ch == b'"' && brace_depth == 0 {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if ch == b'{' {
                brace_depth += 1;
            } else if ch == b'}' && brace_depth > 0 {
                brace_depth -= 1;
            } else if ch == b'[' {
                bracket_depth += 1;
            } else if ch == b']' && bracket_depth > 0 {
                bracket_depth -= 1;
            } else if (ch == b';' || ch == b'\n')
                && brace_depth == 0
                && bracket_depth == 0
            {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Split `[cmd arg1 ...]` into `(text, was_braced)` tuples.
///
/// `was_braced` is `true` when the original argument was wrapped
/// in `{…}` (braces are stripped from the returned text). This
/// lets the caller decide whether to re-wrap the value in braces.
#[must_use]
pub fn parse_cmd_parts(text: &str) -> Vec<(String, bool)> {
    let text = text.trim();
    let text = if text.starts_with('[') && text.ends_with(']') {
        text[1..text.len() - 1].trim()
    } else {
        text
    };
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < n {
        // Skip whitespace
        while i < n && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }

        if bytes[i] == b'"' {
            // Quoted string
            i += 1;
            let start = i;
            while i < n && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            parts.push((text[start..i].to_owned(), false));
            if i < n {
                i += 1; // skip closing "
            }
        } else if bytes[i] == b'{' {
            // Braced string
            let mut depth: i32 = 1;
            i += 1;
            let start = i;
            while i < n && depth > 0 {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'{' {
                    depth += 1;
                } else if bytes[i] == b'}' {
                    depth -= 1;
                }
                if depth > 0 {
                    i += 1;
                }
            }
            parts.push((text[start..i].to_owned(), true));
            if i < n {
                i += 1; // skip closing }
            }
        } else if bytes[i] == b'[' {
            // Command substitution (may have trailing non-ws suffix)
            let start = i;
            let mut depth: i32 = 0;
            while i < n {
                if bytes[i] == b'[' {
                    depth += 1;
                } else if bytes[i] == b']' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            // Continue reading non-whitespace after ']' (e.g. [namespace current]::foo)
            while i < n && bytes[i] != b' ' && bytes[i] != b'\t' {
                if bytes[i] == b'[' {
                    let mut inner_depth: i32 = 0;
                    while i < n {
                        if bytes[i] == b'[' {
                            inner_depth += 1;
                        } else if bytes[i] == b']' {
                            inner_depth -= 1;
                            if inner_depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            parts.push((text[start..i].to_owned(), false));
        } else {
            // Bare word
            let start = i;
            while i < n && bytes[i] != b' ' && bytes[i] != b'\t' {
                if bytes[i] == b'[' {
                    // Command substitution mid-word
                    let mut inner_depth: i32 = 0;
                    while i < n {
                        if bytes[i] == b'[' {
                            inner_depth += 1;
                        } else if bytes[i] == b']' {
                            inner_depth -= 1;
                            if inner_depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            parts.push((text[start..i].to_owned(), false));
        }
    }
    parts
}

// ---------------------------------------------------------------------------
// CodegenCtx methods — emission helpers for command substitutions
// ---------------------------------------------------------------------------

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
impl CodegenCtx {
    /// Emit a single arg from a parsed command substitution.
    ///
    /// Handles: `$var` loads, `$={name}` braced scalars, `[cmd]`
    /// nested substitutions, braced args with `$`/`[`, interpolated
    /// strings, backslash escapes, and plain literals.
    pub fn emit_cmd_subst_arg(&mut self, arg: &str, braced: bool) {
        if !braced && arg.starts_with('$') {
            // Braced scalar marker: $={name} → push + loadStk
            if let Some(name) = parse_braced_scalar_ref(arg) {
                self.push_lit(name);
                self.emit(Op::LOAD_STK, vec![]);
                return;
            }
            // ${var} form
            if let Some(var_name) = parse_simple_var_ref(arg) {
                self.load_var(var_name);
                return;
            }
            // Bare $varname form (not normalised to ${var})
            let rest = &arg[1..];
            if !rest.is_empty() && rest.chars().all(|c| c.is_alphanumeric() || c == '_') {
                self.load_var(rest);
                return;
            }
            // Bare $name(index) array ref form
            if !rest.is_empty() && split_array_ref(rest).is_some() {
                self.load_var(rest);
                return;
            }
            // Fallback: push as literal
            self.push_lit(arg);
        } else if !braced && arg.starts_with('[') && arg.ends_with(']') {
            // Nested command substitution — compile inline.
            let inner_parts = parse_cmd_parts(arg);
            let needs_sc =
                self.is_proc && !inner_parts.is_empty() && inner_parts[0].0 != "expr";
            if needs_sc {
                let sc_end = self.fresh_label("cmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                self.cmd_index += 1;
                self.emit_inline_cmd_subst(arg);
                self.place_label(&sc_end);
            } else {
                self.emit_inline_cmd_subst(arg);
            }
        } else if braced && (arg.contains('$') || arg.contains('[')) {
            // Braced arg with substitution markers — re-wrap in braces
            self.push_lit(&format!("{{{arg}}}"));
        } else if !braced && (arg.contains('$') || arg.contains('[')) {
            // Interpolated string — delegate to emit_value with interpolation
            self.emit_value(arg, true);
        } else if !braced && arg.contains('\\') {
            let processed = tcl_lexer::backslash_subst(arg);
            if processed.contains('$')
                || (processed.contains('[') && processed.contains(']'))
            {
                // After backslash processing, still has subst markers — push raw
                self.push_lit(&processed);
            } else {
                self.push_lit(&processed);
            }
        } else {
            self.push_lit(arg);
        }
    }

    /// Emit a generic command substitution as `push cmd; <args>; invokeStk`.
    pub fn emit_generic_cmd_subst(&mut self, cmd: &str, args: &[(String, bool)]) {
        self.push_lit(cmd);
        for (arg, braced) in args {
            if !braced && arg.starts_with('[') && arg.ends_with(']') {
                let end_label = self.fresh_label("cmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                    "",
                );
                self.emit_inline_cmd_subst(arg);
                self.place_label(&end_label);
            } else if !braced && arg.starts_with('$') {
                if let Some(name) = parse_braced_scalar_ref(arg) {
                    self.push_lit(name);
                    self.emit(Op::LOAD_STK, vec![]);
                } else if let Some(var_name) = parse_simple_var_ref(arg) {
                    self.load_var(var_name);
                } else {
                    self.push_lit(arg);
                }
            } else if *braced && (arg.contains('$') || arg.contains('[')) {
                self.push_lit(&format!("{{{arg}}}"));
            } else if !braced && arg.contains('\\') {
                let processed = tcl_lexer::backslash_subst(arg);
                self.push_lit(&processed);
            } else {
                self.push_lit(arg);
            }
        }
        let argc = (1 + args.len()) as i32;
        let op = if argc < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit(op, vec![Operand::Imm(argc)]);
    }

    /// Inline compile `[list {*}$a {*}$b]` as `load a; load b; listConcat`.
    ///
    /// Only matches the exact two-argument form — `[list {*}$x]` (one
    /// argument) and three-or-more-argument variants fall back to the
    /// generic path. The matched pattern is: `[list` followed by two
    /// `{*}$name` tokens separated by whitespace, closed by `]`.
    ///
    /// Returns `true` if the pattern matched and the bytecode was
    /// emitted. Ported from `core/compiler/codegen/_values.py::
    /// _try_list_expand_concat` (C19).
    pub fn try_list_expand_concat(&mut self, value: &str) -> bool {
        let Some(inner) = value.strip_prefix("[list").and_then(|s| s.strip_suffix(']')) else {
            return false;
        };
        let mut names: Vec<&str> = Vec::new();
        for tok in inner.split_whitespace() {
            let Some(name) = tok.strip_prefix("{*}$") else {
                return false;
            };
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return false;
            }
            names.push(name);
        }
        if names.len() != 2 {
            return false;
        }
        self.load_var(names[0]);
        self.load_var(names[1]);
        self.emit(Op::LIST_CONCAT, vec![]);
        true
    }

    /// Inline compile `[list arg ... [break] ...]` or
    /// `[list arg ... [continue] ...]` when a loop-break/continue
    /// target is in scope.
    ///
    /// tclsh 9.0 compiles `break`/`continue` inside a `list` command
    /// substitution as an inline jump with stack cleanup: values
    /// pushed for earlier list arguments are popped, then a `jump4`
    /// goes to the loop target. The trailing `list N` and any outer
    /// command instructions become dead code that the bytecode layout
    /// still lays out.
    ///
    /// Returns `true` if the pattern matched and was emitted. Ported
    /// from `core/compiler/codegen/_values.py::
    /// _try_inline_list_with_break_continue` (C19).
    pub fn try_inline_list_with_break_continue(&mut self, value: &str) -> bool {
        if !(value.starts_with("[list ") && value.ends_with(']')) {
            return false;
        }
        let parts = parse_cmd_parts(value);
        if parts.first().map(|(n, _)| n.as_str()) != Some("list") {
            return false;
        }
        let args = &parts[1..];
        let has_bc = args
            .iter()
            .any(|(a, _)| a == "[break]" || a == "[continue]");
        if !has_bc {
            return false;
        }

        let mut n_pushed: usize = 0;
        for (arg, _braced) in args {
            if arg == "[break]" && self.break_target.is_some() {
                let target = self.break_target.clone().unwrap();
                let end_label = self.fresh_label("cmd_end");
                self.emit(
                    Op::START_CMD,
                    vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                );
                self.cmd_index += 1;
                for _ in 0..n_pushed {
                    self.emit(Op::POP, vec![]);
                }
                self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], "break");
                self.place_label(&end_label);
                n_pushed += 1; // dead-code placeholder
            } else if arg == "[continue]" && self.continue_target.is_some() {
                let target = self.continue_target.clone().unwrap();
                let end_label = self.fresh_label("cmd_end");
                self.emit(
                    Op::START_CMD,
                    vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                );
                self.cmd_index += 1;
                for _ in 0..n_pushed {
                    self.emit(Op::POP, vec![]);
                }
                self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], "continue");
                self.place_label(&end_label);
                n_pushed += 1;
            } else {
                self.emit_value(arg, false);
                n_pushed += 1;
            }
        }
        let argc = i32::try_from(args.len()).unwrap_or(i32::MAX);
        self.emit(Op::LIST, vec![Operand::Imm(argc)]);
        true
    }

    /// Emit a value with full interpolation support.
    ///
    /// Extends the simplified `emit_value_interpolated` with command
    /// substitution inlining and `parse_subst_template` decomposition.
    pub fn emit_value(&mut self, value: &str, interpolate: bool) {
        // Braced scalar marker: $={name} → push + loadStk
        if let Some(name) = parse_braced_scalar_ref(value) {
            self.push_lit(name);
            self.emit(Op::LOAD_STK, vec![]);
            return;
        }
        // Variable reference: ${var} → load
        if let Some(var_name) = parse_simple_var_ref(value) {
            self.load_var(var_name);
            return;
        }
        // Constant-fold [list arg1 arg2 ...]
        if let Some(folded) = super::helpers::fold_list_cmd(value) {
            self.push_lit_no_dedup(&folded);
            return;
        }
        // Inline [list {*}$a {*}$b] → load a, load b, listConcat (C19).
        // tclsh 9.0 compiles two-list expansion as a specialised
        // listConcat opcode rather than a generic `list` invoke.
        if self.try_list_expand_concat(value) {
            return;
        }
        // Inline [list arg ... [break] ...] or [list arg ... [continue] ...]
        // (C19). tclsh 9.0 compiles break/continue inside `list` command
        // substitutions as inline jumps with stack cleanup.
        if self.try_inline_list_with_break_continue(value) {
            return;
        }
        // Constant-fold [format "..." arg ...] with literal args (C19).
        // Relies on the existing `helpers::try_format_fold` for %s/%d/%%.
        if let Some(folded) = super::helpers::try_format_fold(value) {
            self.push_lit_no_dedup(&folded);
            return;
        }
        // Constant-fold [dict create k v ...]
        if let Some(folded) = super::helpers::fold_dict_create_cmd(value) {
            self.push_lit(&folded);
            self.emit(Op::DUP, vec![]);
            self.emit(Op::VERIFY_DICT, vec![]);
            return;
        }
        // Interpolated string: decompose $var and [cmd] parts
        if interpolate && (value.contains('$') || value.contains('[')) {
            if let Some(parts) = parse_subst_template(value) {
                if parts.len() > 1 {
                    for part in &parts {
                        match part {
                            SubstPart::Lit(text) => self.push_lit(text),
                            SubstPart::Cmd(cmd_text) => {
                                self.emit_inline_cmd_subst(cmd_text);
                            }
                            SubstPart::Scalar(name) => {
                                self.push_lit(name);
                                self.emit(Op::LOAD_STK, vec![]);
                            }
                            SubstPart::Var(name) => {
                                self.load_var(name);
                            }
                        }
                    }
                    self.emit(
                        Op::STR_CONCAT1,
                        vec![Operand::Imm(parts.len() as i32)],
                    );
                    return;
                }
            }
        }
        // Default: push as literal
        self.push_lit(value);
    }

    /// Inline-compile a `[cmd arg ...]` command substitution.
    ///
    /// Handles `[expr {...}]` specially by parsing and inlining the
    /// expression body. Other commands are compiled as
    /// `push cmd; <args>; invokeStk1 N`.
    ///
    /// Multi-command scripts (containing `;` or newlines outside
    /// quotes/braces) fall back to runtime `EVAL_STK`.
    ///
    /// Specialised commands:
    /// - `expr` — inline expression compilation
    /// - `incr` — LVT or stack-based increment
    /// - `info exists` — existScalar / existStk
    /// - `string` subcommands (index, range, equal, compare, length,
    ///   is, replace, map, match, trim, reverse, repeat, tolower,
    ///   toupper, totitle, first, last, cat)
    /// - `lindex`, `lrange`, `lreplace`, `linsert`
    /// - `regexp`
    /// - `list`
    /// - `array exists`
    /// - `dict get`
    /// - `catch` (delegates to control_flow)
    #[allow(clippy::too_many_lines)]
    pub fn emit_inline_cmd_subst(&mut self, text: &str) {
        // Multi-command scripts fall back to runtime eval.
        if has_command_separator(text) {
            let inner = if text.starts_with('[') && text.ends_with(']') {
                &text[1..text.len() - 1]
            } else {
                text
            };
            self.push_lit(&format!("{{{inner}}}"));
            self.emit(Op::EVAL_STK, vec![]);
            return;
        }

        let parts = parse_cmd_parts(text);
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        let cmd = &parts[0].0;
        let args = &parts[1..];

        match cmd.as_str() {
            "expr" if args.len() == 1 => {
                let expr_body = &args[0].0;
                let node = crate::expr_parser::parse_expr(expr_body, None);
                self.emit_expr(&node);
            }
            "incr" if (1..=2).contains(&args.len()) => {
                self.emit_inline_incr(args);
            }
            "info" if args.len() == 2 && args[0].0 == "exists" => {
                self.emit_inline_info_exists(args);
            }
            "string" if args.len() >= 2 => {
                self.emit_inline_string(args);
            }
            "lindex" if args.len() >= 2 => {
                self.emit_inline_lindex(args);
            }
            "lrange" if args.len() == 3 => {
                self.emit_inline_lrange(args);
            }
            "lreplace" if args.len() >= 3 => {
                self.emit_inline_lreplace(args);
            }
            "linsert" if args.len() >= 2 => {
                self.emit_inline_linsert(args);
            }
            "regexp" if args.len() >= 2 => {
                self.emit_inline_regexp(args, &parts);
            }
            "list" if !args.is_empty() && !text.contains("{*}") => {
                self.used_inline_cmd_subst = true;
                for (a, b) in args {
                    self.emit_cmd_subst_arg(a, *b);
                }
                self.emit(Op::LIST, vec![Operand::Imm(args.len() as i32)]);
            }
            "array" if args.len() >= 2 => {
                self.emit_inline_array(args);
            }
            "dict" if args.len() >= 3 && args[0].0 == "get" => {
                self.emit_inline_dict_get(args);
            }
            "catch" if self.is_proc && (1..=3).contains(&args.len()) => {
                let result_var = args.get(1).map(|(s, _)| s.as_str());
                if result_var.is_some_and(|v| v.starts_with("::")) {
                    self.used_inline_cmd_subst = false;
                    self.emit_generic_cmd_subst(cmd, args);
                } else {
                    self.used_inline_cmd_subst = true;
                    let body_text = &args[0].0;
                    let options_var = args.get(2).map(|(s, _)| s.as_str());
                    self.emit_catch_inline(body_text, result_var, options_var);
                }
            }
            _ => {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(cmd, args);
            }
        }
    }

    // -- Private inline helpers for emit_inline_cmd_subst --

    fn emit_inline_incr(&mut self, args: &[(String, bool)]) {
        let var_name = &args[0].0;
        if self.is_proc && !is_qualified(var_name) {
            let slot = self.lvt.intern(var_name) as i32;
            if args.len() == 1 {
                self.emit_comment(
                    Op::INCR_SCALAR1_IMM,
                    vec![Operand::Imm(slot), Operand::Imm(1)],
                    &format!("var \"{var_name}\""),
                );
            } else {
                let amt_str = &args[1].0;
                if let Ok(amt) = amt_str.parse::<i64>() {
                    if (-128..=127).contains(&amt) {
                        self.emit_comment(
                            Op::INCR_SCALAR1_IMM,
                            vec![Operand::Imm(slot), Operand::Imm(amt as i32)],
                            &format!("var \"{var_name}\""),
                        );
                    } else {
                        self.push_lit(amt_str);
                        self.emit_comment(
                            Op::INCR_SCALAR1,
                            vec![Operand::Imm(slot)],
                            &format!("var \"{var_name}\""),
                        );
                    }
                } else {
                    let var_ref = amt_str.strip_prefix('$').unwrap_or(amt_str);
                    self.load_var(var_ref);
                    self.emit_comment(
                        Op::INCR_SCALAR1,
                        vec![Operand::Imm(slot)],
                        &format!("var \"{var_name}\""),
                    );
                }
            }
        } else {
            self.push_lit(var_name);
            if args.len() == 1 {
                self.emit(Op::INCR_STK_IMM, vec![Operand::Imm(1)]);
            } else {
                let amt_str = &args[1].0;
                if let Ok(amt) = amt_str.parse::<i32>() {
                    self.emit(Op::INCR_STK_IMM, vec![Operand::Imm(amt)]);
                } else {
                    let var_ref = amt_str.strip_prefix('$').unwrap_or(amt_str);
                    self.load_var(var_ref);
                    self.emit(Op::INCR_STK, vec![]);
                }
            }
        }
    }

    fn emit_inline_info_exists(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let var_name = &args[1].0;
        if self.is_proc && !is_qualified(var_name) {
            let slot = self.lvt.intern(var_name) as i32;
            self.emit_comment(
                Op::EXIST_SCALAR,
                vec![Operand::Imm(slot)],
                &format!("var \"{var_name}\""),
            );
            self.emit(Op::NOP, vec![]);
        } else {
            self.push_lit(var_name);
            self.emit(Op::EXIST_STK, vec![]);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn emit_inline_string(&mut self, args: &[(String, bool)]) {
        let subcmd = &args[0].0;
        let sargs = &args[1..];
        let prev_inline = self.used_inline_cmd_subst;
        self.used_inline_cmd_subst = true;

        match subcmd.as_str() {
            "index" if sargs.len() == 2 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit(Op::STR_INDEX, vec![]);
            }
            "range" if sargs.len() == 3 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                let start_idx = parse_tcl_index(&sargs[1].0);
                let end_idx = parse_tcl_index(&sargs[2].0);
                if let (Some(s), Some(e)) = (start_idx, end_idx) {
                    self.emit(Op::STR_RANGE_IMM, vec![Operand::Imm(s), Operand::Imm(e)]);
                } else {
                    self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                    self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
                    self.emit(Op::STR_RANGE, vec![]);
                }
            }
            "equal" if sargs.len() == 2 => {
                let sc_end = if self.is_proc {
                    None
                } else {
                    self.used_inline_cmd_subst = prev_inline;
                    let label = self.fresh_label("subcmd_end");
                    self.emit_comment(
                        Op::START_CMD,
                        vec![Operand::Label(label.clone()), Operand::Imm(1)],
                        "",
                    );
                    Some(label)
                };
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit(Op::STR_EQ, vec![]);
                if let Some(label) = sc_end {
                    self.place_label(&label);
                }
            }
            "equal" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.used_inline_cmd_subst = prev_inline;
                let sc_end = self.fresh_label("subcmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                self.push_lit("string");
                self.push_lit("equal");
                self.push_lit("-nocase");
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
                self.push_lit("::tcl::string::equal");
                self.emit(
                    Op::INVOKE_REPLACE,
                    vec![Operand::Imm(5), Operand::Imm(2)],
                );
                self.place_label(&sc_end);
                self.seen_generic_invoke = true;
            }
            "compare" if sargs.len() == 2 => {
                let sc_end = if self.is_proc {
                    None
                } else {
                    self.used_inline_cmd_subst = prev_inline;
                    let label = self.fresh_label("subcmd_end");
                    self.emit_comment(
                        Op::START_CMD,
                        vec![Operand::Label(label.clone()), Operand::Imm(1)],
                        "",
                    );
                    Some(label)
                };
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit(Op::STR_CMP, vec![]);
                if let Some(label) = sc_end {
                    self.place_label(&label);
                }
            }
            "compare" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.used_inline_cmd_subst = prev_inline;
                let sc_end = self.fresh_label("subcmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                self.push_lit("string");
                self.push_lit("compare");
                self.push_lit("-nocase");
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
                self.push_lit("::tcl::string::compare");
                self.emit(
                    Op::INVOKE_REPLACE,
                    vec![Operand::Imm(5), Operand::Imm(2)],
                );
                self.place_label(&sc_end);
                self.seen_generic_invoke = true;
            }
            "compare" if sargs.len() == 4 && sargs[0].0 == "-length" => {
                self.used_inline_cmd_subst = prev_inline;
                let sc_end = self.fresh_label("subcmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                self.push_lit("string");
                self.push_lit("compare");
                self.push_lit("-length");
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
                self.emit_cmd_subst_arg(&sargs[3].0, sargs[3].1);
                self.push_lit("::tcl::string::compare");
                self.emit(
                    Op::INVOKE_REPLACE,
                    vec![Operand::Imm(6), Operand::Imm(2)],
                );
                self.place_label(&sc_end);
                self.seen_generic_invoke = true;
            }
            "replace" if sargs.len() == 4 => {
                self.emit_inline_string_replace(sargs);
            }
            "length" if sargs.len() == 1 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit(Op::STR_LEN, vec![]);
            }
            "is" if sargs.len() >= 2 => {
                self.emit_inline_string_is(sargs);
            }
            _ => {
                // Unhandled string subcommand: use FQN invoke
                self.used_inline_cmd_subst = prev_inline;
                let sc_end = self.fresh_label("subcmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                let fqn = format!("::tcl::string::{subcmd}");
                self.push_lit(&fqn);
                for (a, b) in sargs {
                    self.emit_cmd_subst_arg(a, *b);
                }
                let argc = (1 + sargs.len()) as i32;
                self.emit(Op::INVOKE_STK1, vec![Operand::Imm(argc)]);
                self.place_label(&sc_end);
                self.seen_generic_invoke = true;
            }
        }
    }

    fn emit_inline_string_replace(&mut self, sargs: &[(String, bool)]) {
        let first_lit = &sargs[1].0;
        let last_lit = &sargs[2].0;
        if first_lit == "0" {
            if let Ok(last_int) = last_lit.parse::<i32>() {
                if last_int >= 0 {
                    self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                    self.emit_cmd_subst_arg(&sargs[3].0, sargs[3].1);
                    self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                    self.emit(
                        Op::STR_RANGE_IMM,
                        vec![Operand::Imm(last_int + 1), Operand::Imm(INDEX_END)],
                    );
                    self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
                    return;
                }
            }
        }
        // Fallback: strreplace
        self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
        self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
        self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
        self.emit_cmd_subst_arg(&sargs[3].0, sargs[3].1);
        self.emit(Op::STR_REPLACE, vec![]);
    }

    #[allow(clippy::too_many_lines)]
    fn emit_inline_string_is(&mut self, sargs: &[(String, bool)]) {
        let class_name = &sargs[0].0;
        // Detect -strict flag and value
        let (strict, val_arg) = if sargs.len() == 3 && sargs[1].0 == "-strict" {
            (true, &sargs[2])
        } else {
            (false, &sargs[sargs.len() - 1])
        };

        if let Some(class_id) = str_class_id(class_name) {
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
            self.emit(Op::STR_CLASS, vec![Operand::Imm(i32::from(class_id))]);
        } else if class_name == "integer" {
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
            if strict {
                self.emit(Op::NUMERIC_TYPE, vec![]);
                self.emit(Op::DUP, vec![]);
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP_FALSE1, vec![Operand::Label(end_lbl.clone())]);
                self.push_lit("3");
                self.emit(Op::LE, vec![]);
                self.place_label(&end_lbl);
            } else {
                self.emit(Op::DUP, vec![]);
                self.emit(Op::NUMERIC_TYPE, vec![]);
                self.emit(Op::DUP, vec![]);
                let has_type = self.fresh_label("si_has_type");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(has_type.clone())]);
                self.emit(Op::POP, vec![]);
                self.push_lit("");
                self.emit(Op::STR_EQ, vec![]);
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&has_type);
                self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                self.emit(Op::POP, vec![]);
                self.push_lit("3");
                self.emit(Op::LE, vec![]);
                self.place_label(&end_lbl);
            }
        } else if class_name == "double" {
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
            if strict {
                self.emit(Op::NUMERIC_TYPE, vec![]);
                let true_lbl = self.fresh_label("si_true");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
                self.push_lit("0");
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&true_lbl);
                self.push_lit("1");
                self.place_label(&end_lbl);
            } else {
                self.emit(Op::DUP, vec![]);
                self.push_lit("");
                self.emit(Op::STR_EQ, vec![]);
                let true_lbl = self.fresh_label("si_true");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
                self.emit(Op::NUMERIC_TYPE, vec![]);
                let has_type = self.fresh_label("si_has_type");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(has_type.clone())]);
                self.push_lit("0");
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&true_lbl);
                self.emit(Op::POP, vec![]);
                self.place_label(&has_type);
                self.push_lit("1");
                self.place_label(&end_lbl);
            }
        } else if class_name == "boolean" {
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
            self.emit(Op::TRY_CVT_TO_BOOLEAN, vec![]);
            let true_lbl = self.fresh_label("si_true");
            self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
            self.push_lit("");
            self.emit(Op::STR_EQ, vec![]);
            let end_lbl = self.fresh_label("si_end");
            self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
            self.place_label(&true_lbl);
            self.emit(Op::POP, vec![]);
            self.push_lit("1");
            self.place_label(&end_lbl);
        } else {
            self.used_inline_cmd_subst = false;
            // Rebuild full args list with "string" prefix
            let mut full_args = vec![("string".to_owned(), false)];
            full_args.extend_from_slice(sargs);
            let all_args = &full_args[1..]; // skip "string" cmd itself
            self.emit_generic_cmd_subst("string", all_args);
        }
    }

    fn emit_inline_lindex(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1); // list
        if args.len() == 2 {
            let idx = parse_tcl_index(&args[1].0);
            if let Some(idx) = idx {
                if idx >= 0 || idx <= INDEX_END {
                    self.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(idx)]);
                } else {
                    self.emit_cmd_subst_arg(&args[1].0, args[1].1);
                    self.emit(Op::LIST_INDEX, vec![]);
                }
            } else {
                self.emit_cmd_subst_arg(&args[1].0, args[1].1);
                self.emit(Op::LIST_INDEX, vec![]);
            }
        } else {
            for a in &args[1..] {
                self.emit_cmd_subst_arg(&a.0, a.1);
            }
            self.emit(Op::LINDEX_MULTI, vec![Operand::Imm(args.len() as i32)]);
        }
    }

    fn emit_inline_lrange(&mut self, args: &[(String, bool)]) {
        // Decide between LIST_RANGE_IMM and the generic fallback
        // *before* emitting any arguments — otherwise the fallback
        // would push the list a second time, leaving an extra value
        // on the stack.
        let start_idx = parse_tcl_index(&args[1].0);
        let end_idx = parse_tcl_index(&args[2].0);
        if let (Some(s), Some(e)) = (start_idx, end_idx) {
            self.used_inline_cmd_subst = true;
            self.emit_cmd_subst_arg(&args[0].0, args[0].1);
            self.emit(
                Op::LIST_RANGE_IMM,
                vec![Operand::Imm(s), Operand::Imm(e)],
            );
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("lrange", args);
        }
    }

    fn emit_inline_lreplace(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1);
        for a in &args[1..] {
            self.emit_cmd_subst_arg(&a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(args.len() as i32), Operand::Imm(1)],
        );
    }

    fn emit_inline_linsert(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1);
        for a in &args[1..] {
            self.emit_cmd_subst_arg(&a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(args.len() as i32), Operand::Imm(2)],
        );
    }

    fn emit_inline_regexp(&mut self, args: &[(String, bool)], all_parts: &[(String, bool)]) {
        let mut rargs: Vec<&(String, bool)> = args.iter().collect();
        let mut nocase = false;
        if !rargs.is_empty() && rargs[0].0 == "-nocase" {
            nocase = true;
            rargs.remove(0);
        }
        if !rargs.is_empty() && rargs[0].0 == "--" {
            rargs.remove(0);
        }
        if rargs.len() == 2 && nocase {
            if let Some(glob) = regexp_to_glob(&rargs[0].0) {
                self.used_inline_cmd_subst = true;
                self.push_lit(&glob);
                self.emit_cmd_subst_arg(&rargs[1].0, rargs[1].1);
                self.emit(Op::STR_MATCH, vec![Operand::Imm(1)]);
            } else {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst("regexp", args);
            }
        } else if rargs.len() == 2 && !nocase {
            self.used_inline_cmd_subst = true;
            for (a, b) in &all_parts[1..] {
                self.emit_cmd_subst_arg(a, *b);
            }
            self.emit(Op::REGEXP, vec![Operand::Imm(all_parts.len() as i32)]);
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("regexp", args);
        }
    }

    fn emit_inline_array(&mut self, args: &[(String, bool)]) {
        let sub = &args[0].0;
        let rest = &args[1..];
        if sub == "exists"
            && rest.len() == 1
            && self.is_proc
            && !is_qualified(&rest[0].0)
        {
            self.used_inline_cmd_subst = true;
            let slot = self.lvt.intern(&rest[0].0) as i32;
            self.emit_comment(
                Op::ARRAY_EXISTS_IMM,
                vec![Operand::Imm(slot)],
                &format!("var \"{}\"", rest[0].0),
            );
        } else if (sub == "names" || sub == "size") && !rest.is_empty() {
            let sc_end = self.fresh_label("subcmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                "",
            );
            let fqn = format!("::tcl::array::{sub}");
            self.push_lit(&fqn);
            for (a, b) in rest {
                self.emit_cmd_subst_arg(a, *b);
            }
            self.emit(
                Op::INVOKE_STK1,
                vec![Operand::Imm((1 + rest.len()) as i32)],
            );
            self.place_label(&sc_end);
            self.seen_generic_invoke = true;
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("array", args);
        }
    }

    fn emit_inline_dict_get(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let dict_args = &args[1..]; // skip "get"
        self.emit_cmd_subst_arg(&dict_args[0].0, dict_args[0].1); // dict value
        let keys = &dict_args[1..];
        for (k, b) in keys {
            self.emit_cmd_subst_arg(k, *b);
        }
        self.emit(Op::DICT_GET, vec![Operand::Imm(keys.len() as i32)]);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- unroll_nested_set --

    #[test]
    fn unroll_simple() {
        let r = unroll_nested_set("[set y [set z 42]]").unwrap();
        assert_eq!(r, vec!["y", "z", "42"]);
    }

    #[test]
    fn unroll_not_nested() {
        assert!(unroll_nested_set("hello").is_none());
    }

    // -- is_pure_cmd_subst --

    #[test]
    fn pure_cmd_subst_simple() {
        assert!(is_pure_cmd_subst("[expr {1+2}]"));
    }

    #[test]
    fn pure_cmd_subst_nested() {
        assert!(is_pure_cmd_subst("[set x [set y 1]]"));
    }

    #[test]
    fn pure_cmd_subst_not() {
        assert!(!is_pure_cmd_subst("hello"));
        assert!(!is_pure_cmd_subst("[a] [b]"));
    }

    // -- has_command_separator --

    #[test]
    fn separator_semicolon() {
        assert!(has_command_separator("set x 1; set y 2"));
    }

    #[test]
    fn separator_newline() {
        assert!(has_command_separator("set x 1\nset y 2"));
    }

    #[test]
    fn separator_in_braces_ignored() {
        assert!(!has_command_separator("{set x 1; set y 2}"));
    }

    #[test]
    fn separator_in_quotes_ignored() {
        assert!(!has_command_separator("\"set x 1; set y 2\""));
    }

    #[test]
    fn no_separator() {
        assert!(!has_command_separator("set x 1"));
    }

    // -- parse_cmd_parts --

    #[test]
    fn parse_simple_cmd() {
        let parts = parse_cmd_parts("[set x 42]");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ("set".into(), false));
        assert_eq!(parts[1], ("x".into(), false));
        assert_eq!(parts[2], ("42".into(), false));
    }

    #[test]
    fn parse_braced_arg() {
        let parts = parse_cmd_parts("[expr {1+2}]");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], ("expr".into(), false));
        assert_eq!(parts[1], ("1+2".into(), true));
    }

    #[test]
    fn parse_quoted_arg() {
        let parts = parse_cmd_parts("[puts \"hello world\"]");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], ("puts".into(), false));
        assert_eq!(parts[1], ("hello world".into(), false));
    }

    #[test]
    fn parse_nested_cmd() {
        let parts = parse_cmd_parts("[set x [expr {1+2}]]");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ("set".into(), false));
        assert_eq!(parts[1], ("x".into(), false));
        assert_eq!(parts[2], ("[expr {1+2}]".into(), false));
    }

    // -- emit_cmd_subst_arg --

    #[test]
    fn emit_arg_literal() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_cmd_subst_arg("hello", false);
        assert_eq!(ctx.instructions[0].op, Op::PUSH1);
    }

    #[test]
    fn emit_arg_var_ref() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        ctx.emit_cmd_subst_arg("${x}", false);
        assert_eq!(ctx.instructions[0].op, Op::LOAD_SCALAR1);
    }

    // -- emit_generic_cmd_subst --

    #[test]
    fn emit_generic_simple() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_generic_cmd_subst("puts", &[("hello".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::PUSH1, Op::INVOKE_STK1]);
    }

    // -- emit_inline_cmd_subst --

    #[test]
    fn inline_expr() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.emit_inline_cmd_subst("[expr {1+2}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // Should produce push "1", push "2", add
        assert!(ops.contains(&Op::ADD));
    }

    #[test]
    fn inline_incr_proc() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        ctx.emit_inline_cmd_subst("[incr x]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::INCR_SCALAR1_IMM));
    }

    #[test]
    fn inline_string_length() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.emit_inline_cmd_subst("[string length ${x}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::STR_LEN));
    }

    #[test]
    fn inline_list() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.emit_inline_cmd_subst("[list a b c]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LIST));
    }

    #[test]
    fn inline_multicommand_falls_back() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.emit_inline_cmd_subst("[set x 1; set y 2]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EVAL_STK));
    }

    // -- regression: label reconstruction in string equal/compare --

    /// `string equal` in non-proc context with a nested command
    /// substitution in one arg. The nested substitution allocates
    /// labels (via `fresh_label`), so if the fast-path reconstructed
    /// the end-label name from `label_counter - 1` it would resolve
    /// to a different label.
    #[test]
    fn inline_string_equal_nested_cmd_label_resolves() {
        let mut ctx = CodegenCtx::new(false, &[]);
        // Nested `[expr ...]` does not allocate labels, but nested
        // `[array names ...]` wraps in a startCommand, which does.
        ctx.emit_inline_cmd_subst("[string equal [array names a] foo]");
        // Build expected: the startCommand end label should resolve
        // to the `STR_EQ` position (or later), not a dangling index.
        let has_str_eq = ctx.instructions.iter().any(|i| i.op == Op::STR_EQ);
        assert!(has_str_eq, "expected STR_EQ in output");
        // All Label operands must resolve (i.e. exist in label_positions).
        for instr in &ctx.instructions {
            for op in &instr.operands {
                if let Operand::Label(l) = op {
                    assert!(
                        ctx.label_positions.contains_key(l),
                        "unresolved label {l:?} in {:?}",
                        instr.op
                    );
                }
            }
        }
    }

    #[test]
    fn inline_string_compare_nested_cmd_label_resolves() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_inline_cmd_subst("[string compare [array names a] foo]");
        let has_str_cmp = ctx.instructions.iter().any(|i| i.op == Op::STR_CMP);
        assert!(has_str_cmp, "expected STR_CMP in output");
        for instr in &ctx.instructions {
            for op in &instr.operands {
                if let Operand::Label(l) = op {
                    assert!(
                        ctx.label_positions.contains_key(l),
                        "unresolved label {l:?} in {:?}",
                        instr.op
                    );
                }
            }
        }
    }

    // -- C19 specialised value-emission paths --

    #[test]
    fn try_list_expand_concat_matches_two_vars() {
        let mut ctx = CodegenCtx::new(false, &[]);
        assert!(ctx.try_list_expand_concat("[list {*}$a {*}$b]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "a"; loadStk; push "b"; loadStk; listConcat
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::LOAD_STK,
                Op::PUSH1,
                Op::LOAD_STK,
                Op::LIST_CONCAT,
            ]
        );
    }

    #[test]
    fn try_list_expand_concat_rejects_single_var() {
        let mut ctx = CodegenCtx::new(false, &[]);
        assert!(!ctx.try_list_expand_concat("[list {*}$a]"));
        assert!(ctx.instructions.is_empty());
    }

    #[test]
    fn try_list_expand_concat_rejects_three_vars() {
        let mut ctx = CodegenCtx::new(false, &[]);
        assert!(!ctx.try_list_expand_concat("[list {*}$a {*}$b {*}$c]"));
    }

    #[test]
    fn try_list_expand_concat_rejects_non_expanded_arg() {
        let mut ctx = CodegenCtx::new(false, &[]);
        // Literal first arg without {*} prefix — falls back to generic path.
        assert!(!ctx.try_list_expand_concat("[list a {*}$b]"));
    }

    #[test]
    fn try_inline_list_without_target_emits_break_as_literal() {
        // Matches Python: when `[list ... [break] ...]` appears without
        // a loop target in scope, the pattern still claims the value
        // and emits `[break]` as a literal list element. The generic
        // fallback is never reached.
        let mut ctx = CodegenCtx::new(true, &[]);
        assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // No JUMP4 since no break target.
        assert!(!ops.contains(&Op::JUMP4));
        // Still emits LIST N at the end.
        assert!(ops.contains(&Op::LIST));
    }

    #[test]
    fn try_inline_list_with_break_emits_jump_to_target() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.break_target = Some("loop_break_1".into());
        assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "a"; startCommand; pop; jump4 break_target; push "c"; list 3
        assert!(ops.contains(&Op::JUMP4), "expected JUMP4, got {ops:?}");
        assert!(ops.contains(&Op::START_CMD), "expected START_CMD, got {ops:?}");
        assert!(ops.contains(&Op::LIST), "expected LIST, got {ops:?}");
    }

    #[test]
    fn try_inline_list_without_break_returns_false() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.break_target = Some("loop_break_1".into());
        // No [break]/[continue] inside — should not match.
        assert!(!ctx.try_inline_list_with_break_continue("[list a b c]"));
    }

    /// `lrange` with non-literal indices must not push the list arg
    /// twice when falling back to the generic invoke path.
    #[test]
    fn inline_lrange_variable_indices_no_double_push() {
        let mut ctx = CodegenCtx::new(true, &["lst", "a", "b"]);
        ctx.emit_inline_cmd_subst("[lrange ${lst} ${a} ${b}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // Should be: push "lrange"; load lst; load a; load b; invokeStk1 4
        // → 1 push, 3 loads, 1 invoke. NOT: load lst; push "lrange"; ...
        let load_count = ops
            .iter()
            .filter(|o| matches!(o, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4))
            .count();
        assert_eq!(load_count, 3, "expected 3 var loads, got {ops:?}");
        let invoke_count = ops.iter().filter(|o| **o == Op::INVOKE_STK1).count();
        assert_eq!(invoke_count, 1, "expected one invokeStk1, got {ops:?}");
    }
}
